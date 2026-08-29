//! Sprite trail / afterimage — leave a fading ghost behind a moving sprite.
//!
//! The motion-trail every dash, dodge, or fast projectile wants: as a sprite moves, it drops a
//! trail of fading copies of itself behind it. Attach a [`SpriteTrail`] to an entity that has a
//! [`Sprite`] + [`Transform`] and let [`SpriteTrailSystem`] do the rest — every
//! [`interval`](SpriteTrail::interval) seconds it snapshots the sprite into a short-lived **ghost**
//! entity ([`SpriteTrailGhost`]) drawn just behind the source, and fades each ghost's alpha to zero
//! over [`lifetime`](SpriteTrail::lifetime) before despawning it.
//!
//! ```no_run
//! # use engine::{App, SpriteTrail, SpriteTrailSystem};
//! # let mut app = App::new();
//! app.add_system(SpriteTrailSystem);
//! // give a moving entity a trail:
//! // app.world.add_component(dasher, SpriteTrail::new(0.04, 0.4));
//! ```
//!
//! The ghost is a full clone of the source [`Sprite`] (texture / handle / color all carried over),
//! so it works for solid-color and textured sprites alike. Ghosts render at the source's
//! [`Transform`] `z` minus a small offset, so they sit behind the live sprite. A `SpriteTrail` never
//! spawns ghosts *of* ghosts (they have no `SpriteTrail`), and a ghost only fades + despawns — it
//! never touches the source, so unlike a self-despawning effect this is safe to add to a gameplay
//! entity you keep.

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};

/// Default seconds between ghost snapshots.
pub const DEFAULT_TRAIL_INTERVAL: f32 = 0.045;
/// Default ghost lifetime in seconds (how long a snapshot takes to fade out).
pub const DEFAULT_TRAIL_LIFETIME: f32 = 0.4;
/// Default ghost starting-alpha multiplier (fraction of the source sprite's alpha).
pub const DEFAULT_TRAIL_START_ALPHA: f32 = 0.55;
/// How far behind the source (in `Transform.z`) a ghost is drawn, so the live sprite stays on top.
const GHOST_Z_OFFSET: f32 = 0.1;

/// A motion-trail emitter — attach next to a [`Sprite`] + [`Transform`] (see the
/// [module docs](crate::sprite_trail)).
///
/// [`SpriteTrailSystem`] emits one fading [`SpriteTrailGhost`] every [`interval`](Self::interval)
/// seconds. Build one with [`SpriteTrail::new`] (or [`SpriteTrail::default`]) and tune it with the
/// `with_*` builders.
///
/// ```
/// # use engine::SpriteTrail;
/// let trail = SpriteTrail::new(0.05, 0.3).with_start_alpha(0.4);
/// assert_eq!(trail.interval, 0.05);
/// assert_eq!(trail.lifetime, 0.3);
/// assert_eq!(trail.start_alpha, 0.4);
/// ```
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpriteTrail {
    /// Seconds between ghost snapshots. `<= 0` emits one ghost every frame.
    pub interval: f32,
    /// Each ghost's lifetime in seconds — it fades from its start alpha to `0` over this span.
    pub lifetime: f32,
    /// Ghost starting-alpha multiplier applied to the source sprite's alpha (`0.0..=1.0`).
    pub start_alpha: f32,
    /// Time since the last ghost was emitted (runtime state, advanced by [`SpriteTrailSystem`]).
    #[serde(skip)]
    elapsed: f32,
}

impl Default for SpriteTrail {
    fn default() -> Self {
        Self {
            interval: DEFAULT_TRAIL_INTERVAL,
            lifetime: DEFAULT_TRAIL_LIFETIME,
            start_alpha: DEFAULT_TRAIL_START_ALPHA,
            elapsed: 0.0,
        }
    }
}

