//! Procedural map generation — two deterministic generators over a shared [`DungeonMap`] grid:
//! **BSP rooms + corridors** ([`generate_bsp_dungeon`]) and **cellular-automata caves**
//! ([`generate_cellular_cave`]).
//!
//! [`generate_bsp_dungeon`] carves a roomy dungeon into a grid: it recursively splits the map into
//! partitions, cuts a room into each leaf, and connects sibling partitions with L-shaped corridors
//! while unwinding the recursion — so the whole dungeon is **guaranteed connected** (every room is
//! reachable from every other). Generation is **deterministic**: the same `seed` + [`DungeonParams`]
//! always produce the identical [`DungeonMap`], so a game can store just the seed and regenerate the
//! level, or reproduce a run exactly.
//!
//! [`generate_cellular_cave`] grows an **organic cave** instead: it seeds the interior with random
//! rock, runs a few cellular-automata smoothing passes (the classic "4-5 rule"), then keeps only the
//! largest connected cavern — filling every smaller pocket with wall — so, like the BSP dungeon, the
//! result is **guaranteed connected**. It is deterministic and seeded the same way, and returns the
//! same [`DungeonMap`] type, so everything below (`to_path_grid` / `to_tilemap_tiles` /
//! `FovMap::from_path_grid`) composes with either generator. A cave records a single 1×1 [`Room`] at
//! the cavern's most-central cell, so [`first_room_center`](DungeonMap::first_room_center) is a valid
//! spawn for both.
//!
//! The result is a plain owned grid (like [`PathGrid`] /
//! [`FovMap`](crate::fov::FovMap) — not an ECS component): read [`is_floor`](DungeonMap::is_floor) /
//! [`is_wall`](DungeonMap::is_wall) to render or collide, [`rooms`](DungeonMap::rooms) for spawn
//! placement, [`to_tilemap_tiles`](DungeonMap::to_tilemap_tiles) to build a
//! [`Tilemap`](crate::tilemap::Tilemap), and [`to_path_grid`](DungeonMap::to_path_grid) to get a
//! `PathGrid` for enemy pathfinding *and* — via [`FovMap::from_path_grid`](crate::fov::FovMap::from_path_grid)
//! — field-of-view, all from the same layout (see the `roguelike` example).
//!
//! ```
//! use engine::{generate_bsp_dungeon, generate_cellular_cave, CaveParams, DungeonParams, Tile};
//!
//! let a = generate_bsp_dungeon(48, 32, 1234, &DungeonParams::default());
//! let b = generate_bsp_dungeon(48, 32, 1234, &DungeonParams::default());
//! assert_eq!(a, b, "same seed → identical dungeon");
//! assert!(!a.rooms.is_empty());
//! // The border is always solid wall.
//! assert_eq!(a.tile(0, 0), Tile::Wall);
//!
//! // The cave generator is deterministic too, and its spawn cell is walkable floor.
//! let cave = generate_cellular_cave(48, 32, 1234, &CaveParams::default());
//! assert_eq!(cave, generate_cellular_cave(48, 32, 1234, &CaveParams::default()));
//! let spawn = cave.first_room_center().unwrap();
//! assert!(cave.is_floor(spawn.x, spawn.y));
//! ```

use std::collections::VecDeque;

use glam::IVec2;

use crate::pathfinding::{PathGrid, MAX_PATH_GRID_CELLS};
use crate::rng::Rng;

/// A single dungeon cell: solid `Wall` or walkable `Floor`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tile {
    /// Solid, blocks movement and sight.
    Wall,
    /// Open, walkable.
    Floor,
}

/// A carved rectangular room. Its cells `x..x+w`, `y..y+h` are all [`Tile::Floor`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Room {
    /// Left edge (cell x).
    pub x: i32,
    /// Top edge (cell y).
    pub y: i32,
    /// Width in cells.
    pub w: i32,
    /// Height in cells.
    pub h: i32,
}

impl Room {
    /// The room's center cell (used as the corridor endpoint / a spawn point).
    pub fn center(&self) -> IVec2 {
        IVec2::new(self.x + self.w / 2, self.y + self.h / 2)
    }

