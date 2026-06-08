use super::types::AudioEffect;
use super::AudioManager;

impl AudioManager {
    // ── Audio effects ─────────────────────────────────────────────────────────

    /// Sets an effect on a channel. Applied on the next `play_*` call.
    pub fn set_effect(&mut self, channel: &str, effect: AudioEffect) {
        self.effects.insert(channel.to_string(), effect);
    }

    /// Removes the effect from a channel.
    pub fn clear_effect(&mut self, channel: &str) {
        self.effects.remove(channel);
    }

    /// Returns the current effect on a channel.
    pub fn effect(&self, channel: &str) -> Option<&AudioEffect> {
        self.effects.get(channel)
    }
}
