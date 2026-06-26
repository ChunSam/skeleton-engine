//! Tilemap subsystem.
//!
//! Four sub-modules:
//! - [`mod@animation`] — per-tile-value frame animation: [`TileAnimation`],
//!   [`TileAnimationSet`], [`AnimatedTileCell`], [`AnimatedTileSystem`].
//! - [`mod@autotile`] — autotile bitmask logic: [`Neighborhood`], [`TilemapAutotile`] +
//!   [`AutotileMode`] (single or per-terrain), [`TerrainRule`], [`compute_tile_mask`],
//!   [`compute_tile_mask_typed`].
//! - [`mod@system`] — reactive render system: [`TilemapSystem`].
//! - This file (`mod.rs`) — data model: [`TilemapAtlas`], [`Tilemap`].

pub mod animation;
pub mod autotile;
pub mod system;

pub use animation::{AnimatedTileCell, AnimatedTileSystem, TileAnimation, TileAnimationSet};
pub use autotile::{
    compute_tile_mask, compute_tile_mask_typed, AutotileMode, Neighborhood, TerrainRule,
    TilemapAutotile,
};
pub use system::TilemapSystem;

use glam::Vec2;

use crate::renderer::uv::UvRect;

/// √3, used by the pointy-top hexagonal projection (hex height = width · 2/√3, row pitch =
/// width · √3/2).
const SQRT_3: f32 = 1.732_050_8;

// ─── Data types ───────────────────────────────────────────────────────────────

/// Texture atlas configuration for a tilemap.
#[derive(Debug, Clone)]
pub struct TilemapAtlas {
    /// Texture file path.
    pub texture: String,
    /// Number of tile columns in the atlas.
    pub columns: u32,
    /// Number of tile rows in the atlas.
    pub rows: u32,
}

impl TilemapAtlas {
    pub fn new(texture: impl Into<String>, columns: u32, rows: u32) -> Self {
        Self {
            texture: texture.into(),
            columns,
            rows,
        }
    }

    /// Returns the UV coordinates for the given tile ID (0-based).
    ///
    /// Returns [`UvRect::FULL`] (safe fallback) when the tile ID is out of range
    /// (i.e. `tile_id >= columns * rows`) to avoid producing UVs outside `[0, 1]`.
    pub fn uv_for(&self, tile_id: u32) -> UvRect {
        if self.columns == 0 || self.rows == 0 {
            return UvRect::FULL;
        }
        let col = tile_id % self.columns;
        let row = tile_id / self.columns;
        // Guard: out-of-range tile ID would produce a row >= rows, yielding UV offset >= 1.0.
        if row >= self.rows {
            return UvRect::FULL;
        }
        UvRect::from_grid(col, row, self.columns, self.rows)
    }
}

/// How a [`Tilemap`]'s `(row, col)` grid maps to world positions.
///
/// [`Orthographic`](Self::Orthographic) is the classic top-down/side square grid (the default,
/// unchanged behavior). [`Isometric`](Self::Isometric) is a 2:1 diamond projection: a tile is a
/// diamond `tile_size` wide and `tile_size / 2` tall, cell `(0, 0)`'s center sits at `origin`,
/// increasing `col` goes right-and-down, increasing `row` goes left-and-down. Isometric tiles
/// overlap, so [`TilemapSystem`] depth-sorts them (a cell's render `z` = `row + col`); place
/// entities you want drawn above the floor at a higher `z`.
///
/// [`Hexagonal`](Self::Hexagonal) is a pointy-top hex grid in **odd-r offset** coordinates
/// (odd rows are shifted right by half a tile), so the rectangular `tiles[row][col]` array maps
/// straight onto it. `tile_size` is the hex's flat-to-flat width; cell `(0, 0)`'s center sits at
/// `origin`. Hexes tessellate without overlap, so they share a fixed render `z` like orthographic.
///
/// [`HexagonalFlat`](Self::HexagonalFlat) is the **flat-top** variant in **odd-q offset**
/// coordinates (odd columns shifted down by half a tile) — the 90°-rotated counterpart of
/// [`Hexagonal`](Self::Hexagonal). `tile_size` is the hex's flat-to-flat **height**; a flat-top hex
/// is wider than it is tall (`cell_render_size` returns `tile_size·2/√3 × tile_size`).
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TilemapProjection {
    /// Square grid: cell `(row, col)` center = `origin + (col + 0.5, row + 0.5) * tile_size`.
    #[default]
    Orthographic,
    /// 2:1 diamond grid (see the type docs).
    Isometric,
    /// Pointy-top hex grid, odd-r offset coordinates (see the type docs).
    Hexagonal,
    /// Flat-top hex grid, odd-q offset coordinates (see the type docs).
    HexagonalFlat,
}

