//! Cross-platform audio facade — write **one** audio path for native **and** web.
//!
//! The engine ships two audio backends with deliberately different shapes:
//! `AudioManager` (native, rodio, channel+path based, `&mut self`, needs a per-frame `update(dt)`
//! tick) and `WebAudio` (wasm, Web Audio, bytes based, `&self`, audio-clock scheduled). A game that
//! targets both used to write every
//! audio call twice — a `#[cfg(not(target_arch = "wasm32"))]` arm using `AudioManager` and a
//! `#[cfg(target_arch = "wasm32")]` no-op (or `WebAudio`) stub. [`Audio`] collapses that to a single
//! cross-platform type so the game's audio logic carries **no `cfg` guards at all**.
//!
//! ## What it covers
//!
//! The intersection both backends support, keyed on **bytes** (the only cross-platform clip source —
//! `include_bytes!` works everywhere; wasm has no filesystem):
//!
//! - one-shot SFX: [`play_sfx`](Audio::play_sfx) / [`play_sfx_on_bus`](Audio::play_sfx_on_bus)
//! - synthesized tones (no clip bytes): [`play_tone`](Audio::play_tone) /
//!   [`play_tone_on_bus`](Audio::play_tone_on_bus)
//! - named, trackable tone channels: [`play_tone_on_channel`](Audio::play_tone_on_channel) +
//!   [`is_channel_playing`](Audio::is_channel_playing) + a low-pass filter
//!   ([`set_low_pass`](Audio::set_low_pass) / [`clear_low_pass`](Audio::clear_low_pass)) — for a
//!   sustained/re-triggered tone the game tracks (e.g. a procedural BGM it re-arms when it drains)
//! - 2D positional sound on a named channel: [`play_at_on_channel`](Audio::play_at_on_channel) +
//!   [`update_position`](Audio::update_position) + [`stop_channel`](Audio::stop_channel) — a looping
//!   sound whose distance-based volume + stereo pan track a moving source each frame
//! - looping music: [`play_music`](Audio::play_music) / [`crossfade_music`](Audio::crossfade_music) /
//!   [`stop_music`](Audio::stop_music)
//! - master volume: [`set_master_volume`](Audio::set_master_volume)
//! - named mixer buses: [`set_bus_volume`](Audio::set_bus_volume) / [`bus_volume`](Audio::bus_volume)
//! - ducking: [`duck_bus`](Audio::duck_bus) / [`release_bus`](Audio::release_bus) /
//!   [`bus_duck`](Audio::bus_duck)
//! - [`resume`](Audio::resume) (unlock audio after a user gesture on web; no-op on native)
//! - [`update`](Audio::update) (drives native fades/ducks; no-op on web) — call it from a system, or
//!   add the provided [`AudioFacadeSystem`].
//!
//! Native-only extras (an *untracked* positional one-shot — `AudioManager::play_at` / `WebAudio::play_at`
//! — per-channel effects beyond the low-pass — pitch shift, attack/release envelopes — and automatic
//! sidechains) are intentionally **not** on the facade — reach for the platform backend directly when a
//! game needs them. (Tracked positional sound *is* covered, via [`play_at_on_channel`](Audio::play_at_on_channel).)
//!
//! ## Master-volume nuance (native)
//!
//! On web, named buses nest under the master gain, so [`set_master_volume`](Audio::set_master_volume)
//! scales every sound including bus-routed ones. The native backend's buses do **not** nest: the
//! facade routes unrouted [`play_sfx`](Audio::play_sfx)/[`play_music`](Audio::play_music) through a
//! conventional `"master"` bus that [`set_master_volume`](Audio::set_master_volume) controls, but a
//! sound sent to a *named* bus via [`play_sfx_on_bus`](Audio::play_sfx_on_bus) rides that bus alone
//! and is **not** affected by the master volume on native. Control such sounds with
//! [`set_bus_volume`](Audio::set_bus_volume) instead. (The difference only shows when you mix named
//! buses *and* the master control on native.)
//!
//! ## Usage
//!
//! ```rust,no_run
//! # use engine::{App, Audio, AudioFacadeSystem};
//! static JUMP: &[u8] = b""; // include_bytes!("../assets/jump.wav") — works on native AND wasm
//! let mut app = App::new();
//! if let Some(audio) = Audio::new() {
//!     app.world.insert_resource(audio);
//! }
//! app.add_system(AudioFacadeSystem); // ticks fades/ducks on native; no-op on web
//! // later, from a system — no cfg guards:
//! // if let Some(a) = world.resource_mut::<Audio>() { a.play_sfx(JUMP); }
//! ```

