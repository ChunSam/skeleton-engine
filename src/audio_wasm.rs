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
//! [`AudioManager`](crate::audio::AudioManager) bus mixer. Web Audio is a node graph, so volumes,
//! ramps and ducks need no per-frame tick. [`update`](WebAudio::update) exists only to sample the
//! **level meters** ([`enable_analysis`](WebAudio::enable_analysis)); a page that never analyzes
//! anything need not call it.
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

use crate::audio_analysis::{
    log_band_range, resample_bands, smooth_toward, AudioLevels, MAX_DB, MIN_DB, SPECTRUM_BANDS,
};

/// FFT size of each analysis node. Must be a power of two; 1024 matches the native tap's window
/// so both backends average over a comparable slice of audio (~21 ms at 48 kHz).
const ANALYSER_FFT_SIZE: u32 = 1024;

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
    /// Named, caller-addressable synthesized-tone channels (see [`play_tone_on_channel`]). Unlike the
    /// fire-and-forget [`play_tone`], a named channel is tracked so [`is_channel_playing`] can report
    /// liveness and a replay can cut the previous tone. Keyed by the caller's channel name.
    ///
    /// [`play_tone_on_channel`]: WebAudio::play_tone_on_channel
    /// [`play_tone`]: WebAudio::play_tone
    /// [`is_channel_playing`]: WebAudio::is_channel_playing
    tone_channels: Rc<RefCell<HashMap<String, ToneVoice>>>,
    /// Per-channel low-pass cutoff (Hz) pending for the named tone channels, applied to the next
    /// [`play_tone_on_channel`](WebAudio::play_tone_on_channel) on that channel (mirrors the native
    /// `AudioEffect` "applied on next play" semantics). A `BiquadFilterNode` is inserted into the
    /// channel's graph when an entry is present.
    tone_lowpass: Rc<RefCell<HashMap<String, f32>>>,
    /// Named, caller-addressable **positional** sounds (see [`play_at_on_channel`]). Each is a
    /// looping [`Sfx`] whose distance-based volume/pan is updated each frame via
    /// [`update_position`]; keyed by the caller's channel name so the facade can reposition or stop
    /// it by name (the web analogue of the native `AudioManager` positional channel).
    ///
    /// [`play_at_on_channel`]: WebAudio::play_at_on_channel
    /// [`update_position`]: WebAudio::update_position
    spatial_channels: Rc<RefCell<HashMap<String, Sfx>>>,
    /// Channels being level-analyzed (see [`enable_analysis`](WebAudio::enable_analysis)), keyed by
    /// channel name. An entry exists only while analysis is on; a play path connects the sound's
    /// own gain node to the matching analyser, so an unanalyzed channel builds the same graph it
    /// always did.
    analysers: Rc<RefCell<HashMap<String, AnalyserState>>>,
    /// Meter release time in seconds, shared by every analyzed channel.
    analysis_smoothing: Rc<Cell<f32>>,
}

/// One synthesized tone playing (or scheduled) on a named channel: the oscillator (kept so a replay
/// can cut it early, mirroring the native `stop_immediate` reuse) and its scheduled audio-clock stop
/// time (so [`WebAudio::is_channel_playing`] reports liveness without a callback).
struct ToneVoice {
    osc: web_sys::OscillatorNode,
    stop_time: f64,
}

/// One analyzed channel: the `AnalyserNode` that watches it, a scratch buffer for the time-domain
/// read, and the smoothed values [`WebAudio::levels`] reports.
///
/// Unlike the native tap, an `AnalyserNode` reads the *live* graph — when nothing is playing it
/// simply reads silence, so a stopped channel decays on its own with no staleness detection
/// needed. (The native tap only runs while rodio pulls samples, which is why that side carries a
/// sequence counter.)
struct AnalyserState {
    node: web_sys::AnalyserNode,
    /// Scratch buffer sized to the analyser's FFT size; reused every frame to avoid allocating.
    buf: Vec<f32>,
    rms: f32,
    peak: f32,
    /// Whether a spectrum was requested for this channel (the frequency read is skipped otherwise).
    spectrum: bool,
    /// Scratch for the browser's frequency data, one byte per bin (`fft_size / 2`).
    freq_buf: Vec<u8>,
    /// Smoothed log-spaced bands, in the shared internal resolution.
    bands: [f32; SPECTRUM_BANDS],
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
            tone_channels: Rc::new(RefCell::new(HashMap::new())),
            tone_lowpass: Rc::new(RefCell::new(HashMap::new())),
            spatial_channels: Rc::new(RefCell::new(HashMap::new())),
            analysers: Rc::new(RefCell::new(HashMap::new())),
            analysis_smoothing: Rc::new(Cell::new(
                crate::audio_analysis::DEFAULT_ANALYSIS_SMOOTHING,
            )),
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

