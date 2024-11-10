use {
    crate::{assets::Assets, camera::FlyCamera},
    blocks::BlockRegistry,
    dolly::prelude::YawPitch,
    glam::EulerRot,
    image::GenericImageView,
    std::collections::HashSet,
    web_sys::MouseEvent,
};

use {
    crate::{socket::ConnectionState, transform::Transform},
    anyhow::Result,
    blocks::BlockGrid,
    glam::{Quat, Vec2, Vec3},
    net_types::PlayerId,
    std::{cell::RefCell, collections::HashMap, rc::Rc, slice, time::Duration},
    web_sys::WebSocket,
};

use blocks::BlockId;
// Re-exports
pub use blocks::BlockPos;

use context::EngineMode;
use glam::UVec2;
use net_types::ClientPacket;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

mod assets;
mod audio;
mod camera;
mod context;
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
    context: context::Context,
    renderer: render::Renderer,

    elapsed_time: Duration,
    delta_time: Duration,
    test: Option<TestGltf>,

    ws: WebSocket,
    connection_state: Rc<RefCell<ConnectionState>>,
    incoming_messages: Rc<RefCell<Vec<Vec<u8>>>>,

    controls: Controls,
    player_model: TestGltf,

    cube_mesh_data: render::CubeVao,
    // Textures by path
    // Value is Some(texture) for a loaded texture, None for a texture that errored on loading
    // and vacant for a texture that is still loading
    block_textures: Option<Vec<[render::Texture; 6]>>,

    assets: assets::Assets,
    state: GameState,

    audio_manager: audio::AudioManager,
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

        let renderer = render::Renderer::new(canvas.clone())?;

        let connection_state = Rc::new(RefCell::new(ConnectionState::Connecting));
        let incoming_messages = Rc::new(RefCell::new(Vec::new()));
        let ws = socket::connect_to_server(
            "ws://127.0.0.1:8889",
            connection_state.clone(),
            incoming_messages.clone(),
        )
        .map_err(|e| format!("Failed to connect to server: {e}"))?;

        let player_model = {
            let gltf = gltf::load(include_bytes!("../../assets/NewModel_Anchors_Armor.gltf"))
                .map_err(|e| format!("Error loading GLTF: {e:#?}"))?;
            let render_model = render::RenderModel::from_gltf(&renderer, &gltf);
            TestGltf { gltf, render_model }
        };

        let audio_manager = audio::AudioManager::new()?;

        Ok(Self {
            context: context::Context::new(canvas),
            cube_mesh_data: renderer.create_cube_vao(),
            block_textures: None,

            renderer,
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            test: None,

            ws,
            connection_state,
            incoming_messages,

            controls: Default::default(),
            player_model,

            assets: Assets::new(),
            state: Default::default(),
            audio_manager,
        })
    }

    pub fn key_down(&mut self, event: KeyboardEvent) {
        if event.code() == "KeyR" {
            let gltf = match gltf::load(include_bytes!("../../assets/NewModel_Anchors_Armor.gltf"))
            {
                Ok(g) => g,
                Err(e) => {
                    tracing::info!("Error loading GLTF: {e:#?}");
                    return;
                }
            };

            let render_model = render::RenderModel::from_gltf(&self.renderer, &gltf);

            self.test = Some(TestGltf { gltf, render_model });
        }

        if event.code() == "KeyT" {
            if let Some(ref mut test) = self.test {
                test.gltf.play_animation("idle", 0.5);
            }
        }
        if event.code() == "KeyY" {
            if let Some(ref mut test) = self.test {
                test.gltf.play_animation("walk", 0.5);
            }
        }
        if event.code() == "KeyG" {
            if let Some(ref mut test) = self.test {
                test.gltf.stop_animation(0.5);
            }
        }

        self.controls
            .keyboard_inputs
            .insert(event.code().as_str().to_string());
        self.controls
            .keyboard_pressed
            .insert(event.code().as_str().to_string());
    }

    pub fn key_up(&mut self, event: KeyboardEvent) {
        self.controls.keyboard_inputs.remove(event.code().as_str());
    }

    pub fn mouse_move(&mut self, event: MouseEvent) {
        self.controls.mouse_movement = (
            self.controls.mouse_movement.0 + event.movement_x(),
            self.controls.mouse_movement.1 + event.movement_y(),
        );
    }

    pub fn mouse_up(&mut self, event: MouseEvent) {
        match event.button() {
            0 => self.controls.mouse_left = false,
            2 => self.controls.mouse_right = false,
            _ => {}
        }
    }

    pub fn mouse_down(&mut self, event: MouseEvent) {
        match event.button() {
            0 => self.controls.mouse_left = true,
            2 => self.controls.mouse_right = true,
            _ => {}
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(UVec2::new(width, height));
    }

    pub fn tick(&mut self, time: f64) {
        let current_time = Duration::from_secs_f64(time / 1000.0);
        self.delta_time = current_time - self.elapsed_time;
        self.elapsed_time = current_time;

        // Maintain assets, loading pending assets from remote requests
        self.assets.maintain();

        // Receive packets
        if *self.connection_state.borrow() == ConnectionState::Connected {
            loop {
                if self.incoming_messages.borrow().is_empty() {
                    break;
                }
                // Incoming messages are pushed to the back of the queue, so we process them from
                // the front
                let message = self.incoming_messages.borrow_mut().remove(0);

                let packet: net_types::ServerPacket =
                    bincode::deserialize(&message).expect("Failed to deserialize position update");

                match &mut self.state {
                    GameState::Loading => match packet {
                        net_types::ServerPacket::Init(net_types::Init {
                            blocks,
                            block_registry,
                            ..
                        }) => {
                            tracing::info!("Loaded level of size {:?}", blocks.size());
                            tracing::info!("Block registry: {:#?}", block_registry);

                            // Start fetching textures
                            for block_type in block_registry.iter() {
                                self.assets.get(&block_type.top_texture);
                                self.assets.get(&block_type.bottom_texture);
                                self.assets.get(&block_type.east_texture);
                                self.assets.get(&block_type.west_texture);
                                self.assets.get(&block_type.north_texture);
                                self.assets.get(&block_type.south_texture);
                            }

                            // Tell the React frontend
                            if let Some(on_init) = self.context.on_init_callback.take() {
                                let data = serde_wasm_bindgen::to_value(&block_registry).unwrap();
                                on_init
                                    .call1(&JsValue::NULL, &data)
                                    .expect("Unable to call on_init!");
                            }

                            self.state = GameState::Editing {
                                blocks,
                                block_registry,
                                camera: FlyCamera::new(Vec3::ZERO),
                                target_block: None,
                                selected_block_id: None,
                            };

                            // When we've connected, tell the server we want to switch to edit mode.
                            self.send_packet(net_types::ClientPacket::Edit);
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
                        client_player,
                        ..
                    } => match packet {
                        net_types::ServerPacket::SetBlock(set_block) => {
                            handle_set_block(blocks, set_block).expect("Failed to set block");
                        }
                        net_types::ServerPacket::AddPlayer(add_player) => {
                            handle_add_player(players, add_player).expect("Failed to add player");
                        }
                        net_types::ServerPacket::UpdatePosition(update_position) => {
                            handle_update_position(players, update_position);
                        }
                        net_types::ServerPacket::RemovePlayer(remove_player) => {
                            handle_remove_player(players, remove_player)
                                .expect("Failed to remove player");
                        }
                        // Sent by the server when we leave edit mode
                        net_types::ServerPacket::Reset(net_types::Reset {
                            new_client_player,
                            ..
                        }) => {
                            players.clear();
                            *client_player = new_client_player;
                        }
                        p => {
                            tracing::error!("Received unexpected packet: {:#?}", p);
                            self.ws.close().unwrap();
                            break;
                        }
                    },
                    GameState::Editing { .. } => {}
                }
            }
        }

        // Check for errors
        match &*self.connection_state.borrow() {
            ConnectionState::Error(e) => {
                panic!("Error in websocket connection: {e:#}");
            }
            _ => {}
        }

        // Send packets
        match &mut self.state {
            GameState::Playing {
                players,
                client_player,
                camera,
                ..
            } => {
                let delta_yaw = -self.controls.mouse_movement.0 as f32 * MOUSE_SENSITIVITY_X;
                let delta_pitch = -self.controls.mouse_movement.1 as f32 * MOUSE_SENSITIVITY_Y;

                // Player camera
                self.controls.yaw =
                    (self.controls.yaw + delta_yaw).rem_euclid(std::f32::consts::TAU);
                self.controls.pitch = (self.controls.pitch + delta_pitch)
                    .clamp(-std::f32::consts::FRAC_PI_2, std::f32::consts::FRAC_PI_2);

                let player_position = players
                    .get(client_player)
                    .map(|p| p.position)
                    .unwrap_or_default();
                let rotation = glam::Quat::from_euler(
                    EulerRot::YXZ,
                    self.controls.yaw,
                    self.controls.pitch,
                    0.,
                );

                let look_dir = rotation * -glam::Vec3::Z;
                let position = player_position - (look_dir * CAMERA_DISTANCE)
                    + (glam::Vec3::Y * CAMERA_HEIGHT);

                self.renderer.camera.position = position;
                self.renderer.camera.rotation = rotation;

                // Keep editor camera in sync with player camera
                camera.set_position_and_rotation(position, YawPitch::new().rotation_quat(rotation));
                camera.update(self.delta_time.as_secs_f32());

                // Player input
                let mut move_dir = Vec2::ZERO;
                if self.controls.keyboard_inputs.contains("KeyW") {
                    move_dir.y += 1.0;
                }
                if self.controls.keyboard_inputs.contains("KeyS") {
                    move_dir.y -= 1.0;
                }
                if self.controls.keyboard_inputs.contains("KeyA") {
                    move_dir.x -= 1.0;
                }
                if self.controls.keyboard_inputs.contains("KeyD") {
                    move_dir.x += 1.0;
                }
                move_dir =
                    Vec2::from_angle(move_dir.to_angle() + self.controls.yaw) * move_dir.length();
                let controls = net_types::Controls {
                    move_direction: move_dir.normalize_or_zero(),
                    jump: self.controls.keyboard_pressed.contains("Space"),
                };
                self.send_packet(net_types::ClientPacket::Controls(controls));
            }
            GameState::Editing {
                camera,
                blocks,
                target_block,
                selected_block_id,
                ..
            } => {
                // Camera input
                let key_state = |code| -> f32 {
                    self.controls
                        .keyboard_inputs
                        .contains(code)
                        .then(|| 1.0)
                        .unwrap_or(0.0)
                };

                let delta_yaw = -self.controls.mouse_movement.0 as f32 * MOUSE_SENSITIVITY_X;
                let delta_pitch = -self.controls.mouse_movement.1 as f32 * MOUSE_SENSITIVITY_Y;

                // Collect camera movement
                camera.movement_forward = key_state("KeyW");
                camera.movement_backward = key_state("KeyS");
                camera.movement_left = key_state("KeyA");
                camera.movement_right = key_state("KeyD");
                camera.movement_up = key_state("Space");
                camera.movement_down = key_state("ShiftLeft");
                camera.boost = key_state("CtrlLeft");
                camera.rotate(delta_yaw.to_degrees(), delta_pitch.to_degrees());

                // Update camera
                camera.update(self.delta_time.as_secs_f32());
                let (position, rotation) = camera.position_and_rotation();
                self.renderer.camera.position = position;
                self.renderer.camera.rotation = rotation;

                // Block Selection
                let inv_view_matrix = self.renderer.camera.view_matrix().inverse();
                let ray_dir = inv_view_matrix.transform_vector3(-Vec3::Z).normalize();

                // If we're *placing* blocks, ie. not removing them, we actually want to place a
                // block *above* the raycast target.
                let mode = match selected_block_id {
                    None | Some(0) => blocks::RaycastMode::Selecting,
                    _ => blocks::RaycastMode::Placing,
                };
                *target_block = blocks
                    .raycast(position, ray_dir, mode)
                    .map(|hit| hit.position);

                if self.controls.mouse_left {
                    tracing::debug!("Placing block at {target_block:?}");
                    self.place_block();
                }

                // Send empty player input
                let controls = net_types::Controls {
                    move_direction: Vec2::ZERO,
                    jump: false,
                };
                self.send_packet(net_types::ClientPacket::Controls(controls));
            }
            _ => {}
        }

        self.controls.keyboard_pressed.clear();
        self.controls.mouse_movement = (0, 0);
        self.controls.mouse_left = false;
        self.controls.mouse_right = false;

        self.update_audio_manager();

        self.render();
    }

    fn send_packet(&mut self, packet: ClientPacket) {
        let message = bincode::serialize(&packet).unwrap();
        self.ws
            .send_with_u8_array(&message)
            .expect("Failed to send controls");
    }

    fn place_block(&mut self) {
        // Ensure we're in the editing state and we have a selected block ID
        let GameState::Editing {
            blocks,
            target_block: Some(target_block),
            selected_block_id: Some(block_id),
            ..
        } = &mut self.state
        else {
            return;
        };

        let block_id = *block_id;
        let position = *target_block;
        let set_block = net_types::SetBlock { position, block_id };

        tracing::debug!("Setting block at {position:?} to {block_id}");

        handle_set_block(blocks, set_block).expect("place block");

        self.send_packet(ClientPacket::SetBlock(set_block));
    }

    fn render(&mut self) {
        let mut draw_calls = Vec::new();

        // Gather blocks
        match &self.state {
            GameState::Playing {
                blocks,
                block_registry,
                ..
            }
            | GameState::Editing {
                blocks,
                block_registry,
                ..
            } => {
                if self.block_textures.is_none() {
                    // Try to collect textures for all block types. If any are missing, just wait for all of them to arrive.
                    self.block_textures =
                        collect_block_textures(block_registry, &self.renderer, &mut self.assets);
                }

                if let Some(block_textures) = &self.block_textures {
                    let blocks = blocks.iter_non_empty().filter_map(|(pos, block_id)| {
                        Some((pos, &block_textures[block_id as usize - 1]))
                    });

                    // TODO: If we're trying to *remove* a block, we need to not render a block at that position.
                    draw_calls.extend(
                        render::build_cube_draw_calls(&self.cube_mesh_data, blocks, None)
                            .into_iter(),
                    );
                }
            }
            _ => {}
        };

        // Gather state-specific extras
        match &self.state {
            // Players
            GameState::Playing { players, .. } => {
                for player in players.values() {
                    draw_calls.extend(render::build_render_plan(
                        slice::from_ref(&self.player_model.gltf),
                        slice::from_ref(&self.player_model.render_model),
                        Transform::new(player.position, Quat::IDENTITY),
                    ));
                }
            }
            // Ghost block
            GameState::Editing {
                target_block: Some(block_pos),
                selected_block_id: Some(block_id),
                ..
            } if *block_id != 0 => {
                if let Some(block_textures) = &self.block_textures {
                    let textures = &block_textures[*block_id as usize - 1];
                    let blocks = [(*block_pos, textures)];

                    draw_calls.extend(
                        render::build_cube_draw_calls(
                            &self.cube_mesh_data,
                            blocks.into_iter(),
                            Some([0., 1.0, 0., 1.0].into()),
                        )
                        .into_iter(),
                    );
                }
            }
            _ => (),
        }

        if let Some(ref mut test) = self.test {
            gltf::animate_model(&mut test.gltf, self.delta_time);

            draw_calls.extend(render::build_render_plan(
                slice::from_ref(&test.gltf),
                slice::from_ref(&test.render_model),
                Transform::IDENTITY,
            ));
        }

        self.renderer.render(&draw_calls);
    }

    pub async fn load_sound(&mut self, sound_id: &str) -> Result<(), JsValue> {
        // self.audio_manager.load_sound(url).await
        self.audio_manager.load_sound_from_id(sound_id).await
    }

    pub async fn load_url_sound(&mut self, url: &str) -> Result<(), JsValue> {
        // self.audio_manager.load_sound(url).await
        self.audio_manager.load_sound_from_url(url).await
    }

    pub fn play_sound(&mut self) -> Result<(), JsValue> {
        self.audio_manager.play_sound_at_pos(None)
    }

    pub fn set_sound_position(&mut self, x: f32, y: f32, z: f32) {
        self.audio_manager.set_panner_position(x, y, z);
    }

    pub fn is_audio_manager_debug(&mut self) -> bool {
        self.audio_manager.is_debug()
    }

    fn update_audio_manager(&mut self) {
        // Apply debug tick updates
        // if self.audio_manager.is_debug() {
        //     self.audio_manager.update_debug_sound_on_tick();
        // }

        // Get the camera's position and rotation based on the current game state
        let (position, rotation) = match &self.state {
            GameState::Playing { camera, .. } | GameState::Editing { camera, .. } => {
                camera.position_and_rotation()
            }
            GameState::Loading => return,
        };

        // Update the listener's position and orientation
        self.audio_manager
            .set_listener_position(position.x, position.y, position.z);

        let forward = (rotation * Vec3::new(0.0, 0.0, -1.0)).normalize();
        let up = (rotation * Vec3::new(0.0, 1.0, 0.0)).normalize();

        self.audio_manager
            .set_listener_orientation(forward.x, forward.y, forward.z, up.x, up.y, up.z);
    }
}

#[derive(Clone, Default)]
struct Controls {
    keyboard_inputs: HashSet<String>,
    keyboard_pressed: HashSet<String>,
    mouse_movement: (i32, i32),
    mouse_left: bool,
    mouse_right: bool,

    // Yaw and pitch, radians
    yaw: f32,
    pitch: f32,
}

#[derive(Clone, Debug, Default)]
struct Player {
    position: Vec3,
}

#[derive(Debug, Default)]
enum GameState {
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

// Handlers for incoming packets

/// Handle a `SetBlock` packet
fn handle_set_block(
    blocks: &mut BlockGrid,
    net_types::SetBlock { position, block_id }: net_types::SetBlock,
) -> Result<()> {
    blocks[position] = block_id;
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
) {
    let Some(player) = players.get_mut(&id) else {
        tracing::warn!("Received update position for unknown player {id:?}");
        return;
    };
    player.position = position;
}

fn collect_block_textures(
    block_registry: &BlockRegistry,
    renderer: &render::Renderer,
    assets: &mut Assets,
) -> Option<Vec<[render::Texture; 6]>> {
    block_registry
        .iter()
        .map(|block_type| {
            let load_image = |data: &Vec<u8>| match image::load_from_memory(data) {
                Ok(img) => {
                    let (width, height) = img.dimensions();
                    let data = img.as_rgba8()?.as_raw();
                    tracing::info!(
                        "Loaded image for block {}: {width}x{height}",
                        block_type.name
                    );
                    Some(renderer.create_texture_from_image(data, width, height))
                }
                Err(e) => {
                    tracing::error!("Failed to load image: {e}");
                    None
                }
            };

            let top = assets.get(&block_type.top_texture).and_then(load_image)?;
            let bottom = assets
                .get(&block_type.bottom_texture)
                .and_then(load_image)?;
            let east = assets.get(&block_type.east_texture).and_then(load_image)?;
            let west = assets.get(&block_type.west_texture).and_then(load_image)?;
            let north = assets.get(&block_type.north_texture).and_then(load_image)?;
            let south = assets.get(&block_type.south_texture).and_then(load_image)?;

            // TODO(ll): I just threw these in here, I don't know that they are in the right order
            Some([north, south, east, west, top, bottom])
        })
        .collect::<Option<Vec<_>>>()
}

const MOUSE_SENSITIVITY_X: f32 = 0.005;
const MOUSE_SENSITIVITY_Y: f32 = 0.005;

const CAMERA_DISTANCE: f32 = 15.0;
const CAMERA_HEIGHT: f32 = 2.0;
