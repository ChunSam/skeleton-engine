//! tilemap_layers — stack two orthographic tilemaps with `Tilemap::with_z`.
//!
//! `cell_z()` gives every orthographic/hexagonal tile the same depth, the new per-map
//! `Tilemap::z` (default `-1.0`). Two tilemaps with different `z` therefore layer: a background
//! floor under a foreground decoration map.
//!
//! To prove it is the `z` value — not spawn order — that decides what draws on top, the foreground
//! map is **spawned first** but given the higher `z`, so it still renders over the later-spawned
//! background. (The renderer sorts sprites by `z`, so the floor at `z = -2` stays behind the
//! decorations at `z = -1` regardless of spawn order.)
//!
//! Run from the repo root:  `cargo run --example tilemap_layers`
//! ESC = quit.

use engine::{
    App, Color, DrawText, InputState, KeyCode, ShouldQuit, System, TextQueue, Tilemap,
    TilemapAtlas, TilemapSystem, Vec2, WindowConfig, World,
};

const COLS: usize = 10;
const ROWS: usize = 7;
const TILE: f32 = 56.0;
const FRAME_PX: u32 = 64;

const FLOOR: u32 = 1; // atlas tile 0 — green floor
const DECO: u32 = 2; // atlas tile 1 — orange decoration

/// Writes a 2-tile atlas (green floor, orange decoration) to `target/` and returns the path.
fn write_atlas_png() -> String {
    let path = "target/tilemap_layers_atlas.png";
    let mut img = image::RgbaImage::new(FRAME_PX * 2, FRAME_PX);

    // (col, base, highlight) — an inset highlight makes each tile's edges readable.
    let tiles: [(u32, [u8; 4], [u8; 4]); 2] = [
        (0, [0x2c, 0x6e, 0x3a, 0xff], [0x46, 0x96, 0x54, 0xff]), // green floor
        (1, [0xc8, 0x6e, 0x1e, 0xff], [0xf0, 0x9a, 0x36, 0xff]), // orange decoration
    ];
    for (col, base, hi) in tiles {
        for py in 0..FRAME_PX {
            for px in 0..FRAME_PX {
                let inset = px > 3 && px < FRAME_PX - 4 && py > 3 && py < FRAME_PX - 4;
                let rgba = if inset { hi } else { base };
                img.put_pixel(col * FRAME_PX + px, py, image::Rgba(rgba));
            }
        }
    }
    img.save(path)
        .expect("failed to write atlas PNG to target/");
    path.to_string()
}

/// ESC-to-quit + a HUD explaining the layering.
struct Hud;

impl System for Hud {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(q) = world.resource_mut::<ShouldQuit>() {
                    q.quit();
                }
            }
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "tilemap_layers — two orthographic tilemaps stacked via Tilemap::with_z",
                Vec2::new(16.0, 14.0),
                18.0,
                Color::rgb(0.92, 0.92, 0.96),
            ));
            tq.push(DrawText::new(
                "Background floor: z = -2.0   ·   Foreground decorations: z = -1.0 (drawn on top)",
                Vec2::new(16.0, 40.0),
                15.0,
                Color::rgb(0.95, 0.74, 0.42),
            ));
            tq.push(DrawText::new(
                "Foreground is SPAWNED FIRST but has the higher z, so z — not spawn order — wins.  ESC: quit",
                Vec2::new(16.0, 60.0),
                14.0,
                Color::rgb(0.6, 0.85, 1.0),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "tilemap_layers_hud"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "tilemap_layers — Tilemap::with_z".to_string(),
        width: 760,
        height: 560,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let atlas_path = write_atlas_png();
    app.load_image(&atlas_path);

    let map_w = COLS as f32 * TILE;
    let map_h = ROWS as f32 * TILE;
    let origin = Vec2::new((760.0 - map_w) * 0.5, (560.0 - map_h) * 0.5 + 30.0);

    // Foreground decoration map: a sparse cross-and-border pattern (0 = empty → floor shows
    // through). Spawned FIRST, but z = -1.0 keeps it on top.
    let mut deco = vec![vec![0u32; COLS]; ROWS];
    for (r, row) in deco.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            let border = r == 0 || c == 0 || r == ROWS - 1 || c == COLS - 1;
            let cross = r == ROWS / 2 || c == COLS / 2;
            if border || cross {
                *cell = DECO;
            }
        }
    }
    let foreground =
        Tilemap::new(TilemapAtlas::new(&atlas_path, 2, 1), deco, TILE, origin).with_z(-1.0);
    let fg = app.world.spawn();
    app.world.add_component(fg, foreground);

    // Background floor map: solid floor, spawned SECOND, z = -2.0 → stays behind the decorations.
    let floor_tiles = vec![vec![FLOOR; COLS]; ROWS];
    let background = Tilemap::new(
        TilemapAtlas::new(&atlas_path, 2, 1),
        floor_tiles,
        TILE,
        origin,
    )
    .with_z(-2.0);
    let bg = app.world.spawn();
    app.world.add_component(bg, background);

    app.add_system(TilemapSystem::new());
    app.add_system(Hud);
    app.run();
}
