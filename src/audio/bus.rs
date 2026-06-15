use super::types::Fade;
use super::AudioManager;
use std::collections::HashMap;

/// Merge bus names from the channel→bus assignments and the explicit bus-volume map into a
/// single sorted, deduplicated list. Pure (no device) so it is unit-testable headlessly.
fn collect_bus_names(
    channel_buses: &HashMap<String, String>,
    bus_volumes: &HashMap<String, f32>,
) -> Vec<String> {
    let mut names: Vec<String> = channel_buses
        .values()
        .cloned()
        .chain(bus_volumes.keys().cloned())
        .collect();
    names.sort();
    names.dedup();
    names
}

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

    /// Returns every known bus name, sorted and deduplicated.
    ///
    /// A bus is "known" if any channel is assigned to it (via [`assign_bus`](Self::assign_bus))
    /// **or** its volume has been set (via [`set_bus_volume`](Self::set_bus_volume)). Useful for
    /// building a mixer UI that lists all buses regardless of whether they currently carry a channel.
    pub fn bus_names(&self) -> Vec<String> {
        collect_bus_names(&self.channel_buses, &self.bus_volumes)
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
    /// This is an explicit fade — the release envelope (if any) is **not** applied
    /// on top.  Calling [`stop`](Self::stop) after `fade_out` has been scheduled
    /// cuts immediately (because a `stop_when_done` fade is already active).
    ///
    /// Requires [`update`](Self::update) to be called every frame.
    pub fn fade_out(&mut self, channel: &str, duration_secs: f32) {
        // Use the current interpolated fade volume if a fade is in progress so
        // there is no audible jump when fade_out is called mid-fade_volume.
        let start_vol = self.fade_start_vol(channel);
        self.fades.insert(
            channel.to_string(),
            Fade::stop_fade(start_vol, duration_secs),
        );
    }

    /// Fades the channel volume to `target` over `duration_secs` seconds.
    ///
    /// Requires [`update`](Self::update) to be called every frame.
    pub fn fade_volume(&mut self, channel: &str, target: f32, duration_secs: f32) {
        // Use the current interpolated fade volume if a fade is in progress so
        // chaining fade_volume calls doesn't jump to the stale volume_overrides value.
        let start_vol = self.fade_start_vol(channel);
        self.fades.insert(
            channel.to_string(),
            Fade {
                start_vol,
                target_vol: target.clamp(0.0, 1.0),
                duration: duration_secs.max(0.001),
                elapsed: 0.0,
                stop_when_done: false,
            },
        );
    }
}

#[cfg(test)]
mod bus_name_tests {
    use super::{collect_bus_names, AudioManager};
    use std::collections::HashMap;

    #[test]
    fn collect_bus_names_merges_sorts_and_dedups() {
        let mut channel_buses = HashMap::new();
        channel_buses.insert("bgm".to_string(), "music".to_string());
        channel_buses.insert("ambient".to_string(), "music".to_string()); // dup bus "music"
        channel_buses.insert("jump".to_string(), "sfx".to_string());

        let mut bus_volumes = HashMap::new();
        bus_volumes.insert("sfx".to_string(), 0.8); // already present via channel
        bus_volumes.insert("voice".to_string(), 0.5); // volume-only bus, no channel

        let names = collect_bus_names(&channel_buses, &bus_volumes);
        assert_eq!(
            names,
            vec!["music", "sfx", "voice"],
            "sorted + deduped union"
        );
    }

    #[test]
    fn collect_bus_names_empty_is_empty() {
        assert!(collect_bus_names(&HashMap::new(), &HashMap::new()).is_empty());
    }

    #[test]
    fn bus_names_round_trips_through_audio_manager() {
        // Skips on headless CI with no audio device (matches the other audio tests).
        let Some(mut audio) = AudioManager::new() else {
            return;
        };
        audio.assign_bus("bgm", "music");
        audio.assign_bus("jump", "sfx");
        audio.set_bus_volume("voice", 0.3); // volume-only bus
        assert_eq!(audio.bus_names(), vec!["music", "sfx", "voice"]);
    }
}
