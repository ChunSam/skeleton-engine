//! 2D cutout (rigged) skeletal animation.
//!
//! Bones are hierarchy entities (`Transform` + [`Parent`](crate::hierarchy::Parent)), with
//! sprite pieces attached to each bone. [`SkeletalClip`] animates the bone's **local
//! `Transform`** via keyframes; the [`HierarchySystem`](crate::hierarchy::HierarchySystem)
//! that runs automatically afterward composes `GlobalTransform`. The renderer prefers
//! `GlobalTransform`, so bone sprites are drawn without any extra render changes.
//!
//! ```no_run
//! use engine::{App, SkeletonBuilder, SkeletalClip, BoneTrack, BoneKeyframe,
//!     SkeletalAnimationSystem, components::{Transform, Sprite}};
//! use glam::Vec2;
//!
//! let mut app = App::new();
//! let mut b = SkeletonBuilder::new(&mut app.world, "hip",
//!     Transform { position: Vec2::new(480.0, 270.0), ..Default::default() });
//! b.add_bone(&mut app.world, "torso", "hip",
//!     Transform { position: Vec2::new(0.0, 40.0), ..Default::default() },
//!     Some(Sprite::colored(0.8, 0.7, 0.6)));
//! let clip = SkeletalClip {
//!     name: "idle".into(), duration: 1.0, looping: true,
//!     tracks: vec![BoneTrack { bone: "torso".into(), keys: vec![
//!         BoneKeyframe { time: 0.0, position: Vec2::new(0.0, 40.0), rotation: 0.0, scale: Vec2::ONE },
//!         BoneKeyframe { time: 0.5, position: Vec2::new(0.0, 40.0), rotation: 0.2, scale: Vec2::ONE },
//!         BoneKeyframe { time: 1.0, position: Vec2::new(0.0, 40.0), rotation: 0.0, scale: Vec2::ONE },
//!     ]}],
//! };
//! b.finish(&mut app.world, vec![clip]);
//! app.add_system(SkeletalAnimationSystem);
//! ```

use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

use glam::Vec2;

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};
use crate::hierarchy::attach;

/// The pose of a single bone at a single point in time (local `Transform` values).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoneKeyframe {
    /// Time in seconds from the start of the clip.
    pub time: f32,
    pub position: Vec2,
    /// Radians, Z-axis.
    pub rotation: f32,
    pub scale: Vec2,
}

/// A keyframe track for a single bone. `keys` must be sorted in ascending `time` order.
#[derive(Debug, Clone)]
pub struct BoneTrack {
    /// Key into the [`SkeletalAnimator`] bone-name map.
    pub bone: String,
    pub keys: Vec<BoneKeyframe>,
}

impl BoneTrack {
    /// Interpolates the pose at the given time. Position/scale use linear interpolation;
    /// rotation uses shortest-path angular interpolation.
    ///
    /// Returns `None` if there are no keys; clamps to the first/last key outside range.
    pub fn sample(&self, time: f32) -> Option<(Vec2, f32, Vec2)> {
        if self.keys.is_empty() {
            return None;
        }
        if time <= self.keys[0].time {
            let k = &self.keys[0];
            return Some((k.position, k.rotation, k.scale));
        }
        let last = self.keys.last().unwrap();
        if time >= last.time {
            return Some((last.position, last.rotation, last.scale));
        }
        // Find the two keys [a, b] bracketing time
        let i = self
            .keys
            .iter()
            .position(|k| k.time > time)
            .unwrap_or(self.keys.len() - 1);
        let a = &self.keys[i - 1];
        let b = &self.keys[i];
        let span = b.time - a.time;
        let t = if span > f32::EPSILON {
            (time - a.time) / span
        } else {
            0.0
        };
        Some((
            a.position.lerp(b.position, t),
            lerp_angle(a.rotation, b.rotation, t),
            a.scale.lerp(b.scale, t),
        ))
    }
}

/// A single skeletal animation clip composed of bone tracks.
#[derive(Debug, Clone)]
pub struct SkeletalClip {
    pub name: String,
    /// Clip length in seconds.
    pub duration: f32,
    pub looping: bool,
    pub tracks: Vec<BoneTrack>,
}

