use std::collections::HashMap;

// ─── Autotiling ───────────────────────────────────────────────────────────────

/// Which set of neighbors to use when computing the autotile bitmask.
///
/// [`Edge4`](Self::Edge4) / [`Blob8`](Self::Blob8) are square-grid neighborhoods (4 / 8 neighbors).
/// Because the bitmask is computed from the `tiles[row][col]` topology, these work unchanged for
/// **both** [`Orthographic`](super::TilemapProjection::Orthographic) and
/// [`Isometric`](super::TilemapProjection::Isometric) maps — isometric is the same square grid,
/// just rendered as diamonds.
///
/// [`Hex6`](Self::Hex6) / [`Hex6Flat`](Self::Hex6Flat) are the 6-neighbor hex neighborhoods for
/// [`Hexagonal`](super::TilemapProjection::Hexagonal) (pointy-top, odd-r) and
/// [`HexagonalFlat`](super::TilemapProjection::HexagonalFlat) (flat-top, odd-q): the 6 neighbor
/// offsets are parity-dependent (on row for odd-r, on column for odd-q), so the neighbor set
/// matches the staggered hex layout `cell_center_world` produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Neighborhood {
    /// 4 orthogonal neighbors. Bit order: N=1, E=2, S=4, W=8.
    Edge4,
    /// 8 neighbors with blob corner-validity reduction.
    ///
    /// Bit order: N=1, E=2, S=4, W=8, NE=16, SE=32, SW=64, NW=128.
    /// A diagonal bit is set only when *both* of its orthogonally-adjacent edge
    /// neighbors are also filled (the standard "blob" reduction). This yields the
    /// canonical 47 distinct reduced masks.
    Blob8,
    /// 6 neighbors of a **pointy-top, odd-r** hex (for [`TilemapProjection::Hexagonal`]).
    ///
    /// Bit order: E=1, W=2, NE=4, NW=8, SE=16, SW=32 (mask range `0..64`). The four diagonal
    /// neighbors shift with row parity (odd rows are offset right half a tile).
    ///
    /// [`TilemapProjection::Hexagonal`]: super::TilemapProjection::Hexagonal
    Hex6,
    /// 6 neighbors of a **flat-top, odd-q** hex (for [`TilemapProjection::HexagonalFlat`]).
    ///
    /// Bit order: N=1, S=2, NE=4, SE=8, NW=16, SW=32 (mask range `0..64`). The four diagonal
    /// neighbors shift with column parity (odd columns are offset down half a tile).
    ///
    /// [`TilemapProjection::HexagonalFlat`]: super::TilemapProjection::HexagonalFlat
    Hex6Flat,
}

/// How a [`TilemapAutotile`] maps neighbor connectivity to display tiles.
///
/// [`Single`](Self::Single) treats **any** non-zero cell as connecting to any other non-zero cell
/// (one terrain). [`Multi`](Self::Multi) gives each terrain value its own rule and connects only to
/// same-value neighbors. This mirrors the dispatch [`TilemapSystem`] does internally.
///
/// [`TilemapSystem`]: super::system::TilemapSystem
#[derive(Debug, Clone)]
pub enum AutotileMode {
    /// Single connecting terrain: bitmask → 0-based atlas display id.
    Single {
        /// Computed neighbor bitmask → 0-based atlas display id.
        mask_to_tile: HashMap<u8, u32>,
    },
    /// Per-terrain rules: each non-zero value autotiles against same-value neighbors only.
    Multi {
        /// One rule per distinct terrain value.
        rules: Vec<TerrainRule>,
    },
}

/// Component that makes [`TilemapSystem`] choose tile display UVs based on neighbor connectivity
/// rather than the raw tile value.
///
/// Attach to the same entity as [`super::Tilemap`]. Use [`edge_16`](Self::edge_16) /
/// [`blob_47`](Self::blob_47) for a single connecting terrain, or
/// [`multi_edge_16`](Self::multi_edge_16) for per-terrain autotiling — both produce one
/// `TilemapAutotile`, distinguished by its [`mode`](Self::mode).
///
/// # Example
///
/// ```rust,ignore
/// world.add_component(map_entity, TilemapAutotile::edge_16(0));            // single terrain
/// world.add_component(other, TilemapAutotile::multi_edge_16(&[(1, 0), (2, 16)])); // per-terrain
/// ```
///
/// [`TilemapSystem`]: super::system::TilemapSystem
#[derive(Debug, Clone)]
pub struct TilemapAutotile {
    /// Which neighborhood scheme to use.
    pub neighborhood: Neighborhood,
    /// Treat out-of-bounds neighbors as filled (`true`) or empty (`false`).
    pub oob_filled: bool,
    /// Single-terrain or per-terrain mapping.
    pub mode: AutotileMode,
}

impl TilemapAutotile {
    /// 16-tile edge autotile layout (single terrain).
    ///
    /// Assumes a contiguous 16-tile strip in the atlas starting at `base_atlas_id`
    /// where tile `base_atlas_id + mask` corresponds to bitmask `mask` (N=1,E=2,S=4,W=8).
    pub fn edge_16(base_atlas_id: u32) -> Self {
        let mask_to_tile = (0u8..16).map(|m| (m, base_atlas_id + m as u32)).collect();
        Self {
            neighborhood: Neighborhood::Edge4,
            oob_filled: false,
            mode: AutotileMode::Single { mask_to_tile },
        }
    }

