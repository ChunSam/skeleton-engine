//! Data-driven Zone→Effect bindings — react to [`ZoneEvent`]s with RON-authored effects.
//!
//! The data-driven [`TriggerZone`](crate::TriggerZone)s let a designer author *where* a zone is in
//! a RON file; this module lets them author *what happens* when an entity enters/stays/exits one,
//! also in RON — closing the loop so a level's zones **and** their reactions live in data instead of
//! Rust glue.
//!
//! A [`ZoneEffectBindings`] table maps a zone's [`Tag`] name to a list of [`ZoneEffectRule`]s. Each
//! rule pairs a [`ZonePhase`] (which event fires it) with an [`Effect`] (what to do). The
//! [`Effect`] vocabulary (`SpawnParticles` / `PlayTone` / `Flash`) and its application live in the
//! shared [`crate::effect`] module; [`anim_effect`](crate::anim_effect) reuses them to react to
//! `AnimationEvent`s instead.
//!
//! [`ZoneEffectSystem`] (user-added, like [`TriggerZoneSystem`](crate::TriggerZoneSystem)) reads
//! [`ZoneEvent`]s, resolves each zone to its [`Tag`] name, looks up the matching rules, and applies
//! the effects. **Add it AFTER [`TriggerZoneSystem`](crate::TriggerZoneSystem)** (which sends the
//! events the same frame). Load a binding set with
//! [`App::load_zone_effects`](crate::App::load_zone_effects) (native hot-reload) and name it when
//! constructing the system: `ZoneEffectSystem::new("effects")`.

use std::collections::HashMap;

use serde::Deserialize;

use crate::ecs::{Entity, Events, System, World};
use crate::effect::{apply_pending, resolve_effect, Effect, EffectAnchor};
use crate::prefab::Tag;
use crate::trigger_zone::ZoneEvent;

/// Which [`ZoneEvent`] phase a [`ZoneEffectRule`] fires on.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZonePhase {
    /// The frame an entity enters the zone (default).
    #[default]
    Entered,
    /// Every frame an entity remains inside.
    Stayed,
    /// The frame an entity leaves.
    Exited,
}

/// One rule in a [`ZoneEffectBindings`] table: an [`Effect`] gated to a [`ZonePhase`].
#[derive(Deserialize, Clone, Debug)]
pub struct ZoneEffectRule {
    /// Which event phase fires this rule (default [`ZonePhase::Entered`]).
    #[serde(default)]
    pub on: ZonePhase,
    /// The effect to run.
    pub effect: Effect,
}

/// Error returned by [`ZoneEffectBindings::from_ron_str`] / [`ZoneEffectRegistry::load`].
///
/// A type alias for [`crate::asset::AssetLoadError`] (shared with the particle/trigger-zone config
/// loaders) — keeps the public name stable without duplicating the error boilerplate.
pub type ZoneEffectError = crate::asset::AssetLoadError;

/// A table mapping a zone's [`Tag`] name to the [`ZoneEffectRule`]s that fire for it.
///
/// Load one from RON with [`App::load_zone_effects`](crate::App::load_zone_effects) (native
/// hot-reload), or parse directly with [`from_ron_str`](ZoneEffectBindings::from_ron_str).
///
/// ```
/// # use engine::ZoneEffectBindings;
/// let ron = r#"(bindings: {
///     "heal":   [ (on: Entered, effect: PlayTone(freq: 660.0, dur: 0.2, vol: 0.3)) ],
///     "damage": [ (effect: Flash(color: (1.0, 0.3, 0.3, 1.0), secs: 0.25)) ],
/// })"#;
/// let b = ZoneEffectBindings::from_ron_str(ron).unwrap();
/// assert_eq!(b.len(), 2);
/// assert_eq!(b.rules_for("damage").len(), 1);
/// assert!(b.rules_for("unknown").is_empty());
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct ZoneEffectBindings {
    /// Tag-keyed rules: a zone with `Tag("heal")` runs `bindings["heal"]`.
    pub bindings: HashMap<String, Vec<ZoneEffectRule>>,
}

