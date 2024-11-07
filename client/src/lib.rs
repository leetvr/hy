use {
    crate::camera::FlyCamera, dolly::prelude::YawPitch, glam::EulerRot, std::collections::HashSet,
    web_sys::MouseEvent,
};

use {
    crate::{socket::ConnectionState, transform::Transform},
    anyhow::{Context, Result},
    blocks::BlockGrid,
    glam::{Mat4, Quat, Vec2, Vec3},
    net_types::PlayerId,
    std::{cell::RefCell, collections::HashMap, rc::Rc, slice, time::Duration},
    web_sys::WebSocket,
};

use blocks::BlockPos;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

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
    cube_texture: render::Texture,

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

        let cube_texture = renderer.create_texture_from_color([255, 0, 0, 255]);

        Ok(Self {
            context: context::Context::new(canvas),
            cube_mesh_data: renderer.create_cube_vao(),

            renderer,
            delta_time: Duration::ZERO,
            elapsed_time: Duration::ZERO,
            test: None,

            ws,
            connection_state,
            incoming_messages,

            controls: Default::default(),
            player_model,

            cube_texture,

            game_state: Default::default(),
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
                // Incoming messages are pushed to the back of the queue, so we process them from
                // the front
                let message = self.incoming_messages.borrow_mut().remove(0);

                let packet: net_types::ServerPacket =
                    bincode::deserialize(&message).expect("Failed to deserialize position update");

                match &mut self.game_state {
                    GameState::Loading => match packet {
                        net_types::ServerPacket::Init(net_types::Init {
                            blocks,
                            client_player,
                        }) => {
                            tracing::info!("Loaded level of size {:?}", blocks.size());
                            let blocks_primitive = self.renderer.create_block_primitive(
                                blocks.iter_non_empty().map(|(pos, _)| pos),
                            );

                            self.game_state = GameState::Playing {
                                blocks,
                                blocks_primitive,
                                players: HashMap::new(),
                                client_player,
                                camera: FlyCamera::new(Vec3::ZERO),
                            };

                            // When we've connected, tell the server we want to switch to edit mode.
                            self.ws
                                .send_with_u8_array(
                                    &bincode::serialize(&net_types::ClientPacket::Edit).unwrap(),
                                )
                                .expect("Failed to send message");
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
                        client_player,
                        ..
                    } => match packet {
                        net_types::ServerPacket::SetBlock(set_block) => {
                            handle_set_block(&self.renderer, blocks, blocks_primitive, set_block)
                                .expect("Failed to set block");
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
        match &mut self.game_state {
            GameState::Playing {
                players,
                client_player,
                camera,
                ..
            } => {
                let delta_yaw = -self.controls.mouse_movement.0 as f32 * MOUSE_SENSITIVITY_X;
                let delta_pitch = -self.controls.mouse_movement.1 as f32 * MOUSE_SENSITIVITY_Y;

                match self.context.mode {
                    context::EngineMode::Play => {
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

                        self.renderer.camera.translation = position;
                        self.renderer.camera.rotation = rotation;

                        // Keep editor camera in sync with player camera
                        camera.set_position_and_rotation(
                            position,
                            YawPitch::new().rotation_quat(rotation),
                        );
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
                        move_dir = Vec2::from_angle(move_dir.to_angle() + self.controls.yaw)
                            * move_dir.length();
                        let controls = net_types::Controls {
                            move_direction: move_dir.normalize_or_zero(),
                            jump: self.controls.keyboard_pressed.contains("Space"),
                        };
                        let message =
                            bincode::serialize(&net_types::ClientPacket::Controls(controls))
                                .unwrap();
                        self.ws
                            .send_with_u8_array(&message)
                            .expect("Failed to send controls");
                    }
                    context::EngineMode::Edit => {
                        // Camera input
                        let key_state = |code| -> f32 {
                            self.controls
                                .keyboard_inputs
                                .contains(code)
                                .then(|| 1.0)
                                .unwrap_or(0.0)
                        };

                        camera.movement_forward = key_state("KeyW");
                        camera.movement_backward = key_state("KeyS");
                        camera.movement_left = key_state("KeyA");
                        camera.movement_right = key_state("KeyD");
                        camera.movement_up = key_state("Space");
                        camera.movement_down = key_state("ShiftLeft");
                        camera.boost = key_state("CtrlLeft");
                        camera.rotate(delta_yaw.to_degrees(), delta_pitch.to_degrees());

                        camera.update(self.delta_time.as_secs_f32());

                        let (position, rotation) = camera.position_and_rotation();
                        self.renderer.camera.translation = position;
                        self.renderer.camera.rotation = rotation;

                        // Send empty player input
                        let controls = net_types::Controls {
                            move_direction: Vec2::ZERO,
                            jump: false,
                        };
                        let message =
                            bincode::serialize(&net_types::ClientPacket::Controls(controls))
                                .unwrap();
                        self.ws
                            .send_with_u8_array(&message)
                            .expect("Failed to send controls");
                    }
                }
            }
            _ => {}
        }

        self.controls.keyboard_pressed.clear();
        self.controls.mouse_movement = (0, 0);

        self.render();
    }

    fn render(&mut self) {
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

        if let Some(ref mut test) = self.test {
            gltf::animate_model(&mut test.gltf, self.delta_time);

            draw_calls.extend(render::build_render_plan(
                slice::from_ref(&test.gltf),
                slice::from_ref(&test.render_model),
                Transform::IDENTITY,
            ));
        }

        draw_calls.extend(render::build_cube_draw_calls(
            &self.cube_mesh_data,
            [(BlockPos::new(0, 0, 0), [&self.cube_texture; 6])].into_iter(),
        ));

        self.renderer.render(&draw_calls);
    }
}
#[wasm_bindgen]
pub fn increment(count: i32) -> i32 {
    count + 1
}

#[derive(Clone, Default)]
struct Controls {
    keyboard_inputs: HashSet<String>,
    keyboard_pressed: HashSet<String>,
    mouse_movement: (i32, i32),

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
        client_player: PlayerId,
        camera: FlyCamera,
        blocks_primitive: render::RenderPrimitive,
        players: HashMap<PlayerId, Player>,
    },
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
) {
    let Some(player) = players.get_mut(&id) else {
        tracing::warn!("Received update position for unknown player {id:?}");
        return;
    };
    player.position = position;
}

const MOUSE_SENSITIVITY_X: f32 = 0.005;
const MOUSE_SENSITIVITY_Y: f32 = 0.005;

const CAMERA_DISTANCE: f32 = 15.0;
const CAMERA_HEIGHT: f32 = 2.0;
