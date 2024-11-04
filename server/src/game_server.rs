use {
    anyhow::Result,
    blocks::BlockGrid,
    crossbeam::queue::SegQueue,
    futures_util::{SinkExt, StreamExt},
    glam::Vec3,
    net_types::PlayerId,
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    },
    tokio::{
        net::TcpListener,
        select,
        sync::mpsc::{self, Receiver, Sender},
    },
    tokio_tungstenite::tungstenite::Message,
};

pub struct GameServer {
    spawner: tokio::runtime::Handle,

    blocks: BlockGrid,
    player_spawn_point: glam::Vec3,

    next_client_id: u64,
    clients: HashMap<ClientId, Client>,

    next_player_id: u64,
    players: HashMap<PlayerId, Player>,

    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,
}

impl GameServer {
    pub fn new(spawner: tokio::runtime::Handle) -> Self {
        let incoming_connections = Arc::new(SegQueue::new());

        spawner.spawn(start_client_listener(incoming_connections.clone()));

        let size = 32;
        let blocks = generate_map(size, size);
        // Roughly in the center of the map
        let player_spawn_point = glam::Vec3::new(size as f32 / 2., 4., size as f32 / 2.);

        Self {
            spawner,
            blocks,
            player_spawn_point,
            next_client_id: 0,
            clients: HashMap::new(),
            next_player_id: 0,
            players: HashMap::new(),
            incoming_connections,
        }
    }

    pub fn tick(&mut self) {
        let _handle = self.spawner.enter();

        // Handle new connections
        while let Some(channels) = self.incoming_connections.pop() {
            self.handle_new_client(channels);
        }

        // Handle client messages
        self.client_net_updates();

        // Update player positions, this is all the game logic
        for client in self.clients.values() {
            let player = self.players.get_mut(&client.player_id).unwrap();
            let move_dir = client.last_controls.move_direction;
            player.position += PLAYER_SPEED * Vec3::new(move_dir.x, 0., move_dir.y) * TICK_DT;
        }
    }

    fn handle_new_client(
        &mut self,
        (incoming_rx, outgoing_tx): (ClientMessageReceiver, ServerMessageSender),
    ) {
        let player_id = PlayerId::new(self.next_player_id);
        self.next_player_id += 1;
        self.players
            .insert(player_id, Player::new(self.player_spawn_point));

        let client_id = ClientId(self.next_client_id);
        self.next_client_id += 1;

        // Send level init packet
        let _ = outgoing_tx.blocking_send(
            net_types::InitLevel {
                blocks: self.blocks.clone(),
            }
            .into(),
        );

        self.clients.insert(
            client_id,
            Client {
                last_controls: net_types::Controls::default(),
                player_id,
                known_players: HashMap::new(),
                incoming_rx,
                outgoing_tx,
            },
        );

        tracing::info!("New client connected: {:?}", client_id);
    }

    fn client_net_updates(&mut self) {
        let mut disconnected = Vec::new();
        let live_players = self.players.keys().copied().collect::<HashSet<_>>();
        'client_loop: for (client_id, client) in self.clients.iter_mut() {
            while let Some(controls) = match client.incoming_rx.try_recv() {
                Ok(v) => Some(v),
                Err(e) => match e {
                    mpsc::error::TryRecvError::Empty => None,
                    mpsc::error::TryRecvError::Disconnected => {
                        disconnected.push(*client_id);
                        tracing::info!("Client disconnected: {:?}", client_id);
                        break 'client_loop;
                    }
                },
            } {
                client.last_controls = controls;
            }

            let known_players = client.known_players.keys().copied().collect::<HashSet<_>>();

            let new_players = live_players.difference(&known_players);
            let removed_players = known_players.difference(&live_players);

            // Add new players to this client
            for player_id in new_players {
                let player = self.players.get(player_id).unwrap();
                let _ = client.outgoing_tx.blocking_send(
                    net_types::AddPlayer {
                        id: *player_id,
                        position: player.position,
                    }
                    .into(),
                );
                client.known_players.insert(*player_id, player.position);
            }

            // Remove old players from this client
            for player_id in removed_players {
                let _ = client
                    .outgoing_tx
                    .blocking_send(net_types::RemovePlayer { id: *player_id }.into());
                client.known_players.remove(player_id);
            }

            // Update player positions for all known players
            for (player_id, known_position) in client.known_players.iter_mut() {
                let player = self.players.get(player_id).unwrap();
                if player.position != *known_position {
                    let _ = client.outgoing_tx.blocking_send(
                        net_types::UpdatePosition {
                            id: *player_id,
                            position: player.position,
                        }
                        .into(),
                    );
                    *known_position = player.position;
                }
            }
        }

