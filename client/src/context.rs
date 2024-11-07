use net_types::ClientPacket;
use wasm_bindgen::prelude::*;

use crate::Engine;

#[wasm_bindgen]
#[derive(Default, Copy, Clone)]
pub enum EngineMode {
    Play,
    #[default]
    Edit,
}

pub struct Context {
    pub mode: EngineMode,
    canvas: web_sys::HtmlCanvasElement,
}

impl Context {
    pub fn new(canvas: web_sys::HtmlCanvasElement) -> Self {
        Self {
            mode: EngineMode::default(),
            canvas,
        }
    }
}

#[wasm_bindgen]
impl Engine {
    pub fn ctx_set_engine_mode(&mut self, mode: EngineMode) {
        self.context.mode = mode;

        // Transition to next state
        let packet = match mode {
            EngineMode::Play => ClientPacket::Start,
            EngineMode::Edit => {
                // Clear out all players from the world
                match &mut self.game_state {
                    crate::GameState::Playing { players, .. } => {
                        tracing::debug!("Clearing players");
                        players.clear()
                    }
                    _ => {}
                };
                ClientPacket::Edit
            }
        };

        // Tell the server about the new state
        self.send_packet(packet);
    }

    pub fn ctx_get_engine_mode(&self) -> EngineMode {
        self.context.mode
    }

    pub fn ctx_get_canvas(&self) -> web_sys::HtmlCanvasElement {
        self.context.canvas.clone()
    }
}
