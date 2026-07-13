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

// ── Data-driven trigger zones ─────────────────────────────────────────────────
//
// Author a level's zones in a RON file and spawn them as entities, the same way
// particle/dialogue/animation configs are data-driven (private serde-mirror types →
// a `RonLoadable` set → a `RonRegistry`-backed registry with native hot-reload).

use serde::Deserialize;

/// Serde-only tuple for [`Vec2`]: `(x, y)`.
#[derive(Deserialize, Clone, Copy, Debug, Default)]
struct Vec2Def(f32, f32);

impl From<Vec2Def> for Vec2 {
    fn from(v: Vec2Def) -> Vec2 {
        Vec2::new(v.0, v.1)
    }
}

/// Serde-only mirror of [`TriggerShape`]. RON: `Circle(radius: 60.0)` /
/// `Rect(half_extents: (48.0, 64.0))`.
#[derive(Deserialize, Clone, Copy, Debug)]
enum TriggerShapeDef {
    Circle { radius: f32 },
    Rect { half_extents: Vec2Def },
}

impl From<TriggerShapeDef> for TriggerShape {
    fn from(s: TriggerShapeDef) -> TriggerShape {
        match s {
            TriggerShapeDef::Circle { radius } => TriggerShape::Circle { radius },
            TriggerShapeDef::Rect { half_extents } => TriggerShape::Rect {
                half_extents: half_extents.into(),
            },
        }
    }
}

/// One zone's definition inside a RON file: a `tag` (optional name, attached as a
/// [`Tag`](crate::Tag) for reacting to [`ZoneEvent`]s), a world `pos`, a `shape`, and a `mask`
/// (raw [`CollisionLayer`] bits; defaults to all layers).
#[derive(Deserialize, Clone, Debug)]
struct TriggerZoneDef {
    #[serde(default)]
    tag: String,
    pos: Vec2Def,
    shape: TriggerShapeDef,
    #[serde(default = "default_mask")]
    mask: u32,
}

fn default_mask() -> u32 {
    CollisionLayer::ALL.0
}

#[derive(Deserialize, Debug)]
struct TriggerZoneDoc {
    zones: Vec<TriggerZoneDef>,
}

/// Error returned by [`TriggerZoneSet::from_ron_str`] / [`TriggerZoneRegistry::load`].
///
/// A type alias for [`crate::asset::AssetLoadError`] (shared with the particle/animation config
/// loaders) — keeps the public name stable without duplicating the error boilerplate.
pub type TriggerZoneSetError = crate::asset::AssetLoadError;

/// A set of [`TriggerZone`]s loaded from a single RON file (see the [module docs](crate::trigger_zone)).
///
/// Spawn them with [`spawn_into`](TriggerZoneSet::spawn_into) (or [`App::spawn_trigger_zones`](crate::App::spawn_trigger_zones)):
/// each def becomes an entity with a [`Transform`], a [`TriggerZone`], and — if the def has a
/// non-empty `tag` — a [`Tag`](crate::Tag) so a [`ZoneEvent`] can be resolved to a named zone. The
/// zone entities carry **no** `Sprite`; a game draws them however it likes (the `data_trigger_zones`
/// example sizes a debug quad from each zone's shape).
///
/// ```
/// # use engine::TriggerZoneSet;
/// let ron = r#"(zones: [
///     (tag: "heal",   pos: (190.0, 250.0), shape: Rect(half_extents: (48.0, 64.0)), mask: 1),
///     (tag: "damage", pos: (420.0, 250.0), shape: Circle(radius: 60.0)),
/// ])"#;
/// let set = TriggerZoneSet::from_ron_str(ron).unwrap();
/// assert_eq!(set.len(), 2);
/// assert_eq!(set.tags(), vec!["heal".to_string(), "damage".to_string()]);
/// ```
#[derive(Clone, Debug)]
pub struct TriggerZoneSet {
    defs: Vec<TriggerZoneDef>,
}

impl TriggerZoneSet {
    /// Parse a [`TriggerZoneSet`] from a RON string (cross-platform).
    ///
    /// Returns `Err` on malformed input rather than panicking.
    pub fn from_ron_str(s: &str) -> Result<Self, TriggerZoneSetError> {
        let doc: TriggerZoneDoc =
            ron::from_str(s).map_err(|e| TriggerZoneSetError::Ron(e.to_string()))?;
        Ok(TriggerZoneSet { defs: doc.zones })
    }

    /// Number of zones in the set.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Whether the set has no zones.
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    /// The zones' tags, in file order (empty strings for untagged zones).
    pub fn tags(&self) -> Vec<String> {
        self.defs.iter().map(|d| d.tag.clone()).collect()
    }

    /// Spawn one entity per zone into `world` and return the spawned entities (in file order).
    ///
    /// Each entity gets a [`Transform`] at the def's position, a [`TriggerZone`] with the def's
    /// shape + mask, and — when the def has a non-empty `tag` — a [`Tag`](crate::Tag).
    pub fn spawn_into(&self, world: &mut World) -> Vec<Entity> {
        self.defs
            .iter()
            .map(|d| {
                let e = world.spawn();
                world.add_component(
                    e,
                    Transform {
                        position: d.pos.into(),
                        ..Default::default()
                    },
                );
                world.add_component(
                    e,
                    TriggerZone {
                        shape: d.shape.into(),
                        mask: CollisionLayer(d.mask),
                        occupants: Vec::new(),
                    },
                );
                if !d.tag.is_empty() {
                    world.add_component(e, crate::prefab::Tag(d.tag.clone()));
                }
                e
            })
            .collect()
    }
}

