use std::collections::{HashMap, HashSet};

use glam::Vec2;
use rapier2d::prelude::*;

use crate::components::Transform;
use crate::ecs::{Entity, Events, System, World};
use crate::physics::body::PhysicsBody;
use crate::physics::events::{CollisionEvent, TriggerEvent};
use crate::physics::world::PhysicsWorld;

/// General-purpose physics system: steps the simulation every frame and syncs transforms.
///
/// Positions and sizes used when creating bodies in `PhysicsWorld` are in physics units.
/// This system multiplies Rapier results by `pixels_per_unit` to write pixel coordinates
/// into `Transform.position`. For example, `pixels_per_unit = 50.0` means 1 physics unit
/// equals 50 screen pixels.
///
/// In addition to position, it also **syncs the body's rotation angle to `Transform.rotation`
/// (radians)** — so sprites on freely-rotating bodies (hinge arms, etc.) turn with the
/// physics simulation. Bodies with `lock_rotation: true` always have angle 0, so syncing
/// them is a no-op.
///
/// # Setup
///
/// `PhysicsWorld` is managed as a **World resource**, not inside this system. Before use,
/// insert it via `world.insert_resource(physics)` and register only `PhysicsSystem::new(ppu)`
/// as a system.
///
/// ```ignore
/// let physics = PhysicsWorld::new(gravity);
/// app.world.insert_resource(physics);
/// app.add_system(PhysicsSystem::new(50.0));
/// ```
///
/// To access the physics world from other systems or game code, use
/// `world.resource_mut::<PhysicsWorld>()`.
pub struct PhysicsSystem {
    /// Pixels per physics unit ratio. Example: 50.0 → 1 unit = 50 px.
    pub pixels_per_unit: f32,
    active_contacts: HashSet<(ColliderHandle, ColliderHandle)>,
    active_intersections: HashSet<(ColliderHandle, ColliderHandle)>,
}

impl PhysicsSystem {
    pub fn new(pixels_per_unit: f32) -> Self {
        debug_assert!(
            pixels_per_unit > 0.0,
            "PhysicsSystem::new requires pixels_per_unit > 0"
        );
        Self {
            pixels_per_unit: pixels_per_unit.max(f32::EPSILON),
            active_contacts: HashSet::new(),
            active_intersections: HashSet::new(),
        }
    }
}

fn ordered_pair(a: ColliderHandle, b: ColliderHandle) -> (ColliderHandle, ColliderHandle) {
    if a.into_raw_parts() <= b.into_raw_parts() {
        (a, b)
    } else {
        (b, a)
    }
}

