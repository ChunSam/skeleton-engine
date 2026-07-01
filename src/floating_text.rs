//! Floating combat text — spawn short-lived text at a world position that rises and fades out.
//!
//! The genre-agnostic game-feel staple: a damage number, a "+50", a "MISS", or a pickup name pops
//! up at a world location, drifts (usually upward), fades, and vanishes. Spawn a dedicated entity
//! carrying a [`Transform`] + [`FloatingText`] — via [`spawn_floating_text`] or
//! [`App::spawn_floating_text`](crate::App::spawn_floating_text) — and let [`FloatingTextSystem`]
//! drive it: each frame it advances the age, drifts the entity by [`FloatingText::velocity`], fades
//! the alpha toward zero over [`FloatingText::lifetime`], draws the text through the [`TextQueue`]
//! (screen-space, projected from the world position via [`Camera::world_to_screen`]), and
//! **despawns the whole entity** when the lifetime elapses.
//!
//! Because the system despawns the entity, a `FloatingText` belongs on its own ephemeral entity
//! (like a one-shot particle) — do **not** attach one to an entity you want to keep. This pairs
//! naturally with a damage event: when something is hit, spawn a floating number at its position
//! (and optionally add a [`HitFlash`](crate::HitFlash) to the thing that was hit).
//!
//! ```no_run
//! # use engine::{App, FloatingText, FloatingTextSystem, Vec2, Color};
//! # let mut app = App::new();
//! app.add_system(FloatingTextSystem);
//! // when an entity at `pos` takes 15 damage:
//! app.spawn_floating_text_colored(Vec2::new(120.0, 80.0), "-15", Color::RED);
//! ```
//!
//! World "up" is the negative-Y direction in this engine (Y increases downward, as in
//! [`YSort`](crate::YSort)), so the default [`velocity`](FloatingText::velocity) has a negative Y —
//! the text rises. The drift is in world units, so it is scaled by the camera zoom like anything
//! else in the world.

use crate::camera::Camera;
use crate::color::Color;
use crate::components::Transform;
use crate::ecs::{Entity, System, World};
use crate::renderer::{DrawText, TextQueue};
use glam::Vec2;

/// Default upward drift speed in world units per second (negative Y is up — see the module docs).
pub const DEFAULT_FLOAT_SPEED: f32 = 42.0;
/// Default lifetime in seconds before the entity is despawned.
pub const DEFAULT_FLOAT_LIFETIME: f32 = 1.0;
/// Default font size in pixels.
pub const DEFAULT_FLOAT_SIZE: f32 = 22.0;

/// A short-lived piece of text that drifts and fades at a world position (see the
/// [module docs](crate::floating_text)).
///
/// Build one with [`FloatingText::new`] (white, rising) or [`FloatingText::colored`], tune it with
/// the `with_*` builders, then add it next to a [`Transform`] on a dedicated entity.
/// [`FloatingTextSystem`] drives and eventually despawns it.
///
/// ```
/// # use engine::{FloatingText, Color};
/// let ft = FloatingText::colored("-15", Color::RED).with_lifetime(0.8);
/// assert_eq!(ft.text, "-15");
/// assert_eq!(ft.lifetime, 0.8);
/// assert_eq!(ft.progress(), 0.0);
/// assert!(!ft.is_finished());
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct FloatingText {
    /// The string drawn each frame.
    pub text: String,
    /// The text color. When [`fade`](Self::fade) is set, its alpha is scaled toward zero over the
    /// lifetime (the stored value is the color at birth).
    pub color: Color,
    /// Drift velocity in world units per second. Defaults to rising ([`DEFAULT_FLOAT_SPEED`] up).
    pub velocity: Vec2,
    /// Font size in pixels.
    pub size: f32,
    /// Total lifetime in seconds. When the age reaches this, [`FloatingTextSystem`] despawns the
    /// entity.
    pub lifetime: f32,
    /// Fade the alpha from `color.a` down to `0` across the lifetime (default `true`). Set `false`
    /// to keep full opacity until it pops out.
    pub fade: bool,
    /// Elapsed seconds, advanced by [`FloatingTextSystem`] (runtime state — read via
    /// [`progress`](Self::progress)).
    elapsed: f32,
}

impl Default for FloatingText {
    fn default() -> Self {
        Self {
            text: String::new(),
            color: Color::WHITE,
            velocity: Vec2::new(0.0, -DEFAULT_FLOAT_SPEED),
            size: DEFAULT_FLOAT_SIZE,
            lifetime: DEFAULT_FLOAT_LIFETIME,
            fade: true,
            elapsed: 0.0,
        }
    }
}

