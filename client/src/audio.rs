use std::collections::HashMap;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
#[allow(unused)]
use web_sys::{
    js_sys::ArrayBuffer, window, AudioBuffer, AudioBufferSourceNode, AudioContext, AudioListener,
    AudioParam, GainNode, OscillatorNode, PannerNode, Response,
};
use web_sys::{js_sys::Uint8Array, DistanceModelType, PanningModelType};

const FOOTSTEPS_OGG: &[u8] = include_bytes!("../../assets/footsteps.ogg");
const PAIN_WAV: &[u8] = include_bytes!("../../assets/pain.wav");
const STEP_GRAVEL_WAV: &[u8] = include_bytes!("../../assets/step_gravel.wav");

#[wasm_bindgen]
pub struct AudioManager {
    context: AudioContext,
    gain_node: GainNode,
    // Mapping of sound IDs to loaded AudioBuffers
    sounds_bank: HashMap<String, AudioBuffer>,
    // Active sound instances mapped by a unique handle (e.g., u32)
    active_sounds: HashMap<u32, SoundInstance>,
    // Counter for generating unique handles
    next_sound_handle: u32,
}

#[wasm_bindgen]
impl AudioManager {
    /// Create a new AudioManager and initialize its AudioContext and GainNode
    pub fn new() -> Result<AudioManager, JsValue> {
        let context = AudioContext::new()?;
        let gain_node = context.create_gain()?;
        gain_node.connect_with_audio_node(&context.destination())?;

        Ok(AudioManager {
            context,
            gain_node,
            sounds_bank: HashMap::new(),
            active_sounds: HashMap::new(),
            next_sound_handle: 0,
        })
    }

