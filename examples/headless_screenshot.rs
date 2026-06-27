//! `App::save_screenshot_headless` — render a frame to a PNG with **no window and no display**.
//!
//! Unlike every other example, this one never opens a window: it builds a scene, renders it
//! headlessly into an offscreen GPU texture, reads the pixels back, and writes a PNG — then exits.
//! It works with the monitor off/asleep/locked and on a machine with no display attached, so it
//! can pixel-verify a render in CI or while you're away. The same real render path runs, so the
//! image matches what the windowed app would draw.
//!
//! Run: `cargo run --example headless_screenshot [out.png]` (default `/tmp/headless_screenshot.png`).
use engine::{App, DrawText, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World};

const W: u32 = 480;
const H: u32 = 320;
/// Frames to advance before capturing (lets systems/animation settle).
const FRAMES: u32 = 30;

/// Draws a couple of labels each frame so the screenshot exercises the text path too.
struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "headless screenshot",
                Vec2::new(16.0, 14.0),
                22.0,
                [235, 225, 200, 255],
            ));
            tq.push(DrawText::centered(
                "rendered with the monitor off",
                Vec2::new(W as f32 / 2.0, H as f32 - 28.0),
                15.0,
                [160, 220, 255, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

fn sprite(app: &mut App, pos: Vec2, size: Vec2, color: engine::Sprite) {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: pos,
            scale: size,
            ..Default::default()
        },
    );
    app.world.add_component(e, color);
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/headless_screenshot.png".to_string());

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "headless".into(),
        width: W,
        height: H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    // A small scene: three overlapping colored quads.
    sprite(
        &mut app,
        Vec2::new(170.0, 180.0),
        Vec2::splat(120.0),
        Sprite::colored(0.30, 0.55, 0.90),
    );
    sprite(
        &mut app,
        Vec2::new(240.0, 150.0),
        Vec2::splat(120.0),
        Sprite::colored(0.95, 0.60, 0.25),
    );
    sprite(
        &mut app,
        Vec2::new(310.0, 190.0),
        Vec2::splat(90.0),
        Sprite::colored(0.45, 0.85, 0.55),
    );
    app.add_system(HudSystem);

    // Render headlessly and read the pixels back (no window is ever created).
    let (w, h, pixels) = app.screenshot_headless(FRAMES);

    // Self-check: a rendered frame differs from a blank one. Count pixels that differ noticeably
    // from the top-left (background) pixel — the sprites + text.
    let bg = [pixels[0], pixels[1], pixels[2]];
    let non_bg = pixels
        .chunks_exact(4)
        .filter(|p| {
            (p[0] as i32 - bg[0] as i32).abs() > 12
                || (p[1] as i32 - bg[1] as i32).abs() > 12
                || (p[2] as i32 - bg[2] as i32).abs() > 12
        })
        .count();

    image::RgbaImage::from_raw(w, h, pixels)
        .expect("readback byte count")
        .save(&path)
        .expect("save png");

    println!("HEADLESS_SCREENSHOT: wrote {w}x{h} PNG to {path}");
    println!("non-background pixels: {non_bg}");
    if non_bg > 2000 {
        println!("HEADLESS_SCREENSHOT: PASS (non-blank frame)");
    } else {
        println!("HEADLESS_SCREENSHOT: FAIL (frame looks blank)");
        std::process::exit(1);
    }
}