/// Tilemap component.
///
/// Attach to an entity and `TilemapSystem` will automatically spawn tile entities.
/// `tiles[row][col]` = 0 means an empty tile; 1 or more means use `atlas.uv_for(tile_id - 1)`.
#[derive(Debug, Clone)]
pub struct Tilemap {
    pub atlas: TilemapAtlas,
    /// `tiles[row][col]` layout. 0 = empty cell, 1+ = tile ID + 1.
    ///
    /// Mutating this field directly (rather than via [`set_tile`](Self::set_tile)) requires a
    /// follow-up [`bump_generation`](Self::bump_generation) call so the reactive `TilemapSystem`
    /// re-renders the change.
    pub tiles: Vec<Vec<u32>>,
    /// Side length of one tile (pixels).
    pub tile_size: f32,
    /// Top-left origin of the tilemap (world coordinates). For [`TilemapProjection::Isometric`]
    /// this is the world position of cell `(0, 0)`'s center.
    pub origin: Vec2,
    /// How the grid maps to world positions. Defaults to [`TilemapProjection::Orthographic`].
    pub projection: TilemapProjection,
    /// Render depth (`z`) for **orthographic and hexagonal** tilemaps — raise or lower it to stack
    /// multiple tilemaps (e.g. a background floor under a foreground decoration map). Defaults to
    /// `-1.0` (behind sprites at the default `z = 0`). Ignored for [`TilemapProjection::Isometric`],
    /// which derives a per-cell depth (`row + col`) for back-to-front sorting. Set via
    /// [`with_z`](Self::with_z).
    pub z: f32,
    /// Monotonically increasing counter, bumped on every tile mutation.
    /// [`TilemapSystem`] uses this to skip the diff pass when the map is unchanged.
    pub(crate) generation: u64,
}

impl Tilemap {
    pub fn new(atlas: TilemapAtlas, tiles: Vec<Vec<u32>>, tile_size: f32, origin: Vec2) -> Self {
        Self {
            atlas,
            tiles,
            tile_size,
            origin,
            projection: TilemapProjection::Orthographic,
            z: -1.0,
            generation: 0,
        }
    }

    /// Sets the [`projection`](Self::projection) (builder). Default is
    /// [`TilemapProjection::Orthographic`]; pass [`TilemapProjection::Isometric`] for a 2:1 diamond
    /// grid.
    pub fn with_projection(mut self, projection: TilemapProjection) -> Self {
        self.projection = projection;
        self
    }

    /// Sets the render depth [`z`](Self::z) (builder). Layer two orthographic/hexagonal tilemaps
    /// by giving the background a lower `z` than the foreground. No effect on isometric maps
    /// (their depth is per-cell). Default `-1.0`.
    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    /// Sets the tile value at `(row, col)`.
    ///
    /// `value` follows the [`Tilemap`] encoding: `0` = empty cell, `1+` = filled
    /// (the renderer uses `atlas.uv_for(value - 1)` without autotiling, or — with a
    /// [`TilemapAutotile`] attached — treats any non-zero value as connecting
    /// terrain and picks the display tile from the neighbor mask). So to dig a hole
    /// pass `0`; to fill, pass any non-zero value (commonly `1`).
    ///
    /// Returns `true` if the cell was in bounds and the value actually changed.
    /// Returns `false` if the cell was out of bounds or already had that value.
    pub fn set_tile(&mut self, row: usize, col: usize, value: u32) -> bool {
        if row >= self.tiles.len() {
            return false;
        }
        if col >= self.tiles[row].len() {
            return false;
        }
        if self.tiles[row][col] == value {
            return false;
        }
        self.tiles[row][col] = value;
        self.generation = self.generation.wrapping_add(1);
        true
    }

