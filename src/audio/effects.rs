use super::types::AudioEffect;
use super::AudioManager;
use std::collections::HashMap;

// Keep filter edits independent of the other effect parameters. This is the same
// state mutation used by AudioManager, testable without opening an audio device.
fn update_low_pass(effects: &mut HashMap<String, AudioEffect>, channel: &str, hz: Option<u32>) {
    if let Some(hz) = hz {
        effects.entry(channel.to_owned()).or_default().low_pass_hz = Some(hz);
    } else if let Some(effect) = effects.get_mut(channel) {
        effect.low_pass_hz = None;
    }
}

impl AudioManager {
    pub(crate) fn set_low_pass(&mut self, channel: &str, hz: u32) {
        update_low_pass(&mut self.effects, channel, Some(hz));
    }

    pub(crate) fn clear_low_pass(&mut self, channel: &str) {
        update_low_pass(&mut self.effects, channel, None);
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn configured() -> AudioEffect {
        AudioEffect {
            pitch: 1.5,
            attack_secs: 0.2,
            release_secs: 0.4,
            low_pass_hz: Some(1200),
        }
    }

    fn assert_envelope_and_pitch(effect: &AudioEffect) {
        assert_eq!(effect.pitch, 1.5);
        assert_eq!(effect.attack_secs, 0.2);
        assert_eq!(effect.release_secs, 0.4);
    }

    #[test]
    fn low_pass_set_preserves_other_effects_and_channels() {
        let mut effects = HashMap::from([
            ("tone".to_owned(), configured()),
            ("other".to_owned(), configured()),
        ]);
        update_low_pass(&mut effects, "tone", Some(800));
        assert_eq!(effects["tone"].low_pass_hz, Some(800));
        assert_envelope_and_pitch(&effects["tone"]);
        assert_eq!(effects["other"].low_pass_hz, Some(1200));
        assert_envelope_and_pitch(&effects["other"]);
    }

    #[test]
    fn low_pass_clear_preserves_other_effects_and_channels() {
        let mut effects = HashMap::from([
            ("tone".to_owned(), configured()),
            ("other".to_owned(), configured()),
        ]);
        update_low_pass(&mut effects, "tone", None);
        assert_eq!(effects["tone"].low_pass_hz, None);
        assert_envelope_and_pitch(&effects["tone"]);
        assert_eq!(effects["other"].low_pass_hz, Some(1200));
        update_low_pass(&mut effects, "tone", None);
        assert_envelope_and_pitch(&effects["tone"]);
    }

    #[test]
    fn low_pass_unknown_channel_defaults_only_when_setting() {
        let mut effects = HashMap::new();
        update_low_pass(&mut effects, "missing", None);
        assert!(effects.is_empty());
        update_low_pass(&mut effects, "tone", Some(800));
        let effect = &effects["tone"];
        let defaults = AudioEffect::default();
        assert_eq!(effect.low_pass_hz, Some(800));
        assert_eq!(effect.pitch, defaults.pitch);
        assert_eq!(effect.attack_secs, defaults.attack_secs);
        assert_eq!(effect.release_secs, defaults.release_secs);
    }
}
