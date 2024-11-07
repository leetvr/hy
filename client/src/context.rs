use blocks::BlockId;
use net_types::ClientPacket;
use wasm_bindgen::prelude::*;

use crate::{Engine, GameState};

#[wasm_bindgen]
#[derive(Default, Copy, Clone)]
pub enum EngineMode {
    Play,
    #[default]
    Edit,
}

pub struct Context {
    canvas: web_sys::HtmlCanvasElement,
}

impl Context {
    pub fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        Self { canvas }
    }
}

#[wasm_bindgen]
impl Engine {
    pub fn ctx_set_engine_mode(&mut self, mode: EngineMode) {
        // Transition to next state
        self.state.transition(mode);

        // Tell the server about the new state
        let packet = match mode {
            EngineMode::Play => ClientPacket::Start,
            EngineMode::Edit => ClientPacket::Edit,
        };

        self.send_packet(packet);
    }

    pub fn ctx_get_canvas(&self) -> web_sys::HtmlCanvasElement {
        self.context.canvas.clone()
    }

    pub fn ctx_set_editor_block_id(&mut self, block_id: BlockId) {
        // Ensure we're in edit mode
        let GameState::Editing {
            selected_block_id, ..
        } = &mut self.state
        else {
            return;
        };

        // Set the block ID
        selected_block_id.replace(block_id);
    }
}
