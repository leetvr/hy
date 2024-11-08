use wasm_bindgen::prelude::*;

use {
    serde::{Deserialize, Serialize},
    tsify::Tsify,
};

#[wasm_bindgen]
#[derive(Tsify, Serialize, Deserialize)]
pub struct PlayerState {
    x: f32,
    y: f32,
    z: f32,
}

#[wasm_bindgen]
#[derive(Tsify, Serialize, Deserialize)]
pub struct PlayerControls {
    pub move_x: f32,
    pub move_y: f32,
    pub jump: bool,
}
