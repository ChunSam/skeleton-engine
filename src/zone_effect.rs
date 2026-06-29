//! Data-driven event→effect bindings — react to [`ZoneEvent`]s with RON-authored effects.
//!
//! The data-driven [`TriggerZone`](crate::TriggerZone)s let a designer author *where* a zone is in
//! a RON file; this module lets them author *what happens* when an entity enters/stays/exits one,
//! also in RON — closing the loop so a level's zones **and** their reactions live in data instead of
//! Rust glue.
//!
//! A [`ZoneEffectBindings`] table maps a zone's [`Tag`] name to a list of [`ZoneEffectRule`]s. Each
//! rule pairs a [`ZonePhase`] (which event fires it) with an [`Effect`] (what to do). The supported
//! effects are the three game-feel reactions the engine can already produce:
//!
//! - [`Effect::SpawnParticles`] — a one-shot particle burst, reusing a named emitter from the
//!   [`ParticleConfigRegistry`] (so the particle visuals are *also* data-driven), anchored at the
//!   entering entity or the zone.
//! - [`Effect::PlayTone`] — a synthesized tone via the cross-platform [`Audio`] facade.
//! - [`Effect::Flash`] — a [`HitFlash`] added to the entity that triggered the event.
//!
//! [`ZoneEffectSystem`] (user-added, like [`TriggerZoneSystem`](crate::TriggerZoneSystem)) reads
//! [`ZoneEvent`]s, resolves each zone to its [`Tag`] name, looks up the matching rules, and applies
//! the effects. **Add it AFTER [`TriggerZoneSystem`](crate::TriggerZoneSystem)** (which sends the
//! events the same frame). Load a binding set with
//! [`App::load_zone_effects`](crate::App::load_zone_effects) (native hot-reload) and name it when
//! constructing the system: `ZoneEffectSystem::new("effects")`.
//!
//! The effect vocabulary is deliberately small and ZoneEvent-focused; an `AnimationEvent`→effect
//! binding is a natural future extension that would reuse the same [`Effect`] enum.

use std::collections::HashMap;

use glam::Vec2;
use serde::Deserialize;

use crate::audio_facade::Audio;
use crate::color::Color;
use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, Events, System, World};
use crate::hit_flash::HitFlash;
use crate::particle::{ParticleBurst, ParticleConfigRegistry, ParticleEmitter};
use crate::prefab::Tag;
use crate::trigger_zone::ZoneEvent;

/// Which entity an [`Effect::SpawnParticles`] burst is anchored at.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectAnchor {
    /// The entity that entered/stayed/exited the zone (default).
    #[default]
    Other,
    /// The zone entity itself.
    Zone,
}

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

/// An effect a [`ZoneEffectRule`] runs when its [`ZonePhase`] matches.
///
/// All variants deserialize from RON (the `color` of [`Flash`](Effect::Flash) is an `(r, g, b, a)`
/// tuple of `0.0..=1.0` floats, matching the particle-config RON convention).
#[derive(Deserialize, Clone, Debug)]
pub enum Effect {
    /// Spawn a one-shot particle burst using a named emitter looked up across the loaded
    /// [`ParticleConfigRegistry`] sets (load configs with
    /// [`App::load_particle_configs`](crate::App::load_particle_configs)). The emitter's continuous
    /// emission is forced off — only the `count`-particle burst fires.
    SpawnParticles {
        /// Emitter name to look up across loaded particle-config sets.
        particles: String,
        /// Number of particles in the burst (default 16).
        #[serde(default = "default_count")]
        count: u32,
        /// Where the burst is anchored (default [`EffectAnchor::Other`]).
        #[serde(default)]
        at: EffectAnchor,
    },
    /// Play a synthesized tone via the cross-platform [`Audio`] facade.
    PlayTone {
        /// Frequency in Hz.
        freq: f32,
        /// Duration in seconds.
        dur: f32,
        /// Volume, `0.0..=1.0`.
        vol: f32,
        /// Optional mixer-bus name; routes through [`Audio::play_tone_on_bus`] when set.
        #[serde(default)]
        bus: Option<String>,
    },
    /// Add a [`HitFlash`] to the entity that triggered the event (it needs a [`Sprite`] for the
    /// flash to show; the effect is skipped if it has none).
    Flash {
        /// Flash color as `(r, g, b, a)`, each `0.0..=1.0`.
        color: (f32, f32, f32, f32),
        /// Flash duration in seconds.
        secs: f32,
    },
}

