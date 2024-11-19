use {
    crate::gltf::GLTFModel,
    anyhow::{bail, Result},
    blocks::BlockGrid,
    entities::{EntityData, EntityID, PlayerId},
    glam::Vec3Swizzles,
    net_types::{AddEntity, RemoveEntity, UpdateEntity},
    std::collections::HashMap,
};

use crate::Player;
// Handlers for incoming packets

/// Handle a `SetBlock` packet
pub fn handle_set_block(
    blocks: &mut BlockGrid,
    net_types::SetBlock { position, block_id }: net_types::SetBlock,
) -> Result<()> {
    blocks[position] = block_id;
    Ok(())
}

/// Handle an `AddPlayer` packet
pub fn handle_add_player(
    players: &mut HashMap<PlayerId, Player>,
    model: &GLTFModel,
    net_types::AddPlayer {
        id,
        position,
        animation_state,
        script_state,
    }: net_types::AddPlayer,
) -> Result<()> {
    let mut model = model.clone();
    model.play_animation(&animation_state, 0.5);
    players.insert(
        id,
        Player {
            position,
            facing_angle: 0.,
            model: model.clone(),
            script_state,
        },
    );
    Ok(())
}

/// Handle a `RemovePlayer` packet
pub fn handle_remove_player(
    players: &mut HashMap<PlayerId, Player>,
    net_types::RemovePlayer { id }: net_types::RemovePlayer,
) -> Result<()> {
    players.remove(&id);
    Ok(())
}

/// Handle an `UpdatePosition` packet
pub fn handle_update_position(
    players: &mut HashMap<PlayerId, Player>,
    net_types::UpdatePlayer {
        id,
        position,
        animation_state,
        script_state,
    }: net_types::UpdatePlayer,
) {
    let Some(player) = players.get_mut(&id) else {
        tracing::warn!("Received update position for unknown player {id:?}");
        return;
    };

    let movement: glam::Vec2 = position.xz() - player.position.xz();
    if movement.length() > 0. {
        player.facing_angle = movement.to_angle();
    }

    player.position = position;
    if let Some(animation_state) = animation_state {
        player.model.play_animation(&animation_state, 0.5);
    }
    if let Some(script_state) = script_state {
        player.script_state = script_state;
    }
}

pub(crate) fn handle_add_entity(
    entities: &mut HashMap<EntityID, EntityData>,
    AddEntity {
        entity_id,
        entity_data,
    }: AddEntity,
) {
    tracing::debug!("Added entity {entity_id}");
    entities.insert(entity_id, entity_data);
}

pub(crate) fn handle_update_entity(
    entities: &mut HashMap<EntityID, EntityData>,
    UpdateEntity {
        entity_id,
        position,
        rotation,
        anchor,
    }: UpdateEntity,
) -> Result<()> {
    let Some(entity) = entities.get_mut(&entity_id) else {
        bail!("Received update entity for unknown entity {entity_id:?}");
    };

    entity.state.position = position;
    entity.state.rotation = rotation;
    entity.state.anchor = anchor;

    Ok(())
}

pub(crate) fn handle_remove_entity(
    entities: &mut HashMap<EntityID, EntityData>,
    RemoveEntity { entity_id }: RemoveEntity,
) {
    entities.remove(&entity_id);
}
