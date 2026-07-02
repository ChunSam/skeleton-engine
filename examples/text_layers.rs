//! Text z-ordering — `DrawText::with_z` composites text **among** the UI rects.
//!
//! Historically all text drew in one final pass on top of every UI rect, so an overlay (an open
//! dropdown list, a tooltip, a panel) could never hide a label underneath — the label bled
//! through. A [`DrawText`] with [`with_z`](DrawText::with_z) now interleaves with the rects by z:
//! a rect drawn above it covers it, a rect below it stays behind. Text **without** a z keeps the
//! old always-on-top behavior (right for HUD readouts) — the engine's widget passes set their
//! label z automatically.
//!
//! Two overlapping cards, each with a caption. The overlay card covers part of the bottom card's
//! caption — with the overlay raised, the covered part of the caption disappears (no bleed); drop
//! the overlay below the caption's z and the caption reads through again. The HUD title has no z,
//! so it stays on top of everything.
//!
//! - **Space** — raise/lower the overlay card (z 0.8 ↔ 0.1)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/text_layers.png cargo run --example text_layers`): captures the
//! raised-overlay state. `HEADLESS_FRAMES=N` overrides the default 10 warm-up frames.
use engine::{
    App, Color, DrawRect, DrawText, InputState, KeyCode, ShouldQuit, System, TextQueue, UiQueue,
    Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

struct Demo {
    /// Overlay card raised above the bottom card's caption (true) or dropped below it (false).
    raised: bool,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (quit, toggle) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Escape),
                    i.just_pressed(KeyCode::Space),
                )
            })
            .unwrap_or((false, false));
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if toggle {
            self.raised = !self.raised;
        }
        let overlay_z = if self.raised { 0.8 } else { 0.1 };

        if let Some(uq) = world.resource_mut::<UiQueue>() {
            // Bottom card (z 0.3) — its caption is layered at the same z.
            uq.push(
                DrawRect::new(
                    90.0,
                    120.0,
                    340.0,
                    190.0,
                    Color::rgba(0.16, 0.22, 0.32, 1.0),
                )
                .with_corner_radius(10.0)
                .with_z(0.3),
            );
            // Overlay card — covers the right half of the bottom card's caption when raised.
            uq.push(
                DrawRect::new(
                    260.0,
                    90.0,
                    340.0,
                    190.0,
                    Color::rgba(0.38, 0.26, 0.16, 1.0),
                )
                .with_corner_radius(10.0)
                .with_z(overlay_z),
            );
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            // Bottom card caption — layered at the card's z: covered by the raised overlay.
            tq.push(
                DrawText::new(
                    "bottom card caption reads until it slides under the overlay",
                    Vec2::new(106.0, 210.0),
                    17.0,
                    Color::rgb(0.85, 0.9, 0.97),
                )
                .with_bounds(Vec2::new(310.0, 80.0))
                .with_z(0.3),
            );
            // Overlay caption — layered just above its own card.
            tq.push(
                DrawText::new(
                    if self.raised {
                        "overlay card (raised) — covers the caption underneath"
                    } else {
                        "overlay card (dropped) — the caption reads over me now"
                    },
                    Vec2::new(276.0, 110.0),
                    17.0,
                    Color::rgb(0.97, 0.9, 0.8),
                )
                .with_bounds(Vec2::new(310.0, 60.0))
                .with_z(overlay_z + 0.001),
            );
            // HUD — NO z: always on top, even over the overlay card it crosses.
            tq.push(DrawText::new(
                "text z-ordering — HUD text (no z) stays on top of every card",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!(
                    "Space: raise/lower the overlay (now {})   Esc: quit",
                    if self.raised {
                        "RAISED z=0.8"
                    } else {
                        "DROPPED z=0.1"
                    }
                ),
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "text_layers_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "text_layers — DrawText::with_z".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.add_system(Demo { raised: true });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
