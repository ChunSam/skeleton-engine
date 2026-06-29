//! Trigger zones — fire enter/stay/exit events when entities overlap a region.
//!
//! Attach a [`TriggerZone`] to an entity (with a [`Transform`]) to make it watch for other entities
//! entering its area — the idiomatic 2D "Area2D" pattern for pickups, damage fields, door/room
//! triggers, aggro ranges, and checkpoints, without hand-rolling per-frame overlap polling.
//!
//! [`TriggerZoneSystem`] (user-added, like [`YSortSystem`](crate::YSortSystem)) reads the
//! [`SpatialGrid`] mirrored by
//! [`CollisionGridSystem`](crate::CollisionGridSystem), computes which entities overlap each zone
//! this frame, diffs that against last frame, and sends a [`ZoneEvent`] for every entity that
//! **entered**, **stayed**, or **exited**. Register the bus once with
//! `App::register_event::<ZoneEvent>()` and read it in a later system the same frame. The current
//! occupant set also lives on the component ([`TriggerZone::occupants`]) for direct polling.
//!
//! Watched entities are the ones already in the spatial grid: each needs a [`Transform`] **and** a
//! [`Collider`](crate::Collider) (and optionally a [`CollisionLayer`]; a zone
//! only reports entities whose layer matches its [`mask`](TriggerZone::mask)). The zone entity
//! itself does not need a `Collider` — its shape comes from [`TriggerZone::shape`] — and a zone
//! never reports itself.
//!
//! The event type is **[`ZoneEvent`]**, deliberately distinct from the physics
//! [`TriggerEvent`](crate::TriggerEvent) (rapier sensor overlaps): this is a lightweight,
//! grid-based, physics-free zone.

use glam::Vec2;

use crate::collision::{CollisionLayer, SpatialGrid};
use crate::components::Transform;
use crate::ecs::{Entity, Events, System, World};

/// The shape of a [`TriggerZone`], centered on its entity's [`Transform::position`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TriggerShape {
    /// A circle of `radius` (matched via [`SpatialGrid::query_radius`]).
    Circle {
        /// Radius in world units.
        radius: f32,
    },
    /// An axis-aligned rectangle of the given half-extents (matched via [`SpatialGrid::query_aabb`]).
    Rect {
        /// Half-width / half-height in world units.
        half_extents: Vec2,
    },
}

impl Default for TriggerShape {
    fn default() -> Self {
        Self::Circle { radius: 32.0 }
    }
}

/// A region that emits [`ZoneEvent`]s when entities overlap it (see the [module docs](crate::trigger_zone)).
///
/// Build one with [`TriggerZone::circle`] / [`TriggerZone::rect`] and add it next to a [`Transform`].
/// Use [`with_mask`](TriggerZone::with_mask) to watch only certain
/// [`CollisionLayer`]s.
///
/// ```
/// # use engine::{TriggerZone, TriggerShape, CollisionLayer};
/// let zone = TriggerZone::circle(64.0).with_mask(CollisionLayer(1 << 2));
/// assert!(matches!(zone.shape, TriggerShape::Circle { radius } if radius == 64.0));
/// assert_eq!(zone.mask, CollisionLayer(1 << 2));
/// assert!(zone.occupants.is_empty());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct TriggerZone {
    /// The zone's shape, centered on its `Transform.position`.
    pub shape: TriggerShape,
    /// Only entities whose [`CollisionLayer`] matches this mask are reported.
    /// Defaults to [`CollisionLayer::ALL`](crate::CollisionLayer::ALL).
    pub mask: CollisionLayer,
    /// Entities currently inside the zone, refreshed every frame by [`TriggerZoneSystem`].
    ///
    /// Treat this as **read-only** from game code (poll it to see who is inside, e.g. via
    /// [`contains`](TriggerZone::contains)); the system overwrites it each frame and uses the
    /// previous value to derive the enter/exit transitions.
    pub occupants: Vec<Entity>,
}

