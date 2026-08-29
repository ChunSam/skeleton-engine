//! Hit-flash — briefly tint a [`Sprite`], then fade it back, when something hits it.
//!
//! Every action game flashes a sprite (usually white) for a fraction of a second when it takes a
//! hit, then eases it back to its normal color. Attach a [`HitFlash`] to an entity that has a
//! [`Sprite`] and let [`HitFlashSystem`] drive it: the sprite snaps to [`HitFlash::color`] and fades
//! back to the color it had when the flash began over [`HitFlash::secs`], after which the system
//! removes the component (restoring the original color exactly). This pairs naturally with a damage
//! event — e.g. a [`ZoneEvent`](crate::ZoneEvent) from a damage [`TriggerZone`](crate::TriggerZone),
//! a [`CollisionEvent`](crate::CollisionEvent), or an [`AnimationEvent`](crate::AnimationEvent) — by
//! adding a `HitFlash` to the entity that was hit.
//!
//! ```no_run
//! # use engine::{App, HitFlash, HitFlashSystem, Color};
//! # let mut app = App::new();
//! app.add_system(HitFlashSystem);
//! // when an entity is hit:
//! // app.world.add_component(entity, HitFlash::white(0.15));
//! ```
//!
//! The pre-flash color is captured on the **first** system run, not at construction, so the system
//! does not need to know the sprite's color in advance. (Re-adding a `HitFlash` while one is still
//! fading captures the mid-flash color as the new base — to re-trigger cleanly, let the current
//! flash finish, or set the sprite's color before re-adding.)

use crate::color::Color;
use crate::components::Sprite;
use crate::ecs::{Entity, System, World};
use crate::tween::Lerp;

/// A transient color flash on a [`Sprite`] (see the [module docs](crate::hit_flash)).
///
/// Build one with [`HitFlash::white`] (a white "hit" pop) or [`HitFlash::new`] (any flash color),
/// then add it next to a [`Sprite`]. [`HitFlashSystem`] fades the sprite from [`color`](Self::color)
/// back to its pre-flash color over [`secs`](Self::secs) and then removes the component.
///
/// ```
/// # use engine::{HitFlash, Color};
/// let flash = HitFlash::white(0.2);
/// assert_eq!(flash.color, Color::WHITE);
/// assert_eq!(flash.secs, 0.2);
/// assert_eq!(flash.progress(), 0.0);
/// assert!(!flash.is_finished());
/// ```
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HitFlash {
    /// The color the sprite snaps to at the start of the flash (e.g. white for a "hit" pop).
    pub color: Color,
    /// Total flash duration in seconds — the sprite fades from [`color`](Self::color) back to its
    /// pre-flash color over this span.
    pub secs: f32,
    /// Elapsed time, advanced by [`HitFlashSystem`] (runtime state — read it via
    /// [`progress`](Self::progress)).
    #[serde(skip)]
    elapsed: f32,
    /// The sprite's color captured when the flash began, faded back to as the flash ends.
    /// `None` until the first system run captures it.
    #[serde(skip)]
    base: Option<Color>,
}

impl Default for HitFlash {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            secs: 0.15,
            elapsed: 0.0,
            base: None,
        }
    }
}

impl HitFlash {
    /// A flash toward `color` lasting `secs` seconds.
    pub fn new(color: Color, secs: f32) -> Self {
        Self {
            color,
            secs,
            ..Self::default()
        }
    }

    /// A white "hit" flash lasting `secs` seconds — the common case.
    pub fn white(secs: f32) -> Self {
        Self::new(Color::WHITE, secs)
    }

