use wasm_bindgen::prelude::*;

use {
    serde::{Deserialize, Serialize},
    tsify::Tsify,
};

#[wasm_bindgen]
#[derive(Tsify, Serialize, Deserialize)]
pub struct PlayerState {
    position: glam::Vec3,
}
