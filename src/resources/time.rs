//! Time resources — global `dt` multiplier, the real (unscaled) frame delta, and the
//! per-frame delta cap.

/// Global time-scale multiplier applied to the `dt` that gameplay (scene) systems receive.
///
/// `1.0` = normal speed, `0.0` = frozen (hit-stop / pause-with-rendering), `0.5` = slow motion,
/// `2.0` = fast forward. Set it via [`crate::App::set_time_scale`] or
/// `world.resource_mut::<TimeScale>()`.
///
/// Only **scene** systems are scaled. Built-in tail systems (hierarchy/gizmo) and engine
/// post-frame work (fades, hot-reload, asset upload, camera) always run at real time, so the
/// editor and screen transitions stay responsive even at `time_scale = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeScale(pub f32);

impl Default for TimeScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl TimeScale {
    /// The current multiplier, clamped to be non-negative (negative scales make no sense).
    pub fn get(&self) -> f32 {
        self.0.max(0.0)
    }

    /// Sets the multiplier. Negative values are clamped to `0.0` when read via [`get`](Self::get).
    pub fn set(&mut self, scale: f32) {
        self.0 = scale;
    }
}

/// The real (unscaled) frame delta-time in seconds, written every frame before [`TimeScale`]
/// is applied.
///
/// Most systems should just use the `dt` argument (already time-scaled). Read this when a system
/// must run in real time *regardless* of the time scale — e.g. a hit-stop controller that sets
/// `TimeScale(0.0)` still needs real time to count down its own freeze window, otherwise it would
/// freeze itself forever.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RealDt(pub f32);

/// Caps the per-frame delta-time the main loop hands to systems.
///
/// A single long stall — a window drag, a debugger breakpoint, the tab backgrounded — would
/// otherwise produce a huge `dt` that makes physics tunnel and animations leap. The engine
/// clamps each frame's `dt` to [`max_dt`](Self::max_dt) so one slow frame can't blow up the
/// simulation; the trade-off is that after a real stall the world advances slower than
/// wall-clock for a few frames (it "catches up" in bounded steps rather than one jump).
///
/// Default `0.1` s (a 10 fps floor) — the value the cap was previously hardcoded to, so a
/// `World` that never inserts this resource behaves exactly as before. Raise it if your game
/// tolerates larger catch-up steps (e.g. a fixed-timestep sim that re-derives `dt` itself);
/// lower it to bound per-frame motion even harder. Set it via `world.resource_mut::<FrameConfig>()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameConfig {
    /// Maximum delta-time in seconds passed to systems in a single frame.
    pub max_dt: f32,
}

impl Default for FrameConfig {
    fn default() -> Self {
        // Reproduce the OLD hardcoded cap exactly.
        Self { max_dt: 0.1 }
    }
}

impl FrameConfig {
    /// Clamps a raw elapsed time (seconds) to [`max_dt`](Self::max_dt).
    pub fn cap(&self, raw_dt: f32) -> f32 {
        raw_dt.min(self.max_dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_config_default_matches_historical_cap() {
        // Guard the out-of-the-box behavior: the default must reproduce the 0.1 s cap that was
        // hardcoded in the main loop before it became a resource, so existing games are unchanged.
        assert!((FrameConfig::default().max_dt - 0.1).abs() < 1e-9);
    }

    #[test]
    fn cap_clamps_long_frames_but_passes_short_ones() {
        let cfg = FrameConfig::default(); // max_dt = 0.1
                                          // A normal ~60 fps frame passes through unchanged.
        assert!((cfg.cap(1.0 / 60.0) - 1.0 / 60.0).abs() < 1e-9);
        // A long stall (250 ms) is clamped to max_dt.
        assert!((cfg.cap(0.25) - 0.1).abs() < 1e-9);
        // Raising the cap lets a larger step through.
        let loose = FrameConfig { max_dt: 0.5 };
        assert!((loose.cap(0.25) - 0.25).abs() < 1e-9);
    }
}
