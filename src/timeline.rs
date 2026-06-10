//! Timeline/cutscene system — keyframe-driven entity animation.
//!
//! # Example
//! ```rust,ignore
//! use engine::{Timeline, TimelineSystem, Easing};
//! use glam::Vec2;
//!
//! let mut tl = Timeline::new(2.0);
//! tl.position.add(0.0, Vec2::ZERO, Easing::Linear);
//! tl.position.add(1.0, Vec2::new(200.0, 0.0), Easing::EaseInOut);
//! tl.position.add(2.0, Vec2::new(200.0, 200.0), Easing::EaseOut);
//! world.add_component(entity, tl);
//! app.add_system(TimelineSystem);
//! ```

use crate::tween::Easing;

// ── Lerp trait ─────────────────────────────────────────────────────────────

// `Lerp` moved to `crate::tween` — it is a general interpolation utility used
// well beyond cutscenes (e.g. `network::SnapshotBuffer`). Re-exported here so
// existing `timeline::Lerp` paths keep compiling.
pub use crate::tween::Lerp;

// ── Keyframe<T> ──────────────────────────────────────────────────────────────

/// A keyframe that sets a specific value at a specific time.
#[derive(Debug, Clone)]
pub struct Keyframe<T: Clone> {
    pub time: f32,
    pub value: T,
    pub easing: Easing,
}

// ── Track<T> ─────────────────────────────────────────────────────────────────

/// A sequence of keyframes of the same type, kept sorted by time.
#[derive(Debug, Clone)]
pub struct Track<T: Clone + Lerp> {
    keyframes: Vec<Keyframe<T>>,
}

impl<T: Clone + Lerp> Track<T> {
    pub fn new() -> Self {
        Self {
            keyframes: Vec::new(),
        }
    }

    pub fn with_keyframes(keyframes: Vec<Keyframe<T>>) -> Self {
        let mut track = Self { keyframes };
        track.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
        track
    }

    /// Adds a keyframe and re-sorts by time.
    pub fn add(&mut self, time: f32, value: T, easing: Easing) -> &mut Self {
        self.keyframes.push(Keyframe {
            time,
            value,
            easing,
        });
        self.keyframes.sort_by(|a, b| a.time.total_cmp(&b.time));
        self
    }

    /// Returns the interpolated value at time `t`.
    /// Returns `None` if there are no keyframes; clamps to the first/last value outside range.
    pub fn sample(&self, t: f32) -> Option<T> {
        if t.is_nan() {
            return None;
        }
        if self.keyframes.is_empty() {
            return None;
        }

        // Before first keyframe — clamp to first value
        if t <= self.keyframes[0].time {
            return Some(self.keyframes[0].value.clone());
        }

        // After last keyframe — clamp to last value
        let last = self.keyframes.last().unwrap();
        if t >= last.time {
            return Some(last.value.clone());
        }

        // Find the last keyframe with time <= t.
        // In a degenerate track where all keyframe times are NaN, rposition returns None
        // (NaN <= t == false). Fall back to the first value instead of panicking.
        let Some(idx) = self.keyframes.iter().rposition(|kf| kf.time <= t) else {
            return Some(self.keyframes[0].value.clone());
        };
        let a = &self.keyframes[idx];
        let b = &self.keyframes[idx + 1];

        let span = b.time - a.time;
        let local_t = if span > 1e-6 {
            (t - a.time) / span
        } else {
            1.0
        };
        let eased_t = a.easing.apply(local_t);

        Some(T::lerp(&a.value, &b.value, eased_t))
    }

    /// Total duration of the track (the time of the last keyframe).
    pub fn duration(&self) -> f32 {
        self.keyframes.last().map(|kf| kf.time).unwrap_or(0.0)
    }

    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }
}

impl<T: Clone + Lerp> Default for Track<T> {
    fn default() -> Self {
        Self::new()
    }
}

// ── CameraTarget marker ─────────────────────────────────────────────────────────

/// Marker that redirects a [`Timeline`] from the entity's own `Transform`/`Sprite`
/// to the global [`Camera`](crate::camera::Camera) resource. With this component
/// present, [`TimelineSystem`] writes the `position` track to `Camera::position`
/// and the `zoom` track to `Camera::zoom`, letting a `Timeline` author camera
/// pans/zooms for cutscenes. The entity is a virtual camera rig: its other tracks
/// (rotation/scale/color/alpha) are ignored, and no `Transform` is required.
///
/// ```rust,ignore
/// let mut cam_tl = Timeline::new(3.0);
/// cam_tl.position.add(0.0, Vec2::new(0.0, 0.0), Easing::EaseInOut);
/// cam_tl.position.add(3.0, Vec2::new(400.0, 0.0), Easing::EaseInOut);
/// cam_tl.zoom.add(0.0, 1.0, Easing::Linear);
/// cam_tl.zoom.add(3.0, 1.5, Easing::EaseOut);
/// let rig = world.spawn();
/// world.add_component(rig, cam_tl);
/// world.add_component(rig, CameraTarget);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct CameraTarget;

