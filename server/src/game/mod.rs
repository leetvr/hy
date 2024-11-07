mod game_instance;
mod network;
mod world;

use {
    crate::game::network::{Client, ClientMessageReceiver, ServerMessageSender},
    crossbeam::queue::SegQueue,
    game_instance::GameInstance,
    network::ClientId,
    physics::PhysicsWorld,
    std::{fmt::Display, sync::Arc},
    world::World,
};

const WORLD_SIZE: i32 = 32;

pub struct GameServer {
    spawner: tokio::runtime::Handle,
    state: ServerState,
    incoming_connections: Arc<SegQueue<(ClientMessageReceiver, ServerMessageSender)>>,
}

impl GameServer {
    pub fn new(spawner: tokio::runtime::Handle) -> Self {
        let incoming_connections = Arc::new(SegQueue::new());

        spawner.spawn(network::start_client_listener(incoming_connections.clone()));

        // Roughly in the center of the map
        let player_spawn_point =
            glam::Vec3::new(WORLD_SIZE as f32 / 2., 16., WORLD_SIZE as f32 / 2.);

        // Load the world
        let world = World::load();

        // Set the initial state
        let initial_state = ServerState::Paused(GameInstance::new(world, player_spawn_point));

        Self {
            spawner,
            incoming_connections,
            state: initial_state,
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
            ServerState::Playing(instance) | ServerState::Paused(instance) => instance.tick(),
            _ => None,
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
    Editing(World, Client),
    Transitioning,
}

impl Display for ServerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            ServerState::Playing(_) => "Playing",
            ServerState::Paused(_) => "Paused",
            ServerState::Editing(_, _) => "Editing",
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
            (ServerState::Playing(mut instance), NextServerState::Editing(client_id)) => {
                if let Some(editor_client) = instance.clients.remove(&client_id) {
                    *self = ServerState::Editing(instance.world, editor_client);
                } else {
                    tracing::warn!(
                        "Can't transition to editing - client {client_id:?} does not exist"
                    );
                    *self = ServerState::Playing(instance)
                }
            }
            // Paused -> Playing
            (ServerState::Paused(instance), NextServerState::Playing) => {
                *self = ServerState::Playing(instance);
            }
            // Paused -> Editing
            (ServerState::Paused(mut instance), NextServerState::Editing(client_id)) => {
                if let Some(editor_client) = instance.clients.remove(&client_id) {
                    *self = ServerState::Editing(instance.world, editor_client);
                } else {
                    tracing::warn!(
                        "Can't transition to editing - client {client_id:?} does not exist"
                    );
                    *self = ServerState::Paused(instance)
                }
            }
            // Editing -> Playing
            (ServerState::Editing(world, editor_client), NextServerState::Playing) => {
                let instance = GameInstance::from_editor(world, editor_client);
                *self = ServerState::Playing(instance);
            }
            // Editing -> Paused
            (ServerState::Editing(world, editor_client), NextServerState::Paused) => {
                let instance = GameInstance::from_editor(world, editor_client);
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
    red_points: u32,
    blue_points: u32,
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