    // ── Level analysis ────────────────────────────────────────────────────────

    /// Starts measuring `channel`'s loudness, so [`levels`](Self::levels) reports it.
    ///
    /// **Takes effect on the next play on that channel** — the analyser is wired into the graph
    /// when the sound is built, and a node already playing is not rewired. Enable it during setup.
    /// Matches the native
    /// [`AudioManager::enable_analysis`](crate::audio::AudioManager::enable_analysis) semantics.
    ///
    /// Pass [`MUSIC_CHANNEL`](crate::audio_analysis::MUSIC_CHANNEL) to meter the music channel.
    /// A channel with analysis off builds exactly the graph it did before.
    pub fn enable_analysis(&self, channel: &str) {
        let mut map = self.analysers.borrow_mut();
        if map.contains_key(channel) {
            return; // keep the existing reading rather than resetting it
        }
        let Ok(node) = self.ctx.create_analyser() else {
            return;
        };
        node.set_fft_size(ANALYSER_FFT_SIZE);
        // The node's own smoothing is left off: meter behaviour belongs to the shared
        // `smooth_toward`, so native and web match. Letting the node smooth too would apply a
        // second, different curve on web only.
        node.set_smoothing_time_constant(0.0);
        // Pin the dB window to the shared constants. These happen to be the Web Audio defaults,
        // but setting them explicitly is what guarantees a band here means what it means on
        // native — a browser changing its defaults must not silently desync the two.
        node.set_min_decibels(MIN_DB as f64);
        node.set_max_decibels(MAX_DB as f64);
        let buf = vec![0.0; node.fft_size() as usize];
        let freq_buf = vec![0u8; node.frequency_bin_count() as usize];
        map.insert(
            channel.to_string(),
            AnalyserState {
                node,
                buf,
                rms: 0.0,
                peak: 0.0,
                spectrum: false,
                freq_buf,
                bands: [0.0; SPECTRUM_BANDS],
            },
        );
    }

    /// Starts measuring `channel`'s **frequency spectrum** as well as its levels, so
    /// [`bands`](Self::bands) reports it. Implies [`enable_analysis`](Self::enable_analysis).
    ///
    /// Mirrors the native
    /// [`AudioManager::enable_spectrum`](crate::audio::AudioManager::enable_spectrum).
    pub fn enable_spectrum(&self, channel: &str) {
        self.enable_analysis(channel);
        if let Some(state) = self.analysers.borrow_mut().get_mut(channel) {
            state.spectrum = true;
        }
    }

    /// Stops reporting a spectrum for `channel`, leaving its levels running.
    pub fn disable_spectrum(&self, channel: &str) {
        if let Some(state) = self.analysers.borrow_mut().get_mut(channel) {
            state.spectrum = false;
            state.bands = [0.0; SPECTRUM_BANDS];
        }
    }

    /// Writes `channel`'s current smoothed spectrum into `out` as `out.len()` log-spaced bands.
    ///
    /// See the native
    /// [`AudioManager::bands`](crate::audio::AudioManager::bands) for the full contract — notably
    /// that cross-platform values are *equivalent, not identical*.
    pub fn bands(&self, channel: &str, out: &mut [f32]) {
        match self.analysers.borrow().get(channel) {
            Some(s) if s.spectrum => resample_bands(&s.bands, out),
            _ => out.fill(0.0),
        }
    }

    /// Stops measuring `channel` and unhooks its analyser from the graph.
    pub fn disable_analysis(&self, channel: &str) {
        if let Some(state) = self.analysers.borrow_mut().remove(channel) {
            let _ = state.node.disconnect();
        }
    }

    /// Returns whether `channel` is being analyzed.
    pub fn is_analysis_enabled(&self, channel: &str) -> bool {
        self.analysers.borrow().contains_key(channel)
    }

    /// Returns `channel`'s current smoothed loudness, or
    /// [`SILENT`](crate::AudioLevels::SILENT) if it is not analyzed.
    ///
    /// Values advance in [`update`](Self::update), so call that every frame (`AudioFacadeSystem`
    /// does).
    pub fn levels(&self, channel: &str) -> AudioLevels {
        self.analysers
            .borrow()
            .get(channel)
            .map(|s| AudioLevels {
                rms: s.rms,
                peak: s.peak,
            })
            .unwrap_or(AudioLevels::SILENT)
    }

