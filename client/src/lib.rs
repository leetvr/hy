use std::collections::HashSet;

use {
    crate::{socket::ConnectionState, transform::Transform},
    anyhow::{Context, Result},
    blocks::BlockGrid,
    glam::{Mat4, Quat, Vec2, Vec3},
    net_types::PlayerId,
    std::{cell::RefCell, collections::HashMap, rc::Rc, slice, time::Duration},
    web_sys::WebSocket,
};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

mod gltf;
mod render;
mod socket;
mod transform;

struct TestGltf {
    gltf: gltf::GLTFModel,
    render_model: render::RenderModel,
}

#[wasm_bindgen]
pub struct Engine {
    renderer: render::Renderer,

    elapsed_time: Duration,
    test: Option<TestGltf>,

    ws: WebSocket,
    connection_state: Rc<RefCell<ConnectionState>>,
    incoming_messages: Rc<RefCell<Vec<Vec<u8>>>>,

    controls: Controls,
    player_model: TestGltf,

    game_state: GameState,
}

#[wasm_bindgen]
impl Engine {
    pub fn new() -> Result<Self, JsValue> {
        tracing_wasm::set_as_global_default();
        console_error_panic_hook::set_once();

        // Get the window, etc.
        let window = web_sys::window().ok_or("Could not access window")?;
        let document = window.document().ok_or("Could not access document")?;

        // Access the canvas element
        let canvas = document
            .get_element_by_id("canvas")
            .ok_or("Canvas element not found")?;
        let canvas: HtmlCanvasElement = canvas.dyn_into::<HtmlCanvasElement>()?;

        let renderer = render::Renderer::new(canvas)?;

        let connection_state = Rc::new(RefCell::new(ConnectionState::Connecting));
        let incoming_messages = Rc::new(RefCell::new(Vec::new()));
        let ws = socket::connect_to_server(
            "ws://127.0.0.1:8889",
            connection_state.clone(),
            incoming_messages.clone(),
        )
        .map_err(|e| format!("Failed to connect to server: {e}"))?;

        let player_model = {
            let gltf = gltf::load(include_bytes!("gltf/test.glb"))
                .map_err(|e| format!("Error loading GLTF: {e:#?}"))?;
            let render_model = render::RenderModel::from_gltf(&renderer, &gltf);
            TestGltf { gltf, render_model }
        };

        Ok(Self {
            renderer,
            elapsed_time: Duration::ZERO,
            test: None,

            ws,
            connection_state,
            incoming_messages,

            controls: Default::default(),
            player_model,

            game_state: Default::default(),
        })
    }

    pub fn key_down(&mut self, event: KeyboardEvent) {
        if event.code() == "KeyR" {
            let gltf = match gltf::load(include_bytes!("gltf/test.glb")) {
                Ok(g) => g,
                Err(e) => {
                    tracing::info!("Error loading GLTF: {e:#?}");
                    return;
                }
            };

            tracing::info!("GLTF loaded: {:#?}", gltf);

            let render_model = render::RenderModel::from_gltf(&self.renderer, &gltf);

            tracing::info!("Render model created: {:#?}", render_model);

            self.test = Some(TestGltf { gltf, render_model });
        }

        self.controls
            .keyboard_inputs
            .insert(event.code().as_str().to_string());
    }

    pub fn key_up(&mut self, event: KeyboardEvent) {
        self.controls.keyboard_inputs.remove(event.code().as_str());
    }

    pub fn tick(&mut self, time: f64) {
        let current_time = Duration::from_secs_f64(time / 1000.0);
        let delta_time = current_time - self.elapsed_time;
        self.elapsed_time = current_time;

        if *self.connection_state.borrow() == ConnectionState::Connected {
            send_controls(&self.controls, &mut self.ws).expect("Error while sending controls");

            loop {
                let Some(message) = self.incoming_messages.borrow_mut().pop() else {
                    break;
                };

                let packet: net_types::ServerPacket =
                    bincode::deserialize(&message).expect("Failed to deserialize position update");

                match &mut self.game_state {
                    GameState::Loading => match packet {
                        net_types::ServerPacket::InitLevel(init_level) => {
                            tracing::info!("Loaded level of size {:?}", init_level.blocks.size());
                            let blocks = init_level.blocks.iter_non_empty().map(|(pos, _)| pos);
                            let blocks_primitive = self.renderer.create_block_primitive(blocks);
                            self.game_state = GameState::Playing {
                                blocks: init_level.blocks,
                                blocks_primitive,
                                players: HashMap::new(),
                            };
                        }
                        p => {
                            tracing::error!("Received unexpected packet: {:#?}", p);
                            self.ws.close().unwrap();
                            break;
                        }
                    },
                    GameState::Playing {
                        players,
                        blocks,
                        blocks_primitive,
                    } => match packet {
                        net_types::ServerPacket::SetBlock(set_block) => {
                            handle_set_block(&self.renderer, blocks, blocks_primitive, set_block)
                                .expect("Failed to set block");
                        }
                        net_types::ServerPacket::AddPlayer(add_player) => {
                            handle_add_player(players, add_player).expect("Failed to add player");
                        }
                        net_types::ServerPacket::UpdatePosition(update_position) => {
                            handle_update_position(players, update_position)
                                .expect("Failed to update position");
                        }
                        net_types::ServerPacket::RemovePlayer(remove_player) => {
                            handle_remove_player(players, remove_player)
                                .expect("Failed to remove player");
                        }
                        p => {
                            tracing::error!("Received unexpected packet: {:#?}", p);
                            self.ws.close().unwrap();
                            break;
                        }
                    },
                }

                // match packet {
                //     net_types::ServerPacket::UpdatePosition(net_types::UpdatePosition {
                //         id,
                //         position,
                //     }) => {
                //         let Some(player) = self.players.get_mut(&id) else {
                //             tracing::error!("Received position update for unknown player");
                //             continue;
                //         };
                //         player.position = position;
                //     }
                //     net_types::ServerPacket::AddPlayer(net_types::AddPlayer { id, position }) => {
                //         self.players.insert(id, Player { position });
                //     }
                //     net_types::ServerPacket::RemovePlayer(net_types::RemovePlayer { id }) => {
                //         self.players.remove(&id);
                //     }
                // }
            }
        }

        match &*self.connection_state.borrow() {
            ConnectionState::Error(e) => {
                panic!("Error in websocket connection: {e:#}");
            }
            _ => {}
        }

        // Camera Input
        if self.controls.keyboard_inputs.contains("KeyI") {
            self.renderer.camera.translation.z += 0.1;
        }
        if self.controls.keyboard_inputs.contains("KeyK") {
            self.renderer.camera.translation.z -= 0.1;
        }
        if self.controls.keyboard_inputs.contains("KeyJ") {
            self.renderer.camera.translation.x += 0.1;
        }
        if self.controls.keyboard_inputs.contains("KeyL") {
            self.renderer.camera.translation.x -= 0.1;
        }
        if self.controls.keyboard_inputs.contains("KeyU") {
            self.renderer.camera.translation.y += 0.1;
        }
        if self.controls.keyboard_inputs.contains("KeyO") {
            self.renderer.camera.translation.y -= 0.1;
        }

        if self.controls.keyboard_inputs.contains("ArrowUp") {
            self.renderer.camera.rotation.x -= 0.02;
        }
        if self.controls.keyboard_inputs.contains("ArrowDown") {
            self.renderer.camera.rotation.x += 0.02;
        }
        if self.controls.keyboard_inputs.contains("ArrowLeft") {
            self.renderer.camera.rotation.y -= 0.02;
        }
        if self.controls.keyboard_inputs.contains("ArrowRight") {
            self.renderer.camera.rotation.y += 0.02;
        }

        self.render();
    }

