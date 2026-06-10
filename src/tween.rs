use crate::timer::Timer;

/// Interpolation curve.
#[derive(Clone, Debug, Default)]
pub enum Easing {
    /// Linear.
    #[default]
    Linear,
    /// Slow start, fast end.
    EaseIn,
    /// Fast start, slow end.
    EaseOut,
    /// Slow at both ends, fast in the middle.
    EaseInOut,
    /// Pulls back then springs forward (overshoot at start).
    EaseInBack,
    /// Goes forward, overshoots slightly, then settles back (overshoot at end).
    EaseOutBack,
}

impl Easing {
    /// Applies the easing curve to `t` (0.0–1.0) and returns the result.
    pub fn apply(&self, t: f32) -> f32 {
        match self {
            Easing::Linear => t,
            Easing::EaseIn => t * t,
            Easing::EaseOut => t * (2.0 - t),
            Easing::EaseInOut => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Easing::EaseInBack => {
                const C: f32 = 1.701_58;
                t * t * ((C + 1.0) * t - C)
            }
            Easing::EaseOutBack => {
                const C: f32 = 1.701_58;
                let s = t - 1.0;
                s * s * ((C + 1.0) * s + C) + 1.0
            }
        }
    }
}

// ── Lerp trait ─────────────────────────────────────────────────────────────

/// Trait for linearly interpolating between two values.
///
/// General interpolation utility — used by [`crate::timeline::Timeline`] tracks
/// and [`crate::network::SnapshotBuffer`] alike.
pub trait Lerp: Clone {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(a: &f32, b: &f32, t: f32) -> f32 {
        a + (b - a) * t
    }
}

impl Lerp for glam::Vec2 {
    fn lerp(a: &glam::Vec2, b: &glam::Vec2, t: f32) -> glam::Vec2 {
        a.lerp(*b, t)
    }
}

impl Lerp for [f32; 4] {
    fn lerp(a: &[f32; 4], b: &[f32; 4], t: f32) -> [f32; 4] {
        [
            a[0] + (b[0] - a[0]) * t,
            a[1] + (b[1] - a[1]) * t,
            a[2] + (b[2] - a[2]) * t,
            a[3] + (b[3] - a[3]) * t,
        ]
    }
}

impl Lerp for crate::color::Color {
    fn lerp(a: &Self, b: &Self, t: f32) -> Self {
        Self::rgba(
            a.r + (b.r - a.r) * t,
            a.g + (b.g - a.g) * t,
            a.b + (b.b - a.b) * t,
            a.a + (b.a - a.a) * t,
        )
    }
}

/// Tween that interpolates an f32 value over time.
///
/// # Example
/// ```rust
/// use engine::{Tween, Easing};
///
/// let mut tween = Tween::new(0.0, 100.0, 1.0).with_easing(Easing::EaseOut);
/// let v = tween.tick(0.5);
/// assert!(v > 50.0); // EaseOut is fast at the start
/// ```
#[derive(Clone, Debug)]
pub struct Tween {
    start: f32,
    end: f32,
    timer: Timer,
    easing: Easing,
}

impl Tween {
    /// Creates a tween that linearly interpolates from `start` to `end` over `duration` seconds.
    pub fn new(start: f32, end: f32, duration: f32) -> Self {
        Self {
            start,
            end,
            timer: Timer::once(duration),
            easing: Easing::Linear,
        }
    }

    /// Sets the easing curve (builder pattern).
    pub fn with_easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Advances by `dt` and returns the current interpolated value.
    pub fn tick(&mut self, dt: f32) -> f32 {
        self.timer.tick(dt);
        self.value()
    }

    /// Returns the current interpolated value without advancing time.
    pub fn value(&self) -> f32 {
        let t = self.easing.apply(self.timer.fraction());
        self.start + (self.end - self.start) * t
    }

    /// Returns true if the tween has finished.
    pub fn finished(&self) -> bool {
        self.timer.finished()
    }

    /// Progress from 0.0 to 1.0.
    pub fn fraction(&self) -> f32 {
        self.timer.fraction()
    }

    /// Resets the tween to its initial state.
    pub fn reset(&mut self) {
        self.timer.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_midpoint() {
        let mut tw = Tween::new(0.0, 100.0, 2.0);
        let v = tw.tick(1.0);
        assert!((v - 50.0).abs() < 1e-4);
    }

    #[test]
    fn ease_out_faster_start() {
        let mut tw = Tween::new(0.0, 100.0, 1.0).with_easing(Easing::EaseOut);
        let v = tw.tick(0.5);
        assert!(v > 50.0, "EaseOut at t=0.5 should be > 50, got {v}");
    }

    #[test]
    fn finishes_at_end() {
        let mut tw = Tween::new(10.0, 20.0, 1.0);
        tw.tick(2.0);
        assert!(tw.finished());
        assert!((tw.value() - 20.0).abs() < 1e-4);
    }
}
