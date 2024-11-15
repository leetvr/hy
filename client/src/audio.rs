use std::collections::HashMap;

use entities::EntityID;
use glam::Vec3;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::WaveShaperNode;
use web_sys::{
    js_sys::ArrayBuffer, AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode, PannerNode,
};
use web_sys::{js_sys::Uint8Array, DistanceModelType, PanningModelType};

const FOOTSTEPS_OGG: &[u8] = include_bytes!("../../assets/footsteps.ogg");
const PAIN_WAV: &[u8] = include_bytes!("../../assets/pain.wav");
const STEP_GRAVEL_WAV: &[u8] = include_bytes!("../../assets/step_gravel.wav");
const KANE_CLOMP: &[u8] = include_bytes!("../../assets/kane.wav");
const PORTAL_WAV: &[u8] = include_bytes!("../../assets/portal.wav");

pub struct AudioManager {
    context: AudioContext,
    master_gain_node: GainNode,
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
        let master_gain_node = context.create_gain()?;
        master_gain_node.gain().set_value(1.0);
        master_gain_node.connect_with_audio_node(&context.destination())?;

        Ok(AudioManager {
            context,
            master_gain_node,
            sounds_bank: HashMap::new(),
            active_sounds: HashMap::new(),
            next_sound_handle: 0,
        })
    }

    fn next_handle(&self) -> u32 {
        self.next_sound_handle
    }

    // Use this to preload our sounds
    pub async fn load_sounds_into_bank(&mut self) -> Result<(), JsValue> {
        self.load_sound_from_id("pain").await?;
        self.load_sound_from_id("kane").await?;
        self.load_sound_from_id("footsteps").await?;
        self.load_sound_from_id("portal").await?;
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
            "portal" => PORTAL_WAV,
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
        let window =
            web_sys::window().ok_or_else(|| JsValue::from_str("No window object available"))?;

        // Fetch the URL and wait for the response,
        let response_future = window.fetch_with_str(url);
        let response = JsFuture::from(response_future).await?;
        let response: web_sys::Response = response
            .dyn_into()
            .map_err(|_| JsValue::from_str("Response conversion failed"))?;

        // Get the ArrayBuffer from the response
        let array_buffer = JsFuture::from(response.array_buffer()?).await?;
        let array_buffer: ArrayBuffer = array_buffer
            .dyn_into()
            .map_err(|_| JsValue::from_str("ArrayBuffer conversion failed"))?;

        // Decode the data
        let audio_buffer_promise = self.context.decode_audio_data(&array_buffer)?;
        let audio_buffer = JsFuture::from(audio_buffer_promise).await?;
        let audio_buffer: AudioBuffer = audio_buffer
            .dyn_into()
            .map_err(|_| JsValue::from_str("AudioBuffer conversion failed"))?;

        Ok(audio_buffer)
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
        maybe_position: Option<Vec3>,
        is_ambient: bool,
        is_looping: bool,
        pitch: Option<f32>,
        reference_distance: Option<f32>,
        volume: Option<f32>,
        enable_distortion: bool,
    ) -> Result<u32, JsValue> {
        let Some(ref audio_buffer) = self.sounds_bank.get(sound_id) else {
            web_sys::console::error_1(&"Sound buffer not loaded".into());
            return Err(JsValue::from_str("Sound buffer not loaded"));
        };

        let Ok(sound_instance) = SoundInstance::new(
            &self.context,
            audio_buffer,
            &self.master_gain_node,
            entity_id,
            is_ambient,
            is_looping,
            pitch,
            reference_distance,
            volume,
            enable_distortion,
        ) else {
            web_sys::console::error_1(&"Unable to create sound_instance".into());
            return Err(JsValue::from_str("Unable to create sound_instance"));
        };

        if let Some(pos) = maybe_position {
            sound_instance.set_position_from_vec3(pos);
        }

        if let Err(e) = sound_instance.start() {
            web_sys::console::error_1(&"Failed to start sound instance".into());
            return Err(e);
        }

        // Generate a unique handle
        let handle = self.next_handle();
        self.next_sound_handle += 1;

        // Insert into active_sounds
        self.active_sounds.insert(handle, sound_instance);

        return Ok(handle);
    }

    /// Attempt to update the parameters associated with SoundInstance
    pub fn update_sound_with_handle(
        &mut self,
        handle: u32,
        entity_id: Option<entities::EntityID>,
        position: Option<Vec3>,
        // @Kane: we could handle switching between ambient and non ambient although
        // it's gonna be annoying and I'm not sure that it's specified in the User Stories
        _is_ambient: Option<bool>, // TODO?
        is_looping: Option<bool>,
        pitch: Option<f32>,
        reference_distance: Option<f32>,
        volume: Option<f32>,
        _is_distortion: Option<bool>, // TODO
    ) -> Result<(), JsValue> {
        let sound = self
            .active_sounds
            .get_mut(&handle)
            .ok_or_else(|| JsValue::from_str("Sound instance not found"))?;

        // Handle static position updates and transitions to/from having an associated entity
        if let Some(new_entity_id) = entity_id {
            sound.entity_id = Some(new_entity_id.to_string());
        } else if let Some(new_position) = position {
            sound.entity_id = None;
            sound.set_position_from_vec3(new_position);
        }

        is_looping.map(|looping| sound.source_node.set_loop(looping));
        pitch.map(|p| sound.source_node.playback_rate().set_value(p.max(0.01)));
        reference_distance.map(|rd| sound.panner_node.set_ref_distance(rd as f64));
        volume.map(|v| {
            // What even is safety?
            let clamped_volume = v.clamp(0.0, 5.0);
            sound.sound_gain_node.gain().set_value(clamped_volume);
        });

        Ok(())
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
    pub fn synchronise_positions(&mut self, positions: &HashMap<EntityID, Vec3>) {
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

    /// Returns true if there is at least one SoundInstance that has the given entity_id.
    pub fn has_active_sound_with_entity_id(&self, entity_id: &EntityID) -> bool {
        self.active_sounds
            .values()
            .any(|si| si.entity_id.as_ref() == Some(entity_id))
    }

    /// Returns all SoundInstances that have a given entity_id.
    fn get_sound_instances_with_entity_id(&self, entity_id: &EntityID) -> Vec<&SoundInstance> {
        self.active_sounds
            .values()
            .filter(|si| si.entity_id.as_ref() == Some(entity_id))
            .collect()
    }

    /// Stops all currently playing sounds and clears the active_sounds map.
    pub fn stop_all_sounds(&mut self) -> Result<(), JsValue> {
        for (&handle, sound_instance) in self.active_sounds.iter() {
            match sound_instance.cleanup_sound_instance() {
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

    /// Stops a sound given its handle.
    pub fn _stop_sound_with_handle(&mut self, handle: u32) -> Result<(), JsValue> {
        if let Some(sound_instance) = self.active_sounds.remove(&handle) {
            sound_instance.cleanup_sound_instance()?;
            Ok(())
        } else {
            web_sys::console::error_1(&format!("Sound handle '{}' not found", handle).into());
            Err(JsValue::from_str("Sound handle not found"))
        }
    }

    /// Sets the volume for a specific sound instance
    fn _set_sound_volume(&mut self, handle: u32, volume: f32) -> Result<(), JsValue> {
        let sound = self
            .active_sounds
            .get_mut(&handle)
            .ok_or_else(|| JsValue::from_str("Sound instance not found"))?;

        // Clamp the volume between 0.0 and 1.0
        let clamped_volume = volume.clamp(0.0, 1.0);
        sound.sound_gain_node.gain().set_value(clamped_volume);

        Ok(())
    }

    /// Sets the master volume for all sounds.
    ///
    /// ## Parameters:
    ///
    /// * `volume` - The new master volume level (0.0 to 1.0).
    pub fn _set_master_volume(&self, volume: f32) {
        let clamped_volume = volume.clamp(0.0, 1.0);
        self.master_gain_node.gain().set_value(clamped_volume);
    }
}

/// Represents an instance of a sound being played, managing its audio nodes and properties.
struct SoundInstance {
    source_node: AudioBufferSourceNode,
    sound_gain_node: GainNode,
    panner_node: PannerNode,
    distortion_node: Option<WaveShaperNode>, // TODO: toggle
    entity_id: Option<EntityID>,
}

impl SoundInstance {
    /// Creates a new `SoundInstance`.
    ///
    /// Initializes the audio nodes, sets up the distortion if enabled, and connects the nodes
    /// based on whether the sound is ambient or not.
    fn new(
        context: &AudioContext,
        audio_buffer: &AudioBuffer,
        master_gain_node: &GainNode,
        entity_id: Option<EntityID>,
        is_ambient: bool,
        is_looping: bool,
        pitch: Option<f32>,
        reference_distance: Option<f32>,
        volume: Option<f32>,
        enable_distortion: bool,
    ) -> Result<Self, JsValue> {
        // Create the source node and set its buffer
        let source_node = context.create_buffer_source()?;
        source_node.set_buffer(Some(audio_buffer));
        source_node.set_loop(is_looping);

        // Create and configure the PannerNode
        let panner_node = context.create_panner()?;
        panner_node.set_ref_distance(reference_distance.unwrap_or(1.) as f64);
        // We may want to expose the following (currently just using default values)
        panner_node.set_panning_model(PanningModelType::Equalpower); // Also supports Hrtf
        panner_node.set_distance_model(DistanceModelType::Inverse); // Also supports Linear and Exponential
        panner_node.set_max_distance(10000.);
        panner_node.set_rolloff_factor(1.);

        // Create the per-sound GainNode and ensure volume is between 0.0 and 1.0
        let sound_gain_node = context.create_gain()?;
        let clamped_volume = volume.unwrap_or(1.0).clamp(0.0, 1.0);
        sound_gain_node.gain().set_value(clamped_volume);

        // Initialize the distortion node if enabled
        let maybe_distortion_node = if enable_distortion {
            let wave_shaper = create_wave_shaper(context, false)?;
            Some(wave_shaper)
        } else {
            None
        };

        // Connect the nodes based on whether the sound is ambient and if distortion is enabled
        // 1. Ambient with distortion:        source -> distortion -> gain -> master
        // 2. Ambient without distortion:     source -> gain -> master
        // 3. Non-ambient with distortion:    source -> distortion -> gain -> panner -> master
        // 4. Non-ambient without distortion: source -> gain -> panner -> master
        // Note that I'm not connecting the panner_node for ambient sounds (to disable spatialisation) but I'm
        // including it with the SoundInstance to make it easier to toggle is_ambient status on a pre-existent sound
        if is_ambient {
            if let Some(ref wave_shaper) = maybe_distortion_node {
                source_node.connect_with_audio_node(wave_shaper)?;
                wave_shaper.connect_with_audio_node(&sound_gain_node)?;
            } else {
                source_node.connect_with_audio_node(&sound_gain_node)?;
            }
            // Connect gain to master
            sound_gain_node.connect_with_audio_node(master_gain_node)?;
        } else {
            if let Some(ref wave_shaper) = maybe_distortion_node {
                source_node.connect_with_audio_node(wave_shaper)?;
                wave_shaper.connect_with_audio_node(&sound_gain_node)?;
            } else {
                source_node.connect_with_audio_node(&sound_gain_node)?;
            }
            // Connect gain to panner and panner to master
            sound_gain_node.connect_with_audio_node(&panner_node)?;
            panner_node.connect_with_audio_node(master_gain_node)?;
        }

        // Set the playback rate (pitch)
        source_node
            .playback_rate()
            .set_value(pitch.unwrap_or(1.0).max(0.01));

        Ok(SoundInstance {
            source_node,
            sound_gain_node,
            panner_node,
            distortion_node: maybe_distortion_node,
            entity_id,
        })
    }

    /// Begins the audio buffer source, initiating sound playback.
    fn start(&self) -> Result<(), JsValue> {
        self.source_node.start()
    }

    /// Stops playback and disconnects nodes which should allow them to be garbage collected
    pub fn cleanup_sound_instance(&self) -> Result<(), JsValue> {
        #[allow(deprecated)]
        self.source_node.stop()?;
        self.source_node.disconnect()?;
        self.panner_node.disconnect()?;
        self.sound_gain_node.disconnect()?;
        // If distortion is enabled, disconnect it as well
        if let Some(wave_shaper) = &self.distortion_node {
            wave_shaper.disconnect()?;
        }

        Ok(())
    }

    pub fn get_position(&self) -> glam::Vec3 {
        Vec3::new(
            self.panner_node.position_x().value(),
            self.panner_node.position_y().value(),
            self.panner_node.position_z().value(),
        )
    }

    /// Sets the spatial position of the sound in 3D space.
    pub fn set_position_from_vec3(&self, glam_vec: Vec3) {
        self.set_position(glam_vec.x, glam_vec.y, glam_vec.z);
    }

    pub fn set_position(&self, x: f32, y: f32, z: f32) {
        self.panner_node.position_x().set_value(x);
        self.panner_node.position_y().set_value(y);
        self.panner_node.position_z().set_value(z);
    }
}

/// Creates and configures a `WaveShaperNode` for distortion effects.
///
/// Parameters:
/// - `context`: The `AudioContext` to create the node.
/// - `use_oversampling`: Determines whether to apply oversampling (`N4x`) for smoother distortion.
fn create_wave_shaper(
    context: &AudioContext,
    use_oversampling: bool,
) -> Result<WaveShaperNode, JsValue> {
    let wave_shaper = context.create_wave_shaper()?;

    let mut curve = create_distortion_curve(10.);

    // Get a mutable slice of the curve
    let curve_slice: &mut [f32] = &mut curve[..];

    #[allow(deprecated)]
    // Directly set the curve to the WaveShaperNode
    wave_shaper.set_curve(Some(curve_slice));
    if use_oversampling {
        wave_shaper.set_oversample(web_sys::OverSampleType::N4x)
    } else {
        wave_shaper.set_oversample(web_sys::OverSampleType::None)
    };

    Ok(wave_shaper)
}

/// Generates a distortion curve using the hyperbolic tangent function.
///
/// Parameters:
/// * `scaling_factor`: control the intensity of the distortion
fn create_distortion_curve(scaling_factor: f32) -> Vec<f32> {
    let n_samples = 44100;
    let mut curve = Vec::with_capacity(n_samples);
    for i in 0..n_samples {
        let x = i as f32 * 2.0 / n_samples as f32 - 1.0;
        curve.push((x * scaling_factor).tanh());
    }
    curve
}

// NOTE: @Kane following is all janky testing code but it's very helpful for visualising spatialisation
// and quickly spawning sounds in realtime. You can apply dynamic update to the last
// spawned sound by right clicking, which activates `test_update_last_spawned_sound`
// So I haven't removed this code for this PR but presumably I will once you've reviewed
pub mod audio_debug_tools {
    use super::*;
    pub fn test_audio_manager(engine: &mut crate::Engine) {
        if matches!(engine.state, crate::game_state::GameState::Loading) {
            return;
        }

        // This are just the entities in `entities.json` that start moving when switching to `Playing`
        let moving_entity_id = "16569510173499221049";
        let entity_sound_name = "portal"; // this sound easier to hear spatialisation

        let inputs = engine.controls.keyboard_inputs.clone();

        if engine.controls.mouse_left {
            // Test: Spawn sound at target_raycast positiion
            if inputs.contains("KeyX") {
                test_play_sound_at_pos(engine, "pain");
            }
            // Test: Spawn dynamic sound on entity
            if inputs.contains("KeyC") {
                test_play_sound_at_entity(engine, entity_sound_name, moving_entity_id);
            }
            // Test: ambient sound spawning
            if inputs.contains("KeyV") {
                test_play_ambient_sound(engine, "footsteps");
            }

            if inputs.contains("KeyB") {
                let _ = engine.audio_manager.stop_all_sounds();
            }
        }

        if engine.controls.mouse_right {
            test_update_last_spawned_sound(engine, moving_entity_id);
        }

        visualise_spawned_sound_at_entity(engine, moving_entity_id);
        visualise_positioned_sounds(engine);
    }

    fn test_update_last_spawned_sound(engine: &mut crate::Engine, entity_id: &str) {
        // Well, we probably shouldn't take any usize parameters in the play_sound functions
        let last_sound_handle = (engine.audio_manager.next_handle() as i32) - 1;
        if last_sound_handle <= 0 {
            tracing::error!("Negative handle");
            return; // no sounds emitted yet
        }
        if let Err(e) = engine.update_sound_with_handle(
            last_sound_handle as u32,
            Some(entity_id.to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            Some(0.),
            Some(0.),
            Some(0.),
        ) {
            tracing::error!(
                "Failed to play ambient sound '{}' error: {:?}",
                last_sound_handle,
                e
            );
        }
    }

    fn test_play_ambient_sound(engine: &mut crate::Engine, sound_id: &str) {
        if let Err(e) = engine.play_ambient_sound(sound_id, None, true, None, Some(0.5), false) {
            tracing::error!("Failed to play ambient sound '{}' error: {:?}", sound_id, e);
        }
    }

    fn test_play_sound_at_entity(engine: &mut crate::Engine, sound_id: &str, entity_id: &str) {
        if let Err(e) = engine.play_sound_at_entity(
            sound_id,
            entity_id.to_string(),
            true,
            Some(0.5),
            None,
            Some(10.0),
            false,
        ) {
            tracing::error!(
                "Failed to play_sound_at_entity for {}.... {:?}",
                entity_id,
                e
            );
        }
    }

    fn test_play_sound_at_pos(engine: &mut crate::Engine, sound_id: &str) {
        let crate::game_state::GameState::Editing { target_raycast, .. } = &mut engine.state else {
            return;
        };
        let Some(ray_hit) = target_raycast else {
            return;
        };

        let pos = ray_hit.position;
        if let Err(_) = engine.play_sound_at_pos(
            sound_id,
            pos.x as f32,
            pos.y as f32,
            pos.z as f32,
            false,
            true,
            None,
            None,
            Some(10.),
            false,
        ) {
            tracing::error!("Failed to play sound '{}' at position {:?}", sound_id, pos)
        }
    }

    fn visualise_positioned_sounds(engine: &mut crate::Engine) {
        // Iterate over all sounds and find those without an associated entity
        for (handle, sound_instance) in engine.audio_manager.active_sounds.iter() {
            if sound_instance.entity_id.is_none() {
                let sound_position = sound_instance.get_position();

                // Visualize the sound with a vertical line to indicate its position
                engine
                    .debug_lines
                    .push(crate::render::DebugLine::new_with_color(
                        sound_position,
                        sound_position + glam::Vec3::new(0.0, 10.0, 0.0),
                        glam::Vec4::new(0.0, 0.0, 1.0, 1.0), // Use blue for positioned sounds
                    ));

                tracing::info!(
                    "Visualized ambient sound at position: {:?} with handle: {}",
                    sound_position,
                    handle
                );
            }
        }
    }

    fn visualise_spawned_sound_at_entity(engine: &mut crate::Engine, entity_id: &str) {
        use crate::game_state::GameState;

        let entity_id_str = entity_id.to_string();

        // Make sure the entity has a sound associated with it
        if !engine
            .audio_manager
            .has_active_sound_with_entity_id(&entity_id_str)
        {
            return;
        }

        // Visualise lines above the sound associated with the entity
        let entities = match &engine.state {
            GameState::Playing { entities, .. } | GameState::Editing { entities, .. } => entities,
            GameState::Loading => return,
        };
        for sound_instance in engine
            .audio_manager
            .get_sound_instances_with_entity_id(&entity_id_str)
        {
            let sound_position = sound_instance.get_position();
            if let Some(entity_data) = entities.get(&entity_id_str) {
                let _entity_position = entity_data.state.position;
                engine
                    .debug_lines
                    .push(crate::render::DebugLine::new_with_color(
                        sound_position + Vec3::new(0., 0.0, 0.0),
                        sound_position + Vec3::new(0.0, 10.0, 0.0),
                        glam::Vec4::new(1., 1., 0., 1.),
                    ));
            }
        }
    }
}