impl FloatingText {
    /// A white, rising floating text with the default size and lifetime.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }

    /// A rising floating text in `color`, with the default size and lifetime.
    pub fn colored(text: impl Into<String>, color: Color) -> Self {
        Self {
            color,
            ..Self::new(text)
        }
    }

    /// Set the drift velocity in world units per second (negative Y is up).
    pub fn with_velocity(mut self, velocity: Vec2) -> Self {
        self.velocity = velocity;
        self
    }

    /// Set the font size in pixels.
    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the lifetime in seconds.
    pub fn with_lifetime(mut self, lifetime: f32) -> Self {
        self.lifetime = lifetime;
        self
    }

    /// Enable or disable the alpha fade-out (default enabled).
    pub fn with_fade(mut self, fade: bool) -> Self {
        self.fade = fade;
        self
    }

    /// Lifetime progress in `0.0..=1.0` (`0.0` at birth, `1.0` when it expires). A `lifetime` of
    /// `0` (or less) reads as `1.0` (already finished).
    pub fn progress(&self) -> f32 {
        if self.lifetime > 0.0 {
            (self.elapsed / self.lifetime).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Whether the text has lived its full lifetime — [`FloatingTextSystem`] despawns the entity the
    /// frame this becomes true.
    pub fn is_finished(&self) -> bool {
        self.elapsed >= self.lifetime
    }
}

/// Spawn a dedicated ephemeral entity carrying `ft` at `world_pos` (a fresh [`Transform`] +
/// [`FloatingText`]) and return it.
///
/// Usable anywhere you hold a `&mut World` — including from inside a [`System`] (e.g. a collision or
/// damage handler), which is where floating combat text is usually spawned.
/// [`App::spawn_floating_text`](crate::App::spawn_floating_text) is a thin wrapper for the app
/// context. [`FloatingTextSystem`] despawns the entity when the text expires.
pub fn spawn_floating_text(world: &mut World, world_pos: Vec2, ft: FloatingText) -> Entity {
    let e = world.spawn();
    world.add_component(
        e,
        Transform {
            position: world_pos,
            ..Default::default()
        },
    );
    world.add_component(e, ft);
    e
}

/// Drives every [`FloatingText`] each frame — ages it, drifts it by its velocity, fades its alpha,
/// draws it through the [`TextQueue`], and despawns the entity when its lifetime elapses (see the
/// [module docs](crate::floating_text)).
///
/// User-added (like [`HitFlashSystem`](crate::HitFlashSystem)). Add it once with
/// `App::add_system(FloatingTextSystem)`; it should run before rendering. An entity needs both a
/// [`FloatingText`] and a [`Transform`]; the text is projected to the screen with the
/// [`Camera`] resource (if present — otherwise the `Transform` position is used as a screen
/// coordinate directly, so it still works in a camera-less UI game).
#[derive(Default)]
pub struct FloatingTextSystem;

impl System for FloatingTextSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Copy the camera out first (it is `Copy`) to release the resource borrow before the query.
        let camera = world.resource::<Camera>().copied();

        // Advance + drift every text, collecting draw commands and expired entities. We cannot touch
        // `TextQueue`/despawn while the `query2_mut` borrow is live, so buffer both (the standard
        // collect-then-apply pattern).
        let mut draws: Vec<DrawText> = Vec::new();
        let mut finished: Vec<Entity> = Vec::new();
        for (e, ft, transform) in world.query2_mut::<FloatingText, Transform>() {
            ft.elapsed += dt;
            transform.position += ft.velocity * dt;
            if ft.is_finished() {
                finished.push(e);
                continue;
            }
            let mut color = ft.color;
            if ft.fade {
                color.a *= 1.0 - ft.progress();
            }
            let screen = match camera {
                Some(c) => c.world_to_screen(transform.position),
                None => transform.position,
            };
            draws.push(DrawText::centered(ft.text.clone(), screen, ft.size, color));
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for d in draws {
                tq.push(d);
            }
        }
        for e in finished {
            world.despawn(e);
        }
    }

    fn name(&self) -> &'static str {
        "FloatingTextSystem"
    }
}

impl crate::app::App {
    /// Spawn a white, rising floating text reading `text` at the world position `world_pos`, and
    /// return the ephemeral entity. Convenience wrapper over [`spawn_floating_text`] for the app
    /// context; [`FloatingTextSystem`] despawns it when it expires.
    pub fn spawn_floating_text(&mut self, world_pos: Vec2, text: impl Into<String>) -> Entity {
        spawn_floating_text(&mut self.world, world_pos, FloatingText::new(text))
    }