    /// Sets the meter's release time in seconds — how long a level takes to fall. A rise is always
    /// instant. `0.0` disables smoothing. Applies to every analyzed channel.
    pub fn set_analysis_smoothing(&self, release_secs: f32) {
        self.analysis_smoothing.set(release_secs.max(0.0));
    }

    /// Returns the meter release time in seconds.
    pub fn analysis_smoothing(&self) -> f32 {
        self.analysis_smoothing.get()
    }

    /// Samples every analyser and advances its smoothed levels. Call once per frame.
    ///
    /// Unlike the native side there is no staleness check: an `AnalyserNode` reads the live graph,
    /// so a stopped channel reads silence and decays on its own.
    pub fn update(&self, dt: f32) {
        let release = self.analysis_smoothing.get();
        for state in self.analysers.borrow_mut().values_mut() {
            state.node.get_float_time_domain_data(&mut state.buf);
            let mut sum_sq = 0.0f32;
            let mut peak = 0.0f32;
            for &sample in state.buf.iter() {
                sum_sq += sample * sample;
                let magnitude = sample.abs();
                if magnitude > peak {
                    peak = magnitude;
                }
            }
            let rms = (sum_sq / state.buf.len().max(1) as f32).sqrt();
            state.rms = smooth_toward(state.rms, rms.min(1.0), release, dt);
            state.peak = smooth_toward(state.peak, peak.min(1.0), release, dt);

            if state.spectrum {
                // The browser already did the FFT and normalized each bin to 0..=255 across the
                // dB window we pinned in `enable_analysis` — so dividing by 255 lands on the same
                // 0..=1 scale the native side reaches via `normalized_db`.
                state.node.get_byte_frequency_data(&mut state.freq_buf);
                let usable = state.freq_buf.len();
                for (b, slot) in state.bands.iter_mut().enumerate() {
                    let (start, end) = log_band_range(b, SPECTRUM_BANDS, usable);
                    // Peak within the band, matching the native fold — a mean would average a
                    // narrow tone inside a wide high band into invisibility.
                    let mut bin_peak = 0u8;
                    for k in start..end {
                        if state.freq_buf[k] > bin_peak {
                            bin_peak = state.freq_buf[k];
                        }
                    }
                    let target = bin_peak as f32 / 255.0;
                    *slot = smooth_toward(*slot, target, release, dt);
                }
            }
        }
    }

