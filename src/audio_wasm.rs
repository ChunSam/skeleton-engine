//! Web Audio SFX + music for the web (wasm) via the Web Audio API.
//!
//! The full [`AudioManager`](crate::audio::AudioManager) is rodio-based and **native-only**
//! (`cfg(not(target_arch = "wasm32"))`). [`WebAudio`] is the browser counterpart: a small
//! `AudioContext` wrapper that covers the common needs — fire-and-forget sound effects,
//! **controllable** per-source SFX with **stereo pan** + volume + stop ([`play_sfx`] → [`Sfx`]), a
//! single looping **music** channel you can stop, a **master volume**, **named mixer buses**
//! ([`set_bus_volume`] + the [`play_on_bus`]/[`play_sfx_on_bus`] variants), and **suspend/resume**
//! for pausing all audio. (Crossfade, ducking and full positional audio remain native-only.)
//!
//! A **bus** is a named [`GainNode`](web_sys::GainNode) sitting between sounds and the master gain:
//! route sounds to a bus by name and control them together with [`set_bus_volume`] — the wasm
//! analogue of the native [`AudioManager`](crate::audio::AudioManager) bus mixer. Web Audio is a
//! node graph, so this needs no per-frame `update()` tick.
//!
//! [`play_sfx`]: WebAudio::play_sfx
//! [`play_on_bus`]: WebAudio::play_on_bus
//! [`play_sfx_on_bus`]: WebAudio::play_sfx_on_bus
//! [`set_bus_volume`]: WebAudio::set_bus_volume
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
//!     a.play(JUMP_WAV_BYTES);            // fire-and-forget one-shot SFX
//!     let s = a.play_sfx(JUMP_WAV_BYTES); // controllable SFX:
//!     s.set_pan(-0.8);                    //   pan it left, set volume, or stop it later
//!     a.play_music(THEME_OGG_BYTES);     // looping music on the music channel
//!     // a.stop_music();                 // stop it
//!     // a.suspend(); a.resume();        // pause / unpause everything
//!     a.set_bus_volume("sfx", 0.5);      // a named mixer bus, controlled as a group:
//!     a.play_sfx_on_bus(HIT_WAV_BYTES, "sfx"); //   this SFX rides the "sfx" bus volume
//! }
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
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
    /// Master gain — every source (directly or via a bus) connects here, and here to the
    /// destination.
    master: web_sys::GainNode,
    /// The current looping music source (single channel): set by `play_music`, cleared by
    /// `stop_music`. Stored once decoding finishes (playback is async).
    music: Rc<RefCell<Option<web_sys::AudioBufferSourceNode>>>,
    /// Named mixer buses: each is a `GainNode` wired `bus → master`. Sounds routed to a bus
    /// (via the `*_on_bus` methods) share its gain, so `set_bus_volume` controls them as a group.
    /// Created lazily on first reference and kept (so a volume-only bus persists in `bus_names`).
    buses: Rc<RefCell<HashMap<String, web_sys::GainNode>>>,
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
            buses: Rc::new(RefCell::new(HashMap::new())),
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

    /// Whether the audio context is currently running (i.e. audio is unlocked and not
    /// suspended). Browsers start the context **suspended** until a user gesture, and
    /// [`suspend`](Self::suspend) puts it back to suspended — use this to drive a "tap to
    /// enable sound" prompt or a paused-audio indicator.
    pub fn is_running(&self) -> bool {
        self.ctx.state() == web_sys::AudioContextState::Running
    }

    /// Whether the music channel is currently occupied — `true` after
    /// [`play_music`](Self::play_music) finishes decoding and starts, `false` until then and
    /// after [`stop_music`](Self::stop_music). Useful for a music on/off UI toggle.
    pub fn is_music_playing(&self) -> bool {
        self.music.borrow().is_some()
    }

    // ── Named mixer buses ────────────────────────────────────────────────────

    /// Returns the `GainNode` for the named `bus`, creating it (wired `bus → master`) on first
    /// reference. `None` only if the browser refuses to create/wire the node — callers then route
    /// straight to master so playback still happens.
    fn bus_gain(&self, name: &str) -> Option<web_sys::GainNode> {
        if let Some(g) = self.buses.borrow().get(name) {
            return Some(g.clone());
        }
        let g = self.ctx.create_gain().ok()?;
        g.connect_with_audio_node(&self.master).ok()?;
        self.buses.borrow_mut().insert(name.to_string(), g.clone());
        Some(g)
    }

    /// Sets the volume of the named mixer `bus`, clamped to `0.0..=1.0` — affects every sound
    /// routed to it (via [`play_on_bus`](Self::play_on_bus) / [`play_sfx_on_bus`](Self::play_sfx_on_bus)),
    /// now and in the future. The bus is created on first use, so calling this for an unknown name
    /// registers it (it then shows up in [`bus_names`](Self::bus_names)) — the wasm analogue of the
    /// native mixer's "volume-only" buses.
    pub fn set_bus_volume(&self, bus: &str, v: f32) {
        if let Some(g) = self.bus_gain(bus) {
            g.gain().set_value(v.clamp(0.0, 1.0));
        }
    }

    /// The current volume of the named `bus` (`1.0` if the bus does not exist yet). Read-only — it
    /// does **not** create the bus.
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.buses
            .borrow()
            .get(bus)
            .map(|g| g.gain().value())
            .unwrap_or(1.0)
    }

    /// Every known bus name, sorted. A bus becomes "known" once a sound is routed to it or its
    /// volume is set. Useful for building a mixer UI.
    pub fn bus_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.buses.borrow().keys().cloned().collect();
        names.sort();
        names
    }

    /// Decodes `bytes` (an encoded audio clip — whatever the browser decodes: WAV/MP3/OGG) and
    /// plays it once through the master gain (fire-and-forget). Browsers gate audio behind a user
    /// gesture, so the first call should originate from an input handler (or call
    /// [`resume`](Self::resume)).
    pub fn play(&self, bytes: &[u8]) {
        self.decode_then_play_to(bytes, false, false, &self.master);
    }

    /// Like [`play`](Self::play), but routes the one-shot SFX through the named mixer `bus` (created
    /// on first use) instead of straight to the master gain, so [`set_bus_volume`](Self::set_bus_volume)
    /// scales it along with everything else on that bus.
    pub fn play_on_bus(&self, bytes: &[u8], bus: &str) {
        let dest = self.bus_gain(bus).unwrap_or_else(|| self.master.clone());
        self.decode_then_play_to(bytes, false, false, &dest);
    }

    /// Plays `bytes` as looping music on the single music channel: stops any current music, then
    /// starts the new clip looping (through the master gain). Stop it with
    /// [`stop_music`](Self::stop_music).
    pub fn play_music(&self, bytes: &[u8]) {
        self.stop_music();
        self.decode_then_play_to(bytes, true, true, &self.master);
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

    /// Plays `bytes` as a **controllable** one-shot SFX and returns an [`Sfx`] handle for
    /// per-source volume, stereo pan, and stop. Routes `source → stereo panner → per-source gain →
    /// master gain`, so the returned handle's [`set_volume`](Sfx::set_volume) /
    /// [`set_pan`](Sfx::set_pan) work **immediately** (the panner + gain are created synchronously,
    /// before decoding finishes). Unlike fire-and-forget [`play`](Self::play), keep the handle to
    /// adjust or [`stop`](Sfx::stop) the sound. Like all playback, gated behind a user gesture.
    pub fn play_sfx(&self, bytes: &[u8]) -> Sfx {
        self.play_sfx_to(bytes, &self.master)
    }

    /// Like [`play_sfx`](Self::play_sfx), but the controllable SFX is routed through the named mixer
    /// `bus` (created on first use): `source → panner → per-source gain → bus gain → master`. The
    /// returned [`Sfx`] still has its own independent volume/pan; [`set_bus_volume`](Self::set_bus_volume)
    /// scales the whole bus on top of that.
    pub fn play_sfx_on_bus(&self, bytes: &[u8], bus: &str) -> Sfx {
        let dest = self.bus_gain(bus).unwrap_or_else(|| self.master.clone());
        self.play_sfx_to(bytes, &dest)
    }

    /// Shared `play_sfx` path: wires `source → panner → per-source gain → dest` (where `dest` is the
    /// master gain or a bus gain). Per-source nodes are created now so `set_volume`/`set_pan` apply
    /// before the clip decodes; if node creation/wiring fails, the bare source routes straight to
    /// `dest` (no per-source control) so playback still happens.
    fn play_sfx_to(&self, bytes: &[u8], dest: &web_sys::GainNode) -> Sfx {
        let (gain, panner) = match (self.ctx.create_gain(), self.ctx.create_stereo_panner()) {
            (Ok(g), Ok(p)) => {
                let wired = p.connect_with_audio_node(&g).is_ok()
                    && g.connect_with_audio_node(dest).is_ok();
                if wired {
                    (Some(g), Some(p))
                } else {
                    (None, None)
                }
            }
            _ => (None, None),
        };
        let sfx = Sfx {
            gain,
            panner,
            source: Rc::new(RefCell::new(None)),
        };

        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let dest = dest.clone();
        let panner = sfx.panner.clone();
        let slot = sfx.source.clone();
        let promise = match ctx.decode_audio_data(&buffer) {
            Ok(p) => p,
            Err(_) => return sfx,
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
            // Into the per-source panner when we have one, else straight to dest.
            let connected = match &panner {
                Some(p) => src.connect_with_audio_node(p).is_ok(),
                None => src.connect_with_audio_node(&dest).is_ok(),
            };
            if connected {
                let _ = src.start();
                *slot.borrow_mut() = Some(src);
            }
        });
        sfx
    }

    /// Shared decode→connect→start path. `looping` sets loop mode; `is_music` stores the source
    /// in the music channel so it can be stopped later. `dest` is the master gain or a bus gain.
    fn decode_then_play_to(
        &self,
        bytes: &[u8],
        looping: bool,
        is_music: bool,
        dest: &web_sys::GainNode,
    ) {
        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let dest = dest.clone();
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
            if src.connect_with_audio_node(&dest).is_ok() {
                let _ = src.start();
                if let Some(music) = music {
                    *music.borrow_mut() = Some(src);
                }
            }
        });
    }
}

