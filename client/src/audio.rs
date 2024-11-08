use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    js_sys::ArrayBuffer, window, AudioBuffer, AudioBufferSourceNode, AudioContext, GainNode,
    OscillatorNode, Response,
};

#[wasm_bindgen]
pub struct AudioManager {
    context: AudioContext,
    gain_node: GainNode,

    // We should maybe put the following sound specific properties into an AudioEmitter component or something
    // Then AudioManager can handle a number of simultaneous sound
    panner_node: web_sys::PannerNode,
    distortion_node: Option<web_sys::WaveShaperNode>,

    // Just put a test sound here for now
    test_sound: Option<AudioBuffer>,
}

#[wasm_bindgen]
impl AudioManager {
    pub fn new() -> Result<AudioManager, JsValue> {
        let context = AudioContext::new()?;
        let gain_node = context.create_gain()?;
        let panner_node = context.create_panner()?;

        gain_node.connect_with_audio_node(&context.destination())?;
        panner_node.connect_with_audio_node(&gain_node)?;

        Ok(AudioManager {
            context,
            gain_node,
            panner_node,
            distortion_node: None,
            test_sound: None,
        })
    }

    pub async fn load_sound(&self, url: &str) -> Result<AudioBuffer, JsValue> {
        let window = window().unwrap();
        let response = JsFuture::from(window.fetch_with_str(url)).await?;
        let response: Response = response.dyn_into().unwrap();

        // Fetch the array buffer from the response
        let array_buffer_promise = response.array_buffer()?;
        let array_buffer = JsFuture::from(array_buffer_promise).await?;
        // JsValue to ArrayBuffer
        let array_buffer: ArrayBuffer = array_buffer.dyn_into().unwrap();
        // Decode the audio data from the ArrayBuffer
        let audio_buffer =
            JsFuture::from(self.context.decode_audio_data(&array_buffer).unwrap()).await?;
        let audio_buffer: AudioBuffer = audio_buffer.dyn_into().unwrap();
        Ok(audio_buffer)
    }

    pub fn play_sound(&self, buffer: JsValue) -> Result<(), JsValue> {
        let source = self.context.create_buffer_source()?;
        source.set_buffer(Some(&buffer.into()));
        source.connect_with_audio_node(&self.gain_node)?;
        source.start()?;
        Ok(())
    }

    pub fn set_volume(&self, volume: f32) {
        self.gain_node.gain().set_value(volume);
    }

    pub fn set_position(&self, x: f32, y: f32, z: f32) {
        self.panner_node.position_x().set_value(x);
        self.panner_node.position_y().set_value(y);
        self.panner_node.position_z().set_value(z);
    }

    // pub fn enable_distortion(&mut self) -> Result<(), JsValue> {
    //     let distortion = self.context.create_wave_shaper()?;

    //     self.distortion_node = Some(distortion);
    //     self.update_distortion_chain()?;
    //     Ok(())
    // }

    // pub fn disable_distortion(&mut self) -> Result<(), JsValue> {
    //     if let Some(distortion) = &self.distortion_node {
    //         distortion.disconnect()?;
    //         self.panner_node.connect_with_audio_node(&self.gain_node)?;
    //     }
    //     self.distortion_node = None;
    //     Ok(())
    // }

    // fn update_distortion_chain(&self) -> Result<(), JsValue> {
    //     if let Some(distortion) = &self.distortion_node {
    //         self.panner_node.disconnect()?;
    //         self.panner_node.connect_with_audio_node(distortion)?;
    //         distortion.connect_with_audio_node(&self.gain_node)?;
    //     }
    //     Ok(())
    // }
}

// Try just load and play in a single function
#[wasm_bindgen]
impl AudioManager {
    pub fn test_initialised(&self) -> bool {
        self.test_sound.is_some()
    }

    // Combines loading and playing into one async function for simplicity
    pub async fn debug_load_and_play_sound(&mut self, url: &str) -> Result<(), JsValue> {
        let window = web_sys::window().unwrap();
        let response = JsFuture::from(window.fetch_with_str(url)).await?;
        let response: web_sys::Response = response.dyn_into().unwrap();

        let array_buffer = JsFuture::from(response.array_buffer()?).await?;
        let array_buffer = array_buffer.dyn_into().unwrap();

        let audio_buffer = JsFuture::from(self.context.decode_audio_data(&array_buffer)?).await?;
        let audio_buffer: web_sys::AudioBuffer = audio_buffer.dyn_into().unwrap();

        // Immediately play the sound after loading
        let source = self.context.create_buffer_source()?;
        source.set_buffer(Some(&audio_buffer));
        source.connect_with_audio_node(&self.gain_node)?;
        source.start()?;
        Ok(())
    }
}