    /// Whether `(px, py)` is inside the room.
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// Tunables for [`generate_bsp_dungeon`]. [`Default`] suits a ~48×32 dungeon.
#[derive(Clone, Debug)]
pub struct DungeonParams {
    /// A partition smaller than `2 × min_leaf` in a dimension is never split along it — the floor on
    /// room size + surrounding wall.
    pub min_leaf: i32,
    /// A partition larger than this (in either dimension) is *always* split, for variety.
    pub max_leaf: i32,
    /// Minimum room width/height in cells.
    pub min_room: i32,
    /// Gap in cells kept between a room and its partition edge (keeps rooms apart).
    pub room_margin: i32,
    /// Hard cap on BSP recursion depth (a backstop; `min_leaf` already bounds it).
    pub max_depth: i32,
}

impl Default for DungeonParams {
    fn default() -> Self {
        Self {
            min_leaf: 8,
            max_leaf: 18,
            min_room: 4,
            room_margin: 1,
            max_depth: 5,
        }
    }
}

/// Tunables for [`generate_cellular_cave`]. [`Default`] (45% initial rock, 4 smoothing passes, the
/// classic "5" rule) suits a ~48×32 – 64×48 cave.
#[derive(Clone, Debug)]
pub struct CaveParams {
    /// Probability each interior cell **starts** as wall in the initial random fill (before
    /// smoothing), clamped to `0.0..=1.0`. The classic value is `0.45`; higher = more rock and
    /// smaller caverns, lower = more open.
    pub initial_wall_prob: f32,
    /// Number of cellular-automata smoothing passes. More passes = smoother, blobbier caverns with
    /// fewer stray specks. `0` leaves the raw random fill (then keep-largest still runs).
    pub steps: u32,
    /// Birth threshold: a **floor** cell turns to wall in a smoothing pass when at least this many
    /// of its 8 (Moore) neighbors are wall; an **existing wall** survives with one fewer
    /// (`wall_threshold - 1`). Out-of-bounds counts as wall. The classic "4-5 rule" uses `5`.
    pub wall_threshold: u32,
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            initial_wall_prob: 0.45,
            steps: 4,
            wall_threshold: 5,
        }
    }
}

/// A generated dungeon: a row-major grid of [`Tile`]s plus the list of carved [`Room`]s.
///
/// `(x, y)` addresses with `x` in `0..width`, `y` in `0..height`. Build one with
/// [`generate_bsp_dungeon`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DungeonMap {
    /// Grid width in cells.
    pub width: i32,
    /// Grid height in cells.
    pub height: i32,
    /// The carved rooms, in generation order (`rooms[0]` is a good default spawn).
    pub rooms: Vec<Room>,
    tiles: Vec<Tile>,
}

impl DungeonMap {
    /// A solid-wall map of `width × height`. A non-positive dimension — or a size exceeding
    /// [`MAX_PATH_GRID_CELLS`](crate::pathfinding::MAX_PATH_GRID_CELLS) / overflowing — collapses to
    /// an empty `0 × 0` map with a logged error (mirrors `PathGrid`/`FovMap`).
    fn new_walls(width: i32, height: i32) -> Self {
        let (width, height, size) = if width > 0 && height > 0 {
            match (width as usize).checked_mul(height as usize) {
                Some(size) if size <= MAX_PATH_GRID_CELLS => (width, height, size),
                _ => {
                    log::error!(
                        "DungeonMap {width}×{height} exceeds MAX_PATH_GRID_CELLS \
                         ({MAX_PATH_GRID_CELLS}) or overflows; creating an EMPTY map."
                    );
                    (0, 0, 0)
                }
            }
        } else {
            (0, 0, 0)
        };
        Self {
            width,
            height,
            rooms: Vec::new(),
            tiles: vec![Tile::Wall; size],
        }
    }

    /// The tile at `(x, y)`. Out-of-bounds reads [`Tile::Wall`] (everything outside the map is solid).
    pub fn tile(&self, x: i32, y: i32) -> Tile {
        self.index(x, y)
            .map(|i| self.tiles[i])
            .unwrap_or(Tile::Wall)
    }

