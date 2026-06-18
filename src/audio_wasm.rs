//! Web Audio SFX + music for the web (wasm) via the Web Audio API.
//!
//! The full [`AudioManager`](crate::audio::AudioManager) is rodio-based and **native-only**
//! (`cfg(not(target_arch = "wasm32"))`). [`WebAudio`] is the browser counterpart: a small
//! `AudioContext` wrapper that covers the common needs — fire-and-forget sound effects,
//! **controllable** per-source SFX with **stereo pan** + volume + stop ([`play_sfx`] → [`Sfx`]), a
//! single looping **music** channel you can stop or **crossfade** between tracks
//! ([`crossfade_music`]), a **master volume**, **named mixer buses** ([`set_bus_volume`] + the
//! [`play_on_bus`]/[`play_sfx_on_bus`] variants) with **manual ducking** ([`duck_bus`] /
//! [`release_bus`]), **2D positional** playback ([`play_at`] / [`play_at_on_bus`] +
//! [`Sfx::update_position`]), and **suspend/resume** for pausing all audio. (Automatic *sidechain*
//! ducking remains native-only.)
//!
//! A **bus** is a `duck → volume → master` [`GainNode`](web_sys::GainNode) chain sitting between
//! sounds and the master gain: route sounds to a bus by name and control them together with
//! [`set_bus_volume`] (and dip them with [`duck_bus`]) — the wasm analogue of the native
//! [`AudioManager`](crate::audio::AudioManager) bus mixer. Web Audio is a node graph, so this needs
//! no per-frame `update()` tick.
//!
//! [`play_sfx`]: WebAudio::play_sfx
//! [`play_on_bus`]: WebAudio::play_on_bus
//! [`play_sfx_on_bus`]: WebAudio::play_sfx_on_bus
//! [`set_bus_volume`]: WebAudio::set_bus_volume
//! [`crossfade_music`]: WebAudio::crossfade_music
//! [`duck_bus`]: WebAudio::duck_bus
//! [`release_bus`]: WebAudio::release_bus
//! [`play_at`]: WebAudio::play_at
//! [`play_at_on_bus`]: WebAudio::play_at_on_bus
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
//!     a.crossfade_music(BOSS_OGG_BYTES, 2.0); // ... or cross-fade to a new track over 2s
//!     // a.stop_music();                 // stop it
//!     // a.suspend(); a.resume();        // pause / unpause everything
//!     a.set_bus_volume("sfx", 0.5);      // a named mixer bus, controlled as a group:
//!     a.play_sfx_on_bus(HIT_WAV_BYTES, "sfx"); //   this SFX rides the "sfx" bus volume
//! }
//! ```

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use glam::Vec2;
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
    /// The current looping music (single channel): its source plus a dedicated gain node, so the
    /// volume can be ramped independently for [`crossfade_music`](Self::crossfade_music). Set once
    /// decoding finishes (playback is async), cleared by `stop_music`.
    music: Rc<RefCell<Option<MusicChannel>>>,
    /// Named mixer buses, each a two-gain chain `duck → volume → master`. Sounds routed to a bus
    /// (via the `*_on_bus` methods) connect to its `duck` node, so `set_bus_volume` (the `volume`
    /// node) and `duck_bus`/`release_bus` (the `duck` node) control the group independently — the
    /// duck multiplier rides on top of the bus volume, matching the native mixer. Created lazily on
    /// first reference and kept (so a volume-only bus persists in `bus_names`).
    buses: Rc<RefCell<HashMap<String, Bus>>>,
    /// Monotonically-increasing generation counter for the music channel. Bumped every time a new
    /// `start_music` is initiated or `stop_music` is called. Each pending async decode closure
    /// captures its own generation at spawn time; if the counter has moved on by the time the
    /// closure resolves, the track is superseded — it stops itself instead of orphaning.
    music_gen: Rc<Cell<u64>>,
}

/// The currently-playing looping music: its buffer source and the per-music gain node it routes
/// through (`source → gain → master`). The gain lets [`WebAudio::crossfade_music`] ramp this track's
/// volume on the audio clock without touching the master or any other sound.
struct MusicChannel {
    source: web_sys::AudioBufferSourceNode,
    gain: web_sys::GainNode,
}

