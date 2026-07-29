//! Native amplitude analysis: a `Source` tap that measures a channel as it plays.
//!
//! [`LevelTap`] wraps a channel's source and observes every sample on its way to the device —
//! the same shape as [`PannedSource`](super::source::PannedSource), which is the existing
//! precedent for "see each sample without changing it".
//!
//! # Why the atomics
//!
//! A `Source` is pulled by **rodio's playback thread**, while `levels()` is read from the game
//! thread. The two must communicate without ever blocking the audio callback, so the tap publishes
//! into a lock-free [`LevelSlot`] of atomics rather than taking a lock. A torn read would at worst
//! pair one window's `rms` with the next window's `peak`, which is invisible in a meter.
//!
//! # Why a sequence counter
//!
//! The tap only runs while rodio is pulling samples. When a channel stops, its last published
//! values would otherwise sit there forever and the meter would freeze at full height — reading as
//! a broken bar rather than silence. [`LevelSlot::read`] therefore also returns a monotonic
//! sequence number; [`AudioManager::tick_analysis`] treats "sequence unchanged since last frame"
//! as *not producing* and decays that channel toward silence.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

use crate::audio_analysis::{smooth_toward, AudioLevels, DEFAULT_ANALYSIS_SMOOTHING};

use super::AudioManager;

/// Samples accumulated before the tap publishes one level update.
///
/// 1024 interleaved samples is ~10 ms at 48 kHz stereo — several updates per rendered frame at
/// 60 fps, so the meter is never starved, while the publish itself stays rare enough to be free.
pub(crate) const ANALYSIS_WINDOW: u32 = 1024;

// ─── Lock-free publication slot ───────────────────────────────────────────────

/// The audio thread's side of the meter: two `f32`s (stored as bits) plus a sequence counter.
#[derive(Debug, Default)]
pub(crate) struct LevelSlot {
    /// Last published RMS, as `f32::to_bits`.
    rms: AtomicU32,
    /// Last published peak, as `f32::to_bits`.
    peak: AtomicU32,
    /// Incremented on every publish. A reader uses it to tell "still producing" from "stopped".
    seq: AtomicU64,
}

impl LevelSlot {
    /// Publishes one window's measurement. Called on the **playback thread**; never blocks.
    fn publish(&self, rms: f32, peak: f32) {
        self.rms.store(rms.to_bits(), Ordering::Relaxed);
        self.peak.store(peak.to_bits(), Ordering::Relaxed);
        self.seq.fetch_add(1, Ordering::Release);
    }

    /// Reads the last published `(rms, peak, seq)`. Called on the **game thread**.
    pub(crate) fn read(&self) -> (f32, f32, u64) {
        let rms = f32::from_bits(self.rms.load(Ordering::Relaxed));
        let peak = f32::from_bits(self.peak.load(Ordering::Relaxed));
        // Loaded last so a bumped sequence implies the values above are at least as new.
        let seq = self.seq.load(Ordering::Acquire);
        (rms, peak, seq)
    }
}

// ─── The source wrapper ───────────────────────────────────────────────────────

/// A pass-through [`Source`] that measures RMS and peak over fixed windows.
///
/// Samples are forwarded **unchanged** — this only observes. It is inserted after the sound's own
/// effects and before pan/volume, so it measures the pre-volume envelope (see
/// [`AudioLevels`](crate::AudioLevels)).
pub(crate) struct LevelTap<S: Source> {
    inner: S,
    slot: Arc<LevelSlot>,
    sum_sq: f32,
    peak: f32,
    count: u32,
}

impl<S: Source> LevelTap<S> {
    pub(crate) fn new(inner: S, slot: Arc<LevelSlot>) -> Self {
        Self {
            inner,
            slot,
            sum_sq: 0.0,
            peak: 0.0,
            count: 0,
        }
    }
}

impl<S: Source> Iterator for LevelTap<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.inner.next()?;
        self.sum_sq += sample * sample;
        let magnitude = sample.abs();
        if magnitude > self.peak {
            self.peak = magnitude;
        }
        self.count += 1;
        if self.count >= ANALYSIS_WINDOW {
            let rms = (self.sum_sq / self.count as f32).sqrt();
            // Clamped: a source may legitimately exceed 1.0, but a meter reading above full
            // scale is meaningless and would push a bar off screen.
            self.slot.publish(rms.min(1.0), self.peak.min(1.0));
            self.sum_sq = 0.0;
            self.peak = 0.0;
            self.count = 0;
        }
        Some(sample)
    }
}

impl<S: Source> Source for LevelTap<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.inner.current_span_len()
    }
    fn channels(&self) -> ChannelCount {
        self.inner.channels()
    }
    fn sample_rate(&self) -> SampleRate {
        self.inner.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }
}

// ─── Manager-side smoothed state ──────────────────────────────────────────────