    // Use this to preload our sounds
    pub async fn load_sounds_into_bank(&mut self) -> Result<(), JsValue> {
        self.load_sound_from_id("footsteps").await?;
        self.load_sound_from_id("pain").await?;
        self.load_sound_from_id("step_gravel").await?;
        // self.load_sound_from_url("https://s3-us-west-2.amazonaws.com/s.cdpn.io/858/outfoxing.mp3")
        //     .await?;
        web_sys::console::log_1(&"Sounds successfully loaded into sounds_bank".into());
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
                web_sys::console::error_1(&format!("Unknown sound ID:{}", sound_id).into());
                return Err(JsValue::from_str(&format!(
                    "Unable to load audio buffer: Unknown sound id {sound_id}"
                )));
            }
        };

        // Create a Uint8Array from the embedded bytes
        let uint8_array = Uint8Array::from(sound_bytes);
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
        // TODO: pass in individual gain and other parameters
    ) -> Result<(), JsValue> {
        let Some(ref audio_buffer) = self.sounds_bank.get(sound_id) else {
            web_sys::console::error_1(&"Sound buffer not loaded".into());
            return Err(JsValue::from_str("Sound buffer not loaded"));
        };

        let Ok(sound_instance) =
            SoundInstance::new(&self.context, audio_buffer, &self.gain_node, None)
        else {
            web_sys::console::error_1(&"Unable to create sound_instance".into());
            return Err(JsValue::from_str("Unable to create sound_instance"));
        };

        // Generate a unique handle
        let handle = self.next_sound_handle;
        self.next_sound_handle += 1;

        if let Some(pos) = maybe_position {
            sound_instance.set_position(pos.x, pos.y, pos.z);
        }

        sound_instance.start();

        // Insert into active_sounds
        self.active_sounds.insert(handle, sound_instance);

        return Ok(());
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

    /// Stops a sound given its handle.
    pub fn stop_sound(&mut self, handle: u32) -> Result<(), JsValue> {
        if let Some(sound_instance) = self.active_sounds.remove(&handle) {
            sound_instance.stop()?;
            Ok(())
        } else {
            web_sys::console::error_1(&format!("Sound handle '{}' not found", handle).into());
            Err(JsValue::from_str("Sound handle not found"))
        }
    }

    /// Stops all currently playing sounds and clears the active_sounds map.
    pub fn stop_all_sounds(&mut self) -> Result<(), JsValue> {
        for (&handle, sound_instance) in self.active_sounds.iter() {
            match sound_instance.stop() {
                Ok(_) => {
                    web_sys::console::log_1(
                        &format!("Stopped sound with handle {}", handle).into(),
                    );
                }
                Err(err) => {
                    web_sys::console::error_1(
                        &format!("Failed to stop sound with handle {}: {:?}", handle, err).into(),
                    );
                }
            }
        }
        self.active_sounds.clear();
        web_sys::console::log_1(&"All sounds stopped.".into());
        Ok(())
    }

    /// Clears the sounds_bank, removing all loaded sounds.
    pub fn clear_sounds_bank(&mut self) -> Result<(), JsValue> {
        self.sounds_bank.clear();
        web_sys::console::log_1(&"Sounds bank cleared.".into());
        Ok(())
    }

    pub fn set_master_volume(&self, volume: f32) {
        self.gain_node.gain().set_value(volume);
    }

    /// Update all active sounds in tick
    /// TODO: entity positions, master volume, etc, combine with `update_audio_manager``
    pub fn test_update_sound_positions(&mut self) {
        for (&handle, sound_instance) in &mut self.active_sounds {
            // Apply position updates from entities
            if let Some(entity_id) = sound_instance.entity_id {
                // TODO:
                // if let Some(position) = world.get_entity_position(entity_id) {}
            }
        }
    }

    // DEBUG: currently just shifting all panners, TODO: update with Entity position in tick
    pub fn move_all_panner_nodes(&mut self, move_panner_opt: Option<f32>) {
        for (_, sound_instance) in &mut self.active_sounds {
            // Test from react to make sure it works
            let Some(panner_displacement) = move_panner_opt else {
                continue;
            };
            let old_pos_x = sound_instance.panner_node.position_x().value();
            sound_instance
                .panner_node
                .position_x()
                .set_value(old_pos_x + panner_displacement);
        }
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
// For sounds played via `play_sound_at_entity`, update panner_node in `tick`
// Handle ambient sounds by disabling spatialisation or matching with listener
// Extract distortion functionality from `AudioPlayer`
// Make the sounds library typesafe (and ideally export to typescript)
// expose a `play_ambient_sound` or just pass an optional `is_ambient` parameter to `play_sound`
// - Scripting parameters: From `play_sound` store on `SoundInstance` or trigger mutation in `tick`
// 1. looping parameter SoundInstance and an optional field on `play_sound` functions
// 2. `is_looping`
// 3. `reference distance` - clarify if just max or min as well
// 4. `playback_speed`
// 5. `volume`
// 6. `distortion` -  `distortion_node: Option<web_sys::WaveShaperNode>,`
// Implement a mechanism to interrupt playback from scripts (on entity or position)
// Make sure coordinate systems line up between Hytopia and WebAudio (which is Right handed)

struct SoundInstance {
    source_node: AudioBufferSourceNode,
    panner_node: PannerNode,
    entity_id: Option<u32>, // Associated entity ID, if any
}

impl SoundInstance {
    fn new(
        context: &AudioContext,
        audio_buffer: &AudioBuffer,
        gain_node: &GainNode,
        // TODO: Check if this should be hecs::Entity
        entity_id: Option<u32>,
    ) -> Result<Self, JsValue> {
        let source_node = context.create_buffer_source()?;
        source_node.set_buffer(Some(audio_buffer));

        let panner_node = context.create_panner()?;

        // TODO: Connect these parameters to play_sound functions
        // We'll want to expose them scripts (as per spec)
        // However, For now, I'm just using the defaults
        panner_node.set_panning_model(PanningModelType::Equalpower); // Also supports Hrtf
        panner_node.set_distance_model(DistanceModelType::Inverse); // Also supports Linear and Exponential

        panner_node.set_max_distance(10000.);
        panner_node.set_ref_distance(1.);
        panner_node.set_rolloff_factor(1.);

        source_node.connect_with_audio_node(&panner_node)?;
        panner_node.connect_with_audio_node(gain_node)?;

        // TODO: Implement
        source_node.set_loop(true);

        Ok(SoundInstance {
            source_node,
            panner_node,
            entity_id,
        })
    }

    fn start(&self) -> Result<(), JsValue> {
        self.source_node.start()
    }

    /// Stops playback and disconnects nodes to free resources.
    fn stop(&self) -> Result<(), JsValue> {
        self.source_node.stop()?;
        self.source_node.disconnect()?;
        self.panner_node.disconnect()?;
        Ok(())
    }

    fn set_position(&self, x: f32, y: f32, z: f32) {
        self.panner_node.position_x().set_value(x);
        self.panner_node.position_y().set_value(y);
        self.panner_node.position_z().set_value(z);
    }
}
