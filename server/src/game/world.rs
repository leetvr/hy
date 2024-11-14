use {
    anyhow::Result,
    blocks::{BlockGrid, BlockRegistry},
    entities::{EntityData, EntityPosition, EntityTypeRegistry, PlayerId},
    std::{
        collections::HashMap,
        path::{Path, PathBuf},
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

    pub fn anchor_entity(
        &mut self,
        entity_id: String,
        player_id: u64,
        anchor_name: String,
        offset: glam::Vec3,
        rotation: glam::Quat,
    ) {
        self.command_queue.push(WorldCommand::AnchorEntity {
            entity_id,
            player_id,
            anchor_name,
            offset,
            rotation,
        });
    }

    pub fn detach_entity(&mut self, entity_id: String, position: glam::Vec3) {
        self.command_queue.push(WorldCommand::DetachEntity {
            entity_id,
            position,
        });
    }

    pub fn apply_queued_updates(&mut self) {
        for command in self.command_queue.drain(..) {
            match command {
                WorldCommand::SpawnEntity(entity_id, entity_data) => {
                    self.entities.insert(entity_id, entity_data);
                }
                WorldCommand::DespawnEntity(entity_id) => {
                    self.entities.remove(&entity_id);
                }
                WorldCommand::AnchorEntity {
                    entity_id,
                    player_id,
                    anchor_name,
                    offset,
                    rotation,
                } => {
                    if let Some(entity) = self.entities.get_mut(&entity_id) {
                        entity.state.position = EntityPosition::Anchored {
                            player_id: PlayerId::new(player_id),
                            parent_anchor: anchor_name,
                            translation: offset,
                            rotation: rotation,
                        };
                    }
                }
                WorldCommand::DetachEntity {
                    entity_id,
                    position,
                } => {
                    if let Some(entity) = self.entities.get_mut(&entity_id) {
                        if let EntityPosition::Anchored { .. } = entity.state.position {
                            entity.state.position = EntityPosition::Absolute(position);
                        }
                    }
                }
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

enum WorldCommand {
    SpawnEntity(String, EntityData),
    DespawnEntity(String),
    AnchorEntity {
        entity_id: String,
        player_id: u64,
        anchor_name: String,
        offset: glam::Vec3,
        rotation: glam::Quat,
    },
    DetachEntity {
        entity_id: String,
        // When detaching an anchored entity it should also get a new absolute position
        position: glam::Vec3,
    },
}
