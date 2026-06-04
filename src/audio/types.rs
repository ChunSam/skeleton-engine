use rodio::Sink;

// ─── 오디오 이펙트 ────────────────────────────────────────────────────────────

/// 채널별 오디오 이펙트 설정.
/// `set_effect()` 후 다음 `play_*` 호출 시 자동 적용된다.
#[derive(Debug, Clone)]
pub struct AudioEffect {
    /// 로우패스 컷오프 주파수 (Hz). `None` = 필터 없음.
    pub low_pass_hz: Option<u32>,
    /// 재생 속도 배율 (피치 비례). 1.0 = 원속도.
    pub pitch: f32,
    /// 재생 시작 시 페이드인 시간 (초). 0.0 = 즉시.
    pub attack_secs: f32,
    /// 볼륨 엔벨로프 지속 시간 (초). 0.0 = 무제한.
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

// ─── 페이드 상태 ──────────────────────────────────────────────────────────────

pub(super) struct Fade {
    pub(super) start_vol: f32,
    pub(super) target_vol: f32,
    pub(super) duration: f32,
    pub(super) elapsed: f32,
    /// 페이드 완료 시 싱크를 정지할지 여부 (fade_out에서 true)
    pub(super) stop_when_done: bool,
}
