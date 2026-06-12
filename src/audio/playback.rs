use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use super::types::Fade;

use rodio::source::SineWave;
use rodio::{Decoder, Sink, Source};

use super::source::PannedSource;
use super::types::{is_finished_state, is_playing_state, playback_state_from_sink};
use super::{AudioChannelState, AudioManager};

impl AudioManager {
    /// Initializes the audio device. Returns `None` on failure; the game continues silently.
    pub fn new() -> Option<Self> {
        use rodio::OutputStream;
        match OutputStream::try_default() {
            Ok((_stream, stream_handle)) => Some(Self {
                _stream,
                stream_handle,
                sinks: HashMap::new(),
                volume_overrides: HashMap::new(),
                pans: HashMap::new(),
                bus_volumes: HashMap::new(),
                channel_buses: HashMap::new(),
                fades: HashMap::new(),
                effects: HashMap::new(),
                file_cache: HashMap::new(),
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
        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(_) => return,
        };
        // Apply bus/channel volume to the sink (so set_bus_volume can update it immediately).
        sink.set_volume(self.effective_volume(channel));

        let base = SineWave::new(freq)
            .take_duration(Duration::from_secs_f32(duration_secs))
            .amplify(volume);

        // SineWave produces f32 samples, so low_pass/speed/fade_in can be applied directly without conversion.
        let source: Box<dyn Source<Item = f32> + Send + 'static> =
            match self.effects.get(channel).cloned() {
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
                None => Box::new(base),
            };
        sink.append(source);
        self.sinks.insert(channel.to_string(), sink);
    }

    /// Advances all active fades. Called every frame by the system.
    ///
    /// Must be called when using `fade_out` / `fade_volume`.
    pub fn update(&mut self, dt: f32) {
        let channels: Vec<String> = self.fades.keys().cloned().collect();
        for ch in channels {
            let done = {
                let fade = self.fades.get_mut(&ch).unwrap();
                fade.elapsed += dt;
                let vol = fade.current_vol();
                if let Some(sink) = self.sinks.get(&ch) {
                    let bus_vol = self
                        .channel_buses
                        .get(&ch)
                        .and_then(|b| self.bus_volumes.get(b))
                        .copied()
                        .unwrap_or(1.0);
                    // update() interpolates the pre-bus volume into the sink directly.
                    // The bus multiplier is applied here (same as the fade constructor
                    // which uses effective_volume = base × bus for start_vol).
                    sink.set_volume(vol * bus_vol);
                }
                let t = (fade.elapsed / fade.duration).clamp(0.0, 1.0);
                if t >= 1.0 {
                    let stop = fade.stop_when_done;
                    // Only persist target_vol for plain fade_volume (stop_when_done==false).
                    // For stop_when_done fades (fade_out / release) the sink is being torn
                    // down; writing 0.0 into volume_overrides would silence the channel's
                    // NEXT play (finding 1).
                    if !stop {
                        self.volume_overrides.insert(ch.clone(), fade.target_vol);
                    }
                    stop
                } else {
                    false
                }
            };
            if done {
                // Use stop_immediate: the fade already ran to completion; no release
                // needed (and calling stop() would re-trigger release for release fades).
                self.fades.remove(&ch);
                self.stop_immediate(&ch);
            }
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

        let sink = match Sink::try_new(&self.stream_handle) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Failed to create audio sink: {e}");
                return;
            }
        };

        let eff_vol = self.effective_volume(channel);
        sink.set_volume(eff_vol);

        let pan = self.pans.get(channel).copied().unwrap_or(0.0);

        // Reuse cached file bytes so replaying the same SFX doesn't re-read the
        // file from disk each shot. Decoding still happens per play (rodio decodes
        // the in-memory bytes), but the disk I/O is paid once per path.
        let bytes = match read_cached_bytes(&mut self.file_cache, path) {
            Some(b) => b,
            None => return,
        };
        let source = match Decoder::new(Cursor::new(bytes)) {
            Ok(s) => s,
            Err(e) => {
                log::warn!("Audio decoding failed for '{path}': {e}");
                return;
            }
        };

        // ── Apply effects ─────────────────────────────────────────────────────
        // Unified as Box<dyn Source<Item=i16> + Send> to reduce type complexity.
        let effect = self.effects.get(channel).cloned();
        let effected: Box<dyn Source<Item = i16> + Send + 'static> = if let Some(eff) = effect {
            if (eff.pitch - 1.0).abs() > 0.001 {
                let s = source.speed(eff.pitch);
                if let Some(hz) = eff.low_pass_hz {
                    let s = s
                        .convert_samples::<f32>()
                        .low_pass(hz)
                        .convert_samples::<i16>();
                    if eff.attack_secs > 0.001 {
                        Box::new(s.fade_in(Duration::from_secs_f32(eff.attack_secs)))
                    } else {
                        Box::new(s)
                    }
                } else if eff.attack_secs > 0.001 {
                    Box::new(
                        s.convert_samples::<i16>()
                            .fade_in(Duration::from_secs_f32(eff.attack_secs)),
                    )
                } else {
                    Box::new(s.convert_samples::<i16>())
                }
            } else if let Some(hz) = eff.low_pass_hz {
                let s = source
                    .convert_samples::<f32>()
                    .low_pass(hz)
                    .convert_samples::<i16>();
                if eff.attack_secs > 0.001 {
                    Box::new(s.fade_in(Duration::from_secs_f32(eff.attack_secs)))
                } else {
                    Box::new(s)
                }
            } else if eff.attack_secs > 0.001 {
                Box::new(source.fade_in(Duration::from_secs_f32(eff.attack_secs)))
            } else {
                Box::new(source)
            }
        } else {
            Box::new(source)
        };

        // ── Apply pan / fade-in / repeat ──────────────────────────────────────
        // BufReader would be more efficient without pan or fade-in, but we unify
        // on the Cursor path here (bytes are already in memory, so the cost is identical).
        if pan.abs() > 0.001 {
            let panned = PannedSource::new(effected.convert_samples::<f32>(), pan);
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

    /// Returns the volume to use as the start of a new fade.
    ///
    /// Uses the current interpolated volume of any in-progress fade so that
    /// chained or mid-fade transitions never produce an audible volume jump
    /// (finding 4 — start-volume pop).  Falls back to `effective_volume` when
    /// no fade is active.
    pub(super) fn fade_start_vol(&self, channel: &str) -> f32 {
        self.fades
            .get(channel)
            .map(|f| f.current_vol())
            .unwrap_or_else(|| self.effective_volume(channel))
    }

    /// Effective volume for a channel = base volume × bus volume.
    pub(super) fn effective_volume(&self, channel: &str) -> f32 {
        let base = self.volume_overrides.get(channel).copied().unwrap_or(1.0);
        self.effective_volume_params(base, channel)
    }

    pub(super) fn effective_volume_params(&self, base: f32, channel: &str) -> f32 {
        let bus_vol = self
            .channel_buses
            .get(channel)
            .and_then(|b| self.bus_volumes.get(b))
            .copied()
            .unwrap_or(1.0);
        base * bus_vol
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
    match std::fs::read(path) {
        Ok(b) => {
            let arc: Arc<[u8]> = Arc::from(b.into_boxed_slice());
            cache.insert(path.to_string(), Arc::clone(&arc));
            Some(arc)
        }
        Err(e) => {
            log::warn!("Cannot open audio file '{path}': {e}");
            None
        }
    }
}
