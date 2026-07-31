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
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use rodio::{ChannelCount, Sample, SampleRate, Source};

use crate::audio_analysis::{
    resample_bands, smooth_toward, sum_levels, AudioLevels, DEFAULT_ANALYSIS_SMOOTHING,
    SPECTRUM_BANDS,
};

/// Simultaneous voices one metered one-shot name can sound before it wraps and cuts its own oldest
/// voice (`AudioManager::play_tone_poly`).
///
/// Half the anonymous ring's 16, because these are *per name*: eight overlapping hits of the same
/// sound is already past what a player can distinguish, while the ring is shared by every
/// fire-and-forget sound in the game at once.
pub(crate) const POLY_VOICES: u64 = 8;

/// Sink channel name for one voice of a metered one-shot: `__poly_{meter}_{voice}`.
///
/// These are ordinary channels as far as the sink map is concerned — which is the whole point of
/// the design. The channel model stays one-sink-per-name; only the *meter* is shared, by
/// `AudioManager::poly_tapped`. Pure (no device) so it is unit-testable headlessly, like
/// `sfx_voice_channel`'s anonymous counterpart in `audio_facade`.
pub(crate) fn poly_voice_channel(meter: &str, voice: usize) -> String {
    format!("__poly_{meter}_{voice}")
}

/// Picks the next voice index for `meter` and advances its counter.
///
/// Split out of `play_tone_poly` so the rotation — the thing that makes consecutive plays *not*
/// cut each other — is testable without an audio device, which CI does not have.
pub(crate) fn next_poly_voice(seq: &mut HashMap<String, u64>, meter: &str) -> usize {
    let counter = seq.entry(meter.to_string()).or_insert(0);
    let voice = (*counter % POLY_VOICES) as usize;
    *counter = counter.wrapping_add(1);
    voice
}

/// Mono frames per spectrum transform. A power of two (required by the radix-2 FFT); 1024 frames
/// is ~21 ms at 48 kHz, giving a fresh spectrum several times per rendered frame.
pub(crate) const SPECTRUM_FFT_SIZE: usize = 1024;

use super::AudioManager;

/// Samples accumulated before the tap publishes one level update.
///
/// 1024 interleaved samples is ~10 ms at 48 kHz stereo — several updates per rendered frame at
/// 60 fps, so the meter is never starved, while the publish itself stays rare enough to be free.
pub(crate) const ANALYSIS_WINDOW: u32 = 1024;

// ─── Lock-free publication slot ───────────────────────────────────────────────

/// The audio thread's side of the meter: two `f32`s (stored as bits) plus a sequence counter.
#[derive(Debug)]
pub(crate) struct LevelSlot {
    /// Last published RMS, as `f32::to_bits`.
    rms: AtomicU32,
    /// Last published peak, as `f32::to_bits`.
    peak: AtomicU32,
    /// Incremented on every publish. A reader uses it to tell "still producing" from "stopped".
    seq: AtomicU64,
    /// Last published log-spaced spectrum, each as `f32::to_bits`. All zero unless spectrum
    /// analysis is on for this channel.
    bands: [AtomicU32; SPECTRUM_BANDS],
    /// Whether the tap should run an FFT. Read by the audio thread every window, set by the game
    /// thread, so a game that only wants `levels()` never pays for a transform.
    spectrum_on: AtomicBool,
}

impl Default for LevelSlot {
    fn default() -> Self {
        Self {
            rms: AtomicU32::new(0),
            peak: AtomicU32::new(0),
            seq: AtomicU64::new(0),
            bands: std::array::from_fn(|_| AtomicU32::new(0)),
            spectrum_on: AtomicBool::new(false),
        }
    }
}

impl LevelSlot {
    /// Publishes one window's spectrum. Called on the **playback thread**; never blocks.
    fn publish_bands(&self, bands: &[f32]) {
        for (slot, v) in self.bands.iter().zip(bands) {
            slot.store(v.to_bits(), Ordering::Relaxed);
        }
    }

