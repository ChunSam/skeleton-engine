//! Web Audio SFX + music for the web (wasm) via the Web Audio API.
//!
//! The full [`AudioManager`](crate::audio::AudioManager) is rodio-based and **native-only**
//! (`cfg(not(target_arch = "wasm32"))`). [`WebAudio`] is the browser counterpart: a small
//! `AudioContext` wrapper that covers the common needs — fire-and-forget sound effects, a single
//! looping **music** channel you can stop, a **master volume**, and **suspend/resume** for
//! pausing all audio. (Per-source mixing, crossfade, buses, ducking and positional audio remain
//! native-only.)
//!
//! Store it as a `World` resource and drive it from systems:
//!
//! ```ignore
//! // wasm only
//! if let Some(audio) = engine::WebAudio::new() {
//!     audio.set_volume(0.7);
//!     app.world.insert_resource(audio);
//! }
//! // later, from a system (ideally first triggered by a user gesture — browsers gate audio):
//! if let Some(a) = world.resource::<engine::WebAudio>() {
//!     a.play(JUMP_WAV_BYTES);        // one-shot SFX
//!     a.play_music(THEME_OGG_BYTES); // looping music on the music channel
//!     // a.stop_music();             // stop it
//!     // a.suspend(); a.resume();    // pause / unpause everything
//! }
//! ```

use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

/// A Web Audio SFX + music player for wasm builds.
///
/// All playback routes through a master [`GainNode`](web_sys::GainNode) so
/// [`set_volume`](Self::set_volume) affects everything. Cloning is cheap (the inner JS nodes are
/// reference-counted by the browser, and the music handle is an `Rc`).
#[derive(Clone)]
pub struct WebAudio {
    ctx: web_sys::AudioContext,
    /// Master gain — every source connects here, and here to the destination.
    master: web_sys::GainNode,
    /// The current looping music source (single channel): set by `play_music`, cleared by
    /// `stop_music`. Stored once decoding finishes (playback is async).
    music: Rc<RefCell<Option<web_sys::AudioBufferSourceNode>>>,
}

impl WebAudio {
    /// Creates the audio context + master gain. Returns `None` if the browser refuses.
    pub fn new() -> Option<Self> {
        let ctx = web_sys::AudioContext::new().ok()?;
        let master = ctx.create_gain().ok()?;
        master.connect_with_audio_node(&ctx.destination()).ok()?;
        Some(Self {
            ctx,
            master,
            music: Rc::new(RefCell::new(None)),
        })
    }

    /// Sets the master volume, clamped to `0.0..=1.0`. Affects all current and future playback.
    pub fn set_volume(&self, v: f32) {
        self.master.gain().set_value(v.clamp(0.0, 1.0));
    }

    /// The current master volume.
    pub fn volume(&self) -> f32 {
        self.master.gain().value()
    }

    /// Decodes `bytes` (an encoded audio clip — whatever the browser decodes: WAV/MP3/OGG) and
    /// plays it once through the master gain (fire-and-forget). Browsers gate audio behind a user
    /// gesture, so the first call should originate from an input handler (or call
    /// [`resume`](Self::resume)).
    pub fn play(&self, bytes: &[u8]) {
        self.decode_then_play(bytes, false, false);
    }

    /// Plays `bytes` as looping music on the single music channel: stops any current music, then
    /// starts the new clip looping (through the master gain). Stop it with
    /// [`stop_music`](Self::stop_music).
    pub fn play_music(&self, bytes: &[u8]) {
        self.stop_music();
        self.decode_then_play(bytes, true, true);
    }

    /// Stops the current looping music, if any.
    // web-sys deprecates the `AudioBufferSourceNode::stop*` bindings (no non-deprecated
    // equivalent is exposed for stopping a source); the call is correct, so allow it locally.
    #[allow(deprecated)]
    pub fn stop_music(&self) {
        if let Some(src) = self.music.borrow_mut().take() {
            let _ = src.stop();
        }
    }

    /// Suspends all audio (e.g. on game pause). Resume with [`resume`](Self::resume).
    pub fn suspend(&self) {
        let _ = self.ctx.suspend();
    }

    /// Resumes audio after [`suspend`](Self::suspend) — also the call to satisfy the browser's
    /// user-gesture gate if the context started suspended.
    pub fn resume(&self) {
        let _ = self.ctx.resume();
    }

    /// Shared decode→connect→start path. `looping` sets loop mode; `is_music` stores the source
    /// in the music channel so it can be stopped later.
    fn decode_then_play(&self, bytes: &[u8], looping: bool, is_music: bool) {
        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let master = self.master.clone();
        let music = is_music.then(|| self.music.clone());
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
            src.set_loop(looping);
            if src.connect_with_audio_node(&master).is_ok() {
                let _ = src.start();
                if let Some(music) = music {
                    *music.borrow_mut() = Some(src);
                }
            }
        });
    }
}
