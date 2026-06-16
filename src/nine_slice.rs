//! Nine-slice (9-patch) scalable sprites.
//!
//! Attaching a [`NineSlice`] component to a `Sprite` entity makes the sprite
//! renderer emit **nine** sub-quads instead of one: the four corners keep their
//! fixed size, the four edges stretch along one axis, and the center stretches
//! along both. This is the standard way to draw a resizable UI panel, button, or
//! frame without distorting its rounded/decorated corners.
//!
//! The entity's [`Transform::scale`](crate::components::Transform::scale) is the
//! panel's total size in world pixels. The border is described twice, both
//! indexed `[left, right, top, bottom]` (see [`LEFT`]/[`RIGHT`]/[`TOP`]/[`BOTTOM`]):
//!
//! - [`NineSlice::border`] — the world-pixel widths of the four borders. Corners
//!   are drawn at exactly these sizes regardless of the panel's total size.
//! - [`NineSlice::uv_border`] — the matching fractions (`0.0..1.0`) of the
//!   sprite's UV region taken by each border in the source texture.
//!
//! ```no_run
//! use engine::{App, NineSlice, Sprite, Transform, Vec2};
//! # let mut app = App::new();
//! # let handle = app.load_image("panel.png");
//! let panel = app.world.spawn();
//! app.world.add_component(panel, Sprite::with_handle(handle));
//! app.world.add_component(panel, Transform::new(Vec2::ZERO, Vec2::new(300.0, 120.0), 0.0));
//! // 16px corners; the source border is 25% of the texture on every edge.
//! app.world.add_component(panel, NineSlice::uniform(16.0, 0.25));
//! ```
//!
//! `NineSlice` works with the `texture`/`image_handle` path of [`Sprite`]; it does
//! not apply to [`AtlasSprite`](crate::atlas::AtlasSprite) or to entities that
//! also carry a [`ShaderMaterial`](crate::material::ShaderMaterial).

use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::renderer::uv::UvRect;

/// Index of the left border in [`NineSlice::border`] / [`NineSlice::uv_border`].
pub const LEFT: usize = 0;
/// Index of the right border.
pub const RIGHT: usize = 1;
/// Index of the top border.
pub const TOP: usize = 2;
/// Index of the bottom border.
pub const BOTTOM: usize = 3;

/// Nine-slice (9-patch) parameters for a scalable `Sprite`.
///
/// See the [module docs](self) for the rendering model. Both arrays are indexed
/// `[left, right, top, bottom]`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NineSlice {
    /// World-pixel border widths `[left, right, top, bottom]`. Corners render at
    /// these exact sizes; edges/center fill the remaining space.
    pub border: [f32; 4],
    /// Source-texture border as a UV fraction (`0.0..1.0`) of the sprite's UV
    /// region, `[left, right, top, bottom]`.
    pub uv_border: [f32; 4],
}

impl NineSlice {
    /// Creates a nine-slice with explicit per-edge borders.
    ///
    /// `border` is in world pixels, `uv_border` in UV fractions; both are
    /// `[left, right, top, bottom]`.
    pub fn new(border: [f32; 4], uv_border: [f32; 4]) -> Self {
        Self { border, uv_border }
    }

    /// Creates a nine-slice with the same `border_px` and `uv_frac` on all four edges.
    pub fn uniform(border_px: f32, uv_frac: f32) -> Self {
        Self {
            border: [border_px; 4],
            uv_border: [uv_frac; 4],
        }
    }
}

/// One of the (up to) nine sub-quads of a nine-slice panel, in the panel's local
/// frame (origin = panel center, `+y` up). Crate-internal: the renderer turns
/// each into a sprite instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SubQuad {
    /// Sub-quad center in the panel's local frame.
    pub local_center: Vec2,
    /// Sub-quad size in world pixels.
    pub size: Vec2,
    /// Source-texture region for this sub-quad.
    pub uv: UvRect,
}

