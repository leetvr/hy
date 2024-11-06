use wasm_bindgen::prelude::*;

use crate::Engine;

#[wasm_bindgen]
#[derive(Default, Copy, Clone)]
pub enum EngineMode {
    Play,
    #[default]
    Edit,
}

#[derive(Default)]
pub struct Context {
    mode: EngineMode,
}

#[wasm_bindgen]
impl Engine {
    pub fn ctx_set_engine_mode(&mut self, mode: EngineMode) {
        self.context.mode = mode;
    }

    pub fn ctx_get_engine_mode(&self) -> EngineMode {
        self.context.mode
    }
}