    /// Connects `node` to `channel`'s analyser when analysis is enabled for it; otherwise does
    /// nothing. Called from the play paths *after* the sound's own gain node, so the measurement
    /// is pre-bus/pre-master — matching the native tap's position.
    fn tap(&self, channel: &str, node: &web_sys::AudioNode) {
        if let Some(state) = self.analysers.borrow().get(channel) {
            let _ = node.connect_with_audio_node(&state.node);
        }
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

    /// Synthesizes a pure sine-wave tone of `freq` Hz for `dur` seconds at amplitude `vol`
    /// (clamped `0.0..=1.0`), played fire-and-forget through the master gain — the web analogue of
    /// native tone synthesis ([`AudioManager::play_tone`](crate::audio::AudioManager::play_tone)),
    /// needing no clip bytes. A short attack/release envelope avoids click artifacts. Browsers gate
    /// audio behind a user gesture, so the first call should follow one (or call [`resume`](Self::resume)).
    pub fn play_tone(&self, freq: f32, dur: f32, vol: f32) {
        self.play_tone_to(freq, dur, vol, &self.master);
    }

    /// Like [`play_tone`](Self::play_tone), but routes the tone through the named mixer `bus` (created
    /// on first use) so [`set_bus_volume`](Self::set_bus_volume) / [`duck_bus`](Self::duck_bus) scale
    /// it as a group.
    pub fn play_tone_on_bus(&self, freq: f32, dur: f32, vol: f32, bus: &str) {
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        self.play_tone_to(freq, dur, vol, &dest);
    }

    /// Plays a tone as a **metered one-shot**: it overlaps its own previous plays like
    /// [`play_tone`](Self::play_tone), and `levels(meter)` reports it.
    ///
    /// The web backend needs no voice pool for this — [`play_tone_to`](Self::play_tone_to) already
    /// builds a fresh oscillator per call, so tones never cut each other; the only missing piece was
    /// routing each one into `meter`'s analyser. Connecting several sources to one `AnalyserNode`
    /// makes Web Audio **mix** them, which is exactly the "sum of the sounding voices" contract the
    /// native backend computes by hand (see `sum_levels` in `audio_analysis`).
    pub fn play_tone_metered(&self, meter: &str, freq: f32, dur: f32, vol: f32, bus: &str) {
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        self.play_tone_to_metered(freq, dur, vol, &dest, Some(meter));
    }

    /// Shared tone path for [`play_tone`](Self::play_tone) / [`play_tone_on_bus`](Self::play_tone_on_bus):
    /// wires `oscillator → gain → dest` (where `dest` is the master gain or a bus gain) and schedules
    /// a 0→`vol`→0 attack/release envelope on the gain over `dur` seconds, stopping the oscillator at
    /// the end. The browser keeps the connected nodes alive until the scheduled stop, so the tone is
    /// fully fire-and-forget. Does nothing if node creation/wiring fails.
    // web-sys deprecates the `OscillatorNode::start`/`stop*` (AudioScheduledSourceNode) bindings; the
    // calls are correct (same situation as `stop_at` / `stop_music`), so allow it locally.
    #[allow(deprecated)]
    fn play_tone_to(&self, freq: f32, dur: f32, vol: f32, dest: &web_sys::GainNode) {
        self.play_tone_to_metered(freq, dur, vol, dest, None);
    }

    /// [`play_tone_to`](Self::play_tone_to) with an optional meter name: when `meter` is `Some`,
    /// this tone's gain node is *also* connected to that name's analyser, so the tone is measured
    /// without giving up the fire-and-forget overlap. `None` is byte-identical to the old path.
    #[allow(deprecated)] // OscillatorNode::start/stop* bindings are deprecated — same as elsewhere
    fn play_tone_to_metered(
        &self,
        freq: f32,
        dur: f32,
        vol: f32,
        dest: &web_sys::GainNode,
        meter: Option<&str>,
    ) {
        let dur = dur.max(0.0) as f64;
        let vol = vol.clamp(0.0, 1.0);
        let (Ok(osc), Ok(gain)) = (self.ctx.create_oscillator(), self.ctx.create_gain()) else {
            return;
        };
        osc.set_type(web_sys::OscillatorType::Sine);
        osc.frequency().set_value(freq);
        if osc.connect_with_audio_node(&gain).is_err()
            || gain.connect_with_audio_node(dest).is_err()
        {
            return;
        }
        // Metered one-shot: also feed this voice's gain into the meter's analyser. Taken after the
        // tone's own gain but before `dest`, matching where every other tap sits. Multiple live
        // voices all connect here and the browser mixes them — that mixing IS the sum contract.
        if let Some(meter) = meter {
            self.tap(meter, &gain);
        }
        // Attack/release envelope (≤8ms, or a quarter of very short tones) — ramps prevent the
        // click a hard 0→vol step would make. Held flat between the two edges.
        let now = self.ctx.current_time();
        let edge = (dur * 0.25).min(0.008);
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0, now);
        let _ = g.linear_ramp_to_value_at_time(vol, now + edge);
        let _ = g.set_value_at_time(vol, (now + dur - edge).max(now + edge));
        let _ = g.linear_ramp_to_value_at_time(0.0, now + dur);
        let _ = osc.start();
        let _ = osc.stop_with_when(now + dur);
    }

    /// Plays a synthesized sine tone on a caller-named **channel** routed through the named mixer
    /// `bus`. Unlike the fire-and-forget [`play_tone`](Self::play_tone) (anonymous, can't be queried),
    /// a named channel is stable: [`is_channel_playing`](Self::is_channel_playing) reports whether it
    /// is still sounding (re-arm a sustained tone when it drains), and a replay on the same channel
    /// **cuts** the previous tone first (mirroring the native `play_tone` channel reuse). Any low-pass
    /// set via [`set_low_pass`](Self::set_low_pass) for this channel is inserted into the graph.
    pub fn play_tone_on_channel(&self, channel: &str, freq: f32, dur: f32, vol: f32, bus: &str) {
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        self.play_tone_channel_to(channel, freq, dur, vol, &dest);
    }

