//! Steering Behaviors system (Phase 37a)
//!
//! Reads each entity's `Transform.position`, computes a velocity vector toward the
//! desired direction, and stores it in `SteeringVelocity`. `SteeringSystem` then
//! applies the velocity to `Transform` every frame.
//!
//! # Included behaviors
//! - [`Seek`]   — move toward a target at full speed
//! - [`Flee`]   — flee from a target (only when within `flee_radius`)
//! - [`Arrive`] — decelerate as the target nears; stop within `stop_radius`
//! - [`Wander`] — roam in a random direction (changes every `change_interval`)
//!
//! # Registration example
//! ```rust,no_run
//! use engine::steering::{Seek, SteeringSystem, SteeringVelocity};
//! use engine::{App, Transform};
//! use glam::Vec2;
//!
//! let mut app = App::new();
//! let e = app.world.spawn();
//! app.world.add_component(e, Transform::default());
//! app.world.add_component(e, SteeringVelocity { velocity: Vec2::ZERO, max_speed: 200.0 });
//! app.world.add_component(e, Seek { target: Vec2::new(400.0, 300.0), max_speed: 200.0 });
//! app.add_system(SteeringSystem);
//! ```

use glam::Vec2;

use crate::components::Transform;
use crate::ecs::{Entity, World};
use crate::System;

// ─── SteeringVelocity ─────────────────────────────────────────────────────────

/// Component that stores the result of steering calculations.
///
/// `SteeringSystem` evaluates each steering behavior component (Seek/Flee/Arrive/Wander),
/// updates this field, and finally applies it to `Transform.position`.
#[derive(Debug, Clone, Default)]
pub struct SteeringVelocity {
    pub velocity: Vec2,
    pub max_speed: f32,
}

// ─── Seek ─────────────────────────────────────────────────────────────────────

/// Move in a straight line toward the target at full speed.
#[derive(Debug, Clone)]
pub struct Seek {
    pub target: Vec2,
    pub max_speed: f32,
}

// ─── Flee ─────────────────────────────────────────────────────────────────────

/// Flee from a target position. Only activates when within `flee_radius`.
#[derive(Debug, Clone)]
pub struct Flee {
    pub target: Vec2,
    pub max_speed: f32,
    /// Flee velocity is only generated when within this distance.
    pub flee_radius: f32,
}

// ─── Arrive ───────────────────────────────────────────────────────────────────

/// Decelerate as the target nears. Come to a full stop within `stop_radius`.
#[derive(Debug, Clone)]
pub struct Arrive {
    pub target: Vec2,
    pub max_speed: f32,
    /// Begin decelerating within this distance.
    pub slow_radius: f32,
    /// Zero out velocity within this distance.
    pub stop_radius: f32,
}

// ─── Wander ───────────────────────────────────────────────────────────────────

/// Roam in a random direction, changing direction every `change_interval`.
#[derive(Debug, Clone)]
pub struct Wander {
    pub max_speed: f32,
    /// Direction change interval (seconds).
    pub change_interval: f32,
    pub(crate) timer: f32,
    pub(crate) current_dir: Vec2,
}

impl Wander {
    pub fn new(max_speed: f32, change_interval: f32) -> Self {
        Self {
            max_speed,
            change_interval,
            timer: 0.0,
            current_dir: Vec2::X,
        }
    }

    /// Like [`Wander::new`] but sets an explicit initial wander direction instead of
    /// the default `Vec2::X`. `dir` is stored as-is (not normalized); the built-in
    /// direction picker will overwrite it on the first `change_interval` expiry.
    ///
    /// # Note
    ///
    /// The built-in direction picker is a **deterministic placeholder** based on
    /// `entity.index()` and the previous direction. Entities created in the same
    /// order will always wander the same pattern. Pass a random initial direction
    /// (or replace the direction-update logic in a subclassed system) for truly
    /// varied behaviour.
    pub fn with_initial_dir(max_speed: f32, change_interval: f32, dir: Vec2) -> Self {
        Self {
            max_speed,
            change_interval,
            timer: 0.0,
            current_dir: dir,
        }
    }
}

// ─── SteeringSystem ───────────────────────────────────────────────────────────

/// System that evaluates steering behavior components every frame and moves `Transform`.
///
/// Evaluation order is **Seek → Flee → Arrive → Wander**. When an entity has
/// more than one steering component, each pass overwrites `SteeringVelocity` —
/// the **last behavior evaluated wins** (silent last-wins). Attach only one
/// steering component per entity unless the intentional behaviour is to have
/// Wander always override Seek/Flee/Arrive.
pub struct SteeringSystem;

impl SteeringSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::steering";
}

