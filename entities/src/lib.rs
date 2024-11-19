use {
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
    tsify::Tsify,
    wasm_bindgen::prelude::wasm_bindgen,
};

// THis is only in the entities crate instead of the net-types crate because I need the PlayerId
// here and net-types depends on the entities crate. But it is my dream that players will one
// day also be entities. 🙏
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(u64);

impl PlayerId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn inner(&self) -> u64 {
        self.0
    }
}

pub type EntityTypeID = u8;
pub type EntityID = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EntityData {
    pub id: String,
    pub name: String,
    pub entity_type: EntityTypeID,
    pub model_path: String,
    pub state: EntityState,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntityPhysicsProperties {
    pub collider_kind: EntityColliderKind,
    pub collider_width: f32,
    pub collider_height: f32,
    pub dynamic: bool,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EntityColliderKind {
    #[default]
    Capsule,
    Cube,
    Ball,
}

#[wasm_bindgen]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default, Tsify)]
#[serde(rename_all = "camelCase")]
pub struct EntityType {
    pub id: EntityTypeID,
    name: String,
    script_path: String,
    default_model_path: String,
    physics_properties: Option<EntityPhysicsProperties>,
}

impl EntityType {
    pub fn id(&self) -> EntityTypeID {
        self.id
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn script_path(&self) -> &str {
        &self.script_path
    }

    pub fn default_model_path(&self) -> &str {
        &self.default_model_path
    }

    pub fn physics_properties(&self) -> Option<&EntityPhysicsProperties> {
        self.physics_properties.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    pub player_id: PlayerId,
    pub parent_anchor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub player_id: PlayerId,
    pub position: glam::Vec3,
    pub facing_angle: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EntityState {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub velocity: glam::Vec3,
    pub anchor: Option<Anchor>,
    pub interactions: Vec<Interaction>,
    #[serde(default)]
    pub custom_state: HashMap<String, serde_json::Value>,
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
        self.entity_types.get(entity_type_id as usize)
    }

    pub fn iter(&self) -> impl Iterator<Item = &EntityType> {
        self.entity_types.iter()
    }

    #[cfg(not(target_arch = "wasm32"))]
    // The client should *never* be able to mutate the entity registry.
    pub fn insert(&mut self, entity_type: EntityType) -> EntityTypeID {
        let entity_type_id = self.entity_types.len() as EntityTypeID;
        self.entity_types.push(entity_type);

        entity_type_id
    }
}
