use {
    crate::game::PlayerState,
    anyhow::Result,
    crossbeam::queue::SegQueue,
    entities::{Anchor, EntityID, PlayerId},
    futures_util::{SinkExt, StreamExt},
    net_types::ClientPacket,
    std::{collections::HashMap, ops::Add, sync::Arc},
    tokio::{
        net::TcpListener,
        select,
        sync::mpsc::{self, Receiver, Sender},
    },
    tokio_tungstenite::tungstenite::Message,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClientId(u64);

impl Add<u64> for ClientId {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Self(self.0 + rhs)
    }
}

pub struct Client {
    // The last received controls
    // TODO(ll): Once we have prediction this should be a queue of inputs
    pub last_controls: net_types::Controls,

    // This client's player ID
    pub player_id: PlayerId,

    pub awareness: ClientAwareness,

    // The packet channels for this client
    pub incoming_rx: Receiver<net_types::ClientPacket>,
    pub outgoing_tx: Sender<net_types::ServerPacket>,
}

pub async fn start_client_listener(
    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,
) -> Result<()> {
    let server = TcpListener::bind("127.0.0.1:8889").await?;
    tracing::info!("WebSocket server started on ws://127.0.0.1:8889");

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
                            let message = match message {
                                Ok(message) => message,
                                Err(e) => {
                                    tracing::warn!("Error receiving message: {}", e);
                                    break;
                                }
                            };

                            // Bincode is currently broken, fall back to json for now.
                            // See: https://github.com/leetvr/hy/issues/189
                            let client_packet: ClientPacket = match serde_json::de::from_slice(
                                &message.into_data(),
                            ) {
                                Ok(v) => v,
                                Err(e) => {
                                    tracing::warn!("Error deserializing controls: {}", e);
                                    break;
                                }
                            };

                            incoming_tx.send(client_packet).await.unwrap();
                        }
                        // Handle outgoing messages
                        message = outgoing_rx.recv() => {
                            let Some(message) = message else {
                                // The client has been dropped by the game server, gracefully exit the task
                                break;
                            };

                            // Bincode is currently broken, fall back to json for now.
                            // See: https://github.com/leetvr/hy/issues/189
                            // let message =
                            //     bincode::serialize(&message).unwrap();
                            let message =
                                serde_json::ser::to_vec(&message).unwrap();
                            if let Err(e) = write.send(Message::Binary(message)).await {
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

pub type ClientMessageReceiver = Receiver<net_types::ClientPacket>;
pub type ServerMessageSender = Sender<net_types::ServerPacket>;

// All state that tracks the client's knowledge of the world
// This is used to check what updates the client needs to receive from the server to stay in sync
#[derive(Clone, Debug, Default)]
pub struct ClientAwareness {
    // The players that the client is aware of, and their last known position
    pub players: HashMap<PlayerId, ClientPlayerState>,

    // The scripted entities that the client is aware of, and their last known position
    pub entities: HashMap<EntityID, KnownEntityState>,
}

#[derive(Clone, Debug)]
pub struct KnownEntityState {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub anchor: Option<Anchor>,
}

// The state of a player as the client knows it
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientPlayerState {
    pub position: glam::Vec3,
    pub animation_state: String,
}

impl ClientPlayerState {
    pub fn new(state: &PlayerState) -> ClientPlayerState {
        ClientPlayerState {
            position: state.position,
            animation_state: state.animation_state.clone(),
        }
    }
}