    /// Whether `(x, y)` is walkable floor. Out-of-bounds is `false`.
    pub fn is_floor(&self, x: i32, y: i32) -> bool {
        self.tile(x, y) == Tile::Floor
    }

    /// Whether `(x, y)` is solid wall. Out-of-bounds is `true`.
    pub fn is_wall(&self, x: i32, y: i32) -> bool {
        self.tile(x, y) == Tile::Wall
    }

    /// The center of the first carved room — a convenient default spawn. `None` if no room was
    /// carved (a degenerate / too-small map).
    pub fn first_room_center(&self) -> Option<IVec2> {
        self.rooms.first().map(Room::center)
    }

    /// Converts the grid to `tiles[y][x]` rows of tile ids (floor → `floor_id`, wall → `wall_id`)
    /// for building a [`Tilemap`](crate::tilemap::Tilemap) — or a
    /// [`PathGrid`] / [`FovMap`](crate::fov::FovMap) — from the same
    /// layout.
    pub fn to_tilemap_tiles(&self, floor_id: u32, wall_id: u32) -> Vec<Vec<u32>> {
        (0..self.height)
            .map(|y| {
                (0..self.width)
                    .map(|x| {
                        if self.is_floor(x, y) {
                            floor_id
                        } else {
                            wall_id
                        }
                    })
                    .collect()
            })
            .collect()
    }

    /// Builds a [`PathGrid`] whose walkable cells are exactly this
    /// dungeon's [`Tile::Floor`] cells (walls → unwalkable). The `(x, y)` coordinates map 1:1, so it
    /// is the direct bridge to pathfinding ([`find_path`](crate::pathfinding::find_path) for enemy
    /// navigation) *and*, via [`FovMap::from_path_grid`](crate::fov::FovMap::from_path_grid), to
    /// field-of-view — the same walls that block movement block sight.
    ///
    /// ```
    /// use engine::{generate_bsp_dungeon, DungeonParams, FovMap};
    ///
    /// let map = generate_bsp_dungeon(48, 32, 7, &DungeonParams::default());
    /// let nav = map.to_path_grid(); // for enemy pathfinding
    /// let fov = FovMap::from_path_grid(&nav); // walls block sight
    /// let spawn = map.first_room_center().unwrap();
    /// assert!(nav.is_walkable(spawn.x, spawn.y)); // a room center is floor…
    /// assert!(!fov.is_opaque(spawn.x, spawn.y)); // …and transparent.
    /// ```
    pub fn to_path_grid(&self) -> PathGrid {
        let mut grid = PathGrid::new(self.width, self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                if self.is_wall(x, y) {
                    grid.set_walkable(x, y, false);
                }
            }
        }
        grid
    }

    fn set_floor(&mut self, x: i32, y: i32) {
        if let Some(i) = self.index(x, y) {
            self.tiles[i] = Tile::Floor;
        }
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }
}

/// Generates a BSP dungeon of `width × height` cells from `seed` and `params`.
///
/// Deterministic: the same `(width, height, seed, params)` always returns the identical
/// [`DungeonMap`]. The outer border is always wall. See the [module docs](self) for the algorithm.
pub fn generate_bsp_dungeon(
    width: i32,
    height: i32,
    seed: u64,
    params: &DungeonParams,
) -> DungeonMap {
    let mut map = DungeonMap::new_walls(width, height);
    // Need room for a 1-cell wall border plus a partition inside it.
    if map.width < 3 || map.height < 3 {
        return map;
    }
    let mut rng = Rng::new(seed);
    let (iw, ih) = (map.width - 2, map.height - 2);
    bsp_build(&mut map, &mut rng, 1, 1, iw, ih, 0, params);
    map
}