impl ZoneEffectBindings {
    /// Parse a [`ZoneEffectBindings`] table from a RON string (cross-platform).
    ///
    /// Returns `Err` on malformed input rather than panicking.
    pub fn from_ron_str(s: &str) -> Result<Self, ZoneEffectError> {
        ron::from_str(s).map_err(|e| ZoneEffectError::Ron(e.to_string()))
    }

    /// The rules bound to `tag` (empty slice if none).
    pub fn rules_for(&self, tag: &str) -> &[ZoneEffectRule] {
        self.bindings.get(tag).map_or(&[], Vec::as_slice)
    }

    /// Number of distinct tags with bindings.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// Whether the table has no bindings.
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

/// Registry of named [`ZoneEffectBindings`] tables, stored as a World resource.
///
/// On native builds, load tables via [`App::load_zone_effects`](crate::App::load_zone_effects)
/// (which wires up the file watcher for hot-reload). On wasm, parse with
/// [`ZoneEffectBindings::from_ron_str`] and [`insert`](ZoneEffectRegistry::insert).
#[derive(Default)]
pub struct ZoneEffectRegistry {
    inner: crate::ron_registry::RonRegistry<ZoneEffectBindings>,
}

impl ZoneEffectRegistry {
    /// Load a [`ZoneEffectBindings`] table from `path` and register it under `name` (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), ZoneEffectError> {
        self.inner.load(name, path)
    }

    /// Insert a table directly (in-memory; useful for tests or wasm).
    pub fn insert(&mut self, name: impl Into<String>, bindings: ZoneEffectBindings) {
        self.inner.insert(name, bindings);
    }

    /// Look up a [`ZoneEffectBindings`] table by name.
    pub fn get(&self, name: &str) -> Option<&ZoneEffectBindings> {
        self.inner.get(name)
    }

    /// Sorted list of registered table names.
    pub fn names(&self) -> Vec<String> {
        self.inner.names()
    }