    /// Reads the last published spectrum into `out` (length [`SPECTRUM_BANDS`]).
    pub(crate) fn read_bands(&self, out: &mut [f32; SPECTRUM_BANDS]) {
        for (dst, slot) in out.iter_mut().zip(&self.bands) {
            *dst = f32::from_bits(slot.load(Ordering::Relaxed));
        }
    }

    /// Turns the audio thread's FFT on or off for this channel.
    pub(crate) fn set_spectrum(&self, on: bool) {
        self.spectrum_on.store(on, Ordering::Relaxed);
    }

    fn wants_spectrum(&self) -> bool {
        self.spectrum_on.load(Ordering::Relaxed)
    }
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
    // ── Spectrum state (only touched while the channel wants a spectrum) ──────
    /// Interleaved-channel count, so frames can be downmixed to mono.
    channels: u16,
    /// Accumulator for the current frame's channels, and how many have arrived.
    frame_acc: f32,
    frame_ch: u16,
    /// Mono frames collected toward the next transform.
    spec_buf: Vec<f32>,
    /// Caller-owned FFT scratch — the transform runs on the audio thread and must not allocate.
    scratch_re: Vec<f32>,
    scratch_im: Vec<f32>,
    band_buf: Vec<f32>,
}

impl<S: Source> LevelTap<S> {
    pub(crate) fn new(inner: S, slot: Arc<LevelSlot>) -> Self {
        let channels = inner.channels().get().max(1);
        Self {
            inner,
            slot,
            sum_sq: 0.0,
            peak: 0.0,
            count: 0,
            channels,
            frame_acc: 0.0,
            frame_ch: 0,
            // Allocated once, here on the game thread — never inside `next`.
            spec_buf: Vec::with_capacity(SPECTRUM_FFT_SIZE),
            scratch_re: vec![0.0; SPECTRUM_FFT_SIZE],
            scratch_im: vec![0.0; SPECTRUM_FFT_SIZE],
            band_buf: vec![0.0; SPECTRUM_BANDS],
        }
    }

    /// Feeds one interleaved sample into the spectrum path, downmixing to mono and running a
    /// transform once a full buffer of frames has arrived.
    ///
    /// Mono downmix matters: an FFT over raw *interleaved* stereo alternates L and R samples,
    /// which is not the signal and produces a mirrored, meaningless spectrum.
    fn feed_spectrum(&mut self, sample: f32) {
        self.frame_acc += sample;
        self.frame_ch += 1;
        if self.frame_ch < self.channels {
            return;
        }
        let mono = self.frame_acc / self.channels as f32;
        self.frame_acc = 0.0;
        self.frame_ch = 0;
        self.spec_buf.push(mono);
        if self.spec_buf.len() < SPECTRUM_FFT_SIZE {
            return;
        }
        crate::audio::spectrum::spectrum_into(
            &self.spec_buf,
            &mut self.scratch_re,
            &mut self.scratch_im,
            &mut self.band_buf,
        );
        self.slot.publish_bands(&self.band_buf);
        self.spec_buf.clear();
    }
}