    /// 47-tile blob autotile layout.
    ///
    /// Uses the canonical blob-8 neighborhood. A contiguous 47-tile strip starting
    /// at `base_atlas_id` is assumed. The 47 valid reduced masks are enumerated in
    /// ascending order (0, 2, 8, 10, 11, 16, 18, 22, 24, 26, 27, 30, 31, 64, 66,
    /// 72, 74, 75, 80, 82, 86, 88, 90, 91, 94, 95, 104, 106, 107, 120, 122, 123,
    /// 126, 127, 208, 210, 214, 216, 218, 219, 222, 223, 248, 250, 251, 254, 255)
    /// and each is assigned `base_atlas_id + <index in that sorted list>`.
    ///
    /// # Blob-8 bit order
    /// N=1, E=2, S=4, W=8, NE=16, SE=32, SW=64, NW=128.
    /// A diagonal bit is valid only when both adjacent orthogonal bits are set.
    pub fn blob_47(base_atlas_id: u32) -> Self {
        // The 47 canonical reduced blob masks for the code's bit convention:
        // N=1, E=2, S=4, W=8, NE=16, SE=32, SW=64, NW=128.
        // A mask is valid when every set diagonal bit has both adjacent orthogonal bits set:
        //   NE(16) needs N(1)+E(2); SE(32) needs S(4)+E(2);
        //   SW(64) needs S(4)+W(8); NW(128) needs N(1)+W(8).
        // Generated by iterating 0..=255 and keeping only those passing the validity check.
        const VALID_MASKS: &[u8] = &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 19, 23, 27, 31, 38, 39, 46, 47,
            55, 63, 76, 77, 78, 79, 95, 110, 111, 127, 137, 139, 141, 143, 155, 159, 175, 191, 205,
            207, 223, 239, 255,
        ];
        let mask_to_tile = VALID_MASKS
            .iter()
            .enumerate()
            .map(|(i, &m)| (m, base_atlas_id + i as u32))
            .collect();
        Self {
            neighborhood: Neighborhood::Blob8,
            oob_filled: false,
            mode: AutotileMode::Single { mask_to_tile },
        }
    }

    /// 16-tile edge layout **per terrain** (multi-terrain autotiling).
    ///
    /// `terrains` is a slice of `(terrain_value, base_atlas_id)` pairs. Each terrain gets a
    /// `mask_to_tile` mapping `m → base_atlas_id + m` for `m` in `0..16` (a contiguous 16-tile edge
    /// strip), and connects only to same-value neighbors.
    pub fn multi_edge_16(terrains: &[(u32, u32)]) -> Self {
        let rules = terrains
            .iter()
            .map(|&(terrain, base)| {
                let mask_to_tile = (0u8..16).map(|m| (m, base + m as u32)).collect();
                TerrainRule {
                    terrain,
                    mask_to_tile,
                }
            })
            .collect();
        Self {
            neighborhood: Neighborhood::Edge4,
            oob_filled: false,
            mode: AutotileMode::Multi { rules },
        }
    }

    /// 64-tile pointy-top hex autotile layout (single terrain), for
    /// [`TilemapProjection::Hexagonal`](super::TilemapProjection::Hexagonal).
    ///
    /// Assumes a contiguous 64-tile strip starting at `base_atlas_id` where tile
    /// `base_atlas_id + mask` corresponds to the 6-bit [`Neighborhood::Hex6`] mask
    /// (E=1, W=2, NE=4, NW=8, SE=16, SW=32).
    pub fn hex_6(base_atlas_id: u32) -> Self {
        Self::hex_single(Neighborhood::Hex6, base_atlas_id)
    }

    /// 64-tile flat-top hex autotile layout (single terrain), for
    /// [`TilemapProjection::HexagonalFlat`](super::TilemapProjection::HexagonalFlat). Like
    /// [`hex_6`](Self::hex_6) but with the [`Neighborhood::Hex6Flat`] bit order
    /// (N=1, S=2, NE=4, SE=8, NW=16, SW=32).
    pub fn hex_6_flat(base_atlas_id: u32) -> Self {
        Self::hex_single(Neighborhood::Hex6Flat, base_atlas_id)
    }

    /// Shared constructor for the two 64-tile hex single-terrain layouts.
    fn hex_single(nb: Neighborhood, base_atlas_id: u32) -> Self {
        let mask_to_tile = (0u8..64).map(|m| (m, base_atlas_id + m as u32)).collect();
        Self {
            neighborhood: nb,
            oob_filled: false,
            mode: AutotileMode::Single { mask_to_tile },
        }
    }

    /// Sets [`oob_filled`](Self::oob_filled) and returns `self` (builder style).
    ///
    /// Pass `true` for a contained field (e.g. a cave) whose outer world boundary
    /// should read as solid wall, so only interior holes draw outlines; `false`
    /// (the constructor default) outlines the map edges too.
    pub fn with_oob_filled(mut self, oob_filled: bool) -> Self {
        self.oob_filled = oob_filled;
        self
    }

    /// For [`AutotileMode::Multi`], the rule whose `terrain` equals `terrain`; `None` otherwise.
    pub(super) fn rule_for(&self, terrain: u32) -> Option<&TerrainRule> {
        match &self.mode {
            AutotileMode::Multi { rules } => rules.iter().find(|r| r.terrain == terrain),
            AutotileMode::Single { .. } => None,
        }
    }
}

