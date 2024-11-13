use {
    anyhow::Result,
    blocks::{BlockGrid, BlockRegistry},
    entities::{EntityData, EntityTypeRegistry},
    std::{
        collections::HashMap,
        path::Path,
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
}

impl World {
    pub fn load(storage_dir: impl AsRef<Path>) -> Result<Arc<Mutex<Self>>> {
        let blocks_path = storage_dir.as_ref().join(BLOCKS_PATH);
        let blocks = serde_json::from_slice(&std::fs::read(blocks_path)?)?;

        let block_types_path = storage_dir.as_ref().join(BLOCK_TYPES_PATH);
        let block_registry = serde_json::from_slice(&std::fs::read(&block_types_path)?)?;

        let entities_path = storage_dir.as_ref().join(ENTITIES_PATH);
        let entities = serde_json::from_slice(&std::fs::read(entities_path)?)?;

        let entity_types_path = storage_dir.as_ref().join(ENTITY_TYPES_PATH);
        let entity_type_registry = serde_json::from_slice(&std::fs::read(entity_types_path)?)?;

        Ok(Arc::new(Mutex::new(Self {
            blocks,
            block_registry,
            entities,
            entity_type_registry,
        })))
    }

    pub fn save(&mut self, storage_dir: impl AsRef<Path>) -> anyhow::Result<()> {
        let blocks_path = storage_dir.as_ref().join(BLOCKS_PATH);
        let blocks = serde_json::to_string(&self.blocks)?;
        std::fs::write(blocks_path, blocks)?;
        Ok(())
    }
}
