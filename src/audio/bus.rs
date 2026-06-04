use super::types::Fade;
use super::AudioManager;

impl AudioManager {
    // ── 오디오 버스 ───────────────────────────────────────────────────────────

    /// 채널을 버스에 할당한다.
    ///
    /// 예: `assign_bus("bgm", "music")` → `set_bus_volume("music", v)` 로 일괄 제어.
    pub fn assign_bus(&mut self, channel: &str, bus: &str) {
        self.channel_buses
            .insert(channel.to_string(), bus.to_string());
        // 즉시 버스 볼륨 반영
        let eff = self.effective_volume(channel);
        if let Some(sink) = self.sinks.get(channel) {
            sink.set_volume(eff);
        }
    }

    /// 버스 전체 볼륨을 설정한다. 버스에 속한 모든 채널에 즉시 적용된다.
    pub fn set_bus_volume(&mut self, bus: &str, volume: f32) {
        self.bus_volumes
            .insert(bus.to_string(), volume.clamp(0.0, 1.0));
        // 버스에 속한 모든 채널 싱크 업데이트
        let channels: Vec<String> = self
            .channel_buses
            .iter()
            .filter(|(_, b)| b.as_str() == bus)
            .map(|(ch, _)| ch.clone())
            .collect();
        for ch in channels {
            let eff = self.effective_volume(&ch);
            if let Some(sink) = self.sinks.get(&ch) {
                sink.set_volume(eff);
            }
        }
    }

    /// 버스 볼륨을 반환한다 (없으면 1.0).
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.bus_volumes.get(bus).copied().unwrap_or(1.0)
    }

    /// 채널 볼륨을 즉시 설정한다 (0.0 = 무음, 1.0 = 원본).
    /// 버스 볼륨과 곱해진 값이 실제 음량이 된다.
    pub fn set_volume(&mut self, channel: &str, volume: f32) {
        let vol = volume.clamp(0.0, 1.0);
        self.volume_overrides.insert(channel.to_string(), vol);
        let eff = self.effective_volume(channel);
        if let Some(sink) = self.sinks.get(channel) {
            sink.set_volume(eff);
        }
    }

    /// 채널 재생을 `duration_secs` 초에 걸쳐 페이드아웃 후 정지한다.
    ///
    /// 매 프레임 [`update`](Self::update)를 호출해야 동작한다.
    pub fn fade_out(&mut self, channel: &str, duration_secs: f32) {
        let current_vol = self.effective_volume(channel);
        self.fades.insert(
            channel.to_string(),
            Fade {
                start_vol: current_vol,
                target_vol: 0.0,
                duration: duration_secs.max(0.001),
                elapsed: 0.0,
                stop_when_done: true,
            },
        );
    }

    /// 채널 볼륨을 `duration_secs` 초에 걸쳐 `target` 까지 변경한다.
    ///
    /// 매 프레임 [`update`](Self::update)를 호출해야 동작한다.
    pub fn fade_volume(&mut self, channel: &str, target: f32, duration_secs: f32) {
        let current_vol = self.effective_volume(channel);
        self.fades.insert(
            channel.to_string(),
            Fade {
                start_vol: current_vol,
                target_vol: target.clamp(0.0, 1.0),
                duration: duration_secs.max(0.001),
                elapsed: 0.0,
                stop_when_done: false,
            },
        );
    }
}