/// Recursively partitions `(x, y, w, h)`, carving a room per leaf and connecting the two children of
/// each split with a corridor. Returns a *representative room* of the subtree (for the parent to
/// connect to its sibling), or `None` when the subtree carved no room.
#[allow(clippy::too_many_arguments)]
fn bsp_build(
    map: &mut DungeonMap,
    rng: &mut Rng,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    depth: i32,
    params: &DungeonParams,
) -> Option<Room> {
    let can_split_w = w >= 2 * params.min_leaf;
    let can_split_h = h >= 2 * params.min_leaf;
    let too_big = w > params.max_leaf || h > params.max_leaf;
    // Split when it's feasible and either the partition is oversized or we roll for it.
    let split =
        depth < params.max_depth && (can_split_w || can_split_h) && (too_big || rng.chance(0.75));

    if !split {
        return carve_room(map, rng, x, y, w, h, params);
    }

    // Prefer to split the longer axis; fall back to the only feasible one.
    let split_vertical = if can_split_w && can_split_h {
        if w as f32 > h as f32 * 1.25 {
            true
        } else if h as f32 > w as f32 * 1.25 {
            false
        } else {
            rng.bool()
        }
    } else {
        can_split_w
    };

    let (a, b) = if split_vertical {
        let sx = rng.range(params.min_leaf, w - params.min_leaf + 1);
        ((x, y, sx, h), (x + sx, y, w - sx, h))
    } else {
        let sy = rng.range(params.min_leaf, h - params.min_leaf + 1);
        ((x, y, w, sy), (x, y + sy, w, h - sy))
    };

    let ra = bsp_build(map, rng, a.0, a.1, a.2, a.3, depth + 1, params);
    let rb = bsp_build(map, rng, b.0, b.1, b.2, b.3, depth + 1, params);

    match (ra, rb) {
        (Some(r1), Some(r2)) => {
            carve_corridor(map, rng, r1.center(), r2.center());
            Some(r1)
        }
        (some, None) | (None, some) => some,
    }
}

/// Carves a randomly-sized/placed room inside the partition `(x, y, w, h)` (respecting
/// `room_margin` and `min_room`), records it, and returns it — or `None` if the partition is too
/// small to hold a `min_room`-sized room.
fn carve_room(
    map: &mut DungeonMap,
    rng: &mut Rng,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    params: &DungeonParams,
) -> Option<Room> {
    let m = params.room_margin;
    let avail_w = w - 2 * m;
    let avail_h = h - 2 * m;
    if avail_w < params.min_room || avail_h < params.min_room {
        return None;
    }
    let rw = rng.range(params.min_room, avail_w + 1);
    let rh = rng.range(params.min_room, avail_h + 1);
    let rx = x + m + rng.range(0, avail_w - rw + 1);
    let ry = y + m + rng.range(0, avail_h - rh + 1);
    let room = Room {
        x: rx,
        y: ry,
        w: rw,
        h: rh,
    };
    for cy in room.y..room.y + room.h {
        for cx in room.x..room.x + room.w {
            map.set_floor(cx, cy);
        }
    }
    map.rooms.push(room);
    Some(room)
}

/// Carves a 1-wide L-shaped corridor between `a` and `b` (horizontal-then-vertical or the reverse,
/// chosen at random).
fn carve_corridor(map: &mut DungeonMap, rng: &mut Rng, a: IVec2, b: IVec2) {
    if rng.bool() {
        carve_h(map, a.x, b.x, a.y);
        carve_v(map, a.y, b.y, b.x);
    } else {
        carve_v(map, a.y, b.y, a.x);
        carve_h(map, a.x, b.x, b.y);
    }
}

/// Carves a horizontal floor run at row `y` between columns `x0` and `x1` (inclusive).
fn carve_h(map: &mut DungeonMap, x0: i32, x1: i32, y: i32) {
    for x in x0.min(x1)..=x0.max(x1) {
        map.set_floor(x, y);
    }
}

/// Carves a vertical floor run at column `x` between rows `y0` and `y1` (inclusive).
fn carve_v(map: &mut DungeonMap, y0: i32, y1: i32, x: i32) {
    for y in y0.min(y1)..=y0.max(y1) {
        map.set_floor(x, y);
    }
}

