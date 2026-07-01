//! `ProgressBar` — a read-only UI gauge for health / mana / loading / XP.
//!
//! Attach a [`ProgressBar`] next to a [`UiNode`] and drive its `value` (`0.0..=1.0`) each frame; the
//! [`UiSystem`] draws a background track + a fill scaled to the value. No input — unlike a
//! [`Slider`](engine::Slider) the game owns the value. Rounded corners + an optional border come
//! from the existing `DrawRect` UI pipeline.
//!
//! This demo drives three bars over time: an oscillating **health** bar, a looping **loading** bar,
//! and a slowly-filling **XP** bar.
//!
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_progress.png cargo run --example ui_progress`): the bars animate
//! on a clock so a capture lands mid-fill. `HEADLESS_FRAMES=N` overrides the default 30 warm-up
//! frames.
use engine::{
    App, Color, DrawText, Entity, InputState, KeyCode, ProgressBar, ShouldQuit, System, TextQueue,
    UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;
const BAR_X: f32 = 150.0;
const BAR_W: f32 = 440.0;
const BAR_H: f32 = 30.0;

struct Bar {
    entity: Entity,
    label: &'static str,
    /// Fill fraction for frame time `t`.
    value_at: fn(f32) -> f32,
    y: f32,
}

struct Demo {
    bars: Vec<Bar>,
    t: f32,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.t += dt;

        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Drive each bar's value from the clock.
        for bar in &self.bars {
            let v = (bar.value_at)(self.t);
            if let Some(pb) = world.get_mut::<ProgressBar>(bar.entity) {
                pb.value = v;
            }
        }

        // HUD: title + a label and percentage beside each bar.
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "ProgressBar — read-only gauges (health / loading / XP)",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            for bar in &self.bars {
                tq.push(DrawText::new(
                    bar.label,
                    Vec2::new(28.0, bar.y + 6.0),
                    15.0,
                    Color::rgb(0.78, 0.82, 0.9),
                ));
                let pct = ((bar.value_at)(self.t) * 100.0).round() as i32;
                tq.push(DrawText::new(
                    format!("{pct:>3}%"),
                    Vec2::new(BAR_X + BAR_W + 12.0, bar.y + 6.0),
                    15.0,
                    Color::rgb(0.75, 0.9, 0.95),
                ));
            }
            tq.push(DrawText::new(
                "Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_progress_demo"
    }
}

fn oscillate(t: f32) -> f32 {
    0.5 + 0.5 * (t * 1.2).sin()
}
fn loop_fill(t: f32) -> f32 {
    (t * 0.3).rem_euclid(1.0)
}
fn slow_fill(t: f32) -> f32 {
    (t * 0.11).rem_euclid(1.0)
}

fn spawn_bar(app: &mut App, y: f32, bar: ProgressBar) -> Entity {
    let e = app.world.spawn();
    app.world
        .add_component(e, UiNode::new(BAR_X, y, BAR_W, BAR_H));
    app.world.add_component(e, bar);
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_progress — read-only gauges".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    let border = Color::rgba(0.0, 0.0, 0.0, 0.5);
    let bg = Color::rgba(0.14, 0.15, 0.19, 1.0);
    let (health_y, load_y, xp_y) = (120.0, 200.0, 280.0);

    let health = spawn_bar(
        &mut app,
        health_y,
        ProgressBar::new(1.0)
            .with_colors(Color::rgb(0.35, 0.82, 0.42), bg)
            .with_corner_radius(8.0)
            .with_border(2.0, border),
    );
    let loading = spawn_bar(
        &mut app,
        load_y,
        ProgressBar::new(0.0)
            .with_colors(Color::rgb(0.30, 0.66, 0.95), bg)
            .with_corner_radius(8.0)
            .with_border(2.0, border),
    );
    let xp = spawn_bar(
        &mut app,
        xp_y,
        ProgressBar::new(0.0)
            .with_colors(Color::rgb(0.95, 0.78, 0.30), bg)
            .with_corner_radius(8.0)
            .with_border(2.0, border),
    );

    app.add_system(UiSystem::new());
    app.add_system(Demo {
        bars: vec![
            Bar {
                entity: health,
                label: "Health",
                value_at: oscillate,
                y: health_y,
            },
            Bar {
                entity: loading,
                label: "Loading",
                value_at: loop_fill,
                y: load_y,
            },
            Bar {
                entity: xp,
                label: "XP",
                value_at: slow_fill,
                y: xp_y,
            },
        ],
        t: 0.0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