    /// Like [`spawn_floating_text`](Self::spawn_floating_text) but in `color` (e.g. red for damage,
    /// green for healing).
    pub fn spawn_floating_text_colored(
        &mut self,
        world_pos: Vec2,
        text: impl Into<String>,
        color: Color,
    ) -> Entity {
        spawn_floating_text(
            &mut self.world,
            world_pos,
            FloatingText::colored(text, color),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn(world: &mut World, ft: FloatingText, pos: Vec2) -> Entity {
        spawn_floating_text(world, pos, ft)
    }

    #[test]
    fn rises_and_despawns_when_finished() {
        let mut world = World::new();
        world.insert_resource(TextQueue::default());
        let e = spawn(
            &mut world,
            FloatingText::new("+1").with_lifetime(0.1),
            Vec2::ZERO,
        );

        let mut sys = FloatingTextSystem;
        // Half a lifetime: still alive, drifted up (negative Y) by velocity * dt.
        sys.run(&mut world, 0.05);
        let pos = world.get::<Transform>(e).unwrap().position;
        assert!(pos.y < 0.0, "rose upward (negative Y), got {}", pos.y);
        assert!(world.is_alive(e), "still alive at half lifetime");

        // Past the lifetime: the system despawns the whole entity.
        sys.run(&mut world, 0.05);
        assert!(!world.is_alive(e), "despawned when finished");
    }

    #[test]
    fn fades_alpha_toward_zero() {
        let mut world = World::new();
        world.insert_resource(TextQueue::default());
        let color = Color::rgba(1.0, 0.0, 0.0, 1.0);
        spawn(
            &mut world,
            FloatingText::colored("-9", color).with_lifetime(1.0),
            Vec2::ZERO,
        );

        // At ~25% of the lifetime the queued text's alpha is ~0.75 of the original.
        FloatingTextSystem.run(&mut world, 0.25);
        let tq = world.resource::<TextQueue>().unwrap();
        let item = tq.iter().next().expect("one text queued");
        assert!(
            (item.color.a - 0.75).abs() < 1e-5,
            "alpha faded to 0.75, got {}",
            item.color.a
        );
    }

    #[test]
    fn no_fade_keeps_full_alpha() {
        let mut world = World::new();
        world.insert_resource(TextQueue::default());
        spawn(
            &mut world,
            FloatingText::new("hi").with_lifetime(1.0).with_fade(false),
            Vec2::ZERO,
        );

        FloatingTextSystem.run(&mut world, 0.5);
        let tq = world.resource::<TextQueue>().unwrap();
        assert_eq!(
            tq.iter().next().unwrap().color.a,
            1.0,
            "no fade → full alpha"
        );
    }

    #[test]
    fn projects_through_the_camera() {
        let mut world = World::new();
        world.insert_resource(TextQueue::default());
        // Camera at (100, 100), zoom 2 → world (110, 100) maps to screen ((10)*2, 0) = (20, 0).
        world.insert_resource(Camera::new(Vec2::new(100.0, 100.0), 2.0));
        spawn(
            &mut world,
            // No drift/fade so the projected position is exactly predictable this frame.
            FloatingText::new("x")
                .with_velocity(Vec2::ZERO)
                .with_fade(false),
            Vec2::new(110.0, 100.0),
        );

        FloatingTextSystem.run(&mut world, 0.0);
        let tq = world.resource::<TextQueue>().unwrap();
        let pos = tq.iter().next().unwrap().position;
        assert!(
            (pos.x - 20.0).abs() < 1e-4 && pos.y.abs() < 1e-4,
            "projected {pos:?}"
        );
    }

    #[test]
    fn progress_and_is_finished_accessors() {
        let fresh = FloatingText::new("a").with_lifetime(0.5);
        assert_eq!(fresh.progress(), 0.0);
        assert!(!fresh.is_finished());

        // A zero-lifetime text is finished from the start (progress saturates to 1).
        let instant = FloatingText::new("b").with_lifetime(0.0);
        assert_eq!(instant.progress(), 1.0);
        assert!(instant.is_finished());
    }

    #[test]
    fn works_without_a_camera_resource() {
        // No Camera inserted → the Transform position is used directly as a screen coordinate.
        let mut world = World::new();
        world.insert_resource(TextQueue::default());
        spawn(
            &mut world,
            FloatingText::new("ui")
                .with_velocity(Vec2::ZERO)
                .with_fade(false),
            Vec2::new(64.0, 48.0),
        );

        FloatingTextSystem.run(&mut world, 0.0);
        let tq = world.resource::<TextQueue>().unwrap();
        assert_eq!(tq.iter().next().unwrap().position, Vec2::new(64.0, 48.0));
    }
}
