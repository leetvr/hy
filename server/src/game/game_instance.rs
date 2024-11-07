use std::collections::{HashMap, HashSet};

use glam::Vec3;
use net_types::PlayerId;
use tokio::sync::mpsc;

use super::{
    network::{Client, ClientId, ClientMessageReceiver, ServerMessageSender},
    world::World,
    GameState, Player, ServerState,
};

pub struct GameInstance {
    world: World,
    game_state: GameState,
    next_client_id: ClientId,
    clients: HashMap<ClientId, Client>,

    next_player_id: u64,
    players: HashMap<PlayerId, Player>,
    player_spawn_point: glam::Vec3,
}

impl GameInstance {
    pub fn new(world: World, player_spawn_point: glam::Vec3) -> Self {
        Self {
            world,
            player_spawn_point,
            game_state: Default::default(),
            next_client_id: Default::default(),
            clients: Default::default(),
            next_player_id: 0,
            players: Default::default(),
        }
    }

    pub fn tick(&mut self) {
        // Handle client messages
        self.client_net_updates();

        // Add forces/velocities
        for client in self.clients.values() {
            let player = self.players.get_mut(&client.player_id).unwrap();
            let move_dir = client.last_controls.move_direction;
            self.world.physics_world.set_velocity_piecewise(
                &player.body,
                Some(move_dir.x * PLAYER_SPEED),
                None,
                Some(-move_dir.y * PLAYER_SPEED),
            );
            if client.last_controls.jump {
                self.world
                    .physics_world
                    .apply_impulse(&player.body, Vec3::new(0., JUMP_IMPULSE, 0.));
            }
        }

        // Step physics
        self.world.physics_world.step();

        // Read back positions
        for player in self.players.values_mut() {
            player.position = self.world.physics_world.get_position(&player.body);
        }
    }

    pub fn handle_new_client(
        &mut self,
        (incoming_rx, outgoing_tx): (ClientMessageReceiver, ServerMessageSender),
    ) {
        let player_id = PlayerId::new(self.next_player_id);
        self.next_player_id += 1;
        self.players.insert(
            player_id,
            Player::new(&mut self.world.physics_world, self.player_spawn_point),
        );

        let client_id = self.next_client_id;
        self.next_client_id = self.next_client_id + 1;

        // Send level init packet
        let _ = outgoing_tx.blocking_send(
            net_types::Init {
                blocks: self.world.blocks.clone(),
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
                    self.world.physics_world.remove_body(player.body);
                }
                false
            } else {
                true
            }
        });
    }
}

const PLAYER_SPEED: f32 = 10.;
const JUMP_IMPULSE: f32 = 50.;
