use std::{
    mem,
    sync::{Arc, Mutex},
};

use {
    crate::game::{network::ClientPlayerState, player, PlayerState},
    entities::{EntityData, EntityID},
    std::collections::{HashMap, HashSet},
};

use blocks::{BlockGrid, BlockPos, EMPTY_BLOCK};
use glam::Vec3;
use net_types::{ClientShouldSwitchMode, PlayerId};
use physics::{PhysicsCollider, PhysicsWorld};
use tokio::sync::mpsc;

use crate::js::JSContext;

use super::{
    editor_instance::EditorInstance,
    network::{Client, ClientId, ClientMessageReceiver, ServerMessageSender},
    world::World,
    GameState, NextServerState, Player, WORLD_SIZE,
};

pub struct GameInstance {
    pub world: Arc<Mutex<World>>,
    _game_state: GameState,
    next_client_id: ClientId,
    pub clients: HashMap<ClientId, Client>,

    pub physics_world: Arc<Mutex<PhysicsWorld>>,
    pub colliders: Vec<PhysicsCollider>,
    next_player_id: u64,
    pub players: HashMap<PlayerId, Player>,
    player_spawn_point: glam::Vec3,
}

impl GameInstance {
    pub fn new(world: Arc<Mutex<World>>) -> Self {
        // Roughly in the center of the map
        let player_spawn_point =
            glam::Vec3::new(WORLD_SIZE as f32 / 2., 16., WORLD_SIZE as f32 / 2.);

        let mut physics_world = PhysicsWorld::new();
        let mut colliders = Vec::new();

        {
            let world = world.lock().expect("DEADLOCK!!");
            bake_terrain_colliders(&mut physics_world, &world.blocks, &mut colliders);
        }

        let physics_world = Arc::new(Mutex::new(physics_world));

        Self {
            world,
            player_spawn_point,
            physics_world,
            colliders,
            _game_state: Default::default(),
            next_client_id: Default::default(),
            clients: Default::default(),
            next_player_id: 0,
            players: Default::default(),
        }
    }

    pub async fn from_transition(editor_instance: EditorInstance) -> Self {
        let EditorInstance {
            world,
            mut editor_client,
            physics_world,
        } = editor_instance;
        let mut game_instance = GameInstance::new(world.clone());

        let world = world.lock().expect("DEADLOCK!!");
        let mut physics_world = physics_world.lock().expect("Deadlock");

        {
            bake_terrain_colliders(
                &mut physics_world,
                &world.blocks,
                &mut game_instance.colliders,
            );
        }

        // IMPORTANT: We need the client to forget any previous world state
        editor_client.awareness = Default::default();

        // Create a player for the editor client
        let new_player_id = PlayerId::new(game_instance.next_player_id);
        game_instance.next_player_id += 1;
        game_instance.players.insert(
            new_player_id,
            Player::new(
                new_player_id,
                &mut physics_world,
                game_instance.player_spawn_point,
            ),
        );

        // Set the player ID on the editor client
        editor_client.player_id = new_player_id;

        let client_id = game_instance.next_client_id;
        game_instance.next_client_id = game_instance.next_client_id + 1;

        // IMPORTANT: Send switch mode packet
        let _ = editor_client
            .outgoing_tx
            .send(
                net_types::ServerPacket::ClientShouldSwitchMode(ClientShouldSwitchMode::Play {
                    new_player_id,
                })
                .into(),
            )
            .await;

        // Release the lock on physics_world
        drop(physics_world);

        game_instance.clients.insert(client_id, editor_client);
        game_instance
    }

