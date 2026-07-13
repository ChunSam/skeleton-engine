//! Data-driven Animation→Effect bindings — react to [`AnimationEvent`]s with RON-authored effects.
//!
//! The companion to [`zone_effect`](crate::zone_effect): where that fires effects when an entity
//! overlaps a zone, this fires them when an animation reaches a tagged frame — footstep dust on a
//! contact frame, a swing whoosh, a muzzle flash — authored in RON instead of Rust glue. It reuses
//! the shared [`Effect`] vocabulary + application from [`crate::effect`] directly.
//!
//! An [`AnimEffectBindings`] table maps an [`AnimationEvent::tag`] (the label set on a
//! [`FrameEvent`](crate::animation::events::FrameEvent), e.g. `"footstep"`) to a list of [`Effect`]s.
//! Unlike a zone binding there is **no phase** — an animation event is an instantaneous "this frame
//! was entered" fire — so the value is a bare `Vec<Effect>`, not a phase-gated rule.
//!
//! [`AnimEffectSystem`] (user-added) reads [`AnimationEvent`]s, looks up the effects for each
//! event's `tag`, and applies them anchored at the animating entity. **Add it AFTER
//! [`AnimationSystem`](crate::animation::AnimationSystem)** (which sends the events the same frame).
//! Both a `SpawnParticles` burst and a `Flash` target the animating entity;
//! [`EffectAnchor`](crate::EffectAnchor) is **not** consulted here (there is no second entity to
//! anchor to). Load a table with
//! [`App::load_anim_effects`](crate::App::load_anim_effects) and name it when constructing the
//! system: `AnimEffectSystem::new("effects")`.

use std::collections::HashMap;

use serde::Deserialize;

use crate::animation::events::AnimationEvent;
use crate::ecs::{Events, System, World};
use crate::effect::{apply_pending, resolve_effect, Effect};

/// Error returned by [`AnimEffectBindings::from_ron_str`] / [`AnimEffectRegistry::load`].
///
/// A type alias for [`crate::asset::AssetLoadError`] (shared with the other RON config loaders).
pub type AnimEffectError = crate::asset::AssetLoadError;

/// A table mapping an [`AnimationEvent::tag`] to the [`Effect`]s that fire when that frame is entered.
///
/// Load one from RON with [`App::load_anim_effects`](crate::App::load_anim_effects) (native
/// hot-reload), or parse directly with [`from_ron_str`](AnimEffectBindings::from_ron_str). The value
/// per tag is a bare `Vec<Effect>` (no phase — an animation event is an instantaneous fire).
///
/// ```
/// # use engine::AnimEffectBindings;
/// let ron = r#"(bindings: {
///     "footstep": [ SpawnParticles(particles: "dust", count: 8), PlayTone(freq: 200.0, dur: 0.05, vol: 0.15) ],
///     "hit":      [ Flash(color: (1.0, 1.0, 1.0, 1.0), secs: 0.1) ],
/// })"#;
/// let b = AnimEffectBindings::from_ron_str(ron).unwrap();
/// assert_eq!(b.len(), 2);
/// assert_eq!(b.effects_for("footstep").len(), 2);
/// assert!(b.effects_for("unknown").is_empty());
/// ```
#[derive(Deserialize, Clone, Debug, Default)]
pub struct AnimEffectBindings {
    /// Tag-keyed effects: an [`AnimationEvent`] with `tag == "footstep"` runs `bindings["footstep"]`.
    pub bindings: HashMap<String, Vec<Effect>>,
}

impl AnimEffectBindings {
    /// Parse an [`AnimEffectBindings`] table from a RON string (cross-platform).
    ///
    /// Returns `Err` on malformed input rather than panicking.
    pub fn from_ron_str(s: &str) -> Result<Self, AnimEffectError> {
        ron::from_str(s).map_err(|e| AnimEffectError::Ron(e.to_string()))
    }

