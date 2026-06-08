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

    /// Updates the spatial position of an already-playing channel in real time.
    ///
    /// Call every frame from an ECS system to track a moving sound source.
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

    // ── Volume / Pan ──────────────────────────────────────────────────────

    /// Sets the stereo pan for a channel (-1.0 = left, 0.0 = center, 1.0 = right).
    /// Takes effect from the next `play()` call.
    pub fn set_pan(&mut self, channel: &str, pan: f32) {
        self.pans.insert(channel.to_string(), pan.clamp(-1.0, 1.0));
    }

    /// Computes (volume, pan) from the sound source position and listener position.
    pub(super) fn spatial_params(source_pos: Vec2, listener: Vec2, max_dist: f32) -> (f32, f32) {
        let delta = source_pos - listener;
        let dist = delta.length();
        let volume = (1.0 - (dist / max_dist.max(0.001)).min(1.0)).max(0.0);
        let pan = (delta.x / max_dist.max(0.001)).clamp(-1.0, 1.0);
        (volume, pan)
    }
}