    /// Re-read the file whose registered path matches `path` (native only). Called automatically
    /// by [`App`](crate::App) on every hot-reload tick.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_path(&mut self, path: &str) {
        self.inner.reload_path(path, "zone_effect");
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::ron_registry::RonLoadable for ZoneEffectBindings {
    type Err = ZoneEffectError;
    fn load_ron(path: &str) -> Result<Self, Self::Err> {
        let text = std::fs::read_to_string(crate::asset_path::resolve(path))?;
        Self::from_ron_str(&text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for ZoneEffectRegistry {
    fn reload_path(&mut self, path: &str) {
        ZoneEffectRegistry::reload_path(self, path);
    }
}

/// Applies a named [`ZoneEffectBindings`] table to incoming [`ZoneEvent`]s.
///
/// Add it to the schedule **after** [`TriggerZoneSystem`](crate::TriggerZoneSystem) (and the
/// particle/audio systems so their effects render the same frame). Construct it with the name the
/// table was loaded under: `ZoneEffectSystem::new("effects")`.
pub struct ZoneEffectSystem {
    name: String,
    warned_no_table: bool,
    warned_missing_particles: bool,
}

impl ZoneEffectSystem {
    /// Create a system that applies the binding table registered under `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            warned_no_table: false,
            warned_missing_particles: false,
        }
    }
}

impl System for ZoneEffectSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // 1. Clone the active binding table out so we can take `&mut World` later.
        let Some(bindings) = world
            .resource::<ZoneEffectRegistry>()
            .and_then(|r| r.get(&self.name).cloned())
        else {
            if !self.warned_no_table {
                log::warn!(
                    "ZoneEffectSystem: no zone-effect table registered under '{}'",
                    self.name
                );
                self.warned_no_table = true;
            }
            return;
        };

        // 2. Snapshot this frame's zone events as (phase, zone, other) triples.
        let Some(bus) = world.resource::<Events<ZoneEvent>>() else {
            return;
        };
        let events: Vec<(ZonePhase, Entity, Entity)> = bus
            .read()
            .iter()
            .map(|e| match *e {
                ZoneEvent::Entered { zone, other } => (ZonePhase::Entered, zone, other),
                ZoneEvent::Stayed { zone, other } => (ZonePhase::Stayed, zone, other),
                ZoneEvent::Exited { zone, other } => (ZonePhase::Exited, zone, other),
            })
            .collect();
        if events.is_empty() {
            return;
        }

        // 3. Resolve matching rules into a flat action list (read-only world access). A zone's Tag
        //    names the rule key; `at` picks a SpawnParticles anchor (entrant vs zone), while a Flash
        //    always targets the entrant.
        let mut actions = Vec::new();
        let mut missing_particles = false;
        for (phase, zone, other) in events {
            let Some(tag) = world.get::<Tag>(zone).map(|t| t.0.clone()) else {
                continue;
            };
            for rule in bindings.rules_for(&tag) {
                if rule.on != phase {
                    continue;
                }
                let particle_anchor = match &rule.effect {
                    Effect::SpawnParticles {
                        at: EffectAnchor::Zone,
                        ..
                    } => zone,
                    _ => other,
                };
                if let Some(pe) = resolve_effect(
                    world,
                    &rule.effect,
                    particle_anchor,
                    other,
                    &mut missing_particles,
                ) {
                    actions.push(pe);
                }
            }
        }

        if missing_particles && !self.warned_missing_particles {
            log::warn!(
                "ZoneEffectSystem: a SpawnParticles effect named an emitter not found in any loaded \
                 ParticleConfigRegistry set (did you call App::load_particle_configs?)"
            );
            self.warned_missing_particles = true;
        }

        // 4. Apply the actions (mutating world access).
        apply_pending(world, actions);
    }

    fn name(&self) -> &'static str {
        "ZoneEffectSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::components::{Sprite, Transform};
    use crate::hit_flash::HitFlash;
    use crate::particle::{ParticleBurst, ParticleConfigRegistry};
    use glam::Vec2;