/// Computes the nine sub-quads for a panel of `size` (world pixels) sampling base
/// region `uv`, given the border parameters `ns`.
///
/// Centers are in the panel's local frame (origin = center, `+y` up). The sub-quads
/// tile to cover exactly `[-size/2, size/2]` in position and exactly `uv` in texture
/// space. Sub-quads whose width or height collapses to zero (a zero border, or a
/// center squeezed out by oversized borders) are omitted, so the result holds
/// between 0 and 9 quads.
pub(crate) fn nine_slice_subquads(size: Vec2, uv: UvRect, ns: &NineSlice) -> Vec<SubQuad> {
    let (w, h) = (size.x, size.y);

    // ── Position partition ────────────────────────────────────────────────────
    // Columns (x): left, center, right. Borders clamped to >= 0; the center
    // absorbs whatever is left (clamped to >= 0 so oversized borders just collapse it).
    let bl = ns.border[LEFT].max(0.0);
    let br = ns.border[RIGHT].max(0.0);
    let col_w = [bl, (w - bl - br).max(0.0), br];
    let mut x = -w * 0.5;
    let mut col_cx = [0.0f32; 3];
    for i in 0..3 {
        col_cx[i] = x + col_w[i] * 0.5;
        x += col_w[i];
    }
    // Rows (y): top, middle, bottom. Index 0 is the lowest local y, which — because
    // world `+y` is screen-down and the unit quad puts `v = 0` at its lowest y — is the
    // panel's visual TOP. So row 0 uses the TOP border and samples the lowest `v`.
    let bt = ns.border[TOP].max(0.0);
    let bb = ns.border[BOTTOM].max(0.0);
    let row_h = [bt, (h - bt - bb).max(0.0), bb];
    let mut y = -h * 0.5;
    let mut row_cy = [0.0f32; 3];
    for j in 0..3 {
        row_cy[j] = y + row_h[j] * 0.5;
        y += row_h[j];
    }

    // ── UV partition (matched to the position partition, so it tiles `uv`) ──────
    let uvl = ns.uv_border[LEFT];
    let uvr = ns.uv_border[RIGHT];
    let u_w = [
        uvl * uv.u_size,
        (1.0 - uvl - uvr) * uv.u_size,
        uvr * uv.u_size,
    ];
    let mut u = uv.u_offset;
    let mut u_off = [0.0f32; 3];
    for i in 0..3 {
        u_off[i] = u;
        u += u_w[i];
    }
    // Row 0 (the panel's visual top) samples the lowest v, matching the row-height order
    // above and how the unit quad samples (uv `v_offset` sits at the sprite's top edge).
    let uvt = ns.uv_border[TOP];
    let uvb = ns.uv_border[BOTTOM];
    let v_h = [
        uvt * uv.v_size,
        (1.0 - uvt - uvb) * uv.v_size,
        uvb * uv.v_size,
    ];
    let mut v = uv.v_offset;
    let mut v_off = [0.0f32; 3];
    for j in 0..3 {
        v_off[j] = v;
        v += v_h[j];
    }

    let mut quads = Vec::with_capacity(9);
    for j in 0..3 {
        for i in 0..3 {
            if col_w[i] <= 0.0 || row_h[j] <= 0.0 {
                continue;
            }
            quads.push(SubQuad {
                local_center: Vec2::new(col_cx[i], row_cy[j]),
                size: Vec2::new(col_w[i], row_h[j]),
                uv: UvRect::new(u_off[i], v_off[j], u_w[i], v_h[j]),
            });
        }
    }
    quads
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    #[test]
    fn uniform_full_size_yields_nine_quads() {
        let ns = NineSlice::uniform(10.0, 0.2);
        let q = nine_slice_subquads(Vec2::new(100.0, 80.0), UvRect::FULL, &ns);
        assert_eq!(q.len(), 9);
    }

    #[test]
    fn position_partition_covers_full_extent() {
        let ns = NineSlice::new([8.0, 12.0, 5.0, 9.0], [0.1, 0.1, 0.1, 0.1]);
        let (w, h) = (200.0, 120.0);
        let q = nine_slice_subquads(Vec2::new(w, h), UvRect::FULL, &ns);
        // Leftmost edge and rightmost edge of the union must hit -w/2 .. w/2.
        let min_x = q
            .iter()
            .map(|s| s.local_center.x - s.size.x * 0.5)
            .fold(f32::INFINITY, f32::min);
        let max_x = q
            .iter()
            .map(|s| s.local_center.x + s.size.x * 0.5)
            .fold(f32::NEG_INFINITY, f32::max);
        let min_y = q
            .iter()
            .map(|s| s.local_center.y - s.size.y * 0.5)
            .fold(f32::INFINITY, f32::min);
        let max_y = q
            .iter()
            .map(|s| s.local_center.y + s.size.y * 0.5)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(approx(min_x, -w * 0.5), "min_x={min_x}");
        assert!(approx(max_x, w * 0.5), "max_x={max_x}");
        assert!(approx(min_y, -h * 0.5), "min_y={min_y}");
        assert!(approx(max_y, h * 0.5), "max_y={max_y}");
    }

    #[test]
    fn uv_partition_covers_base_region() {
        let ns = NineSlice::uniform(10.0, 0.25);
        let base = UvRect::new(0.5, 0.5, 0.5, 0.5); // an atlas sub-rect, not full
        let q = nine_slice_subquads(Vec2::new(64.0, 64.0), base, &ns);
        let min_u = q
            .iter()
            .map(|s| s.uv.u_offset)
            .fold(f32::INFINITY, f32::min);
        let max_u = q
            .iter()
            .map(|s| s.uv.u_offset + s.uv.u_size)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(approx(min_u, base.u_offset), "min_u={min_u}");
        assert!(approx(max_u, base.u_offset + base.u_size), "max_u={max_u}");
    }

    #[test]
    fn corner_keeps_border_size_independent_of_panel_size() {
        let ns = NineSlice::uniform(16.0, 0.25);
        let small = nine_slice_subquads(Vec2::new(50.0, 50.0), UvRect::FULL, &ns);
        let big = nine_slice_subquads(Vec2::new(400.0, 300.0), UvRect::FULL, &ns);
        // The bottom-left corner is the first emitted quad in both.
        assert!(approx(small[0].size.x, 16.0) && approx(small[0].size.y, 16.0));
        assert!(approx(big[0].size.x, 16.0) && approx(big[0].size.y, 16.0));
        // ...and samples the same corner UV regardless of panel size.
        assert_eq!(small[0].uv, big[0].uv);
    }

    #[test]
    fn zero_border_collapses_to_single_center_quad() {
        let ns = NineSlice::uniform(0.0, 0.0);
        let q = nine_slice_subquads(Vec2::new(100.0, 100.0), UvRect::FULL, &ns);
        assert_eq!(q.len(), 1, "all borders zero -> only the center survives");
        assert_eq!(q[0].uv, UvRect::FULL);
        assert!(approx(q[0].size.x, 100.0) && approx(q[0].size.y, 100.0));
    }

    #[test]
    fn oversized_borders_collapse_center_only() {
        // Borders wider than the panel: center collapses, corners/edges remain.
        let ns = NineSlice::uniform(80.0, 0.25);
        let q = nine_slice_subquads(Vec2::new(100.0, 100.0), UvRect::FULL, &ns);
        // No sub-quad may have a negative size.
        assert!(q.iter().all(|s| s.size.x > 0.0 && s.size.y > 0.0));
    }
}
