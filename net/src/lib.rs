use {
    derive_more::From,
    serde::{Deserialize, Serialize},
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
}

// Packets from the server to the client

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Update a player's position
pub struct UpdatePosition {
    pub id: PlayerId,
    pub position: glam::Vec2,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Send a new player to the game
pub struct AddPlayer {
    pub id: PlayerId,
    pub position: glam::Vec2,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
/// Remove an existing player from the game
pub struct RemovePlayer {
    pub id: PlayerId,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, From)]
pub enum ServerPacket {
    AddPlayer(AddPlayer),
    UpdatePosition(UpdatePosition),
    RemovePlayer(RemovePlayer),
}