    pub async fn tick(&mut self, js_context: &mut JSContext) -> Option<NextServerState> {
        // Handle client messages
        let maybe_next_state = self.client_net_updates().await;

        // Update players
        for client in self.clients.values() {
            let player = self.players.get_mut(&client.player_id).unwrap();
            let collisions = {
                let world = self.world.lock().expect("Deadlock!");
                player::player_aabb_block_collisions(player.state.position, &world.blocks)
            };

            player.state = js_context
                .get_player_next_state(
                    client.player_id,
                    &player.state,
                    &client.last_controls,
                    collisions,
                )
                .await
                .unwrap();

            // Update Rapier
            {
                let mut physics_world = self.physics_world.lock().expect("Deadlock!");
                physics_world.set_velocity_and_position(
                    &player.body,
                    player.state.velocity,
                    player.state.position,
                );
            }
        }

        // Update entities
        let entity_data = {
            let world = self.world.lock().expect("Deadlock!");
            world
                .entities
                .iter()
                .map(|(entity_id, entity)| (entity_id.clone(), entity.entity_type))
                .collect::<Vec<_>>()
        };

        for (entity_id, entity_type_id) in entity_data {
            js_context
                .run_script_for_entity(&entity_id, entity_type_id)
                .await
                .unwrap();
        }

        // Step physics
        {
            self.physics_world.lock().expect("Deadlock!").step();
        }

        maybe_next_state
    }

    pub async fn handle_new_client(
        &mut self,
        (incoming_rx, outgoing_tx): (ClientMessageReceiver, ServerMessageSender),
    ) {
        let player_id = PlayerId::new(self.next_player_id);
        self.next_player_id += 1;
        {
            let mut physics_world = self.physics_world.lock().expect("Deadlock!");
            self.players.insert(
                player_id,
                Player::new(player_id, &mut physics_world, self.player_spawn_point),
            );
        }

        let client_id = self.next_client_id;
        self.next_client_id = self.next_client_id + 1;

        let world = &self.world.lock().expect("Deadlock!");

        // Send world init packet
        let _ = outgoing_tx
            .send(
                net_types::Init {
                    blocks: world.blocks.clone(),
                    block_registry: world.block_registry.clone(),
                    entities: world.entities.clone(),
                    entity_type_registry: world.entity_type_registry.clone(),
                    client_player: player_id,
                }
                .into(),
            )
            .await;

        self.clients.insert(
            client_id,
            Client {
                last_controls: net_types::Controls::default(),
                player_id,
                awareness: Default::default(),
                incoming_rx,
                outgoing_tx,
            },
        );

        tracing::info!("New client connected: {:?}", client_id);
    }

    async fn client_net_updates(&mut self) -> Option<NextServerState> {
        let mut disconnected = Vec::new();
        let mut maybe_next_state = None;
        let live_players = self.players.keys().copied().collect::<HashSet<_>>();
        let world = self.world.lock().expect("Deadlock!");
        let mut physics_world = self.physics_world.lock().expect("Deadlock!");

        let live_entities = world.entities.keys().cloned().collect::<HashSet<_>>();
        'client_loop: for (client_id, client) in self.clients.iter_mut() {
            while let Some(packet) = match client.incoming_rx.try_recv() {
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
                match packet {
                    net_types::ClientPacket::Controls(controls) => {
                        client.last_controls = controls;
                    }
                    net_types::ClientPacket::Start => {
                        maybe_next_state = Some(NextServerState::Playing)
                    }
                    net_types::ClientPacket::Pause => {
                        maybe_next_state = Some(NextServerState::Paused)
                    }
                    net_types::ClientPacket::Edit => {
                        maybe_next_state = Some(NextServerState::Editing(*client_id))
                    }
                    _ => {}
                }
            }

            sync_players_to_client(&self.players, &live_players, client).await;
            sync_entities_to_client(&world.entities, &live_entities, client).await;
        }

        // Remove disconnected clients, and their associated players
        self.clients.retain(|client_id, client| {
            if disconnected.contains(client_id) {
                if let Some(player) = self.players.remove(&client.player_id) {
                    // Make sure to remove the physics body
                    physics_world.remove_body(player.body);
                }
                false
            } else {
                true
            }
        });

        // If we need to transition to a new state, return that
        maybe_next_state
    }
}