/// Registry of named [`TriggerZoneSet`]s, stored as a World resource.
///
/// On native builds, load sets via [`App::load_trigger_zones`](crate::App::load_trigger_zones)
/// (which wires up the file watcher for hot-reload) and spawn them with
/// [`App::spawn_trigger_zones`](crate::App::spawn_trigger_zones). On wasm, parse with
/// [`TriggerZoneSet::from_ron_str`] and [`insert`](TriggerZoneRegistry::insert).
#[derive(Default)]
pub struct TriggerZoneRegistry {
    inner: crate::ron_registry::RonRegistry<TriggerZoneSet>,
}

impl TriggerZoneRegistry {
    /// Load a [`TriggerZoneSet`] from `path` and register it under `name` (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), TriggerZoneSetError> {
        self.inner.load(name, path)
    }

    /// Insert a set directly (in-memory; useful for tests or wasm).
    pub fn insert(&mut self, name: impl Into<String>, set: TriggerZoneSet) {
        self.inner.insert(name, set);
    }

    /// Look up a [`TriggerZoneSet`] by name.
    pub fn get(&self, name: &str) -> Option<&TriggerZoneSet> {
        self.inner.get(name)
    }

    /// Sorted list of registered set names.
    pub fn names(&self) -> Vec<String> {
        self.inner.names()
    }

    /// Re-read the file whose registered path matches `path` (native only). Called automatically
    /// by [`App`](crate::App) on every hot-reload tick.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) {
        self.inner.reload_path(path, "trigger_zone");
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::ron_registry::RonLoadable for TriggerZoneSet {
    type Err = TriggerZoneSetError;
    fn load_ron(path: &str) -> Result<Self, Self::Err> {
        let text = std::fs::read_to_string(crate::asset_path::resolve(path))?;
        Self::from_ron_str(&text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for TriggerZoneRegistry {
    fn reload_path(&mut self, path: &str) {
        TriggerZoneRegistry::reload_path(self, path);
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

    // ── Data-driven set tests ──────────────────────────────────────────────────

    const ZONES_RON: &str = r#"(zones: [
        (tag: "heal",   pos: (190.0, 250.0), shape: Rect(half_extents: (48.0, 64.0)), mask: 1),
        (tag: "damage", pos: (420.0, 250.0), shape: Circle(radius: 60.0), mask: 1),
        (pos: (640.0, 250.0), shape: Rect(half_extents: (48.0, 64.0))),
    ])"#;

    #[test]
    fn set_parses_and_spawns_entities_with_zones_and_tags() {
        let set = TriggerZoneSet::from_ron_str(ZONES_RON).expect("parse");
        assert_eq!(set.len(), 3);
        assert_eq!(
            set.tags(),
            vec!["heal".to_string(), "damage".to_string(), String::new()]
        );

        let mut world = World::new();
        let spawned = set.spawn_into(&mut world);
        assert_eq!(spawned.len(), 3);

        // First zone: rect, mask 1, tagged "heal".
        let z0 = world.get::<TriggerZone>(spawned[0]).unwrap();
        assert_eq!(
            z0.shape,
            TriggerShape::Rect {
                half_extents: Vec2::new(48.0, 64.0)
            }
        );
        assert_eq!(z0.mask, CollisionLayer(1));
        assert_eq!(
            world.get::<Transform>(spawned[0]).unwrap().position,
            Vec2::new(190.0, 250.0)
        );
        assert_eq!(
            world.get::<crate::prefab::Tag>(spawned[0]).unwrap().0,
            "heal"
        );

        // Second zone: circle.
        assert_eq!(
            world.get::<TriggerZone>(spawned[1]).unwrap().shape,
            TriggerShape::Circle { radius: 60.0 }
        );

        // Third zone: no tag → no Tag component; mask defaults to ALL.
        assert!(world.get::<crate::prefab::Tag>(spawned[2]).is_none());
        assert_eq!(
            world.get::<TriggerZone>(spawned[2]).unwrap().mask,
            CollisionLayer::ALL
        );
    }

    #[test]
    fn set_malformed_ron_returns_err() {
        assert!(TriggerZoneSet::from_ron_str("not ron at all {{{").is_err());
    }

    #[test]
    fn spawned_zones_detect_overlap_end_to_end() {
        // A data-driven zone behaves exactly like a hand-built one once spawned.
        let set = TriggerZoneSet::from_ron_str(
            r#"(zones: [(tag: "z", pos: (0.0, 0.0), shape: Circle(radius: 50.0))])"#,
        )
        .expect("parse");
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());
        let zone = set.spawn_into(&mut world)[0];
        let actor = spawn_actor(&mut world, Vec2::new(10.0, 0.0), None);

        CollisionGridSystem::new(128.0).run(&mut world, 0.016);
        TriggerZoneSystem::default().run(&mut world, 0.016);

        assert_eq!(
            events(&world),
            vec![ZoneEvent::Entered { zone, other: actor }]
        );
    }

    #[test]
    fn registry_insert_get_names() {
        let set = TriggerZoneSet::from_ron_str(ZONES_RON).expect("parse");
        let mut reg = TriggerZoneRegistry::default();
        reg.insert("level1", set);
        assert_eq!(reg.get("level1").unwrap().len(), 3);
        assert!(reg.get("missing").is_none());
        assert_eq!(reg.names(), vec!["level1".to_string()]);
    }
}