use glam::Vec2;

#[cfg(not(target_arch = "wasm32"))]
use crate::audio::{AudioEffect, AudioManager};
#[cfg(target_arch = "wasm32")]
use crate::audio_wasm::WebAudio;

/// Native: the conventional bus that unrouted SFX/music ride, so [`Audio::set_master_volume`]
/// has a single knob to turn. (On web the real master gain is used instead.)
#[cfg(not(target_arch = "wasm32"))]
const MASTER_BUS: &str = "master";

/// Native: the single channel name the facade plays looping music on. Defined in
/// [`audio_analysis`](crate::audio_analysis) because the wasm backend needs the same string as its
/// music-analyser key — one definition, so the two backends cannot disagree.
#[cfg(not(target_arch = "wasm32"))]
use crate::audio_analysis::MUSIC_CHANNEL;

/// Native: number of round-robin SFX voices. A fixed ring bounds the native sink count — a new
/// one-shot reuses (and cuts) the oldest voice when the ring wraps, so fire-and-forget
/// [`Audio::play_sfx`] never leaks sinks. 16 simultaneous one-shots is ample for a 2D game.
#[cfg(not(target_arch = "wasm32"))]
const SFX_VOICES: u64 = 16;

/// Round-robin SFX voice channel name for the native backend: `__facade_sfx_{seq % voices}`.
/// Pure (no device) so it is unit-testable headlessly.
#[cfg(not(target_arch = "wasm32"))]
fn sfx_voice_channel(seq: u64, voices: u64) -> String {
    format!("__facade_sfx_{}", seq % voices)
}

/// A cross-platform audio player — one API over the native `AudioManager` and the web `WebAudio`
/// backends. Insert it as a `World` resource and drive it
/// from systems; clips are passed as encoded `bytes` (e.g. from `include_bytes!`). See the
/// [module docs](crate::audio_facade) for coverage and the native master-volume nuance.
pub struct Audio {
    #[cfg(not(target_arch = "wasm32"))]
    inner: AudioManager,
    /// Monotonic counter selecting the next round-robin SFX voice (see [`sfx_voice_channel`]).
    #[cfg(not(target_arch = "wasm32"))]
    sfx_seq: u64,
    #[cfg(target_arch = "wasm32")]
    inner: WebAudio,
}

