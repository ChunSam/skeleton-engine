//! Asset generator for the `dig_quest` Edge4 autotile sheet.
//!
//! Produces `examples/games/dig_quest/assets/autotile.png`: a 16-tile
//! (4x4, 32px cell = 128x128) sheet where the **tile index == the Edge4 neighbor
//! bitmask** (N=1, E=2, S=4, W=8). A side's bit is SET when that neighbor is filled
//! (connected), so the cell draws a dark outline on each side whose bit is CLEAR
//! (an exposed edge). This matches `TilemapAutotile::edge_16(base)`'s identity
//! `mask -> base + mask` mapping, so a tilemap of solid terrain auto-outlines its
//! borders and any dug hole.
//!
//! Manual tool, not run by CI. From the repo root:
//!     cargo run --example gen_autotile_sheet
//! Deterministic — re-running regenerates the committed PNG byte-for-byte.

use image::{Rgba, RgbaImage};

const CELL: u32 = 32;
const BORDER: u32 = 3;

/// Cheap deterministic per-pixel hash → 0..=255 (keeps the dirt speckle reproducible).
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
    let mut img = RgbaImage::new(128, 128);

    for mask in 0u32..16 {
        let (c, r) = (mask % 4, mask / 4);
        // Bit SET = neighbor on that side is filled (connected) = NO outline.
        let conn_n = mask & 1 != 0;
        let conn_e = mask & 2 != 0;
        let conn_s = mask & 4 != 0;
        let conn_w = mask & 8 != 0;

        for y in 0..CELL {
            for x in 0..CELL {
                let near_top = y < BORDER;
                let near_bottom = y >= CELL - BORDER;
                let near_left = x < BORDER;
                let near_right = x >= CELL - BORDER;
                // Outline a side only when it is EXPOSED (neighbor absent).
                let on_outline = (!conn_n && near_top)
                    || (!conn_s && near_bottom)
                    || (!conn_w && near_left)
                    || (!conn_e && near_right);

                let color = if on_outline {
                    [0x32, 0x20, 0x10, 0xff] // dark exposed-edge outline
                } else {
                    match h(x, y, 3) {
                        n if n < 28 => [0x6f, 0x46, 0x1f, 0xff],  // dark speckle
                        n if n > 232 => [0x9e, 0x6a, 0x36, 0xff], // light speckle
                        _ => [0x8a, 0x5a, 0x2b, 0xff],            // dirt
                    }
                };
                img.put_pixel(c * CELL + x, r * CELL + y, Rgba(color));
            }
        }
    }

    let out = "examples/games/dig_quest/assets/autotile.png";
    img.save(out).expect("write autotile.png");
    println!("wrote {out}");
}
