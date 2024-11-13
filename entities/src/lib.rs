use {
    serde::{Deserialize, Serialize},
    tsify::Tsify,
    wasm_bindgen::prelude::wasm_bindgen,
};
pub type EntityTypeID = u8;
pub type EntityID = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityData {
    pub name: String,
    pub entity_type: EntityTypeID,
    pub model_path: String,
    pub state: EntityState,
}

#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Tsify)]
pub struct EntityType {
    id: EntityTypeID,
    name: String,
    script_path: String,
    default_model_path: String,
}

impl EntityType {
    pub fn id(&self) -> EntityTypeID {
        self.id
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }
}

impl EntityType {
    pub fn script_path(&self) -> &str {
        &self.script_path
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityState {
    pub position: glam::Vec3,
    pub velocity: glam::Vec3,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default, Tsify)]
pub struct EntityTypeRegistry {
    entity_types: Vec<EntityType>,
}

impl EntityTypeRegistry {
    pub fn entity_types(&self) -> Vec<EntityType> {
        self.entity_types.clone()
    }
}

impl EntityTypeRegistry {
    pub fn get(&self, entity_type_id: EntityTypeID) -> Option<&EntityType> {
        let index = entity_type_id as usize - 1;
        self.entity_types.get(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EntityType> {
        self.entity_types.iter()
    }

    #[cfg(not(target_arch = "wasm32"))]
    // The client should *never* be able to mutate the entity registry.
    pub fn insert(&mut self, entity_type: EntityType) -> EntityTypeID {
        self.entity_types.push(entity_type);

        // note(KMRW):
        // We check the length of `entity_types` *after* we insert the entity to avoid having to
        // store an empty entity.
        // This may be a dumb idea.
        let entity_id = self.entity_types.len() as EntityTypeID;

        entity_id
    }
}
