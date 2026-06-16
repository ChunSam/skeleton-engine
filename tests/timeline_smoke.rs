//! Smoke tests for the Timeline / Track API.
//!
//! Pure logic — no window, GPU, or event-loop required.
//! Exercises: `Timeline`, `Track`, `Keyframe`, `Easing` re-exports.

use engine::{Easing, Timeline, Track, Vec2};

/// `Track::sample` on an empty track returns `None`.
#[test]
fn track_sample_empty_returns_none() {
    let track: Track<f32> = Track::new();
    assert!(track.sample(0.5).is_none());
}

/// Single keyframe: sample clamps to that value before, at, and after.
#[test]
fn track_single_keyframe_clamping() {
    let mut track: Track<f32> = Track::new();
    track.add(1.0, 42.0, Easing::Linear);
    assert_eq!(track.sample(0.0), Some(42.0)); // before → clamped to first
    assert_eq!(track.sample(1.0), Some(42.0)); // at
    assert_eq!(track.sample(2.0), Some(42.0)); // after → clamped to last
}

/// Two keyframes: midpoint sample interpolates linearly.
#[test]
fn track_linear_interpolation_midpoint() {
    let mut track: Track<f32> = Track::new();
    track.add(0.0, 0.0, Easing::Linear);
    track.add(2.0, 10.0, Easing::Linear);
    let mid = track.sample(1.0).unwrap();
    // Linear at t=0.5 over [0..10] → 5.0
    assert!((mid - 5.0).abs() < 1e-5, "expected ~5.0 got {}", mid);
}

/// Track keeps keyframes sorted by time regardless of insertion order.
#[test]
fn track_add_sorts_by_time() {
    let mut track: Track<f32> = Track::new();
    track.add(2.0, 20.0, Easing::Linear);
    track.add(0.0, 0.0, Easing::Linear);
    track.add(1.0, 10.0, Easing::Linear);
    let kfs = track.keyframes();
    assert_eq!(kfs[0].time, 0.0);
    assert_eq!(kfs[1].time, 1.0);
    assert_eq!(kfs[2].time, 2.0);
}

/// `Track::remove` removes the keyframe at the given index.
#[test]
fn track_remove() {
    let mut track: Track<f32> = Track::new();
    track.add(0.0, 0.0, Easing::Linear);
    track.add(1.0, 1.0, Easing::Linear);
    assert_eq!(track.len(), 2);
    let removed = track.remove(0);
    assert!(removed.is_some());
    assert_eq!(track.len(), 1);
}

/// `Track::clear` removes all keyframes.
#[test]
fn track_clear() {
    let mut track: Track<f32> = Track::new();
    track.add(0.0, 0.0, Easing::Linear);
    track.add(1.0, 1.0, Easing::Linear);
    track.clear();
    assert!(track.is_empty());
    assert_eq!(track.len(), 0);
}

/// Vec2 track interpolates component-wise.
#[test]
fn track_vec2_interpolation() {
    let mut track: Track<Vec2> = Track::new();
    track.add(0.0, Vec2::ZERO, Easing::Linear);
    track.add(1.0, Vec2::new(10.0, 20.0), Easing::Linear);
    let v = track.sample(0.5).unwrap();
    assert!((v.x - 5.0).abs() < 1e-5);
    assert!((v.y - 10.0).abs() < 1e-5);
}

/// `Timeline::new` initialises with correct duration and `is_finished` = false.
#[test]
fn timeline_new_not_finished() {
    let tl = Timeline::new(3.0);
    assert_eq!(tl.duration, 3.0);
    assert!(!tl.is_finished());
    assert!(tl.playing);
}

/// `Timeline::is_finished` is true once time reaches duration (non-looping).
#[test]
fn timeline_finished_at_duration() {
    let mut tl = Timeline::new(1.0);
    tl.time = 1.0;
    assert!(tl.is_finished());
}

/// Looping timeline is never "finished" even past duration.
#[test]
fn timeline_looping_never_finished() {
    let mut tl = Timeline::new(1.0).looping();
    tl.time = 99.0;
    assert!(!tl.is_finished());
}

/// `Timeline::restart` resets time to zero.
#[test]
fn timeline_restart() {
    let mut tl = Timeline::new(2.0);
    tl.time = 1.5;
    tl.pause();
    tl.restart();
    assert_eq!(tl.time, 0.0);
    assert!(tl.playing);
}

/// `Track::set_time` re-sorts keyframes after mutation.
#[test]
fn track_set_time_resorts() {
    let mut track: Track<f32> = Track::new();
    track.add(0.0, 0.0, Easing::Linear);
    track.add(1.0, 1.0, Easing::Linear);
    // Move first keyframe (index 0, time=0.0) past the second (time=1.0)
    let ok = track.set_time(0, 2.0);
    assert!(ok);
    // After re-sort the first keyframe should now be time=1.0
    assert_eq!(track.keyframes()[0].time, 1.0);
}
