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
mod tests;
