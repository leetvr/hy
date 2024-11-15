use {entities::EntityID, std::collections::HashMap};

use blocks::{BlockGrid, BlockRegistry, BlockTypeID, RayHit};
use entities::{EntityData, EntityTypeRegistry};
use net_types::{ClientShouldSwitchMode, PlayerId};

use crate::{camera::FlyCamera, Player};

#[derive(Debug, Default)]
pub enum GameState {
    #[default]
    Loading,
    Playing {
        blocks: BlockGrid,
        block_registry: BlockRegistry,
        entities: HashMap<EntityID, EntityData>,
        _entity_type_registry: EntityTypeRegistry,
        client_player: PlayerId,
        camera: FlyCamera,
        players: HashMap<PlayerId, Player>,
    },
    Editing {
        blocks: BlockGrid,
        block_registry: BlockRegistry,
        entities: HashMap<EntityID, EntityData>,
        entity_type_registry: EntityTypeRegistry,
        camera: FlyCamera,
        target_raycast: Option<RayHit>,
        selected_block_id: Option<BlockTypeID>,
        preview_entity: Option<EntityData>,
    },
}

impl GameState {
    pub fn block_grid(&self) -> Option<&BlockGrid> {
        match self {
            GameState::Playing { blocks, .. } | GameState::Editing { blocks, .. } => Some(blocks),
            _ => None,
        }
    }

    pub fn switch_mode(&mut self, mode_switch: ClientShouldSwitchMode) {
        let current_state = std::mem::replace(self, GameState::Loading);
        match (current_state, mode_switch) {
            // Playing -> Editing
            (GameState::Playing { camera, .. }, ClientShouldSwitchMode::Edit { world }) => {
                tracing::debug!("Transitioning from playing to editing");
                *self = GameState::Editing {
                    blocks: world.blocks,
                    block_registry: world.block_registry,
                    entities: world.entities,
                    entity_type_registry: world.entity_type_registry,
                    camera,
                    target_raycast: None,
                    selected_block_id: None,
                    preview_entity: None,
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
                ClientShouldSwitchMode::Play { new_player_id },
            ) => {
                tracing::debug!("Transitioning from editing to playing");
                *self = GameState::Playing {
                    blocks,
                    block_registry,
                    entities,
                    _entity_type_registry: entity_type_registry,
                    camera,
                    client_player: new_player_id,
                    players: Default::default(),
                }
            }
            // Invalid or no-op transition
            (current_state, _) => {
                *self = current_state;
            }
        };
    }
}