fn default_count() -> u32 {
    16
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
        let text = std::fs::read_to_string(path)?;
        Self::from_ron_str(&text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for ZoneEffectRegistry {
    fn reload_path(&mut self, path: &str) {
        ZoneEffectRegistry::reload_path(self, path);
    }
}

/// A resolved effect to apply this frame (built read-only, applied with `&mut World`).
enum PendingAction {
    Burst {
        pos: Vec2,
        z: f32,
        emitter: ParticleEmitter,
        count: u32,
    },
    Tone {
        freq: f32,
        dur: f32,
        vol: f32,
        bus: Option<String>,
    },
    Flash {
        target: Entity,
        color: Color,
        secs: f32,
    },
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

/// Find an emitter named `name` across every loaded [`ParticleConfigRegistry`] set (first match).
fn lookup_emitter(world: &World, name: &str) -> Option<ParticleEmitter> {
    let reg = world.resource::<ParticleConfigRegistry>()?;
    reg.names()
        .iter()
        .find_map(|set| reg.get(set).and_then(|s| s.emitter(name)))
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

        // 3. Resolve events against the table into a flat action list (read-only world access).
        let mut actions: Vec<PendingAction> = Vec::new();
        let mut missing_particles = false;
        for (phase, zone, other) in events {
            let Some(tag) = world.get::<Tag>(zone).map(|t| t.0.clone()) else {
                continue;
            };
            for rule in bindings.rules_for(&tag) {
                if rule.on != phase {
                    continue;
                }
                match &rule.effect {
                    Effect::Flash { color, secs } => actions.push(PendingAction::Flash {
                        target: other,
                        color: Color::rgba(color.0, color.1, color.2, color.3),
                        secs: *secs,
                    }),
                    Effect::PlayTone {
                        freq,
                        dur,
                        vol,
                        bus,
                    } => actions.push(PendingAction::Tone {
                        freq: *freq,
                        dur: *dur,
                        vol: *vol,
                        bus: bus.clone(),
                    }),
                    Effect::SpawnParticles {
                        particles,
                        count,
                        at,
                    } => {
                        let anchor = match at {
                            EffectAnchor::Other => other,
                            EffectAnchor::Zone => zone,
                        };
                        let Some(t) = world.get::<Transform>(anchor) else {
                            continue;
                        };
                        let (pos, z) = (t.position, t.z);
                        match lookup_emitter(world, particles) {
                            Some(mut emitter) => {
                                // Burst-only: silence the emitter's continuous emission.
                                emitter.emit = false;
                                emitter.spawn_rate = 0.0;
                                actions.push(PendingAction::Burst {
                                    pos,
                                    z,
                                    emitter,
                                    count: *count,
                                });
                            }
                            None => missing_particles = true,
                        }
                    }
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
        for action in actions {
            match action {
                PendingAction::Flash {
                    target,
                    color,
                    secs,
                } => {
                    // HitFlashSystem only animates entities with a Sprite — skip otherwise so a
                    // dangling HitFlash isn't left on a sprite-less entity forever.
                    if world.get::<Sprite>(target).is_some() {
                        world.add_component(target, HitFlash::new(color, secs));
                    }
                }
                PendingAction::Tone {
                    freq,
                    dur,
                    vol,
                    bus,
                } => {
                    if let Some(audio) = world.resource_mut::<Audio>() {
                        match bus {
                            Some(b) => audio.play_tone_on_bus(freq, dur, vol, &b),
                            None => audio.play_tone(freq, dur, vol),
                        }
                    }
                }
                PendingAction::Burst {
                    pos,
                    z,
                    emitter,
                    count,
                } => {
                    let e = world.spawn();
                    world.add_component(
                        e,
                        Transform {
                            position: pos,
                            z,
                            ..Default::default()
                        },
                    );
                    world.add_component(e, emitter);
                    world.add_component(e, ParticleBurst { remaining: count });
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "ZoneEffectSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // `count`/`at` omitted → defaults (16 / Other).
        match &b.rules_for("damage")[1].effect {
            Effect::SpawnParticles {
                particles,
                count,
                at,
            } => {
                assert_eq!(particles, "blood");
                assert_eq!(*count, 16);
                assert_eq!(*at, EffectAnchor::Other);
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
