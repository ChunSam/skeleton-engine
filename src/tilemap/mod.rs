//! Tilemap subsystem.
//!
//! Three sub-modules:
//! - [`mod@autotile`] — autotile bitmask logic: [`Neighborhood`], [`ConnectRule`],
//!   [`TilemapAutotile`], [`TerrainRule`], [`MultiTerrainAutotile`],
//!   [`compute_tile_mask`], [`compute_tile_mask_typed`].
//! - [`mod@system`] — reactive render system: [`TilemapSystem`].
//! - This file (`mod.rs`) — data model: [`TilemapAtlas`], [`Tilemap`].

pub mod autotile;
pub mod system;

pub use autotile::{
    compute_tile_mask, compute_tile_mask_typed, ConnectRule, MultiTerrainAutotile, Neighborhood,
    TerrainRule, TilemapAutotile,
};
pub use system::TilemapSystem;

use glam::Vec2;

use crate::renderer::uv::UvRect;

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
    /// Top-left origin of the tilemap (world coordinates).
    pub origin: Vec2,
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
            generation: 0,
        }
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

    /// Returns the world-space center of cell `(row, col)`.
    ///
    /// Matches the sprite placement formula used by [`TilemapSystem`]:
    /// `origin + (col * tile_size + tile_size * 0.5, row * tile_size + tile_size * 0.5)`.
    pub fn cell_center_world(&self, row: usize, col: usize) -> Vec2 {
        Vec2::new(
            self.origin.x + col as f32 * self.tile_size + self.tile_size * 0.5,
            self.origin.y + row as f32 * self.tile_size + self.tile_size * 0.5,
        )
    }

    /// Returns the `(row, col)` cell that contains `world_pos`, or `None` if outside
    /// the grid bounds.
    ///
    /// Inverse of [`cell_center_world`](Self::cell_center_world).
    pub fn cell_at_world(&self, world_pos: Vec2) -> Option<(usize, usize)> {
        if self.tile_size <= 0.0 {
            return None;
        }
        let rel_x = world_pos.x - self.origin.x;
        let rel_y = world_pos.y - self.origin.y;
        if rel_x < 0.0 || rel_y < 0.0 {
            return None;
        }
        let col = (rel_x / self.tile_size) as usize;
        let row = (rel_y / self.tile_size) as usize;
        let (rows, cols) = self.dims();
        if row >= rows || col >= cols {
            return None;
        }
        Some((row, col))
    }
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
}