// ── Timeline component ─────────────────────────────────────────────────────────

/// Timeline component attached to an entity. `TimelineSystem` drives it every frame.
///
/// # Example
/// ```rust,ignore
/// let mut tl = Timeline::new(2.0); // 2-second timeline
/// tl.position.add(0.0, Vec2::new(0., 0.), Easing::Linear);
/// tl.position.add(1.0, Vec2::new(200., 0.), Easing::EaseInOut);
/// tl.position.add(2.0, Vec2::new(200., 200.), Easing::EaseOut);
/// world.add_component(entity, tl);
/// app.add_system(TimelineSystem);
/// ```
#[derive(Debug, Clone)]
pub struct Timeline {
    /// Total playback duration in seconds.
    pub duration: f32,
    /// Current playback position in seconds.
    pub time: f32,
    /// Whether the timeline loops.
    pub looping: bool,
    /// Whether the timeline is playing (the system does not advance time when false).
    pub playing: bool,

    // ── Tracks ──────────────────────────────────────────────────────────────────
    pub position: Track<glam::Vec2>,
    pub rotation: Track<f32>,
    pub scale: Track<glam::Vec2>,
    pub color: Track<crate::color::Color>,
    pub alpha: Track<f32>,
    /// Camera zoom track. Only applied when the entity also carries
    /// [`CameraTarget`]; empty (inert) by default, so ordinary timelines that
    /// drive a `Transform`/`Sprite` are unaffected.
    pub zoom: Track<f32>,
}

impl Timeline {
    pub fn new(duration: f32) -> Self {
        Self {
            duration,
            time: 0.0,
            looping: false,
            playing: true,
            position: Track::new(),
            rotation: Track::new(),
            scale: Track::new(),
            color: Track::new(),
            alpha: Track::new(),
            zoom: Track::new(),
        }
    }

    /// Enables looping and returns self (builder pattern).
    pub fn looping(mut self) -> Self {
        self.looping = true;
        self
    }

    pub fn play(&mut self) {
        self.playing = true;
    }
    pub fn pause(&mut self) {
        self.playing = false;
    }
    pub fn restart(&mut self) {
        self.time = 0.0;
        self.playing = true;
    }

    /// Returns `true` if a non-looping timeline has played to the end.
    pub fn is_finished(&self) -> bool {
        !self.looping && self.time >= self.duration
    }
}

impl Default for Timeline {
    fn default() -> Self {
        Self::new(1.0)
    }
}

// ── TimelineSystem ────────────────────────────────────────────────────────────

pub struct TimelineSystem;

impl crate::ecs::System for TimelineSystem {
    fn name(&self) -> &'static str {
        "TimelineSystem"
    }

    fn run(&mut self, world: &mut crate::ecs::World, dt: f32) {
        use crate::camera::Camera;
        use crate::components::{Sprite, Transform};

        // Collect entities with Timeline first to avoid borrow conflicts
        let entities: Vec<crate::ecs::Entity> = world.query::<Timeline>().map(|(e, _)| e).collect();

        for entity in entities {
            // Take the timeline out to avoid double-borrow
            let mut tl = match world.take_component::<Timeline>(entity) {
                Some(t) => t,
                None => continue,
            };

            // Advance time
            if tl.playing && !tl.is_finished() {
                tl.time += dt;
                if tl.looping && tl.time > tl.duration {
                    tl.time -= tl.duration;
                } else {
                    tl.time = tl.time.min(tl.duration);
                }
            }

            let t = tl.time;

            // A `CameraTarget` entity drives the `Camera` resource (a virtual
            // rig) instead of its own Transform/Sprite: position → camera
            // position, zoom → camera zoom. Its other tracks are ignored.
            if world.get::<CameraTarget>(entity).is_some() {
                if let Some(pos) = tl.position.sample(t) {
                    if let Some(cam) = world.resource_mut::<Camera>() {
                        cam.position = pos;
                    }
                }
                if let Some(zoom) = tl.zoom.sample(t) {
                    if let Some(cam) = world.resource_mut::<Camera>() {
                        cam.zoom = zoom;
                    }
                }
                world.add_component(entity, tl);
                continue;
            }

            // Apply position
            if let Some(pos) = tl.position.sample(t) {
                if let Some(transform) = world.get_mut::<Transform>(entity) {
                    transform.position = pos;
                }
            }

            // Apply rotation
            if let Some(rot) = tl.rotation.sample(t) {
                if let Some(transform) = world.get_mut::<Transform>(entity) {
                    transform.rotation = rot;
                }
            }

            // Apply scale
            if let Some(scale) = tl.scale.sample(t) {
                if let Some(transform) = world.get_mut::<Transform>(entity) {
                    transform.scale = scale;
                }
            }

            // Apply color
            if let Some(color) = tl.color.sample(t) {
                if let Some(sprite) = world.get_mut::<Sprite>(entity) {
                    sprite.color = color;
                }
            }

            // Apply alpha (overrides sprite.color.a)
            if let Some(alpha) = tl.alpha.sample(t) {
                if let Some(sprite) = world.get_mut::<Sprite>(entity) {
                    sprite.color.a = alpha;
                }
            }

            // Put the timeline back
            world.add_component(entity, tl);
        }
    }
}

// ── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::components::Transform;
    use crate::ecs::{System, World};
    use crate::tween::Easing;
    use glam::Vec2;

    #[test]
    fn track_sample_before_first_keyframe() {
        let mut track: Track<f32> = Track::new();
        track.add(1.0, 10.0, Easing::Linear);
        assert_eq!(track.sample(0.0), Some(10.0));
    }

    #[test]
    fn track_sample_after_last_keyframe() {
        let mut track: Track<f32> = Track::new();
        track.add(0.0, 0.0, Easing::Linear);
        track.add(1.0, 100.0, Easing::Linear);
        assert_eq!(track.sample(2.0), Some(100.0));
    }

    #[test]
    fn track_linear_interpolation() {
        let mut track: Track<f32> = Track::new();
        track.add(0.0, 0.0, Easing::Linear);
        track.add(1.0, 100.0, Easing::Linear);
        let v = track.sample(0.5).unwrap();
        assert!((v - 50.0).abs() < 1e-4, "expected ~50, got {v}");
    }

    #[test]
    fn track_empty_returns_none() {
        let track: Track<f32> = Track::new();
        assert!(track.sample(0.5).is_none());
    }

    #[test]
    fn all_nan_keyframe_times_do_not_panic() {
        // Degenerate track where all keyframe times are NaN: rposition returns None,
        // which previously caused a .unwrap() panic. Now falls back to the first value.
        let mut track: Track<f32> = Track::new();
        track.add(f32::NAN, 1.0, Easing::Linear);
        track.add(f32::NAN, 2.0, Easing::Linear);
        let v = track.sample(0.5);
        assert!(v.is_some(), "all-NaN track should fall back, not panic");
    }

    #[test]
    fn timeline_is_finished_when_time_reached() {
        let mut tl = Timeline::new(1.0);
        tl.time = 1.0;
        assert!(tl.is_finished());
    }

    #[test]
    fn timeline_looping_not_finished() {
        let mut tl = Timeline::new(1.0).looping();
        tl.time = 1.5;
        assert!(!tl.is_finished());
    }

    #[test]
    fn track_vec2_lerp() {
        let mut track: Track<Vec2> = Track::new();
        track.add(0.0, Vec2::ZERO, Easing::Linear);
        track.add(1.0, Vec2::new(100.0, 0.0), Easing::Linear);
        let v = track.sample(0.25).unwrap();
        assert!((v.x - 25.0).abs() < 1e-4, "expected x≈25, got {}", v.x);
    }

    #[test]
    fn track_with_keyframes_sorted() {
        // Insert keyframes out of order — should still sort correctly
        let kfs = vec![
            Keyframe {
                time: 1.0,
                value: 100.0_f32,
                easing: Easing::Linear,
            },
            Keyframe {
                time: 0.0,
                value: 0.0_f32,
                easing: Easing::Linear,
            },
        ];
        let track = Track::with_keyframes(kfs);
        let v = track.sample(0.5).unwrap();
        assert!((v - 50.0).abs() < 1e-4, "expected ~50, got {v}");
    }

    #[test]
    fn track_sample_nan_returns_none() {
        let mut track: Track<f32> = Track::new();
        track.add(0.0, 0.0, Easing::Linear);
        track.add(1.0, 1.0, Easing::Linear);
        assert_eq!(track.sample(f32::NAN), None);
    }

    #[test]
    fn track_nan_keyframe_does_not_panic_normal_sample() {
        let mut track: Track<f32> = Track::new();
        track.add(f32::NAN, 99.0, Easing::Linear);
        track.add(0.0, 0.0, Easing::Linear);
        track.add(1.0, 1.0, Easing::Linear);
        // Normal time sampling should return a value without panicking even with a NaN keyframe
        let v = track.sample(0.5);
        assert!(
            v.is_some(),
            "normal sample must not panic with NaN keyframe"
        );
    }

    #[test]
    fn track_sorting_accepts_nan_times() {
        let mut track: Track<f32> = Track::new();
        track.add(f32::NAN, 1.0, Easing::Linear);
        track.add(0.0, 2.0, Easing::Linear);
        track.add(0.0, 3.0, Easing::Linear);

        assert_eq!(track.sample(0.0), Some(2.0));
        assert!(track.duration().is_nan());
    }

    #[test]
    fn track_duration() {
        let mut track: Track<f32> = Track::new();
        assert_eq!(track.duration(), 0.0);
        track.add(0.5, 1.0, Easing::Linear);
        track.add(2.0, 5.0, Easing::Linear);
        assert!((track.duration() - 2.0).abs() < 1e-6);
    }

    #[test]
    fn timeline_default_playing() {
        let tl = Timeline::new(1.0);
        assert!(tl.playing);
        assert!(!tl.looping);
        assert!(!tl.is_finished());
    }

    #[test]
    fn timeline_restart_resets_time() {
        let mut tl = Timeline::new(1.0);
        tl.time = 0.9;
        tl.pause();
        tl.restart();
        assert_eq!(tl.time, 0.0);
        assert!(tl.playing);
    }

    // ── CameraTarget camera driving ──────────────────────────────────────────────

    fn pan_timeline() -> Timeline {
        let mut tl = Timeline::new(2.0);
        tl.position.add(0.0, Vec2::new(0.0, 0.0), Easing::Linear);
        tl.position.add(2.0, Vec2::new(200.0, 0.0), Easing::Linear);
        tl
    }

    #[test]
    fn camera_target_timeline_drives_camera_position() {
        let mut world = World::new();
        world.insert_resource(Camera::default());
        let rig = world.spawn();
        world.add_component(rig, pan_timeline());
        world.add_component(rig, CameraTarget);

        // advance 1.0s into a 2.0s linear pan 0→200 → midpoint 100
        TimelineSystem.run(&mut world, 1.0);
        let cam = world.resource::<Camera>().unwrap();
        assert!(
            (cam.position.x - 100.0).abs() < 1e-3,
            "expected camera.x≈100, got {}",
            cam.position.x
        );
    }

    #[test]
    fn camera_target_timeline_drives_camera_zoom() {
        let mut world = World::new();
        world.insert_resource(Camera::default());
        let mut tl = Timeline::new(2.0);
        tl.zoom.add(0.0, 1.0, Easing::Linear);
        tl.zoom.add(2.0, 2.0, Easing::Linear);
        let rig = world.spawn();
        world.add_component(rig, tl);
        world.add_component(rig, CameraTarget);

        TimelineSystem.run(&mut world, 1.0);
        let cam = world.resource::<Camera>().unwrap();
        assert!(
            (cam.zoom - 1.5).abs() < 1e-3,
            "expected zoom≈1.5, got {}",
            cam.zoom
        );
    }

    #[test]
    fn camera_target_ignores_own_transform() {
        let mut world = World::new();
        world.insert_resource(Camera::default());
        let rig = world.spawn();
        world.add_component(rig, pan_timeline());
        world.add_component(rig, CameraTarget);
        world.add_component(
            rig,
            Transform {
                position: Vec2::new(7.0, 9.0),
                ..Default::default()
            },
        );

        TimelineSystem.run(&mut world, 1.0);
        // Camera moved …
        assert!((world.resource::<Camera>().unwrap().position.x - 100.0).abs() < 1e-3);
        // … but the rig's own Transform is left untouched (it's a virtual rig).
        assert_eq!(
            world.get::<Transform>(rig).unwrap().position,
            Vec2::new(7.0, 9.0)
        );
    }

    #[test]
    fn non_camera_timeline_leaves_camera_untouched() {
        let mut world = World::new();
        world.insert_resource(Camera::default());
        let e = world.spawn();
        world.add_component(e, pan_timeline());
        world.add_component(e, Transform::default());

        TimelineSystem.run(&mut world, 1.0);
        // Camera resource is untouched without the CameraTarget marker …
        let cam = world.resource::<Camera>().unwrap();
        assert_eq!(cam.position, Vec2::ZERO);
        assert_eq!(cam.zoom, 1.0);
        // … and the entity's own Transform is driven as before.
        assert!((world.get::<Transform>(e).unwrap().position.x - 100.0).abs() < 1e-3);
    }
}
