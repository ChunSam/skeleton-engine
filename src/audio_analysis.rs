//! Shared audio-level analysis types and smoothing policy, compiled on all targets.
//!
//! Both `AudioManager` (native, via a `Source` tap on the playback thread) and `WebAudio` (wasm,
//! via a Web Audio `AnalyserNode`) measure a channel's loudness and hand the raw reading to the
//! *same* smoothing function here — so the two builds present the same meter behaviour and cannot
//! drift. Same reasoning as the shared `audio_spatial` math.
//!
//! The measurement point is deliberately **pre-volume**: it is the sound's own envelope, taken
//! after per-sound effects but before channel volume, bus volume, ducking and the master gain.
//! A visualizer therefore keeps reacting when the player turns the volume down or mutes — which is
//! what a beat-driven game wants, since a note that stops landing at volume 0 is a bug, not a
//! feature. See [`AudioLevels`] for the full statement.

/// The channel name the cross-platform [`Audio`](crate::Audio) facade plays music on, and
/// therefore the name to pass to `enable_analysis` to meter it.
///
/// Defined here, un-gated, because **both** backends need the same string: on native it is a real
/// `AudioManager` channel, and on wasm it is the key the music analyser is stored under. One
/// definition means the two cannot disagree about what "the music channel" is. Re-exported as
/// [`Audio::MUSIC_CHANNEL`](crate::Audio::MUSIC_CHANNEL).
pub const MUSIC_CHANNEL: &str = "__facade_music";

/// Default release time constant for the meter smoothing, in seconds.
///
/// Used unless a game calls `set_analysis_smoothing`. Chosen so a meter falls visibly but without
/// flicker at 60 fps; raise it for a lazier decay, set `0.0` for none.
pub const DEFAULT_ANALYSIS_SMOOTHING: f32 = 0.15;

/// A channel's measured loudness, as reported by `levels()`.
///
/// Both fields are `0.0..=1.0` and are **smoothed** — they rise instantly and fall over the
/// configured release time, which is what makes a meter readable rather
/// than strobing.
///
/// # What is measured
///
/// The **pre-volume** signal: after the sound's own effects (pitch, low-pass, fade-in) but
/// **before** channel volume, bus volume, ducking and the master gain. Consequences worth knowing:
///
/// - Turning the volume down, ducking a bus, or muting entirely does **not** change these values.
///   A beat-reactive visual keeps working at volume 0.
/// - They therefore describe *the sound*, not *what the player hears*. If you need the latter,
///   scale by your own volume values.
///
/// # Silence
///
/// A channel that is not playing, was never played, or has no analysis enabled reads
/// [`SILENT`](Self::SILENT) — all zeros. A channel that *stops* decays to silence over the release
/// time rather than freezing at its last value, so a bar never sticks at full height.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AudioLevels {
    /// Root-mean-square level — perceived loudness. The value to drive a pulse, a bar or a scale
    /// factor with; it is steady enough to look good directly.
    pub rms: f32,
    /// Peak absolute sample in the same window — transient level. Rises faster and further than
    /// [`rms`](Self::rms) on a drum hit, so it is the better input for beat/kick detection.
    pub peak: f32,
}

impl AudioLevels {
    /// A silent reading — what an unanalyzed, stopped or never-played channel reports.
    pub const SILENT: Self = Self {
        rms: 0.0,
        peak: 0.0,
    };

    /// Returns `true` when both levels are effectively zero (below `1e-4`).
    pub fn is_silent(&self) -> bool {
        self.rms < 1e-4 && self.peak < 1e-4
    }
}

/// Advances one smoothed meter value toward `target`: **instant attack, timed release**.
///
/// A rise is applied immediately, so a transient is never missed or visually delayed — the thing
/// that makes a hit feel connected to its sound. A fall eases toward `target` with time constant
/// `release_secs`, which is what stops a meter from strobing at frame rate.
///
/// `release_secs <= 0.0` disables smoothing entirely (the value tracks `target` exactly).
/// Both backends call this, so the meter behaves identically on native and web.
pub(crate) fn smooth_toward(current: f32, target: f32, release_secs: f32, dt: f32) -> f32 {
    // Instant attack: a transient must not be smeared.
    if target >= current {
        return target;
    }
    if release_secs <= 0.0 || dt <= 0.0 {
        return target;
    }
    let alpha = (dt / release_secs).clamp(0.0, 1.0);
    current + (target - current) * alpha
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rise_is_instant_so_a_transient_is_never_delayed() {
        // The whole point of instant attack: a kick that lands this frame reads this frame.
        assert_eq!(smooth_toward(0.0, 1.0, 0.15, 1.0 / 60.0), 1.0);
        assert_eq!(smooth_toward(0.3, 0.9, 10.0, 1.0 / 60.0), 0.9);
    }

    #[test]
    fn a_fall_eases_instead_of_snapping() {
        let v = smooth_toward(1.0, 0.0, 0.15, 1.0 / 60.0);
        assert!(v > 0.0, "a fall must not snap to the target in one frame");
        assert!(v < 1.0, "a fall must actually make progress");
        // dt/release = (1/60)/0.15 ≈ 0.111 of the way down.
        assert!(
            (v - (1.0 - 0.1111)).abs() < 0.01,
            "unexpected decay rate: {v}"
        );
    }

    #[test]
    fn a_fall_converges_to_silence_rather_than_freezing() {
        // The "frozen bar" bug: a stopped channel must decay to zero, not stick at its last value.
        let mut v = 1.0;
        for _ in 0..600 {
            v = smooth_toward(v, 0.0, 0.15, 1.0 / 60.0);
        }
        assert!(v < 1e-4, "expected decay to silence, got {v}");
    }

    #[test]
    fn zero_release_disables_smoothing_in_both_directions() {
        assert_eq!(smooth_toward(1.0, 0.2, 0.0, 1.0 / 60.0), 0.2);
        assert_eq!(smooth_toward(0.2, 1.0, 0.0, 1.0 / 60.0), 1.0);
    }

    #[test]
    fn a_huge_dt_cannot_overshoot_past_the_target() {
        // A stalled frame must not drive the meter negative.
        let v = smooth_toward(1.0, 0.0, 0.15, 10.0);
        assert_eq!(v, 0.0);
        assert!(v >= 0.0);
    }

    #[test]
    fn silent_is_silent_and_a_real_reading_is_not() {
        assert!(AudioLevels::SILENT.is_silent());
        assert!(AudioLevels::default().is_silent());
        assert!(!AudioLevels {
            rms: 0.5,
            peak: 0.8
        }
        .is_silent());
    }
}
