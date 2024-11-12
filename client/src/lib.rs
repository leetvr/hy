use {net_types::ServerPacket, std::collections::HashMap};

use {
    crate::{assets::Assets, camera::FlyCamera},
    blocks::BlockTypeID,
    dolly::prelude::YawPitch,
    glam::EulerRot,
    image::GenericImageView,
    std::collections::HashSet,
    web_sys::MouseEvent,
};

use {
    crate::{socket::ConnectionState, transform::Transform},
    anyhow::Result,
    glam::{Quat, Vec2, Vec3},
    std::{cell::RefCell, rc::Rc, slice, time::Duration},
    web_sys::WebSocket,
};

// Re-exports
pub use blocks::BlockPos;

use blocks::BlockType;
use game_state::GameState;
use glam::UVec2;
use net_types::ClientPacket;
use render::{Renderer, Texture};
use socket::IncomingMessages;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

mod assets;
mod camera;
mod context;
mod game_state;
mod gltf;
mod packet_handlers;
mod render;
mod socket;
mod transform;

struct LoadedGLTF {
    gltf: gltf::GLTFModel,
    render_model: render::RenderModel,
}

#[wasm_bindgen]
pub struct Engine {
    context: context::Context,
    renderer: render::Renderer,

    elapsed_time: Duration,
    delta_time: Duration,
    test: Option<LoadedGLTF>,

    ws: WebSocket,
    connection_state: Rc<RefCell<ConnectionState>>,
    incoming_messages: IncomingMessages,
    last_seen_sequence_number: u64,

    controls: Controls,
    player_model: LoadedGLTF,

    cube_mesh_data: render::CubeVao,
    // Textures by block type ID
    block_textures: HashMap<BlockTypeID, [render::Texture; 6]>,

    // Entity models by path
    entity_models: HashMap<String, LoadedGLTF>,

    debug_lines: Vec<render::DebugLine>,