/// Core bitmask computation shared by [`compute_tile_mask`] and [`compute_tile_mask_typed`].
///
/// `filled(r, c)` returns `true` when the cell at grid coordinates `(r, c)` should be
/// considered a connected neighbor. Both public functions construct a different `filled`
/// closure (non-zero vs. equals-terrain) and delegate here.
///
/// # Bit order
/// - [`Neighborhood::Edge4`]: N=1, E=2, S=4, W=8
/// - [`Neighborhood::Blob8`]: additionally NE=16, SE=32, SW=64, NW=128
///   (diagonal bits are zeroed if either adjacent orthogonal neighbor is empty)
fn compute_mask_raw(
    row: usize,
    col: usize,
    nb: Neighborhood,
    filled: impl Fn(i32, i32) -> bool,
) -> u8 {
    let r = row as i32;
    let c = col as i32;

    match nb {
        Neighborhood::Hex6 => return hex6_mask(r, c, row % 2 == 1, filled),
        Neighborhood::Hex6Flat => return hex6_flat_mask(r, c, col % 2 == 1, filled),
        Neighborhood::Edge4 | Neighborhood::Blob8 => {}
    }

    let n = filled(r - 1, c);
    let e = filled(r, c + 1);
    let s = filled(r + 1, c);
    let w = filled(r, c - 1);

    let mut mask: u8 = 0;
    if n {
        mask |= 1;
    }
    if e {
        mask |= 2;
    }
    if s {
        mask |= 4;
    }
    if w {
        mask |= 8;
    }

    if nb == Neighborhood::Blob8 {
        // NE corner: valid only when both N and E are filled.
        if filled(r - 1, c + 1) && n && e {
            mask |= 16;
        }
        // SE corner: valid only when both S and E are filled.
        if filled(r + 1, c + 1) && s && e {
            mask |= 32;
        }
        // SW corner: valid only when both S and W are filled.
        if filled(r + 1, c - 1) && s && w {
            mask |= 64;
        }
        // NW corner: valid only when both N and W are filled.
        if filled(r - 1, c - 1) && n && w {
            mask |= 128;
        }
    }

    mask
}

/// Computes a 6-bit hex neighbor mask from a table of neighbor offsets. `offsets[i]` is the
/// `(drow, dcol)` of the neighbor whose presence sets bit `1 << i`, so the table is ordered by
/// ascending bit value (bits 1, 2, 4, 8, 16, 32). The two hex layouts differ only in their offset
/// tables; this shared accumulator keeps the bit-setting loop in one place.
fn hex_mask_from_offsets(
    r: i32,
    c: i32,
    offsets: [(i32, i32); 6],
    filled: impl Fn(i32, i32) -> bool,
) -> u8 {
    let mut mask = 0u8;
    for (i, &(dr, dc)) in offsets.iter().enumerate() {
        if filled(r + dr, c + dc) {
            mask |= 1 << i;
        }
    }
    mask
}

/// Pointy-top, odd-r hex bitmask: E=1, W=2, NE=4, NW=8, SE=16, SW=32. The NE/NW/SE/SW offsets
/// depend on `odd_row` (odd rows are shifted right half a tile).
fn hex6_mask(r: i32, c: i32, odd_row: bool, filled: impl Fn(i32, i32) -> bool) -> u8 {
    // (drow, dcol) in bit order [E, W, NE, NW, SE, SW]; E/W are parity-independent.
    let offsets = if odd_row {
        [(0, 1), (0, -1), (-1, 1), (-1, 0), (1, 1), (1, 0)]
    } else {
        [(0, 1), (0, -1), (-1, 0), (-1, -1), (1, 0), (1, -1)]
    };
    hex_mask_from_offsets(r, c, offsets, filled)
}

/// Flat-top, odd-q hex bitmask: N=1, S=2, NE=4, SE=8, NW=16, SW=32. The NE/SE/NW/SW offsets depend
/// on `odd_col` (odd columns are shifted down half a tile).
fn hex6_flat_mask(r: i32, c: i32, odd_col: bool, filled: impl Fn(i32, i32) -> bool) -> u8 {
    // (drow, dcol) in bit order [N, S, NE, SE, NW, SW]; N/S are parity-independent.
    let offsets = if odd_col {
        [(-1, 0), (1, 0), (0, 1), (1, 1), (0, -1), (1, -1)]
    } else {
        [(-1, 0), (1, 0), (-1, 1), (0, 1), (-1, -1), (0, -1)]
    };
    hex_mask_from_offsets(r, c, offsets, filled)
}

/// Builds the `filled(r, c)` bounds-checking closure shared by [`compute_tile_mask`] and
/// [`compute_tile_mask_typed`]. Out-of-bounds cells return `oob_filled`; in-bounds cells
/// are tested by `pred(cell_value)`.
fn make_filled<'a>(
    tiles: &'a [Vec<u32>],
    oob_filled: bool,
    pred: impl Fn(u32) -> bool + 'a,
) -> impl Fn(i32, i32) -> bool + 'a {
    move |r: i32, c: i32| {
        let row_count = tiles.len() as i32;
        if r < 0 || r >= row_count {
            return oob_filled;
        }
        let row_ref = &tiles[r as usize];
        let col_count = row_ref.len() as i32;
        if c < 0 || c >= col_count {
            return oob_filled;
        }
        pred(row_ref[c as usize])
    }
}

/// Computes the autotile bitmask for cell `(row, col)` in the given grid.
///
/// A neighbor is "filled" when in-bounds and non-zero; if out-of-bounds, it is
/// filled iff `oob_filled` is true.
///
/// # Bit order
/// - [`Neighborhood::Edge4`]: N=1, E=2, S=4, W=8
/// - [`Neighborhood::Blob8`]: N=1, E=2, S=4, W=8, NE=16, SE=32, SW=64, NW=128
///   (diagonal bits are zeroed if either adjacent orthogonal neighbor is empty)
pub fn compute_tile_mask(
    tiles: &[Vec<u32>],
    row: usize,
    col: usize,
    nb: Neighborhood,
    oob_filled: bool,
) -> u8 {
    compute_mask_raw(row, col, nb, make_filled(tiles, oob_filled, |v| v != 0))
}

// ─── Multi-terrain autotiling ─────────────────────────────────────────────────

/// One terrain's autotile mapping: tile value `terrain` (a non-zero cell value)
/// maps each neighbor bitmask to an atlas display id.
#[derive(Debug, Clone)]
pub struct TerrainRule {
    /// The tile value identifying this terrain (must be non-zero).
    pub terrain: u32,
    /// Bitmask → 0-based atlas display id for this terrain.
    pub mask_to_tile: HashMap<u8, u32>,
}

// Per-terrain autotiling now lives on `TilemapAutotile` via `AutotileMode::Multi`
// (built with `TilemapAutotile::multi_edge_16`); `TerrainRule` above is its per-terrain entry.

