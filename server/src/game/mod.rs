mod editor_instance;
mod game_instance;
mod network;
mod player;
mod world;

use {
    crate::{
        game::network::{ClientMessageReceiver, ServerMessageSender},
        js::JSContext,
    },
    blocks::BlockPos,
    crossbeam::queue::SegQueue,
    editor_instance::EditorInstance,
    game_instance::GameInstance,
    net_types::PlayerId,
    network::ClientId,
    physics::PhysicsWorld,
    serde::{Deserialize, Serialize},
    std::{
        fmt::Display,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    },
};

pub use world::World;

const WORLD_SIZE: i32 = 32;

pub struct GameServer {
    state: ServerState,
    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,
    storage_dir: PathBuf,
    js_context: JSContext,
}

impl GameServer {
    pub async fn new(storage_dir: impl Into<PathBuf>) -> Self {
        let incoming_connections = Arc::new(SegQueue::new());

        tokio::spawn(network::start_client_listener(incoming_connections.clone()));

        // Load the world
        let storage_dir: PathBuf = storage_dir.into();
        let world = Arc::new(Mutex::new(
            World::load(&storage_dir).expect("Failed to load world"),
        ));

        let game_instance = GameInstance::new(world.clone());

        tracing::info!("Starting JS context..");
        let script_root = storage_dir.join("dist/");
        let js_context = JSContext::new(
            &script_root,
            world.clone(),
            game_instance.physics_world.clone(),
        )
        .await
        .expect("Failed to load JS Context");

        // Set the initial state
        let initial_state = ServerState::Paused(game_instance);

        tracing::info!("Done!");

        Self {
            incoming_connections,
            state: initial_state,
            storage_dir,
            js_context,
        }
    }

    pub async fn tick(&mut self) {
        // Handle new connections
        while let Some(channels) = self.incoming_connections.pop() {
            match &mut self.state {
                ServerState::Playing(instance) | ServerState::Paused(instance) => {
                    instance.handle_new_client(channels).await
                }
                _ => {}
            }
        }

        // Tick
        let next_state = match &mut self.state {
            ServerState::Playing(instance) | ServerState::Paused(instance) => {
                instance.tick(&mut self.js_context).await
            }
            ServerState::Editing(instance) => instance.tick(&self.storage_dir),
            invalid => panic!("Invalid server state: {invalid}"),
        };

        // Do we need to transition to a different state?
        let Some(next_state) = next_state else { return };

        self.state.transition(&self.storage_dir, next_state).await;
    }
}

pub const TICK_RATE: u32 = 60;
pub const TICK_DT: f32 = 1. / TICK_RATE as f32;

enum ServerState {
    Playing(GameInstance),
    Paused(GameInstance),
    Editing(EditorInstance),
    Transitioning,
}

impl Display for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ServerState::Playing(_) => "Playing",
            ServerState::Paused(_) => "Paused",
            ServerState::Editing(_) => "Editing",
            ServerState::Transitioning => "Transitioning",
        };

        f.write_str(name)
    }
}

impl ServerState {
    // state machines, my beloved
    async fn transition(&mut self, storage_dir: impl AsRef<Path>, next_state: NextServerState) {
        // Take the current state so we can move it
        let current_state = std::mem::replace(self, ServerState::Transitioning);

        tracing::info!("Transitioning from {current_state} to {next_state}.");

        match (current_state, next_state) {
            // Playing -> Paused
            (ServerState::Playing(instance), NextServerState::Paused) => {
                *self = ServerState::Paused(instance);
            }
            // Playing -> Editing
            (ServerState::Playing(mut game_instance), NextServerState::Editing(client_id)) => {
                if let Some(editor_client) = game_instance.clients.remove(&client_id) {
                    // Reload world when switching to editing
                    let world = game_instance.world;
                    *world.lock().expect("Deadlock!") =
                        World::load(storage_dir).expect("couldn't load world");

                    let editor_instance = EditorInstance::from_transition(
                        world,
                        editor_client,
                        game_instance.physics_world,
                    )
                    .await;
                    *self = ServerState::Editing(editor_instance);
                } else {
                    tracing::warn!(
                        "Can't transition to editing - client {client_id:?} does not exist"
                    );
                    *self = ServerState::Playing(game_instance)
                }
            }
            // Paused -> Playing
            (ServerState::Paused(game_instance), NextServerState::Playing) => {
                *self = ServerState::Playing(game_instance);
            }
            // Paused -> Editing
            (ServerState::Paused(mut game_instance), NextServerState::Editing(client_id)) => {
                if let Some(editor_client) = game_instance.clients.remove(&client_id) {
                    // Reload world when switching to editing
                    let world = game_instance.world;
                    *world.lock().expect("Deadlock!") =
                        World::load(storage_dir).expect("couldn't load world");

                    let editor_instance = EditorInstance::from_transition(
                        world,
                        editor_client,
                        game_instance.physics_world,
                    )
                    .await;
                    *self = ServerState::Editing(editor_instance);
                } else {
                    tracing::warn!(
                        "Can't transition to editing - client {client_id:?} does not exist"
                    );
                    *self = ServerState::Paused(game_instance)
                }
            }
            // Editing -> Playing
            (ServerState::Editing(editor_instance), NextServerState::Playing) => {
                let instance = GameInstance::from_transition(editor_instance).await;
                *self = ServerState::Playing(instance);
            }
            // Editing -> Paused
            (ServerState::Editing(editor_instance), NextServerState::Paused) => {
                let instance = GameInstance::from_transition(editor_instance).await;
                *self = ServerState::Paused(instance);
            }
            // Invalid transition
            (current, invalid) => {
                tracing::warn!("Can't transition from {current} to {invalid}");
                *self = current
            }
        }
    }
}

enum NextServerState {
    Playing,
    Paused,
    Editing(ClientId), // the client that wants to edit
}

impl Display for NextServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            NextServerState::Playing => "Playing",
            NextServerState::Paused => "Paused",
            NextServerState::Editing(_) => "Editing",
        };

        f.write_str(name)
    }
}

#[derive(Default)]
struct GameState {
    _red_points: u32,
    _blue_points: u32,
}

#[derive(Debug)]
struct Player {
    state: PlayerState,
    body: physics::PhysicsBody,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    position: glam::Vec3,
    velocity: glam::Vec3,
    #[serde(rename = "animationState")]
    animation_state: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerCollision {
    // The block that the player collided with
    pub block: BlockPos,
    // The normal of the face that the player collided with
    pub normal: glam::Vec3,
    // The movement required to resolve the collision
    pub resolution: glam::Vec3,
}

impl Player {
    pub fn new(id: PlayerId, physics_world: &mut PhysicsWorld, position: glam::Vec3) -> Self {
        let physics_body =
            physics_world.add_player_body(id.inner(), position, glam::Vec3::new(0.5, 1.5, 0.5));
        Self {
            state: PlayerState {
                position,
                ..Default::default()
            },
            body: physics_body,
        }
    }
}
