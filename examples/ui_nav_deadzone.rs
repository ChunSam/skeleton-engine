//! `StickNavConfig` — tunable analog-stick deadzone for UI focus navigation.
//!
//! When a gamepad is connected, the left stick drives UI focus the same way Tab / the D-pad do:
//! pushing it Down/Up steps focus across the focusable widgets. To turn the continuous axis into
//! discrete steps the engine edge-detects it with hysteresis — a push past `activate` fires **one**
//! step, and the stick must relax back inside `release` (the neutral band) before it can fire again.
//! Those two thresholds were hardcoded (`0.6` / `0.35`); `StickNavConfig` makes them a fork-friendly
//! opt-in resource (absent = the historical behavior, byte-identical).
//!
//! This demo shows the deadzone live: a horizontal bar maps the left-stick **X** axis from −1..1 with
//! the `release` (neutral) band and `±activate` step zones marked, plus a marker for the current
//! stick position. Push the stick and watch the marker cross the bands; the focus ring steps as it
//! enters an activate zone, then re-arms once it returns to the neutral band.
//!
//! Tune it at runtime to feel the difference:
//! - **`[` / `]`** — decrease / increase `activate` (a tighter activate = snappier, fires sooner)
//! - **`,` / `.`** — decrease / increase `release` (a wider neutral band = more jitter rejection)
//! - **Tab / Shift+Tab** (or stick / D-pad Down/Up) — move focus · **Esc** — quit
//!
//! With no gamepad connected the bar simply rests at center; keyboard Tab still navigates, and the
//! configured thresholds are shown — the knob's effect is also covered by unit tests in
//! `src/ui/system/state.rs`.
//!
//! Headless: `HEADLESS_SHOT=/tmp/deadzone.png cargo run --example ui_nav_deadzone`.
use engine::{
    App, Button, Color, DrawRect, DrawText, FocusRingStyle, GamepadAxis, GamepadState, InputState,
    KeyCode, ShouldQuit, StickNavConfig, System, TextQueue, UiEvent, UiFocus, UiNode, UiQueue,
    UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 780;
const WIN_H: u32 = 470;

const LIST_X: f32 = 60.0;
const LIST_Y: f32 = 110.0;
const BTN_W: f32 = 260.0;
const BTN_H: f32 = 44.0;
const BTN_GAP: f32 = 14.0;

// Deadzone visualization bar (screen-space, maps stick X ∈ [-1, 1]).
const BAR_X: f32 = 400.0;
const BAR_Y: f32 = 150.0;
const BAR_W: f32 = 320.0;
const BAR_H: f32 = 40.0;

const STEP: f32 = 0.05;

struct Demo {
    items: Vec<engine::Entity>,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input: quit + runtime threshold tuning ─────────────────────────────
        // Collect the edge-triggered keys first (immutable borrow), then mutate resources.
        let mut quit = false;
        let mut d_activate = 0.0;
        let mut d_release = 0.0;
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            if input.just_pressed(KeyCode::BracketRight) {
                d_activate += STEP;
            }
            if input.just_pressed(KeyCode::BracketLeft) {
                d_activate -= STEP;
            }
            if input.just_pressed(KeyCode::Period) {
                d_release += STEP;
            }
            if input.just_pressed(KeyCode::Comma) {
                d_release -= STEP;
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if d_activate != 0.0 || d_release != 0.0 {
            if let Some(cfg) = world.resource_mut::<StickNavConfig>() {
                cfg.activate = (cfg.activate + d_activate).clamp(0.05, 1.0);
                cfg.release = (cfg.release + d_release).clamp(0.0, 1.0);
            }
        }

        // `resolved()` applies the same clamping the focus pass uses, so the bar matches behavior.
        let (activate, release) = world
            .resource::<StickNavConfig>()
            .copied()
            .unwrap_or_default()
            .resolved();

        // Live left-stick X (rest at 0 when no pad is connected).
        let (stick_x, stick_y, connected) = world
            .resource::<GamepadState>()
            .and_then(|gp| gp.primary().map(|p| (gp, p)))
            .map(|(gp, p)| {
                (
                    gp.axis(p, GamepadAxis::LeftStickX),
                    gp.axis(p, GamepadAxis::LeftStickY),
                    true,
                )
            })
            .unwrap_or((0.0, 0.0, false));

        let focused = world.resource::<UiFocus>().and_then(|f| f.entity);
        let focused_idx = self.items.iter().position(|&e| Some(e) == focused);

        self.draw_bar(world, activate, release, stick_x);

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "StickNavConfig — analog-stick deadzone for UI focus navigation",
                Vec2::new(LIST_X, 34.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("activate ±{activate:.2}   release ±{release:.2}   (hold band = release..activate)"),
                Vec2::new(BAR_X, BAR_Y - 30.0),
                15.0,
                Color::rgb(0.75, 0.82, 0.95),
            ));
            let stick_str = if connected {
                format!("left stick: x {stick_x:+.2}  y {stick_y:+.2}")
            } else {
                "left stick: (no gamepad — keyboard Tab still navigates)".to_string()
            };
            tq.push(DrawText::new(
                stick_str,
                Vec2::new(BAR_X, BAR_Y + BAR_H + 16.0),
                15.0,
                Color::rgb(0.7, 0.85, 0.7),
            ));
            let focus_str = match focused_idx {
                Some(i) => format!("focused: item {}", i + 1),
                None => "focused: none".to_string(),
            };
            tq.push(DrawText::new(
                focus_str,
                Vec2::new(BAR_X, BAR_Y + BAR_H + 40.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
            tq.push(DrawText::new(
                "[ / ]: activate    , / .: release    Tab/stick: move focus    Esc: quit",
                Vec2::new(LIST_X, WIN_H as f32 - 30.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_nav_deadzone_demo"
    }
}

impl Demo {
    /// Draws the −1..1 stick-X bar with the neutral (release) band, the two ±activate step zones, and
    /// a marker at the live stick position. Purely illustrative — the focus pass owns the real logic.
    fn draw_bar(&self, world: &mut World, activate: f32, release: f32, stick_x: f32) {
        let Some(uq) = world.resource_mut::<UiQueue>() else {
            return;
        };
        let cx = BAR_X + BAR_W / 2.0;
        let to_x = |v: f32| cx + v.clamp(-1.0, 1.0) * (BAR_W / 2.0);

        // Track background.
        uq.push(DrawRect::new(
            BAR_X,
            BAR_Y,
            BAR_W,
            BAR_H,
            Color::rgb(0.16, 0.17, 0.22),
        ));
        // Activate step zones (outer): |x| >= activate.
        let act_left = to_x(-activate);
        let act_right = to_x(activate);
        uq.push(DrawRect::new(
            BAR_X,
            BAR_Y,
            act_left - BAR_X,
            BAR_H,
            Color::rgba(0.3, 0.7, 0.4, 0.55),
        ));
        uq.push(DrawRect::new(
            act_right,
            BAR_Y,
            BAR_X + BAR_W - act_right,
            BAR_H,
            Color::rgba(0.3, 0.7, 0.4, 0.55),
        ));
        // Neutral (release) band, centered: |x| <= release.
        let rel_left = to_x(-release);
        uq.push(DrawRect::new(
            rel_left,
            BAR_Y,
            to_x(release) - rel_left,
            BAR_H,
            Color::rgba(0.55, 0.35, 0.3, 0.6),
        ));
        // Live stick marker.
        let mx = to_x(stick_x);
        uq.push(DrawRect::new(mx - 3.0, BAR_Y - 6.0, 6.0, BAR_H + 12.0, Color::WHITE).with_z(1.0));
    }
}

fn spawn_item(app: &mut App, idx: usize) -> engine::Entity {
    let y = LIST_Y + idx as f32 * (BTN_H + BTN_GAP);
    let e = app.world.spawn();
    app.world
        .add_component(e, UiNode::new(LIST_X, y, BTN_W, BTN_H));
    app.world
        .add_component(e, Button::new(format!("Menu item {}", idx + 1)));
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_nav_deadzone — analog-stick UI deadzone".into(),
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

    // The knob under test. Defaults to the historical (0.6 / 0.35); tune it live with [ ] , .
    app.world.insert_resource(StickNavConfig::default());

    let items: Vec<_> = (0..4).map(|i| spawn_item(&mut app, i)).collect();
    app.world.insert_resource(UiFocus {
        entity: Some(items[0]),
    });

    app.add_system(UiSystem::new());
    app.add_system(Demo { items });

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
