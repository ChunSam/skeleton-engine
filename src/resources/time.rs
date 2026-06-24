//! Time-scaling resources — global `dt` multiplier and the real (unscaled) frame delta.

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