impl SpriteTrail {
    /// A trail emitting a ghost every `interval` seconds, each fading out over `lifetime` seconds.
    pub fn new(interval: f32, lifetime: f32) -> Self {
        Self {
            interval,
            lifetime,
            ..Self::default()
        }
    }

    /// Set the ghost starting-alpha multiplier (clamped to `0.0..=1.0`).
    pub fn with_start_alpha(mut self, start_alpha: f32) -> Self {
        self.start_alpha = start_alpha.clamp(0.0, 1.0);
        self
    }

    /// Set the ghost lifetime in seconds.
    pub fn with_lifetime(mut self, lifetime: f32) -> Self {
        self.lifetime = lifetime;
        self
    }
}

/// A single fading afterimage dropped by a [`SpriteTrail`]. Carries its own countdown;
/// [`SpriteTrailSystem`] fades its [`Sprite`] alpha toward zero and despawns it when finished.
/// You rarely construct this yourself — the trail emits it.
#[derive(Clone, Debug, PartialEq)]
pub struct SpriteTrailGhost {
    lifetime: f32,
    elapsed: f32,
    /// Alpha at spawn; the ghost fades from here to `0`.
    base_alpha: f32,
}

impl SpriteTrailGhost {
    /// Fade progress in `0.0..=1.0` (`0.0` at spawn, `1.0` when it despawns).
    pub fn progress(&self) -> f32 {
        if self.lifetime > 0.0 {
            (self.elapsed / self.lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Whether the ghost has fully faded — [`SpriteTrailSystem`] despawns it the frame this is true.
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.lifetime
    }
}

/// Emits and fades sprite-trail ghosts each frame (see the [module docs](crate::sprite_trail)).
///
/// User-added (like [`HitFlashSystem`](crate::HitFlashSystem)). Add it once with
/// `App::add_system(SpriteTrailSystem)`; it should run before rendering. It does two passes:
/// snapshot a ghost for every due [`SpriteTrail`], then fade + despawn every [`SpriteTrailGhost`].
#[derive(Default)]
pub struct SpriteTrailSystem;

impl System for SpriteTrailSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Pass 1: advance each source's timer; snapshot a ghost when one is due (at most one per
        // frame, so a slow frame can't spawn a storm). Collect first — we can't spawn while the
        // query borrow is live.
        let mut ghosts: Vec<(Transform, Sprite, f32, f32)> = Vec::new();
        for (_e, trail, transform, sprite) in world.query3_mut::<SpriteTrail, Transform, Sprite>() {
            trail.elapsed += dt;
            let due = trail.interval <= 0.0 || trail.elapsed >= trail.interval;
            if !due {
                continue;
            }
            trail.elapsed = 0.0;
            let mut ghost_tf = transform.clone();
            ghost_tf.z -= GHOST_Z_OFFSET;
            let base_alpha = sprite.color.a * trail.start_alpha.clamp(0.0, 1.0);
            let mut ghost_sprite = sprite.clone();
            ghost_sprite.color.a = base_alpha;
            ghosts.push((ghost_tf, ghost_sprite, trail.lifetime, base_alpha));
        }
        for (tf, sprite, lifetime, base_alpha) in ghosts {
            let g = world.spawn();
            world.add_component(g, tf);
            world.add_component(g, sprite);
            world.add_component(
                g,
                SpriteTrailGhost {
                    lifetime,
                    elapsed: 0.0,
                    base_alpha,
                },
            );
        }

        // Pass 2: fade every ghost toward transparent; despawn the finished ones.
        let mut finished: Vec<Entity> = Vec::new();
        for (e, ghost, sprite) in world.query2_mut::<SpriteTrailGhost, Sprite>() {
            ghost.elapsed += dt;
            sprite.color.a = ghost.base_alpha * (1.0 - ghost.progress());
            if ghost.is_finished() {
                finished.push(e);
            }
        }
        for e in finished {
            world.despawn(e);
        }
    }

