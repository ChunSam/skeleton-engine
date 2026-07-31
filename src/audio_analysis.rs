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

/// Combines the simultaneously-sounding voices of one **metered one-shot** name into a single
/// reading: their **sum**, clamped to full scale.
///
/// # Why sum, and why this lives here
///
/// A metered one-shot (`Audio::play_tone_metered`) lets the same name sound several times at once
/// instead of cutting itself off, so `levels()` has to answer "how loud is that name *right now*"
/// from more than one voice. Sum is the honest answer to that question — three overlapping hits
/// really are louder than one, and a game driving a shake or a flash wants that to show.
///
/// It is also the answer the web backend gives whether we like it or not: every voice connects to
/// the same `AnalyserNode`, and Web Audio mixes multiple inputs into one node. Choosing max on
/// native would make the two backends disagree about *meaning*, not just about rounding. So the
/// policy is stated once, here, un-gated — the same reason [`MUSIC_CHANNEL`] is.
///
/// Native calls this to combine its per-voice taps. Wasm does not: the browser has already summed
/// the voices in the graph before the analyser sees them, so it reads one mixed signal. The two are
/// **equivalent, not identical** — native sums per-voice measurements while the browser measures
/// the summed signal, which differ when voices interfere. Drive visuals with them, not checksums.
/// (Same caveat `bands()` carries.)
///
/// Clamping matters: three voices at 0.6 sum to 1.8, and a meter reading past full scale would
/// push a bar off screen. The per-voice taps already clamp individually for the same reason.
pub fn sum_levels(voices: impl IntoIterator<Item = AudioLevels>) -> AudioLevels {
    let mut total = AudioLevels::SILENT;
    for v in voices {
        total.rms += v.rms;
        total.peak += v.peak;
    }
    AudioLevels {
        rms: total.rms.min(1.0),
        peak: total.peak.min(1.0),
    }
}

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

// ─── Spectrum ─────────────────────────────────────────────────────────────────

/// Number of log-spaced frequency bands each backend publishes internally.
///
/// `bands()` resamples this to whatever length the caller asks for, which is what lets the two
/// backends use different FFT sizes without the API leaking either one.
pub(crate) const SPECTRUM_BANDS: usize = 32;

/// Lower edge of the decibel window mapped to `0.0`.
///
/// Deliberately **Web Audio's `AnalyserNode` default** (`minDecibels = -100`). The wasm backend
/// gets its normalization from the browser, so the native side matching these two constants is the
/// only reason a band value means the same thing on both platforms.
pub(crate) const MIN_DB: f32 = -100.0;
/// Upper edge of the decibel window mapped to `1.0` — Web Audio's `maxDecibels = -30` default.
pub(crate) const MAX_DB: f32 = -30.0;

/// Maps a linear FFT magnitude to `0.0..=1.0` across the [`MIN_DB`]–[`MAX_DB`] window.
///
/// Decibels rather than raw magnitude because hearing is logarithmic: a linear bar spends almost
/// all its travel in the top 10% of the range and looks dead everywhere else.
///
/// **Native only** — this is the one part of the spectrum path the two backends genuinely do not
/// share, because a Web Audio `AnalyserNode` performs the same conversion itself and hands back
/// bytes already normalized over the window. The *constants* are still shared, which is what makes
/// the two results comparable.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn normalized_db(magnitude: f32) -> f32 {
    if magnitude <= 0.0 {
        return 0.0;
    }
    let db = 20.0 * magnitude.log10();
    ((db - MIN_DB) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0)
}

/// Half-open FFT-bin range `[start, end)` covered by log-spaced band `band` of `n_bands`, over
/// `usable_bins` bins.
///
/// **Both backends call this**, so their bands cover the same frequencies even though one folds
/// complex magnitudes from its own FFT and the other folds bytes from a browser's. Without a
/// single definition here, "band 7" would quietly mean two different frequency ranges.
///
/// Bin 0 is DC and carries no pitch, so ranges start at 1. Every band is at least one bin wide.
pub(crate) fn log_band_range(band: usize, n_bands: usize, usable_bins: usize) -> (usize, usize) {
    if n_bands == 0 || usable_bins < 2 {
        return (1, 1);
    }
    let hi = usable_bins as f32;
    // edge(b) = 1 * hi^(b / n_bands) — geometric spacing from bin 1 to the top bin.
    let edge = |b: usize| hi.powf(b as f32 / n_bands as f32);
    let start = (edge(band) as usize).clamp(1, usable_bins - 1);
    let end = ((edge(band + 1) as usize).max(start + 1)).min(usable_bins);
    (start, end)
}