    assets: assets::Assets,
    state: GameState,
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
        let incoming_messages = IncomingMessages::default();
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
            LoadedGLTF { gltf, render_model }
        };

        Ok(Self {
            context: context::Context::new(canvas),
            cube_mesh_data: renderer.create_cube_vao(),
            block_textures: Default::default(),
            entity_models: Default::default(),

            renderer,
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            test: None,

            ws,
            connection_state,
            incoming_messages,
            last_seen_sequence_number: 0,

            controls: Default::default(),
            player_model,

            debug_lines: Vec::new(),

            assets: Assets::new(),
            state: Default::default(),
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

            self.test = Some(LoadedGLTF { gltf, render_model });
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

        // Receive packets
        if *self.connection_state.borrow() == ConnectionState::Connected {
            loop {
                if self.incoming_messages.borrow().is_empty() {
                    break;
                }
                // Incoming messages are stored by their sequence number
                let Some(packet) = self
                    .incoming_messages
                    .borrow_mut()
                    .remove(&self.last_seen_sequence_number)
                else {
                    break;
                };

                // Increment our sequence number
                self.last_seen_sequence_number += 1;

                match &mut self.state {
                    GameState::Loading => match packet {
                        ServerPacket::Init(net_types::Init {
                            blocks,
                            block_registry,
                            entities,
                            entity_type_registry,
                            ..
                        }) => {
                            tracing::info!("Loaded level of size {:?}", blocks.size());
                            tracing::info!("Block registry: {:#?}", block_registry);

                            // Start fetching assets
                            self.assets.load_block_textures(&block_registry);
                            self.assets.load_entity_models(entities.values());

                            // Tell the React frontend
                            if let Some(on_init) = self.context.on_init_callback.take() {
                                let block_registry =
                                    serde_wasm_bindgen::to_value(&block_registry).unwrap();
                                let entity_type_registry =
                                    serde_wasm_bindgen::to_value(&entity_type_registry).unwrap();
                                on_init
                                    .call2(&JsValue::NULL, &block_registry, &entity_type_registry)
                                    .expect("Unable to call on_init!");
                            }

                            self.state = GameState::Editing {
                                blocks,
                                block_registry,
                                entities,
                                entity_type_registry,
                                camera: FlyCamera::new(Vec3::ZERO),
                                target_raycast: None,
                                selected_block_id: None,
                                selected_entity_type_id: None,
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
                        entities,
                        blocks,
                        client_player,
                        ..
                    } => match packet {
                        ServerPacket::SetBlock(set_block) => {
                            packet_handlers::handle_set_block(blocks, set_block)
                                .expect("Failed to set block");
                        }
                        ServerPacket::AddPlayer(add_player) => {
                            packet_handlers::handle_add_player(players, add_player)
                                .expect("Failed to add player");
                        }
                        ServerPacket::UpdatePlayer(update_position) => {
                            packet_handlers::handle_update_position(players, update_position);
                        }
                        ServerPacket::RemovePlayer(remove_player) => {
                            packet_handlers::handle_remove_player(players, remove_player)
                                .expect("Failed to remove player");
                        }
                        // Sent by the server when we leave edit mode
                        ServerPacket::Reset(net_types::Reset {
                            new_client_player, ..
                        }) => {
                            players.clear();
                            *client_player = new_client_player;
                        }
                        ServerPacket::AddEntity(add_entity) => {
                            packet_handlers::handle_add_entity(entities, add_entity);
                        }
                        ServerPacket::UpdateEntity(update_entity) => {
                            if let Err(e) =
                                packet_handlers::handle_update_entity(entities, update_entity)
                            {
                                tracing::error!("Error when handling UpdateEntity: {e:#}");
                            }
                        }
                        ServerPacket::RemoveEntity(remove_entity) => {
                            packet_handlers::handle_remove_entity(entities, remove_entity);
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

        self.load_block_textures();

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
                move_dir = move_dir.normalize_or_zero();
                let controls = net_types::Controls {
                    move_direction: move_dir.normalize_or_zero(),
                    jump: self.controls.keyboard_pressed.contains("Space"),
                    camera_yaw: self.controls.yaw,
                };
                self.send_packet(net_types::ClientPacket::Controls(controls));
            }
            GameState::Editing {
                camera,
                blocks,
                target_raycast,
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

                *target_raycast = blocks.raycast(position, ray_dir);

                if self.controls.mouse_left {
                    tracing::debug!("Placing block at {target_raycast:?}");
                    self.place_block();
                }

                // Send empty player input
                let controls = net_types::Controls {
                    move_direction: Vec2::ZERO,
                    jump: false,
                    camera_yaw: 0.0,
                };
                self.send_packet(net_types::ClientPacket::Controls(controls));
            }
            _ => {}
        }

        self.controls.keyboard_pressed.clear();
        self.controls.mouse_movement = (0, 0);
        self.controls.mouse_left = false;
        self.controls.mouse_right = false;

        self.debug_lines.push(render::DebugLine::new(
            Vec3::new(0.0, 3.0, 0.0),
            Vec3::new(0.0, 3.0, 10.0),
        ));

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
            ref mut blocks,
            target_raycast: Some(ref target_raycast),
            selected_block_id: Some(block_id),
            ..
        } = self.state
        else {
            return;
        };

        let is_deleting = block_id == 0;
        let position = match is_deleting {
            // When deleting a block, we place it at the position of the raycast.
            true => target_raycast.position,
            false => {
                // When we place a block, we place it at the position of the raycast, but offset by
                // the entrance face normal, as we're placing it "on" the face the ray entered.
                let Some(position) = target_raycast
                    .position
                    .add_signed(target_raycast.entrance_face_normal.as_ivec3())
                else {
                    return;
                };

                position
            }
        };

        let set_block = net_types::SetBlock { position, block_id };

        tracing::debug!("Setting block at {position:?} to {block_id}");

        // cheeky: We pretend we received a `set_block` packet
        packet_handlers::handle_set_block(blocks, set_block).expect("place block");

        self.send_packet(ClientPacket::SetBlock(set_block));
    }

    fn load_block_textures(&mut self) {
        let block_registry = match &self.state {
            GameState::Loading => return,
            GameState::Playing { block_registry, .. }
            | GameState::Editing { block_registry, .. } => block_registry,
        };

        let block_textures = &mut self.block_textures;

        for (block_type_id, block_type) in block_registry.iter().enumerate() {
            let block_type_id = block_type_id as BlockTypeID + 1;
            load_textures_for_block(
                block_type,
                block_type_id,
                &self.assets,
                &mut self.renderer,
                block_textures,
            );
        }
    }

    fn render(&mut self) {
        let mut draw_calls = Vec::new();
        self.load_entity_models();

        // Gather blocks and entities
        match &self.state {
            GameState::Playing {
                blocks, entities, ..
            }
            | GameState::Editing {
                blocks, entities, ..
            } => {
                // Collect blocks
                let block_to_remove = match self.state {
                    GameState::Editing {
                        target_raycast: Some(ref raycast),
                        selected_block_id: Some(0),
                        ..
                    } => Some(raycast.position),
                    _ => None,
                };

                let blocks_to_render =
                    blocks.iter_non_empty().filter_map(|(pos, block_type_id)| {
                        if let Some(ref block_to_remove) = block_to_remove {
                            if *block_to_remove == pos {
                                return None;
                            }
                        }

                        let textures = self.block_textures.get(&block_type_id)?;

                        Some((pos, textures))
                    });

                draw_calls.extend(
                    render::build_cube_draw_calls(&self.cube_mesh_data, blocks_to_render, None)
                        .into_iter(),
                );

                if let Some(block_to_remove) = block_to_remove {
                    let block_type_id = blocks.get(block_to_remove).copied().unwrap();
                    if block_type_id != 0 {
                        if let Some(textures) = self.block_textures.get(&block_type_id) {
                            let blocks = [(block_to_remove, textures)];
                            draw_calls.extend(
                                render::build_cube_draw_calls(
                                    &self.cube_mesh_data,
                                    blocks.into_iter(),
                                    Some([1.0, 0.0, 0., 1.0].into()),
                                )
                                .into_iter(),
                            );
                        }
                    }
                }

                for entity in entities.values() {
                    let Some(model) = self.entity_models.get(&entity.model_path) else {
                        continue;
                    };

                    draw_calls.extend(render::build_render_plan(
                        slice::from_ref(&model.gltf),
                        slice::from_ref(&model.render_model),
                        Transform::new(entity.state.position, Quat::IDENTITY),
                    ));
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
                target_raycast: Some(raycast),
                selected_block_id: Some(block_id),
                ..
            } if *block_id != 0 => {
                if let Some(textures) = self.block_textures.get(block_id) {
                    if let Some(block_position) = raycast
                        .position
                        .add_signed(raycast.entrance_face_normal.as_ivec3())
                    {
                        let blocks = [(block_position, textures)];

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

        self.renderer.render(&draw_calls, &self.debug_lines);

        self.debug_lines.clear();
    }

    fn load_entity_models(&mut self) {
        let entities = match &self.state {
            GameState::Editing { entities, .. } | GameState::Playing { entities, .. } => entities,
            _ => return,
        };

        for model_name in entities.values().map(|e| &e.model_path) {
            // If we've already loaded this model, continue
            if self.entity_models.contains_key(model_name) {
                continue;
            }

            // If we don't have the data yet, continue
            let Some(data) = self.assets.get_or_load(model_name) else {
                continue;
            };

            // Load the glTF
            let loaded = {
                let gltf =
                    gltf::load(&data).unwrap_or_else(|e| panic!("Error loading GLTF: {e:#?}"));
                let render_model = render::RenderModel::from_gltf(&self.renderer, &gltf);
                LoadedGLTF { gltf, render_model }
            };

            // Stash it in our map
            self.entity_models.insert(model_name.into(), loaded);
        }
    }
}

fn load_textures_for_block(
    block_type: &BlockType,
    block_type_id: BlockTypeID,
    assets: &Assets,
    renderer: &mut Renderer,
    block_textures: &mut HashMap<BlockTypeID, [Texture; 6]>,
) {
    // If you're loaded already, whatever, trevor.
    if block_textures.contains_key(&block_type_id) {
        return;
    };

    // Collect all the texture data
    let Some(north) = assets.get(&block_type.north_texture) else {
        return;
    };

    let Some(south) = assets.get(&block_type.south_texture) else {
        return;
    };

    let Some(east) = assets.get(&block_type.east_texture) else {
        return;
    };

    let Some(west) = assets.get(&block_type.west_texture) else {
        return;
    };

    let Some(top) = assets.get(&block_type.top_texture) else {
        return;
    };

    let Some(bottom) = assets.get(&block_type.bottom_texture) else {
        return;
    };

    let textures = [north, south, east, west, top, bottom].map(|image_data| {
        load_texture_from_image(renderer, &image_data).expect("Failed to load texture")
    });

    block_textures.insert(block_type_id, textures);
}

fn load_texture_from_image(renderer: &mut Renderer, image_data: &[u8]) -> anyhow::Result<Texture> {
    let image = image::load_from_memory(image_data)?;
    let (width, height) = image.dimensions();
    let image = image.into_rgba8();
    let data = image.as_raw();

    Ok(renderer.create_texture_from_image(data, width, height))
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

const MOUSE_SENSITIVITY_X: f32 = 0.005;
const MOUSE_SENSITIVITY_Y: f32 = 0.005;

const CAMERA_DISTANCE: f32 = 15.0;
const CAMERA_HEIGHT: f32 = 2.0;