/// Like [`compute_tile_mask`], but a neighbor counts as connected **only** when its
/// value equals `terrain` (same-terrain connectivity). Out-of-bounds neighbors count
/// as connected iff `oob_filled` is `true`.
///
/// # Bit order
/// Matches [`compute_tile_mask`]: N=1, E=2, S=4, W=8 for [`Neighborhood::Edge4`];
/// additionally NE=16, SE=32, SW=64, NW=128 for [`Neighborhood::Blob8`].
pub fn compute_tile_mask_typed(
    tiles: &[Vec<u32>],
    row: usize,
    col: usize,
    nb: Neighborhood,
    oob_filled: bool,
    terrain: u32,
) -> u8 {
    compute_mask_raw(
        row,
        col,
        nb,
        make_filled(tiles, oob_filled, move |v| v == terrain),
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::{System, World};
    use crate::tilemap::{Tilemap, TilemapAtlas};

    /// Extracts the single-terrain `mask_to_tile`, panicking if `at` is multi-terrain.
    fn single_map(at: &TilemapAutotile) -> &HashMap<u8, u32> {
        match &at.mode {
            AutotileMode::Single { mask_to_tile } => mask_to_tile,
            AutotileMode::Multi { .. } => panic!("expected AutotileMode::Single"),
        }
    }

    fn make_multi_terrain_atlas() -> TilemapAtlas {
        // 32 tiles in a 32×1 strip: grass tiles 0–15, water tiles 16–31.
        TilemapAtlas::new("tiles.png", 32, 1)
    }

    // ── compute_tile_mask / Edge4 ─────────────────────────────────────────────

    #[test]
    fn mask_edge4_all_neighbors_filled() {
        // A 3×3 map fully filled; center cell (1,1) has all 4 neighbors.
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let mask = compute_tile_mask(&tiles, 1, 1, Neighborhood::Edge4, false);
        assert_eq!(mask, 15, "all 4 orthogonal neighbors → mask 0b1111 = 15");
    }

    #[test]
    fn mask_edge4_isolated_cell() {
        let tiles = vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 0, 0]];
        let mask = compute_tile_mask(&tiles, 1, 1, Neighborhood::Edge4, false);
        assert_eq!(mask, 0, "no neighbors → mask 0");
    }

    #[test]
    fn mask_edge4_north_only() {
        let tiles = vec![vec![1], vec![1], vec![0]];
        // Cell (1,0): N=(0,0)=1, E=(1,1)=oob→0, S=(2,0)=0, W=(1,-1)=oob→0
        let mask = compute_tile_mask(&tiles, 1, 0, Neighborhood::Edge4, false);
        assert_eq!(mask, 1, "only north neighbor → mask 1");
    }

    #[test]
    fn mask_edge4_west_only() {
        let tiles = vec![vec![1, 1, 0]];
        // Cell (0,2): N=oob→0, E=oob→0, S=oob→0, W=(0,1)=1
        let mask = compute_tile_mask(&tiles, 0, 2, Neighborhood::Edge4, false);
        assert_eq!(mask, 8, "only west neighbor → mask 8");
    }

    #[test]
    fn mask_edge4_oob_filled_at_corner() {
        // Cell (0,0) with oob_filled=true: all 4 directions but E and S are real
        // oob=N(-1,0), oob=W(0,-1) → N=1, W=8; also E=(0,1), S=(1,0) depend on grid.
        let tiles = vec![vec![1, 0], vec![0, 0]];
        let mask_oob = compute_tile_mask(&tiles, 0, 0, Neighborhood::Edge4, true);
        // N=oob→1(bit1), E=(0,1)=0(bit0), S=(1,0)=0(bit0), W=oob→1(bit8)
        assert_eq!(mask_oob, 1 | 8, "oob treated as filled: N+W bits set");
        let mask_no_oob = compute_tile_mask(&tiles, 0, 0, Neighborhood::Edge4, false);
        assert_eq!(mask_no_oob, 0, "oob treated as empty: no bits set");
    }

    // ── compute_tile_mask / Blob8 ─────────────────────────────────────────────

    #[test]
    fn mask_blob8_interior_cell_fully_surrounded() {
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let mask = compute_tile_mask(&tiles, 1, 1, Neighborhood::Blob8, false);
        // All 8 neighbors filled → all bits valid (N+E both set for NE, etc.)
        assert_eq!(mask, 255, "fully surrounded interior → mask 255");
    }

    #[test]
    fn mask_blob8_diagonal_bit_zeroed_when_adjacent_edge_empty() {
        // NE diagonal should be zeroed if N is empty.
        // Grid: center (1,1) has E and NE neighbor filled but NOT N.
        let tiles = vec![
            vec![0, 0, 1], // row 0: NW=0, N=0, NE=1
            vec![0, 1, 1], // row 1: W=0, center(1,1)=1, E=1
            vec![0, 0, 0], // row 2: all empty
        ];
        let mask = compute_tile_mask(&tiles, 1, 1, Neighborhood::Blob8, false);
        // N=0 → NE bit must be 0, even though NE cell is filled.
        // E=1 (bit2), S=0, W=0, NE=0 (N missing), SE=0, SW=0, NW=0
        assert_eq!(mask & 16, 0, "NE bit must be 0 when N is empty");
        assert_eq!(mask & 2, 2, "E bit should be set");
    }

    // ── edge_16 identity ─────────────────────────────────────────────────────

    #[test]
    fn edge_16_identity_mapping() {
        let at = TilemapAutotile::edge_16(0);
        for m in 0u8..16 {
            assert_eq!(
                single_map(&at).get(&m).copied(),
                Some(m as u32),
                "edge_16(0): mask {m} should map to tile {m}"
            );
        }
        assert_eq!(single_map(&at).get(&7).copied(), Some(7));
    }

    #[test]
    fn edge_16_with_base_offset() {
        let at = TilemapAutotile::edge_16(100);
        assert_eq!(single_map(&at).get(&7).copied(), Some(107));
    }

    // ── Autotile reactive: center cell UV update + neighbor propagation ───────

    #[test]
    fn autotile_center_cell_uv_is_all_neighbors_mask() {
        // 3×3 fully-filled map with edge_16(0).
        // Center cell (1,1): all 4 neighbors filled → mask=15 → atlas tile 15.
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let atlas = TilemapAtlas::new("tiles.png", 16, 1); // 16 tiles in a 1-row strip
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, TilemapAutotile::edge_16(0));

        sys.run(&mut world, 0.0);

        // Find the tile entity at cell (1,1) — position (48, 48)
        let expected_uv = atlas.uv_for(15); // mask 15 → all neighbors
        let found = world
            .query::<crate::renderer::uv::UvRect>()
            .any(|(_, uv)| *uv == expected_uv);
        assert!(
            found,
            "center cell should have UV for mask=15 (all neighbors filled)"
        );
    }

    #[test]
    fn autotile_neighbor_propagation_updates_center_uv() {
        // 3×3 fully-filled map with edge_16(0). After initial spawn, remove a
        // neighbor of the center cell and verify the center's UvRect updates.
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let atlas = TilemapAtlas::new("tiles.png", 16, 1);
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, TilemapAutotile::edge_16(0));

        // Initial spawn: center (1,1) mask=15
        sys.run(&mut world, 0.0);

        // Find center tile entity (position 48,48 in a 32-pixel grid)
        // We identify it by its Transform position.
        use crate::components::Transform;
        use crate::ecs::Entity;
        let center_entity: Option<Entity> = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 48.0).abs() < 0.1 && (t.position.y - 48.0).abs() < 0.1)
            .map(|(e, _)| e);
        let center_entity = center_entity.expect("center tile entity must exist");

        // Remove north neighbor of center: cell (0,1)
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(0, 1, 0);
        sys.run(&mut world, 0.0);

        // Center (1,1) mask now: N=0, E=1, S=1, W=1 → mask = 2+4+8 = 14
        let expected_uv = atlas.uv_for(14);
        let actual_uv = world
            .get::<crate::renderer::uv::UvRect>(center_entity)
            .copied()
            .expect("center tile entity must still have UvRect");
        assert_eq!(
            actual_uv, expected_uv,
            "center cell UV must update to mask=14 after north neighbor removed"
        );
    }

    // ── compute_tile_mask_typed ───────────────────────────────────────────────

    #[test]
    fn mask_typed_all_same_terrain_gives_mask15() {
        // Grass (value 1) cell surrounded by all-grass neighbors → mask 15.
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let mask = compute_tile_mask_typed(&tiles, 1, 1, Neighborhood::Edge4, false, 1);
        assert_eq!(mask, 15, "all same-terrain neighbors → mask 15");
    }

    #[test]
    fn mask_typed_different_terrain_gives_mask0() {
        // Grass cell surrounded by all-water (value 2) neighbors → mask 0.
        let tiles = vec![vec![2, 2, 2], vec![2, 1, 2], vec![2, 2, 2]];
        let mask = compute_tile_mask_typed(&tiles, 1, 1, Neighborhood::Edge4, false, 1);
        assert_eq!(mask, 0, "all different-terrain neighbors → mask 0");
    }

    #[test]
    fn mask_typed_north_grass_east_south_west_water() {
        // Grass cell: N=grass, E=water, S=water, W=water → mask 1 (N only).
        let tiles = vec![
            vec![2, 1, 2], // row 0: ..N=1..
            vec![2, 1, 2], // row 1: W=2, center=1, E=2
            vec![2, 2, 2], // row 2: S=2
        ];
        let mask = compute_tile_mask_typed(&tiles, 1, 1, Neighborhood::Edge4, false, 1);
        assert_eq!(mask, 1, "only N is same terrain → mask 1");
    }

    // ── MultiTerrainAutotile::edge_16 ────────────────────────────────────────

    #[test]
    fn multi_terrain_edge_16_rule_lookups() {
        let mt = TilemapAutotile::multi_edge_16(&[(1, 0), (2, 16)]);

        // Terrain 1: base 0 → mask 7 maps to 7.
        let rule1 = mt.rule_for(1).expect("rule for terrain 1 must exist");
        assert_eq!(rule1.mask_to_tile.get(&7).copied(), Some(7));

        // Terrain 2: base 16 → mask 7 maps to 23.
        let rule2 = mt.rule_for(2).expect("rule for terrain 2 must exist");
        assert_eq!(rule2.mask_to_tile.get(&7).copied(), Some(23));

        // No rule for terrain 3.
        assert!(mt.rule_for(3).is_none(), "rule_for(3) must be None");
    }

    // ── MultiTerrainAutotile reactive system ─────────────────────────────────

    #[test]
    fn multi_terrain_center_grass_cell_has_full_mask_uv() {
        // 5×5 map: 3×3 grass patch (value 1) in the center, surrounded by water (2).
        // Grass patch spans rows 1–3, cols 1–3.
        let tiles = vec![
            vec![2, 2, 2, 2, 2],
            vec![2, 1, 1, 1, 2],
            vec![2, 1, 1, 1, 2],
            vec![2, 1, 1, 1, 2],
            vec![2, 2, 2, 2, 2],
        ];
        let atlas = make_multi_terrain_atlas();
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(
            map_e,
            TilemapAutotile::multi_edge_16(&[(1, 0), (2, 16)]).with_oob_filled(false),
        );
        sys.run(&mut world, 0.0);

        // Center grass cell: (2,2) — all 4 grass neighbors → mask 15 → atlas tile 15.
        let expected_center_uv = atlas.uv_for(15);
        // Center cell world pos: origin(0,0) + col*32+16, row*32+16 = (80, 80).
        use crate::components::Transform;
        let center_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 80.0).abs() < 0.1 && (t.position.y - 80.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("center grass tile entity at (2,2) must exist");
        let center_uv = world
            .get::<crate::renderer::uv::UvRect>(center_entity)
            .copied()
            .expect("center grass entity must have UvRect");
        assert_eq!(
            center_uv, expected_center_uv,
            "center grass cell (2,2): all grass neighbors → UV for mask 15"
        );

        // An edge grass cell, e.g. (1,2): N=water, E=grass, S=grass, W=grass
        // → same-terrain mask: N=0, E=1(bit2), S=1(bit4), W=1(bit8) → mask = 2+4+8 = 14.
        let expected_edge_uv = atlas.uv_for(14);
        // Edge cell (1,2) world pos: (80, 48).
        let edge_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 80.0).abs() < 0.1 && (t.position.y - 48.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("edge grass tile entity at (1,2) must exist");
        let edge_uv = world
            .get::<crate::renderer::uv::UvRect>(edge_entity)
            .copied()
            .expect("edge grass entity must have UvRect");
        assert_eq!(
            edge_uv, expected_edge_uv,
            "edge grass cell (1,2): only E/S/W grass → UV for mask 14"
        );

        // A water cell, e.g. (0,2): N=water(oob→0 with oob_filled=false), E=water, S=water(val2), W=water
        // Terrain-2 neighbors at (0,2): N=oob→false, E=(0,3)=2, S=(1,2)=1≠2(false), W=(0,1)=2
        // → mask for water: N=0, E=1(bit2), S=0, W=1(bit8) → mask = 2+8 = 10 → atlas tile 16+10=26.
        let expected_water_uv = atlas.uv_for(26);
        // (0,2) world pos: (80, 16).
        let water_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 80.0).abs() < 0.1 && (t.position.y - 16.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("water tile entity at (0,2) must exist");
        let water_uv = world
            .get::<crate::renderer::uv::UvRect>(water_entity)
            .copied()
            .expect("water entity must have UvRect");
        assert_eq!(
            water_uv, expected_water_uv,
            "water cell (0,2): E+W water neighbors → UV for mask 10 (base 16) = tile 26"
        );
    }

    #[test]
    fn multi_terrain_neighbor_propagation_on_set_tile() {
        // Start: 3×3 water (2) map. Set the center cell to grass (1). Run once.
        // Then set (0,1) to grass (1) — the cell north of center. Run again.
        // Assert: center (1,1) UV changes from mask=0 (no grass neighbors) to mask=1 (N=grass).
        let tiles = vec![vec![2, 2, 2], vec![2, 1, 2], vec![2, 2, 2]];
        let atlas = make_multi_terrain_atlas();
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(
            map_e,
            TilemapAutotile::multi_edge_16(&[(1, 0), (2, 16)]).with_oob_filled(false),
        );

        // Initial build: center grass at (1,1), all neighbors are water → grass mask=0 → atlas tile 0.
        sys.run(&mut world, 0.0);

        use crate::components::Transform;
        let center_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 48.0).abs() < 0.1 && (t.position.y - 48.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("center grass tile entity at (1,1) must exist");

        let uv_before = world
            .get::<crate::renderer::uv::UvRect>(center_entity)
            .copied()
            .expect("center grass entity must have UvRect");
        assert_eq!(
            uv_before,
            atlas.uv_for(0),
            "initially center grass has no grass neighbors → mask 0"
        );

        // Set north neighbor (0,1) to grass (1).
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(0, 1, 1);
        sys.run(&mut world, 0.0);

        // Now center (1,1): N=(0,1)=1=grass → mask N=1 → atlas tile 1.
        let uv_after = world
            .get::<crate::renderer::uv::UvRect>(center_entity)
            .copied()
            .expect("center grass entity must still have UvRect");
        assert_eq!(
            uv_after,
            atlas.uv_for(1),
            "after N neighbor becomes grass, center mask becomes 1 → atlas tile 1"
        );

        // Also verify the newly-placed grass cell (0,1) got its UV set.
        // (0,1) pos: (48, 16). Its grass neighbors: S=(1,1)=1 → mask=4 → atlas tile 4.
        let north_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 48.0).abs() < 0.1 && (t.position.y - 16.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("newly-placed grass tile at (0,1) must exist");
        let north_uv = world
            .get::<crate::renderer::uv::UvRect>(north_entity)
            .copied()
            .expect("must have UvRect");
        assert_eq!(
            north_uv,
            atlas.uv_for(4),
            "new north grass cell (0,1): only S grass neighbor → mask 4"
        );
    }

    #[test]
    fn multi_terrain_mode_resolves_via_terrain_base() {
        // A fully-filled single-terrain map under AutotileMode::Multi: terrain 1 → base 100, center
        // mask=15 → atlas tile 115. Confirms the Multi mode drives display-UV selection (not the
        // raw value, which would be tile 0 = value-1).
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let atlas = TilemapAtlas::new("tiles.png", 200, 1);
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, TilemapAutotile::multi_edge_16(&[(1, 100)]));
        sys.run(&mut world, 0.0);

        let expected_uv = atlas.uv_for(115);

        // Center cell (1,1) at world pos (48, 48).
        use crate::components::Transform;
        let center_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 48.0).abs() < 0.1 && (t.position.y - 48.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("center tile entity must exist");
        let center_uv = world
            .get::<crate::renderer::uv::UvRect>(center_entity)
            .copied()
            .expect("must have UvRect");

        assert_eq!(
            center_uv, expected_uv,
            "Multi mode: center (terrain 1, mask 15) should resolve to tile 115"
        );
    }

    // ── blob_47 VALID_MASKS correctness (Task 1) ─────────────────────────────

    /// Verify that VALID_MASKS contains exactly 47 entries and that every entry
    /// passes the blob validity check for the engine's bit convention
    /// (N=1, E=2, S=4, W=8, NE=16, SE=32, SW=64, NW=128).
    #[test]
    fn blob_47_valid_masks_count_and_validity() {
        let at = TilemapAutotile::blob_47(0);
        assert_eq!(
            single_map(&at).len(),
            47,
            "blob_47 must produce exactly 47 mask→tile entries"
        );

        // Every key in mask_to_tile must satisfy the blob validity rule.
        let is_valid_blob = |m: u8| -> bool {
            // NE(16) needs N(1)+E(2)
            if m & 16 != 0 && (m & (1 | 2)) != (1 | 2) {
                return false;
            }
            // SE(32) needs S(4)+E(2)
            if m & 32 != 0 && (m & (4 | 2)) != (4 | 2) {
                return false;
            }
            // SW(64) needs S(4)+W(8)
            if m & 64 != 0 && (m & (4 | 8)) != (4 | 8) {
                return false;
            }
            // NW(128) needs N(1)+W(8)
            if m & 128 != 0 && (m & (1 | 8)) != (1 | 8) {
                return false;
            }
            true
        };
        for &mask in single_map(&at).keys() {
            assert!(
                is_valid_blob(mask),
                "mask {mask} in blob_47 is not a valid reduced blob mask"
            );
        }
    }

    /// A small fully-filled blob map: every cell must map to a NON-zero atlas id
    /// (no spurious tile-0 fallback from a missing mask).
    #[test]
    fn blob_47_no_tile_zero_fallback_on_filled_map() {
        // 3×3 fully-filled map with blob_47(1): base atlas id = 1, so tile 0 is
        // never a valid result for any blob mask hit (the lowest is base+0 = 1).
        let tiles = vec![vec![1u32; 3]; 3];
        let atlas = TilemapAtlas::new("tiles.png", 47, 1);
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO);
        let at = TilemapAutotile::blob_47(1);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();
        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, at);
        sys.run(&mut world, 0.0);

        // All 9 non-zero cells should have spawned tile entities.
        // For each, the UvRect must NOT equal atlas.uv_for(0) (the tile-0 fallback).
        let fallback_uv = atlas.uv_for(0);
        for (_, uv) in world.query::<crate::renderer::uv::UvRect>() {
            assert_ne!(
                *uv, fallback_uv,
                "blob_47 must not fall back to atlas tile 0 for any reachable mask; \
                 got uv={uv:?} which equals tile-0 UV={fallback_uv:?}"
            );
        }
    }

    // ── compute_mask_raw DRY refactor equivalence (Task 2a) ──────────────────
    //
    // These tests assert that `compute_tile_mask` and `compute_tile_mask_typed`
    // continue to produce bit-identical results after being refactored to share
    // `compute_mask_raw`.

    /// When the only non-zero tile value IS the terrain, the two public functions
    /// must agree on the mask for every cell.
    #[test]
    fn compute_mask_raw_typed_and_untyped_agree_when_predicates_coincide() {
        // A 3×3 grid where every non-zero cell has value 1 (= terrain).
        // With terrain=1, `!= 0` and `== 1` are equivalent → same mask.
        let tiles = vec![vec![1u32, 1, 0], vec![0, 1, 1], vec![1, 0, 1]];
        for row in 0..3 {
            for col in 0..3 {
                for nb in [Neighborhood::Edge4, Neighborhood::Blob8] {
                    for oob in [false, true] {
                        let untyped = compute_tile_mask(&tiles, row, col, nb, oob);
                        let typed = compute_tile_mask_typed(&tiles, row, col, nb, oob, 1);
                        assert_eq!(
                            untyped, typed,
                            "compute_tile_mask and compute_tile_mask_typed must agree \
                             for row={row} col={col} nb={nb:?} oob={oob} (terrain=1, \
                             all non-zero cells are 1)"
                        );
                    }
                }
            }
        }
    }

    /// Hand-checked reference value: fully-surrounded Edge4 center cell gives mask 15
    /// for both functions (pre-refactor ground truth).
    #[test]
    fn compute_mask_raw_reference_values_unchanged() {
        let tiles = vec![vec![1u32, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        // Edge4 all-neighbors: N=1, E=2, S=4, W=8 → 15.
        assert_eq!(
            compute_tile_mask(&tiles, 1, 1, Neighborhood::Edge4, false),
            15,
            "Edge4 all-neighbors must give mask 15"
        );
        assert_eq!(
            compute_tile_mask_typed(&tiles, 1, 1, Neighborhood::Edge4, false, 1),
            15,
            "Edge4 typed all-neighbors must give mask 15"
        );
        // Blob8 all-neighbors (3×3 fully filled): all 8 bits → 255.
        assert_eq!(
            compute_tile_mask(&tiles, 1, 1, Neighborhood::Blob8, false),
            255,
            "Blob8 all-neighbors must give mask 255"
        );
        assert_eq!(
            compute_tile_mask_typed(&tiles, 1, 1, Neighborhood::Blob8, false, 1),
            255,
            "Blob8 typed all-neighbors must give mask 255"
        );
    }

    /// `compute_tile_mask` uses `!= 0` (any non-zero connects), while
    /// `compute_tile_mask_typed` uses `== terrain` (only same-value connects).
    /// Verify they DIFFER when the grid contains mixed non-zero values.
    #[test]
    fn compute_mask_raw_typed_differs_from_untyped_on_mixed_terrain() {
        // Grid where center (1,1) has N=2 (different terrain) and S=1 (same terrain).
        let tiles = vec![vec![0u32, 2, 0], vec![0, 1, 0], vec![0, 1, 0]];
        // Untyped (non-zero): N=1(bit1), S=1(bit4) → mask = 1 | 4 = 5.
        let untyped = compute_tile_mask(&tiles, 1, 1, Neighborhood::Edge4, false);
        assert_eq!(
            untyped, 5,
            "untyped: N(value=2,non-zero)=true + S(value=1)=true → 5"
        );
        // Typed terrain=1: N=2≠1→false, S=1=terrain→true → mask = 4.
        let typed = compute_tile_mask_typed(&tiles, 1, 1, Neighborhood::Edge4, false, 1);
        assert_eq!(
            typed, 4,
            "typed terrain=1: only S(value=1) connects → mask 4"
        );
        assert_ne!(
            untyped, typed,
            "the two functions must differ on mixed-terrain grids"
        );
    }

    // ── Hex6 (pointy-top, odd-r) ───────────────────────────────────────────────

    #[test]
    fn hex6_interior_cell_all_six_neighbors() {
        // 5×5 filled; cell (2,2) (even row) has all 6 hex neighbors in bounds.
        let tiles = vec![vec![1u32; 5]; 5];
        let mask = compute_tile_mask(&tiles, 2, 2, Neighborhood::Hex6, false);
        assert_eq!(mask, 63, "all 6 hex neighbors → mask 0b111111 = 63");
    }

    #[test]
    fn hex6_isolated_and_east_only() {
        let mut tiles = vec![vec![0u32; 5]; 5];
        tiles[2][2] = 1;
        assert_eq!(
            compute_tile_mask(&tiles, 2, 2, Neighborhood::Hex6, false),
            0,
            "no neighbors → mask 0"
        );
        tiles[2][3] = 1; // east neighbor
        assert_eq!(
            compute_tile_mask(&tiles, 2, 2, Neighborhood::Hex6, false),
            1,
            "only east neighbor → bit E=1"
        );
    }

    #[test]
    fn hex6_ne_offset_depends_on_row_parity() {
        // Odd row (1,1): NE neighbor is (0,2). Even row (2,1): NE neighbor is (1,1).
        let mut odd = vec![vec![0u32; 4]; 4];
        odd[1][1] = 1;
        odd[0][2] = 1; // NE of the odd-row cell
        assert_eq!(
            compute_tile_mask(&odd, 1, 1, Neighborhood::Hex6, false) & 4,
            4,
            "odd-row NE neighbor at (0,2) sets the NE bit"
        );
        let mut even = vec![vec![0u32; 4]; 4];
        even[2][1] = 1;
        even[1][1] = 1; // NE of the even-row cell
        assert_eq!(
            compute_tile_mask(&even, 2, 1, Neighborhood::Hex6, false) & 4,
            4,
            "even-row NE neighbor at (1,1) sets the NE bit"
        );
    }

    #[test]
    fn hex_6_constructor_identity_mapping() {
        let at = TilemapAutotile::hex_6(0);
        assert_eq!(at.neighborhood, Neighborhood::Hex6);
        assert_eq!(single_map(&at).len(), 64, "hex_6 covers all 64 masks");
        assert_eq!(single_map(&at).get(&63).copied(), Some(63));
        let at100 = TilemapAutotile::hex_6(100);
        assert_eq!(single_map(&at100).get(&5).copied(), Some(105));
    }

    // ── Hex6Flat (flat-top, odd-q) ─────────────────────────────────────────────

    #[test]
    fn hex6_flat_interior_cell_all_six_neighbors() {
        // 5×5 filled; cell (2,2) (even col) has all 6 flat-top hex neighbors in bounds.
        let tiles = vec![vec![1u32; 5]; 5];
        let mask = compute_tile_mask(&tiles, 2, 2, Neighborhood::Hex6Flat, false);
        assert_eq!(mask, 63, "all 6 flat-top hex neighbors → mask 63");
    }

    #[test]
    fn hex6_flat_ne_offset_depends_on_col_parity() {
        // Odd col (1,1): NE neighbor is (1,2). Even col (1,2): NE neighbor is (0,3).
        let mut odd = vec![vec![0u32; 4]; 4];
        odd[1][1] = 1;
        odd[1][2] = 1; // NE of the odd-col cell
        assert_eq!(
            compute_tile_mask(&odd, 1, 1, Neighborhood::Hex6Flat, false) & 4,
            4,
            "odd-col NE neighbor at (1,2) sets the NE bit"
        );
        let mut even = vec![vec![0u32; 4]; 4];
        even[1][2] = 1;
        even[0][3] = 1; // NE of the even-col cell
        assert_eq!(
            compute_tile_mask(&even, 1, 2, Neighborhood::Hex6Flat, false) & 4,
            4,
            "even-col NE neighbor at (0,3) sets the NE bit"
        );
    }

    #[test]
    fn hex_6_flat_constructor_uses_flat_neighborhood() {
        let at = TilemapAutotile::hex_6_flat(0);
        assert_eq!(at.neighborhood, Neighborhood::Hex6Flat);
        assert_eq!(single_map(&at).len(), 64);
    }

    // ── Isometric autotile (same square topology as orthographic) ───────────────

    #[test]
    fn isometric_autotile_uses_square_topology_like_ortho() {
        // Autotile masks come from the tiles[row][col] grid, which is identical for orthographic and
        // isometric (iso only changes rendering). So an iso map autotiles exactly like an ortho one:
        // a fully-filled 3×3 center cell still resolves to mask 15.
        use crate::tilemap::{Tilemap, TilemapProjection};
        let tiles = vec![vec![1u32; 3]; 3];
        let atlas = TilemapAtlas::new("tiles.png", 16, 1);
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, glam::Vec2::ZERO)
            .with_projection(TilemapProjection::Isometric);

        let mut world = World::new();
        let mut sys = crate::tilemap::TilemapSystem::new();
        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, TilemapAutotile::edge_16(0));
        sys.run(&mut world, 0.0);

        let expected_uv = atlas.uv_for(15); // center mask 15 (all 4 ortho neighbors)
        let found = world
            .query::<crate::renderer::uv::UvRect>()
            .any(|(_, uv)| *uv == expected_uv);
        assert!(
            found,
            "isometric autotile center cell should resolve to mask=15 (square topology)"
        );
    }
}
