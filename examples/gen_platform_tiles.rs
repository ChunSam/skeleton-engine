//! Asset generator for the platformer's seamless tileset.
//!
//! The original `tiles.png` is a set of discrete object sprites with per-cell
//! transparent margins, so it shows gaps when laid out as 1:1 `Tilemap` tiles.
//! This produces `platform_tiles.png`: a seamless 4x4 (128x128, 32px cell) tileset
//! whose ground (idx0), stone (idx1), and one-way (idx8) cells fill edge-to-edge —
//! no per-tile border — so horizontal runs tile without seams.
//!
//! Manual tool, not run by CI. From the repo root:
//!     cargo run --example gen_platform_tiles
//! Deterministic — re-running regenerates the committed PNG byte-for-byte.

use image::{Rgba, RgbaImage};

const CELL: u32 = 32;

/// Cheap deterministic per-pixel hash → 0..=255 (keeps speckle/grain reproducible).
fn h(x: u32, y: u32, salt: u32) -> u32 {
    let mut v = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        ^ salt.wrapping_mul(2_246_822_519);
    v ^= v >> 13;
    v = v.wrapping_mul(1_274_126_177);
    (v ^ (v >> 16)) & 0xff
}

fn fill_cell(img: &mut RgbaImage, idx: u32, mut paint: impl FnMut(u32, u32) -> [u8; 4]) {
    let (c, r) = (idx % 4, idx / 4);
    for y in 0..CELL {
        for x in 0..CELL {
            img.put_pixel(c * CELL + x, r * CELL + y, Rgba(paint(x, y)));
        }
    }
}

fn main() {
    let mut img = RgbaImage::new(128, 128);

    // idx0 — ground: grass cap over speckled dirt. No side borders (seamless across x).
    fill_cell(&mut img, 0, |x, y| {
        if y < 6 {
            if (y == 5 && h(x, y, 1) < 90) || h(x, y, 7) < 30 {
                [0x4f, 0x9c, 0x2c, 0xff] // darker grass blade
            } else {
                [0x6c, 0xc3, 0x40, 0xff] // grass
            }
        } else if y < 8 {
            [0x3f, 0x7d, 0x24, 0xff] // grass shadow band
        } else {
            match h(x, y, 3) {
                n if n < 28 => [0x6f, 0x46, 0x1f, 0xff],  // dark speckle
                n if n > 232 => [0x9e, 0x6a, 0x36, 0xff], // light speckle
                _ => [0x8a, 0x5a, 0x2b, 0xff],            // dirt
            }
        }
    });

    // idx1 — block: top highlight + bottom shadow + crack speckles, no border.
    fill_cell(&mut img, 1, |x, y| {
        if y < 2 {
            [0xb6, 0xc0, 0xcd, 0xff] // lit top
        } else if y >= CELL - 2 {
            [0x5d, 0x66, 0x73, 0xff] // bottom shadow
        } else {
            match h(x, y, 9) {
                n if n < 26 => [0x6b, 0x74, 0x82, 0xff],  // crack
                n if n > 236 => [0x97, 0xa2, 0xb0, 0xff], // fleck
                _ => [0x84, 0x8f, 0x9e, 0xff],            // stone
            }
        }
    });

    // idx8 — one-way platform: wooden planks, lit top edge, plank seams, grain.
    fill_cell(&mut img, 8, |x, y| {
        if y < 3 {
            [0xd4, 0x97, 0x57, 0xff] // lit top
        } else if y % 11 == 10 {
            [0x7d, 0x4f, 0x24, 0xff] // plank seam
        } else if h(x, y, 5) < 24 {
            [0x9c, 0x65, 0x33, 0xff] // grain
        } else {
            [0xb5, 0x79, 0x3f, 0xff] // wood
        }
    });

    let out = "examples/games/platformer/assets/platform_tiles.png";
    img.save(out).expect("write platform_tiles.png");
    println!("wrote {out}");
}
