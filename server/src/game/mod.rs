mod level;
mod network;

use {
    crate::game::network::{Client, ClientId, ClientMessageReceiver, ServerMessageSender},
    blocks::BlockGrid,
    crossbeam::queue::SegQueue,
    glam::Vec3,
    net_types::PlayerId,
    physics::{PhysicsCollider, PhysicsWorld},
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
    },
    tokio::sync::mpsc::{self},
};

pub struct GameServer {
    spawner: tokio::runtime::Handle,

    blocks: BlockGrid,
    player_spawn_point: glam::Vec3,

    next_client_id: ClientId,
    clients: HashMap<ClientId, Client>,

    next_player_id: u64,
    players: HashMap<PlayerId, Player>,

    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,

    physics_world: PhysicsWorld,
    _colliders: Vec<PhysicsCollider>,
}

impl GameServer {
    pub fn new(spawner: tokio::runtime::Handle) -> Self {
        let incoming_connections = Arc::new(SegQueue::new());

        spawner.spawn(network::start_client_listener(incoming_connections.clone()));

        let size = 32;
        let blocks = level::generate_map(size, size);
        // Roughly in the center of the map
        let player_spawn_point = glam::Vec3::new(size as f32 / 2., 16., size as f32 / 2.);

        let mut physics_world = PhysicsWorld::new();
        let mut colliders = Vec::new();

        level::bake_terrain_colliders(&mut physics_world, &blocks, &mut colliders);

        Self {
            spawner,
            blocks,
            player_spawn_point,
            next_client_id: Default::default(),
            clients: HashMap::new(),
            next_player_id: 0,
            players: HashMap::new(),
            incoming_connections,
            physics_world,
            _colliders: colliders,
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

        // Add forces/velocities
        for client in self.clients.values() {
            let player = self.players.get_mut(&client.player_id).unwrap();
            let move_dir = client.last_controls.move_direction;
            self.physics_world.set_velocity_piecewise(
                &player.body,
                Some(move_dir.x * PLAYER_SPEED),
                None,
                Some(-move_dir.y * PLAYER_SPEED),
            );
            if client.last_controls.jump {
                self.physics_world
                    .apply_impulse(&player.body, Vec3::new(0., JUMP_IMPULSE, 0.));
            }
        }

        // Step physics
        self.physics_world.step();

        // Read back positions
        for player in self.players.values_mut() {
            player.position = self.physics_world.get_position(&player.body);
        }
    }

    fn handle_new_client(
        &mut self,
        (incoming_rx, outgoing_tx): (ClientMessageReceiver, ServerMessageSender),
    ) {
        let player_id = PlayerId::new(self.next_player_id);
        self.next_player_id += 1;
        self.players.insert(
            player_id,
            Player::new(&mut self.physics_world, self.player_spawn_point),
        );

        let client_id = self.next_client_id;
        self.next_client_id = self.next_client_id + 1;

        // Send level init packet
        let _ = outgoing_tx.blocking_send(
            net_types::Init {
                blocks: self.blocks.clone(),
                client_player: player_id,
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
                if let Some(player) = self.players.remove(&client.player_id) {
                    // Make sure to remove the physics body
                    self.physics_world.remove_body(player.body);
                }
                false
            } else {
                true
            }
        });
    }
}

pub const TICK_RATE: u32 = 60;
pub const TICK_DT: f32 = 1. / TICK_RATE as f32;

const PLAYER_SPEED: f32 = 10.;
const JUMP_IMPULSE: f32 = 50.;

#[derive(Debug)]
struct Player {
    position: glam::Vec3,
    body: physics::PhysicsBody,
}

impl Player {
    pub fn new(physics_world: &mut PhysicsWorld, position: glam::Vec3) -> Self {
        let physics_body = physics_world.add_ball_body(position, 1.);
        Self {
            position,
            body: physics_body,
        }
    }
}
