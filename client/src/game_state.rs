use std::collections::HashMap;

use blocks::{BlockGrid, BlockId, BlockPos, BlockRegistry};
use net_types::PlayerId;

use crate::{camera::FlyCamera, context::EngineMode, Player};

#[derive(Debug, Default)]
pub enum GameState {
    #[default]
    Loading,
    Playing {
        blocks: BlockGrid,
        block_registry: BlockRegistry,
        client_player: PlayerId,
        camera: FlyCamera,
        players: HashMap<PlayerId, Player>,
    },
    Editing {
        blocks: BlockGrid,
        block_registry: BlockRegistry,
        camera: FlyCamera,
        target_block: Option<BlockPos>,
        selected_block_id: Option<BlockId>,
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
                    camera,
                    ..
                },
                EngineMode::Edit,
            ) => {
                *self = GameState::Editing {
                    blocks,
                    block_registry,
                    camera,
                    target_block: None,
                    selected_block_id: None,
                }
            }
            // Editing -> Playing
            (
                GameState::Editing {
                    blocks,
                    block_registry,
                    camera,
                    ..
                },
                EngineMode::Play,
            ) => {
                *self = GameState::Playing {
                    blocks,
                    block_registry,
                    camera,
                    client_player: PlayerId::new(0), // note(KMRW): This will be replaced by the server
                    players: Default::default(),
                }
            }
            _ => {}
        };
    }
}
