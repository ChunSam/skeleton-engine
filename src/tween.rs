use crate::timer::Timer;
use serde::{Deserialize, Serialize};

/// Interpolation curve.
///
/// `#[non_exhaustive]`: more curves may be added in future releases, so external `match`es
/// must include a `_` arm.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
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
    /// Accelerating bounce at the start (mirror of `EaseOutBounce`).
    EaseInBounce,
    /// Decelerating bounce at the end — like a ball dropping and settling.
    EaseOutBounce,
    /// Springy wind-up at the start (damped oscillation).
    EaseInElastic,
    /// Springy overshoot that oscillates and settles at the end.
    EaseOutElastic,
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
            Easing::EaseOutBounce => Self::out_bounce(t),
            Easing::EaseInBounce => 1.0 - Self::out_bounce(1.0 - t),
            Easing::EaseInElastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    const C4: f32 = core::f32::consts::TAU / 3.0;
                    -(2f32.powf(10.0 * t - 10.0)) * ((t * 10.0 - 10.75) * C4).sin()
                }
            }
            Easing::EaseOutElastic => {
                if t <= 0.0 {
                    0.0
                } else if t >= 1.0 {
                    1.0
                } else {
                    const C4: f32 = core::f32::consts::TAU / 3.0;
                    2f32.powf(-10.0 * t) * ((t * 10.0 - 0.75) * C4).sin() + 1.0
                }
            }
        }
    }

    /// The classic decelerating "bounce" curve, shared by `EaseOutBounce`/`EaseInBounce`.
    fn out_bounce(t: f32) -> f32 {
        const N1: f32 = 7.5625;
        const D1: f32 = 2.75;
        if t < 1.0 / D1 {
            N1 * t * t
        } else if t < 2.0 / D1 {
            let t = t - 1.5 / D1;
            N1 * t * t + 0.75
        } else if t < 2.5 / D1 {
            let t = t - 2.25 / D1;
            N1 * t * t + 0.9375
        } else {
            let t = t - 2.625 / D1;
            N1 * t * t + 0.984375
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

/// Tween that interpolates a value of type `T` over time.
///
/// `T` is any [`Lerp`] type — `f32` (the default), [`glam::Vec2`], [`crate::Color`], etc. The
/// type parameter defaults to `f32`, so existing `Tween::new(0.0, 100.0, 1.0)` call sites and
/// [`TweenSequence`] (which is `f32`-only) are unaffected.
///
/// # Example
/// ```rust
/// use engine::{Tween, Easing};
///
/// // f32 (default type parameter)
/// let mut tween = Tween::new(0.0, 100.0, 1.0).with_easing(Easing::EaseOut);
/// let v = tween.tick(0.5);
/// assert!(v > 50.0); // EaseOut is fast at the start
///
/// // Vec2 — interpolate a position in one tween
/// use engine::Vec2;
/// let mut pos = Tween::new(Vec2::ZERO, Vec2::new(200.0, 0.0), 1.0);
/// let p = pos.tick(0.5);
/// assert!((p.x - 100.0).abs() < 1e-4);
/// ```
#[derive(Clone, Debug)]
pub struct Tween<T: Lerp = f32> {
    start: T,
    end: T,
    timer: Timer,
    easing: Easing,
}

impl<T: Lerp> Tween<T> {
    /// Creates a tween that interpolates from `start` to `end` over `duration` seconds (linear).
    pub fn new(start: T, end: T, duration: f32) -> Self {
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
    pub fn tick(&mut self, dt: f32) -> T {
        self.timer.tick(dt);
        self.value()
    }

    /// Returns the current interpolated value without advancing time.
    pub fn value(&self) -> T {
        T::lerp(
            &self.start,
            &self.end,
            self.easing.apply(self.timer.fraction()),
        )
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

// ── TweenSequence ──────────────────────────────────────────────────────────

/// A sequence of [`Tween`] segments played back-to-back.
///
/// Segments are advanced one at a time; leftover `dt` carries into the next
/// segment within the same [`tick`](TweenSequence::tick) call, so a large
/// delta-time never stalls at a segment boundary.
///
/// # Empty sequence
///
/// An empty `TweenSequence` is always [`finished`](TweenSequence::finished)
/// and returns `0.0` from [`value`](TweenSequence::value).
///
/// # `fraction` semantics
///
/// [`fraction`](TweenSequence::fraction) is the **segment count fraction**:
/// `current_segment_index / total_segments` plus the within-segment fraction
/// divided by total segments. This gives uniform progress per-segment regardless
/// of individual durations.
///
/// # Example
///
/// ```rust
/// use engine::{TweenSequence, Easing};
///
/// let mut seq = TweenSequence::new()
///     .then(0.0, 100.0, 1.0, Easing::EaseOut)
///     .then(100.0, 0.0, 1.0, Easing::EaseIn)
///     .looping(true);
///
/// let v = seq.tick(0.5); // mid-way through first segment
/// assert!(v > 50.0);     // EaseOut is fast at the start
/// ```
#[derive(Clone, Debug)]
pub struct TweenSequence {
    segments: Vec<Tween>,
    current: usize,
    looping: bool,
    done: bool,
}

impl TweenSequence {
    /// Creates an empty sequence with no segments.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            current: 0,
            looping: false,
            done: false,
        }
    }

    /// Appends a segment that interpolates from `start` to `end` over
    /// `duration` seconds using the given `easing`. Returns `self` for chaining.
    pub fn then(mut self, start: f32, end: f32, duration: f32, easing: Easing) -> Self {
        self.segments
            .push(Tween::new(start, end, duration).with_easing(easing));
        self
    }

    /// Appends an already-constructed [`Tween`] segment. Returns `self` for chaining.
    pub fn push(mut self, tween: Tween) -> Self {
        self.segments.push(tween);
        self
    }

    /// Sets whether the sequence restarts from the first segment after the last
    /// one finishes. Returns `self` for chaining.
    pub fn looping(mut self, enabled: bool) -> Self {
        self.looping = enabled;
        self
    }

    /// Advances the sequence by `dt` seconds and returns the current interpolated value.
    ///
    /// Leftover time carries across segment boundaries within a single call, so
    /// passing a large `dt` correctly skips through multiple segments.
    ///
    /// Returns `0.0` when the sequence is empty or already finished.
    pub fn tick(&mut self, mut dt: f32) -> f32 {
        if self.segments.is_empty() || self.done {
            return self.value();
        }

        // Drive the current segment, carrying leftover dt into subsequent segments.
        //
        // `zero_crossings` guards against an infinite loop: a *looping* sequence whose
        // segments are all zero-duration consumes no `dt` per boundary, so the `dt <= 0.0`
        // exit below would never trigger. Any positive-duration segment resets the counter,
        // so legitimate multi-loop fast-forward on a large `dt` still works.
        let mut zero_crossings = 0usize;
        loop {
            let seg = &mut self.segments[self.current];

            // Compute how much time is left in this segment.
            let remaining = seg.timer.duration() - seg.timer.elapsed();
            if dt < remaining {
                // Segment won't finish this tick.
                seg.timer.tick(dt);
                break;
            }

            // Consume enough dt to finish the current segment exactly.
            seg.timer.tick(remaining);
            dt -= remaining;
            if remaining > 0.0 {
                zero_crossings = 0;
            } else {
                zero_crossings += 1;
            }

            // Advance to the next segment.
            let next = self.current + 1;
            if next >= self.segments.len() {
                if self.looping {
                    // Reset all segments and wrap around.
                    for s in &mut self.segments {
                        s.reset();
                    }
                    self.current = 0;
                } else {
                    self.done = true;
                    break;
                }
            } else {
                self.current = next;
            }

            if dt <= 0.0 {
                break;
            }
            // A full ring of zero-duration segments would otherwise spin forever.
            if zero_crossings > self.segments.len() {
                break;
            }
        }

        self.value()
    }

    /// Returns the current interpolated value without advancing time.
    ///
    /// Returns `0.0` when the sequence is empty.
    pub fn value(&self) -> f32 {
        if self.segments.is_empty() {
            return 0.0;
        }
        self.segments[self.current].value()
    }

    /// Returns `true` when all segments have finished and the sequence is not looping.
    ///
    /// Always `true` for an empty sequence.
    pub fn finished(&self) -> bool {
        if self.segments.is_empty() {
            return true;
        }
        self.done
    }

    /// Overall progress from `0.0` to `1.0`, measured in **segment count fraction**:
    /// each segment contributes `1 / total_segments` of the total range, and the
    /// within-segment [`fraction`](Tween::fraction) fills that share.
    ///
    /// Returns `0.0` for an empty sequence and `1.0` when finished.
    pub fn fraction(&self) -> f32 {
        let n = self.segments.len();
        if n == 0 {
            return 0.0;
        }
        if self.done {
            return 1.0;
        }
        let seg_share = 1.0 / n as f32;
        let within = self.segments[self.current].fraction();
        self.current as f32 * seg_share + within * seg_share
    }

    /// Resets the sequence to the beginning of the first segment.
    pub fn reset(&mut self) {
        for s in &mut self.segments {
            s.reset();
        }
        self.current = 0;
        self.done = false;
    }

    /// Returns the index of the currently active segment (0-based).
    ///
    /// For a finished non-looping sequence this is the index of the last segment.
    pub fn current_segment(&self) -> usize {
        self.current
    }

    /// Returns the total number of segments.
    pub fn segment_count(&self) -> usize {
        self.segments.len()
    }
}

impl Default for TweenSequence {
    fn default() -> Self {
        Self::new()
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

    // ── TweenSequence tests ────────────────────────────────────────────────

    #[test]
    fn empty_sequence_is_finished() {
        let seq = TweenSequence::new();
        assert!(seq.finished());
        assert_eq!(seq.value(), 0.0);
        assert_eq!(seq.fraction(), 0.0);
    }

    #[test]
    fn single_segment_basic() {
        let mut seq = TweenSequence::new().then(0.0, 10.0, 1.0, Easing::Linear);
        let v = seq.tick(0.5);
        assert!((v - 5.0).abs() < 1e-4, "got {v}");
        assert!(!seq.finished());
    }

    #[test]
    fn single_segment_finishes() {
        let mut seq = TweenSequence::new().then(0.0, 10.0, 1.0, Easing::Linear);
        seq.tick(2.0);
        assert!(seq.finished());
        assert!((seq.value() - 10.0).abs() < 1e-4);
        assert!((seq.fraction() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn two_segments_carry_dt() {
        // Two 1-second segments: 0→10, then 10→20.
        // dt=1.5 should complete the first and advance 0.5 into the second.
        let mut seq = TweenSequence::new()
            .then(0.0, 10.0, 1.0, Easing::Linear)
            .then(10.0, 20.0, 1.0, Easing::Linear);

        let v = seq.tick(1.5);
        // 0.5 into segment 1 (10→20, linear): 10 + 0.5*10 = 15
        assert!((v - 15.0).abs() < 1e-4, "expected 15, got {v}");
        assert_eq!(seq.current_segment(), 1);
        assert!(!seq.finished());
    }

    #[test]
    fn two_segments_full_completion() {
        let mut seq = TweenSequence::new()
            .then(0.0, 10.0, 1.0, Easing::Linear)
            .then(10.0, 20.0, 1.0, Easing::Linear);

        seq.tick(3.0); // well past both segments
        assert!(seq.finished());
        assert!((seq.value() - 20.0).abs() < 1e-4);
        assert!((seq.fraction() - 1.0).abs() < 1e-4);
    }

    #[test]
    fn looping_wraps_around() {
        let mut seq = TweenSequence::new()
            .then(0.0, 10.0, 1.0, Easing::Linear)
            .looping(true);

        // Advance past the end to trigger a loop.
        seq.tick(1.5);
        assert!(!seq.finished());
        assert_eq!(seq.current_segment(), 0);
        // 0.5 into the restarted first segment: value = 5
        let v = seq.value();
        assert!((v - 5.0).abs() < 1e-4, "expected 5 after loop, got {v}");
    }

    #[test]
    fn reset_restarts_sequence() {
        let mut seq = TweenSequence::new()
            .then(0.0, 10.0, 1.0, Easing::Linear)
            .then(10.0, 20.0, 1.0, Easing::Linear);

        seq.tick(3.0); // finish
        assert!(seq.finished());

        seq.reset();
        assert!(!seq.finished());
        assert_eq!(seq.current_segment(), 0);
        let v = seq.tick(0.5);
        assert!((v - 5.0).abs() < 1e-4, "after reset got {v}");
    }

    #[test]
    fn fraction_midway_two_segments() {
        // Two equal segments — at the midpoint of segment 1, fraction = 0.75.
        let mut seq = TweenSequence::new()
            .then(0.0, 1.0, 1.0, Easing::Linear)
            .then(1.0, 2.0, 1.0, Easing::Linear);

        seq.tick(1.5); // exactly in the middle of segment 1
        let f = seq.fraction();
        // segment_share = 0.5; current=1, within=0.5 → 1*0.5 + 0.5*0.5 = 0.75
        assert!((f - 0.75).abs() < 1e-4, "expected 0.75 got {f}");
    }

    #[test]
    fn push_api_works() {
        let tween = Tween::new(5.0, 15.0, 2.0).with_easing(Easing::EaseIn);
        let mut seq = TweenSequence::new().push(tween);
        seq.tick(1.0);
        // EaseIn at t=0.5: t^2 = 0.25 → 5 + 0.25*10 = 7.5
        let v = seq.value();
        assert!((v - 7.5).abs() < 1e-4, "got {v}");
    }

    // ── Generic Tween<T> tests ─────────────────────────────────────────────

    #[test]
    fn generic_vec2_tween_midpoint() {
        use glam::Vec2;
        let mut tw = Tween::new(Vec2::ZERO, Vec2::new(200.0, 100.0), 2.0);
        let v = tw.tick(1.0); // linear midpoint
        assert!((v.x - 100.0).abs() < 1e-4, "x={}", v.x);
        assert!((v.y - 50.0).abs() < 1e-4, "y={}", v.y);
    }

    #[test]
    fn generic_color_tween_midpoint() {
        use crate::color::Color;
        let mut tw = Tween::new(
            Color::rgba(0.0, 0.0, 0.0, 0.0),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
            1.0,
        );
        let c = tw.tick(0.5);
        assert!(
            (c.r - 0.5).abs() < 1e-4 && (c.a - 0.5).abs() < 1e-4,
            "got {c:?}"
        );
    }

    // ── New easing curves ──────────────────────────────────────────────────

    #[test]
    fn bounce_and_elastic_hit_endpoints() {
        for e in [
            Easing::EaseInBounce,
            Easing::EaseOutBounce,
            Easing::EaseInElastic,
            Easing::EaseOutElastic,
        ] {
            assert!(e.apply(0.0).abs() < 1e-4, "{e:?} at 0 should be ~0");
            assert!((e.apply(1.0) - 1.0).abs() < 1e-4, "{e:?} at 1 should be ~1");
        }
    }

    #[test]
    fn looping_zero_duration_does_not_hang() {
        // A looping sequence whose segments are all zero-duration consumes no dt per
        // boundary; before the zero-crossing guard this `tick` spun forever.
        let mut seq = TweenSequence::new()
            .then(0.0, 10.0, 0.0, Easing::Linear)
            .then(10.0, 20.0, 0.0, Easing::Linear)
            .looping(true);
        let v = seq.tick(1.0);
        assert!(!seq.finished(), "looping sequence never finishes");
        assert!(v.is_finite(), "value should stay finite, got {v}");
    }

    #[test]
    fn out_elastic_overshoots() {
        // Elastic oscillates past 1.0 somewhere in the middle.
        let peak = (1..20)
            .map(|i| Easing::EaseOutElastic.apply(i as f32 / 20.0))
            .fold(f32::MIN, f32::max);
        assert!(
            peak > 1.0,
            "EaseOutElastic should overshoot 1.0, peak={peak}"
        );
    }
}