    fn render(&self) {
        let mut draw_calls = Vec::new();

        if let GameState::Playing {
            players,
            blocks_primitive,
            ..
        } = &self.game_state
        {
            for player in players.values() {
                draw_calls.extend(render::build_render_plan(
                    slice::from_ref(&self.player_model.gltf),
                    slice::from_ref(&self.player_model.render_model),
                    Transform::new(player.position, Quat::IDENTITY),
                ));
            }

            draw_calls.push(render::DrawCall {
                primitive: blocks_primitive.clone(),
                transform: Mat4::IDENTITY,
            });
        }

        if let Some(ref test) = self.test {
            draw_calls.extend(render::build_render_plan(
                slice::from_ref(&test.gltf),
                slice::from_ref(&test.render_model),
                Transform::IDENTITY,
            ));

            tracing::debug!("Draw calls created: {:#?}", draw_calls);
        }

        self.renderer.render(self.elapsed_time, &draw_calls);
    }
}
#[wasm_bindgen]
pub fn increment(count: i32) -> i32 {
    count + 1
}

#[derive(Clone, Default)]
struct Controls {
    keyboard_inputs: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
struct Player {
    position: Vec3,
}

#[derive(Clone, Debug, Default)]
enum GameState {
    #[default]
    Loading,
    Playing {
        blocks: BlockGrid,
        blocks_primitive: render::RenderPrimitive,
        players: HashMap<PlayerId, Player>,
    },
}

// Networking

/// Send outgoing controls packet
fn send_controls(controls: &Controls, ws: &WebSocket) -> Result<()> {
    let mut move_dir = Vec2::ZERO;
    if controls.keyboard_inputs.contains("KeyW") {
        move_dir.y += 1.0;
    }
    if controls.keyboard_inputs.contains("KeyS") {
        move_dir.y -= 1.0;
    }
    if controls.keyboard_inputs.contains("KeyA") {
        move_dir.x -= 1.0;
    }
    if controls.keyboard_inputs.contains("KeyD") {
        move_dir.x += 1.0;
    }
    let controls = net_types::Controls {
        move_direction: move_dir.normalize_or_zero(),
    };
    let message = bincode::serialize(&controls).context("Failed to serialize controls")?;
    ws.send_with_u8_array(&message)
        .expect("Failed to send controls");

    Ok(())
}

// Handlers for incoming packets

/// Handle a `SetBlock` packet
fn handle_set_block(
    renderer: &render::Renderer,
    blocks: &mut BlockGrid,
    blocks_primitive: &mut render::RenderPrimitive,
    net_types::SetBlock { position, block_id }: net_types::SetBlock,
) -> Result<()> {
    blocks[position] = block_id;
    *blocks_primitive =
        renderer.create_block_primitive(blocks.iter_non_empty().map(|(pos, _)| pos));
    Ok(())
}

/// Handle an `AddPlayer` packet
fn handle_add_player(
    players: &mut HashMap<PlayerId, Player>,
    net_types::AddPlayer { id, position }: net_types::AddPlayer,
) -> Result<()> {
    players.insert(id, Player { position });
    Ok(())
}

/// Handle a `RemovePlayer` packet
fn handle_remove_player(
    players: &mut HashMap<PlayerId, Player>,
    net_types::RemovePlayer { id }: net_types::RemovePlayer,
) -> Result<()> {
    players.remove(&id);
    Ok(())
}

/// Handle an `UpdatePosition` packet
fn handle_update_position(
    players: &mut HashMap<PlayerId, Player>,
    net_types::UpdatePosition { id, position }: net_types::UpdatePosition,
) -> Result<()> {
    let player = players
        .get_mut(&id)
        .context("Received position update for unknown player")?;
    player.position = position;
    Ok(())
}