    const BINDINGS_RON: &str = r#"(bindings: {
        "heal":   [ (on: Entered, effect: SpawnParticles(particles: "sparkle", count: 12, at: Zone)) ],
        "damage": [
            (effect: Flash(color: (1.0, 0.3, 0.3, 1.0), secs: 0.25)),
            (on: Entered, effect: SpawnParticles(particles: "blood")),
            (on: Exited,  effect: PlayTone(freq: 110.0, dur: 0.1, vol: 0.4)),
        ],
    })"#;

    #[test]
    fn bindings_parse_lookup_and_defaults() {
        let b = ZoneEffectBindings::from_ron_str(BINDINGS_RON).expect("parse");
        assert_eq!(b.len(), 2);
        assert!(b.rules_for("unknown").is_empty());
        assert_eq!(b.rules_for("damage").len(), 3);

        // `on` omitted → defaults to Entered.
        let flash = &b.rules_for("damage")[0];
        assert_eq!(flash.on, ZonePhase::Entered);
        match &flash.effect {
            Effect::Flash { color, secs } => {
                assert_eq!(*color, (1.0, 0.3, 0.3, 1.0));
                assert_eq!(*secs, 0.25);
            }
            other => panic!("expected Flash, got {other:?}"),
        }

        // `count`/`at`/`offset` omitted → defaults (16 / Other / (0,0)).
        match &b.rules_for("damage")[1].effect {
            Effect::SpawnParticles {
                particles,
                count,
                at,
                offset,
            } => {
                assert_eq!(particles, "blood");
                assert_eq!(*count, 16);
                assert_eq!(*at, EffectAnchor::Other);
                assert_eq!(*offset, (0.0, 0.0));
            }
            other => panic!("expected SpawnParticles, got {other:?}"),
        }

        // `at: Zone` parses.
        match &b.rules_for("heal")[0].effect {
            Effect::SpawnParticles { at, count, .. } => {
                assert_eq!(*at, EffectAnchor::Zone);
                assert_eq!(*count, 12);
            }
            other => panic!("expected SpawnParticles, got {other:?}"),
        }
    }

    #[test]
    fn malformed_ron_returns_err() {
        assert!(ZoneEffectBindings::from_ron_str("not ron at all {{{").is_err());
    }

    #[test]
    fn registry_insert_get_names() {
        let mut reg = ZoneEffectRegistry::default();
        assert!(reg.get("fx").is_none());
        reg.insert(
            "fx",
            ZoneEffectBindings::from_ron_str(BINDINGS_RON).unwrap(),
        );
        assert_eq!(reg.get("fx").unwrap().len(), 2);
        assert_eq!(reg.names(), vec!["fx".to_string()]);
    }

    #[test]
    fn system_flashes_entering_entity_end_to_end() {
        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());
        let mut reg = ZoneEffectRegistry::default();
        reg.insert(
            "fx",
            ZoneEffectBindings::from_ron_str(BINDINGS_RON).unwrap(),
        );
        world.insert_resource(reg);

        // A "damage" zone and a sprite-bearing actor.
        let zone = world.spawn();
        world.add_component(zone, Tag("damage".to_string()));
        let actor = world.spawn();
        world.add_component(actor, Transform::default());
        world.add_component(actor, Sprite::default());

        // Send an Entered event and run the system.
        world
            .resource_mut::<Events<ZoneEvent>>()
            .unwrap()
            .send(ZoneEvent::Entered { zone, other: actor });
        ZoneEffectSystem::new("fx").run(&mut world, 0.016);

        // The damage binding's Entered Flash was applied to the actor.
        let flash = world.get::<HitFlash>(actor).expect("HitFlash added");
        assert_eq!(flash.color, Color::rgba(1.0, 0.3, 0.3, 1.0));
        assert_eq!(flash.secs, 0.25);
    }

    #[test]
    fn system_spawns_burst_from_named_emitter() {
        use crate::particle::ParticleConfigSet;

        let mut world = World::new();
        world.insert_resource(Events::<ZoneEvent>::default());

        let mut reg = ZoneEffectRegistry::default();
        reg.insert(
            "fx",
            ZoneEffectBindings::from_ron_str(BINDINGS_RON).unwrap(),
        );
        world.insert_resource(reg);

        // A particle set whose emitter is named "blood" (the damage Entered effect).
        let mut preg = ParticleConfigRegistry::default();
        preg.insert(
            "set",
            ParticleConfigSet::from_ron_str(r#"(emitters: { "blood": (lifetime: 0.5) })"#).unwrap(),
        );
        world.insert_resource(preg);

        let zone = world.spawn();
        world.add_component(zone, Tag("damage".to_string()));
        let actor = world.spawn();
        world.add_component(
            actor,
            Transform {
                position: Vec2::new(40.0, 60.0),
                ..Default::default()
            },
        );

        world
            .resource_mut::<Events<ZoneEvent>>()
            .unwrap()
            .send(ZoneEvent::Entered { zone, other: actor });
        ZoneEffectSystem::new("fx").run(&mut world, 0.016);

        // Exactly one burst emitter was spawned, with the default count (16).
        let bursts: Vec<(Entity, u32)> = world
            .query::<ParticleBurst>()
            .map(|(e, b)| (e, b.remaining))
            .collect();
        assert_eq!(bursts.len(), 1);
        assert_eq!(bursts[0].1, 16);
        // Anchored at the actor (EffectAnchor::Other) — Transform carried the actor's position.
        let pos = world.get::<Transform>(bursts[0].0).unwrap().position;
        assert_eq!(pos, Vec2::new(40.0, 60.0));
    }
}