/// Animator component attached to the skeleton root entity.
///
/// [`SkeletalAnimationSystem`] advances `time` each frame, samples each track of the
/// current clip, and updates the local `Transform` of the corresponding bone entities.
#[derive(Debug, Clone)]
pub struct SkeletalAnimator {
    pub clips: Vec<SkeletalClip>,
    pub current: usize,
    /// Playback time within the current clip, in seconds.
    pub time: f32,
    /// Playback speed multiplier.
    pub speed: f32,
    pub playing: bool,
    /// Bone name → entity. Populated by [`SkeletonBuilder`].
    pub bones: HashMap<String, Entity>,
    /// Set to `true` by [`SkeletalAnimationSystem`] on the first tick. Guards `is_finished()`
    /// against reporting `true` at construction time for a zero-duration non-looping clip
    /// (before any system update has run).
    pub(crate) started: bool,
}

impl SkeletalAnimator {
    pub fn new(clips: Vec<SkeletalClip>, bones: HashMap<String, Entity>) -> Self {
        Self {
            clips,
            current: 0,
            time: 0.0,
            speed: 1.0,
            playing: true,
            bones,
            started: false,
        }
    }

    /// Switches to the given clip index and resets time to 0. No-op if already playing that clip.
    pub fn play(&mut self, clip_index: usize) {
        if self.current != clip_index {
            self.current = clip_index;
            self.time = 0.0;
            self.playing = true;
        }
    }

    /// Switches to a clip by name. Returns `true` if found.
    pub fn play_named(&mut self, name: &str) -> bool {
        if let Some(i) = self.clips.iter().position(|c| c.name == name) {
            self.play(i);
            true
        } else {
            false
        }
    }

    /// Returns `true` if a non-looping clip has played to the end. Always `false` for looping
    /// clips, missing clips, or before the first system tick.
    ///
    /// The `started` guard prevents this from returning `true` at construction time for a
    /// `duration == 0.0` non-looping clip — the system must run at least once first so callers
    /// observe a frame in the constructed state before it is considered finished. After that
    /// first tick a zero-duration non-looping clip is reported finished (it has no content left
    /// to play).
    pub fn is_finished(&self) -> bool {
        if !self.started {
            return false;
        }
        match self.clips.get(self.current) {
            Some(c) => !c.looping && self.time >= c.duration,
            None => false,
        }
    }
}

/// System that advances every [`SkeletalAnimator`] each frame and updates bone local `Transform`s.
///
/// Must be registered in the user system stage (`app.add_system(SkeletalAnimationSystem)`).
/// The `HierarchySystem` that runs automatically afterward composes `GlobalTransform`.
pub struct SkeletalAnimationSystem;

impl SkeletalAnimationSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::skeletal_animation";
}

impl System for SkeletalAnimationSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let animators: Vec<Entity> = world.query::<SkeletalAnimator>().map(|(e, _)| e).collect();

        for animator_entity in animators {
            // 1) Advance time + collect samples (animator borrow ends inside this block)
            let samples: Vec<(Entity, Vec2, f32, Vec2)> = {
                let Some(anim) = world.get_mut::<SkeletalAnimator>(animator_entity) else {
                    continue;
                };
                // Mark as started so is_finished() can return true after at least one tick.
                anim.started = true;
                if !anim.playing {
                    continue;
                }
                let Some(clip) = anim.clips.get(anim.current) else {
                    continue;
                };
                let duration = clip.duration;
                anim.time += dt * anim.speed;
                if clip.looping {
                    if duration > f32::EPSILON {
                        anim.time = anim.time.rem_euclid(duration);
                    }
                } else if anim.time >= duration {
                    anim.time = duration;
                }
                let time = anim.time;

                // borrow workaround: re-read tracks to build a (bone_entity, TRS) list
                let clip = &anim.clips[anim.current];
                clip.tracks
                    .iter()
                    .filter_map(|track| {
                        let bone = *anim.bones.get(&track.bone)?;
                        let (p, r, s) = track.sample(time)?;
                        Some((bone, p, r, s))
                    })
                    .collect()
            };

            // 2) After releasing the animator borrow, update each bone Transform
            for (bone, position, rotation, scale) in samples {
                if let Some(t) = world.get_mut::<Transform>(bone) {
                    t.position = position;
                    t.rotation = rotation;
                    t.scale = scale;
                }
            }
        }
    }
}

