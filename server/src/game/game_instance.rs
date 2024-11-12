use {
    crate::game::PlayerCollision,
    blocks::{BlockGrid, BlockPos, EMPTY_BLOCK},
    std::collections::{HashMap, HashSet},
};

use net_types::PlayerId;
use tokio::sync::mpsc;

use crate::js::JSContext;

use super::{
    editor_instance::EditorInstance,
    network::{Client, ClientId, ClientMessageReceiver, ServerMessageSender},
    world::World,
    GameState, NextServerState, Player, WORLD_SIZE,
};

pub struct GameInstance {
    pub world: World,
    _game_state: GameState,
    next_client_id: ClientId,
    pub clients: HashMap<ClientId, Client>,

    next_player_id: u64,
    players: HashMap<PlayerId, Player>,
    player_spawn_point: glam::Vec3,
}

impl GameInstance {
    pub fn new(world: World) -> Self {
        // Roughly in the center of the map
        let player_spawn_point =
            glam::Vec3::new(WORLD_SIZE as f32 / 2., 16., WORLD_SIZE as f32 / 2.);

        Self {
            world,
            player_spawn_point,
            _game_state: Default::default(),
            next_client_id: Default::default(),
            clients: Default::default(),
            next_player_id: 0,
            players: Default::default(),
        }
    }

    pub async fn from_editor(editor_instance: EditorInstance) -> Self {
        let EditorInstance {
            world,
            mut editor_client,
        } = editor_instance;
        let mut game_instance = GameInstance::new(world);

        // IMPORTANT: We need the client to forget about any players it's seen
        editor_client.known_players.clear();

        // Create a player for the editor client
        let player_id = PlayerId::new(game_instance.next_player_id);
        game_instance.next_player_id += 1;
        game_instance.players.insert(
            player_id,
            Player::new(
                &mut game_instance.world.physics_world,
                game_instance.player_spawn_point,
            ),
        );

        // Set the player ID on the editor client
        editor_client.player_id = player_id;

        let client_id = game_instance.next_client_id;
        game_instance.next_client_id = game_instance.next_client_id + 1;

        // Send reset packet
        let _ = editor_client
            .outgoing_tx
            .send(
                net_types::Reset {
                    new_client_player: player_id,
                }
                .into(),
            )
            .await;

        game_instance.clients.insert(client_id, editor_client);
        game_instance
    }

    pub async fn tick(&mut self, js_context: &mut JSContext) -> Option<NextServerState> {
        // Handle client messages
        let maybe_next_state = self.client_net_updates().await;

        // Update players
        for client in self.clients.values() {
            let player = self.players.get_mut(&client.player_id).unwrap();
            let collisions =
                player_aabb_block_collisions(player.state.position, &self.world.blocks);

            player.state = js_context
                .get_player_next_state(&player.state, &client.last_controls, collisions)
                .await
                .unwrap();
        }

        // Update entities
        for entity in self.world.entities.iter_mut() {
            entity.state = js_context.get_entity_next_state(entity).await.unwrap();
        }

        // Step physics
        self.world.physics_world.step();

        maybe_next_state
    }

