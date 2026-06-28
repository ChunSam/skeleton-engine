//! `Slider::keyboard_step` — per-slider keyboard/gamepad nudge step.
//!
//! A focused `Slider` moves by a fixed step on each ←/→ (or D-pad / left-stick) press. That
//! step was hardcoded to `DEFAULT_SLIDER_STEP_FRAC` (5% of the range) — fine for a continuous
//! volume slider, wrong for a slider that selects a few discrete levels (5% of a 0..3 range is
//! 0.15, so a press lands *between* levels). `Slider::with_keyboard_step(s)` overrides it with
//! an absolute step in value units.
//!
//! This demo has two sliders:
//! - **Volume** (0..100) — default step → 5 per press (20 presses end to end).
//! - **Quality** (0..3, `with_keyboard_step(1.0)`) — exactly one level per press
//!   (Low / Medium / High / Ultra), never landing between levels.
//!
//! **Tab / Shift+Tab** move focus, **←/→** nudge the focused slider, **Esc** quits.
//!
//! Headless: `HEADLESS_SHOT=/tmp/sliders.png cargo run --example slider_keyboard_step`.
use engine::{
    App, Color, DrawText, FocusRingStyle, InputState, KeyCode, ShouldQuit, Slider, System,
    TextQueue, UiEvent, UiFocus, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

const VOLUME_X: f32 = 60.0;
const VOLUME_Y: f32 = 120.0;
const QUALITY_Y: f32 = 230.0;
const SLIDER_W: f32 = 320.0;

const QUALITY_LABELS: [&str; 4] = ["Low", "Medium", "High", "Ultra"];

struct Demo {
    volume: engine::Entity,
    quality: engine::Entity,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        let vol = world
            .get::<Slider>(self.volume)
            .map(|s| s.value)
            .unwrap_or(0.0);
        let (q_val, q_step) = world
            .get::<Slider>(self.quality)
            .map(|s| (s.value, s.resolved_keyboard_step()))
            .unwrap_or((0.0, 0.0));
        let q_level = (q_val.round() as usize).min(QUALITY_LABELS.len() - 1);
        let focused = world.resource::<UiFocus>().and_then(|f| f.entity);
        let which = match focused {
            f if f == Some(self.volume) => "Volume",
            f if f == Some(self.quality) => "Quality",
            _ => "none",
        };

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Slider::keyboard_step — per-slider ←/→ nudge step",
                Vec2::new(60.0, 28.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("Volume  {vol:.0} / 100      (default step: 5% of range = 5 per press)"),
                Vec2::new(VOLUME_X, VOLUME_Y - 34.0),
                15.0,
                Color::rgb(0.75, 0.82, 0.95),
            ));
            tq.push(DrawText::new(
                format!(
                    "Quality  {}  ({q_val:.0}/3)      (with_keyboard_step({q_step:.0}) = one level per press)",
                    QUALITY_LABELS[q_level]
                ),
                Vec2::new(VOLUME_X, QUALITY_Y - 34.0),
                15.0,
                Color::rgb(0.75, 0.82, 0.95),
            ));
            tq.push(DrawText::new(
                format!(
                    "focused: {which}    —    Tab/Shift+Tab: move focus   ←/→: nudge   Esc: quit"
                ),
                Vec2::new(60.0, WIN_H as f32 - 30.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "slider_keyboard_step_demo"
    }
}

fn spawn_slider(app: &mut App, y: f32, slider: Slider) -> engine::Entity {
    let e = app.world.spawn();
    app.world
        .add_component(e, UiNode::new(VOLUME_X, y, SLIDER_W, 28.0));
    app.world.add_component(e, slider);
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "slider_keyboard_step — per-slider nudge step".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    app.world.insert_resource(FocusRingStyle {
        color: Color::rgb(0.3, 0.9, 1.0),
        thickness: 4.0,
        pulse_hz: 1.2,
        pulse_min_alpha: 0.4,
        ..Default::default()
    });

    // Continuous slider: default step (5% of 0..100 = 5 per press).
    let volume = spawn_slider(&mut app, VOLUME_Y, Slider::new(0.0, 100.0, 50.0));
    // Discrete slider: 0..3 with an absolute step of 1 → exactly one quality level per press.
    let quality = spawn_slider(
        &mut app,
        QUALITY_Y,
        Slider::new(0.0, 3.0, 2.0).with_keyboard_step(1.0),
    );

    // Start focused on the Quality slider so the difference is front-and-center.
    app.world.insert_resource(UiFocus {
        entity: Some(quality),
    });

    app.add_system(UiSystem::new());
    app.add_system(Demo { volume, quality });

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
    if let Ok(path) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        app.save_screenshot_headless(frames, &path)
            .expect("headless screenshot");
        println!("wrote {path} ({frames} frames)");
        return;
    }
    app.run();
}
