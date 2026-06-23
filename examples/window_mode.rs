//! Engine **window mode control** — dogfoods [`WindowOptions`](engine::WindowOptions), an
//! opt-in resource for window resizability, fullscreen mode, and aspect-ratio lock.
//!
//! This demo opens a **16:9 aspect-locked** window (`lock_aspect: Some(16.0 / 9.0)`). Drag any
//! edge or corner: the window grows and shrinks but **snaps back to 16:9** — its height is
//! always re-derived from its width, so a fixed-aspect UI can never be distorted by a resize.
//! A border ring hugs the window edges and a 16:9 box stays centered to make the lock visible
//! (both track the live [`ViewportSize`](engine::ViewportSize)).
//!
//! Try the other modes by editing `main`:
//! * `WindowOptions { resizable: false, ..Default::default() }` — a fixed-size window that
//!   simply can't be dragged to a new aspect (the simplest, most robust fixed-aspect option).
//! * `WindowOptions { mode: WindowMode::BorderlessFullscreen, ..Default::default() }` —
//!   borderless fullscreen on the current monitor.
//!
//! Keys: `Esc` quit.
//!
//! Run: `cargo run --example window_mode`

use engine::{
    App, Camera, DrawRect, DrawText, InputState, KeyCode, ShouldQuit, System, TextQueue, UiQueue,
    Vec2, ViewportSize, WindowConfig, WindowMode, WindowOptions, World,
};

/// Locked aspect ratio (width / height).
const ASPECT: f32 = 16.0 / 9.0;

struct WindowModeDemo;

impl System for WindowModeDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                    sq.0 = true;
                }
            }
        }

        // The live inner size — under the aspect lock this is always ~16:9.
        let (w, h) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((1280.0, 720.0));

        if let Some(ui) = world.resource_mut::<UiQueue>() {
            // Border ring hugging the window edges — stays 16:9 because the window is locked.
            ui.push(DrawRect::new(0.0, 0.0, w, h, [90, 170, 250, 230]).with_border(4.0));
            // A 16:9 box centered in the window — stays centered under any resize.
            let bw = w * 0.25;
            let bh = bw / ASPECT;
            ui.push(
                DrawRect::new((w - bw) * 0.5, (h - bh) * 0.5, bw, bh, [60, 120, 200, 110])
                    .with_corner_radius(6.0),
            );
            // Crosshair through the window center.
            ui.push(DrawRect::new(
                w * 0.5 - 1.0,
                0.0,
                2.0,
                h,
                [80, 90, 110, 120],
            ));
            ui.push(DrawRect::new(
                0.0,
                h * 0.5 - 1.0,
                w,
                2.0,
                [80, 90, 110, 120],
            ));
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Window mode: 16:9 aspect-LOCKED (resizable). Drag any edge — it snaps back to 16:9.",
                Vec2::new(20.0, 18.0),
                16.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                format!(
                    "live window size: {:.0} x {:.0}   (ratio {:.3}, target {:.3})",
                    w,
                    h,
                    w / h.max(1.0),
                    ASPECT,
                ),
                Vec2::new(20.0, 44.0),
                15.0,
                [160, 255, 170, 255],
            ));
            tq.push(DrawText::new(
                "Esc quit   ·   edit main() for resizable:false (fixed) or BorderlessFullscreen",
                Vec2::new(20.0, h - 28.0),
                13.0,
                [170, 190, 210, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "WindowModeDemo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — window mode (16:9 lock)".to_string(),
        width: 1280,
        height: 720,
        clear_color: [0.03, 0.03, 0.05, 1.0],
    });
    // Opt into a 16:9 aspect-ratio lock. Absent (or default) = a normal resizable window.
    app.world.insert_resource(WindowOptions {
        resizable: true,
        mode: WindowMode::Windowed,
        lock_aspect: Some(ASPECT),
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(WindowModeDemo);
    app.run();
}
