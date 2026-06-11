use rodio::Sink;

// ─── Audio effects ────────────────────────────────────────────────────────────

/// Per-channel audio effect settings.
/// Automatically applied on the next `play_*` call after `set_effect()`.
#[derive(Debug, Clone)]
pub struct AudioEffect {
    /// Low-pass cutoff frequency (Hz). `None` = no filter.
    pub low_pass_hz: Option<u32>,
    /// Playback speed multiplier (proportional to pitch). 1.0 = original speed.
    pub pitch: f32,
    /// Fade-in duration at playback start (seconds). 0.0 = immediate.
    pub attack_secs: f32,
    /// Volume envelope release duration (seconds). 0.0 = immediate stop.
    ///
    /// When a channel that has `release_secs > 0.0` is stopped via
    /// [`AudioManager::stop`](crate::audio::AudioManager::stop), the engine fades its
    /// volume from the *current interpolated level* to zero over `release_secs` seconds
    /// and only then tears down the sink.  The channel reports
    /// [`AudioChannelState::Playing`] via
    /// [`playback_state`](crate::audio::AudioManager::playback_state) while the release
    /// fade is in progress — the audio is still audible.  Once the fade finishes the
    /// channel becomes [`AudioChannelState::Missing`] and the channel's base volume is
    /// restored to the value that was set before the fade (not locked to zero).
    ///
    /// **Stop paths that honor release (`release_secs > 0.0`):**
    /// - [`AudioManager::stop`](crate::audio::AudioManager::stop) — schedules the
    ///   release fade instead of cutting immediately, **provided** no
    ///   `stop_when_done` fade (release or `fade_out`) is already active on the
    ///   channel and the sink still has audio queued.
    ///
    /// **Stop paths that bypass release (always immediate):**
    /// - A new `play_*` call on the same channel — the old sound is cut immediately
    ///   so the new sound starts without delay.
    /// - [`AudioManager::stop`](crate::audio::AudioManager::stop) called while the
    ///   channel already has a `stop_when_done` fade active (release *or* `fade_out`)
    ///   — the second `stop` cuts immediately.  This means calling `stop` after
    ///   `fade_out` has been scheduled also cuts immediately.
    /// - [`AudioManager::stop`](crate::audio::AudioManager::stop) called on a channel
    ///   whose sink has already drained (naturally finished) — no release needed.
    /// - [`AudioManager::fade_out`](crate::audio::AudioManager::fade_out) — the caller
    ///   is explicitly controlling the fade duration; release is not added on top.
    /// - `AudioManager::stop_immediate` — always cuts, regardless of the release setting.
    ///
    /// Requires [`AudioSystem`](crate::audio::AudioSystem) (or manual
    /// [`AudioManager::update`](crate::audio::AudioManager::update) calls) for the fade
    /// to progress.  Without it the release fade is scheduled but never advances.
    pub release_secs: f32,
}

impl Default for AudioEffect {
    fn default() -> Self {
        Self {
            low_pass_hz: None,
            pitch: 1.0,
            attack_secs: 0.0,
            release_secs: 0.0,
        }
    }
}

/// Public playback state for an [`AudioManager`](crate::audio::AudioManager) channel.
///
/// Natural completion of a non-looping sound leaves the channel in
/// [`Finished`](Self::Finished) until another sound is played on that channel or
/// [`AudioManager::stop`](crate::audio::AudioManager::stop) removes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioChannelState {
    /// No sink exists for this channel. The channel was never played, failed to
    /// play, or was explicitly stopped.
    Missing,
    /// A sink exists and still has audio queued.
    Playing,
    /// A sink exists, but its queue has drained.
    Finished,
}

pub(crate) fn playback_state_from_sink(sink: Option<&Sink>) -> AudioChannelState {
    match sink {
        Some(sink) if sink.empty() => AudioChannelState::Finished,
        Some(_) => AudioChannelState::Playing,
        None => AudioChannelState::Missing,
    }
}

pub(crate) fn is_finished_state(state: AudioChannelState) -> Option<bool> {
    match state {
        AudioChannelState::Missing => None,
        AudioChannelState::Playing => Some(false),
        AudioChannelState::Finished => Some(true),
    }
}

pub(crate) fn is_playing_state(state: AudioChannelState) -> bool {
    state == AudioChannelState::Playing
}

// ─── Fade state ───────────────────────────────────────────────────────────────

pub(super) struct Fade {
    pub(super) start_vol: f32,
    pub(super) target_vol: f32,
    pub(super) duration: f32,
    pub(super) elapsed: f32,
    /// Whether to stop (tear down) the sink when the fade completes.
    ///
    /// `true`  for `fade_out` and release fades (scheduled by `stop()`).
    /// `false` for `fade_volume` (plain volume change — sink is kept alive).
    ///
    /// `stop()` gates on this flag: if a `stop_when_done` fade is already active
    /// (either a release or an explicit `fade_out`) a second `stop()` cuts
    /// immediately instead of scheduling another release fade.  This replaces the
    /// old `releasing: HashSet` shadow state.
    pub(super) stop_when_done: bool,
}

impl Fade {
    /// Construct a *stop-when-done* fade (used by both `stop()` and `fade_out()`).
    ///
    /// `duration` is clamped to `≥ 0.001 s` so the scheduler always has at least
    /// one tick to run.
    pub(super) fn stop_fade(start_vol: f32, duration: f32) -> Self {
        Self {
            start_vol,
            target_vol: 0.0,
            duration: duration.max(0.001),
            elapsed: 0.0,
            stop_when_done: true,
        }
    }

    /// Returns the linearly interpolated volume at the current elapsed time.
    pub(super) fn current_vol(&self) -> f32 {
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        self.start_vol + (self.target_vol - self.start_vol) * t
    }
}
