use super::types::Fade;
use super::AudioManager;

impl AudioManager {
    // ── Audio bus ─────────────────────────────────────────────────────────────

    /// Assigns a channel to a bus.
    ///
    /// Example: `assign_bus("bgm", "music")` → control all channels at once via `set_bus_volume("music", v)`.
    pub fn assign_bus(&mut self, channel: &str, bus: &str) {
        self.channel_buses
            .insert(channel.to_string(), bus.to_string());
        // Apply bus volume immediately
        let eff = self.effective_volume(channel);
        if let Some(sink) = self.sinks.get(channel) {
            sink.set_volume(eff);
        }
    }

    /// Sets the master volume for a bus. Applied immediately to all channels in the bus.
    pub fn set_bus_volume(&mut self, bus: &str, volume: f32) {
        self.bus_volumes
            .insert(bus.to_string(), volume.clamp(0.0, 1.0));
        // Update sinks for all channels in the bus
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

    /// Returns the bus volume (1.0 if not set).
    pub fn bus_volume(&self, bus: &str) -> f32 {
        self.bus_volumes.get(bus).copied().unwrap_or(1.0)
    }

    /// Sets the channel volume immediately (0.0 = silent, 1.0 = original).
    /// The effective volume is this value multiplied by the bus volume.
    pub fn set_volume(&mut self, channel: &str, volume: f32) {
        let vol = volume.clamp(0.0, 1.0);
        self.volume_overrides.insert(channel.to_string(), vol);
        let eff = self.effective_volume(channel);
        if let Some(sink) = self.sinks.get(channel) {
            sink.set_volume(eff);
        }
    }

    /// Fades out the channel playback over `duration_secs` seconds, then stops it.
    ///
    /// Requires [`update`](Self::update) to be called every frame.
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

    /// Fades the channel volume to `target` over `duration_secs` seconds.
    ///
    /// Requires [`update`](Self::update) to be called every frame.
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
