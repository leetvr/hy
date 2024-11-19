use {
    super::PlayerState,
    crate::js::JSContext,
    anyhow::Result,
    blocks::{BlockGrid, BlockRegistry},
    entities::{Anchor, EntityData, EntityTypeRegistry, Interaction, PlayerId},
    net_types::PlaySound,
    physics::PhysicsWorld,
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Mutex},
    },
};

const BLOCKS_PATH: &str = "blocks.json";
const BLOCK_TYPES_PATH: &str = "block_types.json";
const ENTITIES_PATH: &str = "entities.json";
const ENTITY_TYPES_PATH: &str = "entity_types.json";

pub struct World {
    pub blocks: BlockGrid,
    pub block_registry: BlockRegistry,
    pub entities: HashMap<String, EntityData>, // key is EntityID
    pub entity_type_registry: EntityTypeRegistry,
    command_queue: Vec<WorldCommand>,
    pub player_data: HashMap<PlayerId, PlayerState>,
}

impl World {
    pub fn spawn_entity(&mut self, entity_id: String, entity_data: EntityData) {
        self.command_queue
            .push(WorldCommand::SpawnEntity(entity_id, entity_data));
    }

    pub fn despawn_entity(&mut self, entity_id: String) {
        self.command_queue
            .push(WorldCommand::DespawnEntity(entity_id));
    }

    pub fn anchor_entity(&mut self, entity_id: String, player_id: u64, anchor_name: String) {
        self.command_queue.push(WorldCommand::AnchorEntity {
            entity_id,
            player_id,
            anchor_name,
        });
    }

    pub fn detach_entity(&mut self, entity_id: String, position: glam::Vec3) {
        self.command_queue.push(WorldCommand::DetachEntity {
            entity_id,
            position,
        });
    }

    pub fn interact_entity(
        &mut self,
        entity_id: String,
        player_id: PlayerId,
        position: glam::Vec3,
        facing_angle: f32,
        custom_state: HashMap<String, serde_json::Value>,
    ) {
        self.command_queue.push(WorldCommand::InteractEntity {
            entity_id,
            player_id,
            position,
            facing_angle,
            custom_state,
        });
    }

    pub fn play_sound(&mut self, sound_id: String, position: glam::Vec3, volume: f32) {
        self.command_queue.push(WorldCommand::PlaySound {
            sound_id,
            position,
            volume,
        })
    }

    pub fn apply_queued_updates(
        &mut self,
        js_context: &mut JSContext,
        physics_world: Arc<Mutex<PhysicsWorld>>,
        queued_sounds: &mut Vec<PlaySound>,
    ) {
        for command in self.command_queue.drain(..) {
            match command {
                WorldCommand::SpawnEntity(entity_id, mut entity_data) => {
                    spawn_entity(
                        &mut entity_data,
                        js_context,
                        physics_world.clone(),
                        &self.entity_type_registry,
                    );
                    self.entities.insert(entity_id, entity_data);
                }
                WorldCommand::DespawnEntity(entity_id) => {
                    tracing::debug!("Despawning entity {entity_id}");
                    despawn_entity(&entity_id, physics_world.clone());
                    self.entities.remove(&entity_id);
                }
                WorldCommand::AnchorEntity {
                    entity_id,
                    player_id,
                    anchor_name,
                } => {
                    if let Some(entity) = self.entities.get_mut(&entity_id) {
                        entity.state.anchor = Some(Anchor {
                            player_id: PlayerId::new(player_id),
                            parent_anchor: anchor_name,
                        });
                    }
                }
                WorldCommand::DetachEntity {
                    entity_id,
                    position,
                } => {
                    if let Some(entity) = self.entities.get_mut(&entity_id) {
                        entity.state.position = position;
                        entity.state.anchor = None;
                    }
                }
                WorldCommand::InteractEntity {
                    entity_id,
                    player_id,
                    position,
                    facing_angle,
                    custom_state,
                } => {
                    if let Some(entity) = self.entities.get_mut(&entity_id) {
                        entity.state.interactions.push(Interaction {
                            player_id,
                            position,
                            facing_angle,
                            custom_state,
                        });
                    }
                }
                WorldCommand::PlaySound {
                    sound_id,
                    position,
                    volume,
                } => queued_sounds.push(PlaySound {
                    sound_id,
                    position,
                    volume,
                }),
            }
        }
    }

    pub fn load(storage_dir: impl AsRef<Path>) -> Result<Self> {
        let blocks_path = storage_dir.as_ref().join(BLOCKS_PATH);
        let blocks = serde_json::from_slice(&std::fs::read(blocks_path)?)?;

        let block_types_path = storage_dir.as_ref().join(BLOCK_TYPES_PATH);
        let block_registry = serde_json::from_slice(&std::fs::read(&block_types_path)?)?;

        let entities_path = storage_dir.as_ref().join(ENTITIES_PATH);
        let entities = serde_json::from_slice(&std::fs::read(entities_path)?)?;

        let entity_types_path = storage_dir.as_ref().join(ENTITY_TYPES_PATH);
        let entity_type_registry = serde_json::from_slice(&std::fs::read(entity_types_path)?)?;

        Ok(Self {
            blocks,
            block_registry,
            entities,
            entity_type_registry,
            command_queue: Vec::new(),
            player_data: HashMap::new(),
        })
    }

    pub fn save(&mut self, storage_dir: &PathBuf) -> anyhow::Result<()> {
        // Save blocks
        let blocks_path = storage_dir.join(BLOCKS_PATH);
        let blocks = serde_json::to_string(&self.blocks)?;
        std::fs::write(blocks_path, blocks)?;

        // Save entities
        let entities_path = storage_dir.join(ENTITIES_PATH);
        let entities = serde_json::to_string(&self.entities)?;
        std::fs::write(entities_path, entities)?;
        Ok(())
    }
}

pub fn spawn_entity(
    entity_data: &mut EntityData,
    js_context: &mut JSContext,
    physics_world: Arc<Mutex<PhysicsWorld>>,
    entity_type_registry: &EntityTypeRegistry,
) {
    // Call the entity's onSpawn function, if it has one
    js_context.spawn_entity(entity_data);
    physics_world
        .lock()
        .expect("Deadlock!")
        .spawn_entity(entity_data, entity_type_registry);
}

fn despawn_entity(entity_id: &str, physics_world: Arc<Mutex<PhysicsWorld>>) {
    physics_world
        .lock()
        .expect("Deadlock!")
        .despawn_entity(entity_id)
}

enum WorldCommand {
    SpawnEntity(String, EntityData),
    DespawnEntity(String),
    AnchorEntity {
        entity_id: String,
        player_id: u64,
        anchor_name: String,
    },
    DetachEntity {
        entity_id: String,
        position: glam::Vec3,
    },
    InteractEntity {
        entity_id: String,
        player_id: PlayerId,
        position: glam::Vec3,
        facing_angle: f32,
        custom_state: HashMap<String, serde_json::Value>,
    },
    PlaySound {
        sound_id: String,
        position: glam::Vec3,
        volume: f32,
    },
}
