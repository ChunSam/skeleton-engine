use super::AudioManager;

// ─── Types ────────────────────────────────────────────────────────────────────

/// Per-bus duck state: a gain multiplier (1.0 = no duck) that rides on top of
/// the bus volume and is animated toward a target at a fixed rate.
///
/// Created and managed internally by [`AudioManager::duck_bus`] /
/// [`AudioManager::release_bus`] / sidechain evaluation. Exposed read-only
/// through [`AudioManager::bus_duck`].
#[derive(Debug, Clone)]
pub struct BusDuck {
    /// Current duck multiplier (starts/rests at 1.0).
    pub(super) current: f32,
    /// Target duck multiplier the current value is animating toward.
    pub(super) target: f32,
    /// Per-second rate of change toward target (always positive).
    pub(super) rate: f32,
}

impl Default for BusDuck {
    fn default() -> Self {
        Self {
            current: 1.0,
            target: 1.0,
            rate: 0.0,
        }
    }
}

/// Automatically ducks `ducked_bus` while `trigger_bus` has audio playing.
///
/// Registered via [`AudioManager::set_sidechain`]; at most one sidechain per
/// `ducked_bus`.
#[derive(Debug, Clone)]
pub struct Sidechain {
    /// Bus that, when active (has a playing channel), triggers ducking.
    pub trigger_bus: String,
    /// Bus whose duck gain is reduced while the trigger is active.
    pub ducked_bus: String,
    /// Target duck gain when the trigger is active (0.0–1.0).
    pub gain: f32,
    /// Seconds to reach `gain` from 1.0 when the trigger becomes active.
    pub attack_secs: f32,
    /// Seconds to return from `gain` to 1.0 when the trigger goes silent.
    pub release_secs: f32,
}

// ─── Public API ───────────────────────────────────────────────────────────────

impl AudioManager {
    /// Ducks `bus` toward `gain` over `attack_secs` seconds.
    ///
    /// `gain` is clamped to [0.0, 1.0]. `attack_secs` is clamped to ≥ 0.001.
    pub fn duck_bus(&mut self, bus: &str, gain: f32, attack_secs: f32) {
        let gain = gain.clamp(0.0, 1.0);
        let attack = attack_secs.max(0.001);
        let duck = self.bus_ducks.entry(bus.to_string()).or_default();
        duck.target = gain;
        // Rate covers the remaining distance in attack_secs.
        let remaining = (duck.current - gain).abs();
        duck.rate = if remaining > 0.0001 {
            remaining / attack
        } else {
            0.0
        };
    }

    /// Releases the duck on `bus`, animating back toward 1.0 over `release_secs` seconds.
    ///
    /// `release_secs` is clamped to ≥ 0.001.
    pub fn release_bus(&mut self, bus: &str, release_secs: f32) {
        let release = release_secs.max(0.001);
        let duck = self.bus_ducks.entry(bus.to_string()).or_default();
        duck.target = 1.0;
        let remaining = (1.0 - duck.current).abs();
        duck.rate = if remaining > 0.0001 {
            remaining / release
        } else {
            0.0
        };
    }

    /// Returns the current duck multiplier for `bus` (1.0 if no duck is active).
    pub fn bus_duck(&self, bus: &str) -> f32 {
        self.bus_ducks.get(bus).map(|d| d.current).unwrap_or(1.0)
    }

    /// Registers (or replaces) a sidechain rule.
    ///
    /// While `trigger_bus` has at least one channel actively playing, `ducked_bus`
    /// is ducked toward `gain` over `attack_secs`. When no channel on `trigger_bus`
    /// is playing, `ducked_bus` releases toward 1.0 over `release_secs`.
    ///
    /// At most one sidechain per `ducked_bus` is stored; calling again with the
    /// same `ducked_bus` replaces the previous rule.
    ///
    /// A sidechain **owns** its `ducked_bus`: every `update(dt)` it sets that bus's
    /// duck target (to `gain` while the trigger plays, back to `1.0` otherwise), so a
    /// manual [`duck_bus`](Self::duck_bus) / [`release_bus`](Self::release_bus) on a
    /// sidechained bus is overridden each frame. Manually duck a *different* bus.
    pub fn set_sidechain(
        &mut self,
        trigger_bus: &str,
        ducked_bus: &str,
        gain: f32,
        attack_secs: f32,
        release_secs: f32,
    ) {
        let gain = gain.clamp(0.0, 1.0);
        let attack_secs = attack_secs.max(0.001);
        let release_secs = release_secs.max(0.001);

        // Replace any existing sidechain for the same ducked_bus.
        let key = ducked_bus.to_string();
        if let Some(existing) = self.sidechains.iter_mut().find(|sc| sc.ducked_bus == key) {
            existing.trigger_bus = trigger_bus.to_string();
            existing.gain = gain;
            existing.attack_secs = attack_secs;
            existing.release_secs = release_secs;
        } else {
            self.sidechains.push(Sidechain {
                trigger_bus: trigger_bus.to_string(),
                ducked_bus: key,
                gain,
                attack_secs,
                release_secs,
            });
        }
    }

