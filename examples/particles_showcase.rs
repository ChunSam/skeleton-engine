//! particles_showcase — the new `ParticleEmitter` depth knobs (gravity + emit shape).
//!
//! Three emitters side by side:
//!   * **Fountain** — particles shoot up and arc back down under positive `gravity`.
//!   * **Fire** — negative (buoyant) `gravity` makes embers rise + accelerate, spawned over a
//!     `Circle` so the base has width.
//!   * **Sparks** — strong downward `gravity` + a `Box` emit shape for a scattered shower.
//!
//! Run from the repo root:  `cargo run --example particles_showcase`   (ESC to quit)

use engine::{
    App, Color, DrawText, EmitShape, InputState, KeyCode, ParticleEmitter, ParticleSystem,
    ShouldQuit, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

struct QuitHud;

impl System for QuitHud {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(q) = world.resource_mut::<ShouldQuit>() {
                    q.quit();
                }
            }
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for (label, x) in [
                ("fountain (gravity down)", 120.0),
                ("fire (gravity up)", 400.0),
                ("sparks (box + gravity down)", 600.0),
            ] {
                tq.push(DrawText::new(
                    label,
                    Vec2::new(x, 40.0),
                    16.0,
                    Color::rgb(0.7, 0.8, 0.9),
                ));
            }
            tq.push(DrawText::new(
                "ESC to quit",
                Vec2::new(20.0, 16.0),
                16.0,
                Color::rgb(0.5, 0.6, 0.7),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "quit_hud"
    }
}

/// Builds a configured emitter from `Default` then sets the public fields. (External code can't use
/// a `ParticleEmitter { .. }` struct literal because the internal `timer` field is crate-private.)
#[allow(clippy::too_many_arguments, clippy::field_reassign_with_default)]
fn emitter(
    spawn_rate: f32,
    lifetime: f32,
    velocity: Vec2,
    velocity_spread: Vec2,
    color_start: Color,
    color_end: Color,
    size: Vec2,
    gravity: Vec2,
    emit_shape: EmitShape,
) -> ParticleEmitter {
    let mut e = ParticleEmitter::default();
    e.spawn_rate = spawn_rate;
    e.lifetime = lifetime;
    e.velocity = velocity;
    e.velocity_spread = velocity_spread;
    e.color_start = color_start;
    e.color_end = color_end;
    e.size = size;
    e.gravity = gravity;
    e.emit_shape = emit_shape;
    e
}

fn spawn_emitter(app: &mut App, pos: Vec2, em: ParticleEmitter) {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: pos,
            scale: Vec2::splat(1.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(e, em);
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "particles_showcase — gravity + emit shape".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.03, 0.03, 0.05, 1.0],
    });

    // Fountain: launch up, arc back down under positive gravity.
    spawn_emitter(
        &mut app,
        Vec2::new(220.0, 470.0),
        emitter(
            220.0,
            1.6,
            Vec2::new(0.0, -260.0),
            Vec2::new(60.0, 30.0),
            Color::rgb(0.4, 0.8, 1.0),
            Color::rgba(0.1, 0.3, 0.8, 0.0),
            Vec2::splat(6.0),
            Vec2::new(0.0, 360.0),
            EmitShape::Point,
        ),
    );

    // Fire: buoyant (negative gravity) embers rising from a circular base.
    spawn_emitter(
        &mut app,
        Vec2::new(470.0, 430.0),
        emitter(
            160.0,
            1.2,
            Vec2::new(0.0, -30.0),
            Vec2::new(18.0, 12.0),
            Color::rgb(1.0, 0.85, 0.3),
            Color::rgba(1.0, 0.2, 0.05, 0.0),
            Vec2::splat(7.0),
            Vec2::new(0.0, -90.0),
            EmitShape::Circle { radius: 16.0 },
        ),
    );

    // Sparks: scattered from a box, raining down hard.
    spawn_emitter(
        &mut app,
        Vec2::new(690.0, 200.0),
        emitter(
            120.0,
            1.4,
            Vec2::new(0.0, 40.0),
            Vec2::new(90.0, 30.0),
            Color::rgb(1.0, 0.95, 0.5),
            Color::rgba(0.8, 0.4, 0.1, 0.0),
            Vec2::splat(4.0),
            Vec2::new(0.0, 320.0),
            EmitShape::Box {
                half_extents: Vec2::new(80.0, 6.0),
            },
        ),
    );

    app.add_system(ParticleSystem);
    app.add_system(QuitHud);
    app.run();
}