    /// Marks the tilemap dirty so the reactive `TilemapSystem` rebuilds its cells next frame.
    ///
    /// [`set_tile`](Self::set_tile) bumps the generation automatically; call this **only** after
    /// mutating the public [`tiles`](Self::tiles), [`tile_size`](Self::tile_size), or
    /// [`origin`](Self::origin) fields directly (e.g. replacing the whole grid). Without it the
    /// system's change-detection fast-path skips the rebuild and the change is not rendered.
    pub fn bump_generation(&mut self) {
        self.generation = self.generation.wrapping_add(1);
    }

    /// Returns the tile value at `(row, col)`, or `None` if out of bounds.
    pub fn get_tile(&self, row: usize, col: usize) -> Option<u32> {
        self.tiles.get(row)?.get(col).copied()
    }

    /// Returns `(rows, cols)` where `cols` is the maximum row length (jagged-safe).
    /// Returns `(0, 0)` if the tile grid is empty.
    pub fn dims(&self) -> (usize, usize) {
        let rows = self.tiles.len();
        let cols = self.tiles.iter().map(|r| r.len()).max().unwrap_or(0);
        (rows, cols)
    }

    /// Returns the world-space center of cell `(row, col)`, honoring [`projection`](Self::projection).
    ///
    /// Matches the sprite placement [`TilemapSystem`] uses. Orthographic:
    /// `origin + ((col + 0.5) * tile_size, (row + 0.5) * tile_size)`. Isometric (2:1 diamond,
    /// `w = tile_size`, `h = tile_size / 2`): `origin + ((col - row) * w/2, (col + row) * h/2)`.
    pub fn cell_center_world(&self, row: usize, col: usize) -> Vec2 {
        match self.projection {
            TilemapProjection::Orthographic => Vec2::new(
                self.origin.x + col as f32 * self.tile_size + self.tile_size * 0.5,
                self.origin.y + row as f32 * self.tile_size + self.tile_size * 0.5,
            ),
            TilemapProjection::Isometric => {
                let (hw, hh) = (self.tile_size * 0.5, self.tile_size * 0.25);
                Vec2::new(
                    self.origin.x + (col as f32 - row as f32) * hw,
                    self.origin.y + (col as f32 + row as f32) * hh,
                )
            }
            TilemapProjection::Hexagonal => {
                // Pointy-top, odd-r offset: odd rows shifted right by half a width; rows are
                // packed at 3/4 of the hex height (= width * sqrt(3)/2 here, since width = √3·size).
                let x_off = if row % 2 == 1 {
                    self.tile_size * 0.5
                } else {
                    0.0
                };
                Vec2::new(
                    self.origin.x + col as f32 * self.tile_size + x_off,
                    self.origin.y + row as f32 * self.tile_size * (SQRT_3 * 0.5),
                )
            }
            TilemapProjection::HexagonalFlat => {
                // Flat-top, odd-q offset (90°-rotated mirror of Hexagonal): odd columns shifted
                // down by half a height; columns packed at 3/4 of the hex width (= height·√3/2,
                // since height = √3·size). `tile_size` is the flat-to-flat height.
                let y_off = if col % 2 == 1 {
                    self.tile_size * 0.5
                } else {
                    0.0
                };
                Vec2::new(
                    self.origin.x + col as f32 * self.tile_size * (SQRT_3 * 0.5),
                    self.origin.y + row as f32 * self.tile_size + y_off,
                )
            }
        }
    }

    /// Returns the render `z` [`TilemapSystem`] assigns to cell `(row, col)`.
    ///
    /// Orthographic tiles don't overlap, so they share a fixed `z` (`-1.0`, behind gameplay
    /// sprites). Isometric tiles overlap, so each cell gets a painter's-order depth (`row + col`):
    /// tiles further back draw first. (Higher `z` draws on top — so for an isometric scene, give
    /// entities meant to stand on the floor a `z` above the cells they occupy.)
    pub fn cell_z(&self, row: usize, col: usize) -> f32 {
        match self.projection {
            // Orthographic and hexagonal (both orientations) tiles tessellate without overlap, so
            // the whole map shares one depth: the caller-set `z` (default -1.0).
            TilemapProjection::Orthographic
            | TilemapProjection::Hexagonal
            | TilemapProjection::HexagonalFlat => self.z,
            TilemapProjection::Isometric => (row + col) as f32,
        }
    }

