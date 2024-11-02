use std::{slice, time::Duration};

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

// Import the necessary web_sys features
use web_sys::{HtmlCanvasElement, KeyboardEvent};

mod gltf;
mod render;
mod transform;

struct TestGltf {
    gltf: gltf::GLTFModel,
    render_model: render::RenderModel,
}

#[wasm_bindgen]
struct Engine {
    renderer: render::Renderer,

    elapsed_time: Duration,
    test: Option<TestGltf>,
}

#[wasm_bindgen]
impl Engine {
    pub fn new() -> Result<Self, JsValue> {
        tracing_wasm::set_as_global_default();

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
            test: None,
        })
    }

    pub fn key_down(&mut self, event: KeyboardEvent) {
        tracing::info!("Key pressed: {}", event.key());

        if event.code() == "KeyW" {
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
    }

    pub fn tick(&mut self, time: f64) {
        let current_time = Duration::from_secs_f64(time / 1000.0);
        let delta_time = current_time - self.elapsed_time;
        self.elapsed_time = current_time;

        let draw_calls;
        if let Some(ref test) = self.test {
            draw_calls = render::build_render_plan(
                slice::from_ref(&test.gltf),
                slice::from_ref(&test.render_model),
            );

            tracing::info!("Draw calls created: {:#?}", draw_calls);
        } else {
            draw_calls = Vec::new();
        }

        self.renderer.render(self.elapsed_time, &draw_calls);
    }
}

#[wasm_bindgen]
pub fn increment(count: i32) -> i32 {
    count + 1
}
