use {
    blocks::{BlockGrid, BlockPos, BlockRegistry},
    derive_more::From,
    entities::{EntityData, EntityID, EntityTypeRegistry},
    serde::{Deserialize, Serialize},
    std::collections::HashMap,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(u64);

impl PlayerId {
    pub fn new(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
// Client's input state, sent greedily to the server
pub struct Controls {
    pub move_direction: glam::Vec2,
    pub jump: bool,
    pub camera_yaw: f32, // radians
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Controls(Controls),
    Start,
    Pause,
    Edit,
    SetBlock(SetBlock), // used by editor
}

// Packets from the server to the client

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Update a player's state
pub struct UpdatePlayer {
    pub id: PlayerId,
    pub position: glam::Vec3,
    // Included if the animation state has changed
    pub animation_state: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
/// Send a new player to the game
pub struct AddPlayer {
    pub id: PlayerId,
    pub position: glam::Vec3,
    pub animation_state: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Remove an existing player from the game
pub struct RemovePlayer {
    pub id: PlayerId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Init {
    pub blocks: BlockGrid,
    pub block_registry: BlockRegistry,
    pub entities: HashMap<EntityID, EntityData>,
    pub entity_type_registry: EntityTypeRegistry,
    pub client_player: PlayerId,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reset {
    pub new_client_player: PlayerId,
}

#[derive(Clone, Debug, Copy, Serialize, Deserialize)]
pub struct SetBlock {
    pub position: BlockPos,
    pub block_id: blocks::BlockTypeID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AddEntity {
    pub entity_id: EntityID,
    pub entity_data: EntityData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoveEntity {
    pub entity_id: EntityID,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UpdateEntity {
    pub entity_id: EntityID,
    pub position: glam::Vec3,
}

#[derive(Clone, Debug, Serialize, Deserialize, From)]
pub enum ServerPacket {
    Init(Init),
    Reset(Reset),
    SetBlock(SetBlock),
    AddPlayer(AddPlayer),
    UpdatePlayer(UpdatePlayer),
    RemovePlayer(RemovePlayer),
    AddEntity(AddEntity),
    UpdateEntity(UpdateEntity),
    RemoveEntity(RemoveEntity),
}
