use std::time::Duration;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

mod gltf;
mod render;

// Enable console.log for debugging
#[macro_export]
macro_rules! console_log {
    ($($t:tt)*) => (web_sys::console::log_1(&format!($($t)*).into()))
}

#[wasm_bindgen]
struct Engine {
    renderer: render::Renderer,

    elapsed_time: Duration,
}

#[wasm_bindgen]
impl Engine {
    pub fn new() -> Result<Self, JsValue> {
        // Get the window, etc.
        let window = web_sys::window().ok_or("Could not access window")?;
        let document = window.document().ok_or("Could not access document")?;

        // Access the canvas element
        let canvas = document
            .get_element_by_id("canvas")
            .ok_or("Canvas element not found")?;
        let canvas: HtmlCanvasElement = canvas.dyn_into::<HtmlCanvasElement>()?;

        let renderer = render::Renderer::new(canvas)?;

        Ok(Self {
            renderer,
            elapsed_time: Duration::ZERO,
        })
    }

    pub fn key_down(&self, event: KeyboardEvent) {
        console_log!("Key pressed: {}", event.key());

        if event.code() == "KeyW" {
            match gltf::load(include_bytes!("gltf/test.glb")) {
                Err(e) => {
                    console_log!("Error loading GLTF: {e:#?}");
                }
                Ok(s) => console_log!("Loaded GLTF: {s:#?}"),
            }
        }
    }

    pub fn tick(&mut self, time: f64) {
        let current_time = Duration::from_secs_f64(time / 1000.0);
        let delta_time = current_time - self.elapsed_time;
        self.elapsed_time = current_time;

        self.renderer.render(self.elapsed_time);
    }
}

#[wasm_bindgen]
pub fn increment(count: i32) -> i32 {
    count + 1
}