/// A handle to one sound effect started by [`WebAudio::play_sfx`].
///
/// Holds this source's own gain + stereo-panner nodes — created synchronously, so
/// [`set_volume`](Self::set_volume) / [`set_pan`](Self::set_pan) take effect immediately, even
/// before the clip finishes decoding — plus the buffer source once it starts. Cloning is cheap
/// (the JS nodes are reference-counted by the browser; the source slot is an `Rc`), so a clone
/// controls the same sound. (If the per-source nodes couldn't be created, volume/pan are no-ops and
/// the sound still plays through the master gain.)
#[derive(Clone)]
pub struct Sfx {
    /// Per-source gain (independent of master volume); `None` if node creation failed.
    gain: Option<web_sys::GainNode>,
    /// Per-source stereo panner; `None` if node creation failed.
    panner: Option<web_sys::StereoPannerNode>,
    /// The buffer source, stored once decoding finishes and playback starts.
    source: Rc<RefCell<Option<web_sys::AudioBufferSourceNode>>>,
}

impl Sfx {
    /// Sets this sound's volume, clamped to `0.0..=1.0` — independent of the master volume.
    pub fn set_volume(&self, v: f32) {
        if let Some(g) = &self.gain {
            g.gain().set_value(v.clamp(0.0, 1.0));
        }
    }

    /// Sets this sound's stereo pan: `-1.0` = full left, `0.0` = center, `1.0` = full right
    /// (clamped).
    pub fn set_pan(&self, pan: f32) {
        if let Some(p) = &self.panner {
            p.pan().set_value(pan.clamp(-1.0, 1.0));
        }
    }

    /// Whether this sound has started and not been [`stop`](Self::stop)ped. (A non-looping clip is
    /// **not** auto-cleared when it finishes naturally — same semantics as
    /// [`WebAudio::is_music_playing`].)
    pub fn is_playing(&self) -> bool {
        self.source.borrow().is_some()
    }

    /// Stops this sound if it is still playing.
    // web-sys deprecates `AudioBufferSourceNode::stop*` (no non-deprecated stop binding is exposed);
    // the call is correct, so allow it locally — same as `WebAudio::stop_music`.
    #[allow(deprecated)]
    pub fn stop(&self) {
        if let Some(src) = self.source.borrow_mut().take() {
            let _ = src.stop();
        }
    }
}
