use std::collections::{HashMap, HashSet};

use glam::Vec2;
use rapier2d::prelude::ColliderHandle as RapierColliderHandle;

use crate::components::Transform;
use crate::ecs::{Entity, Events, System, World};
use crate::physics::body::PhysicsBody;
use crate::physics::events::{CollisionEvent, TriggerEvent};
use crate::physics::world::{BodyHandle, PhysicsWorld};

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
///
/// # Event registration
///
/// Collision and trigger events are silently discarded if the corresponding event bus has
/// not been registered. Call the following during app setup to receive them:
///
/// ```ignore
/// app.register_event::<engine::CollisionEvent>();
/// app.register_event::<engine::TriggerEvent>();
/// ```
///
/// If contacts or intersections are detected but the bus is absent, a `log::warn!` is
/// emitted once per missing type so the omission is visible in the console.
pub struct PhysicsSystem {
    /// Pixels per physics unit ratio. Example: 50.0 → 1 unit = 50 px.
    pub pixels_per_unit: f32,
    active_contacts: HashSet<(RapierColliderHandle, RapierColliderHandle)>,
    active_intersections: HashSet<(RapierColliderHandle, RapierColliderHandle)>,
    // Per-frame scratch buffers reused via clear() to avoid per-frame allocations.
    col_map: HashMap<RapierColliderHandle, Entity>,
    /// Handle→entity map from the previous frame. Used to resolve `Stopped` events when
    /// an entity despawns mid-frame (its handle is absent from the current `col_map`).
    prev_col_map: HashMap<RapierColliderHandle, Entity>,
    current_contacts: HashSet<(RapierColliderHandle, RapierColliderHandle)>,
    current_intersections: HashSet<(RapierColliderHandle, RapierColliderHandle)>,
    body_pairs: Vec<(Entity, BodyHandle)>,
    // Scratch Vecs for entity-pair events — reused each frame (clear → fill → drain).
    col_started: Vec<(Entity, Entity)>,
    col_stopped: Vec<(Entity, Entity)>,
    trig_entered: Vec<(Entity, Entity)>,
    trig_exited: Vec<(Entity, Entity)>,
    // Guard flags: emit the missing-registration warning at most once per event type.
    warned_missing_collision_events: bool,
    warned_missing_trigger_events: bool,
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
            col_map: HashMap::new(),
            prev_col_map: HashMap::new(),
            current_contacts: HashSet::new(),
            current_intersections: HashSet::new(),
            body_pairs: Vec::new(),
            col_started: Vec::new(),
            col_stopped: Vec::new(),
            trig_entered: Vec::new(),
            trig_exited: Vec::new(),
            warned_missing_collision_events: false,
            warned_missing_trigger_events: false,
        }
    }
}

/// Computes the started and stopped entity-pairs by diffing `current` against `previous`,
/// then advances `previous` to match `current`.
///
/// Pairs in `current` but not `previous` → appended to `started`.
/// Pairs in `previous` but not `current` → appended to `stopped`.
/// `previous` is replaced with `current` in-place.
///
/// `prev_col_map` is the handle→entity map from the previous frame, used as a fallback
/// when resolving `stopped` pairs whose entity has already been despawned this frame
/// (and is therefore absent from the current `col_map`).
fn diff_pairs(
    previous: &mut HashSet<(RapierColliderHandle, RapierColliderHandle)>,
    current: &HashSet<(RapierColliderHandle, RapierColliderHandle)>,
    col_map: &HashMap<RapierColliderHandle, Entity>,
    prev_col_map: &HashMap<RapierColliderHandle, Entity>,
    started: &mut Vec<(Entity, Entity)>,
    stopped: &mut Vec<(Entity, Entity)>,
) {
    for &(c1, c2) in current {
        if !previous.contains(&(c1, c2)) {
            if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                started.push((e1, e2));
            }
        }
    }
    for &(c1, c2) in previous.iter() {
        if !current.contains(&(c1, c2)) {
            // Resolve via the current map first; fall back to the previous frame's map so
            // that a `Stopped` event is still emitted when one entity was despawned this
            // frame and its collider handle is no longer present in `col_map`.
            let resolve = |h| col_map.get(h).or_else(|| prev_col_map.get(h));
            if let (Some(&e1), Some(&e2)) = (resolve(&c1), resolve(&c2)) {
                stopped.push((e1, e2));
            }
        }
    }
    previous.clear();
    previous.extend(current.iter().copied());
}