/// Generates an organic cave of `width × height` cells from `seed` and `params`, via cellular
/// automata.
///
/// The interior is seeded with random rock, smoothed with a few CA passes (the "4-5 rule"), then
/// reduced to its **largest connected cavern** — every smaller pocket is filled in — so the result
/// is **guaranteed connected**, just like [`generate_bsp_dungeon`]. Deterministic: the same
/// `(width, height, seed, params)` always returns the identical [`DungeonMap`]. The outer border is
/// always wall, and a single 1×1 [`Room`] is recorded at the cavern's most-central cell so
/// [`first_room_center`](DungeonMap::first_room_center) is a valid spawn. See the [module docs](self).
pub fn generate_cellular_cave(
    width: i32,
    height: i32,
    seed: u64,
    params: &CaveParams,
) -> DungeonMap {
    let mut map = DungeonMap::new_walls(width, height);
    // Need a 1-cell wall border around at least one interior cell.
    if map.width < 3 || map.height < 3 {
        return map;
    }
    // 1. Random fill — each interior cell is floor unless it rolls rock. Border stays wall.
    //    (This is the ONLY step that draws from `rng`, so determinism holds.)
    let mut rng = Rng::new(seed);
    let wall_prob = params.initial_wall_prob.clamp(0.0, 1.0);
    for y in 1..map.height - 1 {
        for x in 1..map.width - 1 {
            if !rng.chance(wall_prob) {
                map.set_floor(x, y);
            }
        }
    }
    // 2. Cellular-automata smoothing — grow walls into crowded cells, open sparse ones.
    for _ in 0..params.steps {
        cave_smooth(&mut map, params.wall_threshold);
    }
    // 3. Keep only the largest connected cavern → guaranteed connected + records the spawn room.
    keep_largest_cavern(&mut map);
    map
}

/// One cellular-automata pass (the classic "4-5 rule" with hysteresis): a **floor** cell turns to
/// wall when at least `wall_threshold` of its 8 Moore neighbors are wall; an **existing wall**
/// survives with one fewer (`wall_threshold - 1`). Out-of-bounds counts as wall; the border is left
/// solid. The sticky-wall rule keeps caverns structured instead of dilating them to open blobs.
fn cave_smooth(map: &mut DungeonMap, wall_threshold: u32) {
    let (w, h) = (map.width, map.height);
    // Read from a snapshot so every cell sees the same previous generation; out-of-bounds = wall.
    let prev = map.tiles.clone();
    let is_wall_at = |x: i32, y: i32| -> bool {
        if x < 0 || y < 0 || x >= w || y >= h {
            return true;
        }
        prev[(y * w + x) as usize] == Tile::Wall
    };
    let survive = wall_threshold.saturating_sub(1);
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let mut walls = 0u32;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if (dx, dy) != (0, 0) && is_wall_at(x + dx, y + dy) {
                        walls += 1;
                    }
                }
            }
            let i = (y * w + x) as usize;
            let threshold = if prev[i] == Tile::Wall {
                survive
            } else {
                wall_threshold
            };
            map.tiles[i] = if walls >= threshold {
                Tile::Wall
            } else {
                Tile::Floor
            };
        }
    }
}

