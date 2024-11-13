use {
    crate::{
        assets::Assets, camera::FlyCamera, gltf::GLTFModel, socket::ConnectionState,
        transform::Transform,
    },
    anyhow::Result,
    blocks::BlockTypeID,
    dolly::prelude::YawPitch,
    glam::{EulerRot, Quat, Vec2, Vec3},
    image::GenericImageView,
    net_types::ServerPacket,
    std::{
        cell::RefCell,
        collections::{HashMap, HashSet},
        rc::Rc,
        slice,
        time::Duration,
    },
    web_sys::{MouseEvent, WebSocket},
};

// Re-exports
pub use blocks::BlockPos;

use blocks::BlockType;
use game_state::GameState;
use glam::UVec2;
use nanorand::Rng;
use net_types::ClientPacket;
use render::{Renderer, Texture};
use socket::IncomingMessages;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

mod assets;
mod audio;
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

        let audio_manager = audio::AudioManager::new()?;

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
                            packet_handlers::handle_add_player(
                                players,
                                &self.player_model.gltf,
                                add_player,
                            )
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

        if self.is_audio_manager_debug() {
            // Test: Spawn hurt at position with left click
            spawn_test_sound_at_pos_on_left_click(self, "pain");
            // Test: Spawn "kane" at entity with right click
            spawn_sound_at_kane_face(self, "kane");
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
                selected_block_id,
                selected_entity_type_id,
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
                    if selected_block_id.is_some() {
                        tracing::debug!("Placing block at {target_raycast:?}");
                        self.place_block();
                    } else if selected_entity_type_id.is_some() {
                        tracing::debug!("Placing entity at {target_raycast:?}");
                        self.place_entity();
                    }
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

        if let GameState::Playing { players, .. } = &mut self.state {
            for player in players.values_mut() {
                gltf::animate_model(&mut player.model, self.delta_time);
            }
        }

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

    fn place_entity(&mut self) {
        // Ensure we're in the editing state and we have a selected block ID
        let GameState::Editing {
            target_raycast: Some(ref target_raycast),
            selected_entity_type_id: Some(entity_type_id),
            ..
        } = self.state
        else {
            return;
        };

        let is_deleting = false;
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

        // Idk how to make an entity_id
        let entity_id = nanorand::tls_rng().generate::<u64>();

        let add_entity = net_types::AddEntity {
            entity_id: format!("{entity_id:x}"),
            entity_data: entities::EntityData {
                name: "Jeff".into(),
                entity_type: entity_type_id,
                model_path: "kibble_ctf/test_entity.gltf".into(),
                state: entities::EntityState {
                    position: position.into(),
                    velocity: Vec3::ZERO,
                },
            },
        };

        tracing::debug!("Setting entity at {position:?} to {entity_type_id}");

        self.send_packet(ClientPacket::AddEntity(add_entity));
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
                // HACK: The player model is rotated 90 degrees, also
                // it rotates the wrong way? I'm just fixing it here but someone
                // should figure out why it is like this.
                const PLAYER_BASE_ANGLE: f32 = std::f32::consts::FRAC_PI_2;
                for player in players.values() {
                    draw_calls.extend(render::build_render_plan(
                        slice::from_ref(&player.model),
                        slice::from_ref(&self.player_model.render_model),
                        Transform::new(
                            player.position,
                            Quat::from_rotation_y(-player.facing_angle - PLAYER_BASE_ANGLE),
                        ),
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

    pub async fn load_sounds_into_bank(&mut self) -> Result<(), JsValue> {
        self.audio_manager.load_sounds_into_bank().await
    }

    pub async fn load_sound(&mut self, sound_id: &str) -> Result<(), JsValue> {
        // self.audio_manager.load_sound(url).await
        self.audio_manager.load_sound_from_id(sound_id).await
    }

    pub async fn load_url_sound(&mut self, url: &str) -> Result<(), JsValue> {
        // self.audio_manager.load_sound(url).await
        self.audio_manager.load_sound_from_url(url).await
    }

    /// Plays a sound associated with a specific entity.
    ///
    /// ## Parameters:
    ///
    /// * `sound_id` - the name of the sound to play
    /// * `entity_id` - The identifier of the entity to associate the sound with.
    /// * TODO - remaining params
    pub fn play_sound_at_entity(
        &mut self,
        sound_id: &str,
        entity_id: entities::EntityID,
        // is_ambient: bool,
        is_looping: bool,
    ) -> Result<(), JsValue> {
        // Retrieve the entity's current position
        let position = self
            .get_entity_sound_pos(entity_id.clone())
            .ok_or_else(|| {
                let error_msg = format!("Entity ID {} not found", entity_id);
                web_sys::console::error_1(&error_msg.clone().into());
                JsValue::from_str(&error_msg)
            })?;

        // Play the sound at the entity's position
        self.audio_manager
            .spawn_sound(sound_id, Some(entity_id), Some(position), false, is_looping)
    }

    fn get_entity_sound_pos(&self, entity_id: entities::EntityID) -> Option<audio::SoundPosition> {
        match &self.state {
            GameState::Playing { entities, .. } | GameState::Editing { entities, .. } => entities
                .get(&entity_id)
                .map(|entity_data| audio::SoundPosition {
                    x: entity_data.state.position.x,
                    y: entity_data.state.position.y,
                    z: entity_data.state.position.z,
                }),
            GameState::Loading => None,
        }
    }

    pub fn play_sound(
        &mut self,
        sound_id: &str,
        is_ambient: bool,
        is_looping: bool,
    ) -> Result<(), JsValue> {
        self.audio_manager
            .spawn_sound(sound_id, None, None, is_ambient, is_looping)
    }

    pub fn play_sound_at_pos(
        &mut self,
        sound_id: &str,
        x: f32,
        y: f32,
        z: f32,
        is_ambient: bool,
        is_looping: bool,
    ) -> Result<(), JsValue> {
        let sound_position = audio::SoundPosition::new(x, y, z);
        self.audio_manager
            .spawn_sound(sound_id, None, Some(sound_position), is_ambient, is_looping)
    }

    // if `true` then TestStopSounds Component will be rendered
    // and spawn_debug_sound_on_left_click will be enabled
    pub fn is_audio_manager_debug(&mut self) -> bool {
        true
    }

    pub fn kill_sounds(&mut self) -> Result<(), JsValue> {
        self.audio_manager.clear_sounds_bank()
    }

    pub fn stop_sounds(&mut self) -> Result<(), JsValue> {
        self.audio_manager.stop_all_sounds()
    }

    // TODO delete or rename
    pub fn move_all_panner_nodes(&mut self, move_panner_opt: Option<f32>) {
        self.audio_manager.move_all_panner_nodes(move_panner_opt);
    }

    fn update_audio_manager(&mut self) {
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

        // Update the positions of all active sounds and handle cleanup for non-existent entities
        // Access entities based on the current game state
        let entities = match &self.state {
            GameState::Playing { entities, .. } | GameState::Editing { entities, .. } => entities,
            GameState::Loading => return, // Early exit if the game is loading
        };

        // Step 1: Collect positions of entities with active sounds
        let mut positions = HashMap::new();

        for (entity_id, entity_data) in entities.iter() {
            if self.audio_manager.has_active_sound(entity_id.clone()) {
                positions.insert(entity_id.clone(), entity_data.state.position);
            }
        }

        // Update sound positions in AudioManager
        self.audio_manager.synchronise_positions(&positions);
        // Collect existing entity IDs for cleanup
        let existing_entity_ids: HashSet<entities::EntityID> = entities.keys().cloned().collect();
        // Clean up sounds associated with non-existent entities
        self.audio_manager
            .cleanup_entity_sounds(&existing_entity_ids);

        // We also need to cleanup up one shot (non looping) sounds that have finished playing
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
    facing_angle: f32, // radians
    model: GLTFModel,
}

const MOUSE_SENSITIVITY_X: f32 = 0.005;
const MOUSE_SENSITIVITY_Y: f32 = 0.005;

const CAMERA_DISTANCE: f32 = 15.0;
const CAMERA_HEIGHT: f32 = 2.0;

// Will fail if the sound hasn't been loaded
pub fn spawn_test_sound_at_pos_on_left_click(engine: &mut Engine, sound_id: &str) {
    if let GameState::Editing { target_raycast, .. } = &mut engine.state {
        if engine.controls.mouse_left {
            if let Some(ray_hit) = target_raycast {
                let pos = ray_hit.position;
                if let Err(_) = engine.play_sound_at_pos(
                    sound_id,
                    pos.x as f32,
                    pos.y as f32,
                    pos.z as f32,
                    false,
                    false,
                ) {
                    tracing::debug!("Failed to play_sound_at_pos: {:?}", pos);
                }
            }
        }
    }
}

pub fn spawn_sound_at_kane_face(engine: &mut Engine, sound_id: &str) {
    // Early exit if the game state is Loading or if debug is off and the right mouse button is not pressed.
    if matches!(engine.state, GameState::Loading) || !engine.controls.mouse_right {
        return;
    }

    let entity_id = "0".to_string();
    let is_looping = true;

    // Attempt to play the sound at the specified entity; log the result.
    match engine.play_sound_at_entity(sound_id, entity_id.clone(), is_looping) {
        Ok(_) => tracing::debug!(
            "Successfully played sound '{}' at EntityID '{}'",
            sound_id,
            entity_id
        ),
        Err(e) => tracing::debug!(
            "Failed to play sound '{}' at EntityID '{}': {:?}",
            sound_id,
            entity_id,
            e
        ),
    }
}
