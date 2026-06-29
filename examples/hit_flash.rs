//! `HitFlash` — briefly flash a sprite white when it is hit, then fade it back.
//!
//! The game-feel staple every action game re-implements: on taking a hit a sprite pops to white (or
//! any color) for a fraction of a second and eases back to normal. Add a [`HitFlash`] to an entity
//! with a [`Sprite`] and [`HitFlashSystem`] does the rest — fading from the flash color back to the
//! sprite's original color, then removing itself. It pairs naturally with a damage event (a
//! [`ZoneEvent`](engine::ZoneEvent), [`CollisionEvent`](engine::CollisionEvent), or animation
//! event): when the entity is hit, add a `HitFlash`.
//!
//! This demo shows three resting-colored targets. Hitting them flashes all three white at once and
//! they fade back to their own colors.
//!
//! - **Space** — hit the targets (flash them white)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/hit_flash.png cargo run --example hit_flash`): with no input the
//! targets continuously re-flash on slightly different durations, so the capture always lands with at
//! least one mid-flash. `HEADLESS_FRAMES=N` overrides the default 90 warm-up frames.
use engine::{
    App, Camera, Color, DrawText, Entity, HitFlash, HitFlashSystem, InputState, KeyCode,
    ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;
const TARGET_Y: f32 = 210.0;
const FLASH_SECS: f32 = 0.18;

/// A target's entity, its display name, and the flash duration used to re-flash it in headless mode
/// (deliberately staggered so the targets fall out of phase and a capture always catches one). The
/// resting color lives on the sprite — `HitFlash` captures it when the flash starts.
struct Target {
    entity: Entity,
    name: &'static str,
    x: f32,
    auto_secs: f32,
}

struct Demo {
    targets: Vec<Target>,
    /// Headless / "plays itself" mode: keep re-flashing each target instead of waiting for input.
    auto: bool,
    /// Total flashes fired (one per target per hit).
    flashes: u32,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // --- input ---
        let (hit, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // --- trigger flashes ---
        if hit {
            // Live "hit": flash every target white at once.
            for t in &self.targets {
                world.add_component(t.entity, HitFlash::white(FLASH_SECS));
            }
            self.flashes += self.targets.len() as u32;
        }
        if self.auto {
            // Headless: re-arm any target whose flash has finished (HitFlashSystem removed it).
            // HitFlashSystem runs before this system, so a finished flash is re-added the same frame
            // it ends — no visible gap.
            let mut rearm: Vec<(Entity, f32)> = Vec::new();
            for t in &self.targets {
                if world.get::<HitFlash>(t.entity).is_none() {
                    rearm.push((t.entity, t.auto_secs));
                }
            }
            for (e, secs) in rearm {
                world.add_component(e, HitFlash::white(secs));
                self.flashes += 1;
            }
        }

        // --- HUD ---
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "HitFlash — flash a sprite when it is hit, then fade back",
                Vec2::new(32.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("flashes fired: {}", self.flashes),
                Vec2::new(32.0, 50.0),
                15.0,
                Color::rgb(0.75, 0.92, 0.95),
            ));
            tq.push(DrawText::new(
                "Space: hit (flash) the targets    Esc: quit",
                Vec2::new(32.0, WIN_H as f32 - 28.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
            // Name each target under its square.
            for t in &self.targets {
                tq.push(DrawText::centered(
                    t.name,
                    Vec2::new(t.x, TARGET_Y + 64.0),
                    14.0,
                    Color::rgb(0.7, 0.72, 0.78),
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "hit_flash_demo"
    }
}

fn spawn_target(app: &mut App, name: &'static str, x: f32, rest: Color, auto_secs: f32) -> Target {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(x, TARGET_Y),
            scale: Vec2::new(84.0, 84.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::colored(rest.r, rest.g, rest.b));
    Target {
        entity: e,
        name,
        x,
        auto_secs,
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "hit_flash — flash on hit, fade back".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    let targets = vec![
        spawn_target(&mut app, "ember", 180.0, Color::rgb(0.85, 0.45, 0.18), 0.16),
        spawn_target(&mut app, "tide", 360.0, Color::rgb(0.20, 0.65, 0.62), 0.22),
        spawn_target(
            &mut app,
            "violet",
            540.0,
            Color::rgb(0.55, 0.35, 0.78),
            0.28,
        ),
    ];

    let headless = std::env::var("HEADLESS_SHOT").is_ok();

    // Order: HitFlashSystem (advances/removes flashes) → Demo (re-arms + triggers). Running the
    // flash system first lets the demo re-flash a just-finished target with no one-frame gap.
    app.add_system(HitFlashSystem);
    app.add_system(Demo {
        targets,
        auto: headless,
        flashes: 0,
    });

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(90);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
