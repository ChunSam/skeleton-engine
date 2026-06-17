//! Minimal one-shot SFX for the web (wasm) via the Web Audio API.
//!
//! The full [`AudioManager`](crate::audio::AudioManager) is rodio-based and **native-only**
//! (`cfg(not(target_arch = "wasm32"))`), so browser builds have no sound. [`WebAudio`] fills the
//! most common gap — fire-and-forget sound effects — with a tiny `AudioContext` wrapper. It is
//! intentionally small: decode a clip and play it once. Music, mixing, fades, and positional audio
//! remain native-only for now.
//!
//! Store it as a `World` resource and call [`play`](WebAudio::play) with encoded audio bytes
//! (whatever the browser can decode — WAV/MP3/OGG):
//!
//! ```ignore
//! // wasm only
//! if let Some(audio) = engine::WebAudio::new() {
//!     app.world.insert_resource(audio);
//! }
//! // later, from a system (ideally first triggered by a user gesture — browsers gate audio):
//! if let Some(a) = world.resource::<engine::WebAudio>() {
//!     a.play(JUMP_WAV_BYTES);
//! }
//! ```

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

/// A minimal Web Audio one-shot SFX player for wasm builds.
///
/// Each [`play`](Self::play) decodes the given bytes and plays them once (fire-and-forget). Cloning
/// is cheap (the inner `AudioContext` is reference-counted by the browser).
#[derive(Clone)]
pub struct WebAudio {
    ctx: web_sys::AudioContext,
}

impl WebAudio {
    /// Creates the audio context. Returns `None` if the browser refuses to create one.
    pub fn new() -> Option<Self> {
        web_sys::AudioContext::new().ok().map(|ctx| Self { ctx })
    }

    /// Decodes `bytes` (an encoded audio clip) and plays it once. No-op if decoding or playback
    /// fails. Browsers gate audio behind a user gesture, so the first call should originate from an
    /// input handler or the context may stay suspended.
    pub fn play(&self, bytes: &[u8]) {
        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let promise = match ctx.decode_audio_data(&buffer) {
            Ok(p) => p,
            Err(_) => return,
        };
        spawn_local(async move {
            let Ok(decoded) = JsFuture::from(promise).await else {
                return;
            };
            let Ok(audio_buffer) = decoded.dyn_into::<web_sys::AudioBuffer>() else {
                return;
            };
            let Ok(src) = ctx.create_buffer_source() else {
                return;
            };
            src.set_buffer(Some(&audio_buffer));
            if src.connect_with_audio_node(&ctx.destination()).is_ok() {
                let _ = src.start();
            }
        });
    }
}
