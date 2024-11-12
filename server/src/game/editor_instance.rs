use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use net_types::SetBlock;
use tokio::sync::mpsc;

use super::{network::Client, world::World, NextServerState};

pub struct EditorInstance {
    pub world: Arc<Mutex<World>>,
    pub editor_client: Client,
}

impl EditorInstance {
    pub fn new(world: Arc<Mutex<World>>, editor_client: Client) -> Self {
        Self {
            world,
            editor_client,
        }
    }

    pub(crate) fn tick(&mut self, storage_dir: &PathBuf) -> Option<super::NextServerState> {
        let mut maybe_next_state = None;

        while let Some(packet) = match self.editor_client.incoming_rx.try_recv() {
            Ok(v) => Some(v),
            Err(e) => match e {
                mpsc::error::TryRecvError::Empty => None,
                mpsc::error::TryRecvError::Disconnected => {
                    // If the editor client disconnected, we must leave the editing state and wait
                    // for new clients.
                    return Some(NextServerState::Paused);
                }
            },
        } {
            match packet {
                net_types::ClientPacket::Start => maybe_next_state = Some(NextServerState::Playing),
                net_types::ClientPacket::Pause => maybe_next_state = Some(NextServerState::Paused),
                net_types::ClientPacket::SetBlock(set_block) => {
                    self.set_block(set_block, storage_dir);
                }
                _ => {}
            }
        }

        maybe_next_state
    }

    fn set_block(&mut self, set_block: SetBlock, storage_dir: impl AsRef<Path>) {
        let SetBlock { position, block_id } = set_block;
        tracing::debug!("Setting block at {position:?} to {block_id}");

        let mut world = self.world.lock().expect("Deadlock!!");

        world.blocks[position] = block_id;
        world.save(storage_dir).expect("save world");
    }
}