    /// Shared named-tone-channel path: cuts any tone already on `channel`, then wires
    /// `oscillator → [low-pass biquad] → gain → dest` and schedules the same click-free envelope as
    /// [`play_tone_to`](Self::play_tone_to), recording the oscillator + stop time so
    /// [`is_channel_playing`](Self::is_channel_playing) can report liveness. Does nothing if node
    /// creation/wiring fails.
    #[allow(deprecated)] // OscillatorNode::start/stop* bindings are deprecated — same as play_tone_to
    fn play_tone_channel_to(
        &self,
        channel: &str,
        freq: f32,
        dur: f32,
        vol: f32,
        dest: &web_sys::GainNode,
    ) {
        let dur = dur.max(0.0) as f64;
        let vol = vol.clamp(0.0, 1.0);
        // Replay cuts the previous tone on this channel (mirrors native stop_immediate reuse).
        if let Some(prev) = self.tone_channels.borrow_mut().remove(channel) {
            let _ = prev.osc.stop();
        }
        let (Ok(osc), Ok(gain)) = (self.ctx.create_oscillator(), self.ctx.create_gain()) else {
            return;
        };
        osc.set_type(web_sys::OscillatorType::Sine);
        osc.frequency().set_value(freq);
        // Optional low-pass: osc → biquad → gain, else osc → gain. Fall back to no filter if the
        // biquad can't be created so a tone still plays.
        let cutoff = self.tone_lowpass.borrow().get(channel).copied();
        let osc_out: web_sys::AudioNode = match cutoff.and_then(|hz| {
            let biquad = self.ctx.create_biquad_filter().ok()?;
            biquad.set_type(web_sys::BiquadFilterType::Lowpass);
            biquad.frequency().set_value(hz);
            osc.connect_with_audio_node(&biquad).ok()?;
            Some(biquad)
        }) {
            Some(biquad) => biquad.into(),
            None => osc.clone().into(),
        };
        if osc_out.connect_with_audio_node(&gain).is_err()
            || gain.connect_with_audio_node(dest).is_err()
        {
            return;
        }
        // Meter the tone's own envelope when this channel is analyzed (no-op otherwise). Taken
        // after the tone's gain but before `dest` (a bus or the master), so bus volume, ducking
        // and master volume are excluded — the same point the native tap sits at.
        self.tap(channel, &gain);
        let now = self.ctx.current_time();
        let edge = (dur * 0.25).min(0.008);
        let g = gain.gain();
        let _ = g.set_value_at_time(0.0, now);
        let _ = g.linear_ramp_to_value_at_time(vol, now + edge);
        let _ = g.set_value_at_time(vol, (now + dur - edge).max(now + edge));
        let _ = g.linear_ramp_to_value_at_time(0.0, now + dur);
        let _ = osc.start();
        let _ = osc.stop_with_when(now + dur);
        self.tone_channels.borrow_mut().insert(
            channel.to_string(),
            ToneVoice {
                osc,
                stop_time: now + dur,
            },
        );
    }

    /// Whether the named tone `channel` still has a tone sounding — `true` until the tone's scheduled
    /// stop time passes, `false` for an unknown channel. Use it to re-arm a sustained
    /// [`play_tone_on_channel`](Self::play_tone_on_channel) when it drains (the web analogue of the
    /// native [`AudioManager::is_playing`](crate::audio::AudioManager::is_playing) for tones).
    pub fn is_channel_playing(&self, channel: &str) -> bool {
        match self.tone_channels.borrow().get(channel) {
            Some(v) => self.ctx.current_time() < v.stop_time,
            None => false,
        }
    }

    /// Sets a low-pass filter cutoff (Hz) on the named tone `channel`, applied to the **next**
    /// [`play_tone_on_channel`](Self::play_tone_on_channel) on that channel (toggle, then replay to
    /// hear it) — the web `BiquadFilterNode` analogue of the native `AudioEffect.low_pass_hz`
    /// "applied on next play" semantics.
    pub fn set_low_pass(&self, channel: &str, cutoff_hz: u32) {
        self.tone_lowpass
            .borrow_mut()
            .insert(channel.to_string(), cutoff_hz as f32);
    }

    /// Removes the low-pass filter from the named tone `channel` (applied on the next play).
    pub fn clear_low_pass(&self, channel: &str) {
        self.tone_lowpass.borrow_mut().remove(channel);
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
        self.play_at_to(bytes, source, listener, max_dist, &self.master)
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
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        self.play_at_to(bytes, source, listener, max_dist, &dest)
    }