    fn name(&self) -> &'static str {
        "SpriteTrailSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn mover(world: &mut World, trail: SpriteTrail) -> Entity {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(10.0, 0.0),
                ..Default::default()
            },
        );
        world.add_component(e, Sprite::colored(1.0, 1.0, 1.0));
        world.add_component(e, trail);
        e
    }

    /// Count ghost entities (those with a `SpriteTrailGhost`).
    fn ghost_count(world: &mut World) -> usize {
        world.query::<SpriteTrailGhost>().count()
    }

    #[test]
    fn emits_a_ghost_after_the_interval() {
        let mut world = World::new();
        mover(&mut world, SpriteTrail::new(0.1, 0.5));
        let mut sys = SpriteTrailSystem;

        sys.run(&mut world, 0.05); // < interval → no ghost yet
        assert_eq!(ghost_count(&mut world), 0);
        sys.run(&mut world, 0.06); // crosses 0.1 → one ghost
        assert_eq!(ghost_count(&mut world), 1);
    }

    #[test]
    fn ghost_snapshots_position_and_sits_behind() {
        let mut world = World::new();
        let src = mover(&mut world, SpriteTrail::new(0.0, 0.5)); // interval 0 → every frame
        let src_z = world.get::<Transform>(src).unwrap().z;
        SpriteTrailSystem.run(&mut world, 0.016);

        let (_e, gt) = world
            .query::<Transform>()
            .find(|(e, _)| *e != src)
            .expect("a ghost transform");
        assert_eq!(
            gt.position,
            Vec2::new(10.0, 0.0),
            "snapshot the source position"
        );
        assert!(gt.z < src_z, "ghost is drawn behind the source");
    }

    #[test]
    fn ghost_starts_at_the_start_alpha_and_fades() {
        let mut world = World::new();
        mover(&mut world, SpriteTrail::new(0.0, 0.2).with_start_alpha(0.5));
        let mut sys = SpriteTrailSystem;
        sys.run(&mut world, 0.0); // emit a ghost at t=0 (progress 0 → full base alpha 0.5)

        let a0 = world
            .query::<Sprite>()
            .filter(|(_, s)| s.color.a < 1.0)
            .map(|(_, s)| s.color.a)
            .next()
            .expect("ghost sprite");
        assert!((a0 - 0.5).abs() < 1e-5, "starts at start_alpha, got {a0}");
    }

    #[test]
    fn ghost_despawns_when_faded() {
        let mut world = World::new();
        let src = mover(&mut world, SpriteTrail::new(0.0, 0.1)); // interval 0 → emit every frame
        let mut sys = SpriteTrailSystem;
        sys.run(&mut world, 0.01); // emit one ghost (its age is 0.01 after this frame's fade pass)
        assert_eq!(ghost_count(&mut world), 1, "one ghost emitted");

        // Stop the source emitting, then advance past the ghost lifetime → it despawns.
        world.remove_component::<SpriteTrail>(src);
        sys.run(&mut world, 0.2);
        assert_eq!(ghost_count(&mut world), 0, "faded ghost despawned");
    }

    #[test]
    fn a_ghost_never_spawns_ghosts() {
        // After emitting, running many frames must not blow up the ghost count unboundedly beyond
        // what one source at its interval produces (ghosts have no SpriteTrail → emit nothing).
        let mut world = World::new();
        mover(&mut world, SpriteTrail::new(0.05, 1.0));
        let mut sys = SpriteTrailSystem;
        for _ in 0..4 {
            sys.run(&mut world, 0.05);
        }
        // One source, ~4 ghosts alive (lifetime 1.0 keeps them). No exponential growth.
        assert!(ghost_count(&mut world) <= 5, "no ghost-of-ghost explosion");
    }

    #[test]
    fn progress_and_is_finished_accessors() {
        let g = SpriteTrailGhost {
            lifetime: 0.2,
            elapsed: 0.0,
            base_alpha: 1.0,
        };
        assert_eq!(g.progress(), 0.0);
        assert!(!g.is_finished());
    }
}
