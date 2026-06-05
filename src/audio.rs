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
    /// 경로 → 인코딩된 파일 바이트 캐시. 같은 SFX 를 다시 재생할 때 디스크에서
    /// 다시 읽지 않도록 한다 (`play`/`play_internal` 경로에서만 사용; 대용량 BGM 용
    /// `play_streaming` 은 스트리밍하므로 캐시하지 않는다).
    file_cache: HashMap<String, std::sync::Arc<[u8]>>,
}

// ─── AudioSystem ──────────────────────────────────────────────────────────────

/// 매 프레임 오디오 페이드를 진행시키는 빌트인 시스템.
///
/// `AudioManager::fade_out` / `fade_volume` 는 페이드를 *예약*만 하며, 실제 진행은
/// `AudioManager::update(dt)` 가 매 프레임 호출돼야 일어난다. 이 시스템을 등록하면
/// 페이드가 자동으로 동작한다. (엔진의 다른 빌트인 시스템들처럼 사용자가 명시적으로
/// 추가한다.)
///
/// ```rust,no_run
/// # use engine::{App, AudioManager, AudioSystem};
/// let mut app = App::new();
/// if let Some(audio) = AudioManager::new() {
///     app.world.insert_resource(audio);
/// }
/// app.add_system(AudioSystem);
/// ```
///
/// `AudioManager` 리소스가 없으면 아무 일도 하지 않는다.
#[derive(Default)]
pub struct AudioSystem;

impl crate::ecs::System for AudioSystem {
    fn run(&mut self, world: &mut crate::ecs::World, dt: f32) {
        if let Some(audio) = world.resource_mut::<AudioManager>() {
            audio.update(dt);
        }
    }
}

#[cfg(test)]
mod tests;
