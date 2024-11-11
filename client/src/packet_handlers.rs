use std::collections::HashMap;

use anyhow::Result;
use blocks::BlockGrid;
use net_types::PlayerId;

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
    net_types::AddPlayer { id, position }: net_types::AddPlayer,
) -> Result<()> {
    players.insert(id, Player { position });
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
    net_types::UpdatePosition { id, position }: net_types::UpdatePosition,
) {
    let Some(player) = players.get_mut(&id) else {
        tracing::warn!("Received update position for unknown player {id:?}");
        return;
    };
    player.position = position;
}
