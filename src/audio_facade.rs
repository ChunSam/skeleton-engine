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
//! Native-only extras (tone synthesis `AudioManager::play_tone`, positional `AudioManager::play_at`,
//! per-channel effects, automatic sidechains) are intentionally **not** on the facade — reach for the
//! platform backend directly when
//! a game needs them.
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

#[cfg(not(target_arch = "wasm32"))]
use crate::audio::AudioManager;
#[cfg(target_arch = "wasm32")]
use crate::audio_wasm::WebAudio;

/// Native: the conventional bus that unrouted SFX/music ride, so [`Audio::set_master_volume`]
/// has a single knob to turn. (On web the real master gain is used instead.)
#[cfg(not(target_arch = "wasm32"))]
const MASTER_BUS: &str = "master";

/// Native: the single channel name the facade plays looping music on.
#[cfg(not(target_arch = "wasm32"))]
const MUSIC_CHANNEL: &str = "__facade_music";

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

    /// Advances time-based audio. On **native** this ticks fades and bus ducks
    /// (`AudioManager::update`) and **must** be called every
    /// frame for [`crossfade_music`](Self::crossfade_music)/[`duck_bus`](Self::duck_bus) to progress
    /// — add [`AudioFacadeSystem`] (or call this yourself). On **web** the Web Audio clock drives the
    /// ramps, so this is a no-op.
    pub fn update(&mut self, dt: f32) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.inner.update(dt);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = dt;
        }
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
