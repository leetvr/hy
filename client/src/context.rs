use blocks::BlockTypeID;
use entities::{EntityData, EntityState, EntityTypeID};
use nanorand::Rng;
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
        // Tell the server about the new state
        let packet = match mode {
            EngineMode::Play => ClientPacket::Start,
            EngineMode::Edit => ClientPacket::Edit,
        };

        self.send_packet(packet);
    }

    pub fn ctx_get_engine_mode(&mut self) -> EngineMode {
        match self.state {
            GameState::Loading => EngineMode::Play,
            GameState::Playing { .. } => EngineMode::Play,
            GameState::Editing { .. } => EngineMode::Edit,
        }
    }

    pub fn ctx_get_canvas(&self) -> web_sys::HtmlCanvasElement {
        self.context.canvas.clone()
    }

    pub fn ctx_on_init(&mut self, cb: js_sys::Function) {
        // Store the callback to keep it alive
        self.context.on_init_callback = Some(cb);
    }

    pub fn ctx_set_editor_block_id(&mut self, block_id: BlockTypeID) {
        // Ensure we're in edit mode
        let GameState::Editing {
            selected_block_id,
            preview_entity,
            ..
        } = &mut self.state
        else {
            return;
        };

        // Set the block ID
        *selected_block_id = Some(block_id);
        *preview_entity = None;
    }

    pub fn ctx_set_editor_entity_type_id(&mut self, entity_type_id: EntityTypeID) {
        // Ensure we're in edit mode
        let GameState::Editing {
            selected_block_id,
            preview_entity,
            entity_type_registry,
            ..
        } = &mut self.state
        else {
            return;
        };

        tracing::info!("Set entity type id to {}", entity_type_id);

        // Create a preview entity
        *selected_block_id = None;

        let Some(entity_type) = entity_type_registry.get(entity_type_id) else {
            tracing::warn!("{entity_type_id} is not a valid entity type ID!");
            return;
        };

        let entity_id = nanorand::tls_rng().generate::<u64>().to_string();
        *preview_entity = Some(EntityData {
            id: entity_id,
            name: "We should let you set entity names in the editor".into(),
            entity_type: entity_type_id,
            model_path: entity_type.default_model_path().into(),
            state: EntityState::default(),
            physics_properties: None,
        });
    }
}