impl System for SteeringSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── 1. Seek ────────────────────────────────────────────────────────────
        {
            let entities: Vec<Entity> = world.query::<Seek>().map(|(e, _)| e).collect();

            for entity in entities {
                let (pos, target, max_speed) = {
                    let t = world.get::<Transform>(entity).map(|t| t.position);
                    let seek = world.get::<Seek>(entity).map(|s| (s.target, s.max_speed));
                    match (t, seek) {
                        (Some(p), Some((tgt, ms))) => (p, tgt, ms),
                        _ => continue,
                    }
                };

                let dir = target - pos;
                let velocity = if dir.length_squared() > 1e-6 {
                    dir.normalize() * max_speed
                } else {
                    Vec2::ZERO
                };

                if let Some(sv) = world.get_mut::<SteeringVelocity>(entity) {
                    sv.velocity = velocity;
                    sv.max_speed = max_speed;
                }
            }
        }

        // ── 2. Flee ────────────────────────────────────────────────────────────
        {
            let entities: Vec<Entity> = world.query::<Flee>().map(|(e, _)| e).collect();

            for entity in entities {
                let (pos, target, max_speed, flee_radius) = {
                    let t = world.get::<Transform>(entity).map(|t| t.position);
                    let flee = world
                        .get::<Flee>(entity)
                        .map(|f| (f.target, f.max_speed, f.flee_radius));
                    match (t, flee) {
                        (Some(p), Some((tgt, ms, fr))) => (p, tgt, ms, fr),
                        _ => continue,
                    }
                };

                let diff = pos - target;
                let dist = diff.length();
                let velocity = if dist < flee_radius && dist > 1e-6 {
                    diff.normalize() * max_speed
                } else {
                    Vec2::ZERO
                };

                if let Some(sv) = world.get_mut::<SteeringVelocity>(entity) {
                    sv.velocity = velocity;
                    sv.max_speed = max_speed;
                }
            }
        }

        // ── 3. Arrive ──────────────────────────────────────────────────────────
        {
            let entities: Vec<Entity> = world.query::<Arrive>().map(|(e, _)| e).collect();

            for entity in entities {
                let (pos, target, max_speed, slow_radius, stop_radius) = {
                    let t = world.get::<Transform>(entity).map(|t| t.position);
                    let arrive = world
                        .get::<Arrive>(entity)
                        .map(|a| (a.target, a.max_speed, a.slow_radius, a.stop_radius));
                    match (t, arrive) {
                        (Some(p), Some((tgt, ms, sr, pr))) => (p, tgt, ms, sr, pr),
                        _ => continue,
                    }
                };

                let dir = target - pos;
                let dist = dir.length();
                let velocity = if dist <= stop_radius {
                    Vec2::ZERO
                } else if dist <= slow_radius {
                    // Linear deceleration
                    let ratio = (dist - stop_radius) / (slow_radius - stop_radius);
                    dir.normalize() * max_speed * ratio
                } else if dist > 1e-6 {
                    dir.normalize() * max_speed
                } else {
                    Vec2::ZERO
                };

                if let Some(sv) = world.get_mut::<SteeringVelocity>(entity) {
                    sv.velocity = velocity;
                    sv.max_speed = max_speed;
                }
            }
        }

        // ── 4. Wander ─────────────────────────────────────────────────────────
        {
            let entities: Vec<Entity> = world.query::<Wander>().map(|(e, _)| e).collect();

            for entity in entities {
                // Advance timer and determine direction
                let (max_speed, current_dir) = {
                    let wander = match world.get_mut::<Wander>(entity) {
                        Some(w) => w,
                        None => continue,
                    };
                    wander.timer += dt;
                    if wander.timer >= wander.change_interval {
                        wander.timer = 0.0;
                        // Pseudo-random direction: simple deterministic calculation based on entity id
                        // (use the `rand` crate in a real project)
                        let seed =
                            (entity.index() as f32 * 1.6180339) + wander.current_dir.x * 31.7;
                        let angle = (seed.sin() * 6283.185).abs() % std::f32::consts::TAU;
                        wander.current_dir = Vec2::new(angle.cos(), angle.sin());
                    }
                    (wander.max_speed, wander.current_dir)
                };

                if let Some(sv) = world.get_mut::<SteeringVelocity>(entity) {
                    sv.velocity = current_dir * max_speed;
                    sv.max_speed = max_speed;
                }
            }
        }

        // ── 5. Apply movement to Transform ────────────────────────────────────
        {
            let entities: Vec<Entity> = world.query::<SteeringVelocity>().map(|(e, _)| e).collect();

            for entity in entities {
                let velocity = match world.get::<SteeringVelocity>(entity).map(|sv| sv.velocity) {
                    Some(v) => v,
                    None => continue,
                };

                if let Some(transform) = world.get_mut::<Transform>(entity) {
                    transform.position += velocity * dt;
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "SteeringSystem"
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Transform;
    use crate::ecs::World;
    use glam::Vec2;

    fn make_world_with_transform(pos: Vec2) -> (World, Entity) {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        (world, e)
    }

    #[test]
    fn seek_generates_velocity_toward_target() {
        let (mut world, e) = make_world_with_transform(Vec2::ZERO);
        world.add_component(e, SteeringVelocity::default());
        world.add_component(
            e,
            Seek {
                target: Vec2::new(100.0, 0.0),
                max_speed: 200.0,
            },
        );

        let mut sys = SteeringSystem;
        sys.run(&mut world, 0.016);

        let sv = world
            .query::<SteeringVelocity>()
            .find(|(en, _)| *en == e)
            .map(|(_, sv)| sv.velocity)
            .unwrap();

        // Should move right (+x)
        assert!(sv.x > 0.0, "velocity.x should be positive, got {}", sv.x);
        assert!(sv.y.abs() < 1e-4, "velocity.y should be ~0, got {}", sv.y);
        let speed = sv.length();
        assert!(
            (speed - 200.0).abs() < 1e-3,
            "speed should equal max_speed=200, got {speed}"
        );
    }

    #[test]
    fn arrive_stops_within_stop_radius() {
        // Same position as target — within stop_radius(5.0)
        let (mut world, e) = make_world_with_transform(Vec2::new(1.0, 0.0));
        world.add_component(e, SteeringVelocity::default());
        world.add_component(
            e,
            Arrive {
                target: Vec2::new(2.0, 0.0), // distance 1.0 < stop_radius=5.0
                max_speed: 200.0,
                slow_radius: 50.0,
                stop_radius: 5.0,
            },
        );

        let mut sys = SteeringSystem;
        sys.run(&mut world, 0.016);

        let sv = world
            .query::<SteeringVelocity>()
            .find(|(en, _)| *en == e)
            .map(|(_, sv)| sv.velocity)
            .unwrap();

        assert!(
            sv.length() < 1e-5,
            "velocity should be ~0 within stop_radius, got {sv:?}"
        );
    }

    #[test]
    fn flee_zero_velocity_outside_radius() {
        // flee_radius = 50, entity is 100 away from target
        let (mut world, e) = make_world_with_transform(Vec2::new(100.0, 0.0));
        world.add_component(e, SteeringVelocity::default());
        world.add_component(
            e,
            Flee {
                target: Vec2::ZERO,
                max_speed: 200.0,
                flee_radius: 50.0,
            },
        );

        let mut sys = SteeringSystem;
        sys.run(&mut world, 0.016);

        let sv = world
            .query::<SteeringVelocity>()
            .find(|(en, _)| *en == e)
            .map(|(_, sv)| sv.velocity)
            .unwrap();

        assert!(
            sv.length() < 1e-5,
            "velocity outside flee_radius should be 0, got {sv:?}"
        );
    }

    #[test]
    fn flee_generates_velocity_inside_radius() {
        // distance 30 < flee_radius 50
        let (mut world, e) = make_world_with_transform(Vec2::new(30.0, 0.0));
        world.add_component(e, SteeringVelocity::default());
        world.add_component(
            e,
            Flee {
                target: Vec2::ZERO,
                max_speed: 200.0,
                flee_radius: 50.0,
            },
        );

        let mut sys = SteeringSystem;
        sys.run(&mut world, 0.016);

        let sv = world
            .query::<SteeringVelocity>()
            .find(|(en, _)| *en == e)
            .map(|(_, sv)| sv.velocity)
            .unwrap();

        // Flee direction = +x (target is origin, position is at +x)
        assert!(
            sv.x > 0.0,
            "flee velocity.x should be positive (away from origin)"
        );
        assert!(
            (sv.length() - 200.0).abs() < 1e-3,
            "flee speed should be max_speed when inside radius"
        );
    }

    /// Regression guard for the O(1) per-entity lookup refactor (was O(N²)
    /// `query().find()` self-scans): with many seekers, *every* one must resolve
    /// its own `Seek`/`Transform`/`SteeringVelocity` and advance toward a shared
    /// target in a single tick. A lookup that crossed entity wires would fail this.
    #[test]
    fn many_seekers_each_advance_toward_shared_target() {
        let mut world = World::new();
        let target = Vec2::new(500.0, 500.0);
        let n = 64;

        // Ring of seekers around the target, each at a distinct position.
        let mut entities = Vec::with_capacity(n);
        for i in 0..n {
            let angle = (i as f32 / n as f32) * std::f32::consts::TAU;
            let pos = target + Vec2::new(angle.cos(), angle.sin()) * 200.0;
            let e = world.spawn();
            world.add_component(
                e,
                Transform {
                    position: pos,
                    scale: Vec2::ONE,
                    rotation: 0.0,
                    z: 0.0,
                },
            );
            world.add_component(e, SteeringVelocity::default());
            world.add_component(
                e,
                Seek {
                    target,
                    max_speed: 100.0,
                },
            );
            entities.push((e, pos));
        }

        let dt = 0.1;
        let mut sys = SteeringSystem;
        sys.run(&mut world, dt);

        for (e, start) in entities {
            let now = world
                .query::<Transform>()
                .find(|(en, _)| *en == e)
                .map(|(_, t)| t.position)
                .unwrap();
            // Each seeker moved (Transform changed) and got strictly closer to
            // the shared target — proving its own components were read, not a
            // neighbour's.
            assert!(
                now.distance(target) < start.distance(target) - 1.0,
                "seeker {e:?} should be closer to target after one tick"
            );
            let expected_speed = 100.0 * dt;
            assert!(
                (now.distance(start) - expected_speed).abs() < 1e-2,
                "seeker {e:?} should advance max_speed*dt = {expected_speed}"
            );
        }
    }
}
