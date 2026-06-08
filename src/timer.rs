/// Countdown or repeating timer.
///
/// # Example
/// ```rust
/// use engine::Timer;
///
/// let mut t = Timer::once(2.0);
/// t.tick(1.5);
/// assert!(!t.finished());
/// t.tick(0.6);
/// assert!(t.finished());
/// ```
#[derive(Clone, Debug)]
pub struct Timer {
    duration: f32,
    elapsed: f32,
    repeating: bool,
    just_finished: bool,
}

impl Timer {
    /// Creates a timer that fires once after the specified duration (seconds).
    pub fn once(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            repeating: false,
            just_finished: false,
        }
    }

    /// Creates a timer that fires repeatedly every specified duration (seconds).
    pub fn repeating(duration: f32) -> Self {
        Self {
            duration,
            elapsed: 0.0,
            repeating: true,
            just_finished: false,
        }
    }

    /// Advances the timer by `dt`. Call this every frame from a system.
    pub fn tick(&mut self, dt: f32) {
        if self.finished() {
            self.just_finished = false;
            return;
        }
        self.elapsed += dt;
        if self.elapsed >= self.duration {
            self.just_finished = true;
            if self.repeating {
                // When duration <= 0 (e.g. repeating(0.0)) reset elapsed to 0 to prevent
                // unbounded accumulation. For positive durations wrap with modulo so elapsed
                // stays within bounds even on slow frames where dt > duration.
                if self.duration > 0.0 {
                    self.elapsed %= self.duration;
                } else {
                    self.elapsed = 0.0;
                }
            } else {
                self.elapsed = self.duration;
            }
        } else {
            self.just_finished = false;
        }
    }

    /// Returns whether the timer has finished. Always `false` for repeating timers.
    pub fn finished(&self) -> bool {
        !self.repeating && self.elapsed >= self.duration
    }

    /// Returns `true` only on the tick the timer fired (including repeating; true for one frame only).
    pub fn just_finished(&self) -> bool {
        self.just_finished
    }

    /// Elapsed time in seconds.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Total duration in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Completion fraction from 0.0 to 1.0.
    pub fn fraction(&self) -> f32 {
        if self.duration <= 0.0 {
            1.0
        } else {
            (self.elapsed / self.duration).min(1.0)
        }
    }

    /// Resets the timer to its initial state.
    pub fn reset(&mut self) {
        self.elapsed = 0.0;
        self.just_finished = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn once_finishes() {
        let mut t = Timer::once(1.0);
        t.tick(0.5);
        assert!(!t.finished());
        assert!(!t.just_finished());
        t.tick(0.6);
        assert!(t.finished());
        assert!(t.just_finished());
        // just_finished is false after ticking past completion
        t.tick(0.1);
        assert!(t.finished());
        assert!(!t.just_finished());
    }

    #[test]
    fn repeating_wraps() {
        let mut t = Timer::repeating(1.0);
        t.tick(1.1);
        assert!(!t.finished());
        assert!(t.just_finished());
        assert!((t.elapsed() - 0.1).abs() < 1e-5);
    }

    #[test]
    fn fraction_clamps() {
        let mut t = Timer::once(2.0);
        t.tick(1.0);
        assert!((t.fraction() - 0.5).abs() < 1e-5);
        t.tick(5.0);
        assert!((t.fraction() - 1.0).abs() < 1e-5);
    }

    #[test]
    fn repeating_zero_duration_stays_bounded() {
        // repeating(0.0) fires just_finished every tick but elapsed must not grow
        // unboundedly (previously elapsed -= 0 failed to wrap, causing infinite growth).
        let mut t = Timer::repeating(0.0);
        for _ in 0..1000 {
            t.tick(0.016);
            assert!(t.just_finished());
        }
        assert!(t.elapsed() < 1.0, "elapsed grew unbounded: {}", t.elapsed());
    }

    #[test]
    fn repeating_catches_up_when_dt_exceeds_duration() {
        // elapsed stays within bounds via modulo even on slow frames where dt > duration.
        let mut t = Timer::repeating(1.0);
        t.tick(3.5);
        assert!(t.just_finished());
        assert!(t.elapsed() < 1.0, "elapsed not wrapped: {}", t.elapsed());
    }
}
