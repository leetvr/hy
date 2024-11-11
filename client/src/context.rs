use blocks::BlockId;
use net_types::ClientPacket;
use wasm_bindgen::prelude::*;
use web_sys::js_sys;

use crate::{game_state::GameState, Engine};

#[wasm_bindgen]
#[derive(Default, Copy, Clone)]
pub enum EngineMode {
    Play,
    #[default]
    Edit,
}

pub struct Context {
    canvas: web_sys::HtmlCanvasElement,
    pub on_init_callback: Option<js_sys::Function>,
}

impl Context {
    pub fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        Self {
            canvas,
            on_init_callback: None,
        }
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

    pub fn ctx_on_init(&mut self, cb: js_sys::Function) {
        // Store the callback to keep it alive
        self.context.on_init_callback = Some(cb);
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