    /// Removes the sidechain rule for `ducked_bus`, if any.
    pub fn clear_sidechain(&mut self, ducked_bus: &str) {
        self.sidechains.retain(|sc| sc.ducked_bus != ducked_bus);
    }
}

// ─── Internal: sidechain + duck progression ───────────────────────────────────

impl AudioManager {
    /// Returns `true` if any channel assigned to `bus` has a non-empty (playing) sink.
    pub(super) fn bus_is_active(&self, bus: &str) -> bool {
        self.channel_buses
            .iter()
            .filter(|(_, b)| b.as_str() == bus)
            .any(|(ch, _)| self.sinks.get(ch).map(|s| !s.empty()).unwrap_or(false))
    }

    /// Evaluate all sidechains and set each ducked bus's target and rate.
    /// Called once per frame from `update` (before `progress_ducks`).
    pub(super) fn evaluate_sidechains(&mut self) {
        // Collect sidechain data first to avoid simultaneous borrow of `self`.
        let rules: Vec<(String, String, f32, f32, f32)> = self
            .sidechains
            .iter()
            .map(|sc| {
                (
                    sc.trigger_bus.clone(),
                    sc.ducked_bus.clone(),
                    sc.gain,
                    sc.attack_secs,
                    sc.release_secs,
                )
            })
            .collect();

        for (trigger_bus, ducked_bus, gain, attack_secs, release_secs) in rules {
            let active = self.bus_is_active(&trigger_bus);
            let duck = self.bus_ducks.entry(ducked_bus).or_default();

            if active {
                // Set new target only if the trigger just became active or target changed.
                let new_target = gain;
                if (duck.target - new_target).abs() > 0.0001 || duck.rate == 0.0 {
                    duck.target = new_target;
                    let remaining = (duck.current - new_target).abs();
                    duck.rate = if remaining > 0.0001 {
                        // Travel the full |1.0 - gain| range in attack_secs.
                        (1.0_f32 - gain).abs().max(remaining) / attack_secs
                    } else {
                        0.0
                    };
                }
            } else {
                // Release: animate back toward 1.0.
                if (duck.target - 1.0).abs() > 0.0001 || (duck.current - 1.0).abs() > 0.0001 {
                    duck.target = 1.0;
                    let remaining = (1.0 - duck.current).abs();
                    duck.rate = if remaining > 0.0001 {
                        // Travel the full |1.0 - gain| range in release_secs.
                        (1.0_f32 - gain).abs().max(remaining) / release_secs
                    } else {
                        0.0
                    };
                }
            }
        }
    }

    /// Advance all `BusDuck` values toward their targets and re-apply sink
    /// volumes for any bus whose duck value changed.
    pub(super) fn progress_ducks(&mut self, dt: f32) {
        const EPSILON: f32 = 0.0001;

        let buses_to_update: Vec<String> = self
            .bus_ducks
            .iter()
            .filter(|(_, d)| (d.current - d.target).abs() > EPSILON)
            .map(|(bus, _)| bus.clone())
            .collect();

        for bus in buses_to_update {
            let changed = {
                // `bus` came from `bus_ducks` above and nothing removes entries here, but guard
                // anyway so a future refactor can't turn this into a mid-frame panic.
                let Some(duck) = self.bus_ducks.get_mut(&bus) else {
                    continue;
                };
                let prev = duck.current;
                let step = duck.rate * dt;
                if duck.target < duck.current {
                    duck.current = (duck.current - step).max(duck.target);
                } else {
                    duck.current = (duck.current + step).min(duck.target);
                }
                (duck.current - prev).abs() > EPSILON / 10.0
            };

            if changed {
                // Re-apply effective volume to all sinks on this bus.
                let channels: Vec<String> = self
                    .channel_buses
                    .iter()
                    .filter(|(_, b)| b.as_str() == bus.as_str())
                    .map(|(ch, _)| ch.clone())
                    .collect();
                for ch in channels {
                    let eff = self.effective_volume(&ch) * self.output_gain;
                    if let Some(sink) = self.sinks.get(&ch) {
                        sink.set_volume(eff);
                    }
                }
            }
        }
    }
}