/// Authoring helper that spawns the bone hierarchy and builds the name→entity map.
///
/// Internally uses [`crate::hierarchy::attach`] to manage `Parent`/`Children`.
pub struct SkeletonBuilder {
    root: Entity,
    bones: HashMap<String, Entity>,
}

impl SkeletonBuilder {
    /// Spawns the root bone. `root_transform` is the world-space origin of the entire skeleton.
    pub fn new(world: &mut World, root_name: impl Into<String>, root_transform: Transform) -> Self {
        let root = world.spawn();
        world.add_component(root, root_transform);
        let mut bones = HashMap::new();
        bones.insert(root_name.into(), root);
        Self { root, bones }
    }

    /// The root entity.
    pub fn root(&self) -> Entity {
        self.root
    }

    /// Looks up a previously added bone entity by name.
    pub fn bone(&self, name: &str) -> Option<Entity> {
        self.bones.get(name).copied()
    }

    /// Adds a bone and attaches it to the `parent_name` bone. Attaches `sprite` if provided.
    ///
    /// Falls back to the root if `parent_name` is not yet registered.
    pub fn add_bone(
        &mut self,
        world: &mut World,
        name: impl Into<String>,
        parent_name: &str,
        local_transform: Transform,
        sprite: Option<Sprite>,
    ) -> Entity {
        let parent = self.bones.get(parent_name).copied().unwrap_or(self.root);
        let bone = world.spawn();
        world.add_component(bone, local_transform);
        if let Some(s) = sprite {
            world.add_component(bone, s);
        }
        attach(world, bone, parent);
        self.bones.insert(name.into(), bone);
        bone
    }

    /// Inserts a [`SkeletalAnimator`] on the root entity and returns it.
    pub fn finish(self, world: &mut World, clips: Vec<SkeletalClip>) -> Entity {
        world.add_component(self.root, SkeletalAnimator::new(clips, self.bones));
        self.root
    }
}

