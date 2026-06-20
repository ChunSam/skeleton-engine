//! `DrawText::centered` horizontal-centering visual check (wishlist **EW-001** regression demo).
//!
//! Renders [`DrawText::centered`] labels at several **off-center** screen-x positions, each with a
//! full-height vertical guide line drawn through that exact `position.x`. The fix (skeleton-engine
//! v0.43.6) makes the [`TextAnchor::Center`] anchor offset measure the *real* shaped glyph center
//! instead of assuming `text_width / 2`, so every label's horizontal center lands ON its guide line
//! for any x. Before the fix, an off-center centered label drifted ~half the viewport to the right.
//!
//! What to look for (everything is in logical-pixel screen space — `DrawText`, `DebugDraw`, and
//! `ViewportSize` all share it, so the alignment holds at any window scale / DPR):
//! * Each **cyan** `centered` label straddles its white guide line symmetrically — center on the line.
//! * The **green** multi-line block centers *every* line on the middle guide: the fix keeps
//!   `align = Center`, so a multi-line centered title still centers per line (it does not left-align).
//! * For contrast, the **amber** [`DrawText::new`] label (default top-left anchor) starts *at* the
//!   guide, not centered on it — the visible difference between anchoring and not.
//!
//! Run natively: `cargo run --example centered_text` (Esc quits).
//!
//! Run in the browser (eyeball the EW-001 fix on the web):
//! ```text
//! examples/centered_text/web/build.sh
//! python3 -m http.server 8080 --directory examples/centered_text/web
//! open http://localhost:8080        # click "Start"
//! ```

use engine::{
    App, DebugDraw, DrawText, FontData, InputState, KeyCode, ShouldQuit, System, TextQueue, Vec2,
    ViewportSize, WindowConfig, World,
};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

/// Bundled deterministic Latin font (same as the headless EW-001 regression test) so the demo's
/// metrics match across platforms instead of depending on the host's system fonts.
const DEJAVU: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/DejaVuSans.ttf"
));

const GUIDE: [u8; 4] = [255, 255, 255, 70]; // faint white vertical guides
const CYAN: [u8; 4] = [120, 220, 255, 255]; // centered single-line labels
const GREEN: [u8; 4] = [170, 255, 180, 255]; // multi-line centered block
const AMBER: [u8; 4] = [255, 200, 120, 255]; // top-left anchor contrast
const WHITE: [u8; 4] = [235, 235, 245, 255]; // title
const GRAY: [u8; 4] = [170, 170, 185, 255]; // caption

struct CenteredTextDemo;

impl System for CenteredTextDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                    quit.quit();
                }
                return;
            }
        }

        // Logical screen size (DPR-independent — `DrawText`/`DebugDraw` use the same space).
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((960.0, 540.0));

        // Three guide columns: two genuinely off-center (the EW-001 trouble case) + the center as a
        // control. Pre-fix, only the center column would have looked right.
        let cols = [vw * 0.2, vw * 0.5, vw * 0.8];

        // Vertical guide lines through each guide x (screen space, top-left origin, y down).
        if let Some(dbg) = world.resource_mut::<DebugDraw>() {
            for &x in &cols {
                dbg.line(Vec2::new(x, 70.0), Vec2::new(x, vh - 60.0), GUIDE);
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            // Title (itself centered at the top — a centered label at the exact viewport center).
            tq.push(DrawText::centered(
                "EW-001 \u{2014} DrawText::centered centers on position.x   (Esc quits)",
                Vec2::new(vw * 0.5, 36.0),
                22.0,
                WHITE,
            ));

            // Row of single-line centered labels, one per guide column. Each reports its own x; its
            // shaped center lands exactly on the guide regardless of how off-center x is.
            for &x in &cols {
                tq.push(DrawText::centered(
                    format!("centered @ x = {x:.0}"),
                    Vec2::new(x, vh * 0.34),
                    24.0,
                    CYAN,
                ));
            }

            // Multi-line centered block on the middle guide: both lines center on the same x — proof
            // the fix keeps per-line `align = Center` instead of left-aligning a multi-line title.
            tq.push(DrawText::centered(
                "two lines, both\ncentered on the guide",
                Vec2::new(cols[1], vh * 0.55),
                24.0,
                GREEN,
            ));

            // Contrast: a default top-left-anchored label at the left guide. Its LEFT edge sits on
            // the guide, visibly NOT centered — the difference `centered` exists to remove.
            tq.push(DrawText::new(
                "<- DrawText::new (top-left anchor) starts at the guide",
                Vec2::new(cols[0], vh * 0.72),
                18.0,
                AMBER,
            ));

            tq.push(DrawText::centered(
                "Each cyan/green label's center sits on its white guide line for any x.",
                Vec2::new(vw * 0.5, vh - 36.0),
                16.0,
                GRAY,
            ));
        }
    }

    fn name(&self) -> &'static str {
        "CenteredTextDemo"
    }
}

fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — centered text (EW-001)".to_string(),
        width: 960,
        height: 540,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });
    app.world.insert_resource(FontData(DEJAVU.to_vec()));

    app.add_system(CenteredTextDemo);
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run();
}

/// WASM entry point — `examples/centered_text/web/index.html` calls this after `init()` (and on the
/// "Start" click; winit wants a user gesture before grabbing the canvas). Renders the same
/// `DrawText::centered` guide-line demo as native, so the EW-001 fix is eyeball-checkable in a browser.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_centered_text() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