/// One analyzed channel: the shared slot the audio thread writes, plus the smoothed values the
/// game reads.
#[derive(Debug)]
pub(crate) struct AnalysisChannel {
    pub(crate) slot: Arc<LevelSlot>,
    rms: f32,
    peak: f32,
    last_seq: u64,
}

impl AnalysisChannel {
    fn new() -> Self {
        Self {
            slot: Arc::new(LevelSlot::default()),
            rms: 0.0,
            peak: 0.0,
            last_seq: 0,
        }
    }
}

impl AudioManager {
    // ── Amplitude analysis ────────────────────────────────────────────────────

    /// Starts measuring `channel`'s loudness, so [`levels`](Self::levels) reports it.
    ///
    /// **Takes effect on the next `play_*` call on that channel** — the meter is a wrapper around
    /// the playing source, and a source already handed to the device cannot be re-wrapped. This
    /// mirrors [`set_effect`](Self::set_effect)'s "applied on the next play" semantics, so enable
    /// analysis during setup rather than mid-sound.
    ///
    /// Enabling a channel that is already analyzed keeps its current reading (it does not reset).
    /// A channel with analysis **off** pays nothing: no wrapper is inserted and its source chain is
    /// byte-identical to before.
    pub fn enable_analysis(&mut self, channel: &str) {
        self.analysis
            .entry(channel.to_string())
            .or_insert_with(AnalysisChannel::new);
    }

    /// Stops measuring `channel` and drops its meter state.
    ///
    /// A sound already playing keeps its tap until it ends (it is part of the source chain), but
    /// nothing reads it and [`levels`](Self::levels) reports
    /// [`SILENT`](crate::AudioLevels::SILENT) immediately.
    pub fn disable_analysis(&mut self, channel: &str) {
        self.analysis.remove(channel);
    }

    /// Returns whether `channel` is being analyzed.
    pub fn is_analysis_enabled(&self, channel: &str) -> bool {
        self.analysis.contains_key(channel)
    }

    /// Returns `channel`'s current smoothed loudness, or
    /// [`SILENT`](crate::AudioLevels::SILENT) if it is not analyzed, never played, or stopped.
    ///
    /// Values advance in [`update`](Self::update), so call that every frame (`AudioSystem` and
    /// `AudioFacadeSystem` both do).
    pub fn levels(&self, channel: &str) -> AudioLevels {
        self.analysis
            .get(channel)
            .map(|c| AudioLevels {
                rms: c.rms,
                peak: c.peak,
            })
            .unwrap_or(AudioLevels::SILENT)
    }

    /// Sets the meter's release time in seconds — how long a level takes to fall.
    ///
    /// A rise is always instant, so a transient is never visually late; this controls only the
    /// decay. `0.0` disables smoothing. Defaults to [`DEFAULT_ANALYSIS_SMOOTHING`]. Applies to
    /// every analyzed channel.
    ///
    /// [`DEFAULT_ANALYSIS_SMOOTHING`]: crate::DEFAULT_ANALYSIS_SMOOTHING
    pub fn set_analysis_smoothing(&mut self, release_secs: f32) {
        self.analysis_smoothing = release_secs.max(0.0);
    }

    /// Returns the meter release time in seconds.
    pub fn analysis_smoothing(&self) -> f32 {
        self.analysis_smoothing
    }

    /// Wraps `source` in a [`LevelTap`] when `channel` is analyzed, otherwise returns it unchanged.
    ///
    /// Called from the play paths. The unanalyzed case returns the very same box, so a game that
    /// never enables analysis has an identical source chain and pays no per-sample cost.
    pub(super) fn tapped(
        &self,
        channel: &str,
        source: Box<dyn Source + Send + 'static>,
    ) -> Box<dyn Source + Send + 'static> {
        match self.analysis.get(channel) {
            Some(ch) => Box::new(LevelTap::new(source, Arc::clone(&ch.slot))),
            None => source,
        }
    }

    /// Advances every analyzed channel's smoothed levels. Called once per frame from
    /// [`update`](Self::update).
    ///
    /// A channel whose sequence counter has not moved since the last tick is **not producing**
    /// (stopped, drained or paused), so it decays toward silence instead of holding its last
    /// reading — otherwise a stopped sound leaves its bar pinned at full height.
    pub(super) fn tick_analysis(&mut self, dt: f32) {
        let release = self.analysis_smoothing;
        for channel in self.analysis.values_mut() {
            let (rms, peak, seq) = channel.slot.read();
            let producing = seq != channel.last_seq;
            channel.last_seq = seq;
            let (target_rms, target_peak) = if producing { (rms, peak) } else { (0.0, 0.0) };
            channel.rms = smooth_toward(channel.rms, target_rms, release, dt);
            channel.peak = smooth_toward(channel.peak, target_peak, release, dt);
        }
    }
}

