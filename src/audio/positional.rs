use glam::Vec2;

use super::AudioManager;

impl AudioManager {
    // ── Positional audio ──────────────────────────────────────────────────

    /// Plays a sound at `source_pos` in 2D space.
    ///
    /// - Silent when the distance between `source_pos` and `listener_pos` reaches `max_dist`.
    /// - Stereo pan is computed automatically from the X-axis difference.
    /// - Distance attenuation multiplies the volume set by `set_volume`; neither overwrites
    ///   the other. Both settings persist across stop/replay on this named channel.
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
        self.volume_overrides
            .entry(channel.to_string())
            .or_default()
            .set_spatial(vol);
        self.pans.insert(channel.to_string(), pan);
        self.play(channel, path, repeat);
    }

    /// Plays already-in-memory encoded audio `bytes` at `source_pos` in 2D space — the byte-slice
    /// analogue of [`play_at`](Self::play_at) (which reads a file path). Use this for audio embedded
    /// with `include_bytes!`, the cross-platform clip source. Distance/pan are computed exactly like
    /// `play_at`, and an already-playing channel is repositioned with
    /// [`update_position`](Self::update_position).
    ///
    /// Backs the cross-platform [`Audio`](crate::Audio) facade's positional playback.
    pub fn play_bytes_at(
        &mut self,
        channel: &str,
        bytes: &[u8],
        repeat: bool,
        source_pos: Vec2,
        listener: Vec2,
        max_dist: f32,
    ) {
        let (vol, pan) = Self::spatial_params(source_pos, listener, max_dist);
        self.volume_overrides
            .entry(channel.to_string())
            .or_default()
            .set_spatial(vol);
        self.pans.insert(channel.to_string(), pan);
        self.play_bytes(channel, bytes, repeat);
    }

    /// Updates the spatial position of an already-playing channel in real time.
    ///
    /// Call every frame from an ECS system to track a moving sound source.
    ///
    /// Updates distance attenuation independently of the user volume. Active volume,
    /// fade-out and release fades continue; their current base is multiplied by the
    /// new attenuation immediately. Position is remembered across stop/replay.
    pub fn update_position(
        &mut self,
        channel: &str,
        source_pos: Vec2,
        listener: Vec2,
        max_dist: f32,
    ) {
        let (vol, pan) = Self::spatial_params(source_pos, listener, max_dist);

        self.volume_overrides
            .entry(channel.to_string())
            .or_default()
            .set_spatial(vol);
        self.pans.insert(channel.to_string(), pan);
        // Write the LIVE pan too. Storing it in `pans` alone only affected the next play, so a
        // positional sound tracked the listener in volume while its stereo image stayed frozen
        // wherever it started — the web `StereoPannerNode` path repositioned correctly, so the
        // same game sounded different on the two platforms.
        if let Some(handle) = self.pan_handles.get(channel) {
            handle.store(
                crate::audio::source::pack_pan(pan),
                std::sync::atomic::Ordering::Relaxed,
            );
        }

        if let Some(sink) = self.sinks.get(channel) {
            let base = self.fade_start_vol(channel);
            sink.set_volume(self.effective_volume_params(base, channel) * self.output_gain);
        }
    }

    // ── Volume / Pan ──────────────────────────────────────────────────────

    /// Sets the stereo pan for a channel (-1.0 = left, 0.0 = center, 1.0 = right).
    ///
    /// Applies **immediately** to a decoded clip already playing on the channel, and is remembered
    /// for the next clip playback. Synthesized tones bypass the panner; this setting has no effect
    /// on them.
    ///
    /// Stereo clips use linear balance: centre preserves both channels; hard pan silences the
    /// opposite channel without mixing it into the other. Web Audio instead uses a
    /// `StereoPannerNode`, which crossfeeds stereo input as it moves off centre.
    ///
    /// Mono clips stay mono and only get quieter: gain is 1.0 at centre and 0.5 at either extreme.
    /// Use a stereo asset for native sounds that must move in the stereo image.
    pub fn set_pan(&mut self, channel: &str, pan: f32) {
        let pan = pan.clamp(-1.0, 1.0);
        self.pans.insert(channel.to_string(), pan);
        if let Some(handle) = self.pan_handles.get(channel) {
            handle.store(
                crate::audio::source::pack_pan(pan),
                std::sync::atomic::Ordering::Relaxed,
            );
        }
    }

    /// Computes (volume, pan) from the sound source position and listener position.
    pub(super) fn spatial_params(source_pos: Vec2, listener: Vec2, max_dist: f32) -> (f32, f32) {
        crate::audio_spatial::spatial_params(source_pos, listener, max_dist)
    }
}

