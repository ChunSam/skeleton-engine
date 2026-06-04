use super::types::AudioEffect;
use super::AudioManager;

impl AudioManager {
    // ── 오디오 이펙트 ─────────────────────────────────────────────────────────

    /// 채널에 이펙트를 설정한다. 다음 `play_*` 호출 시 적용된다.
    pub fn set_effect(&mut self, channel: &str, effect: AudioEffect) {
        self.effects.insert(channel.to_string(), effect);
    }

    /// 채널의 이펙트를 제거한다.
    pub fn clear_effect(&mut self, channel: &str) {
        self.effects.remove(channel);
    }

    /// 채널의 현재 이펙트를 반환한다.
    pub fn effect(&self, channel: &str) -> Option<&AudioEffect> {
        self.effects.get(channel)
    }
}
