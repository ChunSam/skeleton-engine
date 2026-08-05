use glam::Vec2;

use super::AudioManager;

impl AudioManager {
    // ── Positional audio ──────────────────────────────────────────────────

    /// Plays a sound at `source_pos` in 2D space.
    ///
    /// - Silent when the distance between `source_pos` and `listener_pos` reaches `max_dist`.
    /// - Stereo pan is computed automatically from the X-axis difference.
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
        self.volume_overrides.insert(channel.to_string(), vol);
        self.pans.insert(channel.to_string(), pan);
        self.play_bytes(channel, bytes, repeat);
    }

    /// Updates the spatial position of an already-playing channel in real time.
    ///
    /// Call every frame from an ECS system to track a moving sound source.
    ///
    /// **Fade interaction**:
    /// - If a non-`stop_when_done` fade (i.e. `fade_volume`) is active, it is
    ///   cancelled before writing the spatial volume. The game explicitly
    ///   repositioned the source, so the spatial volume takes precedence.
    /// - If a `stop_when_done` fade (i.e. `fade_out` / release envelope) is
    ///   active, the fade is left running and the immediate sink write is
    ///   skipped. The spatial volume is stored in `volume_overrides` so it
    ///   takes effect once the fade completes and the channel is replayed.
    pub fn update_position(
        &mut self,
        channel: &str,
        source_pos: Vec2,
        listener: Vec2,
        max_dist: f32,
    ) {
        let (vol, pan) = Self::spatial_params(source_pos, listener, max_dist);

        // Determine how to handle an in-progress fade.
        let skip_sink_write = match self.fades.get(channel) {
            Some(fade) if fade.stop_when_done => {
                // A stop_when_done fade (fade_out / release) is winding down the
                // channel. Leave it running; the repositioned volume is stored in
                // volume_overrides for when the channel is next played.
                true
            }
            Some(_) => {
                // A plain fade_volume is active. The explicit reposition overrides
                // it — cancel the fade so the spatial volume isn't discarded next frame.
                self.fades.remove(channel);
                false
            }
            None => false,
        };

        self.volume_overrides.insert(channel.to_string(), vol);
        self.pans.insert(channel.to_string(), pan);

        if !skip_sink_write {
            if let Some(sink) = self.sinks.get(channel) {
                sink.set_volume(self.effective_volume_params(vol, channel) * self.output_gain);
            }
        }
    }

    // ── Volume / Pan ──────────────────────────────────────────────────────

    /// Sets the stereo pan for a channel (-1.0 = left, 0.0 = center, 1.0 = right).
    /// Takes effect from the next `play()` call.
    pub fn set_pan(&mut self, channel: &str, pan: f32) {
        self.pans.insert(channel.to_string(), pan.clamp(-1.0, 1.0));
    }

    /// Computes (volume, pan) from the sound source position and listener position.
    pub(super) fn spatial_params(source_pos: Vec2, listener: Vec2, max_dist: f32) -> (f32, f32) {
        crate::audio_spatial::spatial_params(source_pos, listener, max_dist)
    }
}

#[cfg(test)]
mod positional_tests {
    use super::*;

    // ── Fix #2: update_position fade-interaction ───────────────────────────────

    /// `update_position` with a non-`stop_when_done` fade active must cancel that
    /// fade (the reposition takes precedence) and write the spatial volume to
    /// `volume_overrides`. The fade entry must be gone afterward.
    #[test]
    fn update_position_cancels_fade_volume_and_writes_spatial_vol() {
        let Some(mut audio) = AudioManager::new() else {
            return;
        };
        // Play a tone and start a plain fade_volume (stop_when_done = false).
        audio.play_tone("sfx", 440.0, 10.0, 0.5);
        audio.fade_volume("sfx", 0.1, 5.0);
        assert!(
            audio.fades.contains_key("sfx"),
            "fade_volume must be active before update_position"
        );

        // Reposition: move the source close to the listener → high spatial volume.
        let source = Vec2::new(10.0, 0.0);
        let listener = Vec2::ZERO;
        audio.update_position("sfx", source, listener, 500.0);

        // The plain fade must be cancelled.
        assert!(
            !audio.fades.contains_key("sfx"),
            "update_position must cancel a non-stop_when_done fade"
        );

        // volume_overrides must reflect the spatial volume (close → ~1.0).
        let stored = audio.volume_overrides.get("sfx").copied().unwrap_or(0.0);
        assert!(
            stored > 0.95,
            "volume_overrides must store the spatial volume (~1.0 at dist=10), got {stored}"
        );
    }

    /// `update_position` while a `stop_when_done` fade is active must leave the
    /// fade running and skip the sink write. The spatial volume is stored in
    /// `volume_overrides` for when the channel is next played.
    #[test]
    fn update_position_preserves_stop_when_done_fade() {
        let Some(mut audio) = AudioManager::new() else {
            return;
        };
        audio.play_tone("sfx", 440.0, 10.0, 0.5);
        // fade_out schedules a stop_when_done fade.
        audio.fade_out("sfx", 5.0);
        assert!(
            audio.fades.contains_key("sfx"),
            "fade_out must be active before update_position"
        );

        // Reposition — should NOT cancel the stop_when_done fade.
        let source = Vec2::new(50.0, 0.0);
        audio.update_position("sfx", source, Vec2::ZERO, 500.0);

        // The stop_when_done fade must still be present.
        let fade = audio
            .fades
            .get("sfx")
            .expect("stop_when_done fade must not be cancelled by update_position");
        assert!(
            fade.stop_when_done,
            "the preserved fade must still be stop_when_done"
        );

        // volume_overrides is updated (will apply after the fade + re-play).
        let stored = audio.volume_overrides.get("sfx").copied().unwrap_or(0.0);
        let (expected_vol, _) = AudioManager::spatial_params(source, Vec2::ZERO, 500.0);
        assert!(
            (stored - expected_vol).abs() < 0.01,
            "volume_overrides must store the new spatial volume, got {stored}"
        );
    }
}