    /// Flash progress in `0.0..=1.0` (`0.0` at the start, `1.0` when finished). `0` means the sprite
    /// is fully [`color`](Self::color); `1` means it is back to its pre-flash color.
    pub fn progress(&self) -> f32 {
        if self.secs > 0.0 {
            (self.elapsed / self.secs).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Whether the flash has run its full duration. [`HitFlashSystem`] removes the component the
    /// frame this becomes true (after restoring the pre-flash color).
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.secs
    }
}

/// Fades every [`HitFlash`] sprite back to its pre-flash color each frame and removes the component
/// when the flash finishes (see the [module docs](crate::hit_flash)).
///
/// User-added (like [`YSortSystem`](crate::YSortSystem)). Add it once with
/// `App::add_system(HitFlashSystem)`; it should run before rendering. An entity needs both a
/// [`HitFlash`] and a [`Sprite`]; a `HitFlash` on an entity without a `Sprite` is inert (and is
/// never removed, since there is nothing to flash).
#[derive(Default)]
pub struct HitFlashSystem;

impl System for HitFlashSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let mut finished: Vec<Entity> = Vec::new();
        for (e, flash, sprite) in world.query2_mut::<HitFlash, Sprite>() {
            // Capture the pre-flash color on the first run, so the fade-out lands back on it exactly.
            let base = *flash.base.get_or_insert(sprite.color);
            flash.elapsed += dt;
            // t = 0 → full flash color; t = 1 → original color restored.
            sprite.color = Color::lerp(&flash.color, &base, flash.progress());
            if flash.is_finished() {
                finished.push(e);
            }
        }
        for e in finished {
            world.remove_component::<HitFlash>(e);
        }
    }

    fn name(&self) -> &'static str {
        "HitFlashSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_sprite(world: &mut World, color: Color) -> Entity {
        let e = world.spawn();
        world.add_component(e, Sprite::colored(color.r, color.g, color.b));
        e
    }

    #[test]
    fn fades_from_flash_color_back_to_base_then_removes() {
        let mut world = World::new();
        let e = spawn_sprite(&mut world, Color::RED);
        world.add_component(e, HitFlash::white(0.1));

        let mut sys = HitFlashSystem;

        // Halfway: color is lerped halfway between white (flash) and red (base).
        sys.run(&mut world, 0.05);
        let mid = world.get::<Sprite>(e).unwrap().color;
        assert!((mid.r - 1.0).abs() < 1e-5, "r={}", mid.r); // 1→1
        assert!((mid.g - 0.5).abs() < 1e-5, "g={}", mid.g); // 0→1 at t=0.5
        assert!((mid.b - 0.5).abs() < 1e-5, "b={}", mid.b);
        assert!(
            world.get::<HitFlash>(e).is_some(),
            "still flashing at t=0.5"
        );

        // Finish: fully back to the base color, component removed.
        sys.run(&mut world, 0.05);
        assert_eq!(world.get::<Sprite>(e).unwrap().color, Color::RED);
        assert!(
            world.get::<HitFlash>(e).is_none(),
            "HitFlash removed when finished"
        );
    }

    #[test]
    fn flash_color_dominates_at_the_start() {
        let mut world = World::new();
        let e = spawn_sprite(&mut world, Color::BLACK);
        world.add_component(e, HitFlash::white(0.2));

        // A tiny step in → almost fully the flash (white) color.
        HitFlashSystem.run(&mut world, 0.001);
        let c = world.get::<Sprite>(e).unwrap().color;
        assert!(c.r > 0.99 && c.g > 0.99 && c.b > 0.99, "near-white: {c:?}");
        assert!(world.get::<HitFlash>(e).is_some());
    }

    #[test]
    fn base_is_captured_at_first_run_not_construction() {
        // A flash added to a green sprite must fade back to green even though the sprite's color is
        // only read at the first system run.
        let mut world = World::new();
        let e = spawn_sprite(&mut world, Color::GREEN);
        world.add_component(e, HitFlash::new(Color::RED, 0.1));

        let mut sys = HitFlashSystem;
        sys.run(&mut world, 0.05); // capture base (green), half-faded
        sys.run(&mut world, 0.05); // finish
        assert_eq!(world.get::<Sprite>(e).unwrap().color, Color::GREEN);
    }

    #[test]
    fn zero_duration_restores_immediately_and_removes() {
        let mut world = World::new();
        let e = spawn_sprite(&mut world, Color::RED);
        world.add_component(e, HitFlash::new(Color::WHITE, 0.0));

        HitFlashSystem.run(&mut world, 0.016);
        // secs == 0 → progress is 1 → sprite stays its base color, component gone the same frame.
        assert_eq!(world.get::<Sprite>(e).unwrap().color, Color::RED);
        assert!(world.get::<HitFlash>(e).is_none());
    }

    #[test]
    fn sprite_without_hitflash_is_untouched() {
        let mut world = World::new();
        let e = spawn_sprite(&mut world, Color::BLUE);
        HitFlashSystem.run(&mut world, 0.016);
        assert_eq!(world.get::<Sprite>(e).unwrap().color, Color::BLUE);
    }

    #[test]
    fn progress_and_is_finished_accessors() {
        let fresh = HitFlash::white(0.1);
        assert_eq!(fresh.progress(), 0.0);
        assert!(!fresh.is_finished());

        // A zero-duration flash is finished from the start (progress saturates to 1).
        let instant = HitFlash::new(Color::WHITE, 0.0);
        assert_eq!(instant.progress(), 1.0);
        assert!(instant.is_finished());
    }
}
