//! Asset roots — why a packaged build no longer renders a magenta screen.
//!
//! Every engine asset API is path-based, and a *relative* path used to be read straight through
//! `std::fs`, which resolves against the **process working directory**. A shipped executable
//! therefore only worked when launched from one specific directory: double-clicked from a file
//! manager, or run as `cd /elsewhere && game.exe`, every texture load failed, the renderer
//! substituted its magenta 1×1 fallback, and the window turned solid magenta with only a `warn!`
//! to explain it.
//!
//! The engine now resolves relative paths against an **asset root it determines itself** — a macOS
//! bundle's `Contents/Resources`, then the executable's directory and its ancestors, then the
//! working directory (see [`engine::asset_path`]). This example proves it:
//!
//! - It loads `examples/assets/…` by a **relative** path. If the sprite below renders (rather than
//!   magenta), resolution worked — **run this binary from any directory and it still does**:
//!   `cd / && /path/to/target/debug/examples/packaged_assets`
//! - It also loads one deliberately **missing** texture, so you can see the other half of the fix:
//!   the failure is now *reported* (`App::asset_failures()`) instead of silently swallowed. That
//!   is the panel on the right — the engine names the file **and the roots it searched**.
//!
//! `App::set_strict_assets(true)` turns a missing asset into a panic at the load instead.
//!
//! Headless: `HEADLESS_SHOT=/tmp/assets.png cargo run --example packaged_assets`.
//! In headless mode the example exits non-zero if the *real* texture failed to resolve, so
//! `scripts/packaged_assets_smoke.sh` can run it from a foreign working directory as a real test.
use engine::{
    App, Camera, Color, DrawText, ShouldQuit, Sprite, System, TextQueue, Transform, Vec2,
    WindowConfig, World,
};

const WIN_W: u32 = 900;
const WIN_H: u32 = 460;

/// Loaded by a RELATIVE path — the whole point. An absolute path would sidestep the bug.
const REAL_TEXTURE: &str = "examples/assets/hex_tiles.png";
/// Deliberately absent, to demonstrate that a failed load is now surfaced rather than swallowed.
const MISSING_TEXTURE: &str = "examples/assets/__deliberately_missing__.png";

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
        let roots = engine::asset_path::candidate_roots();
        let failures = engine::asset_path::asset_failures();

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        let ink = Color::rgb(0.92, 0.9, 0.78);
        let dim = Color::rgb(0.55, 0.55, 0.5);

        tq.push(DrawText::new(
            "Asset roots — this sprite loaded from a RELATIVE path, whatever the working directory",
            Vec2::new(28.0, 24.0),
            17.0,
            ink,
        ));
        tq.push(DrawText::new(
            format!("working dir:  {cwd}"),
            Vec2::new(28.0, 52.0),
            13.0,
            dim,
        ));
        tq.push(DrawText::new(
            format!("loaded:  {REAL_TEXTURE}"),
            Vec2::new(28.0, 72.0),
            13.0,
            Color::rgb(0.5, 0.85, 0.55),
        ));

        tq.push(DrawText::new(
            "roots searched, in order:",
            Vec2::new(28.0, 106.0),
            13.0,
            dim,
        ));
        for (i, root) in roots.iter().take(5).enumerate() {
            tq.push(DrawText::new(
                format!("{}. {}", i + 1, root.display()),
                Vec2::new(40.0, 128.0 + i as f32 * 18.0),
                12.0,
                dim,
            ));
        }

        // The other half of the fix: a failed load is reported, not swallowed.
        let panel_x = WIN_W as f32 * 0.52;
        tq.push(DrawText::new(
            "App::asset_failures()",
            Vec2::new(panel_x, 106.0),
            14.0,
            Color::rgb(0.95, 0.45, 0.45),
        ));
        if failures.is_empty() {
            tq.push(DrawText::new(
                "(none)",
                Vec2::new(panel_x, 130.0),
                12.0,
                dim,
            ));
        }
        for (i, f) in failures.iter().take(3).enumerate() {
            let y = 130.0 + i as f32 * 40.0;
            tq.push(DrawText::new(
                &f.path,
                Vec2::new(panel_x, y),
                12.0,
                Color::rgb(0.95, 0.6, 0.6),
            ));
            // Truncated: the message carries every root the engine searched, which is long by design.
            let mut why = f.error.clone();
            why.truncate(52);
            tq.push(DrawText::new(why, Vec2::new(panel_x, y + 17.0), 11.0, dim));
        }

        tq.push(DrawText::new(
            "Esc — quit",
            Vec2::new(28.0, WIN_H as f32 - 28.0),
            13.0,
            dim,
        ));
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Packaged assets — asset roots + loud failures".into(),
        width: WIN_W,
        height: WIN_H,
        ..Default::default()
    });
    app.world.insert_resource(Camera::default());

    let real = app.load_image(REAL_TEXTURE);
    let _missing = app.load_image(MISSING_TEXTURE);

    let sprite = app.world.spawn();
    app.world.add_component(
        sprite,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.22, WIN_H as f32 * 0.62),
            scale: Vec2::splat(150.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(
        sprite,
        Sprite::textured_with_handle(REAL_TEXTURE, Some(real)),
    );

    app.add_system(Hud);

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");

        // The acceptance test: the REAL texture must have resolved, no matter where we were
        // launched from. (The deliberately-missing one is expected to fail and is ignored.)
        let real_failed = app.asset_failures().iter().any(|f| f.path == REAL_TEXTURE);
        if real_failed {
            eprintln!(
                "FAIL: '{REAL_TEXTURE}' did not resolve from working dir {:?}",
                std::env::current_dir()
            );
            std::process::exit(1);
        }
        println!(
            "OK: '{REAL_TEXTURE}' resolved; failures = {:?}",
            app.asset_failures()
                .iter()
                .map(|f| &f.path)
                .collect::<Vec<_>>()
        );
        return;
    }

    app.run();
}