    /// Returns the sprite size [`TilemapSystem`] draws a tile at.
    ///
    /// Square (`tile_size × tile_size`) for orthographic and isometric (both use square art — a
    /// diamond drawn inside a square for isometric). Pointy-top **hexagons are taller than wide**
    /// (`tile_size × tile_size · 2/√3`), so hex tiles get that taller sprite; the transparent
    /// corners overlap neighbors harmlessly.
    pub fn cell_render_size(&self) -> Vec2 {
        match self.projection {
            TilemapProjection::Orthographic | TilemapProjection::Isometric => {
                Vec2::splat(self.tile_size)
            }
            TilemapProjection::Hexagonal => {
                Vec2::new(self.tile_size, self.tile_size * 2.0 / SQRT_3)
            }
            // Flat-top hexes are wider than tall (the transpose of pointy-top).
            TilemapProjection::HexagonalFlat => {
                Vec2::new(self.tile_size * 2.0 / SQRT_3, self.tile_size)
            }
        }
    }

    /// Returns the `(row, col)` cell that contains `world_pos`, or `None` if outside
    /// the grid bounds.
    ///
    /// Inverse of [`cell_center_world`](Self::cell_center_world); honors [`projection`](Self::projection).
    pub fn cell_at_world(&self, world_pos: Vec2) -> Option<(usize, usize)> {
        if self.tile_size <= 0.0 {
            return None;
        }
        let (rows, cols) = self.dims();
        let (row, col) = match self.projection {
            TilemapProjection::Orthographic => {
                let rel_x = world_pos.x - self.origin.x;
                let rel_y = world_pos.y - self.origin.y;
                if rel_x < 0.0 || rel_y < 0.0 {
                    return None;
                }
                (
                    (rel_y / self.tile_size) as usize,
                    (rel_x / self.tile_size) as usize,
                )
            }
            TilemapProjection::Isometric => {
                // Invert the diamond transform, then round to the nearest cell (the diamond cells
                // tessellate, so independent rounding of the continuous (col, row) is exact).
                let (hw, hh) = (self.tile_size * 0.5, self.tile_size * 0.25);
                let a = (world_pos.x - self.origin.x) / hw; // = col - row
                let b = (world_pos.y - self.origin.y) / hh; // = col + row
                let colf = (a + b) * 0.5;
                let rowf = (b - a) * 0.5;
                let (row, col) = (rowf.round(), colf.round());
                if row < 0.0 || col < 0.0 {
                    return None;
                }
                (row as usize, col as usize)
            }
            TilemapProjection::Hexagonal => {
                // Pixel → fractional axial (pointy-top, size = width/√3) → cube-round → odd-r
                // offset. Cube-rounding picks the correct hex (independent rounding of axial is
                // not exact near hex borders).
                let size = self.tile_size / SQRT_3;
                let px = world_pos.x - self.origin.x;
                let py = world_pos.y - self.origin.y;
                let q = (SQRT_3 / 3.0 * px - py / 3.0) / size;
                let r = (2.0 / 3.0 * py) / size;
                let (rq, rr) = axial_round(q, r);
                // odd-r offset from axial: col = q + (r - (r & 1)) / 2, row = r.
                let row = rr;
                let col = rq + (rr - (rr & 1)) / 2;
                if row < 0 || col < 0 {
                    return None;
                }
                (row as usize, col as usize)
            }
            TilemapProjection::HexagonalFlat => {
                // Flat-top pixel → fractional axial (size = height/√3) → cube-round → odd-q offset.
                let size = self.tile_size / SQRT_3;
                let px = world_pos.x - self.origin.x;
                let py = world_pos.y - self.origin.y;
                let q = (2.0 / 3.0 * px) / size;
                let r = (-px / 3.0 + SQRT_3 / 3.0 * py) / size;
                let (rq, rr) = axial_round(q, r);
                // odd-q offset from axial: col = q, row = r + (q - (q & 1)) / 2.
                let col = rq;
                let row = rr + (rq - (rq & 1)) / 2;
                if row < 0 || col < 0 {
                    return None;
                }
                (row as usize, col as usize)
            }
        };
        if row >= rows || col >= cols {
            return None;
        }
        Some((row, col))
    }
}

