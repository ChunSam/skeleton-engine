//! `SpriteTrail` — leave a fading afterimage behind a moving sprite.
//!
//! The motion trail every dash / dodge / fast projectile wants: attach a [`SpriteTrail`] to a moving
//! entity with a [`Sprite`] and [`SpriteTrailSystem`] drops a fading ghost copy behind it every few
//! frames, each fading out and despawning on its own. Because a ghost only fades itself (it never
//! touches the source), it is safe to add to a gameplay entity you keep.
//!
//! This demo orbits a box; its trail arcs behind it. Toggle the trail to watch the ghosts fade out.
//!
//! - **Space** — toggle the trail on/off
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/sprite_trail.png cargo run --example sprite_trail`): the box orbits
//! with the trail on, so a capture shows the fading arc. `HEADLESS_FRAMES=N` overrides the default
//! 60 warm-up frames.
use engine::{
    App, Camera, Color, DrawText, Entity, InputState, KeyCode, ShouldQuit, Sprite, SpriteTrail,
    SpriteTrailGhost, SpriteTrailSystem, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 460;
const CENTER: Vec2 = Vec2::new(380.0, 235.0);
const RADIUS: f32 = 130.0;
const ANGULAR_SPEED: f32 = 2.6; // rad/s

struct Demo {
    box_entity: Entity,
    angle: f32,
    trail_on: bool,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (space, quit) = world
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

        // Space toggles the trail: add the component to start, remove it to stop (existing ghosts
        // keep fading out on their own).
        if space {
            self.trail_on = !self.trail_on;
            if self.trail_on {
                world.add_component(
                    self.box_entity,
                    SpriteTrail::new(0.03, 0.45).with_start_alpha(0.7),
                );
            } else {
                world.remove_component::<SpriteTrail>(self.box_entity);
            }
        }

        // Orbit the box (fast enough for a visible trail).
        self.angle += ANGULAR_SPEED * dt;
        let pos = CENTER + Vec2::new(self.angle.cos(), self.angle.sin()) * RADIUS;
        if let Some(t) = world.get_mut::<Transform>(self.box_entity) {
            t.position = pos;
        }

        // Count live ghosts before borrowing the TextQueue.
        let ghosts = world.query::<SpriteTrailGhost>().count();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "SpriteTrail — a fading afterimage behind a moving sprite",
                Vec2::new(28.0, 22.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!(
                    "trail: {}    ghosts alive: {}",
                    if self.trail_on { "ON" } else { "OFF" },
                    ghosts
                ),
                Vec2::new(28.0, 50.0),
                15.0,
                if self.trail_on {
                    Color::rgb(0.55, 0.9, 0.7)
                } else {
                    Color::rgb(0.85, 0.7, 0.6)
                },
            ));
            tq.push(DrawText::new(
                "Space: toggle trail    Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "sprite_trail_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "sprite_trail — fading afterimage".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    let box_entity = app.world.spawn();
    app.world.add_component(
        box_entity,
        Transform {
            position: CENTER + Vec2::new(RADIUS, 0.0),
            scale: Vec2::new(36.0, 36.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(box_entity, Sprite::colored(0.35, 0.75, 1.0));
    app.world.add_component(
        box_entity,
        SpriteTrail::new(0.03, 0.45).with_start_alpha(0.7),
    );

    // Order: Demo moves the box, then SpriteTrailSystem snapshots it at the new position → the trail
    // marks where the box has just been.
    app.add_system(Demo {
        box_entity,
        angle: 0.0,
        trail_on: true,
    });
    app.add_system(SpriteTrailSystem);

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
