use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Controls {
    pub move_direction: glam::Vec2,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Position(pub glam::Vec2);