/// Rounds fractional axial hex coordinates `(q, r)` to the nearest hex, via cube rounding.
fn axial_round(q: f32, r: f32) -> (i32, i32) {
    // axial → cube (x = q, z = r, y = -x - z); round each; then restore x + y + z == 0 by
    // recomputing whichever component had the largest rounding error. We only return (q, r) =
    // (x, z), so when y has the largest error there is nothing to fix.
    let (x, z) = (q, r);
    let y = -x - z;
    let (mut rx, ry, mut rz) = (x.round(), y.round(), z.round());
    let (dx, dy, dz) = ((rx - x).abs(), (ry - y).abs(), (rz - z).abs());
    if dx > dy && dx > dz {
        rx = -ry - rz;
    } else if dz >= dy {
        rz = -rx - ry;
    }
    (rx as i32, rz as i32) // (q, r)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn make_atlas() -> TilemapAtlas {
        TilemapAtlas::new("tiles.png", 4, 4) // 16 tiles in a 4×4 grid
    }

    fn make_tilemap(tiles: Vec<Vec<u32>>) -> Tilemap {
        Tilemap::new(make_atlas(), tiles, 32.0, Vec2::ZERO)
    }

    // ── TilemapAtlas ──────────────────────────────────────────────────────────

    #[test]
    fn tilemap_atlas_zero_grid_returns_full_uv() {
        let atlas = TilemapAtlas::new("tiles.png", 0, 0);
        assert_eq!(atlas.uv_for(0), UvRect::FULL);
    }

    #[test]
    fn tilemap_atlas_out_of_range_tile_id_returns_full_uv() {
        let atlas = TilemapAtlas::new("tiles.png", 2, 2);
        let uv = atlas.uv_for(99);
        assert_eq!(
            uv,
            UvRect::FULL,
            "out-of-range tile_id must return UvRect::FULL, got {uv:?}"
        );
        let boundary = atlas.uv_for(4);
        assert_eq!(
            boundary,
            UvRect::FULL,
            "tile_id == columns*rows must return UvRect::FULL, got {boundary:?}"
        );
    }

    #[test]
    fn tilemap_atlas_in_range_tile_id_returns_correct_uv() {
        let atlas = TilemapAtlas::new("tiles.png", 2, 2);
        let uv0 = atlas.uv_for(0);
        assert!(
            uv0.u_offset.abs() < 1e-5 && uv0.v_offset.abs() < 1e-5,
            "tile 0 must be top-left, got {uv0:?}"
        );
        let uv3 = atlas.uv_for(3);
        assert!(
            (uv3.u_offset - 0.5).abs() < 1e-5 && (uv3.v_offset - 0.5).abs() < 1e-5,
            "tile 3 must be bottom-right, got {uv3:?}"
        );
        for id in 0..4 {
            let uv = atlas.uv_for(id);
            assert!(
                uv.u_offset < 1.0 && uv.v_offset < 1.0,
                "tile {id} UV offset must be < 1.0, got {uv:?}"
            );
        }
    }

    // ── Tilemap mutation API ──────────────────────────────────────────────────

    #[test]
    fn set_tile_in_bounds_returns_true_on_change() {
        let mut tm = make_tilemap(vec![vec![1, 2], vec![3, 4]]);
        assert!(tm.set_tile(0, 0, 5), "in-bounds change should return true");
        assert_eq!(tm.tiles[0][0], 5);
    }

    #[test]
    fn set_tile_unchanged_returns_false() {
        let mut tm = make_tilemap(vec![vec![1, 2]]);
        assert!(
            !tm.set_tile(0, 0, 1),
            "setting the same value should return false"
        );
    }

    #[test]
    fn set_tile_out_of_bounds_returns_false() {
        let mut tm = make_tilemap(vec![vec![1, 2]]);
        assert!(!tm.set_tile(5, 0, 3), "row OOB should return false");
        assert!(!tm.set_tile(0, 5, 3), "col OOB should return false");
    }

    #[test]
    fn get_tile_bounds() {
        let tm = make_tilemap(vec![vec![7, 8]]);
        assert_eq!(tm.get_tile(0, 0), Some(7));
        assert_eq!(tm.get_tile(0, 1), Some(8));
        assert_eq!(tm.get_tile(1, 0), None, "out of row bounds");
        assert_eq!(tm.get_tile(0, 2), None, "out of col bounds");
    }

    #[test]
    fn dims_jagged() {
        let tm = Tilemap::new(make_atlas(), vec![vec![1, 2, 3], vec![4]], 32.0, Vec2::ZERO);
        assert_eq!(tm.dims(), (2, 3), "cols = max row length");
        let empty = Tilemap::new(make_atlas(), vec![], 32.0, Vec2::ZERO);
        assert_eq!(empty.dims(), (0, 0));
    }

    #[test]
    fn cell_center_world_and_at_world_round_trip() {
        let tm = make_tilemap(vec![vec![1, 2], vec![3, 4]]);
        // center of (0, 0) should map back to (0, 0)
        let center = tm.cell_center_world(0, 0);
        assert_eq!(center, Vec2::new(16.0, 16.0));
        assert_eq!(tm.cell_at_world(center), Some((0, 0)));

        // center of (1, 1) round-trip
        let center11 = tm.cell_center_world(1, 1);
        assert_eq!(tm.cell_at_world(center11), Some((1, 1)));
    }

    #[test]
    fn cell_at_world_outside_returns_none() {
        let tm = make_tilemap(vec![vec![1, 2], vec![3, 4]]);
        // Negative coordinates
        assert_eq!(tm.cell_at_world(Vec2::new(-1.0, 0.0)), None);
        // Beyond grid (2 rows × 2 cols × 32 pixels = 64×64)
        assert_eq!(tm.cell_at_world(Vec2::new(100.0, 100.0)), None);
    }

    // ── Isometric projection ───────────────────────────────────────────────────

    #[test]
    fn isometric_cell_centers_form_a_diamond() {
        let tm = make_tilemap(vec![vec![1, 2], vec![3, 4]])
            .with_projection(TilemapProjection::Isometric);
        // tile_size 32 → half-width 16, quarter-height 8. Cell (0,0) sits at origin.
        assert_eq!(tm.cell_center_world(0, 0), Vec2::new(0.0, 0.0));
        // +col → right & down; +row → left & down.
        assert_eq!(tm.cell_center_world(0, 1), Vec2::new(16.0, 8.0));
        assert_eq!(tm.cell_center_world(1, 0), Vec2::new(-16.0, 8.0));
        assert_eq!(tm.cell_center_world(1, 1), Vec2::new(0.0, 16.0));
    }

    #[test]
    fn isometric_center_and_at_world_round_trip() {
        let tm = Tilemap::new(
            make_atlas(),
            vec![vec![1; 4]; 4],
            48.0,
            Vec2::new(100.0, 50.0),
        )
        .with_projection(TilemapProjection::Isometric);
        for row in 0..4 {
            for col in 0..4 {
                let center = tm.cell_center_world(row, col);
                assert_eq!(
                    tm.cell_at_world(center),
                    Some((row, col)),
                    "iso round-trip failed for ({row}, {col})"
                );
            }
        }
    }

    #[test]
    fn isometric_picks_nearest_diamond_off_center() {
        let tm = Tilemap::new(make_atlas(), vec![vec![1; 3]; 3], 32.0, Vec2::ZERO)
            .with_projection(TilemapProjection::Isometric);
        // A point a few px from cell (1,1)'s center must still resolve to (1,1).
        let near = tm.cell_center_world(1, 1) + Vec2::new(3.0, -2.0);
        assert_eq!(tm.cell_at_world(near), Some((1, 1)));
    }

    #[test]
    fn isometric_depth_z_orders_back_to_front() {
        let tm = make_tilemap(vec![vec![1, 1], vec![1, 1]])
            .with_projection(TilemapProjection::Isometric);
        // Back cell (0,0) draws behind front cell (1,1).
        assert!(tm.cell_z(0, 0) < tm.cell_z(1, 1));
        assert_eq!(tm.cell_z(0, 0), 0.0);
        assert_eq!(tm.cell_z(1, 1), 2.0);
    }

    #[test]
    fn orthographic_z_is_fixed_behind_sprites() {
        let tm = make_tilemap(vec![vec![1, 2], vec![3, 4]]);
        assert_eq!(tm.cell_z(0, 0), -1.0);
        assert_eq!(tm.cell_z(5, 9), -1.0);
    }

    // ── Hexagonal projection (pointy-top, odd-r offset) ──────────────────────────

    #[test]
    fn hexagonal_cell_centers_offset_odd_rows() {
        let tm = make_tilemap(vec![vec![1; 3]; 3]).with_projection(TilemapProjection::Hexagonal);
        // tile_size 32: cell (0,0) at origin; +col steps a full width; odd rows shift right by half.
        assert_eq!(tm.cell_center_world(0, 0), Vec2::new(0.0, 0.0));
        assert_eq!(tm.cell_center_world(0, 1), Vec2::new(32.0, 0.0));
        // row 1 (odd) is shifted right by 16 and down by 32 * √3/2 ≈ 27.7128.
        let c10 = tm.cell_center_world(1, 0);
        assert!(
            (c10.x - 16.0).abs() < 1e-3,
            "odd row x offset, got {}",
            c10.x
        );
        assert!((c10.y - 27.712_8).abs() < 1e-2, "row pitch, got {}", c10.y);
        // even row 2 is back to no x offset.
        assert!((tm.cell_center_world(2, 0).x - 0.0).abs() < 1e-3);
    }

    #[test]
    fn hexagonal_center_and_at_world_round_trip() {
        let tm = Tilemap::new(
            make_atlas(),
            vec![vec![1; 5]; 5],
            40.0,
            Vec2::new(70.0, 30.0),
        )
        .with_projection(TilemapProjection::Hexagonal);
        for row in 0..5 {
            for col in 0..5 {
                let center = tm.cell_center_world(row, col);
                assert_eq!(
                    tm.cell_at_world(center),
                    Some((row, col)),
                    "hex round-trip failed for ({row}, {col})"
                );
            }
        }
    }

    #[test]
    fn hexagonal_picks_nearest_hex_off_center() {
        let tm = Tilemap::new(make_atlas(), vec![vec![1; 4]; 4], 48.0, Vec2::ZERO)
            .with_projection(TilemapProjection::Hexagonal);
        // A small nudge from a cell center must still resolve to that cell.
        let near = tm.cell_center_world(2, 2) + Vec2::new(4.0, -3.0);
        assert_eq!(tm.cell_at_world(near), Some((2, 2)));
    }

    #[test]
    fn hexagonal_z_is_fixed_no_overlap() {
        let tm = make_tilemap(vec![vec![1; 3]; 3]).with_projection(TilemapProjection::Hexagonal);
        assert_eq!(tm.cell_z(0, 0), -1.0);
        assert_eq!(tm.cell_z(2, 2), -1.0);
    }

    // ── Hexagonal-flat projection (flat-top, odd-q offset) ───────────────────────

    #[test]
    fn flat_top_cell_centers_offset_odd_cols() {
        let tm =
            make_tilemap(vec![vec![1; 3]; 3]).with_projection(TilemapProjection::HexagonalFlat);
        // tile_size 32 = flat-to-flat height; cell (0,0) at origin; +row steps a full height; odd
        // columns shift down by half. Mirror of the pointy-top case with axes swapped.
        assert_eq!(tm.cell_center_world(0, 0), Vec2::new(0.0, 0.0));
        assert_eq!(tm.cell_center_world(1, 0), Vec2::new(0.0, 32.0));
        // col 1 (odd) is shifted down by 16 and right by 32 * √3/2 ≈ 27.7128.
        let c01 = tm.cell_center_world(0, 1);
        assert!(
            (c01.y - 16.0).abs() < 1e-3,
            "odd col y offset, got {}",
            c01.y
        );
        assert!((c01.x - 27.712_8).abs() < 1e-2, "col pitch, got {}", c01.x);
        // even col 2 is back to no y offset.
        assert!((tm.cell_center_world(0, 2).y - 0.0).abs() < 1e-3);
    }

    #[test]
    fn flat_top_center_and_at_world_round_trip() {
        let tm = Tilemap::new(
            make_atlas(),
            vec![vec![1; 5]; 5],
            40.0,
            Vec2::new(70.0, 30.0),
        )
        .with_projection(TilemapProjection::HexagonalFlat);
        for row in 0..5 {
            for col in 0..5 {
                let center = tm.cell_center_world(row, col);
                assert_eq!(
                    tm.cell_at_world(center),
                    Some((row, col)),
                    "flat-top hex round-trip failed for ({row}, {col})"
                );
            }
        }
    }

    #[test]
    fn flat_top_picks_nearest_hex_off_center() {
        let tm = Tilemap::new(make_atlas(), vec![vec![1; 4]; 4], 48.0, Vec2::ZERO)
            .with_projection(TilemapProjection::HexagonalFlat);
        let near = tm.cell_center_world(2, 2) + Vec2::new(-3.0, 4.0);
        assert_eq!(tm.cell_at_world(near), Some((2, 2)));
    }

    #[test]
    fn flat_top_render_size_wider_than_tall_and_z_fixed() {
        let tm =
            make_tilemap(vec![vec![1; 3]; 3]).with_projection(TilemapProjection::HexagonalFlat);
        let rs = tm.cell_render_size();
        assert!(
            rs.x > rs.y,
            "flat-top hex sprite is wider than tall, got {rs:?}"
        );
        assert!((rs.y - 32.0).abs() < 1e-3);
        assert!((rs.x - 32.0 * 2.0 / SQRT_3).abs() < 1e-3);
        assert_eq!(tm.cell_z(0, 0), -1.0);
    }

    // ── Generation guard / set_tile bumps generation ──────────────────────────

    #[test]
    fn set_tile_bumps_generation() {
        let mut tm = make_tilemap(vec![vec![1, 2], vec![3, 4]]);
        let gen0 = tm.generation;
        tm.set_tile(0, 0, 5);
        assert!(
            tm.generation > gen0,
            "generation must increase after a tile change"
        );
    }

    #[test]
    fn set_tile_no_change_does_not_bump_generation() {
        let mut tm = make_tilemap(vec![vec![1, 2]]);
        let gen0 = tm.generation;
        tm.set_tile(0, 0, 1); // same value — returns false
        assert_eq!(
            tm.generation, gen0,
            "generation must not change when set_tile is a no-op"
        );
    }

    #[test]
    fn cell_z_defaults_to_minus_one() {
        let tm = make_tilemap(vec![vec![1; 2]; 2]);
        assert_eq!(tm.cell_z(0, 0), -1.0);
        assert_eq!(tm.cell_z(1, 1), -1.0);
    }

    #[test]
    fn with_z_sets_render_depth_for_ortho_and_hex() {
        // Orthographic: the whole map shares the caller's z.
        let ortho = make_tilemap(vec![vec![1; 2]; 2]).with_z(-5.0);
        assert_eq!(ortho.cell_z(0, 0), -5.0);
        assert_eq!(ortho.cell_z(1, 1), -5.0);
        // Hexagonal: same — a single shared depth.
        let hex = make_tilemap(vec![vec![1; 2]; 2])
            .with_projection(TilemapProjection::Hexagonal)
            .with_z(3.0);
        assert_eq!(hex.cell_z(1, 1), 3.0);
    }

    #[test]
    fn isometric_ignores_z() {
        // Isometric depth is per-cell (row + col); `z` does not apply.
        let iso = make_tilemap(vec![vec![1; 2]; 2])
            .with_projection(TilemapProjection::Isometric)
            .with_z(99.0);
        assert_eq!(iso.cell_z(0, 0), 0.0);
        assert_eq!(iso.cell_z(1, 1), 2.0);
    }
}