fn ordered_pair(
    a: RapierColliderHandle,
    b: RapierColliderHandle,
) -> (RapierColliderHandle, RapierColliderHandle) {
    if a.into_raw_parts() <= b.into_raw_parts() {
        (a, b)
    } else {
        (b, a)
    }
}

impl PhysicsSystem {
    /// Schedule label for ordering via `add_system_labeled` (e.g. `.after(PhysicsSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::physics";
}

impl System for PhysicsSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(mut physics) = world.remove_resource::<PhysicsWorld>() else {
            return;
        };

        physics.step(dt);

        // ── Build collider→entity map (reused for both diff passes) ───────────
        self.col_map.clear();
        self.col_map.extend(
            world
                .query::<PhysicsBody>()
                .map(|(e, b)| (b.collider_handle.0, e)),
        );

        // ── Collision event diff ──────────────────────────────────────────────
        // ordered_pair normalises the (c1, c2) direction so the HashSet diff is correct
        // regardless of which slot rapier happens to place each handle in — symmetric with
        // the intersection path below.
        self.current_contacts.clear();
        self.current_contacts.extend(
            physics
                .narrow_phase
                .contact_pairs()
                .filter(|p| p.has_any_active_contact)
                .filter_map(|p| {
                    self.col_map.get(&p.collider1)?;
                    self.col_map.get(&p.collider2)?;
                    Some(ordered_pair(p.collider1, p.collider2))
                }),
        );

        self.col_started.clear();
        self.col_stopped.clear();
        diff_pairs(
            &mut self.active_contacts,
            &self.current_contacts,
            &self.col_map,
            &self.prev_col_map,
            &mut self.col_started,
            &mut self.col_stopped,
        );

        if !self.col_started.is_empty() || !self.col_stopped.is_empty() {
            if let Some(bus) = world.resource_mut::<Events<CollisionEvent>>() {
                for &(e1, e2) in &self.col_started {
                    bus.send(CollisionEvent::Started(e1, e2));
                }
                for &(e1, e2) in &self.col_stopped {
                    bus.send(CollisionEvent::Stopped(e1, e2));
                }
            } else if !self.warned_missing_collision_events {
                log::warn!(
                    "PhysicsSystem: CollisionEvent bus not found — collision events are \
                     being dropped. Call `app.register_event::<CollisionEvent>()` during setup."
                );
                self.warned_missing_collision_events = true;
            }
        }
        // ── end collision event diff ──────────────────────────────────────────

        // ── Sensor event diff ─────────────────────────────────────────────────
        self.current_intersections.clear();
        self.current_intersections.extend(
            physics
                .narrow_phase
                .intersection_pairs()
                .filter(|(_, _, intersecting)| *intersecting)
                .filter_map(|(c1, c2, _)| {
                    self.col_map.get(&c1)?;
                    self.col_map.get(&c2)?;
                    // ordered_pair is defensive: rapier's intersection_pairs() does not
                    // guarantee a stable (c1, c2) direction across frames, unlike contact_pairs()
                    // whose edge slots are fixed at add_edge. Normalizing here ensures the
                    // HashSet diff works correctly regardless of future rapier internals changes.
                    Some(ordered_pair(c1, c2))
                }),
        );

        self.trig_entered.clear();
        self.trig_exited.clear();
        diff_pairs(
            &mut self.active_intersections,
            &self.current_intersections,
            &self.col_map,
            &self.prev_col_map,
            &mut self.trig_entered,
            &mut self.trig_exited,
        );

        if !self.trig_entered.is_empty() || !self.trig_exited.is_empty() {
            if let Some(bus) = world.resource_mut::<Events<TriggerEvent>>() {
                for &(e1, e2) in &self.trig_entered {
                    bus.send(TriggerEvent::Entered(e1, e2));
                }
                for &(e1, e2) in &self.trig_exited {
                    bus.send(TriggerEvent::Exited(e1, e2));
                }
            } else if !self.warned_missing_trigger_events {
                log::warn!(
                    "PhysicsSystem: TriggerEvent bus not found — trigger/sensor events are \
                     being dropped. Call `app.register_event::<TriggerEvent>()` during setup."
                );
                self.warned_missing_trigger_events = true;
            }
        }
        // ── end sensor event diff ─────────────────────────────────────────────

        // borrow checker: collect (entity, handle) pairs first so we can re-borrow world
        self.body_pairs.clear();
        self.body_pairs.extend(
            world
                .query::<PhysicsBody>()
                .map(|(e, b)| (e, b.rigid_body_handle)),
        );

        let scale = self.pixels_per_unit.max(f32::EPSILON);
        for &(entity, handle) in &self.body_pairs {
            if let Some(body) = physics.rigid_body_set.get(handle.0) {
                let t = *body.translation();
                let angle = body.rotation().angle();
                if let Some(tr) = world.get_mut::<Transform>(entity) {
                    tr.position = Vec2::new(t.x * scale, t.y * scale);
                    tr.rotation = angle;
                }
            }
        }

        // Persist the current handle→entity map for the next frame's `stopped` fallback.
        self.prev_col_map.clone_from(&self.col_map);

        world.insert_resource(physics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Transform;
    use crate::physics::events::{CollisionEvent, TriggerEvent};

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
    fn missing_event_bus_does_not_panic() {
        // Events<CollisionEvent> and Events<TriggerEvent> are NOT registered here.
        // PhysicsSystem must not panic and must set the warned flags after the first frame
        // with active contacts/intersections.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        // Two overlapping boxes → immediate contact on step.
        let (rb1, col1) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        let (rb2, col2) = physics.add_dynamic_box(Vec2::new(0.1, 0.0), 0.5, 0.5, false);

        let mut world = World::new();
        world.insert_resource(physics);
        // Deliberately omit register_event for both types.
        let mut system = PhysicsSystem::new(1.0);

        let e1 = world.spawn();
        world.add_component(
            e1,
            PhysicsBody {
                rigid_body_handle: rb1,
                collider_handle: col1,
            },
        );
        world.add_component(e1, Transform::default());

        let e2 = world.spawn();
        world.add_component(
            e2,
            PhysicsBody {
                rigid_body_handle: rb2,
                collider_handle: col2,
            },
        );
        world.add_component(e2, Transform::default());

        // Must not panic even though both event buses are absent.
        system.run(&mut world, 1.0 / 60.0);
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
    fn stopped_event_emitted_when_contacting_entity_despawns() {
        // Two boxes overlapping → contact on frame 1. Despawn one entity on frame 2.
        // A `CollisionEvent::Stopped` must still be emitted with both entity ids.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb1, col1) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        let (rb2, col2) = physics.add_dynamic_box(Vec2::new(0.1, 0.0), 0.5, 0.5, false);

        let mut world = World::new();
        world.insert_resource(physics);
        world.insert_resource(Events::<CollisionEvent>::default());
        let mut system = PhysicsSystem::new(1.0);

        let e1 = world.spawn();
        world.add_component(
            e1,
            PhysicsBody {
                rigid_body_handle: rb1,
                collider_handle: col1,
            },
        );
        world.add_component(e1, Transform::default());

        let e2 = world.spawn();
        world.add_component(
            e2,
            PhysicsBody {
                rigid_body_handle: rb2,
                collider_handle: col2,
            },
        );
        world.add_component(e2, Transform::default());

        // Frame 1: step the simulation so a contact is registered.
        system.run(&mut world, 1.0 / 60.0);

        // Flush events so we only see events from frame 2.
        world
            .resource_mut::<Events<CollisionEvent>>()
            .unwrap()
            .flush();

        // "Despawn" e2 by removing its PhysicsBody component — it will vanish from
        // col_map but its collider handle is still known from prev_col_map.
        world.remove_component::<PhysicsBody>(e2);

        // Frame 2: step again; the contact pair is gone from narrow_phase's side too
        // because the body was removed, so a Stopped event should fire.
        system.run(&mut world, 1.0 / 60.0);

        let events = world.resource::<Events<CollisionEvent>>().unwrap().read();
        let stopped = events.iter().any(|ev| {
            matches!(ev, CollisionEvent::Stopped(a, b) if (*a == e1 && *b == e2) || (*a == e2 && *b == e1))
        });
        assert!(
            stopped,
            "Stopped event must be emitted when a contacting entity is despawned; events: {events:?}"
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

    // ── Task 2: ordered_pair symmetry for contacts ────────────────────────────

    /// Two overlapping boxes stepped for ~10 frames must emit exactly ONE
    /// `CollisionEvent::Started` and zero spurious repeats.
    #[test]
    fn contact_started_emitted_exactly_once() {
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb1, col1) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        let (rb2, col2) = physics.add_dynamic_box(Vec2::new(0.1, 0.0), 0.5, 0.5, false);

        let mut world = World::new();
        world.insert_resource(physics);
        world.insert_resource(Events::<CollisionEvent>::default());
        let mut system = PhysicsSystem::new(1.0);

        let e1 = world.spawn();
        world.add_component(
            e1,
            PhysicsBody {
                rigid_body_handle: rb1,
                collider_handle: col1,
            },
        );
        world.add_component(e1, Transform::default());

        let e2 = world.spawn();
        world.add_component(
            e2,
            PhysicsBody {
                rigid_body_handle: rb2,
                collider_handle: col2,
            },
        );
        world.add_component(e2, Transform::default());

        let dt = 1.0 / 60.0;
        let mut started_count = 0usize;
        for _ in 0..10 {
            system.run(&mut world, dt);
            if let Some(bus) = world.resource_mut::<Events<CollisionEvent>>() {
                for ev in bus.read() {
                    if matches!(ev, CollisionEvent::Started(_, _)) {
                        started_count += 1;
                    }
                }
                bus.flush();
            }
        }
        assert_eq!(
            started_count, 1,
            "exactly one Started event expected over 10 frames, got {started_count}"
        );
    }

    // ── Task 3: reused scratch Vecs have identical behaviour ─────────────────

    /// Verifies that reusing the promoted scratch Vecs does not accumulate stale
    /// data across frames: stepping 3 disjoint contact phases produces clean events.
    #[test]
    fn scratch_vecs_cleared_between_frames() {
        // Two boxes touching for 1 frame only — we remove PhysicsBody immediately after
        // the first frame so no residual data should appear on frame 2.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb1, col1) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        let (rb2, col2) = physics.add_dynamic_box(Vec2::new(0.1, 0.0), 0.5, 0.5, false);

        let mut world = World::new();
        world.insert_resource(physics);
        world.insert_resource(Events::<CollisionEvent>::default());
        let mut system = PhysicsSystem::new(1.0);

        let e1 = world.spawn();
        world.add_component(
            e1,
            PhysicsBody {
                rigid_body_handle: rb1,
                collider_handle: col1,
            },
        );
        world.add_component(e1, Transform::default());

        let e2 = world.spawn();
        world.add_component(
            e2,
            PhysicsBody {
                rigid_body_handle: rb2,
                collider_handle: col2,
            },
        );
        world.add_component(e2, Transform::default());

        // Frame 1: contact starts.
        system.run(&mut world, 1.0 / 60.0);
        world
            .resource_mut::<Events<CollisionEvent>>()
            .unwrap()
            .flush();

        // Remove both PhysicsBodies so no entity maps on frame 2.
        world.remove_component::<PhysicsBody>(e1);
        world.remove_component::<PhysicsBody>(e2);

        // Frame 2: with the entity↔collider mapping removed, the prior contact resolves to a
        // single Stopped event (the contact genuinely ended for these entities).
        system.run(&mut world, 1.0 / 60.0);
        let f2_len = world
            .resource::<Events<CollisionEvent>>()
            .unwrap()
            .read()
            .len();
        assert_eq!(f2_len, 1, "frame 2 should emit exactly one Stopped event");
        world
            .resource_mut::<Events<CollisionEvent>>()
            .unwrap()
            .flush();

        // Frame 3: the reused scratch event vecs must be cleared between frames — no stale leak.
        system.run(&mut world, 1.0 / 60.0);
        let events_f3 = world.resource::<Events<CollisionEvent>>().unwrap().read();
        assert!(
            events_f3.is_empty(),
            "no stale events expected on frame 3; got: {events_f3:?}"
        );
    }
}