impl Audio {
    /// Creates the audio output, or `None` if the platform refuses (no device on native, no
    /// `AudioContext` on web) — the game then runs silently. Mirrors `AudioManager::new` /
    /// `WebAudio::new`.
    pub fn new() -> Option<Self> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            AudioManager::new().map(|inner| Self { inner, sfx_seq: 0 })
        }
        #[cfg(target_arch = "wasm32")]
        {
            WebAudio::new().map(|inner| Self { inner })
        }
    }

    /// Picks the next round-robin SFX voice channel and advances the counter.
    #[cfg(not(target_arch = "wasm32"))]
    fn next_sfx_channel(&mut self) -> String {
        let ch = sfx_voice_channel(self.sfx_seq, SFX_VOICES);
        self.sfx_seq = self.sfx_seq.wrapping_add(1);
        ch
    }

    /// Plays `bytes` as a fire-and-forget one-shot sound effect (no looping). On native it rides
    /// the master bus (so [`set_master_volume`](Self::set_master_volume) scales it); on web it plays
    /// through the master gain. Browsers gate audio behind a user gesture — see [`resume`](Self::resume).
    pub fn play_sfx(&mut self, bytes: &[u8]) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let channel = self.next_sfx_channel();
            self.inner.assign_bus(&channel, MASTER_BUS);
            self.inner.play_bytes(&channel, bytes, false);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.play(bytes);
        }
    }

    /// Like [`play_sfx`](Self::play_sfx), but routes the one-shot through the named mixer `bus` so
    /// [`set_bus_volume`](Self::set_bus_volume) / [`duck_bus`](Self::duck_bus) scale it as a group.
    /// On native a bus-routed sound is **not** affected by [`set_master_volume`](Self::set_master_volume)
    /// (see the [module docs](crate::audio_facade) — native buses don't nest); on web it is.
    pub fn play_sfx_on_bus(&mut self, bytes: &[u8], bus: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let channel = self.next_sfx_channel();
            self.inner.assign_bus(&channel, bus);
            self.inner.play_bytes(&channel, bytes, false);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.play_sfx_on_bus(bytes, bus);
        }
    }

    /// Plays a synthesized sine-wave tone — `freq` Hz for `dur` seconds at amplitude `vol` — as a
    /// fire-and-forget one-shot, no clip bytes needed. Handy for retro blips/beeps without bundling
    /// audio files. Rides the master bus on native / the master gain on web (so
    /// [`set_master_volume`](Self::set_master_volume) scales it); route it through a named bus with
    /// [`play_tone_on_bus`](Self::play_tone_on_bus) instead. Browsers gate audio behind a user
    /// gesture — see [`resume`](Self::resume).
    ///
    /// On native this shares the same round-robin voice ring as [`play_sfx`](Self::play_sfx), so
    /// consecutive tones overlap (rather than cutting each other off) until the ring wraps.
    pub fn play_tone(&mut self, freq: f32, dur: f32, vol: f32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let channel = self.next_sfx_channel();
            self.inner.assign_bus(&channel, MASTER_BUS);
            self.inner.play_tone(&channel, freq, dur, vol);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.play_tone(freq, dur, vol);
        }
    }

    /// Like [`play_tone`](Self::play_tone), but routes the tone through the named mixer `bus` so
    /// [`set_bus_volume`](Self::set_bus_volume) / [`duck_bus`](Self::duck_bus) scale it as a group.
    /// On native a bus-routed tone is **not** affected by [`set_master_volume`](Self::set_master_volume)
    /// (see the [module docs](crate::audio_facade) — native buses don't nest); on web it is.
    ///
    /// Like [`play_tone`](Self::play_tone) this rides the shared round-robin voice ring, so
    /// consecutive tones overlap — and so it **cannot be metered**
    /// ([`enable_analysis`](Self::enable_analysis) needs a stable name). For a tone that overlaps
    /// *and* is metered use [`play_tone_metered`](Self::play_tone_metered); for one that is
    /// metered, stoppable and filterable but cuts itself on replay use
    /// [`play_tone_on_channel`](Self::play_tone_on_channel).
    pub fn play_tone_on_bus(&mut self, freq: f32, dur: f32, vol: f32, bus: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let channel = self.next_sfx_channel();
            self.inner.assign_bus(&channel, bus);
            self.inner.play_tone(&channel, freq, dur, vol);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.play_tone_on_bus(freq, dur, vol, bus);
        }
    }

    /// Plays a synthesized tone on a caller-named **channel** routed through the named mixer `bus`.
    /// Unlike the fire-and-forget [`play_tone`](Self::play_tone) (which round-robins anonymous
    /// voices), a named channel is stable: query it with [`is_channel_playing`](Self::is_channel_playing),
    /// filter it with [`set_low_pass`](Self::set_low_pass) / [`clear_low_pass`](Self::clear_low_pass),
    /// and a replay on the same channel **replaces** (cuts) the previous tone. Use it for a sustained
    /// or re-triggered tone the game tracks — e.g. a procedural BGM the game re-arms when it drains.
    pub fn play_tone_on_channel(
        &mut self,
        channel: &str,
        freq: f32,
        dur: f32,
        vol: f32,
        bus: &str,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.assign_bus(channel, bus);
            self.inner.play_tone(channel, freq, dur, vol);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner
                .play_tone_on_channel(channel, freq, dur, vol, bus);
        }
    }

    /// Plays a synthesized tone as a **metered one-shot**: it overlaps its own previous plays, and
    /// `levels(meter)` reports it. Routed through the named mixer `bus`.
    ///
    /// This exists because those two properties used to be mutually exclusive, and a game had to
    /// pick. [`play_tone`](Self::play_tone) / [`play_tone_on_bus`](Self::play_tone_on_bus) overlap
    /// but ride anonymous voices with no name to meter;
    /// [`play_tone_on_channel`](Self::play_tone_on_channel) is meterable but **cuts** the sound
    /// already on its channel, so a rapid-fire sound silences itself. Enable the meter for `meter`
    /// with [`enable_analysis`](Self::enable_analysis) exactly as for a named channel:
    ///
    /// ```rust,no_run
    /// # use engine::Audio;
    /// # let mut audio = Audio::new().unwrap();
    /// audio.enable_analysis("hit");
    /// audio.play_tone_metered("hit", 150.0, 0.14, 0.4, "sfx");   // ×3 in one frame …
    /// audio.play_tone_metered("hit", 150.0, 0.14, 0.4, "sfx");   // … none of them cutting
    /// audio.play_tone_metered("hit", 150.0, 0.14, 0.4, "sfx");   // … and all three metered
    /// ```
    ///
    /// # What `levels` reports
    ///
    /// The **sum** of the voices currently sounding, clamped to full scale — three overlapping hits
    /// read louder than one, which is the point. The web backend gives the same answer structurally
    /// (every voice feeds one `AnalyserNode`, and Web Audio mixes them), so the two platforms agree
    /// on meaning; they are equivalent rather than bit-identical, the same caveat
    /// [`bands`](Self::bands) carries. See `sum_levels` in `audio_analysis` for the full rationale.
    ///
    /// # Limits worth knowing
    ///
    /// - **`meter` is a meter name, not a channel name.** It is not addressable by
    ///   [`stop_channel`](Self::stop_channel), [`set_low_pass`](Self::set_low_pass) or
    ///   [`is_channel_playing`](Self::is_channel_playing) — a one-shot you can cut or filter is a
    ///   named channel, which is what [`play_tone_on_channel`](Self::play_tone_on_channel) is for.
    /// - **Eight voices per name**, then it wraps and reuses its oldest — a bound, not a leak.
    /// - **[`bands`](Self::bands) reports zeros for it.** A spectrum costs an FFT per voice per
    ///   window, and its use case (a soundtrack) is not a one-shot.
    pub fn play_tone_metered(&mut self, meter: &str, freq: f32, dur: f32, vol: f32, bus: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.play_tone_poly(meter, freq, dur, vol, bus);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.play_tone_metered(meter, freq, dur, vol, bus);
        }
    }

    /// Whether the named tone `channel` still has audio playing — re-arm a sustained
    /// [`play_tone_on_channel`](Self::play_tone_on_channel) when this turns `false`. Mirrors the
    /// native [`AudioManager::is_playing`](crate::audio::AudioManager::is_playing) / a tracked Web
    /// Audio oscillator. Returns `false` for an unknown channel.
    pub fn is_channel_playing(&self, channel: &str) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.is_playing(channel)
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.is_channel_playing(channel)
        }
    }

    /// Sets a low-pass filter (cutoff Hz) on the named tone `channel`, applied to the **next**
    /// [`play_tone_on_channel`](Self::play_tone_on_channel) on that channel (toggle, then replay to
    /// hear it). Native: an [`AudioEffect`] with `low_pass_hz`; web: a
    /// `BiquadFilterNode` — same "applied on next play" semantics on both. Affects only named tone
    /// channels (not the fire-and-forget [`play_sfx`](Self::play_sfx)/[`play_tone`](Self::play_tone)).
    pub fn set_low_pass(&mut self, channel: &str, cutoff_hz: u32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.set_effect(
                channel,
                AudioEffect {
                    low_pass_hz: Some(cutoff_hz),
                    ..Default::default()
                },
            );
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.set_low_pass(channel, cutoff_hz);
        }
    }

    /// Removes the low-pass filter from the named tone `channel` (applied on the next play). Inverse
    /// of [`set_low_pass`](Self::set_low_pass).
    pub fn clear_low_pass(&mut self, channel: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.clear_effect(channel);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.clear_low_pass(channel);
        }
    }

    /// Plays `bytes` as a **looping positional** sound on a caller-named `channel` routed through the
    /// named mixer `bus`, tracked so [`update_position`](Self::update_position) /
    /// [`stop_channel`](Self::stop_channel) can address it by name. The distance-based volume (silent
    /// at `max_dist`) and stereo pan are applied immediately from `source`/`listener`; call
    /// [`update_position`](Self::update_position) every frame to follow a moving source. Replacing a
    /// channel stops its previous sound first. (Positional one-shots you don't track aren't covered —
    /// reach for the platform backend's `play_at` directly.)
    pub fn play_at_on_channel(
        &mut self,
        channel: &str,
        bytes: &[u8],
        source: Vec2,
        listener: Vec2,
        max_dist: f32,
        bus: &str,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.assign_bus(channel, bus);
            self.inner
                .play_bytes_at(channel, bytes, true, source, listener, max_dist);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner
                .play_at_on_channel(channel, bytes, source, listener, max_dist, bus);
        }
    }

    /// Repositions a named positional `channel` — recomputes its distance-based volume + stereo pan
    /// from `source`/`listener`/`max_dist`. Call every frame to track a moving source. No-op for an
    /// unknown channel. Both backends expose `update_position`, so this needs no `cfg` split.
    pub fn update_position(&mut self, channel: &str, source: Vec2, listener: Vec2, max_dist: f32) {
        self.inner
            .update_position(channel, source, listener, max_dist);
    }

    /// Stops and forgets whatever is playing on the named `channel` — a positional sound
    /// ([`play_at_on_channel`](Self::play_at_on_channel)) and/or a named tone
    /// ([`play_tone_on_channel`](Self::play_tone_on_channel)). No-op for an unknown channel.
    pub fn stop_channel(&mut self, channel: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.stop(channel);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.stop_channel(channel);
        }
    }

    /// Plays `bytes` as looping music on the single music channel, replacing any current music.
    /// Rides the master bus on native / the master gain on web. Stop with [`stop_music`](Self::stop_music)
    /// or transition with [`crossfade_music`](Self::crossfade_music).
    pub fn play_music(&mut self, bytes: &[u8]) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.assign_bus(MUSIC_CHANNEL, MASTER_BUS);
            self.inner.play_bytes(MUSIC_CHANNEL, bytes, true);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.play_music(bytes);
        }
    }

    /// Crossfades from the current music track to `bytes` over `dur` seconds (the old track fades
    /// out as the new one fades in; a plain fade-in if nothing is playing).
    pub fn crossfade_music(&mut self, bytes: &[u8], dur: f32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.assign_bus(MUSIC_CHANNEL, MASTER_BUS);
            self.inner.crossfade_bytes(MUSIC_CHANNEL, bytes, true, dur);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.crossfade_music(bytes, dur);
        }
    }

    /// Stops the looping music, if any.
    pub fn stop_music(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.stop(MUSIC_CHANNEL);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.stop_music();
        }
    }

    /// Sets the overall (master) volume, clamped to `0.0..=1.0`. On web this is the master gain; on
    /// native it is the `"master"` bus that unrouted SFX/music ride (named-bus sounds are not scaled
    /// by it on native — see the [module docs](crate::audio_facade)).
    pub fn set_master_volume(&mut self, v: f32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.set_bus_volume(MASTER_BUS, v);
        }
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.set_volume(v);
        }
    }

    /// Sets the volume of a named mixer `bus`, clamped to `0.0..=1.0`, scaling every sound routed to
    /// it. The bus is created on first reference. Same name on both backends.
    pub fn set_bus_volume(&mut self, bus: &str, v: f32) {
        self.inner.set_bus_volume(bus, v);
    }

    /// The current volume of a named `bus` (`1.0` if it doesn't exist yet).
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.inner.bus_volume(bus)
    }

    /// Ducks a named `bus` — ramps its level toward `gain` (clamped `0.0..=1.0`) over `attack_secs`,
    /// on top of its volume. Use it to dip music/SFX while a cue plays, then [`release_bus`](Self::release_bus).
    pub fn duck_bus(&mut self, bus: &str, gain: f32, attack_secs: f32) {
        self.inner.duck_bus(bus, gain, attack_secs);
    }

    /// Releases a duck — ramps the `bus` back to its full volume over `release_secs`. Inverse of
    /// [`duck_bus`](Self::duck_bus).
    pub fn release_bus(&mut self, bus: &str, release_secs: f32) {
        self.inner.release_bus(bus, release_secs);
    }

    /// The current duck multiplier for a `bus` (`1.0` = no duck / unknown bus).
    pub fn bus_duck(&self, bus: &str) -> f32 {
        self.inner.bus_duck(bus)
    }

    /// Ensures the audio output is running. On **web** this resumes the `AudioContext`, which
    /// browsers start suspended until a user gesture — call it once from an input handler (a key
    /// press / click) so subsequent playback is audible. On **native** there is nothing to unlock,
    /// so this is a no-op.
    pub fn resume(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.inner.resume();
        }
    }

    /// Advances time-based audio. On **native** this ticks fades, bus ducks and level meters
    /// (`AudioManager::update`) and **must** be called every
    /// frame for [`crossfade_music`](Self::crossfade_music)/[`duck_bus`](Self::duck_bus) to progress
    /// — add [`AudioFacadeSystem`] (or call this yourself). On **web** the Web Audio clock drives the
    /// ramps, so only the level meters need ticking here.
    pub fn update(&mut self, dt: f32) {
        self.inner.update(dt);
    }

    // ── Level analysis (audio-reactive hooks) ─────────────────────────────────

    /// The channel [`play_music`](Self::play_music) uses — pass it to
    /// [`enable_analysis`](Self::enable_analysis) to meter the music track.
    ///
    /// ```rust,no_run
    /// # use engine::Audio;
    /// # let mut audio = Audio::new().unwrap();
    /// audio.enable_analysis(Audio::MUSIC_CHANNEL);
    /// ```
    pub const MUSIC_CHANNEL: &'static str = crate::audio_analysis::MUSIC_CHANNEL;

    /// Starts measuring `channel`'s loudness so [`levels`](Self::levels) reports it — the hook a
    /// music visualizer, a beat-reactive effect or a talking-head mouth flap reads.
    ///
    /// **Enable during setup, before the first play on that channel.** The meter is wired into the
    /// sound when it starts, and a sound already playing is not rewired (the same "applies on the
    /// next play" rule as the native `set_effect`). Enabling an already-analyzed channel keeps its
    /// current reading.
    ///
    /// Analysis is **opt-in and costs nothing until asked for**: an unanalyzed channel builds
    /// exactly the audio graph it did before.
    ///
    /// # Which channels can be metered
    ///
    /// Any channel with a stable name: [`MUSIC_CHANNEL`](Self::MUSIC_CHANNEL), and the named
    /// channels from [`play_tone_on_channel`](Self::play_tone_on_channel) /
    /// [`play_at_on_channel`](Self::play_at_on_channel) — plus the meter name of a metered one-shot
    /// ([`play_tone_metered`](Self::play_tone_metered)).
    ///
    /// **Not** the fire-and-forget one-shots — [`play_sfx`](Self::play_sfx),
    /// [`play_sfx_on_bus`](Self::play_sfx_on_bus), [`play_tone`](Self::play_tone) and
    /// [`play_tone_on_bus`](Self::play_tone_on_bus) *all* round-robin the same ring of 16 anonymous
    /// voices on native, so none of them has a name to address.
    ///
    /// Moving a *tone* onto a named channel to meter it used to be a real trade-off rather than a
    /// free rename — a replay on a named channel **cuts** the sound already there, where the
    /// anonymous ring lets consecutive one-shots overlap.
    /// [`play_tone_metered`](Self::play_tone_metered) removes that choice: it overlaps *and* is
    /// metered. Reach for it when a sound repeats faster than it decays; a named channel is still
    /// the right answer when you also need to stop, filter or query the sound. Clips
    /// ([`play_sfx`](Self::play_sfx)) still face the original trade-off.
    pub fn enable_analysis(&mut self, channel: &str) {
        self.inner.enable_analysis(channel);
    }

    /// Stops measuring `channel`. [`levels`](Self::levels) reports
    /// [`SILENT`](crate::AudioLevels::SILENT) immediately afterwards.
    pub fn disable_analysis(&mut self, channel: &str) {
        self.inner.disable_analysis(channel);
    }

    /// Returns whether `channel` is being analyzed.
    pub fn is_analysis_enabled(&self, channel: &str) -> bool {
        self.inner.is_analysis_enabled(channel)
    }

    /// Returns `channel`'s current smoothed loudness — see [`AudioLevels`](crate::AudioLevels) for
    /// exactly what is measured (the **pre-volume** envelope, so a visual keeps reacting at
    /// volume 0).
    ///
    /// Reports [`SILENT`](crate::AudioLevels::SILENT) for a channel that is not analyzed, never
    /// played, or has stopped. Requires [`update`](Self::update) each frame, which
    /// [`AudioFacadeSystem`] does.
    ///
    /// ```rust,no_run
    /// # use engine::Audio;
    /// # let mut audio = Audio::new().unwrap();
    /// # let sprite_scale = &mut 1.0_f32;
    /// let level = audio.levels(Audio::MUSIC_CHANNEL);
    /// *sprite_scale = 1.0 + level.rms * 0.5;   // pulse with the music
    /// if level.peak > 0.8 {
    ///     // a transient just landed — shake, flash, spawn
    /// }
    /// ```
    pub fn levels(&self, channel: &str) -> crate::AudioLevels {
        self.inner.levels(channel)
    }

    /// Starts measuring `channel`'s **frequency spectrum** as well as its levels, so
    /// [`bands`](Self::bands) reports it. Implies [`enable_analysis`](Self::enable_analysis).
    ///
    /// Kept separate from `enable_analysis` because a spectrum costs an FFT per window on native,
    /// while most uses — a pulse, a kick flash — only need [`levels`](Self::levels). A channel
    /// with levels-only analysis runs no transform at all.
    pub fn enable_spectrum(&mut self, channel: &str) {
        self.inner.enable_spectrum(channel);
    }

    /// Stops reporting a spectrum for `channel`, leaving its levels running.
    pub fn disable_spectrum(&mut self, channel: &str) {
        self.inner.disable_spectrum(channel);
    }

    /// Writes `channel`'s current smoothed spectrum into `out` — `out.len()` log-spaced bands from
    /// low frequency to high, each `0.0..=1.0`. The classic spectrum-analyzer bar display.
    ///
    /// **You choose the band count.** `out.len()` is resampled from the backend's internal
    /// resolution, so a 8-bar HUD and a 64-bar visualizer both work and neither platform's FFT
    /// size leaks into your code. Fills zeros for a channel with no spectrum enabled or one that
    /// is silent.
    ///
    /// Bands are spaced logarithmically and normalized over a decibel window, because pitch and
    /// loudness are both perceived logarithmically — linear bands and linear magnitudes both give
    /// a display that looks dead.
    ///
    /// ⚠️ **Cross-platform values are equivalent, not identical.** Native runs its own FFT; wasm
    /// reads a Web Audio `AnalyserNode`. Both use the same dB window and the same band edges and
    /// track each other closely, but they are not bit-for-bit equal — drive visuals with them,
    /// not equality checks.
    ///
    /// # Resolution
    ///
    /// The transform is 1024 points, so at 44.1 kHz each FFT bin spans about 43 Hz. Because the
    /// bands are log-spaced, the lowest few cover only one or two bins and therefore **move
    /// together** — a bass note lights several bottom bars at once. That is the frequency
    /// resolution of the transform, not a display artifact, and asking for more bands does not
    /// create detail that is not there.
    ///
    /// ```rust,no_run
    /// # use engine::Audio;
    /// # let mut audio = Audio::new().unwrap();
    /// audio.enable_spectrum(Audio::MUSIC_CHANNEL);
    /// let mut bars = [0.0f32; 24];
    /// audio.bands(Audio::MUSIC_CHANNEL, &mut bars);
    /// for (i, b) in bars.iter().enumerate() {
    ///     let _height = b * 100.0;   // draw bar i
    ///     let _ = i;
    /// }
    /// ```
    pub fn bands(&self, channel: &str, out: &mut [f32]) {
        self.inner.bands(channel, out);
    }

    /// Sets how long a meter takes to fall, in seconds (default
    /// [`DEFAULT_ANALYSIS_SMOOTHING`](crate::DEFAULT_ANALYSIS_SMOOTHING)).
    ///
    /// A rise is always instant, so a transient is never delayed; this controls only the decay,
    /// which is what keeps a meter from strobing. `0.0` disables smoothing. Applies to every
    /// analyzed channel, on both backends.
    pub fn set_analysis_smoothing(&mut self, release_secs: f32) {
        self.inner.set_analysis_smoothing(release_secs);
    }

    /// Returns the meter release time in seconds.
    pub fn analysis_smoothing(&self) -> f32 {
        self.inner.analysis_smoothing()
    }
}

