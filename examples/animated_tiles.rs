//! Animated tiles demo.
//!
//! Demonstrates per-tile-value animation via [`TileAnimationSet`] + [`AnimatedTileSystem`].
//!
//! A small tilemap is shown with three regions:
//!   - **Water** (value 1) — four blue frames that cycle, simulating ripples.
//!   - **Lava**  (value 2) — three orange/red frames that glow and pulse.
//!   - **Ground** (value 3) — static grey tile (no animation registered).
//!
//! The atlas texture is generated procedurally at startup and saved to a temporary file
//! before loading, so no external asset is required.  The 8-frame horizontal strip
//! (each frame 32×32 px) is written to `target/animated_tiles_atlas.png`.
//!
//! Press **Esc** to quit.

use engine::ecs::System;
use engine::{
    AnimatedTileSystem, App, DrawText, InputState, KeyCode, ShouldQuit, TextQueue, TileAnimation,
    TileAnimationSet, Tilemap, TilemapAtlas, TilemapSystem, Vec2, WindowConfig, World,
};

const WIN_W: f32 = 640.0;
const WIN_H: f32 = 480.0;

/// Number of atlas columns (frames): 4 water + 3 lava + 1 ground = 8.
const ATLAS_COLS: u32 = 8;
/// Single-row atlas.
const ATLAS_ROWS: u32 = 1;
/// Pixel size of each tile frame in the atlas.
const FRAME_PX: u32 = 32;

// ─── Procedural atlas generation ──────────────────────────────────────────────

/// Generates a flat RGBA8 image (`ATLAS_COLS × FRAME_PX` wide, `FRAME_PX` tall).
///
/// Column layout:
///   0-3 : water frames (four shades of blue, simulating ripple)
///   4-6 : lava  frames (orange → red glow)
///   7   : ground tile  (dark grey stone, static)
fn generate_atlas_rgba8() -> image::RgbaImage {
    let w = ATLAS_COLS * FRAME_PX;
    let h = ATLAS_ROWS * FRAME_PX;
    let mut img = image::RgbaImage::new(w, h);

    /// Fill one atlas frame with a base colour and an inner highlight region.
    fn fill_frame(
        img: &mut image::RgbaImage,
        col_idx: u32,
        base: [u8; 4],
        highlight: [u8; 4],
        frame_px: u32,
    ) {
        for py in 0..frame_px {
            for px in 0..frame_px {
                let x = col_idx * frame_px + px;
                let y = py;
                // Inner 4-pixel inset for a subtle depth/shine effect.
                let is_hi = px > 3 && px < frame_px - 4 && py > 3 && py < frame_px - 4;
                img.put_pixel(x, y, image::Rgba(if is_hi { highlight } else { base }));
            }
        }
    }

    // Water frames 0-3: dark blue → lighter blue → bright cyan → back to medium.
    let water: [([u8; 4], [u8; 4]); 4] = [
        ([0x10, 0x30, 0xa0, 0xff], [0x30, 0x60, 0xd0, 0xff]),
        ([0x18, 0x48, 0xb8, 0xff], [0x40, 0x80, 0xe0, 0xff]),
        ([0x28, 0x68, 0xcc, 0xff], [0x60, 0xa0, 0xf0, 0xff]),
        ([0x1c, 0x58, 0xbc, 0xff], [0x4c, 0x90, 0xe4, 0xff]),
    ];
    for (i, (base, hi)) in water.iter().enumerate() {
        fill_frame(&mut img, i as u32, *base, *hi, FRAME_PX);
    }

    // Lava frames 4-6: deep orange → bright orange-red → dark crimson.
    let lava: [([u8; 4], [u8; 4]); 3] = [
        ([0xc0, 0x48, 0x08, 0xff], [0xff, 0x80, 0x20, 0xff]),
        ([0xe0, 0x60, 0x10, 0xff], [0xff, 0xa0, 0x30, 0xff]),
        ([0xa0, 0x20, 0x20, 0xff], [0xe0, 0x50, 0x10, 0xff]),
    ];
    for (i, (base, hi)) in lava.iter().enumerate() {
        fill_frame(&mut img, 4 + i as u32, *base, *hi, FRAME_PX);
    }

    // Ground frame 7: dark stone grey.
    fill_frame(
        &mut img,
        7,
        [0x55, 0x58, 0x5c, 0xff],
        [0x6e, 0x72, 0x78, 0xff],
        FRAME_PX,
    );

    img
}

