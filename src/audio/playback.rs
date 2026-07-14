use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use super::types::Fade;

use rodio::buffer::SamplesBuffer;
use rodio::source::SineWave;
use rodio::{ChannelCount, Decoder, Player, SampleRate, Source};

use super::source::PannedSource;
use super::types::{is_finished_state, is_playing_state, playback_state_from_sink};
use super::{AudioChannelState, AudioManager};

/// De-click envelope for synthesized tones: a short linear attack + release ramp so a `play_tone`
/// beep starts and ends at zero gain instead of clicking. Edge = `min(25% of the tone, 8 ms)`,
/// matching the wasm `WebAudio` tone envelope (`audio_wasm.rs`) for cross-platform parity.
const TONE_ENVELOPE_FRAC: f32 = 0.25;
const TONE_ENVELOPE_MAX_SECS: f32 = 0.008;
const TONE_SAMPLE_RATE: f32 = 48_000.0; // rodio `SineWave` is fixed at 48 kHz mono.

/// `TONE_SAMPLE_RATE` / mono, in the `NonZero` newtypes rodio's buffer constructor takes.
const TONE_RATE: SampleRate = match SampleRate::new(TONE_SAMPLE_RATE as u32) {
    Some(r) => r,
    None => unreachable!(),
};
const TONE_CHANNELS: ChannelCount = ChannelCount::MIN; // 1 = mono

/// Synthesizes a mono 48 kHz sine of `freq` for `duration_secs` at `volume`, with a linear
/// attack/release envelope so the tone does not click on or off. The sine body mirrors rodio's
/// [`SineWave`] phase (`sin(2π·freq·(i+1)/48000)`), so apart from the edge ramp the samples are
/// identical to the previous un-enveloped tone.
fn enveloped_tone_samples(freq: f32, duration_secs: f32, volume: f32) -> Vec<f32> {
    let n = (duration_secs.max(0.0) * TONE_SAMPLE_RATE).round() as usize;
    let edge = (duration_secs * TONE_ENVELOPE_FRAC).clamp(0.0, TONE_ENVELOPE_MAX_SECS);
    let edge_n = (edge * TONE_SAMPLE_RATE).round() as usize;
    let mut buf = Vec::with_capacity(n);
    for i in 0..n {
        let phase = 2.0 * std::f32::consts::PI * freq * (i as f32 + 1.0) / TONE_SAMPLE_RATE;
        let mut s = phase.sin() * volume;
        if edge_n > 0 {
            // Linear attack over the first `edge_n` samples (0 → full gain) …
            if i < edge_n {
                s *= i as f32 / edge_n as f32;
            }
            // … and a symmetric linear release over the last `edge_n` (full → 0).
            let from_end = n.saturating_sub(1).saturating_sub(i);
            if from_end < edge_n {
                s *= from_end as f32 / edge_n as f32;
            }
        }
        buf.push(s);
    }
    buf
}

