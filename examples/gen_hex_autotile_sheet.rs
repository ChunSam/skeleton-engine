//! Asset generator for the 64-tile hex autotile **blob** sheets.
//!
//! The hexagonal analogue of `gen_autotile_sheet` (square Edge4, 16 tiles): produces two
//! sheets where the **tile index == the 6-bit hex neighbor bitmask**, so tile `base + mask`
//! lines up with `TilemapAutotile::hex_6(base)` / `hex_6_flat(base)`'s identity mask→tile map.
//!
//! - `examples/assets/hex_autotile.png`      — pointy-top (`Neighborhood::Hex6`, bit order
//!   E=1, W=2, NE=4, NW=8, SE=16, SW=32), 8×8 grid of 64×74 cells (512×592).
//! - `examples/assets/hex_autotile_flat.png` — flat-top (`Neighborhood::Hex6Flat`, bit order
//!   N=1, S=2, NE=4, SE=8, NW=16, SW=32), 8×8 grid of 74×64 cells (592×512).
//!
//! Each cell draws a regular hexagon (whose vertices exactly fill the cell, matching
//! `Tilemap::cell_render_size`'s aspect for each projection). An edge whose neighbor bit is
//! CLEAR is an **exposed** side and gets a dark outline; a SET (connected) side blends to the
//! hex boundary so neighbouring filled hexes tessellate seamlessly. A field of solid terrain
//! therefore auto-outlines its rim and any dug hole, exactly like the square sheet.
//!
//! Manual tool, not run by CI. From the repo root:
//!     cargo run --example gen_hex_autotile_sheet
//! Deterministic — re-running regenerates the committed PNGs byte-for-byte.

use image::{Rgba, RgbaImage};

/// Pointy-top cell: taller than wide (`W : W·2/√3`), matching `cell_render_size` for `Hexagonal`.
const CELL_POINTY: (u32, u32) = (64, 74);
/// Flat-top cell: wider than tall, matching `cell_render_size` for `HexagonalFlat`.
const CELL_FLAT: (u32, u32) = (74, 64);
/// Outline band width, in pixels, drawn inside an exposed edge.
const BORDER: f32 = 3.0;

/// Cheap deterministic per-pixel hash → 0..=255 (keeps the grass speckle reproducible).
fn h(x: u32, y: u32, salt: u32) -> u32 {
    let mut v = x
        .wrapping_mul(374_761_393)
        .wrapping_add(y.wrapping_mul(668_265_263))
        ^ salt.wrapping_mul(2_246_822_519);
    v ^= v >> 13;
    v = v.wrapping_mul(1_274_126_177);
    (v ^ (v >> 16)) & 0xff
}

/// The 6 hexagon vertices (clockwise, image y-down) of a regular pointy-top hex filling a `w×h` cell.
fn pointy_vertices(w: f32, h: f32) -> [(f32, f32); 6] {
    [
        (w * 0.5, 0.0),  // top
        (w, h * 0.25),   // upper-right
        (w, h * 0.75),   // lower-right
        (w * 0.5, h),    // bottom
        (0.0, h * 0.75), // lower-left
        (0.0, h * 0.25), // upper-left
    ]
}

/// The 6 hexagon vertices (clockwise, image y-down) of a regular flat-top hex filling a `w×h` cell.
fn flat_vertices(w: f32, h: f32) -> [(f32, f32); 6] {
    [
        (w * 0.25, 0.0), // upper-left
        (w * 0.75, 0.0), // upper-right
        (w, h * 0.5),    // right
        (w * 0.75, h),   // lower-right
        (w * 0.25, h),   // lower-left
        (0.0, h * 0.5),  // left
    ]
}

/// Bit (in the layout's neighbor mask) each edge `i` (vertex `i`→`i+1`) sets when its neighbor
/// is present. Order follows `pointy_vertices`: NE, E, SE, SW, W, NW.
const POINTY_EDGE_BITS: [u32; 6] = [4, 1, 16, 32, 2, 8];
/// As `POINTY_EDGE_BITS`, for `flat_vertices`: N, NE, SE, S, SW, NW.
const FLAT_EDGE_BITS: [u32; 6] = [1, 4, 8, 2, 32, 16];

/// Renders one 8×8 = 64-tile blob sheet and writes it to `out`.
fn draw_sheet(cell: (u32, u32), verts: [(f32, f32); 6], edge_bits: [u32; 6], out: &str) {
    let (cw, ch) = cell;
    let mut img = RgbaImage::new(cw * 8, ch * 8);

    // Centroid gives the "inside" half-plane sign for each edge (the hexagon is convex).
    let cx = verts.iter().map(|v| v.0).sum::<f32>() / 6.0;
    let cy = verts.iter().map(|v| v.1).sum::<f32>() / 6.0;
    // Per-edge endpoints + length, for half-plane tests and perpendicular distance.
    let edges: [(f32, f32, f32, f32, f32); 6] = std::array::from_fn(|i| {
        let (ax, ay) = verts[i];
        let (bx, by) = verts[(i + 1) % 6];
        (ax, ay, bx, by, (bx - ax).hypot(by - ay))
    });

    for mask in 0u32..64 {
        let (gc, gr) = (mask % 8, mask / 8);
        for py in 0..ch {
            for px in 0..cw {
                let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);

                // Inside the hex iff on the centroid side of every edge; track the nearest
                // EXPOSED edge so we can outline it.
                let mut inside = true;
                let mut nearest_exposed = f32::INFINITY;
                for (i, &(ax, ay, bx, by, len)) in edges.iter().enumerate() {
                    let cross = (bx - ax) * (fy - ay) - (by - ay) * (fx - ax);
                    let cross_c = (bx - ax) * (cy - ay) - (by - ay) * (cx - ax);
                    if (cross >= 0.0) != (cross_c >= 0.0) {
                        inside = false;
                        break;
                    }
                    if mask & edge_bits[i] == 0 {
                        nearest_exposed = nearest_exposed.min(cross.abs() / len);
                    }
                }
                if !inside {
                    continue; // outside the hexagon → transparent corner
                }

                let color = if nearest_exposed < BORDER {
                    [0x24, 0x2c, 0x18, 0xff] // dark exposed-edge outline
                } else {
                    match h(px, py, mask + 1) {
                        n if n < 30 => [0x3f, 0x63, 0x28, 0xff], // dark grass speckle
                        n if n > 226 => [0x66, 0x93, 0x40, 0xff], // light grass speckle
                        _ => [0x4f, 0x7a, 0x32, 0xff],           // grass
                    }
                };
                img.put_pixel(gc * cw + px, gr * ch + py, Rgba(color));
            }
        }
    }

    img.save(out).unwrap_or_else(|e| panic!("write {out}: {e}"));
    println!("wrote {out} ({}×{})", cw * 8, ch * 8);
}

fn main() {
    draw_sheet(
        CELL_POINTY,
        pointy_vertices(CELL_POINTY.0 as f32, CELL_POINTY.1 as f32),
        POINTY_EDGE_BITS,
        "examples/assets/hex_autotile.png",
    );
    draw_sheet(
        CELL_FLAT,
        flat_vertices(CELL_FLAT.0 as f32, CELL_FLAT.1 as f32),
        FLAT_EDGE_BITS,
        "examples/assets/hex_autotile_flat.png",
    );
}
