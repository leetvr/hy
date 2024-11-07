mod game_instance;
mod network;
mod world;

use {
    crate::game::network::{Client, ClientMessageReceiver, ServerMessageSender},
    crossbeam::queue::SegQueue,
    game_instance::GameInstance,
    physics::PhysicsWorld,
    std::sync::Arc,
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
        match &mut self.state {
            ServerState::Playing(instance) | ServerState::Paused(instance) => instance.tick(),
            _ => {}
        }
    }
}

pub const TICK_RATE: u32 = 60;
pub const TICK_DT: f32 = 1. / TICK_RATE as f32;

enum ServerState {
    Playing(GameInstance),
    Paused(GameInstance),
    Editing(World, Client),
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