async fn sync_players_to_client(
    players: &HashMap<PlayerId, Player>,
    live_players: &HashSet<PlayerId>,
    client: &mut Client,
) {
    let known_players = client
        .awareness
        .players
        .keys()
        .copied()
        .collect::<HashSet<_>>();

    let new_players = live_players.difference(&known_players);
    let removed_players = known_players.difference(live_players);

    // Add new players to this client
    for player_id in new_players {
        let player = players.get(player_id).unwrap();
        let _ = client
            .outgoing_tx
            .send(
                net_types::AddPlayer {
                    id: *player_id,
                    position: player.state.position,
                    animation_state: player.state.animation_state.clone(),
                }
                .into(),
            )
            .await;
        client
            .awareness
            .players
            .insert(*player_id, ClientPlayerState::new(&player.state));
    }

    // Remove old players from this client
    for player_id in removed_players {
        let _ = client
            .outgoing_tx
            .send(net_types::RemovePlayer { id: *player_id }.into())
            .await;
        client.awareness.players.remove(player_id);
    }

    // Update player positions for all known players
    // TODO: Update sates instead?
    for (player_id, known_state) in client.awareness.players.iter_mut() {
        let player = players.get(player_id).unwrap();
        if let Some(update) = player_update(*player_id, known_state, &player.state) {
            let _ = client.outgoing_tx.send(update.into()).await;
            *known_state = ClientPlayerState::new(&player.state);
        }
    }
}

fn player_update(
    id: PlayerId,
    last_state: &ClientPlayerState,
    current_state: &PlayerState,
) -> Option<net_types::UpdatePlayer> {
    let animation_change = if last_state.animation_state != current_state.animation_state {
        Some(current_state.animation_state.clone())
    } else {
        None
    };
    if animation_change.is_some() || last_state.position != current_state.position {
        Some(net_types::UpdatePlayer {
            id,
            position: current_state.position,
            animation_state: animation_change,
        })
    } else {
        None
    }
}

async fn sync_entities_to_client(
    entities: &HashMap<EntityID, EntityData>,
    live_entities: &HashSet<String>,
    client: &mut Client,
) {
    let known_entities = client
        .awareness
        .entities
        .keys()
        .cloned()
        .collect::<HashSet<_>>();

    let new_entities = live_entities.difference(&known_entities);
    let removed_entities = known_entities.difference(live_entities);

    // Add new entities to this client
    for entity_id in new_entities {
        let entity = entities.get(entity_id).unwrap();
        let _ = client
            .outgoing_tx
            .send(
                net_types::AddEntity {
                    entity_id: entity_id.clone(),
                    entity_data: entity.clone(),
                }
                .into(),
            )
            .await;
        client
            .awareness
            .entities
            .insert(entity_id.clone(), entity.state.position);
    }

    // Remove old entities from this client
    for entity_id in removed_entities {
        let _ = client
            .outgoing_tx
            .send(
                net_types::RemoveEntity {
                    entity_id: entity_id.clone(),
                }
                .into(),
            )
            .await;
        client.awareness.entities.remove(entity_id);
    }

    // Update client's entity positions for all known entities
    for (entity_id, known_position) in &mut client.awareness.entities {
        let entity = entities.get(entity_id).unwrap();
        if entity.state.position != *known_position {
            let _ = client
                .outgoing_tx
                .send(
                    net_types::UpdateEntity {
                        entity_id: entity_id.clone(),
                        position: entity.state.position,
                    }
                    .into(),
                )
                .await;
            *known_position = entity.state.position;
        }
    }
}