impl Default for TriggerZone {
    fn default() -> Self {
        Self {
            shape: TriggerShape::default(),
            mask: CollisionLayer::ALL,
            occupants: Vec::new(),
        }
    }
}

impl TriggerZone {
    /// A circular zone of `radius` that reports all layers.
    pub fn circle(radius: f32) -> Self {
        Self {
            shape: TriggerShape::Circle { radius },
            ..Self::default()
        }
    }

    /// A rectangular zone of the given half-extents that reports all layers.
    pub fn rect(half_extents: Vec2) -> Self {
        Self {
            shape: TriggerShape::Rect { half_extents },
            ..Self::default()
        }
    }

    /// Builder: only report entities whose [`CollisionLayer`] matches `mask`.
    pub fn with_mask(mut self, mask: CollisionLayer) -> Self {
        self.mask = mask;
        self
    }

    /// Whether `entity` was inside the zone as of the last [`TriggerZoneSystem`] run.
    pub fn contains(&self, entity: Entity) -> bool {
        self.occupants.contains(&entity)
    }
}

/// Emitted by [`TriggerZoneSystem`] when an entity's overlap state with a [`TriggerZone`] changes
/// (or, for [`Stayed`](ZoneEvent::Stayed), persists). `zone` is the zone entity; `other` is the
/// entity overlapping it.
///
/// Register the bus with `App::register_event::<ZoneEvent>()` and read it the same frame via
/// `world.resource::<Events<ZoneEvent>>()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneEvent {
    /// `other` was not inside `zone` last frame and is now.
    Entered {
        /// The zone entity.
        zone: Entity,
        /// The entity that entered.
        other: Entity,
    },
    /// `other` was inside `zone` last frame and still is — fires every frame it remains.
    Stayed {
        /// The zone entity.
        zone: Entity,
        /// The entity still inside.
        other: Entity,
    },
    /// `other` was inside `zone` last frame and is no longer.
    Exited {
        /// The zone entity.
        zone: Entity,
        /// The entity that left.
        other: Entity,
    },
}

/// Detects entity-vs-zone overlaps each frame and emits [`ZoneEvent`]s (see the
/// [module docs](crate::trigger_zone)).
///
/// User-added (like [`YSortSystem`](crate::YSortSystem)). It reads the [`SpatialGrid`] resource, so
/// add a [`CollisionGridSystem`](crate::CollisionGridSystem) **before** it. Register the event bus
/// with `App::register_event::<ZoneEvent>()` to receive events — otherwise they are dropped with a
/// one-time warning, but [`TriggerZone::occupants`] is still updated for polling.
#[derive(Default)]
pub struct TriggerZoneSystem {
    warned_no_grid: bool,
    warned_no_bus: bool,
}