/// Writes the atlas to `target/animated_tiles_atlas.png` and returns the path.
///
/// Using `target/` keeps generated assets out of the source tree; the directory
/// is guaranteed to exist once Cargo has built anything.
fn write_atlas_png() -> String {
    let path = "target/animated_tiles_atlas.png";
    let img = generate_atlas_rgba8();
    img.save(path)
        .expect("failed to write atlas PNG to target/");
    path.to_string()
}

// ─── HUD system ───────────────────────────────────────────────────────────────

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "Animated tiles demo — Esc to quit",
            Vec2::new(16.0, 14.0),
            18.0,
            [240, 240, 255, 230],
        ));
        tq.push(DrawText::new(
            "Blue  = water  (4 frames @ 0.18 s)",
            Vec2::new(16.0, 42.0),
            14.0,
            [100, 160, 240, 220],
        ));
        tq.push(DrawText::new(
            "Orange = lava  (3 frames @ 0.22 s)",
            Vec2::new(16.0, 60.0),
            14.0,
            [255, 140, 50, 220],
        ));
        tq.push(DrawText::new(
            "Grey  = ground (static — no animation)",
            Vec2::new(16.0, 78.0),
            14.0,
            [160, 165, 170, 220],
        ));
    }
}

// ─── Quit system ──────────────────────────────────────────────────────────────

struct QuitSystem;

impl System for QuitSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let esc = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false);
        if esc {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.quit();
            }
        }
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "animated_tiles — skeleton-engine".to_string(),
        width: WIN_W as u32,
        height: WIN_H as u32,
        clear_color: [0.08, 0.08, 0.12, 1.0],
    });

    // Generate the atlas PNG and load it via App::load_image.
    let atlas_path = write_atlas_png();
    app.load_image(&atlas_path);

    // ── Build the tilemap ──────────────────────────────────────────────────────
    // Value encoding (Tilemap convention): 0=empty, 1+=tile (atlas ID = value-1).
    //   value 1 → first animation (water frames 0-3)
    //   value 2 → second animation (lava  frames 4-6)
    //   value 3 → static (ground, atlas ID 7 = value-1 = 2... wait, value 3 → atlas col 2?)
    //
    // To use a specific atlas column for a static tile we need value = atlas_col + 1.
    // Ground is atlas column 7, so ground value = 8.  Assign a memorable constant:
    let tile_size = 48.0f32;
    let map_cols = 10usize;
    let map_rows = 6usize;

    // Layout: top 4 rows alternate water/lava, bottom 2 rows are ground.
    // Water = value 1 (animated, frames 0-3)
    // Lava  = value 2 (animated, frames 4-6)
    // Ground= value 8 (static,   atlas col 7 = value-1=7)
    let water_val: u32 = 1;
    let lava_val: u32 = 2;
    let ground_val: u32 = 8; // atlas col 7 = value-1 → static ground

    let mut tiles = Vec::with_capacity(map_rows);
    for r in 0..map_rows {
        let mut row = Vec::with_capacity(map_cols);
        for c in 0..map_cols {
            let v = if r < 4 {
                if (r + c) % 2 == 0 {
                    water_val
                } else {
                    lava_val
                }
            } else {
                ground_val
            };
            row.push(v);
        }
        tiles.push(row);
    }

    // Centre the map on-screen (below the HUD text).
    let map_w = map_cols as f32 * tile_size;
    let map_h = map_rows as f32 * tile_size;
    let origin = Vec2::new((WIN_W - map_w) * 0.5, (WIN_H - map_h) * 0.5 + 40.0);

    let atlas = TilemapAtlas::new(&atlas_path, ATLAS_COLS, ATLAS_ROWS);
    let tilemap = Tilemap::new(atlas, tiles, tile_size, origin);

    // ── Animation set ──────────────────────────────────────────────────────────
    let mut anim_set = TileAnimationSet::new();
    // value 1 (water): atlas frames 0, 1, 2, 3 — 180 ms each.
    anim_set.insert(water_val, TileAnimation::new(vec![0, 1, 2, 3], 0.18));
    // value 2 (lava): atlas frames 4, 5, 6 — 220 ms each.
    anim_set.insert(lava_val, TileAnimation::new(vec![4, 5, 6], 0.22));
    // value 8 (ground): no entry → static.

    // ── Spawn the map entity with both components ──────────────────────────────
    let map_e = app.world.spawn();
    app.world.add_component(map_e, tilemap);
    app.world.add_component(map_e, anim_set);

    // ── Systems ────────────────────────────────────────────────────────────────
    // TilemapSystem must run first (spawns tile entities + tags them).
    // AnimatedTileSystem runs after (advances clocks, writes UvRect).
    app.add_system(TilemapSystem::new());
    app.add_system(AnimatedTileSystem::new());
    app.add_system(HudSystem);
    app.add_system(QuitSystem);

    app.run();
}