    /// The effects bound to `tag` (empty slice if none).
    pub fn effects_for(&self, tag: &str) -> &[Effect] {
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

/// Registry of named [`AnimEffectBindings`] tables, stored as a World resource.
///
/// On native builds, load tables via [`App::load_anim_effects`](crate::App::load_anim_effects)
/// (which wires up the file watcher for hot-reload). On wasm, parse with
/// [`AnimEffectBindings::from_ron_str`] and [`insert`](AnimEffectRegistry::insert).
#[derive(Default)]
pub struct AnimEffectRegistry {
    inner: crate::ron_registry::RonRegistry<AnimEffectBindings>,
}

impl AnimEffectRegistry {
    /// Load an [`AnimEffectBindings`] table from `path` and register it under `name` (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(&mut self, name: impl Into<String>, path: &str) -> Result<(), AnimEffectError> {
        self.inner.load(name, path)
    }

    /// Insert a table directly (in-memory; useful for tests or wasm).
    pub fn insert(&mut self, name: impl Into<String>, bindings: AnimEffectBindings) {
        self.inner.insert(name, bindings);
    }

    /// Look up an [`AnimEffectBindings`] table by name.
    pub fn get(&self, name: &str) -> Option<&AnimEffectBindings> {
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
        self.inner.reload_path(path, "anim_effect");
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::ron_registry::RonLoadable for AnimEffectBindings {
    type Err = AnimEffectError;
    fn load_ron(path: &str) -> Result<Self, Self::Err> {
        let text = std::fs::read_to_string(crate::asset_path::resolve(path))?;
        Self::from_ron_str(&text)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl crate::asset::HotReloadable for AnimEffectRegistry {
    fn reload_path(&mut self, path: &str) {
        AnimEffectRegistry::reload_path(self, path);
    }
}

/// Applies a named [`AnimEffectBindings`] table to incoming [`AnimationEvent`]s.
///
/// Add it to the schedule **after** [`AnimationSystem`](crate::animation::AnimationSystem) (and the
/// particle/audio systems so their effects render the same frame). Construct it with the name the
/// table was loaded under: `AnimEffectSystem::new("effects")`.
pub struct AnimEffectSystem {
    name: String,
    warned_no_table: bool,
    warned_missing_particles: bool,
}

impl AnimEffectSystem {
    /// Create a system that applies the binding table registered under `name`.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            warned_no_table: false,
            warned_missing_particles: false,
        }
    }
}

impl System for AnimEffectSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // 1. Clone the active binding table out so we can take `&mut World` later.
        let Some(bindings) = world
            .resource::<AnimEffectRegistry>()
            .and_then(|r| r.get(&self.name).cloned())
        else {
            if !self.warned_no_table {
                log::warn!(
                    "AnimEffectSystem: no anim-effect table registered under '{}'",
                    self.name
                );
                self.warned_no_table = true;
            }
            return;
        };

        // 2. Snapshot this frame's animation events as (entity, tag) pairs.
        let Some(bus) = world.resource::<Events<AnimationEvent>>() else {
            return;
        };
        let events: Vec<(crate::ecs::Entity, String)> = bus
            .read()
            .iter()
            .map(|e| (e.entity, e.tag.clone()))
            .collect();
        if events.is_empty() {
            return;
        }

        // 3. Resolve matching effects into a flat action list (read-only world access). Both the
        //    particle anchor and the flash target are the animating entity (no second anchor here).
        let mut actions = Vec::new();
        let mut missing_particles = false;
        for (entity, tag) in events {
            for effect in bindings.effects_for(&tag) {
                if let Some(pe) =
                    resolve_effect(world, effect, entity, entity, &mut missing_particles)
                {
                    actions.push(pe);
                }
            }
        }

        if missing_particles && !self.warned_missing_particles {
            log::warn!(
                "AnimEffectSystem: a SpawnParticles effect named an emitter not found in any loaded \
                 ParticleConfigRegistry set (did you call App::load_particle_configs?)"
            );
            self.warned_missing_particles = true;
        }

        // 4. Apply the actions (mutating world access).
        apply_pending(world, actions);
    }

    fn name(&self) -> &'static str {
        "AnimEffectSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::components::{Sprite, Transform};
    use crate::ecs::Entity;
    use crate::hit_flash::HitFlash;
    use crate::particle::{ParticleBurst, ParticleConfigRegistry};
    use glam::Vec2;

    const BINDINGS_RON: &str = r#"(bindings: {
        "footstep": [
            SpawnParticles(particles: "dust", count: 8),
            PlayTone(freq: 200.0, dur: 0.05, vol: 0.15),
        ],
        "hit": [ Flash(color: (1.0, 1.0, 1.0, 1.0), secs: 0.1) ],
    })"#;

    fn send_anim_event(world: &mut World, entity: Entity, tag: &str) {
        world
            .resource_mut::<Events<AnimationEvent>>()
            .unwrap()
            .send(AnimationEvent {
                entity,
                clip: 0,
                frame: 1,
                tag: tag.to_string(),
            });
    }

    #[test]
    fn bindings_parse_lookup_and_defaults() {
        let b = AnimEffectBindings::from_ron_str(BINDINGS_RON).expect("parse");
        assert_eq!(b.len(), 2);
        assert!(b.effects_for("unknown").is_empty());
        assert_eq!(b.effects_for("footstep").len(), 2);

        // `count`/`at` defaults apply to the bare-Effect list too.
        match &b.effects_for("footstep")[0] {
            Effect::SpawnParticles {
                particles, count, ..
            } => {
                assert_eq!(particles, "dust");
                assert_eq!(*count, 8);
            }
            other => panic!("expected SpawnParticles, got {other:?}"),
        }
        match &b.effects_for("hit")[0] {
            Effect::Flash { color, secs } => {
                assert_eq!(*color, (1.0, 1.0, 1.0, 1.0));
                assert_eq!(*secs, 0.1);
            }
            other => panic!("expected Flash, got {other:?}"),
        }
    }

    #[test]
    fn malformed_ron_returns_err() {
        assert!(AnimEffectBindings::from_ron_str("not ron at all {{{").is_err());
    }

    #[test]
    fn registry_insert_get_names() {
        let mut reg = AnimEffectRegistry::default();
        assert!(reg.get("fx").is_none());
        reg.insert(
            "fx",
            AnimEffectBindings::from_ron_str(BINDINGS_RON).unwrap(),
        );
        assert_eq!(reg.get("fx").unwrap().len(), 2);
        assert_eq!(reg.names(), vec!["fx".to_string()]);
    }

    #[test]
    fn system_flashes_animating_entity_end_to_end() {
        let mut world = World::new();
        world.insert_resource(Events::<AnimationEvent>::default());
        let mut reg = AnimEffectRegistry::default();
        reg.insert(
            "fx",
            AnimEffectBindings::from_ron_str(BINDINGS_RON).unwrap(),
        );
        world.insert_resource(reg);

        let actor = world.spawn();
        world.add_component(actor, Transform::default());
        world.add_component(actor, Sprite::default());

        send_anim_event(&mut world, actor, "hit");
        AnimEffectSystem::new("fx").run(&mut world, 0.016);

        let flash = world.get::<HitFlash>(actor).expect("HitFlash added");
        assert_eq!(flash.color, Color::rgba(1.0, 1.0, 1.0, 1.0));
        assert_eq!(flash.secs, 0.1);
    }

    #[test]
    fn system_spawns_burst_at_animating_entity() {
        use crate::particle::ParticleConfigSet;

        let mut world = World::new();
        world.insert_resource(Events::<AnimationEvent>::default());
        let mut reg = AnimEffectRegistry::default();
        reg.insert(
            "fx",
            AnimEffectBindings::from_ron_str(BINDINGS_RON).unwrap(),
        );
        world.insert_resource(reg);

        let mut preg = ParticleConfigRegistry::default();
        preg.insert(
            "set",
            ParticleConfigSet::from_ron_str(r#"(emitters: { "dust": (lifetime: 0.4) })"#).unwrap(),
        );
        world.insert_resource(preg);

        let actor = world.spawn();
        world.add_component(
            actor,
            Transform {
                position: Vec2::new(70.0, 20.0),
                ..Default::default()
            },
        );

        send_anim_event(&mut world, actor, "footstep");
        AnimEffectSystem::new("fx").run(&mut world, 0.016);

        // The footstep binding's SpawnParticles fired one burst (count 8) at the actor's position.
        let bursts: Vec<(Entity, u32)> = world
            .query::<ParticleBurst>()
            .map(|(e, b)| (e, b.remaining))
            .collect();
        assert_eq!(bursts.len(), 1);
        assert_eq!(bursts[0].1, 8);
        let pos = world.get::<Transform>(bursts[0].0).unwrap().position;
        assert_eq!(pos, Vec2::new(70.0, 20.0));
    }

    #[test]
    fn spawn_particles_offset_displaces_burst() {
        use crate::particle::ParticleConfigSet;

        let mut world = World::new();
        world.insert_resource(Events::<AnimationEvent>::default());
        let mut reg = AnimEffectRegistry::default();
        reg.insert(
            "fx",
            AnimEffectBindings::from_ron_str(
                r#"(bindings: { "footstep": [
                    SpawnParticles(particles: "dust", count: 4, offset: (0.0, 50.0)),
                ] })"#,
            )
            .unwrap(),
        );
        world.insert_resource(reg);

        let mut preg = ParticleConfigRegistry::default();
        preg.insert(
            "set",
            ParticleConfigSet::from_ron_str(r#"(emitters: { "dust": (lifetime: 0.4) })"#).unwrap(),
        );
        world.insert_resource(preg);

        let actor = world.spawn();
        world.add_component(
            actor,
            Transform {
                position: Vec2::new(70.0, 20.0),
                ..Default::default()
            },
        );

        send_anim_event(&mut world, actor, "footstep");
        AnimEffectSystem::new("fx").run(&mut world, 0.016);

        // The burst spawns at the anchor position PLUS the RON offset (feet, not center).
        let burst = world
            .query::<ParticleBurst>()
            .map(|(e, _)| e)
            .next()
            .expect("one burst");
        let pos = world.get::<Transform>(burst).unwrap().position;
        assert_eq!(pos, Vec2::new(70.0, 70.0));
    }
}
