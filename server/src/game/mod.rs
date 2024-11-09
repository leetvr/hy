mod editor_instance;
mod game_instance;
mod network;
mod world;

use {
    crate::{
        game::network::{ClientMessageReceiver, ServerMessageSender},
        js::JSContext,
    },
    crossbeam::queue::SegQueue,
    editor_instance::EditorInstance,
    game_instance::GameInstance,
    network::ClientId,
    physics::PhysicsWorld,
    std::{fmt::Display, path::PathBuf, sync::Arc},
    world::World,
};

const WORLD_SIZE: i32 = 32;

pub struct GameServer {
    spawner: tokio::runtime::Handle,
    state: ServerState,
    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,
    storage_dir: PathBuf,
    js_context: JSContext,
}

impl GameServer {
    pub fn new(spawner: tokio::runtime::Handle, storage_dir: impl Into<PathBuf>) -> Self {
        let incoming_connections = Arc::new(SegQueue::new());

        spawner.spawn(network::start_client_listener(incoming_connections.clone()));

        // Load the world
        let storage_dir: PathBuf = storage_dir.into();
        let world = World::load(&storage_dir).expect("Failed to load world");

        // Set the initial state
        let initial_state = ServerState::Paused(GameInstance::new(world));

        let mut player_script_path = storage_dir.clone();
        player_script_path.push("player.js");

        let js_context = spawner
            .block_on(JSContext::new(&player_script_path))
            .expect("Failed to load JS Context");

        Self {
            spawner,
            incoming_connections,
            state: initial_state,
            storage_dir,
            js_context,
        }
    }

    pub fn tick(&mut self) {
        let _handle = self.spawner.enter();

        // Handle new connections
        while let Some(channels) = self.incoming_connections.pop() {
            match &mut self.state {
                ServerState::Playing(instance) | ServerState::Paused(instance) => {
                    instance.handle_new_client(channels)
                }
                _ => {}
            }
        }

        // Tick
        let next_state = match &mut self.state {
            ServerState::Playing(instance) | ServerState::Paused(instance) => {
                self.spawner.block_on(instance.tick(&mut self.js_context))
            }
            ServerState::Editing(instance) => instance.tick(&self.storage_dir),
            invalid => panic!("Invalid server state: {invalid}"),
        };

        // Do we need to transition to a different state?
        let Some(next_state) = next_state else { return };

        self.state.transition(next_state);
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
    fn transition(&mut self, next_state: NextServerState) {
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
                    let editor_instance = EditorInstance::new(game_instance.world, editor_client);
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
                    let editor_instance = EditorInstance::new(game_instance.world, editor_client);
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
                let instance = GameInstance::from_editor(editor_instance);
                *self = ServerState::Playing(instance);
            }
            // Editing -> Paused
            (ServerState::Editing(editor_instance), NextServerState::Paused) => {
                let instance = GameInstance::from_editor(editor_instance);
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
