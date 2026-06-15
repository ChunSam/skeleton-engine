//! Tile-painting editor acceptance test.
//!
//! Spawns a blank 20×15 tilemap with a 4-tile colour atlas (generated at runtime),
//! then lets the user open the F2 docked editor and paint tiles directly on the map.
//!
//! ## How to paint
//! 1. Press **F2** to open the docked editor.
//! 2. Click the tilemap entity in the left-hand entity list.
//! 3. Open the **Tile Paint** panel, enable **Paint mode**.
//! 4. **Left-click** to paint, **Right-click** to erase.
//! 5. Keys **1–4** pick a tile colour; **0** picks erase.
//! 6. **Ctrl+Z** undoes the last painted stroke.
//!
//! ## Atlas layout (2×2, 32×32 px per tile)
//! ```
//!  col 0      col 1
//!  green (1)  brown (2)    ← row 0
//!  grey  (3)  blue  (4)    ← row 1
//! ```
//! Tile value 0 = empty, 1–4 = the four coloured tiles (atlas index = value − 1).
//!
//! Engine convention: camera position (0,0) maps to the viewport's TOP-LEFT (Y down),
//! so all world coordinates are positive.

use engine::{
    App, Camera, DrawText, System, Tag, TextQueue, Tilemap, TilemapAtlas, TilemapSystem,
    WindowConfig, World,
};
use glam::Vec2;
use image::{Rgba, RgbaImage};

// ── Layout constants ──────────────────────────────────────────────────────────

const WIN_W: u32 = 900;
const WIN_H: u32 = 640;

const TILE: f32 = 32.0;
const COLS: usize = 20;
const ROWS: usize = 15;

/// Top-left origin of the tilemap — centred in the window (positive world coords).
const ORIGIN: Vec2 = Vec2::new(
    (WIN_W as f32 - COLS as f32 * TILE) * 0.5,
    (WIN_H as f32 - ROWS as f32 * TILE) * 0.5,
);

// ── Runtime atlas generation ──────────────────────────────────────────────────

/// Generates a 2×2 tile atlas (64×64 px) with four solid-colour tiles and writes
/// it to a unique temp-file, returning the path as a `String`.
///
/// Tile layout (atlas indices):
///  0 = green   1 = brown
///  2 = grey    3 = blue
fn generate_atlas_png() -> String {
    let tile_px = 32u32;
    let atlas_w = tile_px * 2; // 2 columns
    let atlas_h = tile_px * 2; // 2 rows

    let colors: [[u8; 4]; 4] = [
        [0x4c, 0xaf, 0x50, 0xff], // green
        [0x79, 0x55, 0x48, 0xff], // brown
        [0x90, 0x9e, 0xa0, 0xff], // grey
        [0x42, 0x7a, 0xc1, 0xff], // blue
    ];

    let mut img = RgbaImage::new(atlas_w, atlas_h);
    for (idx, color) in colors.iter().enumerate() {
        let atlas_col = (idx % 2) as u32;
        let atlas_row = (idx / 2) as u32;
        for dy in 0..tile_px {
            for dx in 0..tile_px {
                img.put_pixel(
                    atlas_col * tile_px + dx,
                    atlas_row * tile_px + dy,
                    Rgba(*color),
                );
            }
        }
    }

    let path = std::env::temp_dir()
        .join(format!("tile_paint_atlas_{}.png", std::process::id()))
        .to_string_lossy()
        .into_owned();
    img.save(&path).expect("write tile_paint atlas PNG");
    path
}

// ── HUD ───────────────────────────────────────────────────────────────────────

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "F2: editor  |  select Tilemap entity  |  Tile Paint > Paint mode",
            Vec2::new(12.0, 10.0),
            15.0,
            [230, 230, 240, 220],
        ));
        tq.push(DrawText::new(
            "L-click: paint   R-click: erase   1-4: pick tile   0: erase   Ctrl+Z: undo",
            Vec2::new(12.0, WIN_H as f32 - 26.0),
            15.0,
            [180, 200, 220, 200],
        ));
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let atlas_path = generate_atlas_png();

    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "Tile Paint — skeleton-engine editor demo".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.08, 0.09, 0.12, 1.0],
    });

    // Camera at (0,0) shows world [0, WIN_W] × [0, WIN_H] — the whole grid is visible.
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // Register the atlas so TilemapSystem can resolve tile sprites by texture path.
    // 2 columns × 2 rows in the atlas.
    app.load_atlas(&atlas_path, 2, 2);

    // Blank 20×15 tilemap: all cells start at 0 (empty).
    let tiles: Vec<Vec<u32>> = vec![vec![0u32; COLS]; ROWS];
    let tilemap = Tilemap::new(TilemapAtlas::new(&atlas_path, 2, 2), tiles, TILE, ORIGIN);

    let tilemap_entity = app.world.spawn();
    app.world.add_component(tilemap_entity, tilemap);
    // Tag so the entity is easy to spot and select in the editor's entity list.
    app.world
        .add_component(tilemap_entity, Tag("Tilemap".into()));

    // TilemapSystem: reacts to cell mutations and manages tile-sprite entities.
    app.add_system(TilemapSystem::new());

    // HUD: instructions overlay rendered each frame via DrawText (screen-space, Y down).
    app.add_system(HudSystem);

    app.run();
}