/// Rebuilds the terrain colliders from the block grid
///
/// This builds trimesh colliders, two for each layer along each axis: X+, X-, Y+, Y-, Z+, Z-
pub fn bake_terrain_colliders(
    physics_world: &mut PhysicsWorld,
    blocks: &BlockGrid,
    colliders: &mut Vec<PhysicsCollider>,
) {
    // Remove old colliders
    for collider in colliders.drain(..) {
        physics_world.remove_collider(collider);
    }

    // Vertices can be shared between many faces, store indices for each unique vertex
    let mut vert_indices = HashMap::new();
    let mut last_vert_index: u32 = 0;

    let mut layer_meshes = Vec::new();
    let size = blocks.size();

    for axis in [Axis::X, Axis::Y, Axis::Z] {
        let forward_offset = match axis {
            Axis::X => BlockPos::new(1, 0, 0),
            Axis::Y => BlockPos::new(0, 1, 0),
            Axis::Z => BlockPos::new(0, 0, 1),
        };
        let (layers, rows, cols) = match axis {
            Axis::X => (size.0, size.1, size.2),
            Axis::Y => (size.1, size.0, size.2),
            Axis::Z => (size.2, size.0, size.1),
        };

        for layer_pos in 0..layers {
            // We generate 2 meshes for each axis, one for the front face and one for the back face
            let mut front_mesh = Vec::new();
            let mut back_mesh = Vec::new();

            for row in 0..rows {
                for col in 0..cols {
                    let mut pos = BlockPos {
                        x: layer_pos,
                        y: row,
                        z: col,
                    };
                    match axis {
                        Axis::X => {}
                        Axis::Y => mem::swap(&mut pos.x, &mut pos.y),
                        Axis::Z => mem::swap(&mut pos.x, &mut pos.z),
                    }

                    if blocks.get(pos).copied().unwrap_or(EMPTY_BLOCK) == EMPTY_BLOCK {
                        // Empty blocks have no collider
                        continue;
                    }

                    // Block has a collider in the front if there is no block in front of it
                    let front_block = if layer_pos + 1 < layers {
                        blocks
                            .get(pos + forward_offset)
                            .copied()
                            .unwrap_or(EMPTY_BLOCK)
                    } else {
                        EMPTY_BLOCK
                    };
                    if front_block == EMPTY_BLOCK {
                        let vert_indices = axis_face_vertices(axis).map(|vertex| {
                            let vertex_pos = pos + vertex + forward_offset;

                            // Get the unique index for this vertex
                            *vert_indices.entry(vertex_pos).or_insert_with(|| {
                                let i = last_vert_index;
                                last_vert_index += 1;
                                i
                            })
                        });

                        // Add face to mesh, 2 triangles
                        front_mesh.push([vert_indices[0], vert_indices[1], vert_indices[2]]);
                        front_mesh.push([vert_indices[0], vert_indices[2], vert_indices[3]]);
                    }

                    // Block has a collider in the back if there is no block behind it
                    let back_block = if layer_pos > 0 {
                        blocks
                            .get(pos - forward_offset)
                            .copied()
                            .unwrap_or(EMPTY_BLOCK)
                    } else {
                        EMPTY_BLOCK
                    };
                    if back_block == EMPTY_BLOCK {
                        let vert_indices = axis_face_vertices(axis).map(|vertex| {
                            let vertex_pos = pos + vertex;

                            *vert_indices.entry(vertex_pos).or_insert_with(|| {
                                let i = last_vert_index;
                                last_vert_index += 1;
                                i
                            })
                        });

                        // Add face to mesh, 2 triangles
                        back_mesh.push([vert_indices[0], vert_indices[1], vert_indices[2]]);
                        back_mesh.push([vert_indices[0], vert_indices[2], vert_indices[3]]);
                    }
                }
            }

            if front_mesh.len() > 0 {
                layer_meshes.push(front_mesh);
            }
            if back_mesh.len() > 0 {
                layer_meshes.push(back_mesh);
            }
        }
    }

    // Invert the vertices map, putting the vertices in a vec where the indices correspond to the
    // indices generated for each layer mesh
    let mut vertices = vec![Vec3::ZERO; vert_indices.len() as usize];
    for (vertex, index) in vert_indices {
        vertices[index as usize] = Vec3::new(vertex.x as f32, vertex.y as f32, vertex.z as f32);
    }
    tracing::info!(
        "Generating trimesh colliders from {} unique vertices",
        vertices.len()
    );

    for layer_mesh in layer_meshes {
        let collider =
            physics_world.add_trimesh_collider(vertices.iter().copied(), layer_mesh.into_iter());
        colliders.push(collider);
    }
}

/// Get 4 vertices for a block's face aligned along the specified axis
fn axis_face_vertices(axis: Axis) -> [BlockPos; 4] {
    match axis {
        Axis::X => [
            BlockPos::new(0, 0, 0),
            BlockPos::new(0, 0, 1),
            BlockPos::new(0, 1, 1),
            BlockPos::new(0, 1, 0),
        ],
        Axis::Y => [
            BlockPos::new(0, 0, 0),
            BlockPos::new(0, 0, 1),
            BlockPos::new(1, 0, 1),
            BlockPos::new(1, 0, 0),
        ],
        Axis::Z => [
            BlockPos::new(0, 0, 0),
            BlockPos::new(1, 0, 0),
            BlockPos::new(1, 1, 0),
            BlockPos::new(0, 1, 0),
        ],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Axis {
    X,
    Y,
    Z,
}