impl AudioManager {
    /// Initializes the audio device. Returns `None` on failure; the game continues silently.
    pub fn new() -> Option<Self> {
        match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(stream) => Some(Self {
                stream,
                sinks: HashMap::new(),
                volume_overrides: HashMap::new(),
                pans: HashMap::new(),
                bus_volumes: HashMap::new(),
                channel_buses: HashMap::new(),
                fades: HashMap::new(),
                effects: HashMap::new(),
                file_cache: HashMap::new(),
                bus_ducks: HashMap::new(),
                sidechains: Vec::new(),
            }),
            Err(e) => {
                log::warn!("Audio initialization failed (running without audio): {e}");
                None
            }
        }
    }

    // ── Basic playback ────────────────────────────────────────────────────────

    /// Plays an audio file on a channel. Stops any existing playback on that channel first.
    pub fn play(&mut self, channel: &str, path: &str, repeat: bool) {
        self.play_internal(channel, path, repeat, None);
    }

    /// Plays with a fade-in applied.
    pub fn play_fade_in(&mut self, channel: &str, path: &str, repeat: bool, fade_secs: f32) {
        self.play_internal(channel, path, repeat, Some(fade_secs));
    }

    /// Stops playback on a channel, honoring the release envelope.
    ///
    /// If the channel has an [`AudioEffect`](crate::AudioEffect) with `release_secs > 0.0`,
    /// no `stop_when_done` fade is already active, and the sink still has audio queued,
    /// the engine schedules a release fade from the *current interpolated volume* to zero
    /// over `release_secs` seconds.  The sink is torn down only after that fade completes.
    ///
    /// In all other cases the sink is torn down immediately:
    /// - `release_secs == 0.0` or no effect configured.
    /// - A `stop_when_done` fade (release *or* `fade_out`) is already active — this
    ///   second `stop` cuts through it immediately.
    /// - The sink has already drained (naturally finished).
    ///
    /// Requires [`AudioSystem`](crate::AudioSystem) (or manual
    /// [`update`](Self::update) calls) for the release fade to progress.
    pub fn stop(&mut self, channel: &str) {
        let release = self
            .effects
            .get(channel)
            .map(|e| e.release_secs)
            .unwrap_or(0.0);

        // Cut immediately if any stop_when_done fade is already active (covers both
        // release fades and explicit fade_out calls — finding 3).
        let stop_when_done_active = self
            .fades
            .get(channel)
            .map(|f| f.stop_when_done)
            .unwrap_or(false);

        // A naturally-finished sink has no audio to release — cut immediately (finding 5).
        let sink_has_audio = self.sinks.get(channel).map(|s| !s.empty()).unwrap_or(false);

        if release > 0.001 && !stop_when_done_active && sink_has_audio {
            // Use the current interpolated fade volume as start_vol so there is no
            // audible jump when stop() is called mid-fade_volume (finding 4).
            let start_vol = self.fade_start_vol(channel);
            self.fades
                .insert(channel.to_string(), Fade::stop_fade(start_vol, release));
        } else {
            self.stop_immediate(channel);
        }
    }

    /// Tears down a channel's sink immediately, bypassing any release envelope.
    ///
    /// Used internally by `play_*` (channel reuse) and by `update()` when a
    /// `stop_when_done` fade (including a release fade) completes.
    pub(super) fn stop_immediate(&mut self, channel: &str) {
        self.fades.remove(channel);
        if let Some(sink) = self.sinks.remove(channel) {
            sink.stop();
        }
    }

    /// Returns the playback state for a channel.
    ///
    /// A missing channel reports [`AudioChannelState::Missing`]. A non-looping
    /// sound that naturally reaches the end remains queryable as
    /// [`AudioChannelState::Finished`] until the channel is stopped or reused.
    pub fn playback_state(&self, channel: &str) -> AudioChannelState {
        playback_state_from_sink(self.sinks.get(channel))
    }

    /// Returns whether a channel has finished playback.
    ///
    /// `None` means the channel has no sink. `Some(false)` means it still has
    /// audio queued. `Some(true)` means it exists and has drained.
    pub fn is_finished(&self, channel: &str) -> Option<bool> {
        is_finished_state(self.playback_state(channel))
    }

    /// Returns true when a channel exists and still has queued audio.
    pub fn is_playing(&self, channel: &str) -> bool {
        is_playing_state(self.playback_state(channel))
    }

    /// Plays a pure sine-wave tone.
    ///
    /// `volume` is the amplitude of the tone itself. The bus volume for the channel
    /// is multiplied in via the sink volume (`effective_volume`), same as `play_internal`.
    /// Channel effects set via `set_effect` (low-pass, pitch, fade-in) are also applied.
    pub fn play_tone(&mut self, channel: &str, freq: f32, duration_secs: f32, volume: f32) {
        // Channel reuse: tear down immediately (same as play_internal).
        self.stop_immediate(channel);
        let sink = Player::connect_new(self.stream.mixer());
        // Apply bus/channel volume to the sink (so set_bus_volume can update it immediately).
        sink.set_volume(self.effective_volume(channel));

        let base = SineWave::new(freq)
            .take_duration(Duration::from_secs_f32(duration_secs))
            .amplify(volume);

        let source: Box<dyn Source + Send + 'static> = match self.effects.get(channel).cloned() {
            Some(eff) => {
                let pitch = if eff.pitch > 0.0 { eff.pitch } else { 1.0 };
                let s = base.speed(pitch);
                match (eff.low_pass_hz, eff.attack_secs) {
                    (Some(hz), a) if a > 0.001 => {
                        Box::new(s.low_pass(hz).fade_in(Duration::from_secs_f32(a)))
                    }
                    (Some(hz), _) => Box::new(s.low_pass(hz)),
                    (None, a) if a > 0.001 => Box::new(s.fade_in(Duration::from_secs_f32(a))),
                    (None, _) => Box::new(s),
                }
            }
            // No channel effect: emit the tone with a default de-click envelope (attack +
            // release) so a plain `play_tone` beep doesn't click on/off. The envelope is
            // materialized into a buffer rather than composed from source combinators so that
            // `enveloped_tone_samples` stays assertable on the CPU (see the golden phase test).
            None => Box::new(SamplesBuffer::new(
                TONE_CHANNELS,
                TONE_RATE,
                enveloped_tone_samples(freq, duration_secs, volume),
            )),
        };
        sink.append(source);
        self.sinks.insert(channel.to_string(), sink);
    }

    /// Advances all active fades and duck/sidechain animations. Called every frame by the system.
    ///
    /// Must be called when using `fade_out` / `fade_volume` / `duck_bus` / `set_sidechain`.
    pub fn update(&mut self, dt: f32) {
        // ── Sidechain evaluation + duck progression ───────────────────────────
        self.evaluate_sidechains();
        self.progress_ducks(dt);

        // ── Fade progression ──────────────────────────────────────────────────
        // Collect channel keys once; use `smallvec`-style stack buffer to avoid a
        // heap allocation on the common case of ≤ 4 simultaneous fades. We fall
        // back to a Vec for larger counts. Using a plain array avoids re-borrowing
        // `self.fades` inside the loop.
        //
        // Note: we intentionally do NOT rewrite ducking.rs allocations here —
        // those loops are less hot and the refactor risk outweighs the gain.
        let mut channel_buf: [Option<String>; 8] = Default::default();
        let mut overflow: Vec<String> = Vec::new();
        let mut buf_len = 0usize;
        for ch in self.fades.keys() {
            if buf_len < channel_buf.len() {
                channel_buf[buf_len] = Some(ch.clone());
                buf_len += 1;
            } else {
                overflow.push(ch.clone());
            }
        }

        let iter_stack = channel_buf[..buf_len].iter().filter_map(|o| o.as_deref());
        let iter_overflow = overflow.iter().map(|s| s.as_str());

        let mut to_stop: Vec<String> = Vec::new(); // only populated when a fade completes
        for ch in iter_stack.chain(iter_overflow) {
            let done = {
                // `ch` was collected from `self.fades` just above; guard anyway so a future
                // change that removes a fade mid-loop can't turn this into a panic.
                let Some(fade) = self.fades.get_mut(ch) else {
                    continue;
                };
                fade.elapsed += dt;
                let vol = fade.current_vol();
                if let Some(sink) = self.sinks.get(ch) {
                    let bus = self.channel_buses.get(ch);
                    let bus_vol = bus
                        .and_then(|b| self.bus_volumes.get(b))
                        .copied()
                        .unwrap_or(1.0);
                    let duck = bus
                        .and_then(|b| self.bus_ducks.get(b))
                        .map(|d| d.current)
                        .unwrap_or(1.0);
                    // Fades store/interpolate the pre-bus (base) volume. The bus
                    // multiplier and duck factor are applied exactly once here, so the
                    // sink receives `base_vol × bus_vol × duck` — never squared.
                    sink.set_volume(vol * bus_vol * duck);
                }
                let t = (fade.elapsed / fade.duration).clamp(0.0, 1.0);
                if t >= 1.0 {
                    let stop = fade.stop_when_done;
                    // Only persist target_vol for plain fade_volume (stop_when_done==false).
                    // For stop_when_done fades (fade_out / release) the sink is being torn
                    // down; writing 0.0 into volume_overrides would silence the channel's
                    // NEXT play (finding 1).
                    if !stop {
                        self.volume_overrides.insert(ch.to_owned(), fade.target_vol);
                    }
                    stop
                } else {
                    false
                }
            };
            if done {
                to_stop.push(ch.to_owned());
            }
        }
        for ch in to_stop {
            // Use stop_immediate: the fade already ran to completion; no release
            // needed (and calling stop() would re-trigger release for release fades).
            self.fades.remove(&ch);
            self.stop_immediate(&ch);
        }
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    pub(super) fn play_internal(
        &mut self,
        channel: &str,
        path: &str,
        repeat: bool,
        fade_in_secs: Option<f32>,
    ) {
        // Channel reuse: tear down the old sink immediately regardless of release_secs.
        // The new sound must start cleanly without waiting for a release fade.
        self.stop_immediate(channel);

        let sink = Player::connect_new(self.stream.mixer());

        let eff_vol = self.effective_volume(channel);
        sink.set_volume(eff_vol);

        // Reuse cached file bytes so replaying the same SFX doesn't re-read the
        // file from disk each shot. Decoding still happens per play (rodio decodes
        // the in-memory bytes), but the disk I/O is paid once per path. The old sink
        // is torn down above before this read, so a failed read leaves the channel
        // silent (the prior behavior — preserved by reading before `append_decoded`).
        let bytes = match read_cached_bytes(&mut self.file_cache, path) {
            Some(b) => b,
            None => return,
        };
        self.append_decoded(channel, sink, bytes, repeat, fade_in_secs);
    }

    /// Plays already-in-memory encoded audio `bytes` on a channel — the byte-slice analogue of
    /// [`play`](Self::play). Use this for audio embedded with `include_bytes!`, which is the
    /// cross-platform clip source (wasm has no filesystem, so there is no path to read). Stops
    /// any existing playback on the channel first, exactly like [`play`](Self::play).
    ///
    /// Backs the cross-platform [`Audio`](crate::Audio) facade's `play_sfx`/`play_music`.
    pub fn play_bytes(&mut self, channel: &str, bytes: &[u8], repeat: bool) {
        self.play_bytes_internal(channel, Arc::from(bytes), repeat, None);
    }

    /// Shared byte-playback entry: tears down the old sink, creates + volume-sets a new one, then
    /// decodes and appends `bytes`. The byte counterpart of [`play_internal`]'s tail; used by
    /// `play_bytes` and `crossfade_bytes`.
    pub(super) fn play_bytes_internal(
        &mut self,
        channel: &str,
        bytes: Arc<[u8]>,
        repeat: bool,
        fade_in_secs: Option<f32>,
    ) {
        self.stop_immediate(channel);
        let sink = Player::connect_new(self.stream.mixer());
        sink.set_volume(self.effective_volume(channel));
        self.append_decoded(channel, sink, bytes, repeat, fade_in_secs);
    }

    /// Shared decode → effects → pan/fade/repeat → insert tail of `play_internal` and
    /// `play_bytes_internal`. The caller has already torn down the old sink, created the new one,
    /// and set its base×bus volume; this reads the channel pan, decodes `bytes`, and appends the
    /// (effected) source.
    fn append_decoded(
        &mut self,
        channel: &str,
        sink: Player,
        bytes: Arc<[u8]>,
        repeat: bool,
        fade_in_secs: Option<f32>,
    ) {
        let pan = self.pans.get(channel).copied().unwrap_or(0.0);
        let source = match Decoder::new(Cursor::new(bytes)) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Audio decoding failed for channel '{channel}': {e}");
                return;
            }
        };

        // ── Apply effects ─────────────────────────────────────────────────────
        // Applied in the order speed → low-pass → fade-in. Decoded samples are f32
        // throughout, so the stages compose directly.
        let effect = self.effects.get(channel).cloned();
        let effected: Box<dyn Source + Send + 'static> = if let Some(eff) = effect {
            let pitched: Box<dyn Source + Send + 'static> = if (eff.pitch - 1.0).abs() > 0.001 {
                Box::new(source.speed(eff.pitch))
            } else {
                Box::new(source)
            };
            let filtered: Box<dyn Source + Send + 'static> = match eff.low_pass_hz {
                Some(hz) => Box::new(pitched.low_pass(hz)),
                None => pitched,
            };
            if eff.attack_secs > 0.001 {
                Box::new(filtered.fade_in(Duration::from_secs_f32(eff.attack_secs)))
            } else {
                filtered
            }
        } else {
            Box::new(source)
        };

        // ── Apply pan / fade-in / repeat ──────────────────────────────────────
        // BufReader would be more efficient without pan or fade-in, but we unify
        // on the Cursor path here (bytes are already in memory, so the cost is identical).
        if pan.abs() > 0.001 {
            let panned = PannedSource::new(effected, pan);
            if let Some(fade_dur) = fade_in_secs {
                let faded = panned.fade_in(Duration::from_secs_f32(fade_dur));
                if repeat {
                    sink.append(faded.repeat_infinite());
                } else {
                    sink.append(faded);
                }
            } else if repeat {
                sink.append(panned.repeat_infinite());
            } else {
                sink.append(panned);
            }
        } else if let Some(fade_dur) = fade_in_secs {
            let faded = effected.fade_in(Duration::from_secs_f32(fade_dur));
            if repeat {
                sink.append(faded.repeat_infinite());
            } else {
                sink.append(faded);
            }
        } else if repeat {
            sink.append(effected.repeat_infinite());
        } else {
            sink.append(effected);
        }

        self.sinks.insert(channel.to_string(), sink);
    }

    /// Crossfades from the current track on `channel` to a new one.
    ///
    /// The old track (if any) fades **out** over `dur` seconds on a temporary
    /// internal channel (`"<channel>__xfade"`) while the new track simultaneously
    /// fades **in** over `dur` seconds on `channel`. The two sounds overlap for a
    /// true crossfade rather than a cut.
    ///
    /// When the fade-out completes, `AudioSystem` (via `update`) automatically
    /// tears down the temporary channel — no manual cleanup is required.
    ///
    /// If nothing is currently playing on `channel`, this is equivalent to
    /// [`play_fade_in`](Self::play_fade_in).
    ///
    /// # Arguments
    /// * `channel` — logical channel name for the *new* track.
    /// * `new_path` — audio file path for the new track.
    /// * `repeat`   — whether the new track loops.
    /// * `dur`      — crossfade duration in seconds (both fade-out and fade-in).
    ///
    /// # Example
    /// ```rust,no_run
    /// # use engine::AudioManager;
    /// # let mut am = AudioManager::new().unwrap();
    /// am.play("bgm", "assets/track_a.ogg", true);
    /// // … later …
    /// am.crossfade("bgm", "assets/track_b.ogg", true, 2.0);
    /// ```
    pub fn crossfade(&mut self, channel: &str, new_path: &str, repeat: bool, dur: f32) {
        self.begin_crossfade(channel, dur);
        // Start the new track on `channel` with a fade-in.
        self.play_fade_in(channel, new_path, repeat, dur);
    }

    /// Byte-slice analogue of [`crossfade`](Self::crossfade) for `include_bytes!` audio (the
    /// cross-platform clip source). Relocates the current track to a temp channel that fades out
    /// while the new `bytes` fade in over `dur` seconds. If nothing is playing, this is just a
    /// fade-in. Backs the cross-platform [`Audio`](crate::Audio) facade's `crossfade_music`.
    pub fn crossfade_bytes(&mut self, channel: &str, bytes: &[u8], repeat: bool, dur: f32) {
        self.begin_crossfade(channel, dur);
        self.play_bytes_internal(channel, Arc::from(bytes), repeat, Some(dur));
    }

    /// Relocates the track currently on `channel` (if any) to a temp channel scheduled to fade out
    /// over `dur` seconds, so a freshly-started track on `channel` can fade in for a true overlap.
    /// Shared by [`crossfade`](Self::crossfade) and [`crossfade_bytes`](Self::crossfade_bytes).
    fn begin_crossfade(&mut self, channel: &str, dur: f32) {
        let temp = format!("{channel}__xfade");

        // If something is currently on `channel`, relocate it to the temp channel
        // so the two sinks overlap during the crossfade.
        if let Some(old_sink) = self.sinks.remove(channel) {
            // Carry over the existing fade (if any) so the volume ramp starts
            // from the correct interpolated level rather than jumping.
            let old_fade = self.fades.remove(channel);
            // Transfer the sink and any in-progress fade to the temp channel.
            // Stop whatever was already there (shouldn't exist in normal use).
            self.stop_immediate(&temp);
            self.sinks.insert(temp.clone(), old_sink);
            if let Some(f) = old_fade {
                self.fades.insert(temp.clone(), f);
            }

            // Schedule a stop-when-done fade-out on the temp channel.
            let start_vol = self.fade_start_vol(&temp);
            self.fades.insert(temp, Fade::stop_fade(start_vol, dur));
        }
    }

    /// Clears the in-memory file-bytes cache.
    ///
    /// The cache grows unbounded as new paths are played (one entry per unique path). Call
    /// this when a level unloads to free the retained audio bytes. The next `play` call for
    /// a previously-cached path will re-read the file from disk.
    pub fn clear_file_cache(&mut self) {
        self.file_cache.clear();
    }

    /// Returns the **pre-bus** base volume to use as the start of a new fade.
    ///
    /// Fades store and interpolate the pre-bus (base) volume.  `update()` applies
    /// the bus multiplier exactly once when writing to the sink (`vol * bus_vol`).
    /// This avoids the double-multiply that would occur if the bus factor were baked
    /// into `start_vol`/`target_vol` AND applied again in `update()`.
    ///
    /// Uses the current interpolated (pre-bus) volume of any in-progress fade so
    /// that chained or mid-fade transitions never produce an audible volume jump
    /// (finding 4 — start-volume pop).  Falls back to the channel's base volume
    /// (pre-bus) when no fade is active.
    pub(super) fn fade_start_vol(&self, channel: &str) -> f32 {
        self.fades
            .get(channel)
            .map(|f| f.current_vol())
            .unwrap_or_else(|| self.volume_overrides.get(channel).copied().unwrap_or(1.0))
    }

    /// Effective volume for a channel = base volume × bus volume.
    pub(super) fn effective_volume(&self, channel: &str) -> f32 {
        let base = self.volume_overrides.get(channel).copied().unwrap_or(1.0);
        self.effective_volume_params(base, channel)
    }

    pub(super) fn effective_volume_params(&self, base: f32, channel: &str) -> f32 {
        let bus = self.channel_buses.get(channel);
        let bus_vol = bus
            .and_then(|b| self.bus_volumes.get(b))
            .copied()
            .unwrap_or(1.0);
        let duck = bus
            .and_then(|b| self.bus_ducks.get(b))
            .map(|d| d.current)
            .unwrap_or(1.0);
        base * bus_vol * duck
    }
}