/// Finds every connected floor region (4-connectivity), keeps the largest, fills the rest with wall,
/// and records a single 1×1 [`Room`] at the kept cavern's most-central cell (the floor cell nearest
/// its centroid). A map with no floor at all is left as solid wall with no room.
fn keep_largest_cavern(map: &mut DungeonMap) {
    let (w, h) = (map.width, map.height);
    let n = (w * h) as usize;
    let mut region = vec![u32::MAX; n]; // region id per cell; MAX = wall or not-yet-visited
    let mut regions: Vec<Vec<usize>> = Vec::new(); // flat cell indices per region

    for y in 0..h {
        for x in 0..w {
            let start = (y * w + x) as usize;
            if map.tiles[start] != Tile::Floor || region[start] != u32::MAX {
                continue;
            }
            let id = regions.len() as u32;
            let mut cells = Vec::new();
            let mut queue = VecDeque::new();
            region[start] = id;
            queue.push_back((x, y));
            while let Some((cx, cy)) = queue.pop_front() {
                cells.push((cy * w + cx) as usize);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (cx + dx, cy + dy);
                    if let Some(ni) = map.index(nx, ny) {
                        if map.tiles[ni] == Tile::Floor && region[ni] == u32::MAX {
                            region[ni] = id;
                            queue.push_back((nx, ny));
                        }
                    }
                }
            }
            regions.push(cells);
        }
    }

    // Largest region wins; ties break toward the later id (deterministic via `max_by_key`).
    let Some(best_id) = (0..regions.len()).max_by_key(|&i| regions[i].len()) else {
        return; // no floor anywhere — leave the map solid, no spawn room
    };

    // Fill every cell not in the winning cavern.
    for (i, r) in region.iter().enumerate() {
        if map.tiles[i] == Tile::Floor && *r != best_id as u32 {
            map.tiles[i] = Tile::Wall;
        }
    }

    // Spawn = the kept cavern's floor cell nearest its centroid (a roomy, central start).
    let cells = &regions[best_id];
    let count = cells.len() as i64;
    let (mut sx, mut sy) = (0i64, 0i64);
    for &i in cells {
        sx += (i as i32 % w) as i64;
        sy += (i as i32 / w) as i64;
    }
    let (cx, cy) = ((sx / count) as i32, (sy / count) as i32);
    let spawn = cells
        .iter()
        .min_by_key(|&&i| {
            let (px, py) = (i as i32 % w, i as i32 / w);
            let (dx, dy) = ((px - cx) as i64, (py - cy) as i64);
            dx * dx + dy * dy
        })
        .copied()
        .expect("the winning region has at least one cell");
    map.rooms.push(Room {
        x: spawn as i32 % w,
        y: spawn as i32 / w,
        w: 1,
        h: 1,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn default_map(seed: u64) -> DungeonMap {
        generate_bsp_dungeon(48, 32, seed, &DungeonParams::default())
    }

    #[test]
    fn same_seed_is_identical() {
        assert_eq!(default_map(42), default_map(42));
    }

    #[test]
    fn different_seeds_differ() {
        // Two different seeds on a 48×32 map are astronomically unlikely to coincide.
        assert_ne!(default_map(1), default_map(2));
    }

    #[test]
    fn border_is_all_wall() {
        let m = default_map(7);
        for x in 0..m.width {
            assert_eq!(m.tile(x, 0), Tile::Wall);
            assert_eq!(m.tile(x, m.height - 1), Tile::Wall);
        }
        for y in 0..m.height {
            assert_eq!(m.tile(0, y), Tile::Wall);
            assert_eq!(m.tile(m.width - 1, y), Tile::Wall);
        }
    }

    #[test]
    fn rooms_are_floor_in_bounds_and_min_sized() {
        let m = default_map(99);
        assert!(!m.rooms.is_empty(), "a 48×32 dungeon should carve rooms");
        for r in &m.rooms {
            assert!(r.w >= DungeonParams::default().min_room);
            assert!(r.h >= DungeonParams::default().min_room);
            assert!(r.x >= 1 && r.y >= 1);
            assert!(r.x + r.w < m.width && r.y + r.h < m.height);
            for cy in r.y..r.y + r.h {
                for cx in r.x..r.x + r.w {
                    assert!(m.is_floor(cx, cy), "room cell ({cx},{cy}) must be floor");
                }
            }
        }
    }

    /// Every room center is reachable from the first room center via floor cells — the corridors
    /// connect the whole dungeon.
    #[test]
    fn all_rooms_are_connected() {
        let m = default_map(2026);
        let start = m.first_room_center().expect("at least one room");
        // BFS flood fill over floor cells.
        let mut seen = vec![false; (m.width * m.height) as usize];
        let mut q = VecDeque::new();
        let idx = |p: IVec2| (p.y * m.width + p.x) as usize;
        seen[idx(start)] = true;
        q.push_back(start);
        while let Some(p) = q.pop_front() {
            for d in [
                IVec2::new(1, 0),
                IVec2::new(-1, 0),
                IVec2::new(0, 1),
                IVec2::new(0, -1),
            ] {
                let n = p + d;
                if m.is_floor(n.x, n.y) && !seen[idx(n)] {
                    seen[idx(n)] = true;
                    q.push_back(n);
                }
            }
        }
        for (i, r) in m.rooms.iter().enumerate() {
            let c = r.center();
            assert!(
                seen[idx(c)],
                "room {i} center {c:?} is not reachable from room 0 — dungeon is disconnected"
            );
        }
    }

    #[test]
    fn to_tilemap_tiles_matches_grid() {
        let m = default_map(5);
        let tiles = m.to_tilemap_tiles(1, 0);
        assert_eq!(tiles.len(), m.height as usize);
        assert_eq!(tiles[0].len(), m.width as usize);
        for y in 0..m.height {
            for x in 0..m.width {
                let expected = if m.is_floor(x, y) { 1 } else { 0 };
                assert_eq!(tiles[y as usize][x as usize], expected);
            }
        }
    }

    #[test]
    fn to_path_grid_walkable_matches_floor() {
        let m = default_map(11);
        let g = m.to_path_grid();
        assert_eq!((g.width, g.height), (m.width, m.height));
        for y in 0..m.height {
            for x in 0..m.width {
                assert_eq!(
                    g.is_walkable(x, y),
                    m.is_floor(x, y),
                    "walkability must mirror floor at ({x},{y})"
                );
            }
        }
    }

    /// The capstone composition seam: a `FovMap` built straight from a generated dungeon sees the
    /// spawn room's floor (walls block sight, floor does not).
    #[test]
    fn fov_from_dungeon_sees_the_spawn_room() {
        use crate::fov::FovMap;
        let m = default_map(2026);
        let mut fov = FovMap::from_path_grid(&m.to_path_grid());
        let spawn = m.first_room_center().expect("a room");
        fov.compute(spawn, 20);
        assert!(
            fov.is_visible(spawn.x, spawn.y),
            "the observer sees its own cell"
        );
        for d in [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ] {
            let n = spawn + d;
            if m.is_floor(n.x, n.y) {
                assert!(
                    fov.is_visible(n.x, n.y),
                    "an adjacent floor cell {n:?} in the spawn room is in sight"
                );
            }
        }
    }

    #[test]
    fn degenerate_sizes_are_safe() {
        // Too-small maps carve nothing and never panic.
        for (w, h) in [(0, 0), (2, 2), (-4, 10), (1, 50)] {
            let m = generate_bsp_dungeon(w, h, 1, &DungeonParams::default());
            assert!(m.rooms.is_empty());
            assert!(!m.is_floor(0, 0));
        }
        // Overflowing dimensions collapse to an empty map.
        let huge = generate_bsp_dungeon(i32::MAX, 2, 1, &DungeonParams::default());
        assert_eq!((huge.width, huge.height), (0, 0));
    }

    // ── Cellular-automata caves ──────────────────────────────────────────────

    fn default_cave(seed: u64) -> DungeonMap {
        generate_cellular_cave(64, 44, seed, &CaveParams::default())
    }

    /// Every floor cell reachable from `map`'s spawn via 4-connectivity, as a boolean grid.
    fn flood_from_spawn(map: &DungeonMap) -> Vec<bool> {
        let mut seen = vec![false; (map.width * map.height) as usize];
        let Some(spawn) = map.first_room_center() else {
            return seen;
        };
        let idx = |x: i32, y: i32| (y * map.width + x) as usize;
        let mut q = VecDeque::new();
        seen[idx(spawn.x, spawn.y)] = true;
        q.push_back(spawn);
        while let Some(p) = q.pop_front() {
            for d in [
                IVec2::new(1, 0),
                IVec2::new(-1, 0),
                IVec2::new(0, 1),
                IVec2::new(0, -1),
            ] {
                let n = p + d;
                if map.is_floor(n.x, n.y) && !seen[idx(n.x, n.y)] {
                    seen[idx(n.x, n.y)] = true;
                    q.push_back(n);
                }
            }
        }
        seen
    }

    #[test]
    fn cave_same_seed_is_identical() {
        assert_eq!(default_cave(42), default_cave(42));
    }

    #[test]
    fn cave_different_seeds_differ() {
        assert_ne!(default_cave(1), default_cave(2));
    }

    #[test]
    fn cave_border_is_all_wall() {
        let m = default_cave(7);
        for x in 0..m.width {
            assert_eq!(m.tile(x, 0), Tile::Wall);
            assert_eq!(m.tile(x, m.height - 1), Tile::Wall);
        }
        for y in 0..m.height {
            assert_eq!(m.tile(0, y), Tile::Wall);
            assert_eq!(m.tile(m.width - 1, y), Tile::Wall);
        }
    }

    #[test]
    fn cave_spawn_is_central_floor() {
        let m = default_cave(2026);
        let spawn = m.first_room_center().expect("a cave has a spawn room");
        assert!(m.is_floor(spawn.x, spawn.y), "spawn cell must be floor");
        // Exactly one synthetic 1×1 spawn room is recorded.
        assert_eq!(m.rooms.len(), 1);
        assert_eq!((m.rooms[0].w, m.rooms[0].h), (1, 1));
    }

    /// The defining cave invariant: after keep-largest, EVERY floor cell is reachable from the
    /// spawn — the cavern is a single connected region (mirrors BSP's `all_rooms_are_connected`).
    #[test]
    fn cave_is_a_single_connected_cavern() {
        for seed in [1, 7, 42, 99, 2026] {
            let m = default_cave(seed);
            let seen = flood_from_spawn(&m);
            let mut floor = 0;
            for y in 0..m.height {
                for x in 0..m.width {
                    if m.is_floor(x, y) {
                        floor += 1;
                        assert!(
                            seen[(y * m.width + x) as usize],
                            "seed {seed}: floor cell ({x},{y}) is unreachable from spawn — \
                             the cavern is disconnected"
                        );
                    }
                }
            }
            assert!(floor > 0, "seed {seed}: a 64×44 cave should carve floor");
        }
    }

    /// A cave is a `DungeonMap`, so it feeds `to_path_grid` (walkable == floor) and, through it,
    /// `FovMap` — the same composition seam the BSP dungeon uses.
    #[test]
    fn cave_composes_with_path_grid_and_fov() {
        use crate::fov::FovMap;
        let m = default_cave(2026);
        let nav = m.to_path_grid();
        assert_eq!((nav.width, nav.height), (m.width, m.height));
        for y in 0..m.height {
            for x in 0..m.width {
                assert_eq!(nav.is_walkable(x, y), m.is_floor(x, y));
            }
        }
        let mut fov = FovMap::from_path_grid(&nav);
        let spawn = m.first_room_center().expect("a spawn");
        fov.compute(spawn, 20);
        assert!(
            fov.is_visible(spawn.x, spawn.y),
            "the observer sees its cell"
        );
    }

    #[test]
    fn cave_steps_zero_is_still_connected() {
        // With no smoothing passes the keep-largest pass alone must still leave one connected region.
        let params = CaveParams {
            steps: 0,
            ..CaveParams::default()
        };
        let m = generate_cellular_cave(48, 32, 3, &params);
        let seen = flood_from_spawn(&m);
        for y in 0..m.height {
            for x in 0..m.width {
                if m.is_floor(x, y) {
                    assert!(seen[(y * m.width + x) as usize]);
                }
            }
        }
    }

    #[test]
    fn cave_degenerate_sizes_are_safe() {
        for (w, h) in [(0, 0), (2, 2), (-4, 10), (1, 50)] {
            let m = generate_cellular_cave(w, h, 1, &CaveParams::default());
            assert!(m.rooms.is_empty());
            assert!(!m.is_floor(0, 0));
            assert!(m.first_room_center().is_none());
        }
        let huge = generate_cellular_cave(i32::MAX, 2, 1, &CaveParams::default());
        assert_eq!((huge.width, huge.height), (0, 0));
    }
}