    /// Shared positional path for [`play_at`](Self::play_at) /
    /// [`play_at_on_bus`](Self::play_at_on_bus): plays a controllable SFX routed to `dest` (the
    /// master gain or a bus gain), then applies the distance-based volume/pan via
    /// [`Sfx::update_position`]. Mirrors [`play_sfx_to`](Self::play_sfx_to).
    fn play_at_to(
        &self,
        bytes: &[u8],
        source: Vec2,
        listener: Vec2,
        max_dist: f32,
        dest: &web_sys::GainNode,
    ) -> Sfx {
        let sfx = self.play_sfx_to(bytes, dest);
        sfx.update_position(source, listener, max_dist);
        sfx
    }

    /// Shared `play_sfx` path: wires `source → panner → per-source gain → dest` (where `dest` is the
    /// master gain or a bus gain). Per-source nodes are created now so `set_volume`/`set_pan` apply
    /// before the clip decodes; if node creation/wiring fails, the bare source routes straight to
    /// `dest` (no per-source control) so playback still happens.
    fn play_sfx_to(&self, bytes: &[u8], dest: &web_sys::GainNode) -> Sfx {
        self.play_sfx_to_opts(bytes, dest, false)
    }

    /// [`play_sfx_to`](Self::play_sfx_to) with a `repeat` flag — when `true`, the buffer source
    /// loops (`set_loop(true)`), used by the tracked positional channels
    /// ([`play_at_on_channel`](Self::play_at_on_channel)) which sustain while their position updates.
    fn play_sfx_to_opts(&self, bytes: &[u8], dest: &web_sys::GainNode, repeat: bool) -> Sfx {
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
            src.set_loop(repeat);
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

    /// Plays `bytes` as a **looping positional** sound on a caller-named **channel**, routed through
    /// the named mixer `bus`, and tracks it so [`update_position`](Self::update_position) /
    /// [`stop_channel`](Self::stop_channel) can address it by name. Distance-based volume + stereo pan
    /// are applied immediately from `source`/`listener`/`max_dist`; call `update_position` each frame
    /// to follow a moving source. Replacing a channel stops its previous sound first. The web analogue
    /// of the native `AudioManager::play_bytes_at` + positional channel.
    pub fn play_at_on_channel(
        &self,
        channel: &str,
        bytes: &[u8],
        source: Vec2,
        listener: Vec2,
        max_dist: f32,
        bus: &str,
    ) {
        // Stop any sound already on this channel (replay semantics, like the native channel reuse).
        if let Some(prev) = self.spatial_channels.borrow_mut().remove(channel) {
            prev.stop();
        }
        let dest = self.bus_input(bus).unwrap_or_else(|| self.master.clone());
        let sfx = self.play_sfx_to_opts(bytes, &dest, true);
        sfx.update_position(source, listener, max_dist);
        self.spatial_channels
            .borrow_mut()
            .insert(channel.to_string(), sfx);
    }

    /// Repositions the named positional `channel` — recomputes its distance-based volume + pan from
    /// `source`/`listener`/`max_dist` (call each frame to track a moving source). No-op for an
    /// unknown channel. Mirrors the native `AudioManager::update_position`.
    pub fn update_position(&self, channel: &str, source: Vec2, listener: Vec2, max_dist: f32) {
        if let Some(sfx) = self.spatial_channels.borrow().get(channel) {
            sfx.update_position(source, listener, max_dist);
        }
    }

    /// Stops and forgets whatever is playing on the named `channel` — a positional sound
    /// ([`play_at_on_channel`](Self::play_at_on_channel)) and/or a named tone
    /// ([`play_tone_on_channel`](Self::play_tone_on_channel)). No-op for an unknown channel. The web
    /// analogue of the native `AudioManager::stop`.
    #[allow(deprecated)] // ToneVoice::osc.stop() — same deprecated AudioScheduledSourceNode binding
    pub fn stop_channel(&self, channel: &str) {
        if let Some(sfx) = self.spatial_channels.borrow_mut().remove(channel) {
            sfx.stop();
        }
        if let Some(voice) = self.tone_channels.borrow_mut().remove(channel) {
            let _ = voice.osc.stop();
        }
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
        // Music starts only after an async decode, so the analyser must be looked up inside the
        // closure — by then `enable_analysis(MUSIC_CHANNEL)` may have been called (or not).
        let analysers = self.analysers.clone();
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
            // Meter the music when it is analyzed. Taken after the per-music gain (so a
            // crossfade ramp is included, matching the native fade) but before the master.
            if let Some(state) = analysers.borrow().get(crate::audio_analysis::MUSIC_CHANNEL) {
                let _ = gain.connect_with_audio_node(&state.node);
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
