//! Procedural dungeon generation — BSP (binary space partition) rooms + corridors.
//!
//! [`generate_bsp_dungeon`] carves a roomy dungeon into a grid: it recursively splits the map into
//! partitions, cuts a room into each leaf, and connects sibling partitions with L-shaped corridors
//! while unwinding the recursion — so the whole dungeon is **guaranteed connected** (every room is
//! reachable from every other). Generation is **deterministic**: the same `seed` + [`DungeonParams`]
//! always produce the identical [`DungeonMap`], so a game can store just the seed and regenerate the
//! level, or reproduce a run exactly.
//!
//! The result is a plain owned grid (like [`PathGrid`](crate::pathfinding::PathGrid) /
//! [`FovMap`](crate::fov::FovMap) — not an ECS component): read [`is_floor`](DungeonMap::is_floor) /
//! [`is_wall`](DungeonMap::is_wall) to render or collide, [`rooms`](DungeonMap::rooms) for spawn
//! placement, and [`to_tilemap_tiles`](DungeonMap::to_tilemap_tiles) to build a
//! [`Tilemap`](crate::tilemap::Tilemap) / `PathGrid` / `FovMap` from the same layout.
//!
//! ```
//! use engine::{generate_bsp_dungeon, DungeonParams, Tile};
//!
//! let a = generate_bsp_dungeon(48, 32, 1234, &DungeonParams::default());
//! let b = generate_bsp_dungeon(48, 32, 1234, &DungeonParams::default());
//! assert_eq!(a, b, "same seed → identical dungeon");
//! assert!(!a.rooms.is_empty());
//! // The border is always solid wall.
//! assert_eq!(a.tile(0, 0), Tile::Wall);
//! ```

use glam::IVec2;

use crate::pathfinding::MAX_PATH_GRID_CELLS;

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
    /// [`PathGrid`](crate::pathfinding::PathGrid) / [`FovMap`](crate::fov::FovMap) — from the same
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

/// A tiny deterministic PRNG (SplitMix64) — kept private so generation depends only on the seed,
/// never on `rand`'s thread RNG. Same seed → same stream → same dungeon.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `[lo, hi)`. Returns `lo` when the range is empty.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i32
    }

    fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// `true` with probability `p` (clamped to `0..=1`).
    fn chance(&mut self, p: f32) -> bool {
        let unit = (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32;
        unit < p.clamp(0.0, 1.0)
    }
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

    #[test]
    fn rng_is_deterministic_and_ranges_are_bounded() {
        let mut a = Rng::new(123);
        let mut b = Rng::new(123);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        let mut r = Rng::new(9);
        for _ in 0..1000 {
            let v = r.range(3, 7);
            assert!((3..7).contains(&v));
        }
        // An empty range yields the low bound rather than panicking on `% 0`.
        assert_eq!(Rng::new(1).range(5, 5), 5);
    }
}
