//! Embedded texture atlas — a whole sprite sheet baked into the binary via `include_bytes!`.
//!
//! [`App::load_atlas`] is path-based: it reads a sheet from disk, resolved against the
//! [asset root](engine::asset_path). That is right for a game shipping an `assets/` folder next to
//! its executable — but a jam entry, a single-file distribution or a wasm demo often wants its art
//! baked in at compile time. [`App::load_image_bytes`] already covered single images (see the
//! `embedded_image` example); [`App::load_atlas_bytes`] is the same door for a **gridded sheet**.
//!
//! Hand it a `&[u8]` you already hold — here from `include_bytes!` — a logical `key`, and the
//! `cols × rows` grid. The `key` is used **verbatim** as the atlas cache key, the handle path and
//! the renderer's texture key, so an [`AtlasSprite`] on the returned handle renders exactly like a
//! path-loaded atlas. Nothing is read from disk, so it works from any working directory — and on
//! wasm, where `include_bytes!` sidesteps the async fetch a path load needs.
//!
//! The sheet here is the 4×3, 64px-cell locomotion sheet the `blend_locomotion` demo loads *by
//! path*; this example renders all 12 of its tiles from bytes instead, which is what makes the grid
//! maths visible. It also feeds one deliberately **corrupt** byte slice, to show that a bad embed is
//! *reported* through [`App::asset_failures`] just like a missing file.
//!
//! Headless: `HEADLESS_SHOT=/tmp/embedded_atlas.png cargo run --example embedded_atlas`. The
//! headless run exits non-zero unless the sheet decoded to 256×192 under the verbatim key, the grid
//! tiles exactly like a path-loaded atlas, and the corrupt embed was reported — so it doubles as a
//! runnable acceptance test.
use engine::{
    App, AtlasSprite, Camera, Color, DrawText, ShouldQuit, System, TextQueue, Transform, Vec2,
    WindowConfig, World,
};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

const WIN_W: u32 = 820;
const WIN_H: u32 = 470;

/// The whole point: the sheet's pixels live in the binary, not in a file on disk. `include_bytes!`
/// resolves relative to THIS source file — a 256×192 PNG holding a 4×3 grid of 64px cells.
static SHEET_PNG: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/assets/blend_locomotion.png"
));
/// The logical key. It names no file; it is just the identity the atlas renders by.
const SHEET_KEY: &str = "embedded/locomotion";
const COLS: u32 = 4;
const ROWS: u32 = 3;

/// Not a valid PNG — to show that a bad embedded sheet is surfaced, not swallowed.
static CORRUPT_PNG: &[u8] = b"\x89PNG\r\n\x1a\n this is not really a sheet";
const CORRUPT_KEY: &str = "embedded/corrupt-sheet";

/// Displayed tile size and spacing, in world units.
const TILE: f32 = 72.0;
const GAP: f32 = 14.0;
/// Top-left corner of the tile grid. With the default camera (position zero, zoom 1)
/// `world_to_screen` is the identity, so world units here are top-left-origin screen pixels —
/// which is why the grid is laid out in absolute coordinates rather than centered on the origin.
const GRID_ORIGIN: Vec2 = Vec2::new(60.0, 110.0);

/// World-space center of the tile at `(col, row)` in the `COLS × ROWS` grid.
fn tile_center(col: u32, row: u32) -> Vec2 {
    let step = TILE + GAP;
    Vec2::new(
        GRID_ORIGIN.x + TILE * 0.5 + col as f32 * step,
        GRID_ORIGIN.y + TILE * 0.5 + row as f32 * step,
    )
}

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

        // Tile index labels are placed by projecting each tile's WORLD position to the screen, so
        // they follow the sprites rather than being hand-positioned twice.
        let camera = world.resource::<Camera>().cloned().unwrap_or_default();
        let label_spots: Vec<(u32, Vec2)> = (0..ROWS)
            .flat_map(|r| (0..COLS).map(move |c| (r * COLS + c, tile_center(c, r))))
            .map(|(index, world_pos)| {
                let screen = camera.world_to_screen(world_pos + Vec2::new(0.0, TILE * 0.5 + 4.0));
                (index, screen)
            })
            .collect();

        let failures = engine::asset_path::asset_failures();
        let cwd = std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".into());

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        let ink = Color::rgb(0.92, 0.9, 0.78);
        let dim = Color::rgb(0.55, 0.55, 0.5);
        let ok = Color::rgb(0.5, 0.85, 0.55);

        tq.push(DrawText::new(
            "Embedded atlas — this whole sprite sheet is baked into the binary",
            Vec2::new(24.0, 22.0),
            17.0,
            ink,
        ));
        tq.push(DrawText::new(
            format!(
                "load_atlas_bytes(\"{SHEET_KEY}\", {} bytes, {COLS}, {ROWS})  — no file read",
                SHEET_PNG.len()
            ),
            Vec2::new(24.0, 48.0),
            13.0,
            ok,
        ));
        tq.push(DrawText::new(
            format!("working dir:  {cwd}   (no assets/ needed at runtime)"),
            Vec2::new(24.0, 68.0),
            12.0,
            dim,
        ));

        // Every tile of the grid, labelled by its atlas index — the grid maths, made visible.
        for (index, pos) in label_spots {
            tq.push(DrawText::centered(
                format!("{index}"),
                pos,
                12.0,
                Color::rgb(0.75, 0.78, 0.85),
            ));
        }

        let panel_x = WIN_W as f32 - 250.0;
        tq.push(DrawText::new(
            "App::asset_failures()",
            Vec2::new(panel_x, 110.0),
            14.0,
            Color::rgb(0.95, 0.45, 0.45),
        ));
        if failures.is_empty() {
            tq.push(DrawText::new(
                "(none)",
                Vec2::new(panel_x, 134.0),
                12.0,
                dim,
            ));
        }
        for (i, f) in failures.iter().take(3).enumerate() {
            let y = 134.0 + i as f32 * 34.0;
            tq.push(DrawText::new(
                &f.path,
                Vec2::new(panel_x, y),
                12.0,
                Color::rgb(0.95, 0.6, 0.6),
            ));
            let mut why = f.error.clone();
            why.truncate(30);
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

/// Builds the demo — shared by the native and wasm entry points, so the browser runs exactly what
/// `cargo run` does. Returns the atlas handle too, for the native acceptance test to inspect.
fn build_app() -> (App, engine::Handle<engine::TextureAtlas>) {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Embedded atlas — include_bytes!".into(),
        width: WIN_W,
        height: WIN_H,
        ..Default::default()
    });
    app.world.insert_resource(Camera::default());

    // The real work: register a gridded sheet already in memory. No path, no file, no asset root.
    let sheet = app.load_atlas_bytes(SHEET_KEY, SHEET_PNG, COLS, ROWS);
    // And a bad embed, to prove failures are still loud.
    let _corrupt = app.load_atlas_bytes(CORRUPT_KEY, CORRUPT_PNG, 2, 2);

    // One entity per tile — if the grid maths were wrong, this would show it immediately.
    for row in 0..ROWS {
        for col in 0..COLS {
            let index = row * COLS + col;
            let e = app.world.spawn();
            app.world.add_component(
                e,
                Transform {
                    position: tile_center(col, row),
                    scale: Vec2::splat(TILE),
                    rotation: 0.0,
                    z: 0.0,
                },
            );
            app.world
                .add_component(e, AtlasSprite::new(sheet.clone(), index));
        }
    }

    app.add_system(Hud);

    (app, sheet)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let (mut app, sheet) = build_app();

    // The acceptance test is native-only (headless capture needs a GPU adapter), but the FEATURE is
    // cross-platform: this example also builds and RUNS on wasm32 (see `web/`), which is the
    // single-file build `load_atlas_bytes` exists for.
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        if run_acceptance_test(&mut app, &sheet, &out) {
            return;
        }
        std::process::exit(1);
    }

    app.run();
}

