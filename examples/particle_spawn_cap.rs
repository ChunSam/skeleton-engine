//! `ParticleEmitter::max_per_frame` — the per-frame spawn cap.
//!
//! Two rain emitters with the **same** high `spawn_rate` (≈7680/s ⇒ 128 particles/frame at 60 fps)
//! sit side by side. A continuous emitter spawns at most `max_per_frame` particles per frame as a
//! runaway guard:
//!
//! - **Left**  — default cap `64`: requests 128/frame but only 64 spawn, so dense rain silently
//!   under-emits (≈half the density).
//! - **Right** — `with_max_per_frame(256)`: all 128/frame spawn ⇒ full density.
//!
//! The live particle count under each column makes the difference objective. Before this knob the
//! cap was hardcoded to 64, so a fork wanting dense rain/snow had to edit engine source.
use engine::{
    App, Color, DrawText, EmitShape, ParticleEmitter, ParticleSystem, System, TextQueue, Transform,
    Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 460;
const LEFT_CX: f32 = 180.0;
const RIGHT_CX: f32 = 540.0;
/// x splitting the two columns (for the per-column live count).
const SPLIT_X: f32 = 360.0;

/// Builds a downpour emitter: same rate for both columns, only `max_per_frame` differs.
fn rain(max_per_frame: u32) -> ParticleEmitter {
    let mut e = ParticleEmitter::default();
    e.spawn_rate = 7680.0; // 128 / frame at 60 fps — above the default 64 cap
    e.lifetime = 0.62;
    e.velocity = Vec2::new(0.0, 650.0); // fall down (top-left-origin: +y is down)
    e.velocity_spread = Vec2::new(25.0, 50.0);
    e.gravity = Vec2::new(0.0, 450.0);
    e.size = Vec2::new(3.0, 11.0);
    e.color_start = Color::rgba(0.6, 0.8, 1.0, 0.95);
    e.color_end = Color::rgba(0.5, 0.6, 0.9, 0.0);
    e.emit_shape = EmitShape::Box {
        half_extents: Vec2::new(150.0, 3.0),
    };
    e.max_per_frame = max_per_frame;
    e
}

fn spawn_rain(app: &mut App, cx: f32, max_per_frame: u32) {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(cx, 8.0),
            ..Default::default()
        },
    );
    app.world.add_component(e, rain(max_per_frame));
}

/// Counts live particles per column and draws the labels + counts.
struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (mut left, mut right) = (0u32, 0u32);
        for (_, _p, tr) in world.query2::<engine::Particle, Transform>() {
            if tr.position.x < SPLIT_X {
                left += 1;
            } else {
                right += 1;
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::centered(
                "Default cap (64/frame)",
                Vec2::new(LEFT_CX, 24.0),
                17.0,
                [240, 210, 160, 255],
            ));
            tq.push(DrawText::centered(
                format!("live: {left}"),
                Vec2::new(LEFT_CX, 46.0),
                15.0,
                [220, 220, 230, 255],
            ));
            tq.push(DrawText::centered(
                "Raised cap (256/frame)",
                Vec2::new(RIGHT_CX, 24.0),
                17.0,
                [160, 220, 255, 255],
            ));
            tq.push(DrawText::centered(
                format!("live: {right}"),
                Vec2::new(RIGHT_CX, 46.0),
                15.0,
                [220, 220, 230, 255],
            ));
            tq.push(DrawText::new(
                "Same spawn_rate (7680/s). The 64-cap emitter under-emits dense rain; raising the cap restores full density.",
                Vec2::new(16.0, WIN_H as f32 - 26.0),
                13.0,
                [180, 195, 215, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ParticleEmitter::max_per_frame — per-frame spawn cap".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    spawn_rain(&mut app, LEFT_CX, 64); // default cap
    spawn_rain(&mut app, RIGHT_CX, 256); // raised cap

    app.add_system(ParticleSystem);
    app.add_system(HudSystem);
    app.run();
}