/// Shortest-path linear interpolation between two angles (radians).
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = (b - a).rem_euclid(TAU);
    if diff > PI {
        diff -= TAU;
    }
    a + diff * t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kf(time: f32, x: f32, rot: f32) -> BoneKeyframe {
        BoneKeyframe {
            time,
            position: Vec2::new(x, 0.0),
            rotation: rot,
            scale: Vec2::ONE,
        }
    }

    #[test]
    fn sample_clamps_outside_range() {
        let track = BoneTrack {
            bone: "b".into(),
            keys: vec![kf(0.0, 0.0, 0.0), kf(1.0, 10.0, 0.0)],
        };
        assert_eq!(track.sample(-1.0).unwrap().0.x, 0.0);
        assert_eq!(track.sample(2.0).unwrap().0.x, 10.0);
    }

    #[test]
    fn sample_interpolates_midpoint() {
        let track = BoneTrack {
            bone: "b".into(),
            keys: vec![kf(0.0, 0.0, 0.0), kf(2.0, 10.0, 0.0)],
        };
        let (p, _, _) = track.sample(1.0).unwrap();
        assert!((p.x - 5.0).abs() < 1e-3);
    }

    #[test]
    fn lerp_angle_takes_shortest_path() {
        // 350° → 10° shortest path is +20° (not -340° going the long way)
        let a = 350f32.to_radians();
        let b = 10f32.to_radians();
        let mid = lerp_angle(a, b, 0.5);
        // midpoint should be near 0° (=360°)
        let mid_deg = mid.to_degrees().rem_euclid(360.0);
        assert!(
            !(5.0..=355.0).contains(&mid_deg),
            "expected ~0deg, got {mid_deg}"
        );
    }

    #[test]
    fn empty_track_samples_none() {
        let track = BoneTrack {
            bone: "b".into(),
            keys: vec![],
        };
        assert!(track.sample(0.5).is_none());
    }

    // ── started-guard tests (fix 4) ────────────────────────────────────────────

    /// A duration=0.0 non-looping clip must report `is_finished()==false` before
    /// the first `SkeletalAnimationSystem` tick.
    #[test]
    fn zero_duration_nonlooping_clip_not_finished_before_first_tick() {
        let mut world = World::new();
        let b = SkeletonBuilder::new(
            &mut world,
            "root",
            Transform {
                position: Vec2::ZERO,
                ..Default::default()
            },
        );
        let clip = SkeletalClip {
            name: "instant".into(),
            duration: 0.0,
            looping: false,
            tracks: vec![],
        };
        let root = b.finish(&mut world, vec![clip]);

        // Before any system tick: must not be finished.
        assert!(
            !world.get::<SkeletalAnimator>(root).unwrap().is_finished(),
            "duration=0 non-looping clip must not be finished before the first tick"
        );

        // After one tick: now finished (started=true, time=0 >= duration=0).
        SkeletalAnimationSystem.run(&mut world, 0.01);
        assert!(
            world.get::<SkeletalAnimator>(root).unwrap().is_finished(),
            "duration=0 non-looping clip must be finished after the first tick"
        );
    }

    /// A normal non-looping clip with duration > 0 must not be finished before time >= duration.
    #[test]
    fn nonlooping_clip_not_finished_before_duration_elapsed() {
        let mut world = World::new();
        let b = SkeletonBuilder::new(
            &mut world,
            "root",
            Transform {
                position: Vec2::ZERO,
                ..Default::default()
            },
        );
        let clip = SkeletalClip {
            name: "one_shot".into(),
            duration: 0.5,
            looping: false,
            tracks: vec![],
        };
        let root = b.finish(&mut world, vec![clip]);

        // Before any tick: not finished.
        assert!(
            !world.get::<SkeletalAnimator>(root).unwrap().is_finished(),
            "non-looping clip must not be finished before first tick"
        );

        // One tick that doesn't reach duration.
        SkeletalAnimationSystem.run(&mut world, 0.2);
        assert!(
            !world.get::<SkeletalAnimator>(root).unwrap().is_finished(),
            "non-looping clip must not be finished before time >= duration"
        );

        // Advance past duration.
        SkeletalAnimationSystem.run(&mut world, 0.4);
        assert!(
            world.get::<SkeletalAnimator>(root).unwrap().is_finished(),
            "non-looping clip must be finished after time >= duration"
        );
    }

    #[test]
    fn system_drives_bone_transform_and_loops() {
        let mut world = World::new();
        let mut b = SkeletonBuilder::new(
            &mut world,
            "root",
            Transform {
                position: Vec2::ZERO,
                ..Default::default()
            },
        );
        b.add_bone(
            &mut world,
            "arm",
            "root",
            Transform {
                position: Vec2::ZERO,
                ..Default::default()
            },
            None,
        );
        let clip = SkeletalClip {
            name: "wave".into(),
            duration: 2.0,
            looping: true,
            tracks: vec![BoneTrack {
                bone: "arm".into(),
                keys: vec![kf(0.0, 0.0, 0.0), kf(2.0, 20.0, 0.0)],
            }],
        };
        let arm = b.bone("arm").unwrap();
        let root = b.finish(&mut world, vec![clip]);

        // Advance 1 s → arm.x ≈ 10
        SkeletalAnimationSystem.run(&mut world, 1.0);
        let x = world.get::<Transform>(arm).unwrap().position.x;
        assert!((x - 10.0).abs() < 1e-3, "expected 10, got {x}");

        // Advance another 1.5 s (cumulative 2.5 → wraps to 0.5 after loop) → arm.x ≈ 5
        SkeletalAnimationSystem.run(&mut world, 1.5);
        let x = world.get::<Transform>(arm).unwrap().position.x;
        assert!((x - 5.0).abs() < 1e-3, "expected 5 after loop, got {x}");

        // Verify clip switching via play_named
        let anim = world.get_mut::<SkeletalAnimator>(root).unwrap();
        assert!(anim.play_named("wave"));
        assert!(!anim.play_named("nope"));
    }
}