/// A named mixer bus: two gain nodes in series, `duck → volume → master`. Sounds connect to `duck`
/// (the input). `volume` is the user-set bus level ([`WebAudio::set_bus_volume`]); `duck` is the
/// automatic/temporary ducking multiplier ([`WebAudio::duck_bus`]). Keeping them separate means a
/// duck ramp never clobbers the bus volume and vice-versa. Cloning is cheap (JS nodes are
/// reference-counted by the browser).
#[derive(Clone)]
struct Bus {
    /// User-set bus volume (`volume → master`).
    volume: web_sys::GainNode,
    /// Ducking multiplier and the bus input (`duck → volume`); rests at `1.0`.
    duck: web_sys::GainNode,
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
            music_gen: Rc::new(Cell::new(0)),
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

    /// Returns the named `bus` (its `duck`+`volume` gain chain), creating it wired
    /// `duck → volume → master` on first reference. `None` only if the browser refuses to
    /// create/wire the nodes — callers then route straight to master so playback still happens.
    fn bus_entry(&self, name: &str) -> Option<Bus> {
        if let Some(b) = self.buses.borrow().get(name) {
            return Some(b.clone());
        }
        let volume = self.ctx.create_gain().ok()?;
        let duck = self.ctx.create_gain().ok()?;
        duck.connect_with_audio_node(&volume).ok()?;
        volume.connect_with_audio_node(&self.master).ok()?;
        let bus = Bus { volume, duck };
        self.buses
            .borrow_mut()
            .insert(name.to_string(), bus.clone());
        Some(bus)
    }

    /// The input node sounds connect to for the named `bus` (its `duck` gain), creating the bus on
    /// first use. `None` if the bus can't be created (caller falls back to master).
    fn bus_input(&self, name: &str) -> Option<web_sys::GainNode> {
        self.bus_entry(name).map(|b| b.duck)
    }

    /// Sets the volume of the named mixer `bus`, clamped to `0.0..=1.0` — affects every sound
    /// routed to it (via [`play_on_bus`](Self::play_on_bus) / [`play_sfx_on_bus`](Self::play_sfx_on_bus)),
    /// now and in the future. The bus is created on first use, so calling this for an unknown name
    /// registers it (it then shows up in [`bus_names`](Self::bus_names)) — the wasm analogue of the
    /// native mixer's "volume-only" buses.
    pub fn set_bus_volume(&self, bus: &str, v: f32) {
        if let Some(b) = self.bus_entry(bus) {
            b.volume.gain().set_value(v.clamp(0.0, 1.0));
        }
    }

