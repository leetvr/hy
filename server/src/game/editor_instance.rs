use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use net_types::{ClientShouldSwitchMode, PlayerId, ServerPacket, SetBlock};
use physics::PhysicsWorld;
use tokio::sync::mpsc;

use super::{network::Client, world::World, NextServerState};

pub struct EditorInstance {
    pub world: Arc<Mutex<World>>,
    pub editor_client: Client,
    pub physics_world: Arc<Mutex<PhysicsWorld>>,
}

impl EditorInstance {
    pub fn new(
        world: Arc<Mutex<World>>,
        editor_client: Client,
        physics_world: Arc<Mutex<PhysicsWorld>>,
    ) -> Self {
        Self {
            world,
            editor_client,
            physics_world,
        }
    }

    pub async fn from_transition(
        world: Arc<Mutex<World>>,
        editor_client: Client,
        physics_world: Arc<Mutex<PhysicsWorld>>,
    ) -> Self {
        // The most important thing to do here is tell the client to switch to edit mode.
        {
            let world = world.lock().expect("Deadlock!");
            editor_client
                .outgoing_tx
                .send(ServerPacket::ClientShouldSwitchMode(
                    ClientShouldSwitchMode::Edit {
                        world: net_types::Init {
                            blocks: world.blocks.clone(),
                            block_registry: world.block_registry.clone(),
                            entities: world.entities.clone(),
                            entity_type_registry: world.entity_type_registry.clone(),
                            client_player: PlayerId::new(0), // ignored by the editor
                        },
                    },
                ))
                .await
                .expect("Failed to send packet");
        }

        tracing::debug!("We're now in edit mode");

        Self {
            world,
            editor_client,
            physics_world,
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
                net_types::ClientPacket::AddEntity(entity) => {
                    self.add_entity(entity, storage_dir);
                }
                _ => {}
            }
        }

        maybe_next_state
    }

    fn set_block(&mut self, set_block: SetBlock, storage_dir: &PathBuf) {
        let SetBlock { position, block_id } = set_block;
        tracing::debug!("Setting block at {position:?} to {block_id}");

        let mut world = self.world.lock().expect("Deadlock!!");

        world.blocks[position] = block_id;
        world.save(storage_dir).expect("save world");
    }

    fn add_entity(&mut self, entity: net_types::AddEntity, storage_dir: &PathBuf) {
        let id = entity.entity_id;
        let position = entity.entity_data.state.position;
        let entity_type_id = entity.entity_data.entity_type;
        tracing::info!("Adding entity {id:?} at {position:?} of type {entity_type_id}");

        let mut world = self.world.lock().expect("Deadlock!!");

        world.entities.insert(id, entity.entity_data);
        world.save(storage_dir).expect("save world");
    }
}