/// Built-in system that ticks the [`Audio`] resource's [`update`](Audio::update) every frame, so
/// native fades and bus ducks progress (the cross-platform analogue of the native-only
/// [`AudioSystem`](crate::AudioSystem)). Does nothing if no [`Audio`] resource is present, and is a
/// no-op tick on web (where the audio clock drives ramps). Add it once with `app.add_system(..)`.
///
/// ```rust,no_run
/// # use engine::{App, Audio, AudioFacadeSystem};
/// let mut app = App::new();
/// if let Some(audio) = Audio::new() {
///     app.world.insert_resource(audio);
/// }
/// app.add_system(AudioFacadeSystem);
/// ```
#[derive(Default)]
pub struct AudioFacadeSystem;

impl AudioFacadeSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::audio_facade";
}

impl crate::ecs::System for AudioFacadeSystem {
    fn run(&mut self, world: &mut crate::ecs::World, dt: f32) {
        if let Some(audio) = world.resource_mut::<Audio>() {
            audio.update(dt);
        }
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "wasm32"))]
mod tests {
    use super::*;

    #[test]
    fn sfx_voices_wrap_round_robin() {
        // The ring cycles through exactly `voices` distinct channel names and wraps.
        assert_eq!(sfx_voice_channel(0, 16), "__facade_sfx_0");
        assert_eq!(sfx_voice_channel(15, 16), "__facade_sfx_15");
        assert_eq!(sfx_voice_channel(16, 16), "__facade_sfx_0"); // wraps to reuse voice 0
        assert_eq!(sfx_voice_channel(17, 16), "__facade_sfx_1");
        // Distinct names within one ring revolution → independent sinks.
        let names: std::collections::HashSet<String> = (0..SFX_VOICES)
            .map(|s| sfx_voice_channel(s, SFX_VOICES))
            .collect();
        assert_eq!(names.len(), SFX_VOICES as usize);
    }

    #[test]
    fn next_sfx_channel_advances_and_wraps() {
        // Without an audio device `Audio::new()` may return None in headless CI, so drive the
        // counter logic directly on a hand-built instance (no device touched).
        let mut seq = 0u64;
        let mut chans = Vec::new();
        for _ in 0..(SFX_VOICES + 2) {
            chans.push(sfx_voice_channel(seq, SFX_VOICES));
            seq = seq.wrapping_add(1);
        }
        assert_eq!(chans[0], "__facade_sfx_0");
        assert_eq!(chans[SFX_VOICES as usize], "__facade_sfx_0"); // wrapped
        assert_eq!(chans[SFX_VOICES as usize + 1], "__facade_sfx_1");
    }
}
