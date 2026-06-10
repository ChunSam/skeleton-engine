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
    /// Volume envelope release duration (seconds). 0.0 = unlimited.
    ///
    /// **Not yet applied by the engine.** This field is reserved for a future
    /// release-envelope implementation. Setting it has no effect in the current version.
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
    /// Whether to stop the sink when the fade completes (true for fade_out).
    pub(super) stop_when_done: bool,
}
