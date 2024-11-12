use {
    crate::game::{network::ClientPlayerState, player, PlayerState},
    entities::{EntityData, EntityID},
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

        // IMPORTANT: We need the client to forget any previous world state
        editor_client.awareness = Default::default();

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
                player::player_aabb_block_collisions(player.state.position, &self.world.blocks);

            player.state = js_context
                .get_player_next_state(&player.state, &client.last_controls, collisions)
                .await
                .unwrap();
        }

        // Update entities
        for entity in self.world.entities.values_mut() {
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
        let live_entities = self.world.entities.keys().copied().collect::<HashSet<_>>();
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
            sync_entities_to_client(&self.world.entities, &live_entities, client).await;
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
    live_entities: &HashSet<u64>,
    client: &mut Client,
) {
    let known_entities = client
        .awareness
        .entities
        .keys()
        .copied()
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
                    entity_id: *entity_id,
                    entity_data: entity.clone(),
                }
                .into(),
            )
            .await;
        client
            .awareness
            .entities
            .insert(*entity_id, entity.state.position);
    }

    // Remove old entities from this client
    for entity_id in removed_entities {
        let _ = client
            .outgoing_tx
            .send(
                net_types::RemoveEntity {
                    entity_id: *entity_id,
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
                        entity_id: *entity_id,
                        position: entity.state.position,
                    }
                    .into(),
                )
                .await;
            *known_position = entity.state.position;
        }
    }
}