impl System for PhysicsSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(mut physics) = world.remove_resource::<PhysicsWorld>() else {
            return;
        };

        physics.step(dt);

        // ── Collision event diff ──────────────────────────────────────────────
        let col_map: HashMap<ColliderHandle, Entity> = world
            .query::<PhysicsBody>()
            .map(|(e, b)| (b.collider_handle, e))
            .collect();

        // Rapier preserves the collider1/collider2 order for the same pair across frames.
        let current: HashSet<(ColliderHandle, ColliderHandle)> = physics
            .narrow_phase
            .contact_pairs()
            .filter(|p| p.has_any_active_contact)
            .filter_map(|p| {
                col_map.get(&p.collider1)?;
                col_map.get(&p.collider2)?;
                Some((p.collider1, p.collider2))
            })
            .collect();

        let mut collision_events: Vec<CollisionEvent> = Vec::new();
        for &(c1, c2) in &current {
            if !self.active_contacts.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    collision_events.push(CollisionEvent::Started(e1, e2));
                }
            }
        }
        for &(c1, c2) in &self.active_contacts {
            if !current.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    collision_events.push(CollisionEvent::Stopped(e1, e2));
                }
            }
        }
        self.active_contacts = current;

        if !collision_events.is_empty() {
            if let Some(bus) = world.resource_mut::<Events<CollisionEvent>>() {
                for ev in collision_events {
                    bus.send(ev);
                }
            }
        }
        // ── end collision event diff ──────────────────────────────────────────

        // ── Sensor event diff ─────────────────────────────────────────────────
        let current_intersections: HashSet<(ColliderHandle, ColliderHandle)> = physics
            .narrow_phase
            .intersection_pairs()
            .filter(|(_, _, intersecting)| *intersecting)
            .filter_map(|(c1, c2, _)| {
                col_map.get(&c1)?;
                col_map.get(&c2)?;
                Some(ordered_pair(c1, c2))
            })
            .collect();

        let mut trigger_events: Vec<TriggerEvent> = Vec::new();
        for &(c1, c2) in &current_intersections {
            if !self.active_intersections.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    trigger_events.push(TriggerEvent::Entered(e1, e2));
                }
            }
        }
        for &(c1, c2) in &self.active_intersections {
            if !current_intersections.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    trigger_events.push(TriggerEvent::Exited(e1, e2));
                }
            }
        }
        self.active_intersections = current_intersections;

        if !trigger_events.is_empty() {
            if let Some(bus) = world.resource_mut::<Events<TriggerEvent>>() {
                for ev in trigger_events {
                    bus.send(ev);
                }
            }
        }
        // ── end sensor event diff ─────────────────────────────────────────────

        // borrow checker: collect (entity, handle) pairs first so we can re-borrow world
        let pairs: Vec<(Entity, RigidBodyHandle)> = world
            .query::<PhysicsBody>()
            .map(|(e, b)| (e, b.rigid_body_handle))
            .collect();

        let scale = self.pixels_per_unit.max(f32::EPSILON);
        for (entity, handle) in pairs {
            if let Some(body) = physics.rigid_body(handle) {
                let t = *body.translation();
                let angle = body.rotation().angle();
                if let Some(tr) = world.get_mut::<Transform>(entity) {
                    tr.position = Vec2::new(t.x * scale, t.y * scale);
                    tr.rotation = angle;
                }
            }
        }

        world.insert_resource(physics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Transform;
    use crate::physics::events::TriggerEvent;

    #[test]
    fn sensor_intersection_emits_trigger_entered() {
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (sensor_body, sensor_col) = physics.add_sensor_box(Vec2::ZERO, 1.0, 1.0);
        let (actor_body, actor_col) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);

        let mut world = World::new();
        world.insert_resource(physics);
        world.insert_resource(Events::<TriggerEvent>::default());
        let mut system = PhysicsSystem::new(1.0);

        let sensor = world.spawn();
        world.add_component(
            sensor,
            PhysicsBody {
                rigid_body_handle: sensor_body,
                collider_handle: sensor_col,
            },
        );
        world.add_component(sensor, Transform::default());

        let actor = world.spawn();
        world.add_component(
            actor,
            PhysicsBody {
                rigid_body_handle: actor_body,
                collider_handle: actor_col,
            },
        );
        world.add_component(actor, Transform::default());

        system.run(&mut world, 1.0 / 60.0);

        let events = world.resource::<Events<TriggerEvent>>().unwrap().read();
        assert_eq!(events, &[TriggerEvent::Entered(sensor, actor)]);
    }

    #[test]
    #[should_panic(expected = "PhysicsSystem::new requires pixels_per_unit > 0")]
    fn non_positive_pixels_per_unit_is_clamped() {
        let _ = PhysicsSystem::new(0.0);
    }

    #[test]
    fn syncs_body_rotation_to_transform() {
        // The angle of a rotating dynamic body must be synced to Transform.rotation.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb, col) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        physics.rigid_body_mut(rb).unwrap().set_angvel(3.0, true);

        let mut world = World::new();
        world.insert_resource(physics);
        let mut system = PhysicsSystem::new(1.0);
        let e = world.spawn();
        world.add_component(
            e,
            PhysicsBody {
                rigid_body_handle: rb,
                collider_handle: col,
            },
        );
        world.add_component(e, Transform::default());

        let dt = 1.0 / 60.0;
        system.run(&mut world, dt);

        let rotation = world.get_mut::<Transform>(e).unwrap().rotation;
        // angvel 3.0 rad/s integrated over dt → ~0.05 rad. Without sync it stays at 0.
        assert!(
            (rotation - 3.0 * dt).abs() < 1e-2,
            "rotation must match body angle (≈angvel*dt): {rotation}"
        );
    }

    #[test]
    fn locked_rotation_body_keeps_zero_rotation() {
        // A rotation-locked body stays at angle 0 even with torque — verifies syncing is harmless.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb, col) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, true);
        physics
            .rigid_body_mut(rb)
            .unwrap()
            .apply_torque_impulse(5.0, true);

        let mut world = World::new();
        world.insert_resource(physics);
        let mut system = PhysicsSystem::new(1.0);
        let e = world.spawn();
        world.add_component(
            e,
            PhysicsBody {
                rigid_body_handle: rb,
                collider_handle: col,
            },
        );
        world.add_component(e, Transform::default());

        system.run(&mut world, 1.0 / 60.0);

        let rotation = world.get_mut::<Transform>(e).unwrap().rotation;
        assert!(
            rotation.abs() < 1e-6,
            "rotation-locked body must keep rotation 0: {rotation}"
        );
    }
}
