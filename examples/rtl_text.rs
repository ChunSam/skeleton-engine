//! RTL text + multi-font demo.
//!
//! Loads **DejaVu Sans** (Latin) as [`FontData`] and the bundled **Noto Sans Hebrew** (OFL) as
//! [`ExtraFonts`], then renders mixed Latin + Hebrew (right-to-left) text. Bidirectional shaping is
//! done by cosmic-text (`Shaping::Advanced`) — this example demonstrates the two pieces the engine
//! adds on top:
//!
//! * **Multi-font coverage**: a single `DrawText` mixing Latin + Hebrew shapes both scripts because
//!   cosmic-text falls back across every loaded font (DejaVu has no Hebrew; Noto Hebrew supplies it).
//! * **RTL-aware alignment**: [`TextAlign::Auto`] right-aligns the Hebrew line automatically (its
//!   resolved direction is RTL), while [`TextAlign::Left`] forces it left for contrast.
//!
//! Run: `cargo run --example rtl_text` (Esc quits).

use engine::{
    App, DrawText, ExtraFonts, FontData, InputState, KeyCode, ShouldQuit, System, TextAlign,
    TextQueue, Vec2, WindowConfig, World,
};

const DEJAVU: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/DejaVuSans.ttf"
));
const NOTO_HEBREW: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/NotoSansHebrew-Regular.ttf"
));

/// "Hello world" in Hebrew (RTL).
const HEBREW: &str = "שלום עולם";
/// Layout width the lines align within (so RTL/auto alignment is visible).
const BOX_W: f32 = 600.0;

struct TextSystem;

impl System for TextSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                    quit.quit();
                }
                return;
            }
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        // Each line aligns within a BOX_W-wide box anchored at x = 40.
        let mut line = |y: f32, text: &str, align: TextAlign| {
            let mut d = DrawText::new(text, Vec2::new(40.0, y), 30.0, [235u8, 235, 245, 255]);
            d.align = align;
            d.bounds = Some(Vec2::new(BOX_W, 44.0));
            tq.push(d);
        };

        line(
            30.0,
            "RTL text + multi-font demo  (Esc quits)",
            TextAlign::Left,
        );
        line(100.0, "Latin LTR, left-aligned", TextAlign::Left);
        // Hebrew with Auto → right-aligned automatically (resolved RTL direction).
        line(150.0, HEBREW, TextAlign::Auto);
        // Same Hebrew forced left, for contrast with the Auto line above.
        line(200.0, HEBREW, TextAlign::Left);
        // Mixed bidi in one run: Latin + Hebrew shape together via font fallback.
        line(250.0, "Hello שלום world", TextAlign::Auto);
        // End alignment: hugs the right edge for this (LTR-resolved) line.
        line(300.0, "End-aligned Latin", TextAlign::End);
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine RTL text".to_string(),
        width: 960,
        height: 540,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });
    // Latin default + Hebrew fallback font.
    app.world.insert_resource(FontData(DEJAVU.to_vec()));
    app.world
        .insert_resource(ExtraFonts(vec![NOTO_HEBREW.to_vec()]));

    app.add_system(TextSystem);
    app.run();
}