/// WASM entry point — `examples/embedded_atlas/web/index.html` calls this after `init()` (and on the
/// "Start" click; winit wants a user gesture before grabbing the canvas).
///
/// This is the demo that matters for the feature: the sheet is inside the `.wasm` module itself, so
/// the page renders all 12 tiles with **no image fetch at all** — the thing a path-loaded atlas
/// cannot do on the web without async plumbing.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_embedded_atlas() {
    let (app, _sheet) = build_app();
    app.run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}

/// Headless acceptance test — returns `true` when every claim this example makes holds.
///
/// Checks, in order: (1) the sheet decoded to its real 256×192 — a byte load that silently did
/// nothing would leave no image; (2) the atlas is keyed by the caller's key **verbatim** on both the
/// handle and the render side, the invariant that keeps a byte-sourced atlas from rendering white;
/// (3) the grid tiles exactly as a path-loaded atlas would; (4) a valid embed is not a failure;
/// (5) the corrupt embed IS reported — a bad `include_bytes!` is as invisible as a missing file if
/// it is swallowed.
#[cfg(not(target_arch = "wasm32"))]
fn run_acceptance_test(
    app: &mut App,
    sheet: &engine::Handle<engine::TextureAtlas>,
    out: &str,
) -> bool {
    let frames = std::env::var("HEADLESS_FRAMES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);
    app.save_screenshot_headless(frames, out)
        .expect("headless screenshot");

    let assets = app
        .world
        .resource::<engine::AssetServer>()
        .expect("AssetServer");
    let atlas = assets.get_atlas(sheet).expect("atlas registered");
    let dims = assets
        .get_image(&atlas.handle)
        .map(|img| (img.width, img.height));
    let texture_path = atlas.texture_path().to_string();
    let grid = (atlas.cols, atlas.rows);
    // A middle tile, where a row/column mix-up would show: index 5 of a 4×3 grid is col 1, row 1.
    let uv_mid = atlas.uv_rect(5);

    let failures = app.asset_failures();
    let reported = |k: &str| failures.iter().any(|f| f.path == k);

    let mut problems: Vec<String> = Vec::new();
    if dims != Some((256, 192)) {
        problems.push(format!("sheet should decode to 256x192, got {dims:?}"));
    }
    if texture_path != SHEET_KEY {
        problems.push(format!(
            "atlas texture key must be the caller's key verbatim ('{SHEET_KEY}'), got \
             '{texture_path}' — a diverging key renders the atlas white"
        ));
    }
    if grid != (COLS, ROWS) {
        problems.push(format!("grid should be {COLS}x{ROWS}, got {grid:?}"));
    }
    if uv_mid != engine::UvRect::from_grid(1, 1, COLS, ROWS) {
        problems.push(format!(
            "tile 5 of a {COLS}x{ROWS} grid must be col 1 row 1, got {uv_mid:?}"
        ));
    }
    if reported(SHEET_KEY) {
        problems.push(format!("a valid embed '{SHEET_KEY}' must not be a failure"));
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
        return false;
    }
    println!(
        "OK: 256x192 {COLS}x{ROWS} atlas decoded from include_bytes! with no external file; \
         keyed verbatim as '{SHEET_KEY}'; {} tiles rendered; corrupt embed reported.",
        COLS * ROWS
    );
    true
}
