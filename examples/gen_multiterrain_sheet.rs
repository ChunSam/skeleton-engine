//! Asset generator for the `multi_terrain` example's 3-terrain Edge4 autotile sheet.
//!
//! Produces `examples/games/multi_terrain/assets/terrains.png`: a 48-tile
//! (16 cols × 3 rows, 32px cell = 512×96) sheet. Each ROW is one terrain
//! (0 = grass, 1 = water, 2 = sand); each COLUMN is the Edge4 neighbor bitmask
//! (N=1, E=2, S=4, W=8). A side's bit is SET when the same-terrain neighbor is
//! present, so a dark outline is drawn on each side whose bit is CLEAR (an exposed
//! edge against a different terrain).
//!
//! Atlas tile index = row*16 + mask, matching
//! `TilemapAutotile::multi_edge_16(&[(1, 0), (2, 16), (3, 32)])` (terrain value → base id).
//!
//! Manual tool, not run by CI:
//!     cargo run --example gen_multiterrain_sheet
//! Deterministic — re-running regenerates the committed PNG byte-for-byte.

use image::{Rgba, RgbaImage};

const CELL: u32 = 32;
const BORDER: u32 = 3;

/// (fill, light speckle, dark speckle, outline) RGB for one terrain.
type TerrainPalette = ([u8; 3], [u8; 3], [u8; 3], [u8; 3]);

const TERRAINS: [TerrainPalette; 3] = [
    // grass
    (
        [0x5c, 0xa8, 0x3a],
        [0x6f, 0xc0, 0x49],
        [0x47, 0x86, 0x2c],
        [0x26, 0x47, 0x18],
    ),
    // water
    (
        [0x39, 0x6e, 0xc8],
        [0x4c, 0x86, 0xe0],
        [0x2b, 0x55, 0xa0],
        [0x18, 0x30, 0x5e],
    ),
    // sand
    (
        [0xd6, 0xb8, 0x70],
        [0xe6, 0xcd, 0x8c],
        [0xbf, 0x9d, 0x55],
        [0x82, 0x66, 0x30],
    ),
];

/// Cheap deterministic per-pixel hash → 0..=255.
fn h(x: u32, y: u32, salt: u32) -> u32 {
    let mut v = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        ^ salt.wrapping_mul(2_246_822_519);
    v ^= v >> 13;
    v = v.wrapping_mul(1_274_126_177);
    (v ^ (v >> 16)) & 0xff
}

fn main() {
    let mut img = RgbaImage::new(16 * CELL, 3 * CELL);

    for (row, (fill, light, dark, outline)) in TERRAINS.iter().enumerate() {
        for mask in 0u32..16 {
            let conn_n = mask & 1 != 0;
            let conn_e = mask & 2 != 0;
            let conn_s = mask & 4 != 0;
            let conn_w = mask & 8 != 0;

            for y in 0..CELL {
                for x in 0..CELL {
                    let on_outline = (!conn_n && y < BORDER)
                        || (!conn_s && y >= CELL - BORDER)
                        || (!conn_w && x < BORDER)
                        || (!conn_e && x >= CELL - BORDER);

                    let [r, g, b] = if on_outline {
                        *outline
                    } else {
                        match h(x, y, row as u32 + 1) {
                            n if n < 28 => *dark,
                            n if n > 232 => *light,
                            _ => *fill,
                        }
                    };
                    let px = mask * CELL + x;
                    let py = row as u32 * CELL + y;
                    img.put_pixel(px, py, Rgba([r, g, b, 0xff]));
                }
            }
        }
    }

    let out = "examples/games/multi_terrain/assets/terrains.png";
    img.save(out).expect("write terrains.png");
    println!("wrote {out}");
}