/// Resamples `src` bands into `out`, averaging the source bands that fall in each output slot.
///
/// Averaging rather than point-sampling so that asking for fewer bands than the backend publishes
/// cannot drop a spike entirely — a 4-bar display should still react to a kick. An empty `out` is
/// a no-op; an `out` longer than `src` stretches (each output slot maps to one source band).
pub(crate) fn resample_bands(src: &[f32], out: &mut [f32]) {
    if out.is_empty() {
        return;
    }
    if src.is_empty() {
        out.fill(0.0);
        return;
    }
    let n_out = out.len();
    let n_src = src.len();
    for (i, slot) in out.iter_mut().enumerate() {
        // Half-open source range [lo, hi) for output slot i, always at least one band wide.
        let lo = i * n_src / n_out;
        let hi = (((i + 1) * n_src).div_ceil(n_out)).max(lo + 1).min(n_src);
        let span = &src[lo..hi];
        *slot = span.iter().sum::<f32>() / span.len() as f32;
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

    // ── Spectrum policy ───────────────────────────────────────────────────────

    #[test]
    fn the_db_window_matches_web_audios_defaults() {
        // This is the whole reason a native band and a web band mean the same thing: the native
        // side normalizes over exactly the window the browser's AnalyserNode uses. If someone
        // "tunes" these, the two platforms silently diverge.
        assert_eq!(MIN_DB, -100.0, "AnalyserNode.minDecibels default");
        assert_eq!(MAX_DB, -30.0, "AnalyserNode.maxDecibels default");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn normalized_db_maps_the_window_to_the_unit_range() {
        assert_eq!(normalized_db(0.0), 0.0, "silence");
        // -100 dB is the bottom of the window, -30 dB the top.
        let bottom = 10f32.powf(MIN_DB / 20.0);
        let top = 10f32.powf(MAX_DB / 20.0);
        assert!(
            normalized_db(bottom).abs() < 1e-3,
            "{}",
            normalized_db(bottom)
        );
        assert!(
            (normalized_db(top) - 1.0).abs() < 1e-3,
            "{}",
            normalized_db(top)
        );
        // Halfway in dB is halfway in the output — the mapping is linear in decibels.
        let mid = 10f32.powf((MIN_DB + MAX_DB) / 2.0 / 20.0);
        assert!(
            (normalized_db(mid) - 0.5).abs() < 1e-3,
            "{}",
            normalized_db(mid)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn normalized_db_clamps_rather_than_escaping_the_unit_range() {
        assert_eq!(normalized_db(100.0), 1.0, "over-loud must not exceed 1.0");
        assert_eq!(normalized_db(1e-12), 0.0, "inaudible must not go below 0.0");
    }

    #[test]
    fn log_bands_are_ordered_contiguous_and_never_empty() {
        let (n_bands, bins) = (SPECTRUM_BANDS, 512);
        let mut prev_end = 1;
        for b in 0..n_bands {
            let (start, end) = log_band_range(b, n_bands, bins);
            assert!(end > start, "band {b} is empty: {start}..{end}");
            assert!(start >= 1, "band {b} must skip the DC bin");
            assert!(
                end <= bins,
                "band {b} runs past the spectrum: {end} > {bins}"
            );
            assert!(start >= prev_end - 1, "band {b} goes backwards");
            prev_end = end;
        }
    }

    #[test]
    fn log_bands_get_wider_toward_high_frequencies() {
        // The point of log spacing: an octave near the top spans many more bins than one near the
        // bottom, so the bass does not collapse into a single bar.
        let (lo_s, lo_e) = log_band_range(1, SPECTRUM_BANDS, 512);
        let (hi_s, hi_e) = log_band_range(SPECTRUM_BANDS - 1, SPECTRUM_BANDS, 512);
        assert!(
            (hi_e - hi_s) > (lo_e - lo_s),
            "high band {}..{} should be wider than low band {}..{}",
            hi_s,
            hi_e,
            lo_s,
            lo_e
        );
    }

    #[test]
    fn log_bands_degenerate_safely() {
        assert_eq!(log_band_range(0, 0, 512), (1, 1));
        assert_eq!(log_band_range(0, 8, 1), (1, 1));
        assert_eq!(log_band_range(0, 8, 0), (1, 1));
    }

    #[test]
    fn resampling_preserves_a_flat_spectrum_at_any_output_length() {
        let src = [0.5f32; SPECTRUM_BANDS];
        for n in [1usize, 4, 8, 32, 64] {
            let mut out = vec![0.0; n];
            resample_bands(&src, &mut out);
            assert!(
                out.iter().all(|v| (v - 0.5).abs() < 1e-6),
                "n={n} gave {out:?}"
            );
        }
    }

    #[test]
    fn resampling_down_averages_rather_than_dropping_a_spike() {
        // A 4-bar display must still react to a kick that lands in one source band; point
        // sampling would miss it entirely most of the time.
        let mut src = [0.0f32; SPECTRUM_BANDS];
        src[1] = 1.0;
        let mut out = [0.0f32; 4];
        resample_bands(&src, &mut out);
        assert!(out[0] > 0.0, "the spike must survive downsampling: {out:?}");
        assert_eq!(&out[1..], &[0.0, 0.0, 0.0], "and not smear everywhere");
    }

    #[test]
    fn resampling_handles_empty_input_and_output() {
        let mut out = [1.0f32; 4];
        resample_bands(&[], &mut out);
        assert_eq!(out, [0.0; 4], "no source data reads as silence");
        let mut none: [f32; 0] = [];
        resample_bands(&[0.5; SPECTRUM_BANDS], &mut none); // must not panic
    }

    #[test]
    fn summing_voices_adds_them_rather_than_taking_the_loudest() {
        // The decision this function exists to pin down. Max would report 0.5 here; sum reports
        // 0.9, and the difference is the whole reason a metered one-shot is worth having — three
        // overlapping hits should read louder than one.
        let combined = sum_levels([
            AudioLevels {
                rms: 0.5,
                peak: 0.6,
            },
            AudioLevels {
                rms: 0.4,
                peak: 0.2,
            },
        ]);
        assert!((combined.rms - 0.9).abs() < 1e-6, "rms {}", combined.rms);
        assert!((combined.peak - 0.8).abs() < 1e-6, "peak {}", combined.peak);
    }

    #[test]
    fn a_sum_past_full_scale_is_clamped() {
        let combined = sum_levels(
            [AudioLevels {
                rms: 0.7,
                peak: 0.9,
            }; 3],
        );
        assert_eq!(combined.rms, 1.0);
        assert_eq!(combined.peak, 1.0);
    }

    #[test]
    fn summing_one_voice_or_none_is_the_identity_or_silence() {
        let one = AudioLevels {
            rms: 0.3,
            peak: 0.45,
        };
        assert_eq!(sum_levels([one]), one);
        assert_eq!(sum_levels([]), AudioLevels::SILENT);
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
