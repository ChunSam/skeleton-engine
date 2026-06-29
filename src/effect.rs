//! Shared effect vocabulary + application for the data-driven event→effect bindings.
//!
//! Both [`zone_effect`](crate::zone_effect) (react to `ZoneEvent`s) and
//! [`anim_effect`](crate::anim_effect) (react to `AnimationEvent`s) author an [`Effect`] per
//! trigger and apply it the same way — spawn a one-shot particle burst, play a tone, or flash a
//! sprite. This module holds that shared [`Effect`] / [`EffectAnchor`] vocabulary plus the
//! `pub(crate)` application machinery (`PendingEffect`, `lookup_emitter`, `resolve_effect`,
//! `apply_pending`) so neither binding module duplicates it.
//!
//! A binding module's job is only to (1) read its event stream, (2) resolve each event to the
//! effect list it triggers, and (3) pick the anchor/target entities for each effect — then call
//! `resolve_effect` + `apply_pending`.

use glam::Vec2;
use serde::Deserialize;

use crate::audio_facade::Audio;
use crate::color::Color;
use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, World};
use crate::hit_flash::HitFlash;
use crate::particle::{ParticleBurst, ParticleConfigRegistry, ParticleEmitter};

/// Which entity an [`Effect::SpawnParticles`] burst is anchored at.
///
/// Interpreted by the binding module: [`zone_effect`](crate::zone_effect) maps `Other` → the entity
/// that entered the zone and `Zone` → the zone entity; [`anim_effect`](crate::anim_effect) anchors
/// every burst at the animating entity, so it does **not** consult this field.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EffectAnchor {
    /// The entity the event is about (the entrant, for a zone) — the default.
    #[default]
    Other,
    /// The zone entity (zone bindings only).
    Zone,
}

/// An effect an event→effect binding runs when its trigger fires.
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
        /// Where the burst is anchored (default [`EffectAnchor::Other`]; ignored by `anim_effect`).
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
    /// Add a [`HitFlash`] to the target entity (it needs a [`Sprite`] for the flash to show; the
    /// effect is skipped if it has none).
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

/// A resolved effect to apply this frame (built read-only by [`resolve_effect`], applied with
/// `&mut World` by [`apply_pending`]).
pub(crate) enum PendingEffect {
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

/// Find an emitter named `name` across every loaded [`ParticleConfigRegistry`] set (first match).
pub(crate) fn lookup_emitter(world: &World, name: &str) -> Option<ParticleEmitter> {
    let reg = world.resource::<ParticleConfigRegistry>()?;
    reg.names()
        .iter()
        .find_map(|set| reg.get(set).and_then(|s| s.emitter(name)))
}

/// Resolve one [`Effect`] into a [`PendingEffect`] (read-only world access).
///
/// `particle_anchor` is the entity whose [`Transform`] positions a [`Effect::SpawnParticles`] burst;
/// `flash_target` is the entity an [`Effect::Flash`] adds a [`HitFlash`] to. Returns `None` when the
/// effect can't fire this frame — a `SpawnParticles` whose emitter name isn't loaded (then
/// `*missing_particles` is set, so the caller can warn once) or whose anchor has no `Transform`.
pub(crate) fn resolve_effect(
    world: &World,
    effect: &Effect,
    particle_anchor: Entity,
    flash_target: Entity,
    missing_particles: &mut bool,
) -> Option<PendingEffect> {
    match effect {
        Effect::Flash { color, secs } => Some(PendingEffect::Flash {
            target: flash_target,
            color: Color::rgba(color.0, color.1, color.2, color.3),
            secs: *secs,
        }),
        Effect::PlayTone {
            freq,
            dur,
            vol,
            bus,
        } => Some(PendingEffect::Tone {
            freq: *freq,
            dur: *dur,
            vol: *vol,
            bus: bus.clone(),
        }),
        Effect::SpawnParticles {
            particles, count, ..
        } => {
            let t = world.get::<Transform>(particle_anchor)?;
            let (pos, z) = (t.position, t.z);
            match lookup_emitter(world, particles) {
                Some(mut emitter) => {
                    // Burst-only: silence the emitter's continuous emission.
                    emitter.emit = false;
                    emitter.spawn_rate = 0.0;
                    Some(PendingEffect::Burst {
                        pos,
                        z,
                        emitter,
                        count: *count,
                    })
                }
                None => {
                    *missing_particles = true;
                    None
                }
            }
        }
    }
}

/// Apply resolved effects (mutating world access): spawn burst entities, add [`HitFlash`]es, play
/// tones. Shared by both binding systems.
pub(crate) fn apply_pending(world: &mut World, actions: Vec<PendingEffect>) {
    for action in actions {
        match action {
            PendingEffect::Flash {
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
            PendingEffect::Tone {
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
            PendingEffect::Burst {
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
