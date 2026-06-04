use std::collections::HashMap;

use rodio::{OutputStream, OutputStreamHandle, Sink};

mod bus;
mod effects;
mod playback;
mod positional;
mod source;
mod types;

pub use types::{AudioChannelState, AudioEffect};

// ─── AudioManager ─────────────────────────────────────────────────────────────

/// 오디오 재생 관리자 (ECS 리소스로 삽입)
///
/// ## 기본 재생
/// ```rust,no_run
/// # use engine::AudioManager;
/// # let mut am = AudioManager::new().unwrap();
/// am.play("bgm", "assets/music.ogg", true);
/// am.set_volume("bgm", 0.6);
/// am.stop("bgm");
/// ```
///
/// ## 위치 오디오
/// ```rust,no_run
/// # use engine::AudioManager;
/// # use glam::Vec2;
/// # let mut am = AudioManager::new().unwrap();
/// let source_pos = Vec2::new(300.0, 200.0);
/// let listener   = Vec2::new(0.0, 0.0);
/// am.play_at("sfx_hit", "assets/hit.wav", false, source_pos, listener, 500.0);
/// ```
///
/// ## 오디오 버스 (그룹 볼륨)
/// ```rust,no_run
/// # use engine::AudioManager;
/// # let mut am = AudioManager::new().unwrap();
/// am.assign_bus("bgm",      "music");
/// am.assign_bus("sfx_jump", "sfx");
/// am.set_bus_volume("music", 0.5);  // 음악 버스 절반으로
/// ```
pub struct AudioManager {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sinks: HashMap<String, Sink>,
    /// 채널별 기본 볼륨 (버스 볼륨과 곱하기 전)
    volume_overrides: HashMap<String, f32>,
    /// 채널별 스테레오 팬
    pans: HashMap<String, f32>,
    /// 버스 이름 → 볼륨 배율
    bus_volumes: HashMap<String, f32>,
    /// 채널 → 버스 이름
    channel_buses: HashMap<String, String>,
    /// 활성 페이드 상태
    fades: HashMap<String, types::Fade>,
    /// 채널별 오디오 이펙트
    effects: HashMap<String, AudioEffect>,
}

#[cfg(test)]
mod tests;