        // Remove disconnected clients, and their associated players
        self.clients.retain(|client_id, client| {
            if disconnected.contains(client_id) {
                self.players.remove(&client.player_id);
                false
            } else {
                true
            }
        });
    }
}

pub const TICK_RATE: u32 = 60;
pub const TICK_DT: f32 = 1. / TICK_RATE as f32;

const PLAYER_SPEED: f32 = 5.;

#[derive(Clone, Copy, Debug)]
struct Player {
    position: glam::Vec3,
}

impl Player {
    pub fn new(position: glam::Vec3) -> Self {
        Self { position }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct ClientId(u64);

struct Client {
    // The last received controls
    // TODO(ll): Once we have prediction this should be a queue of inputs
    last_controls: net_types::Controls,

    // This client's player ID
    player_id: PlayerId,

    // The clients that the client is aware of, and their last known position
    known_players: HashMap<PlayerId, glam::Vec3>,

    // The packet channels for this client
    incoming_rx: Receiver<net_types::Controls>,
    outgoing_tx: Sender<net_types::ServerPacket>,
}

pub async fn start_client_listener(
    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,
) -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:8889").await?;

    while let Ok((stream, _)) = server.accept().await {
        let incoming_connections = incoming_connections.clone();
        tokio::spawn(async move {
            let addr = stream.peer_addr().expect("no peer address found");
            tracing::info!("Client connected from: {}", addr);

            let ws_stream = tokio_tungstenite::accept_async(stream)
                .await
                .expect("error during the websocket handshake");

            // Create channels for serialized messages
            let (incoming_tx, incoming_rx) = mpsc::channel(16);
            let (outgoing_tx, mut outgoing_rx) = mpsc::channel(16);

            incoming_connections.push((incoming_rx, outgoing_tx));

            // Spawn a task that handles networking and serialization
            let (mut write, mut read) = ws_stream.split();
            tokio::spawn(async move {
                loop {
                    select! {
                        // Handle incoming messages
                        message = read.next() => {
                            let Some(message) = message else {
                                // The stream has been closed, gracefully exit the task
                                break;
                            };

                            // Deserialize the message and pass it to the client's incoming channel
                            let controls = match message {
                                Ok(v) => match bincode::deserialize::<net_types::Controls>(&v.into_data()) {
                                    Ok(v) => v,
                                    Err(e) => {
                                        tracing::warn!("Error deserializing controls: {}", e);
                                        break;
                                    }
                                },
                                Err(e) => {
                                    tracing::warn!("Error receiving message: {}", e);
                                    break;
                                }
                            };

                            incoming_tx.send(controls).await.unwrap();
                        }
                        // Handle outgoing messages
                        message = outgoing_rx.recv() => {
                            let Some(message) = message else {
                                // The client has been dropped by the game server, gracefully exit the task
                                break;
                            };


                            let position_message =
                                bincode::serialize(&message).unwrap();
                            if let Err(e) = write.send(Message::Binary(position_message)).await {
                                tracing::info!("Error sending message: {}", e);
                                break;
                            }
                        }
                    }
                }
            });
        });
    }

    Ok(())
}

type ClientMessageReceiver = Receiver<net_types::Controls>;
type ServerMessageSender = Sender<net_types::ServerPacket>;

/// Generate a simple map for testing
fn generate_map(x: u32, z: u32) -> BlockGrid {
    let mut blocks = BlockGrid::new(x, 16, z);

    for x in 0..x {
        // Generate flat ground
        for y in 0..1 {
            for z in 0..z {
                if x == 0 || y == 0 || z == 0 || x == 127 || y == 127 || z == 63 {
                    blocks[[x, y, z].into()] = 1;
                }
            }
        }
    }

    blocks
}
