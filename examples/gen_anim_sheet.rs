//! Asset generator for the `data_anim` example's animation spritesheet.
//!
//! Produces `examples/games/data_anim/assets/anim.png`: a 4x4 (32px cell = 128x128)
//! sheet. Each ROW is a distinct base color (a different clip); within a row the four
//! COLUMNS move a white dot clockwise around the cell (top → right → bottom → left), so
//! playing a row's frames in order reads as clear motion. Frame index = row*4 + col, which
//! matches the indices in `examples/games/data_anim/assets/clips.ron`.
//!
//! Manual tool, not run by CI:
//!     cargo run --example gen_anim_sheet
//! Deterministic — re-running regenerates the committed PNG byte-for-byte.

use image::{Rgba, RgbaImage};

const CELL: u32 = 32;

/// Base colour per row (= per clip).
const ROW_COLORS: [[u8; 3]; 4] = [
    [0x3a, 0x8a, 0x40], // green  (idle)
    [0x35, 0x6a, 0xc8], // blue   (run)
    [0xd0, 0x86, 0x2c], // orange (spin)
    [0x8a, 0x44, 0xc0], // purple (spare)
];

fn main() {
    let mut img = RgbaImage::new(4 * CELL, 4 * CELL);
    let half = CELL as f32 * 0.5;
    let r_dot = 5.0;

    for row in 0..4u32 {
        let base = ROW_COLORS[row as usize];
        for col in 0..4u32 {
            // Dot centre: clockwise around the cell by column.
            let (dx, dy) = match col {
                0 => (0.0, -8.0), // top
                1 => (8.0, 0.0),  // right
                2 => (0.0, 8.0),  // bottom
                _ => (-8.0, 0.0), // left
            };
            let cx = half + dx;
            let cy = half + dy;
            for y in 0..CELL {
                for x in 0..CELL {
                    let px = (x as f32) + 0.5;
                    let py = (y as f32) + 0.5;
                    let in_dot = (px - cx).powi(2) + (py - cy).powi(2) <= r_dot * r_dot;
                    let [r, g, b] = if in_dot { [0xff, 0xff, 0xff] } else { base };
                    img.put_pixel(col * CELL + x, row * CELL + y, Rgba([r, g, b, 0xff]));
                }
            }
        }
    }

    let out = "examples/games/data_anim/assets/anim.png";
    img.save(out).expect("write anim.png");
    println!("wrote {out}");
}
