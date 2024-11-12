use std::collections::HashMap;

use blocks::{BlockGrid, BlockRegistry, BlockTypeID, RayHit};
use entities::{EntityData, EntityTypeRegistry};
use net_types::PlayerId;

use crate::{camera::FlyCamera, context::EngineMode, Player};

#[derive(Debug, Default)]
pub enum GameState {
    #[default]
    Loading,
    Playing {
        blocks: BlockGrid,
        block_registry: BlockRegistry,
        entities: Vec<EntityData>,
        entity_type_registry: EntityTypeRegistry,
        client_player: PlayerId,
        camera: FlyCamera,
        players: HashMap<PlayerId, Player>,
    },
    Editing {
        blocks: BlockGrid,
        block_registry: BlockRegistry,
        entities: Vec<EntityData>,
        entity_type_registry: EntityTypeRegistry,
        camera: FlyCamera,
        target_raycast: Option<RayHit>,
        selected_block_id: Option<BlockTypeID>,
    },
}

impl GameState {
    pub fn transition(&mut self, next_state: EngineMode) {
        let current_state = std::mem::replace(self, GameState::Loading);
        match (current_state, next_state) {
            // Playing -> Editing
            (
                GameState::Playing {
                    blocks,
                    block_registry,
                    entities,
                    entity_type_registry,
                    camera,
                    ..
                },
                EngineMode::Edit,
            ) => {
                *self = GameState::Editing {
                    blocks,
                    block_registry,
                    entities,
                    entity_type_registry,
                    camera,
                    target_raycast: None,
                    selected_block_id: None,
                }
            }
            // Editing -> Playing
            (
                GameState::Editing {
                    blocks,
                    block_registry,
                    camera,
                    entities,
                    entity_type_registry,
                    ..
                },
                EngineMode::Play,
            ) => {
                *self = GameState::Playing {
                    blocks,
                    block_registry,
                    entities,
                    entity_type_registry,
                    camera,
                    client_player: PlayerId::new(0), // note(KMRW): This will be replaced by the server
                    players: Default::default(),
                }
            }
            _ => {}
        };
    }
}
