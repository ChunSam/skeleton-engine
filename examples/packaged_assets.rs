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
//! - It also loads one deliberately **missing** texture and one deliberately **missing data
//!   table**, so you can see the other half of the fix: the failure is now *reported*
//!   (`App::asset_failures()`) instead of silently swallowed. That is the panel on the right — the
//!   engine names the file **and the roots it searched**.
//!
//! The data table is the case that is easiest to ship by accident. A texture that fails to load is
//! at least *visible* (a magenta quad); a data table that fails to load registers **empty**, and a
//! game reading it runs perfectly happily with no items, no enemies, no levels — a content bug
//! three screens away from its cause. Both now land in the same list.
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
/// A real data table, also loaded by a relative path — the game content half of the fix.
const REAL_TABLE: &str = "examples/games/stat_editor_game/items.ron";
/// Deliberately absent. This is the failure that used to be *invisible*: the table registers
/// empty and the game boots with no content, saying nothing.
const MISSING_TABLE: &str = "examples/assets/__deliberately_missing__.ron";

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
        // Read the row count out before borrowing the TextQueue mutably.
        let item_rows = world
            .resource::<engine::DataTableRegistry>()
            .and_then(|reg| reg.get("items"))
            .map_or(0, |t| t.rows.len());

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
        // A data table that loaded is proved by its rows: 0 rows is exactly what the silent
        // failure used to look like.
        tq.push(DrawText::new(
            format!("loaded:  items.ron  ({item_rows} rows)"),
            Vec2::new(28.0, 90.0),
            13.0,
            Color::rgb(0.5, 0.85, 0.55),
        ));

        tq.push(DrawText::new(
            "roots searched, in order:",
            Vec2::new(28.0, 124.0),
            13.0,
            dim,
        ));
        for (i, root) in roots.iter().take(5).enumerate() {
            tq.push(DrawText::new(
                format!("{}. {}", i + 1, root.display()),
                Vec2::new(40.0, 146.0 + i as f32 * 18.0),
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
        for (i, f) in failures.iter().take(4).enumerate() {
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

    // The game-content half. Both go through the same asset root; the missing one now reports
    // itself instead of quietly registering an empty table.
    app.load_data_table("items", REAL_TABLE);
    app.load_data_table("dungeons", MISSING_TABLE);

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

        // The acceptance test, in three parts. (1) The real texture and the real data table must
        // have resolved, no matter where we were launched from. (2) The real table must actually
        // carry rows — an empty table is precisely what the silent failure looked like. (3) The
        // missing table must be *reported*, which is the part that used to be invisible.
        let failures = app.asset_failures();
        let failed = |p: &str| failures.iter().any(|f| f.path == p);

        let item_rows = app
            .world
            .resource::<engine::DataTableRegistry>()
            .and_then(|reg| reg.get("items"))
            .map_or(0, |t| t.rows.len());

        let mut problems: Vec<String> = Vec::new();
        if failed(REAL_TEXTURE) {
            problems.push(format!("'{REAL_TEXTURE}' did not resolve"));
        }
        if failed(REAL_TABLE) {
            problems.push(format!("'{REAL_TABLE}' did not resolve"));
        }
        if item_rows == 0 {
            problems.push("the 'items' table loaded no rows".into());
        }
        if !failed(MISSING_TABLE) {
            problems.push(format!(
                "'{MISSING_TABLE}' is missing but was NOT reported — a failed data table must \
                 never be silent"
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
            "OK: texture + data table ({item_rows} rows) resolved; reported failures = {:?}",
            failures.iter().map(|f| &f.path).collect::<Vec<_>>()
        );
        return;
    }

    app.run();
}