    /// The current volume of the named `bus` (`1.0` if the bus does not exist yet). Read-only — it
    /// does **not** create the bus. This is the user-set bus level, independent of any active duck
    /// (see [`bus_duck`](Self::bus_duck)).
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.buses
            .borrow()
            .get(bus)
            .map(|b| b.volume.gain().value())
            .unwrap_or(1.0)
    }

    /// Every known bus name, sorted. A bus becomes "known" once a sound is routed to it or its
    /// volume is set. Useful for building a mixer UI.
    pub fn bus_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.buses.borrow().keys().cloned().collect();
        names.sort();
        names
    }

    /// Ducks the named `bus` — ramps its duck multiplier toward `gain` (clamped `0.0..=1.0`) over
    /// `attack_secs` seconds on the audio clock, on top of the bus volume. Use it to dip music/SFX
    /// while a voice line or important cue plays, then call [`release_bus`](Self::release_bus) to
    /// bring it back. The bus is created on first use.
    ///
    /// Like all the bus/crossfade ramps this is scheduled on the Web Audio clock
    /// (`AudioParam::linear_ramp_to_value_at_time`), so there is **no per-frame `update()` tick** to
    /// drive — unlike the native [`AudioManager::duck_bus`](crate::audio::AudioManager::duck_bus).
    /// (Automatic **sidechain** ducking stays native-only: it needs continuous "is the trigger bus
    /// playing?" evaluation, which doesn't fit the fire-and-forget Web Audio model — drive ducking
    /// manually from your game logic with `duck_bus`/`release_bus` instead.)
    pub fn duck_bus(&self, bus: &str, gain: f32, attack_secs: f32) {
        if let Some(b) = self.bus_entry(bus) {
            self.ramp_gain_to(&b.duck, gain.clamp(0.0, 1.0), attack_secs.max(0.0) as f64);
        }
    }

    /// Releases the duck on the named `bus` — ramps its duck multiplier back to `1.0` over
    /// `release_secs` seconds. The inverse of [`duck_bus`](Self::duck_bus).
    pub fn release_bus(&self, bus: &str, release_secs: f32) {
        if let Some(b) = self.bus_entry(bus) {
            self.ramp_gain_to(&b.duck, 1.0, release_secs.max(0.0) as f64);
        }
    }

    /// The current duck multiplier for the named `bus` (`1.0` = no duck / unknown bus). Read-only —
    /// does **not** create the bus. During a [`duck_bus`](Self::duck_bus)/[`release_bus`](Self::release_bus)
    /// ramp this reflects the in-progress value.
    pub fn bus_duck(&self, bus: &str) -> f32 {
        self.buses
            .borrow()
            .get(bus)
            .map(|b| b.duck.gain().value())
            .unwrap_or(1.0)
    }

    /// Decodes `bytes` (an encoded audio clip — whatever the browser decodes: WAV/MP3/OGG) and
    /// plays it once through the master gain (fire-and-forget). Browsers gate audio behind a user
    /// gesture, so the first call should originate from an input handler (or call
    /// [`resume`](Self::resume)).
    pub fn play(&self, bytes: &[u8]) {
        self.decode_then_play_to(bytes, &self.master);
    }

    /// Like [`play`](Self::play), but routes the one-shot SFX through the named mixer `bus` (created
    /// on first use) instead of straight to the master gain, so [`set_bus_volume`](Self::set_bus_volume)
    /// scales it along with everything else on that bus.
    pub fn play_on_bus(&self, bytes: &[u8], bus: &str) {
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        self.decode_then_play_to(bytes, &dest);
    }

    /// Plays `bytes` as looping music on the single music channel: stops any current music, then
    /// starts the new clip looping (through the master gain). Stop it with
    /// [`stop_music`](Self::stop_music). For a smooth track-to-track transition use
    /// [`crossfade_music`](Self::crossfade_music) instead.
    pub fn play_music(&self, bytes: &[u8]) {
        self.stop_music();
        self.start_music(bytes, None);
    }

    /// Crossfades from the current music track to a new one over `dur` seconds: the old track fades
    /// **out** (then stops) while the new track simultaneously fades **in**, so the two overlap for
    /// a true crossfade rather than a cut. If nothing is playing, this is just a fade-in.
    ///
    /// The fades are scheduled on the Web Audio clock via the per-track gain nodes
    /// (`AudioParam::linear_ramp_to_value_at_time`), so — unlike the native
    /// [`AudioManager::crossfade`](crate::audio::AudioManager::crossfade) — there is **no** per-frame
    /// `update()` tick to drive and no temporary channel to tear down.
    pub fn crossfade_music(&self, bytes: &[u8], dur: f32) {
        let dur = dur.max(0.0) as f64;
        // Fade the current track out on its own gain, then stop it at the end of the fade.
        if let Some(old) = self.music.borrow_mut().take() {
            self.ramp_gain_to(&old.gain, 0.0, dur);
            self.stop_at(&old.source, self.ctx.current_time() + dur);
        }
        // Start the new track with a fade-in; it becomes the current music channel.
        self.start_music(bytes, Some(dur));
    }

    /// Stops the current looping music, if any.
    // web-sys deprecates the `AudioBufferSourceNode::stop*` bindings (no non-deprecated
    // equivalent is exposed for stopping a source); the call is correct, so allow it locally.
    #[allow(deprecated)]
    pub fn stop_music(&self) {
        // Bump the generation so any in-flight start_music async closure sees it is superseded
        // and does not install or start its track.
        self.music_gen.set(self.music_gen.get() + 1);
        if let Some(ch) = self.music.borrow_mut().take() {
            let _ = ch.source.stop();
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
    /// `bus` (created on first use): `source → panner → per-source gain → bus duck → bus volume →
    /// master`. The returned [`Sfx`] still has its own independent volume/pan;
    /// [`set_bus_volume`](Self::set_bus_volume) (and [`duck_bus`](Self::duck_bus)) scale the whole
    /// bus on top of that.
    pub fn play_sfx_on_bus(&self, bytes: &[u8], bus: &str) -> Sfx {
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        self.play_sfx_to(bytes, &dest)
    }

    /// Plays `bytes` as a **positional** one-shot SFX at `source` in 2D space, heard from
    /// `listener`, and returns the [`Sfx`] handle. Volume falls off linearly with distance (silent
    /// at `max_dist`) and stereo pan follows the x-offset — the wasm analogue of the native
    /// [`AudioManager::play_at`](crate::audio::AudioManager::play_at). Keep the handle and call
    /// [`Sfx::update_position`] each frame to track a moving source. Routes to the master gain (use
    /// the returned [`Sfx`] for per-source control).
    pub fn play_at(&self, bytes: &[u8], source: Vec2, listener: Vec2, max_dist: f32) -> Sfx {
        let sfx = self.play_sfx(bytes);
        sfx.update_position(source, listener, max_dist);
        sfx
    }

    /// Like [`play_at`](Self::play_at), but routes the positional SFX through the named mixer `bus`
    /// (created on first use) instead of straight to master — so the named bus's
    /// [`set_bus_volume`](Self::set_bus_volume) and [`duck_bus`](Self::duck_bus) scale the whole
    /// group on top of this sound's distance-based volume/pan. The returned [`Sfx`]'s per-source
    /// volume/pan still carry the spatial result (independent of the bus level).
    pub fn play_at_on_bus(
        &self,
        bytes: &[u8],
        source: Vec2,
        listener: Vec2,
        max_dist: f32,
        bus: &str,
    ) -> Sfx {
        let sfx = self.play_sfx_on_bus(bytes, bus);
        sfx.update_position(source, listener, max_dist);
        sfx
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
            stopped: Rc::new(Cell::new(false)),
        };

        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let dest = dest.clone();
        let panner = sfx.panner.clone();
        let slot = sfx.source.clone();
        let stopped = sfx.stopped.clone();
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
                // If stop() was called before decode finished, don't start the sound.
                if stopped.get() {
                    return;
                }
                let _ = src.start();
                *slot.borrow_mut() = Some(src);
            }
        });
        sfx
    }

    /// Shared decode→connect→start path for one-shot SFX. `dest` is the master gain or a bus gain.
    fn decode_then_play_to(&self, bytes: &[u8], dest: &web_sys::GainNode) {
        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let dest = dest.clone();
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
            if src.connect_with_audio_node(&dest).is_ok() {
                let _ = src.start();
            }
        });
    }

    /// Decodes `bytes` and starts it as the looping music channel through a fresh per-music gain
    /// node (`source → gain → master`), replacing whatever is in `self.music`. When `fade_in` is
    /// `Some(dur)`, the gain ramps `0 → 1` over `dur` seconds starting at the source's real start
    /// time (so the ramp can't race the async decode). Used by both
    /// [`play_music`](Self::play_music) (no fade) and [`crossfade_music`](Self::crossfade_music).
    ///
    /// **Fix 2 (gain leak):** The per-music gain node is created and wired *inside* the async
    /// closure, after decoding succeeds, so a decode failure never leaves a connected gain node
    /// dangling in the audio graph.
    ///
    /// **Fix 3 (orphaned track):** A generation counter is bumped each call (and in `stop_music`).
    /// The closure checks its captured generation before installing; a superseded decode stops its
    /// source instead of orphaning it.
    #[allow(deprecated)] // web-sys deprecates AudioBufferSourceNode::stop* — no non-deprecated alt
    fn start_music(&self, bytes: &[u8], fade_in: Option<f64>) {
        // Bump generation; capture the new value for this specific decode task.
        let my_gen = self.music_gen.get() + 1;
        self.music_gen.set(my_gen);
        let music_gen = self.music_gen.clone();

        let array = js_sys::Uint8Array::from(bytes);
        let buffer = array.buffer();
        let ctx = self.ctx.clone();
        let master = self.master.clone();
        let music = self.music.clone();
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
            // Fix 3: if a newer start_music or stop_music has run while we were decoding,
            // this track is superseded — don't start it.
            if music_gen.get() != my_gen {
                return;
            }
            // Fix 2: create and wire the gain node only after a successful decode. This ensures
            // a decode failure never leaves an orphaned connected node in the audio graph.
            let Ok(gain) = ctx.create_gain() else {
                return;
            };
            if gain.connect_with_audio_node(&master).is_err() {
                return;
            }
            let Ok(src) = ctx.create_buffer_source() else {
                return;
            };
            src.set_buffer(Some(&audio_buffer));
            src.set_loop(true);
            if src.connect_with_audio_node(&gain).is_ok() {
                // Schedule the fade-in at the *actual* start time, anchoring gain=0 first so the
                // ramp is well-defined regardless of how long decoding took.
                if let Some(dur) = fade_in {
                    let now = ctx.current_time();
                    let _ = gain.gain().set_value_at_time(0.0, now);
                    let _ = gain.gain().linear_ramp_to_value_at_time(1.0, now + dur);
                }
                // Final generation check immediately before installing + starting: another
                // crossfade/stop may have raced in after the first check above.
                if music_gen.get() != my_gen {
                    let _ = src.stop();
                    return;
                }
                let _ = src.start();
                *music.borrow_mut() = Some(MusicChannel { source: src, gain });
            }
        });
    }

    /// Ramps a gain node's value to `target` over `dur` seconds on the audio clock, anchoring the
    /// current value first so the linear ramp starts from where the gain actually is.
    ///
    /// `dur <= 0.0` is an **instant** set: it cancels any pending automation and writes the value
    /// directly (the `value=` setter), which also makes it immediately readable via
    /// [`AudioParam::value`] (a scheduled ramp's live value is not — it's computed on the audio
    /// render thread).
    fn ramp_gain_to(&self, gain: &web_sys::GainNode, target: f32, dur: f64) {
        let param = gain.gain();
        if dur <= 0.0 {
            let _ = param.cancel_scheduled_values(0.0);
            param.set_value(target);
            return;
        }
        let now = self.ctx.current_time();
        let _ = param.set_value_at_time(param.value(), now);
        let _ = param.linear_ramp_to_value_at_time(target, now + dur);
    }

    /// Schedules a source to stop at audio-clock time `when`.
    // web-sys deprecates the `AudioBufferSourceNode::stop*` bindings (no non-deprecated equivalent
    // is exposed); the call is correct — same as `stop_music` / `Sfx::stop`.
    #[allow(deprecated)]
    fn stop_at(&self, source: &web_sys::AudioBufferSourceNode, when: f64) {
        let _ = source.stop_with_when(when);
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
    /// Set to `true` by [`stop`](Self::stop). The async decode closure checks this flag before
    /// calling `src.start()` so that a stop-before-decode truly silences the sound.
    stopped: Rc<Cell<bool>>,
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

    /// This sound's current volume (its per-source gain; `1.0` if the gain node is absent).
    pub fn volume(&self) -> f32 {
        self.gain.as_ref().map(|g| g.gain().value()).unwrap_or(1.0)
    }

    /// This sound's current stereo pan (`0.0` if the panner is absent).
    pub fn pan(&self) -> f32 {
        self.panner.as_ref().map(|p| p.pan().value()).unwrap_or(0.0)
    }

    /// Repositions this sound in 2D space: recomputes volume (linear distance falloff, silent at
    /// `max_dist`) and stereo pan (x-offset) from `source`/`listener` and applies them via
    /// [`set_volume`](Self::set_volume)/[`set_pan`](Self::set_pan). Call every frame to track a
    /// moving source. See [`WebAudio::play_at`].
    pub fn update_position(&self, source: Vec2, listener: Vec2, max_dist: f32) {
        let (vol, pan) = crate::audio_spatial::spatial_params(source, listener, max_dist);
        self.set_volume(vol);
        self.set_pan(pan);
    }

    /// Whether this sound has started and not been [`stop`](Self::stop)ped. (A non-looping clip is
    /// **not** auto-cleared when it finishes naturally — same semantics as
    /// [`WebAudio::is_music_playing`].)
    pub fn is_playing(&self) -> bool {
        self.source.borrow().is_some()
    }

    /// Stops this sound if it is still playing.
    ///
    /// Also sets the `stopped` flag so that if the async decode hasn't finished yet, the sound
    /// will not start when it eventually does (fix for stop-before-decode no-op).
    // web-sys deprecates `AudioBufferSourceNode::stop*` (no non-deprecated stop binding is exposed);
    // the call is correct, so allow it locally — same as `WebAudio::stop_music`.
    #[allow(deprecated)]
    pub fn stop(&self) {
        // Mark stopped first so the async decode closure won't start the sound.
        self.stopped.set(true);
        if let Some(src) = self.source.borrow_mut().take() {
            let _ = src.stop();
        }
    }
}