impl System for TriggerZoneSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(grid) = world.resource::<SpatialGrid>() else {
            if !self.warned_no_grid {
                log::warn!(
                    "TriggerZoneSystem: no SpatialGrid resource — add a CollisionGridSystem before \
                     TriggerZoneSystem so zones can detect overlaps. Zones are inert this run."
                );
                self.warned_no_grid = true;
            }
            return;
        };

        // Phase 1 (needs &World only): compute each zone's current occupants + the transitions.
        // `grid` and the query iterator are both shared borrows of `world`, so they coexist.
        let mut updates: Vec<(Entity, Vec<Entity>)> = Vec::new();
        let mut pending: Vec<ZoneEvent> = Vec::new();
        for (zone_e, transform, zone) in world.query2::<Transform, TriggerZone>() {
            let center = transform.position;
            let mut current = match zone.shape {
                TriggerShape::Circle { radius } => grid.query_radius(center, radius, zone.mask),
                TriggerShape::Rect { half_extents } => {
                    grid.query_aabb(center - half_extents, center + half_extents, zone.mask)
                }
            };
            current.retain(|&e| e != zone_e); // a zone never reports itself

            for &e in &current {
                if zone.occupants.contains(&e) {
                    pending.push(ZoneEvent::Stayed {
                        zone: zone_e,
                        other: e,
                    });
                } else {
                    pending.push(ZoneEvent::Entered {
                        zone: zone_e,
                        other: e,
                    });
                }
            }
            for &e in &zone.occupants {
                if !current.contains(&e) {
                    pending.push(ZoneEvent::Exited {
                        zone: zone_e,
                        other: e,
                    });
                }
            }
            updates.push((zone_e, current));
        }

        // Phase 2 (&mut World): store this frame's occupants for next frame's diff. (The `grid`
        // borrow above has ended — its last use was inside the phase-1 loop.)
        for (zone_e, current) in updates {
            if let Some(zone) = world.get_mut::<TriggerZone>(zone_e) {
                zone.occupants = current;
            }
        }

        // Phase 3: flush events, or warn once if the bus is unregistered.
        if pending.is_empty() {
            return;
        }
        if let Some(bus) = world.resource_mut::<Events<ZoneEvent>>() {
            for ev in pending {
                bus.send(ev);
            }
        } else if !self.warned_no_bus {
            log::warn!(
                "TriggerZoneSystem: Events<ZoneEvent> is not registered, so zone events are being \
                 dropped. Call `app.register_event::<ZoneEvent>()` during setup. \
                 (TriggerZone::occupants is still updated for polling.)"
            );
            self.warned_no_bus = true;
        }
    }

    fn name(&self) -> &'static str {
        "TriggerZoneSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::{Collider, CollisionGridSystem};

    const PLAYER: CollisionLayer = CollisionLayer(1 << 0);
    const ENEMY: CollisionLayer = CollisionLayer(1 << 1);

    fn spawn_actor(world: &mut World, pos: Vec2, layer: Option<CollisionLayer>) -> Entity {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: pos,
                ..Default::default()
            },
        );
        world.add_component(e, Collider::Circle { radius: 5.0 });
        if let Some(l) = layer {
            world.add_component(e, l);
        }
        e
    }

    fn events(world: &World) -> Vec<ZoneEvent> {
        world
            .resource::<Events<ZoneEvent>>()
            .unwrap()
            .read()
            .to_vec()
    }

    fn flush(world: &mut World) {
        world.resource_mut::<Events<ZoneEvent>>().unwrap().flush();
    }

    #[test]
    fn enter_stay_exit_lifecycle() {
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());

        let zone = world.spawn();
        world.add_component(zone, Transform::default()); // origin
        world.add_component(zone, TriggerZone::circle(50.0));

        let actor = spawn_actor(&mut world, Vec2::new(200.0, 0.0), None);

        let mut grid_sys = CollisionGridSystem::new(128.0);
        let mut zone_sys = TriggerZoneSystem::default();

        // Frame 1: actor far away → no events, no occupants.
        grid_sys.run(&mut world, 0.016);
        zone_sys.run(&mut world, 0.016);
        assert!(events(&world).is_empty());
        assert!(world.get::<TriggerZone>(zone).unwrap().occupants.is_empty());
        flush(&mut world);

        // Frame 2: move inside → Entered.
        world.get_mut::<Transform>(actor).unwrap().position = Vec2::new(10.0, 0.0);
        grid_sys.run(&mut world, 0.016);
        zone_sys.run(&mut world, 0.016);
        assert_eq!(
            events(&world),
            vec![ZoneEvent::Entered { zone, other: actor }]
        );
        assert!(world.get::<TriggerZone>(zone).unwrap().contains(actor));
        flush(&mut world);

        // Frame 3: still inside → Stayed.
        grid_sys.run(&mut world, 0.016);
        zone_sys.run(&mut world, 0.016);
        assert_eq!(
            events(&world),
            vec![ZoneEvent::Stayed { zone, other: actor }]
        );
        flush(&mut world);

        // Frame 4: move out → Exited, occupants cleared.
        world.get_mut::<Transform>(actor).unwrap().position = Vec2::new(300.0, 0.0);
        grid_sys.run(&mut world, 0.016);
        zone_sys.run(&mut world, 0.016);
        assert_eq!(
            events(&world),
            vec![ZoneEvent::Exited { zone, other: actor }]
        );
        assert!(world.get::<TriggerZone>(zone).unwrap().occupants.is_empty());
    }

    #[test]
    fn mask_filters_by_layer() {
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());

        let zone = world.spawn();
        world.add_component(zone, Transform::default());
        world.add_component(zone, TriggerZone::circle(50.0).with_mask(PLAYER));

        // An ENEMY-layer actor inside the zone must NOT be reported.
        let _enemy = spawn_actor(&mut world, Vec2::new(10.0, 0.0), Some(ENEMY));
        // A PLAYER-layer actor inside the zone IS reported.
        let player = spawn_actor(&mut world, Vec2::new(-10.0, 0.0), Some(PLAYER));

        CollisionGridSystem::new(128.0).run(&mut world, 0.016);
        TriggerZoneSystem::default().run(&mut world, 0.016);

        assert_eq!(
            events(&world),
            vec![ZoneEvent::Entered {
                zone,
                other: player
            }]
        );
    }

    #[test]
    fn rect_zone_detects_overlap() {
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());

        let zone = world.spawn();
        world.add_component(zone, Transform::default());
        world.add_component(zone, TriggerZone::rect(Vec2::new(40.0, 20.0)));

        // Inside the 80×40 box.
        let inside = spawn_actor(&mut world, Vec2::new(30.0, 0.0), None);
        // Outside it on X.
        let _outside = spawn_actor(&mut world, Vec2::new(80.0, 0.0), None);

        CollisionGridSystem::new(128.0).run(&mut world, 0.016);
        TriggerZoneSystem::default().run(&mut world, 0.016);

        assert_eq!(
            events(&world),
            vec![ZoneEvent::Entered {
                zone,
                other: inside
            }]
        );
    }

    #[test]
    fn zone_does_not_report_itself() {
        // A zone that also carries a Collider (so it lands in the grid) must not report itself.
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());

        let zone = world.spawn();
        world.add_component(zone, Transform::default());
        world.add_component(zone, TriggerZone::circle(50.0));
        world.add_component(zone, Collider::Circle { radius: 10.0 });

        CollisionGridSystem::new(128.0).run(&mut world, 0.016);
        TriggerZoneSystem::default().run(&mut world, 0.016);

        assert!(events(&world).is_empty());
        assert!(world.get::<TriggerZone>(zone).unwrap().occupants.is_empty());
    }

    #[test]
    fn unregistered_bus_drops_events_but_updates_occupants_and_warns_once() {
        let mut world = World::new(); // NOTE: no Events<ZoneEvent> registered.

        let zone = world.spawn();
        world.add_component(zone, Transform::default());
        world.add_component(zone, TriggerZone::circle(50.0));
        let actor = spawn_actor(&mut world, Vec2::new(10.0, 0.0), None);

        let mut zone_sys = TriggerZoneSystem::default();

        CollisionGridSystem::new(128.0).run(&mut world, 0.016);
        zone_sys.run(&mut world, 0.016); // must not panic
        assert!(
            world.get::<TriggerZone>(zone).unwrap().contains(actor),
            "occupants still updated without a bus"
        );
        assert!(zone_sys.warned_no_bus);

        // Second run: still no panic, the warn-once flag stays set.
        CollisionGridSystem::new(128.0).run(&mut world, 0.016);
        zone_sys.run(&mut world, 0.016);
        assert!(zone_sys.warned_no_bus);
    }

    #[test]
    fn missing_grid_warns_once_and_is_inert() {
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());

        let zone = world.spawn();
        world.add_component(zone, Transform::default());
        world.add_component(zone, TriggerZone::circle(50.0));
        // An actor exists but there is no SpatialGrid resource (no CollisionGridSystem run).
        let _actor = spawn_actor(&mut world, Vec2::new(10.0, 0.0), None);

        let mut zone_sys = TriggerZoneSystem::default();
        zone_sys.run(&mut world, 0.016);

        assert!(zone_sys.warned_no_grid);
        assert!(events(&world).is_empty());
        assert!(world.get::<TriggerZone>(zone).unwrap().occupants.is_empty());
    }
}