impl<S: Source> Iterator for LevelTap<S> {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        let sample = self.inner.next()?;
        // Gated so a channel that only wants `levels()` never pays for a transform. Checked per
        // sample because the game can toggle it at any time, but it is one relaxed atomic load.
        if self.slot.wants_spectrum() {
            self.feed_spectrum(sample);
        }
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

/// One voice of a polyphonic metered name: its own publication slot, plus the sequence number last
/// seen for it so a drained voice can be told from a sounding one.
///
/// Each overlapping voice needs its **own** slot. Pointing several taps at one slot would make them
/// race — `LevelSlot::publish` is a plain store, so the reader would see whichever voice published
/// most recently, which is neither the sum nor the loudest.
#[derive(Debug)]
struct PolyVoice {
    slot: Arc<LevelSlot>,
    last_seq: u64,
}

/// Combines a polyphonic name's voices into this frame's raw target, applying the **per-voice**
/// staleness test before summing.
///
/// The per-voice part is what makes it correct: a voice whose sequence has not moved since the last
/// tick has drained, so it contributes nothing. Testing staleness once for the whole name would
/// hold a finished voice's level up for as long as any *other* voice was still sounding.
///
/// Split out of `tick_analysis` so it can be tested without an audio device.
fn combine_voices(single: AudioLevels, poly: &mut [PolyVoice]) -> AudioLevels {
    sum_levels(std::iter::once(single).chain(poly.iter_mut().map(|voice| {
        let (rms, peak, seq) = voice.slot.read();
        let sounding = seq != voice.last_seq;
        voice.last_seq = seq;
        if sounding {
            AudioLevels { rms, peak }
        } else {
            AudioLevels::SILENT
        }
    })))
}

/// One analyzed channel: the shared slot the audio thread writes, plus the smoothed values the
/// game reads.
#[derive(Debug)]
pub(crate) struct AnalysisChannel {
    pub(crate) slot: Arc<LevelSlot>,
    rms: f32,
    peak: f32,
    last_seq: u64,
    /// Per-voice slots for a polyphonic metered name. **Empty for an ordinary channel**, so a
    /// single-voice meter takes exactly the path it did before this existed.
    poly: Vec<PolyVoice>,
    /// Smoothed spectrum, in the backend's internal band count. `bands()` resamples it to whatever
    /// length the caller asks for.
    bands: [f32; SPECTRUM_BANDS],
    /// Whether a spectrum was requested for this channel.
    spectrum: bool,
}

impl AnalysisChannel {
    fn new() -> Self {
        Self {
            slot: Arc::new(LevelSlot::default()),
            rms: 0.0,
            peak: 0.0,
            last_seq: 0,
            poly: Vec::new(),
            bands: [0.0; SPECTRUM_BANDS],
            spectrum: false,
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

    /// Starts measuring `channel`'s **frequency spectrum** as well as its levels, so
    /// [`bands`](Self::bands) reports it. Implies [`enable_analysis`](Self::enable_analysis).
    ///
    /// Separate from `enable_analysis` because a spectrum costs an FFT per window on the playback
    /// thread, and most uses (a pulse, a kick flash) only need [`levels`](Self::levels). A channel
    /// with levels-only analysis runs no transform at all.
    ///
    /// Like `enable_analysis`, this takes effect on the next `play_*` for a channel that is not
    /// already analyzed; for one that is, the transform starts on the next window.
    pub fn enable_spectrum(&mut self, channel: &str) {
        let ch = self
            .analysis
            .entry(channel.to_string())
            .or_insert_with(AnalysisChannel::new);
        ch.spectrum = true;
        ch.slot.set_spectrum(true);
    }

    /// Stops computing a spectrum for `channel`, leaving its levels running.
    pub fn disable_spectrum(&mut self, channel: &str) {
        if let Some(ch) = self.analysis.get_mut(channel) {
            ch.spectrum = false;
            ch.slot.set_spectrum(false);
            ch.bands = [0.0; SPECTRUM_BANDS];
        }
    }

    /// Writes `channel`'s current smoothed spectrum into `out`, as `out.len()` log-spaced bands
    /// from low frequency to high, each `0.0..=1.0`.
    ///
    /// **You choose the band count** — `out.len()` is resampled from the backend's internal
    /// resolution, which is what keeps the two platforms' different FFT sizes out of the API.
    /// Fills zeros for a channel with no spectrum enabled, or one that is silent.
    ///
    /// Values are normalized over a decibel window (not linear magnitude), because hearing is
    /// logarithmic: a linear bar spends nearly all its travel in the top of the range.
    ///
    /// ⚠️ **Cross-platform values are equivalent, not identical.** Native runs its own FFT while
    /// wasm reads a Web Audio `AnalyserNode`; the two use the same dB window and band spacing and
    /// track each other closely, but they are not bit-for-bit equal. Drive visuals with them, not
    /// checksums.
    pub fn bands(&self, channel: &str, out: &mut [f32]) {
        match self.analysis.get(channel) {
            Some(ch) if ch.spectrum => resample_bands(&ch.bands, out),
            _ => out.fill(0.0),
        }
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

    /// Like [`tapped`](Self::tapped), but publishes into one **voice slot** of a polyphonic metered
    /// name instead of the channel's single shared slot.
    ///
    /// `meter` is the logical name the game passes to `levels()`; `voice` is the index within that
    /// name's ring. Slots are created on demand and then reused, so a name that has sounded once
    /// never allocates again. Returns the source unchanged when `meter` is not analyzed — the same
    /// pay-nothing rule as `tapped`.
    pub(super) fn poly_tapped(
        &mut self,
        meter: &str,
        voice: usize,
        source: Box<dyn Source + Send + 'static>,
    ) -> Box<dyn Source + Send + 'static> {
        let Some(channel) = self.analysis.get_mut(meter) else {
            return source;
        };
        while channel.poly.len() <= voice {
            channel.poly.push(PolyVoice {
                slot: Arc::new(LevelSlot::default()),
                last_seq: 0,
            });
        }
        // A wrapped voice reuses its slot, so the tap that owned it previously has already been
        // dropped with its sink.
        //
        // Spectrum is deliberately left OFF on a voice slot. `bands()` reads the channel's own
        // slot, and summing eight band arrays per window would put a per-voice FFT on the playback
        // thread to feed an API whose use case (a soundtrack) is not a one-shot. `levels()` is what
        // a metered one-shot is for; `bands()` reports zeros for one, as it does for any channel
        // with no spectrum enabled.
        let slot = Arc::clone(&channel.poly[voice].slot);
        Box::new(LevelTap::new(source, slot))
    }

    /// Advances every analyzed channel's smoothed levels. Called once per frame from
    /// [`update`](Self::update).
    ///
    /// A channel whose sequence counter has not moved since the last tick is **not producing**
    /// (stopped, drained or paused), so it decays toward silence instead of holding its last
    /// reading — otherwise a stopped sound leaves its bar pinned at full height. A polyphonic name
    /// applies that test **per voice** and sums the ones still sounding
    /// ([`sum_levels`](crate::audio_analysis::sum_levels)).
    pub(super) fn tick_analysis(&mut self, dt: f32) {
        let release = self.analysis_smoothing;
        let mut raw_bands = [0.0f32; SPECTRUM_BANDS];
        for channel in self.analysis.values_mut() {
            let (rms, peak, seq) = channel.slot.read();
            let producing = seq != channel.last_seq;
            channel.last_seq = seq;
            let (mut target_rms, mut target_peak) =
                if producing { (rms, peak) } else { (0.0, 0.0) };
            if !channel.poly.is_empty() {
                let single = AudioLevels {
                    rms: target_rms,
                    peak: target_peak,
                };
                let combined = combine_voices(single, &mut channel.poly);
                target_rms = combined.rms;
                target_peak = combined.peak;
            }
            channel.rms = smooth_toward(channel.rms, target_rms, release, dt);
            channel.peak = smooth_toward(channel.peak, target_peak, release, dt);

            if channel.spectrum {
                // Same staleness rule as the levels: a stopped channel's bars must fall, not
                // freeze at the last spectrum the tap managed to publish.
                if producing {
                    channel.slot.read_bands(&mut raw_bands);
                } else {
                    raw_bands = [0.0; SPECTRUM_BANDS];
                }
                for (smoothed, target) in channel.bands.iter_mut().zip(raw_bands) {
                    *smoothed = smooth_toward(*smoothed, target, release, dt);
                }
            }
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
    fn a_tap_outside_repeat_keeps_publishing_on_every_pass() {
        // Regression for a looping sound's meter going dead after one pass. `repeat_infinite`
        // buffers its input and replays that buffer, so a tap wrapped INSIDE it is polled exactly
        // once: `play_music` metered its first pass and then read silence over an audible,
        // still-looping track. `append_decoded` therefore repeats first and taps second. This is
        // source composition, not playback, so it needs no device.
        let one_window: Vec<f32> = vec![1.0; ANALYSIS_WINDOW as usize];
        let passes = 3u64;
        let pull = |src: &mut dyn Iterator<Item = f32>| {
            for _ in 0..ANALYSIS_WINDOW as u64 * passes {
                src.next().expect("a repeating source never ends");
            }
        };

        // The shipped order: repeat first, so the tap is on the outside.
        let outside = Arc::new(LevelSlot::default());
        let buffer = SamplesBuffer::new(TEST_CHANNELS, TEST_RATE, one_window.clone());
        pull(&mut LevelTap::new(
            buffer.repeat_infinite(),
            Arc::clone(&outside),
        ));
        assert_eq!(
            outside.read().2,
            passes,
            "a tap outside repeat must publish once per pass"
        );

        // The order that shipped before the fix, kept as the contrast that gives the assertion
        // above its meaning — and as a tripwire if rodio ever stops buffering `Repeat`.
        let inside = Arc::new(LevelSlot::default());
        let buffer = SamplesBuffer::new(TEST_CHANNELS, TEST_RATE, one_window);
        pull(&mut LevelTap::new(buffer, Arc::clone(&inside)).repeat_infinite());
        assert_eq!(
            inside.read().2,
            1,
            "tapping inside repeat is expected to publish only the first pass — if this now \
             publishes more, rodio's Repeat no longer buffers and append_decoded's comment is stale"
        );
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

    // ─── Metered one-shots (polyphonic metering) ──────────────────────────────

    /// A slot that has published `(rms, peak)` exactly once, as a real tap would.
    fn published_slot(rms: f32, peak: f32) -> Arc<LevelSlot> {
        let slot = Arc::new(LevelSlot::default());
        slot.publish(rms, peak);
        slot
    }

    fn voice(rms: f32, peak: f32) -> PolyVoice {
        PolyVoice {
            slot: published_slot(rms, peak),
            last_seq: 0,
        }
    }

    #[test]
    fn consecutive_metered_one_shots_take_different_voices() {
        // THE property the API exists for: two triggers in a row must not land on the same sink,
        // because that is what `play_tone_on_channel` does and why it cuts. Different voice index
        // ⇒ different channel name ⇒ `stop_immediate` never touches the sound already playing.
        let mut seq = HashMap::new();
        let first = next_poly_voice(&mut seq, "hit");
        let second = next_poly_voice(&mut seq, "hit");
        assert_ne!(
            first, second,
            "a replay reused the same voice — it would cut the sound already there"
        );
        assert_ne!(
            poly_voice_channel("hit", first),
            poly_voice_channel("hit", second)
        );
    }

    #[test]
    fn voices_rotate_and_wrap_at_the_ring_size() {
        let mut seq = HashMap::new();
        let voices: Vec<usize> = (0..POLY_VOICES + 2)
            .map(|_| next_poly_voice(&mut seq, "hit"))
            .collect();
        assert_eq!(
            voices[..POLY_VOICES as usize],
            (0..POLY_VOICES as usize).collect::<Vec<_>>()
        );
        // Bounded, not unbounded: voice 8 reuses slot 0 rather than opening a ninth sink.
        assert_eq!(voices[POLY_VOICES as usize], 0);
        assert_eq!(voices[POLY_VOICES as usize + 1], 1);
    }

    #[test]
    fn a_tone_and_a_clip_under_one_meter_share_the_ring() {
        // `play_tone_poly` and `play_sfx_poly` are two producers on ONE counter, which is what
        // makes a single meter name legal for both. If they kept separate counters, a tone and a
        // clip fired together under the same name would land on the same voice channel and
        // `stop_immediate` would cut one of them — the exact failure this API exists to avoid.
        let mut seq = HashMap::new();
        let taken: Vec<usize> = (0..4)
            .map(|_| next_poly_voice(&mut seq, "impact"))
            .collect();
        assert_eq!(
            taken,
            vec![0, 1, 2, 3],
            "the two paths must not fork the ring"
        );
    }

    #[test]
    fn a_voice_channel_is_private_and_never_the_meter_name() {
        // The meter is a name in `analysis`; the voices are names in `sinks`. They must not
        // collide, or `enable_analysis(meter)` would tap a sink instead of the shared entry (and
        // `stop_channel(meter)` would start cutting voices). The `__poly_` prefix is what keeps a
        // game's own channel named "impact" clear of the voices metering "impact".
        for voice in 0..POLY_VOICES as usize {
            let channel = poly_voice_channel("impact", voice);
            assert_ne!(channel, "impact");
            assert!(channel.starts_with("__poly_"), "{channel}");
        }
    }

    #[test]
    fn each_name_rotates_independently() {
        // Two metered names must not share a counter, or one sound's rate would push the other's
        // voices around and they would start cutting each other.
        let mut seq = HashMap::new();
        assert_eq!(next_poly_voice(&mut seq, "hit"), 0);
        assert_eq!(next_poly_voice(&mut seq, "step"), 0);
        assert_eq!(next_poly_voice(&mut seq, "hit"), 1);
        assert_eq!(next_poly_voice(&mut seq, "step"), 1);
    }

    #[test]
    fn overlapping_voices_sum_into_one_reading() {
        // The documented contract: while several voices sound, `levels()` reports their sum. This
        // is what a single shared slot could not do — `publish` is a plain store, so the reader
        // would have seen only whichever voice published last.
        let mut poly = vec![voice(0.2, 0.3), voice(0.1, 0.25)];
        let combined = combine_voices(AudioLevels::SILENT, &mut poly);
        assert!((combined.rms - 0.3).abs() < 1e-6, "rms {}", combined.rms);
        assert!(
            (combined.peak - 0.55).abs() < 1e-6,
            "peak {}",
            combined.peak
        );
    }

    #[test]
    fn a_drained_voice_stops_contributing_while_others_still_sound() {
        // Staleness is tested per voice, not per name. First tick: both are new, so both count.
        let mut poly = vec![voice(0.2, 0.2), voice(0.3, 0.3)];
        let first = combine_voices(AudioLevels::SILENT, &mut poly);
        assert!((first.rms - 0.5).abs() < 1e-6);

        // Second tick: voice 1 published again (still sounding), voice 0 did not (drained). Only
        // voice 1 may contribute — a per-name staleness test would have kept voice 0 alive.
        poly[1].slot.publish(0.3, 0.3);
        let second = combine_voices(AudioLevels::SILENT, &mut poly);
        assert!(
            (second.rms - 0.3).abs() < 1e-6,
            "a drained voice kept contributing: {}",
            second.rms
        );
    }

    #[test]
    fn a_summed_reading_is_clamped_to_full_scale() {
        // Eight loud voices must not push a meter past 1.0 and a bar off screen.
        let mut poly: Vec<PolyVoice> = (0..POLY_VOICES).map(|_| voice(0.6, 0.9)).collect();
        let combined = combine_voices(AudioLevels::SILENT, &mut poly);
        assert_eq!(combined.rms, 1.0);
        assert_eq!(combined.peak, 1.0);
    }

    #[test]
    fn an_ordinary_channel_is_unaffected_by_the_poly_path() {
        // No voices ⇒ the single reading passes through untouched, so an existing meter behaves
        // exactly as it did before metered one-shots existed.
        let single = AudioLevels {
            rms: 0.42,
            peak: 0.77,
        };
        assert_eq!(combine_voices(single, &mut []), single);
    }

    #[test]
    fn levels_are_clamped_to_full_scale() {
        // An over-unity source must not push a bar off screen.
        let (rms, peak, _) = run_tap(vec![5.0; ANALYSIS_WINDOW as usize]).read();
        assert_eq!(rms, 1.0);
        assert_eq!(peak, 1.0);
    }
}