#[cfg(test)]
mod positional_tests {
    use super::*;

    fn close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < 1e-5, "{actual} != {expected}");
    }

    #[test]
    fn spatial_volume_and_user_volume_are_independent_when_device_exists() {
        let Some(mut audio) = AudioManager::new() else {
            eprintln!("SKIP: no audio device");
            return;
        };
        audio.set_volume("sfx", 0.4);
        audio.play_bytes_at(
            "sfx",
            include_bytes!("fixtures/tone.wav"),
            true,
            Vec2::new(50.0, 0.0),
            Vec2::ZERO,
            100.0,
        );
        close(audio.effective_volume("sfx"), 0.2);
        audio.set_volume("sfx", 0.8);
        close(audio.effective_volume("sfx"), 0.4);
        audio.update_position("sfx", Vec2::new(75.0, 0.0), Vec2::ZERO, 100.0);
        close(audio.effective_volume("sfx"), 0.2);
        close(audio.volume_overrides["sfx"].base, 0.8);
        audio.update_position("sfx", Vec2::new(200.0, 0.0), Vec2::ZERO, 100.0);
        audio.set_volume("sfx", 0.6);
        close(audio.effective_volume("sfx"), 0.0);
        audio.play_at(
            "sfx",
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/audio/fixtures/tone.wav"),
            true,
            Vec2::new(50.0, 0.0),
            Vec2::ZERO,
            100.0,
        );
        close(audio.effective_volume("sfx"), 0.3);
        audio.stop_immediate("sfx");
        audio.play_bytes("sfx", include_bytes!("fixtures/tone.wav"), true);
        close(audio.effective_volume("sfx"), 0.3);
        close(audio.effective_volume("untouched"), 1.0);
    }

    #[test]
    fn spatial_movement_preserves_fades_and_mixer_gains_when_device_exists() {
        let Some(mut audio) = AudioManager::new() else {
            eprintln!("SKIP: no audio device");
            return;
        };
        // Pause before observing real sink gains; no sound reaches the speakers even
        // while output_gain is 1, so a muted test cannot make zero-equals-zero pass.
        audio.output_gain = 0.0;
        audio.play_bytes("sfx", include_bytes!("fixtures/tone.wav"), true);
        audio.sinks["sfx"].pause();
        audio.output_gain = 1.0;
        audio.set_volume("sfx", 0.8);
        audio.assign_bus("sfx", "fx");
        audio.set_bus_volume("fx", 0.5);
        audio.bus_ducks.insert(
            "fx".into(),
            super::super::ducking::BusDuck {
                current: 0.5,
                target: 0.5,
                rate: 0.0,
            },
        );
        audio.update_position("sfx", Vec2::new(50.0, 0.0), Vec2::ZERO, 100.0);
        close(audio.sinks["sfx"].volume(), 0.1);
        audio.fade_volume("sfx", 0.4, 2.0);
        audio.update(0.5);
        close(audio.sinks["sfx"].volume(), 0.0875);
        audio.update_position("sfx", Vec2::new(75.0, 0.0), Vec2::ZERO, 100.0);
        close(audio.fades["sfx"].elapsed, 0.5);
        close(audio.sinks["sfx"].volume(), 0.04375);
        audio.update(0.5);
        close(audio.sinks["sfx"].volume(), 0.0375);
        audio.fade_out("sfx", 1.0);
        audio.update(0.5);
        close(audio.sinks["sfx"].volume(), 0.01875);
        audio.update_position("sfx", Vec2::ZERO, Vec2::ZERO, 100.0);
        assert!(audio.fades["sfx"].stop_when_done);
        close(audio.sinks["sfx"].volume(), 0.075);
        audio.update(0.5);
        assert!(!audio.sinks.contains_key("sfx"));
        close(audio.volume_overrides["sfx"].base, 0.8);
    }
}
