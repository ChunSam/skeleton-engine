//! Embedded images — a sprite that renders from `include_bytes!`, with no external asset file.
//!
//! Every other image API in the engine is *path*-based: `App::load_image("assets/hero.png")` reads
//! a file, resolved against the [asset root](engine::asset_path). That is right for a game that
//! ships an `assets/` folder next to its executable — but a small game, a jam entry, or a wasm demo
//! often wants to ship as **one file** with its art baked in at compile time.
//!
//! [`App::load_image_bytes`] is that path: hand it a `&[u8]` you already hold — here from
//! `include_bytes!` — and a logical `key`, and it decodes the image immediately and registers it
//! under `key`. The `key` is used **verbatim** as both the texture-cache key and the handle path,
//! so a [`Sprite::with_handle`] (or a [`Sprite::textured(key)`](engine::Sprite::textured) naming
//! the same string) renders it exactly like a path-loaded image. Nothing is read from disk, so it
//! works from any working directory — and on wasm, where `include_bytes!` sidesteps the async fetch
//! that a path load needs.
//!
//! This example embeds a real 32×32 PNG and renders it. It also feeds one deliberately **corrupt**
//! byte slice, to show that a bad embed is *reported* through [`App::asset_failures`] just like a
//! missing file — not silently turned into a blank sprite.
//!
//! Headless: `HEADLESS_SHOT=/tmp/embedded.png cargo run --example embedded_image`. In headless mode
//! the example exits non-zero unless the embedded image decoded to 32×32 (and was not reported as a
//! failure) *and* the corrupt embed was reported — so it doubles as a runnable acceptance test.
use engine::{
    App, AssetServer, Camera, Color, DrawText, ShouldQuit, Sprite, System, TextQueue, Transform,
    Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

/// The whole point: the pixels live in the binary, not in a file on disk. `include_bytes!` resolves
/// relative to THIS source file, so this is `examples/assets/player.png` — a 32×32 PNG.
static PLAYER_PNG: &[u8] = include_bytes!("assets/player.png");
/// The logical key. It names no file; it is just the identity the sprite renders by.
const PLAYER_KEY: &str = "embedded/player";

/// Not a valid PNG — to show that a bad embed is surfaced, not swallowed.
static CORRUPT_PNG: &[u8] = b"\x89PNG\r\n\x1a\n this is not really a png";
const CORRUPT_KEY: &str = "embedded/corrupt";

struct Hud;

impl System for Hud {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<engine::InputState>()
            .is_some_and(|i| i.just_pressed(engine::KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".into());
        let failures = engine::asset_path::asset_failures();

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        let ink = Color::rgb(0.92, 0.9, 0.78);
        let dim = Color::rgb(0.55, 0.55, 0.5);
        let ok = Color::rgb(0.5, 0.85, 0.55);

        tq.push(DrawText::new(
            "Embedded image — this sprite is baked into the binary via include_bytes!",
            Vec2::new(24.0, 22.0),
            17.0,
            ink,
        ));
        tq.push(DrawText::new(
            format!("working dir:  {cwd}   (no assets/ needed)"),
            Vec2::new(24.0, 50.0),
            13.0,
            dim,
        ));
        tq.push(DrawText::new(
            format!(
                "load_image_bytes(\"{PLAYER_KEY}\", {} bytes)",
                PLAYER_PNG.len()
            ),
            Vec2::new(24.0, 70.0),
            13.0,
            ok,
        ));

        let panel_x = WIN_W as f32 * 0.5;
        tq.push(DrawText::new(
            "App::asset_failures()",
            Vec2::new(panel_x, 96.0),
            14.0,
            Color::rgb(0.95, 0.45, 0.45),
        ));
        if failures.is_empty() {
            tq.push(DrawText::new(
                "(none)",
                Vec2::new(panel_x, 120.0),
                12.0,
                dim,
            ));
        }
        for (i, f) in failures.iter().take(3).enumerate() {
            let y = 120.0 + i as f32 * 34.0;
            tq.push(DrawText::new(
                &f.path,
                Vec2::new(panel_x, y),
                12.0,
                Color::rgb(0.95, 0.6, 0.6),
            ));
            let mut why = f.error.clone();
            why.truncate(48);
            tq.push(DrawText::new(why, Vec2::new(panel_x, y + 15.0), 11.0, dim));
        }

        tq.push(DrawText::new(
            "Esc — quit",
            Vec2::new(24.0, WIN_H as f32 - 26.0),
            13.0,
            dim,
        ));
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Embedded image — include_bytes!".into(),
        width: WIN_W,
        height: WIN_H,
        ..Default::default()
    });
    app.world.insert_resource(Camera::default());

    // The real work: decode an image already in memory. No path, no file, no asset root.
    let hero = app.load_image_bytes(PLAYER_KEY, PLAYER_PNG);
    // And a bad embed, to prove failures are still loud.
    let _corrupt = app.load_image_bytes(CORRUPT_KEY, CORRUPT_PNG);

    let sprite = app.world.spawn();
    app.world.add_component(
        sprite,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.28, WIN_H as f32 * 0.66),
            scale: Vec2::splat(160.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    // with_handle renders by the handle's key — the same verbatim string we loaded under.
    app.world
        .add_component(sprite, Sprite::with_handle(hero.clone()));

    app.add_system(Hud);

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");

        // The acceptance test, in three parts. (1) The embedded image must have decoded to its real
        // 32×32 size — a byte load that silently did nothing would leave no image. (2) A valid embed
        // must NOT be an asset failure. (3) The corrupt embed MUST be reported — a bad `include_bytes!`
        // is exactly as invisible as a missing file if it is swallowed.
        let dims = app
            .world
            .resource::<AssetServer>()
            .and_then(|a| a.get_image(&hero).map(|img| (img.width, img.height)));
        let failures = app.asset_failures();
        let reported = |k: &str| failures.iter().any(|f| f.path == k);

        let mut problems: Vec<String> = Vec::new();
        match dims {
            Some((32, 32)) => {}
            other => problems.push(format!(
                "embedded image should decode to 32x32, got {other:?}"
            )),
        }
        if reported(PLAYER_KEY) {
            problems.push(format!(
                "a valid embed '{PLAYER_KEY}' must not be a failure"
            ));
        }
        if !reported(CORRUPT_KEY) {
            problems.push(format!(
                "corrupt embed '{CORRUPT_KEY}' is invalid but was NOT reported — a bad \
                 include_bytes! must never be silent"
            ));
        }

        if !problems.is_empty() {
            eprintln!(
                "FAIL (working dir {:?}):\n  - {}",
                std::env::current_dir(),
                problems.join("\n  - ")
            );
            std::process::exit(1);
        }
        println!(
            "OK: embedded 32x32 image decoded from include_bytes! with no external file; \
             corrupt embed reported. failures = {:?}",
            failures.iter().map(|f| &f.path).collect::<Vec<_>>()
        );
        return;
    }

    app.run();
}
