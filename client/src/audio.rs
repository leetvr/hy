use std::default;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::ArrayBuffer, window, AudioBuffer, AudioBufferSourceNode, AudioContext, AudioListener,
    AudioParam, GainNode, OscillatorNode, PannerNode, Response,
};

// TODO
// Figure out sound imports
// Extract and test distortion from `AudioPlayer`
// Attach a sound to an entity (Player initially)
// Make sure sound spatialisation updates in relation to listener
// Refactor AudioManager to keep track of a set of different `SoundInstance`s

#[wasm_bindgen]
pub struct AudioManager {
    context: AudioContext,
    gain_node: GainNode,
    sound_buffer: Option<AudioBuffer>,
    source_node: Option<AudioBufferSourceNode>,
    panner_node: Option<PannerNode>,
}

#[wasm_bindgen]
impl AudioManager {
    pub fn new() -> Result<AudioManager, JsValue> {
        let context = AudioContext::new()?;
        let gain_node = context.create_gain()?;
        gain_node.connect_with_audio_node(&context.destination())?;

        Ok(AudioManager {
            context,
            gain_node,
            sound_buffer: None,
            source_node: None,
            panner_node: None,
        })
    }

    pub async fn load_sound(&mut self, url: &str) -> Result<(), JsValue> {
        let audio_buffer = self.load_audio_buffer(url).await?;
        self.sound_buffer = Some(audio_buffer);
        web_sys::console::log_1(&"Sound buffer loaded successfully".into());
        Ok(())
    }

    async fn load_audio_buffer(&mut self, url: &str) -> Result<AudioBuffer, JsValue> {
        let window = web_sys::window().unwrap();
        let response = JsFuture::from(window.fetch_with_str(url)).await?;
        let response: web_sys::Response = response.dyn_into().unwrap();

        let array_buffer = JsFuture::from(response.array_buffer()?).await?;
        let array_buffer: ArrayBuffer = array_buffer.dyn_into().unwrap();

        let audio_buffer_promise = self.context.decode_audio_data(&array_buffer)?;
        let audio_buffer = JsFuture::from(audio_buffer_promise).await?;
        Ok(audio_buffer.dyn_into().unwrap())
    }

    pub fn play_sound_at_pos(&mut self) -> Result<(), JsValue> {
        if let Some(ref audio_buffer) = self.sound_buffer {
            let source_node = self.context.create_buffer_source()?;
            source_node.set_buffer(Some(audio_buffer));

            let panner_node = self.context.create_panner()?;
            source_node.connect_with_audio_node(&panner_node)?;
            panner_node.connect_with_audio_node(&self.gain_node)?;

            source_node.start()?;

            self.source_node = Some(source_node);
            self.panner_node = Some(panner_node);

            Ok(())
        } else {
            web_sys::console::error_1(&"Sound buffer not loaded".into());
            Err(JsValue::from_str("Sound buffer not loaded"))
        }
    }

    pub fn set_volume(&self, volume: f32) {
        self.gain_node.gain().set_value(volume);
    }

    pub fn set_panner_position(&self, x: f32, y: f32, z: f32) {
        if let Some(ref panner_node) = self.panner_node {
            panner_node.position_x().set_value(x);
            panner_node.position_y().set_value(y);
            panner_node.position_z().set_value(z);
        }
    }

    pub fn set_listener_position(&self, x: f32, y: f32, z: f32) {
        let listener = self.context.listener();
        listener.set_position(x as f64, y as f64, z as f64);
    }

    pub fn set_listener_orientation(
        &self,
        forward_x: f32,
        forward_y: f32,
        forward_z: f32,
        up_x: f32,
        up_y: f32,
        up_z: f32,
    ) {
        let listener = self.context.listener();
        listener.set_orientation(
            forward_x as f64,
            forward_y as f64,
            forward_z as f64,
            up_x as f64,
            up_y as f64,
            up_z as f64,
        );
    }

    // DEBUG: Spawn a sound on Engine initialisation and
    // apply panning effect in tick via `update_debug_sound`
    pub fn is_debug(&self) -> bool {
        true
    }

    pub fn update_debug_sound_on_tick(&mut self) {
        if self.is_debug() {
            if let Some(ref panner_node) = self.panner_node {
                // Get the current x position
                let current_x = panner_node.position_x().value();
                // Update the x position
                let mut new_x = current_x + 0.5;
                if new_x > 5.0 {
                    new_x = -5.0;
                }
                // Set the new x position
                panner_node.position_x().set_value(new_x);
            }
        }
    }
}

// use serde::{Deserialize, Serialize};
// #[wasm_bindgen]
// #[derive(Serialize, Deserialize, Debug, Clone)]
// pub struct Position {
//     pub x: f32,
//     pub y: f32,
//     pub z: f32,
// }
