use std::collections::HashMap;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
#[allow(unused)]
use web_sys::{
    js_sys::ArrayBuffer, window, AudioBuffer, AudioBufferSourceNode, AudioContext, AudioListener,
    AudioParam, GainNode, OscillatorNode, PannerNode, Response,
};

const FOOTSTEPS_OGG: &[u8] = include_bytes!("../../assets/footsteps.ogg");
const PAIN_WAV: &[u8] = include_bytes!("../../assets/pain.wav");
const STEP_GRAVEL_WAV: &[u8] = include_bytes!("../../assets/step_gravel.wav");

#[wasm_bindgen]
pub struct AudioManager {
    context: AudioContext,
    gain_node: GainNode,
    // Initial prototyping: Just track the latest sound
    source_node: Option<AudioBufferSourceNode>,
    panner_node: Option<PannerNode>,
    // distortion_node: Option<web_sys::WaveShaperNode>,

    // // TODO: Track all current instances of a sound
    // sound_instances: RefCell<Vec<SoundInstance>>, // All current instances of a sound

    // Mapping of sound IDs to loaded AudioBuffers
    sounds_bank: HashMap<String, AudioBuffer>,
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
            source_node: None,
            panner_node: None,
            // distortion_node: None,

            // sound_instances: RefCell::new(Vec::new()),
            sounds_bank: HashMap::new(),
        })
    }

    // Use this to preload our sounds
    pub async fn load_sounds_into_bank(&mut self) -> Result<(), JsValue> {
        self.load_sound_from_id("footsteps").await?;
        self.load_sound_from_id("pain").await?;
        self.load_sound_from_id("step_gravel").await?;
        // self.load_sound_from_url("https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3")
        //     .await?;
        Ok(())
    }

    pub async fn load_sound_from_id(&mut self, sound_id: &str) -> Result<(), JsValue> {
        let audio_buffer = self.load_audio_buffer_from_bytes(sound_id).await?;
        self.sounds_bank.insert(sound_id.to_string(), audio_buffer);
        // self.sound_buffer = Some(audio_buffer);
        web_sys::console::log_1(&"Embedded sound loaded successfully".into());
        Ok(())
    }

    /// Loads the audio buffer from embedded bytes.
    async fn load_audio_buffer_from_bytes(&self, sound_id: &str) -> Result<AudioBuffer, JsValue> {
        // Map sound IDs to embedded byte slices
        let sound_bytes = match sound_id {
            "footsteps" => FOOTSTEPS_OGG,
            "pain" => PAIN_WAV,
            "step_gravel" => STEP_GRAVEL_WAV,
            _ => {
                web_sys::console::error_1(&format!("Unknown sound ID:{} -> pain", sound_id).into());
                // return Err(JsValue::from_str("Unknown sound ID"));
                PAIN_WAV
            }
        };

        // Create a Uint8Array from the embedded bytes
        let uint8_array = web_sys::js_sys::Uint8Array::from(sound_bytes);
        // Get the ArrayBuffer from the Uint8Array
        let array_buffer = uint8_array.buffer();
        // Decode the audio data
        let decode_promise = self.context.decode_audio_data(&array_buffer)?;
        let decoded_buffer = JsFuture::from(decode_promise).await?;
        // Cast the decoded buffer to AudioBuffer
        let audio_buffer: AudioBuffer = decoded_buffer.dyn_into()?;
        Ok(audio_buffer)
    }

    pub async fn load_sound_from_url(&mut self, url: &str) -> Result<(), JsValue> {
        let audio_buffer = self.load_audio_buffer_from_url(url).await?;
        // self.sound_buffer = Some(audio_buffer);
        self.sounds_bank.insert(url.to_string(), audio_buffer);
        web_sys::console::log_1(&"Sound buffer loaded successfully".into());
        Ok(())
    }

    async fn load_audio_buffer_from_url(&mut self, url: &str) -> Result<AudioBuffer, JsValue> {
        let window = web_sys::window().unwrap();
        let response = JsFuture::from(window.fetch_with_str(url)).await?;
        let response: web_sys::Response = response.dyn_into().unwrap();

        let array_buffer = JsFuture::from(response.array_buffer()?).await?;
        let array_buffer: ArrayBuffer = array_buffer.dyn_into().unwrap();

        let audio_buffer_promise = self.context.decode_audio_data(&array_buffer)?;
        let audio_buffer = JsFuture::from(audio_buffer_promise).await?;
        Ok(audio_buffer.dyn_into().unwrap())
    }

    pub fn play_sound_at_pos(
        &mut self,
        sound_id: &str,
        maybe_position: Option<SoundPosition>,
    ) -> Result<(), JsValue> {
        if let Some(ref audio_buffer) = self.sounds_bank.get(sound_id) {
            let source_node = self.context.create_buffer_source()?;
            source_node.set_buffer(Some(audio_buffer));

            let panner_node = self.context.create_panner()?;
            source_node.connect_with_audio_node(&panner_node)?;
            panner_node.connect_with_audio_node(&self.gain_node)?;

            // Set panner position if provided
            if let Some(pos) = &maybe_position {
                panner_node.position_x().set_value(pos.x);
                panner_node.position_y().set_value(pos.y);
                panner_node.position_z().set_value(pos.z);
            }

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

    // DEBUG: Spawn footsteps sound on Engine initialisation and play a sound when placing blocks
    pub fn is_debug(&self) -> bool {
        true
    }
}

use serde::{Deserialize, Serialize};
#[wasm_bindgen]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SoundPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[wasm_bindgen]
impl SoundPosition {
    pub fn new(x: f32, y: f32, z: f32) -> SoundPosition {
        SoundPosition { x, y, z }
    }
}

// TODO:
// Preload sound assets
// Refactor AudioManager to keep track of a set of different `SoundInstance`s
// For sounds played via `play_sound_at_entity`, update panner_node in `tick`
// Handle ambient sounds by disabling spatialisation or matching with listener
// Extract distortion functionality from `AudioPlayer`

// TODO: (Later)
// Make the sounds library good (and ideally export to typescript)
// expose a `play_ambient_sound`
// Scripting parameters: From `play_sound` store on `SoundInstance` or trigger mutation in `tick`
// 1. looping parameter SoundInstance and an optional field on `play_sound` functions
// 2. `is_looping`
// 3. `reference distance` - clarify if just max or min as well
// 4. `playback_speed`
// 5. `volume`
// 6. `distortion`
// Implement a mechanism to interrupt playback from scripts (on entity or position)
// Make sure coordinate systems line up between Hytopia and WebAudio (Right handed)

// #[wasm_bindgen]
// impl AudioManager {
//     pub fn play_sound_at_entity(&self, entity_id: u32, sound_id: &str) -> Result<(), JsValue> {
//         todo!()
//     }
// }

// struct SoundInstance {
//     source_node: AudioBufferSourceNode,
//     panner_node: PannerNode,
//     entity_id: Option<u32>, // Associated entity ID, if any
// }

// impl SoundInstance {
//     fn new(
//         context: &AudioContext,
//         audio_buffer: &AudioBuffer,
//         gain_node: &GainNode,
//     ) -> Result<Self, JsValue> {
//         let source_node = context.create_buffer_source()?;
//         source_node.set_buffer(Some(audio_buffer));

//         let panner_node = context.create_panner()?;
//         source_node.connect_with_audio_node(&panner_node)?;
//         panner_node.connect_with_audio_node(gain_node)?;

//         Ok(SoundInstance {
//             source_node,
//             panner_node,
//             entity_id: None,
//         })
//     }

//     fn start(&self) -> Result<(), JsValue> {
//         self.source_node.start()
//     }

//     fn set_position(&self, x: f32, y: f32, z: f32) {
//         self.panner_node.position_x().set_value(x);
//         self.panner_node.position_y().set_value(y);
//         self.panner_node.position_z().set_value(z);
//     }
// }