/// Builds the empty analysis map for a fresh [`AudioManager`].
pub(super) fn new_analysis_map() -> HashMap<String, AnalysisChannel> {
    HashMap::new()
}

/// The default meter release time, re-exported for `AudioManager::new`.
pub(super) const DEFAULT_SMOOTHING: f32 = DEFAULT_ANALYSIS_SMOOTHING;

#[cfg(test)]
mod tests {
    use super::*;
    use rodio::buffer::SamplesBuffer;

    const TEST_CHANNELS: ChannelCount = ChannelCount::MIN; // 1 = mono
    const TEST_RATE: SampleRate = match SampleRate::new(48_000) {
        Some(r) => r,
        None => panic!("48 kHz is non-zero"),
    };

    /// Pulls every sample of `samples` through a tap, returning the slot it published into.
    fn run_tap(samples: Vec<f32>) -> Arc<LevelSlot> {
        let slot = Arc::new(LevelSlot::default());
        let buffer = SamplesBuffer::new(TEST_CHANNELS, TEST_RATE, samples);
        let mut tap = LevelTap::new(buffer, Arc::clone(&slot));
        while tap.next().is_some() {}
        slot
    }

    #[test]
    fn a_full_scale_square_wave_measures_rms_and_peak_of_one() {
        // Every sample is ±1.0, so RMS == peak == 1.0 exactly. Device-free: this is the
        // measurement math, not playback.
        let samples: Vec<f32> = (0..ANALYSIS_WINDOW * 2)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let (rms, peak, seq) = run_tap(samples).read();
        assert!((rms - 1.0).abs() < 1e-5, "rms {rms}");
        assert!((peak - 1.0).abs() < 1e-5, "peak {peak}");
        assert_eq!(seq, 2, "two full windows should publish twice");
    }

    #[test]
    fn a_sine_measures_rms_of_one_over_root_two() {
        // The classic result: a unit sine has RMS 1/√2 ≈ 0.7071 while its peak is 1.0. This is
        // what pins that `rms` is really RMS and not just a rectified average (which would be
        // 2/π ≈ 0.6366) or the peak itself.
        let samples: Vec<f32> = (0..ANALYSIS_WINDOW)
            .map(|i| {
                let phase = i as f32 / ANALYSIS_WINDOW as f32 * std::f32::consts::TAU * 8.0;
                phase.sin()
            })
            .collect();
        let (rms, peak, _) = run_tap(samples).read();
        assert!(
            (rms - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01,
            "expected RMS ≈ 0.707 for a unit sine, got {rms}"
        );
        assert!((peak - 1.0).abs() < 0.01, "peak {peak}");
    }

    #[test]
    fn silence_measures_zero() {
        let (rms, peak, seq) = run_tap(vec![0.0; ANALYSIS_WINDOW as usize]).read();
        assert_eq!(rms, 0.0);
        assert_eq!(peak, 0.0);
        assert_eq!(seq, 1);
    }

    #[test]
    fn a_partial_window_never_publishes() {
        // Fewer samples than one window: nothing is published, so `seq` stays 0 and a reader
        // sees "not producing" rather than a level derived from a fraction of a window.
        let (_, _, seq) = run_tap(vec![1.0; (ANALYSIS_WINDOW - 1) as usize]).read();
        assert_eq!(seq, 0);
    }

    #[test]
    fn the_tap_forwards_samples_unchanged() {
        // It measures; it must not alter the audio.
        let input: Vec<f32> = (0..100).map(|i| (i as f32 / 100.0) - 0.5).collect();
        let slot = Arc::new(LevelSlot::default());
        let buffer = SamplesBuffer::new(TEST_CHANNELS, TEST_RATE, input.clone());
        let tap = LevelTap::new(buffer, slot);
        let output: Vec<f32> = tap.collect();
        assert_eq!(input, output);
    }

    #[test]
    fn peak_exceeds_rms_on_a_transient() {
        // A single loud spike in an otherwise quiet window: peak sees it, RMS barely moves.
        // This is why beat detection should key on `peak`.
        let mut samples = vec![0.01_f32; ANALYSIS_WINDOW as usize];
        samples[10] = 1.0;
        let (rms, peak, _) = run_tap(samples).read();
        assert!((peak - 1.0).abs() < 1e-5, "peak {peak}");
        assert!(rms < 0.05, "a lone spike should barely move RMS, got {rms}");
        assert!(peak > rms * 10.0);
    }

    #[test]
    fn levels_are_clamped_to_full_scale() {
        // An over-unity source must not push a bar off screen.
        let (rms, peak, _) = run_tap(vec![5.0; ANALYSIS_WINDOW as usize]).read();
        assert_eq!(rms, 1.0);
        assert_eq!(peak, 1.0);
    }
}
