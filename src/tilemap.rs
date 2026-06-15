use std::collections::HashMap;

use glam::Vec2;

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};
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
    pub tiles: Vec<Vec<u32>>,
    /// Side length of one tile (pixels).
    pub tile_size: f32,
    /// Top-left origin of the tilemap (world coordinates).
    pub origin: Vec2,
}

impl Tilemap {
    pub fn new(atlas: TilemapAtlas, tiles: Vec<Vec<u32>>, tile_size: f32, origin: Vec2) -> Self {
        Self {
            atlas,
            tiles,
            tile_size,
            origin,
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
        true
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

// ─── Autotiling ───────────────────────────────────────────────────────────────

/// Which set of neighbors to use when computing the autotile bitmask.
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
}

/// Connection rule for autotiling.
///
/// Currently a unit struct (marker): any non-zero cell connects to any other
/// non-zero cell. Kept as an extension point for future per-tile-type connection
/// rules (e.g. "grass only connects to grass").
#[derive(Debug, Clone, Copy, Default)]
pub struct ConnectRule;

/// Component that makes [`TilemapSystem`] choose tile display UVs based on
/// neighbor connectivity rather than the raw tile value.
///
/// Attach this component to the same entity as [`Tilemap`].
///
/// # Example
///
/// ```rust,ignore
/// world.add_component(map_entity, TilemapAutotile::edge_16(0));
/// ```
pub struct TilemapAutotile {
    /// Which neighborhood scheme to use.
    pub neighborhood: Neighborhood,
    /// Computed bitmask → 0-based atlas display id.
    pub mask_to_tile: HashMap<u8, u32>,
    /// Treat out-of-bounds neighbors as filled (`true`) or empty (`false`).
    pub oob_filled: bool,
    /// Connection rule (v1: any non-zero connects to any non-zero).
    pub connect: ConnectRule,
}

impl TilemapAutotile {
    /// 16-tile edge autotile layout.
    ///
    /// Assumes a contiguous 16-tile strip in the atlas starting at `base_atlas_id`
    /// where tile `base_atlas_id + mask` corresponds to bitmask `mask` (N=1,E=2,S=4,W=8).
    pub fn edge_16(base_atlas_id: u32) -> Self {
        let mask_to_tile = (0u8..16).map(|m| (m, base_atlas_id + m as u32)).collect();
        Self {
            neighborhood: Neighborhood::Edge4,
            mask_to_tile,
            oob_filled: false,
            connect: ConnectRule,
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
        // The 47 canonical reduced blob masks, sorted ascending.
        // A mask is valid if every set diagonal bit has both adjacent orthogonal bits set.
        // Adjacent pairs: NE(16) needs N(1)+E(2); SE(32) needs S(4)+E(2);
        //                 SW(64) needs S(4)+W(8); NW(128) needs N(1)+W(8).
        const VALID_MASKS: &[u8] = &[
            0, 2, 8, 10, 11, 16, 18, 22, 24, 26, 27, 30, 31, 64, 66, 72, 74, 75, 80, 82, 86, 88,
            90, 91, 94, 95, 104, 106, 107, 120, 122, 123, 126, 127, 208, 210, 214, 216, 218, 219,
            222, 223, 248, 250, 251, 254, 255,
        ];
        let mask_to_tile = VALID_MASKS
            .iter()
            .enumerate()
            .map(|(i, &m)| (m, base_atlas_id + i as u32))
            .collect();
        Self {
            neighborhood: Neighborhood::Blob8,
            mask_to_tile,
            oob_filled: false,
            connect: ConnectRule,
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
    let filled = |r: i32, c: i32| -> bool {
        let row_count = tiles.len() as i32;
        if r < 0 || r >= row_count {
            return oob_filled;
        }
        let row_ref = &tiles[r as usize];
        let col_count = row_ref.len() as i32;
        if c < 0 || c >= col_count {
            return oob_filled;
        }
        row_ref[c as usize] != 0
    };

    let r = row as i32;
    let c = col as i32;

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

/// Multi-terrain autotile component. Attach to the same entity as the [`Tilemap`]
/// **instead of** [`TilemapAutotile`]. Each non-zero cell autotiles using the rule
/// whose `terrain` equals the cell's value, connecting only to same-value neighbors.
///
/// When both [`TilemapAutotile`] and [`MultiTerrainAutotile`] are present on the
/// same entity, `MultiTerrainAutotile` takes precedence.
///
/// # Example
///
/// ```rust,ignore
/// // Grass (value 1) uses atlas tiles 0–15; water (value 2) uses tiles 16–31.
/// world.add_component(map_entity, MultiTerrainAutotile::edge_16(&[(1, 0), (2, 16)]));
/// ```
#[derive(Debug, Clone)]
pub struct MultiTerrainAutotile {
    /// Which neighborhood scheme to use.
    pub neighborhood: Neighborhood,
    /// Treat out-of-bounds neighbors as filled (`true`) or empty (`false`).
    pub oob_filled: bool,
    /// Per-terrain rules. Each entry handles one distinct tile value.
    pub rules: Vec<TerrainRule>,
}

impl MultiTerrainAutotile {
    /// 16-tile edge layout per terrain.
    ///
    /// `terrains` is a slice of `(terrain_value, base_atlas_id)` pairs. Each terrain
    /// gets a `mask_to_tile` mapping `m → base_atlas_id + m` for `m` in `0..16`,
    /// matching a contiguous 16-tile edge strip in the atlas.
    pub fn edge_16(terrains: &[(u32, u32)]) -> Self {
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
            rules,
        }
    }

    /// Sets [`oob_filled`](Self::oob_filled) and returns `self` (builder style).
    pub fn with_oob_filled(mut self, v: bool) -> Self {
        self.oob_filled = v;
        self
    }

    /// Returns the rule for the given terrain value, or `None` if no rule exists.
    fn rule_for(&self, terrain: u32) -> Option<&TerrainRule> {
        self.rules.iter().find(|r| r.terrain == terrain)
    }
}

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
    let filled = |r: i32, c: i32| -> bool {
        let row_count = tiles.len() as i32;
        if r < 0 || r >= row_count {
            return oob_filled;
        }
        let row_ref = &tiles[r as usize];
        let col_count = row_ref.len() as i32;
        if c < 0 || c >= col_count {
            return oob_filled;
        }
        row_ref[c as usize] == terrain
    };

    let r = row as i32;
    let c = col as i32;

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
        if filled(r - 1, c + 1) && n && e {
            mask |= 16;
        }
        if filled(r + 1, c + 1) && s && e {
            mask |= 32;
        }
        if filled(r + 1, c - 1) && s && w {
            mask |= 64;
        }
        if filled(r - 1, c - 1) && n && w {
            mask |= 128;
        }
    }

    mask
}

// ─── System internals ─────────────────────────────────────────────────────────

/// Per-tilemap-entity view tracked by [`TilemapSystem`].
struct TilemapView {
    /// (row, col) → tile entity
    cells: HashMap<(usize, usize), Entity>,
    /// Last-seen tile grid (for diffing)
    cached_tiles: Vec<Vec<u32>>,
    /// Last-seen dimensions
    cached_dims: (usize, usize),
}

// ─── Systems ──────────────────────────────────────────────────────────────────

/// System that reads Tilemap components and manages tile entities.
///
/// - Spawns tile entities the first time a Tilemap entity is seen.
/// - Reacts to tile mutations each frame: despawns removed tiles, spawns new ones,
///   and updates UvRect in place for changed tiles.
/// - Despawns all tile entities when the Tilemap entity disappears.
/// - When a [`TilemapAutotile`] component is present, chooses display UVs based on
///   neighbor connectivity and propagates UV updates to the 8 surrounding cells when
///   any cell changes.
pub struct TilemapSystem {
    views: HashMap<Entity, TilemapView>,
}

impl TilemapSystem {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }
}

impl Default for TilemapSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TilemapSystem {
    /// Schedule label for ordering via `add_system_labeled`.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::tilemap";
}

/// Chooses the display UV for a non-zero cell.
///
/// When `autotile` is `None`, falls back to `tilemap.atlas.uv_for(value - 1)`.
/// When `Some`, computes the neighbor bitmask and looks up the display atlas id.
///
/// This is the clean seam for autotiling: [`TilemapSystem`] delegates all UV
/// resolution through this function so future connection-rule changes live here.
pub fn cell_display_uv(
    tilemap: &Tilemap,
    autotile: Option<&TilemapAutotile>,
    row: usize,
    col: usize,
    value: u32,
) -> UvRect {
    match autotile {
        None => tilemap.atlas.uv_for(value - 1),
        Some(at) => {
            let mask = compute_tile_mask(&tilemap.tiles, row, col, at.neighborhood, at.oob_filled);
            let display_id = at.mask_to_tile.get(&mask).copied().unwrap_or(0);
            tilemap.atlas.uv_for(display_id)
        }
    }
}

/// Spawns a single tile entity for cell `(row, col)` with the given UV.
fn spawn_tile_entity(
    world: &mut World,
    tm: &Tilemap,
    row: usize,
    col: usize,
    uv: UvRect,
) -> Entity {
    let x = tm.origin.x + col as f32 * tm.tile_size + tm.tile_size * 0.5;
    let y = tm.origin.y + row as f32 * tm.tile_size + tm.tile_size * 0.5;
    let tile_entity = world.spawn();
    world.add_component(
        tile_entity,
        Transform {
            position: Vec2::new(x, y),
            scale: Vec2::splat(tm.tile_size),
            rotation: 0.0,
            z: -1.0,
        },
    );
    world.add_component(tile_entity, Sprite::textured(tm.atlas.texture.as_str()));
    world.add_component(tile_entity, uv);
    tile_entity
}

impl System for TilemapSystem {
    // `clippy::map_entry` is suppressed here because the "first-time" branch
    // calls `world.spawn`/`world.add_component` (mutable World borrows) between
    // the `contains_key` check and the eventual `self.views.insert`.  The entry
    // API would require holding a mutable borrow on `self.views` across those
    // world mutations, which the borrow checker disallows.
    #[allow(clippy::map_entry)]
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Step 1: collect alive tilemap entities (avoids holding borrow) ──────
        let tilemap_entities: Vec<Entity> = world.query::<Tilemap>().map(|(e, _)| e).collect();

        // ── Step 2: despawn views for disappeared tilemap entities ────────────
        let removed: Vec<Entity> = self
            .views
            .keys()
            .filter(|e| !tilemap_entities.contains(e))
            .copied()
            .collect();
        for map_entity in removed {
            if let Some(view) = self.views.remove(&map_entity) {
                for (_, tile_e) in view.cells {
                    world.despawn(tile_e);
                }
            }
        }

        // ── Step 3: process each alive tilemap entity ──────────────────────────
        for map_entity in tilemap_entities {
            // Clone out the data we need before any mutation.
            // Precedence: MultiTerrainAutotile > TilemapAutotile > plain.
            enum AutotileMode {
                None,
                Single {
                    nb: Neighborhood,
                    oob: bool,
                    m2t: HashMap<u8, u32>,
                },
                Multi(MultiTerrainAutotile),
            }

            let (tm_clone, autotile_mode) = {
                let tm = world.get::<Tilemap>(map_entity).unwrap();
                let mode = if let Some(mt) = world.get::<MultiTerrainAutotile>(map_entity) {
                    AutotileMode::Multi(mt.clone())
                } else if let Some(at) = world.get::<TilemapAutotile>(map_entity) {
                    AutotileMode::Single {
                        nb: at.neighborhood,
                        oob: at.oob_filled,
                        m2t: at.mask_to_tile.clone(),
                    }
                } else {
                    AutotileMode::None
                };
                (tm.clone(), mode)
            };

            // Whether any autotile mode is active (controls neighbor UV refresh).
            let any_autotile = !matches!(autotile_mode, AutotileMode::None);

            // Helper: resolve UV for a non-zero cell.
            let resolve_uv = |row: usize, col: usize, value: u32| -> UvRect {
                match &autotile_mode {
                    AutotileMode::None => tm_clone.atlas.uv_for(value - 1),
                    AutotileMode::Single { nb, oob, m2t } => {
                        let mask = compute_tile_mask(&tm_clone.tiles, row, col, *nb, *oob);
                        let display_id = m2t.get(&mask).copied().unwrap_or(0);
                        tm_clone.atlas.uv_for(display_id)
                    }
                    AutotileMode::Multi(mt) => {
                        if let Some(rule) = mt.rule_for(value) {
                            let mask = compute_tile_mask_typed(
                                &tm_clone.tiles,
                                row,
                                col,
                                mt.neighborhood,
                                mt.oob_filled,
                                value,
                            );
                            let display_id = rule.mask_to_tile.get(&mask).copied().unwrap_or(0);
                            tm_clone.atlas.uv_for(display_id)
                        } else {
                            // No rule for this terrain value: plain non-autotiled mapping.
                            tm_clone.atlas.uv_for(value - 1)
                        }
                    }
                }
            };

            if !self.views.contains_key(&map_entity) {
                // ── First time seen: full build ────────────────────────────────
                let dims = tm_clone.dims();
                let mut cells: HashMap<(usize, usize), Entity> = HashMap::new();

                for (row_idx, row) in tm_clone.tiles.iter().enumerate() {
                    for (col_idx, &tile_value) in row.iter().enumerate() {
                        if tile_value == 0 {
                            continue;
                        }
                        let uv = resolve_uv(row_idx, col_idx, tile_value);
                        let tile_e = spawn_tile_entity(world, &tm_clone, row_idx, col_idx, uv);
                        cells.insert((row_idx, col_idx), tile_e);
                    }
                }

                self.views.insert(
                    map_entity,
                    TilemapView {
                        cells,
                        cached_tiles: tm_clone.tiles.clone(),
                        cached_dims: dims,
                    },
                );
            } else {
                // ── Already seen: diff and react ───────────────────────────────
                let current_dims = tm_clone.dims();
                let cached_dims = self.views[&map_entity].cached_dims;

                if current_dims != cached_dims {
                    // Dimension changed — full rebuild.
                    let old_cells: Vec<Entity> = self
                        .views
                        .get_mut(&map_entity)
                        .unwrap()
                        .cells
                        .drain()
                        .map(|(_, e)| e)
                        .collect();
                    for e in old_cells {
                        world.despawn(e);
                    }

                    let mut cells: HashMap<(usize, usize), Entity> = HashMap::new();
                    for (row_idx, row) in tm_clone.tiles.iter().enumerate() {
                        for (col_idx, &tile_value) in row.iter().enumerate() {
                            if tile_value == 0 {
                                continue;
                            }
                            let uv = resolve_uv(row_idx, col_idx, tile_value);
                            let tile_e = spawn_tile_entity(world, &tm_clone, row_idx, col_idx, uv);
                            cells.insert((row_idx, col_idx), tile_e);
                        }
                    }

                    let view = self.views.get_mut(&map_entity).unwrap();
                    view.cells = cells;
                    view.cached_tiles = tm_clone.tiles.clone();
                    view.cached_dims = current_dims;
                } else {
                    // Same dimensions: cell-level diff.
                    // Collect changed cells first (borrow checker: we need view after).
                    let cached_clone = self.views[&map_entity].cached_tiles.clone();

                    // Find changed cells.
                    let mut changed_cells: Vec<(usize, usize, u32, u32)> = Vec::new(); // (row, col, old, new)
                    for (r, row) in tm_clone.tiles.iter().enumerate() {
                        for (c, &new_val) in row.iter().enumerate() {
                            let old_val = cached_clone
                                .get(r)
                                .and_then(|rr| rr.get(c))
                                .copied()
                                .unwrap_or(0);
                            if old_val != new_val {
                                changed_cells.push((r, c, old_val, new_val));
                            }
                        }
                    }

                    if changed_cells.is_empty() {
                        continue;
                    }

                    // Determine which cells need a UV refresh (changed + neighbors when autotile is active).
                    let mut uv_refresh: Vec<(usize, usize)> = Vec::new();

                    for &(r, c, old_val, new_val) in &changed_cells {
                        let view = self.views.get_mut(&map_entity).unwrap();

                        if old_val != 0 && new_val == 0 {
                            // Non-zero → zero: despawn tile entity.
                            if let Some(tile_e) = view.cells.remove(&(r, c)) {
                                world.despawn(tile_e);
                            }
                        } else if old_val == 0 && new_val != 0 {
                            // Zero → non-zero: spawn new tile entity.
                            let uv = resolve_uv(r, c, new_val);
                            let tile_e = spawn_tile_entity(world, &tm_clone, r, c, uv);
                            view.cells.insert((r, c), tile_e);
                        } else if old_val != 0 && new_val != 0 {
                            // Non-zero → different non-zero: update UvRect in place.
                            let uv = resolve_uv(r, c, new_val);
                            if let Some(&tile_e) = view.cells.get(&(r, c)) {
                                if let Some(uv_comp) = world.get_mut::<UvRect>(tile_e) {
                                    *uv_comp = uv;
                                }
                            } else {
                                // Safety spawn (shouldn't happen in normal flow).
                                let tile_e = spawn_tile_entity(world, &tm_clone, r, c, uv);
                                view.cells.insert((r, c), tile_e);
                            }
                        }

                        // When autotile is active, neighbors also need UV refresh.
                        if any_autotile {
                            let (rows, cols) = current_dims;
                            for dr in -1i32..=1 {
                                for dc in -1i32..=1 {
                                    if dr == 0 && dc == 0 {
                                        continue;
                                    }
                                    let nr = r as i32 + dr;
                                    let nc = c as i32 + dc;
                                    if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                                        uv_refresh.push((nr as usize, nc as usize));
                                    }
                                }
                            }
                        }
                    }

                    // Apply neighbor UV refreshes (only for non-zero cells that have entities).
                    for (nr, nc) in uv_refresh {
                        let current_val = tm_clone
                            .tiles
                            .get(nr)
                            .and_then(|r| r.get(nc))
                            .copied()
                            .unwrap_or(0);
                        if current_val == 0 {
                            continue;
                        }
                        let uv = resolve_uv(nr, nc, current_val);
                        let view = self.views.get_mut(&map_entity).unwrap();
                        if let Some(&tile_e) = view.cells.get(&(nr, nc)) {
                            if let Some(uv_comp) = world.get_mut::<UvRect>(tile_e) {
                                *uv_comp = uv;
                            }
                        }
                    }

                    // Update cached tiles.
                    let view = self.views.get_mut(&map_entity).unwrap();
                    view.cached_tiles = tm_clone.tiles.clone();
                    view.cached_dims = current_dims;
                }
            }
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

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

    // ── Reactive TilemapSystem ────────────────────────────────────────────────

    fn run_tilemap_system(world: &mut World, sys: &mut TilemapSystem) {
        sys.run(world, 0.0);
    }

    #[test]
    fn tilemap_system_spawns_one_entity_per_non_zero_cell() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        // 2×2 map: 3 non-zero cells
        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 0], vec![2, 3]]));
        run_tilemap_system(&mut world, &mut sys);

        let count = world.query::<UvRect>().count();
        assert_eq!(
            count, 3,
            "should spawn 3 tile entities for 3 non-zero cells"
        );
    }

    #[test]
    fn tilemap_system_reactive_despawn_on_zero() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 2], vec![3, 4]]));
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 4, "initial: 4 tiles");

        // Erase cell (0,0)
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(0, 0, 0);
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 3, "after erase: 3 tiles");
    }

    #[test]
    fn tilemap_system_reactive_spawn_on_fill() {
        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, make_tilemap(vec![vec![1, 0], vec![0, 0]]));
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 1, "initial: 1 tile");

        // Fill cell (1,1)
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(1, 1, 2);
        run_tilemap_system(&mut world, &mut sys);
        assert_eq!(world.query::<UvRect>().count(), 2, "after fill: 2 tiles");
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
                at.mask_to_tile.get(&m).copied(),
                Some(m as u32),
                "edge_16(0): mask {m} should map to tile {m}"
            );
        }
        assert_eq!(at.mask_to_tile.get(&7).copied(), Some(7));
    }

    #[test]
    fn edge_16_with_base_offset() {
        let at = TilemapAutotile::edge_16(100);
        assert_eq!(at.mask_to_tile.get(&7).copied(), Some(107));
    }

    // ── Autotile reactive: center cell UV update + neighbor propagation ───────

    #[test]
    fn autotile_center_cell_uv_is_all_neighbors_mask() {
        // 3×3 fully-filled map with edge_16(0).
        // Center cell (1,1): all 4 neighbors filled → mask=15 → atlas tile 15.
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let atlas = TilemapAtlas::new("tiles.png", 16, 1); // 16 tiles in a 1-row strip
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, Vec2::ZERO);

        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, TilemapAutotile::edge_16(0));

        sys.run(&mut world, 0.0);

        // Find the tile entity at cell (1,1) — position (48, 48)
        let expected_uv = atlas.uv_for(15); // mask 15 → all neighbors
        let found = world.query::<UvRect>().any(|(_, uv)| *uv == expected_uv);
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
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, Vec2::ZERO);

        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(map_e, TilemapAutotile::edge_16(0));

        // Initial spawn: center (1,1) mask=15
        sys.run(&mut world, 0.0);

        // Find center tile entity (position 48,48 in a 32-pixel grid)
        // We identify it by its Transform position.
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
            .get::<UvRect>(center_entity)
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
        let mt = MultiTerrainAutotile::edge_16(&[(1, 0), (2, 16)]);

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

    fn make_multi_terrain_atlas() -> TilemapAtlas {
        // 32 tiles in a 32×1 strip: grass tiles 0–15, water tiles 16–31.
        TilemapAtlas::new("tiles.png", 32, 1)
    }

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
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, Vec2::ZERO);

        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(
            map_e,
            MultiTerrainAutotile::edge_16(&[(1, 0), (2, 16)]).with_oob_filled(false),
        );
        run_tilemap_system(&mut world, &mut sys);

        // Center grass cell: (2,2) — all 4 grass neighbors → mask 15 → atlas tile 15.
        let expected_center_uv = atlas.uv_for(15);
        // Center cell world pos: origin(0,0) + col*32+16, row*32+16 = (80, 80).
        let center_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 80.0).abs() < 0.1 && (t.position.y - 80.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("center grass tile entity at (2,2) must exist");
        let center_uv = world
            .get::<UvRect>(center_entity)
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
            .get::<UvRect>(edge_entity)
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
            .get::<UvRect>(water_entity)
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
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, Vec2::ZERO);

        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        world.add_component(
            map_e,
            MultiTerrainAutotile::edge_16(&[(1, 0), (2, 16)]).with_oob_filled(false),
        );

        // Initial build: center grass at (1,1), all neighbors are water → grass mask=0 → atlas tile 0.
        run_tilemap_system(&mut world, &mut sys);

        let center_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 48.0).abs() < 0.1 && (t.position.y - 48.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("center grass tile entity at (1,1) must exist");

        let uv_before = world
            .get::<UvRect>(center_entity)
            .copied()
            .expect("center grass entity must have UvRect");
        assert_eq!(
            uv_before,
            atlas.uv_for(0),
            "initially center grass has no grass neighbors → mask 0"
        );

        // Set north neighbor (0,1) to grass (1).
        world.get_mut::<Tilemap>(map_e).unwrap().set_tile(0, 1, 1);
        run_tilemap_system(&mut world, &mut sys);

        // Now center (1,1): N=(0,1)=1=grass → mask N=1 → atlas tile 1.
        let uv_after = world
            .get::<UvRect>(center_entity)
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
            .get::<UvRect>(north_entity)
            .copied()
            .expect("must have UvRect");
        assert_eq!(
            north_uv,
            atlas.uv_for(4),
            "new north grass cell (0,1): only S grass neighbor → mask 4"
        );
    }

    #[test]
    fn multi_terrain_takes_precedence_over_single_autotile() {
        // Entity with BOTH TilemapAutotile and MultiTerrainAutotile: multi wins.
        // Single-terrain edge_16(0): for a fully-filled map, center mask=15 → atlas tile 15.
        // Multi-terrain edge_16(&[(1, 100)]): center mask=15 → atlas tile 115 (100+15).
        // If multi takes precedence, UV matches atlas tile 115, not 15.
        let tiles = vec![vec![1, 1, 1], vec![1, 1, 1], vec![1, 1, 1]];
        let atlas = TilemapAtlas::new("tiles.png", 200, 1);
        let tm = Tilemap::new(atlas.clone(), tiles, 32.0, Vec2::ZERO);

        let mut world = World::new();
        let mut sys = TilemapSystem::new();

        let map_e = world.spawn();
        world.add_component(map_e, tm);
        // Single-terrain: center → tile 15.
        world.add_component(map_e, TilemapAutotile::edge_16(0));
        // Multi-terrain: center → tile 115 (100 + 15).
        world.add_component(map_e, MultiTerrainAutotile::edge_16(&[(1, 100)]));
        run_tilemap_system(&mut world, &mut sys);

        let expected_multi_uv = atlas.uv_for(115);
        let expected_single_uv = atlas.uv_for(15);

        // Center cell (1,1) at world pos (48, 48).
        let center_entity = world
            .query::<Transform>()
            .find(|(_, t)| (t.position.x - 48.0).abs() < 0.1 && (t.position.y - 48.0).abs() < 0.1)
            .map(|(e, _)| e)
            .expect("center tile entity must exist");
        let center_uv = world
            .get::<UvRect>(center_entity)
            .copied()
            .expect("must have UvRect");

        assert_eq!(
            center_uv, expected_multi_uv,
            "MultiTerrainAutotile must take precedence: center should be tile 115"
        );
        assert_ne!(
            center_uv, expected_single_uv,
            "single-terrain result (tile 15) must NOT be used when multi is present"
        );
    }
}