/// Return the cached encoded bytes for `path`, reading (and caching) from disk on
/// the first request. Returns `None` (with a warning) if the file can't be read.
///
/// Kept as a free function — independent of the audio device — so it is unit
/// testable in headless CI where `AudioManager::new()` returns `None`.
pub(super) fn read_cached_bytes(
    cache: &mut HashMap<String, Arc<[u8]>>,
    path: &str,
) -> Option<Arc<[u8]>> {
    if let Some(bytes) = cache.get(path) {
        return Some(Arc::clone(bytes));
    }
    // Resolved only for the read; the cache stays keyed by the caller's path.
    match std::fs::read(crate::asset_path::resolve(path)) {
        Ok(b) => {
            let arc: Arc<[u8]> = Arc::from(b.into_boxed_slice());
            cache.insert(path.to_string(), Arc::clone(&arc));
            Some(arc)
        }
        Err(e) => {
            crate::asset_path::record_failure(path, format!("audio file read failed: {e}"));
            None
        }
    }
}

#[cfg(test)]
mod tone_envelope_tests {
    use super::enveloped_tone_samples;

    #[test]
    fn tone_envelope_ramps_to_zero_at_both_ends() {
        let buf = enveloped_tone_samples(440.0, 0.1, 1.0);
        assert!(!buf.is_empty());
        // De-click: the tone starts and ends at (near) zero gain instead of an abrupt edge.
        assert!(
            buf[0].abs() < 1e-6,
            "tone must start at zero gain, got {}",
            buf[0]
        );
        assert!(
            buf[buf.len() - 1].abs() < 1e-6,
            "tone must end at zero gain, got {}",
            buf[buf.len() - 1]
        );
        // The envelope only touches the edges — the body still peaks near full amplitude.
        let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(
            peak > 0.9,
            "enveloped tone should still peak near 1.0, got {peak}"
        );
    }

    #[test]
    fn tone_sample_count_matches_duration() {
        // 0.1 s @ 48 kHz mono = 4800 samples; a zero-length tone is empty.
        assert_eq!(enveloped_tone_samples(440.0, 0.1, 1.0).len(), 4800);
        assert!(enveloped_tone_samples(440.0, 0.0, 1.0).is_empty());
    }
}
