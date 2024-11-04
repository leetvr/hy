use std::collections::HashSet;

use {
    crate::{socket::ConnectionState, transform::Transform},
    anyhow::{Context, Result},
    glam::{Quat, Vec3},
    net::PlayerId,
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

    players: HashMap<PlayerId, Player>,
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
            players: Default::default(),
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

            let mut incoming_messages = self.incoming_messages.borrow_mut();
            for message in incoming_messages.drain(..) {
                let packet: net::ServerPacket =
                    bincode::deserialize(&message).expect("Failed to deserialize position update");
                match packet {
                    net::ServerPacket::UpdatePosition(net::UpdatePosition { id, position }) => {
                        let Some(player) = self.players.get_mut(&id) else {
                            tracing::error!("Received position update for unknown player");
                            continue;
                        };
                        player.position = position;
                    }
                    net::ServerPacket::AddPlayer(net::AddPlayer { id, position }) => {
                        self.players.insert(id, Player { position });
                    }
                    net::ServerPacket::RemovePlayer(net::RemovePlayer { id }) => {
                        self.players.remove(&id);
                    }
                }
            }
        }

        match &*self.connection_state.borrow() {
            ConnectionState::Error(e) => {
                panic!("Error in websocket connection: {e:#}");
            }
            _ => {}
        }

        let mut draw_calls = Vec::new();
        for player in self.players.values() {
            draw_calls.extend(render::build_render_plan(
                slice::from_ref(&self.player_model.gltf),
                slice::from_ref(&self.player_model.render_model),
                Transform::new(
                    Vec3::new(player.position.x, player.position.y, 0.),
                    Quat::IDENTITY,
                ),
            ));
        }

        if let Some(ref test) = self.test {
            draw_calls.extend(render::build_render_plan(
                slice::from_ref(&test.gltf),
                slice::from_ref(&test.render_model),
                Transform::IDENTITY,
            ));

            tracing::debug!("Draw calls created: {:#?}", draw_calls);
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

fn send_controls(controls: &Controls, ws: &WebSocket) -> Result<()> {
    let mut move_dir = glam::Vec2::ZERO;
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
    let controls = net::Controls {
        move_direction: move_dir.normalize_or_zero(),
    };
    let message = bincode::serialize(&controls).context("Failed to serialize controls")?;
    ws.send_with_u8_array(&message)
        .expect("Failed to send controls");

    Ok(())
}

struct Player {
    position: glam::Vec2,
}
