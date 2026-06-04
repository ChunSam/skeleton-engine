use glam::Vec2;

use super::AudioManager;

impl AudioManager {
    // ── 위치 오디오 ───────────────────────────────────────────────────────────

    /// 2D 공간의 `source_pos`에서 소리를 재생한다.
    ///
    /// - 거리(`source_pos`와 `listener_pos` 사이)가 `max_dist` 이상이면 무음.
    /// - X 방향 차이로 스테레오 팬을 자동 계산한다.
    pub fn play_at(
        &mut self,
        channel: &str,
        path: &str,
        repeat: bool,
        source_pos: Vec2,
        listener: Vec2,
        max_dist: f32,
    ) {
        let (vol, pan) = Self::spatial_params(source_pos, listener, max_dist);
        self.volume_overrides.insert(channel.to_string(), vol);
        self.pans.insert(channel.to_string(), pan);
        self.play(channel, path, repeat);
    }

    /// 이미 재생 중인 채널의 공간 위치를 실시간으로 업데이트한다.
    ///
    /// ECS 시스템에서 매 프레임 호출해 움직이는 소리 발생원에 적용한다.
    pub fn update_position(
        &mut self,
        channel: &str,
        source_pos: Vec2,
        listener: Vec2,
        max_dist: f32,
    ) {
        let (vol, pan) = Self::spatial_params(source_pos, listener, max_dist);
        self.volume_overrides.insert(channel.to_string(), vol);
        self.pans.insert(channel.to_string(), pan);
        if let Some(sink) = self.sinks.get(channel) {
            sink.set_volume(self.effective_volume_params(vol, channel));
        }
    }

    // ── 볼륨 / 팬 ────────────────────────────────────────────────────────────

    /// 채널 스테레오 팬을 설정한다 (-1.0 = 좌, 0.0 = 중앙, 1.0 = 우).
    /// 다음 `play()` 호출부터 적용된다.
    pub fn set_pan(&mut self, channel: &str, pan: f32) {
        self.pans.insert(channel.to_string(), pan.clamp(-1.0, 1.0));
    }

    /// 소리 발생 위치와 리스너 위치로부터 (볼륨, 팬)을 계산한다.
    pub(super) fn spatial_params(source_pos: Vec2, listener: Vec2, max_dist: f32) -> (f32, f32) {
        let delta = source_pos - listener;
        let dist = delta.length();
        let volume = (1.0 - (dist / max_dist.max(0.001)).min(1.0)).max(0.0);
        let pan = (delta.x / max_dist.max(0.001)).clamp(-1.0, 1.0);
        (volume, pan)
    }
}
