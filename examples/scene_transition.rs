//! Scene transitions — fade / wipe / iris with automatic scene swapping.
//!
//! Each numbered key transitions to the *next* level with a different [`TransitionStyle`]. The
//! transition covers the screen, [`start_scene_transition`] swaps to the next level *while hidden*
//! (a fresh background colour), then the same style reveals it — so you watch the wipe or iris
//! uncover the new scene. The whole thing is one call from a system; `App` handles the cover → swap
//! → reveal timing.
//!
//! - **1** Fade   **2** WipeLeft   **3** WipeRight   **4** WipeDown
//! - **5** WipeUp   **6** IrisIn   **7** IrisOut
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/transition.png cargo run --example scene_transition`): captures a
//! mid-iris overlay over the first level so the styled coverage is visible in a still.
use engine::{
    start_scene_transition, App, Color, DrawRect, DrawText, InputState, KeyCode, Scene,
    SceneTransition, ShouldQuit, System, SystemRegistrar, TextQueue, TransitionStyle, UiQueue,
    WindowConfig, World,
};
use glam::Vec2;

// 1280×720 matches the headless capture size, so the full-screen level colour + the fullscreen
// transition overlay both fill the frame and the iris centres on the content in a still.
const WIN_W: f32 = 1280.0;
const WIN_H: f32 = 720.0;

/// Per-level (background colour, title). Transitions cycle through them.
const LEVELS: [(Color, &str); 4] = [
    (Color::rgba(0.60, 0.20, 0.24, 1.0), "LEVEL 1"),
    (Color::rgba(0.20, 0.44, 0.30, 1.0), "LEVEL 2"),
    (Color::rgba(0.22, 0.34, 0.56, 1.0), "LEVEL 3"),
    (Color::rgba(0.40, 0.28, 0.52, 1.0), "LEVEL 4"),
];

/// (key, style, label) — the transition menu.
const STYLES: [(KeyCode, TransitionStyle, &str); 7] = [
    (KeyCode::Digit1, TransitionStyle::Fade, "1 Fade"),
    (KeyCode::Digit2, TransitionStyle::WipeLeft, "2 WipeLeft"),
    (KeyCode::Digit3, TransitionStyle::WipeRight, "3 WipeRight"),
    (KeyCode::Digit4, TransitionStyle::WipeDown, "4 WipeDown"),
    (KeyCode::Digit5, TransitionStyle::WipeUp, "5 WipeUp"),
    (KeyCode::Digit6, TransitionStyle::IrisIn, "6 IrisIn"),
    (KeyCode::Digit7, TransitionStyle::IrisOut, "7 IrisOut"),
];

struct LevelScene {
    idx: usize,
}

impl Scene for LevelScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(LevelSystem { idx: self.idx });
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct LevelSystem {
    idx: usize,
}

impl System for LevelSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input: pick a style → transition to the next level (ignored mid-transition) ──
        let busy = world.resource::<SceneTransition>().is_some();
        let (mut chosen, mut quit) = (None, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            if !busy {
                for (key, style, _) in STYLES {
                    if input.just_pressed(key) {
                        chosen = Some(style);
                    }
                }
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if let Some(style) = chosen {
            let next = (self.idx + 1) % LEVELS.len();
            start_scene_transition(world, Box::new(LevelScene { idx: next }), style, 0.45);
        }

        // ── Render: full-screen level colour + title ──
        let (bg, title) = LEVELS[self.idx];
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            ui.items
                .push(DrawRect::new(0.0, 0.0, WIN_W, WIN_H, bg).with_z(0.0));
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::centered(
                title,
                Vec2::new(WIN_W * 0.5, WIN_H * 0.42),
                72.0,
                Color::rgb(0.96, 0.95, 0.9),
            ));
            tq.push(DrawText::centered(
                "1 Fade    2 WipeLeft    3 WipeRight    4 WipeDown",
                Vec2::new(WIN_W * 0.5, WIN_H - 84.0),
                20.0,
                Color::rgba(0.95, 0.95, 0.95, 0.9),
            ));
            tq.push(DrawText::centered(
                "5 WipeUp    6 IrisIn    7 IrisOut        Esc quit",
                Vec2::new(WIN_W * 0.5, WIN_H - 54.0),
                20.0,
                Color::rgba(0.95, 0.95, 0.95, 0.9),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "scene_transition_level"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Scene transitions — 1-7 styles, Esc quit".to_string(),
        width: WIN_W as u32,
        height: WIN_H as u32,
        clear_color: [0.05, 0.05, 0.07, 1.0],
    });
    app.set_scene(Box::new(LevelScene { idx: 0 }));

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        // Freeze a mid-iris overlay over the first level so the styled coverage shows in a still.
        // IrisIn covers *outside* a shrinking circle, so the level shows through a central window.
        let mut t = SceneTransition::new(TransitionStyle::IrisIn, 100.0); // ~static (tiny speed)
        t.coverage = 0.5;
        t.color = Color::rgba(0.03, 0.03, 0.05, 1.0);
        app.world.insert_resource(t);
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(6);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