    pub async fn handle_new_client(
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

        let world = &self.world;

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
                known_players: HashMap::new(),
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

            let known_players = client.known_players.keys().copied().collect::<HashSet<_>>();

            let new_players = live_players.difference(&known_players);
            let removed_players = known_players.difference(&live_players);

            // Add new players to this client
            for player_id in new_players {
                let player = self.players.get(player_id).unwrap();
                let _ = client
                    .outgoing_tx
                    .send(
                        net_types::AddPlayer {
                            id: *player_id,
                            position: player.state.position,
                        }
                        .into(),
                    )
                    .await;
                client
                    .known_players
                    .insert(*player_id, player.state.position);
            }

            // Remove old players from this client
            for player_id in removed_players {
                let _ = client
                    .outgoing_tx
                    .send(net_types::RemovePlayer { id: *player_id }.into())
                    .await;
                client.known_players.remove(player_id);
            }

            // Update player positions for all known players
            // TODO: Update sates instead?
            for (player_id, known_position) in client.known_players.iter_mut() {
                let player = self.players.get(player_id).unwrap();
                if player.state.position != *known_position {
                    let _ = client
                        .outgoing_tx
                        .send(
                            net_types::UpdatePosition {
                                id: *player_id,
                                position: player.state.position,
                            }
                            .into(),
                        )
                        .await;
                    *known_position = player.state.position;
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

        // If we need to transition to a new state, return that
        maybe_next_state
    }
}

fn player_aabb_block_collisions(position: glam::Vec3, blocks: &BlockGrid) -> Vec<PlayerCollision> {
    let player_aabb = aabb_for_player(position);

    let mut collisions = Vec::new();
    for BlockPos { x, y, z } in aabb_colliding_blocks(blocks, &player_aabb) {
        // Collect the 6 possible collisions and the way to resolve getting out of the block
        collisions.extend(
            [
                (glam::Vec3::X, player_aabb.min.x - (x + 1) as f32),
                (-glam::Vec3::X, x as f32 - player_aabb.max.x),
                (glam::Vec3::Y, player_aabb.min.y - (y + 1) as f32),
                (-glam::Vec3::Y, y as f32 - player_aabb.max.y),
                (glam::Vec3::Z, player_aabb.min.z - (z + 1) as f32),
                (-glam::Vec3::Z, z as f32 - player_aabb.max.z),
            ]
            .into_iter()
            .map(|(normal, dist)| PlayerCollision {
                block: BlockPos::new(x, y, z),
                normal,
                resolution: normal * dist.abs() * 1.1,
            })
            .filter(|collision| {
                // Filter out collisions where the resolution would make the player
                // collide even more. We want less collisions, not more.
                if aabb_colliding_blocks(blocks, &aabb_for_player(position + collision.resolution))
                    .next()
                    .is_none()
                {
                    true
                } else {
                    false
                }
            }),
        );
    }
    collisions
}

fn aabb_colliding_blocks<'a>(
    blocks: &'a BlockGrid,
    aabb: &'a AABB,
) -> impl Iterator<Item = BlockPos> + 'a {
    let min = aabb.min_block_pos();
    let max = aabb.max_block_pos();

    // Broad phase, all the blocks we could possibly collide with
    let collidable_blocks = (min.x..=max.x).flat_map(move |x| {
        (min.y..=max.y).flat_map(move |y| {
            (min.z..=max.z).filter_map(move |z| {
                let pos = BlockPos::new(x, y, z);
                if blocks
                    .get(BlockPos::new(x, y, z))
                    .cloned()
                    .map(|b| b != EMPTY_BLOCK)
                    // Out of bounds is solid
                    .unwrap_or(true)
                {
                    Some(pos)
                } else {
                    None
                }
            })
        })
    });

    // Narrow phase, only the blocks we actually collide with
    collidable_blocks.filter(|pos| {
        if aabb.min.x > pos.x as f32 + 1.
            || aabb.max.x < pos.x as f32
            || aabb.min.y > pos.y as f32 + 1.
            || aabb.max.y < pos.y as f32
            || aabb.min.z > pos.z as f32 + 1.
            || aabb.max.z < pos.z as f32
        {
            false
        } else {
            true
        }
    })
}

fn aabb_for_player(player_pos: glam::Vec3) -> AABB {
    let size = glam::Vec3::new(0.5, 1.5, 0.5);
    let min = player_pos - size / 2.;
    let max = player_pos + size / 2.;
    AABB { min, max }
}

struct AABB {
    min: glam::Vec3,
    max: glam::Vec3,
}

impl AABB {
    fn min_block_pos(&self) -> BlockPos {
        BlockPos::new(
            self.min.x.floor() as u32,
            self.min.y.floor() as u32,
            self.min.z.floor() as u32,
        )
    }

    fn max_block_pos(&self) -> BlockPos {
        BlockPos::new(
            self.max.x.floor() as u32,
            self.max.y.floor() as u32,
            self.max.z.floor() as u32,
        )
    }
}
