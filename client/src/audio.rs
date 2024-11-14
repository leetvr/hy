use std::collections::HashMap;

use entities::EntityID;
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
const KANE_CLOMP: &[u8] = include_bytes!("../../assets/kane.wav");

pub struct AudioManager {
    context: AudioContext,
    gain_node: GainNode,
    // Mapping of sound names to loaded AudioBuffers
    sounds_bank: HashMap<String, AudioBuffer>,
    // Active sound instances mapped by a unique handle
    active_sounds: HashMap<u32, SoundInstance>,
    // Counter for generating unique handles for each SoundInstance in `active_sounds`
    next_sound_handle: u32,
}

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
        self.load_sound_from_id("pain").await?;
        self.load_sound_from_id("kane").await?;
        self.load_sound_from_id("footsteps").await?;
        // self.load_sound_from_id("step_gravel").await?;
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
            "kane" => KANE_CLOMP,
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

    /// Attempt to create and play a sound
    ///
    /// TODO Remaining parameters [volume, distortion]
    /// Should we be gracefully handling errors for methods exposed to user in the `play_sound` functions calling `spawn_sound`?
    ///
    /// ## Parameters:
    ///
    /// `sound_id` - the name of the sound to play from `sounds_bank`
    /// `entity_id` - Optional EntityID of the Hytopia entity used to dynamically update sound position
    /// `maybe_position` - Optional position of the sound (Otherwise will default to origin)
    /// `is_ambient` - If set, spatialisation will be disabled so the sound will have constant volume from anywhere
    /// `is_looping` - Loop the track
    /// `pitch` - playbackRate which is technically the same as `pitch` in minecraft
    /// `reference_distance` - the distance from listener at which the volume starts attenuating
    ///
    /// ## Returns
    /// Result<u32, JsValue> to return the sound's unique handle which can be used to update the sound
    pub fn spawn_sound(
        &mut self,
        sound_id: &str,
        entity_id: Option<EntityID>,
        maybe_position: Option<SoundPosition>,
        is_ambient: bool,
        is_looping: bool,
        pitch: Option<f32>,
        reference_distance: Option<f32>,
    ) -> Result<u32, JsValue> {
        let Some(ref audio_buffer) = self.sounds_bank.get(sound_id) else {
            web_sys::console::error_1(&"Sound buffer not loaded".into());
            return Err(JsValue::from_str("Sound buffer not loaded"));
        };

        let Ok(sound_instance) = SoundInstance::new(
            &self.context,
            audio_buffer,
            &self.gain_node,
            entity_id,
            is_ambient,
            is_looping,
            pitch,
            reference_distance,
        ) else {
            web_sys::console::error_1(&"Unable to create sound_instance".into());
            return Err(JsValue::from_str("Unable to create sound_instance"));
        };

        if let Some(pos) = maybe_position {
            sound_instance.set_position(pos.x, pos.y, pos.z);
        }

        if let Err(e) = sound_instance.start() {
            web_sys::console::error_1(&"Failed to start sound instance".into());
            return Err(e);
        }

        // Generate a unique handle
        let handle = self.next_sound_handle;
        self.next_sound_handle += 1;

        // Insert into active_sounds
        self.active_sounds.insert(handle, sound_instance);

        return Ok(handle);
    }

    /// Attempt to update the parameters associated with SoundInstance
    pub fn update_sound_with_handle(
        &mut self,
        handle: u32,
        _is_ambient: Option<bool>,
        is_looping: Option<bool>,
        pitch: Option<f32>,
        reference_distance: Option<f32>,
    ) -> Result<(), JsValue> {
        let sound = self
            .active_sounds
            .get_mut(&handle)
            .ok_or_else(|| JsValue::from_str("Sound instance not found"))?;

        // TODO: handle switch ambient status which requires reconfiguring the nodes
        // TODO: Update panner if position or new entity provided

        is_looping.map(|looping| sound.source_node.set_loop(looping));
        pitch.map(|p| sound.source_node.playback_rate().set_value(p.max(0.01)));
        reference_distance.map(|rd| sound.panner_node.set_ref_distance(rd as f64));
        Ok(())
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

    // DEBUG: currently just shifting all panners, (Make sure doesn't affect entity)
    pub fn move_all_panner_nodes(&mut self, move_panner_opt: Option<f32>) {
        if let Some(d) = move_panner_opt {
            self.active_sounds.values_mut().for_each(|s| {
                s.panner_node
                    .position_x()
                    .set_value(s.panner_node.position_x().value() + d)
            });
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

    /// Updates the positions of all active sounds associated with entities.
    pub fn synchronise_positions(&mut self, positions: &HashMap<EntityID, glam::Vec3>) {
        for sound_instance in self.active_sounds.values_mut() {
            if let Some(entity_id) = &sound_instance.entity_id {
                if let Some(pos) = positions.get(entity_id) {
                    sound_instance.set_position_from_vec3(*pos);
                }
            }
        }
    }

    /// Cleans up sounds associated with non-existent entities.
    pub fn cleanup_entity_sounds(
        &mut self,
        existing_entity_ids: &std::collections::HashSet<EntityID>,
    ) {
        self.active_sounds.retain(|_, sound_instance| {
            if let Some(entity_id) = &sound_instance.entity_id {
                existing_entity_ids.contains(entity_id)
            } else {
                true
            }
        });
    }

    /// Checks if a sound is active for a given entity.
    pub fn has_active_sound(&self, entity_id: EntityID) -> bool {
        self.active_sounds
            .values()
            .any(|sound_instance| sound_instance.entity_id == Some(entity_id.clone()))
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

impl SoundPosition {
    pub fn new(x: f32, y: f32, z: f32) -> SoundPosition {
        SoundPosition { x, y, z }
    }
}

struct SoundInstance {
    source_node: AudioBufferSourceNode,
    panner_node: PannerNode,
    entity_id: Option<EntityID>,
}

impl SoundInstance {
    fn new(
        context: &AudioContext,
        audio_buffer: &AudioBuffer,
        gain_node: &GainNode,
        entity_id: Option<EntityID>,
        is_ambient: bool,
        is_looping: bool,
        pitch: Option<f32>,
        reference_distance: Option<f32>,
        // TODO: distortion, volume
        // distortion_node: distortion_node: Option<web_sys::WaveShaperNode>....
        // volume_override: GainNode
    ) -> Result<Self, JsValue> {
        let source_node = context.create_buffer_source()?;
        source_node.set_buffer(Some(audio_buffer));
        let panner_node = context.create_panner()?;

        source_node.set_loop(is_looping);
        if is_ambient {
            // For ambient sounds, we can connect the source_node directly to gain_node.
            // Omitting the panner_node from the chain will functionally disable spatialistaion.
            // Note to self: if converting a pre-existing already-running sound from non-ambient to ambient
            // then I'll need to connect the PannerNode. It may turn out to be easier to connect the panner_node
            // for ambient sounds from the outset and disable spatialisation some other way, like synchronising
            // with the listener transform, rather than reconfiguring an already active sound when updating
            source_node.connect_with_audio_node(&gain_node)?;
            return Ok(SoundInstance {
                source_node,
                panner_node: panner_node,
                entity_id,
            });
        }

        // I've included the following panning configurations as we may want to
        // expose them to scripts. However, For now, I'm just using the defaults
        panner_node.set_panning_model(PanningModelType::Equalpower); // Also supports Hrtf
        panner_node.set_distance_model(DistanceModelType::Inverse); // Also supports Linear and Exponential
        panner_node.set_ref_distance(reference_distance.unwrap_or(1.) as f64);
        panner_node.set_max_distance(10000.);
        panner_node.set_rolloff_factor(1.);

        // Connect nodes: source -> panner -> gain
        source_node.connect_with_audio_node(&panner_node)?;
        panner_node.connect_with_audio_node(gain_node)?;

        // `playback_rate` is technically the same as `pitch` in minecraft
        // Hytopia may be after altering playback speed without altering pitch
        // which is a more advanced effect that could be done manually or
        // possibly more easily with `Tone.js`
        source_node
            .playback_rate()
            .set_value(pitch.unwrap_or(1.0).max(0.01));

        Ok(SoundInstance {
            source_node,
            panner_node,
            entity_id,
        })
    }

    fn start(&self) -> Result<(), JsValue> {
        // It may make sense to call this in SoundInstance.new(), Will need to check any difference between start/resume
        // Also, it may depend on whether we decide to clean up any sounds not attached to an entity that are no longer playing
        self.source_node.start()
    }

    /// Stops playback and disconnects nodes to free resources.
    pub fn stop(&self) -> Result<(), JsValue> {
        // TODO: use non deprecated
        self.source_node.stop()?;
        self.source_node.disconnect()?;
        self.panner_node.disconnect()?;
        Ok(())
    }

    pub fn set_position(&self, x: f32, y: f32, z: f32) {
        self.panner_node.position_x().set_value(x);
        self.panner_node.position_y().set_value(y);
        self.panner_node.position_z().set_value(z);
    }

    // Alternatively, you can add a method that accepts `glam::Vec3` directly
    pub fn set_position_from_vec3(&self, glam_vec: glam::Vec3) {
        self.set_position(glam_vec.x, glam_vec.y, glam_vec.z);
    }
}
