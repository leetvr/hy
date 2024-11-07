use tokio::sync::mpsc;

use super::{network::Client, world::World, NextServerState};

pub struct EditorInstance {
    pub world: World,
    pub editor_client: Client,
}

impl EditorInstance {
    pub fn new(world: World, editor_client: Client) -> Self {
        Self {
            world,
            editor_client,
        }
    }

    pub(crate) fn tick(&mut self) -> Option<super::NextServerState> {
        let mut maybe_next_state = None;
        let client = &mut self.editor_client;

        while let Some(packet) = match client.incoming_rx.try_recv() {
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
                _ => {}
            }
        }

        maybe_next_state
    }
}
