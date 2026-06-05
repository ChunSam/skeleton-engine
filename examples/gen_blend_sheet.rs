//! Asset generator for the `blend_locomotion` demo's spritesheet.
//!
//! Produces `examples/assets/blend_locomotion.png`: a 4×3 grid (64px cells) where each
//! **row** is a locomotion clip — idle (blue), walk (green), run (orange) — and each
//! **column** is a frame. A little stick figure bobs vertically (small for idle, large
//! for run) and swings its legs per frame, so frames within a row animate and the rows
//! are strongly distinct in hue. That hue contrast is what makes the 2-UV crossfade
//! blend (idle↔walk↔run) read clearly in `blend_locomotion`.
//!
//! Manual tool, not run by CI. From the repo root:
//!     cargo run --example gen_blend_sheet
//! Deterministic — re-running regenerates the committed PNG byte-for-byte.

use image::{Rgba, RgbaImage};

const CELL: u32 = 64;
const COLS: u32 = 4;
const ROWS: u32 = 3;

/// Per-row base hue: idle / walk / run.
const HUES: [[u8; 3]; 3] = [
    [70, 120, 225], // idle — blue
    [80, 195, 95],  // walk — green
    [235, 135, 55], // run  — orange
];
/// Vertical bob amplitude (px) per row — idle barely moves, run leaps.
const BOB: [f32; 3] = [2.0, 6.0, 12.0];

/// Scale a hue by `f` (shade), opaque.
fn shade(c: [u8; 3], f: f32) -> [u8; 4] {
    let g = |v: u8| (v as f32 * f).round().clamp(0.0, 255.0) as u8;
    [g(c[0]), g(c[1]), g(c[2]), 255]
}

/// Lighten a hue toward white by `add`, opaque.
fn lighten(c: [u8; 3], add: f32) -> [u8; 4] {
    let g = |v: u8| (v as f32 + add).clamp(0.0, 255.0) as u8;
    [g(c[0]), g(c[1]), g(c[2]), 255]
}

fn paint_cell(img: &mut RgbaImage, col: u32, row: u32) {
    let hue = HUES[row as usize];
    let phase = std::f32::consts::TAU * (col as f32) / COLS as f32;
    let bob = BOB[row as usize] * phase.sin();
    // Legs apart on even frames, together on odd → a readable stride.
    let stride = if col.is_multiple_of(2) { 8.0 } else { 2.5 };

    let (ox, oy) = (col * CELL, row * CELL);
    let cx = CELL as f32 / 2.0;
    let ground = CELL as f32 - 8.0;
    let body_w = 20.0;
    let body_h = 22.0;
    let body_bottom = ground - 4.0 - bob;
    let body_top = body_bottom - body_h;
    let head_r = 7.0;
    let head_cy = body_top - head_r + 1.0;

    for y in 0..CELL {
        for x in 0..CELL {
            let fx = x as f32;
            let fy = y as f32;
            let mut px = shade(hue, 0.18); // dark background

            // ground strip
            if (ground..ground + 2.0).contains(&fy) {
                px = shade(hue, 0.32);
            }
            // legs (two vertical bars from body to ground)
            if fy >= body_bottom
                && fy <= ground - bob.max(0.0)
                && ((fx - (cx - stride)).abs() < 2.5 || (fx - (cx + stride)).abs() < 2.5)
            {
                px = shade(hue, 0.7);
            }
            // body
            if (cx - body_w / 2.0..=cx + body_w / 2.0).contains(&fx)
                && (body_top..=body_bottom).contains(&fy)
            {
                px = shade(hue, 1.0);
                if (fy - (body_top + body_h * 0.45)).abs() < 2.0 {
                    px = shade(hue, 0.6); // belt band
                }
            }
            // head
            if (fx - cx).powi(2) + (fy - head_cy).powi(2) <= head_r * head_r {
                px = lighten(hue, 40.0);
            }
            img.put_pixel(ox + x, oy + y, Rgba(px));
        }
    }
}

fn main() {
    let mut img = RgbaImage::new(COLS * CELL, ROWS * CELL);
    for row in 0..ROWS {
        for col in 0..COLS {
            paint_cell(&mut img, col, row);
        }
    }
    std::fs::create_dir_all("examples/assets").expect("create examples/assets");
    let out = "examples/assets/blend_locomotion.png";
    img.save(out).expect("write blend_locomotion.png");
    println!("wrote {out} ({}×{})", COLS * CELL, ROWS * CELL);
}
