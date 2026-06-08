use glam::IVec2;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use crate::tilemap::Tilemap;

/// Grid storing walkability (row-major array).
pub struct PathGrid {
    pub width: i32,
    pub height: i32,
    cells: Vec<bool>,
}

const MAX_PATH_GRID_CELLS: usize = 10_000_000;

impl PathGrid {
    /// Creates a grid with all cells walkable.
    pub fn new(width: i32, height: i32) -> Self {
        let size = grid_cell_count(width, height);
        Self {
            width,
            height,
            cells: vec![true; size],
        }
    }

    /// Creates a grid with all cells blocked.
    pub fn new_blocked(width: i32, height: i32) -> Self {
        let size = grid_cell_count(width, height);
        Self {
            width,
            height,
            cells: vec![false; size],
        }
    }

    pub fn set_walkable(&mut self, x: i32, y: i32, walkable: bool) {
        if let Some(idx) = self.index(x, y) {
            self.cells[idx] = walkable;
        }
    }

    /// Returns `false` for out-of-bounds coordinates (no panic).
    pub fn is_walkable(&self, x: i32, y: i32) -> bool {
        self.index(x, y).map(|i| self.cells[i]).unwrap_or(false)
    }

    /// Build a `PathGrid` from a `Tilemap`, judging each tile via the
    /// `is_blocked` predicate. The grid's `(x, y)` coordinates map 1:1 to the
    /// tilemap's `tiles[y][x]`. Cells where `is_blocked` returns `true` are
    /// marked unwalkable. Jagged tile rows use the longest row as width;
    /// missing cells are treated as the empty tile (`0`).
    ///
    /// # Example
    /// ```
    /// use engine::{PathGrid, Tilemap, TilemapAtlas};
    /// use glam::Vec2;
    ///
    /// let atlas = TilemapAtlas::new("tiles.png", 4, 4);
    /// // 0 = floor, 1 = wall
    /// let tiles = vec![
    ///     vec![0, 0, 0],
    ///     vec![0, 1, 0],
    ///     vec![0, 0, 0],
    /// ];
    /// let map = Tilemap::new(atlas, tiles, 32.0, Vec2::ZERO);
    /// let grid = PathGrid::from_tilemap(&map, |id| id == 1);
    /// assert!(grid.is_walkable(0, 0));
    /// assert!(!grid.is_walkable(1, 1));
    /// ```
    pub fn from_tilemap(tilemap: &Tilemap, is_blocked: impl Fn(u32) -> bool) -> Self {
        let height = tilemap.tiles.len() as i32;
        let width = tilemap.tiles.iter().map(|row| row.len()).max().unwrap_or(0) as i32;

        let mut grid = Self::new(width, height);
        for (row_idx, row) in tilemap.tiles.iter().enumerate() {
            for (col_idx, &tile_id) in row.iter().enumerate() {
                if is_blocked(tile_id) {
                    grid.set_walkable(col_idx as i32, row_idx as i32, false);
                }
            }
        }
        grid
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        let idx = y.checked_mul(self.width)?.checked_add(x)? as usize;
        (idx < self.cells.len()).then_some(idx)
    }
}

fn grid_cell_count(width: i32, height: i32) -> usize {
    let Some(size) = width.checked_mul(height) else {
        return 0;
    };
    if size <= 0 {
        return 0;
    }
    let size = size as usize;
    if size > MAX_PATH_GRID_CELLS {
        0
    } else {
        size
    }
}

// ── A* Internals ──────────────────────────────────────────────────────────────

#[derive(Eq, PartialEq)]
struct Node {
    f: i32, // f = g + h
    pos: IVec2,
}

// BinaryHeap is a max-heap → reverse comparison so smaller f has higher priority
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.cmp(&self.f).then_with(|| {
            other
                .pos
                .x
                .cmp(&self.pos.x)
                .then(other.pos.y.cmp(&self.pos.y))
        })
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn manhattan(a: IVec2, b: IVec2) -> i32 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

const NEIGHBORS: [IVec2; 4] = [
    IVec2::new(0, -1),
    IVec2::new(0, 1),
    IVec2::new(-1, 0),
    IVec2::new(1, 0),
];

/// Reusable A* scratch collections. Kept in a thread-local so repeated
/// `find_path` calls (e.g. many AI agents repathing each frame) reuse their
/// allocations instead of re-allocating the open list / score maps every call.
#[derive(Default)]
struct AStarScratch {
    open: BinaryHeap<Node>,
    /// g_score: actual cost from start to each node
    g_score: HashMap<IVec2, i32>,
    /// came_from: used for path reconstruction
    came_from: HashMap<IVec2, IVec2>,
    /// closed: nodes already expanded with their optimal cost; filters duplicate heap entries
    closed: HashSet<IVec2>,
}

impl AStarScratch {
    fn reset(&mut self) {
        self.open.clear();
        self.g_score.clear();
        self.came_from.clear();
        self.closed.clear();
    }
}

thread_local! {
    static ASTAR_SCRATCH: RefCell<AStarScratch> = RefCell::new(AStarScratch::default());
}

/// A* pathfinding.
/// Returns the path excluding the start and including the goal, or `None` if no path exists.
///
/// The Manhattan heuristic is consistent on a 4-directional uniform grid, so a node's g
/// value is optimal the first time it is popped. Skipping stale duplicate heap entries for
/// already-closed nodes therefore preserves shortest-path guarantees.
pub fn find_path(grid: &PathGrid, start: IVec2, goal: IVec2) -> Option<Vec<IVec2>> {
    // start == goal
    if start == goal {
        return Some(vec![goal]);
    }

    // goal is blocked — return None immediately
    if !grid.is_walkable(goal.x, goal.y) {
        return None;
    }

    ASTAR_SCRATCH.with(|cell| {
        let mut s = cell.borrow_mut();
        s.reset();

        s.g_score.insert(start, 0);
        s.open.push(Node {
            f: manhattan(start, goal),
            pos: start,
        });

        while let Some(Node { pos: current, .. }) = s.open.pop() {
            if current == goal {
                // reconstruct path
                let mut path = Vec::new();
                let mut cur = current;
                while let Some(&prev) = s.came_from.get(&cur) {
                    path.push(cur);
                    cur = prev;
                }
                path.reverse();
                return Some(path);
            }

            // already closed — stale duplicate entry, skip re-expansion
            if !s.closed.insert(current) {
                continue;
            }

            let g_current = *s.g_score.get(&current).unwrap_or(&i32::MAX);

            for &dir in &NEIGHBORS {
                let next = current + dir;
                // already-closed neighbor cannot be improved
                if s.closed.contains(&next) {
                    continue;
                }
                if !grid.is_walkable(next.x, next.y) {
                    continue;
                }
                let g_next = g_current + 1;
                if g_next < *s.g_score.get(&next).unwrap_or(&i32::MAX) {
                    s.g_score.insert(next, g_next);
                    s.came_from.insert(next, current);
                    s.open.push(Node {
                        f: g_next + manhattan(next, goal),
                        pos: next,
                    });
                }
            }
        }

        None
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_straight_path() {
        // 5x1 grid, no obstacles → straight line from start(0,0) to goal(4,0)
        let grid = PathGrid::new(5, 1);
        let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(4, 0)).unwrap();
        assert_eq!(path.last(), Some(&IVec2::new(4, 0)));
        // start excluded, goal included
        assert!(!path.contains(&IVec2::new(0, 0)));
        assert_eq!(path.len(), 4);
    }

    #[test]
    fn test_obstacle_detour() {
        // 3x3 grid with obstacle at (0,1) forcing a detour
        // S . .
        // X X .
        // . . G
        let mut grid = PathGrid::new(3, 3);
        grid.set_walkable(0, 1, false);
        grid.set_walkable(1, 1, false);

        let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(2, 2)).unwrap();
        // path must exist and every cell must be walkable
        for pos in &path {
            assert!(
                grid.is_walkable(pos.x, pos.y),
                "path includes blocked cell: {pos:?}"
            );
        }
        assert_eq!(path.last(), Some(&IVec2::new(2, 2)));
        assert!(!path.contains(&IVec2::new(0, 0)));
    }

    #[test]
    fn test_no_path() {
        // 3x3 grid with entire right column blocked → goal(2,0) unreachable
        let mut grid = PathGrid::new(3, 3);
        grid.set_walkable(1, 0, false);
        grid.set_walkable(1, 1, false);
        grid.set_walkable(1, 2, false);
        // goal itself is in the blocked column so result is immediately None
        let result = find_path(&grid, IVec2::new(0, 0), IVec2::new(2, 0));
        assert!(result.is_none());
    }

    #[test]
    fn test_start_equals_goal() {
        let grid = PathGrid::new(5, 5);
        let pos = IVec2::new(2, 3);
        let result = find_path(&grid, pos, pos);
        assert_eq!(result, Some(vec![pos]));
    }

    #[test]
    fn from_tilemap_marks_blocked_tiles_unwalkable() {
        use crate::tilemap::{Tilemap, TilemapAtlas};
        use glam::Vec2;

        // 3x3, only the center (1,1) is blocked
        let atlas = TilemapAtlas::new("tiles.png", 4, 4);
        let tiles = vec![vec![0, 0, 0], vec![0, 1, 0], vec![0, 0, 0]];
        let map = Tilemap::new(atlas, tiles, 32.0, Vec2::ZERO);
        let grid = PathGrid::from_tilemap(&map, |id| id == 1);

        assert_eq!(grid.width, 3);
        assert_eq!(grid.height, 3);
        assert!(grid.is_walkable(0, 0));
        assert!(grid.is_walkable(2, 0));
        assert!(!grid.is_walkable(1, 1));
        assert!(grid.is_walkable(1, 0));
    }

    #[test]
    fn from_tilemap_with_jagged_rows_uses_max_width() {
        use crate::tilemap::{Tilemap, TilemapAtlas};
        use glam::Vec2;

        let atlas = TilemapAtlas::new("tiles.png", 4, 4);
        // Second row is shorter → missing cells treated as empty tile (0)
        let tiles = vec![vec![0, 0, 1], vec![0, 0]];
        let map = Tilemap::new(atlas, tiles, 32.0, Vec2::ZERO);
        let grid = PathGrid::from_tilemap(&map, |id| id == 1);

        assert_eq!(grid.width, 3);
        assert_eq!(grid.height, 2);
        // (2,0) blocked, missing (2,1) stays walkable
        assert!(!grid.is_walkable(2, 0));
        assert!(grid.is_walkable(2, 1));
    }

    #[test]
    fn optimal_path_length_on_open_grid() {
        // 5x5 open grid: (0,0) → (4,4). Shortest path = 8 steps (Manhattan),
        // start excluded / goal included → 8 cells. The closed-set must not
        // break optimality.
        let grid = PathGrid::new(5, 5);
        let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(4, 4)).unwrap();
        assert_eq!(path.len(), 8, "shortest path length must be 8: {path:?}");
        assert_eq!(path.last(), Some(&IVec2::new(4, 4)));
        // each step between adjacent cells is always distance 1 (no diagonals)
        let mut prev = IVec2::new(0, 0);
        for &p in &path {
            assert_eq!(manhattan(prev, p), 1, "non-adjacent jump: {prev:?}→{p:?}");
            prev = p;
        }
    }

    #[test]
    fn repeated_calls_reuse_scratch_without_contamination() {
        // Reusing the same thread_local scratch must not let the previous call's state
        // contaminate the next result. Alternates between blocked and open cases.
        let mut grid = PathGrid::new(3, 3);
        grid.set_walkable(1, 0, false);
        grid.set_walkable(1, 1, false);
        grid.set_walkable(1, 2, false);
        // blocked column makes goal unreachable
        assert!(find_path(&grid, IVec2::new(0, 0), IVec2::new(2, 0)).is_none());

        // reachable goal on the same grid — must work even with leftover scratch from previous None call
        let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(0, 2)).unwrap();
        assert_eq!(path, vec![IVec2::new(0, 1), IVec2::new(0, 2)]);

        // remove obstacle and retry — cached closed/g_score must be cleared so a new path is found
        grid.set_walkable(1, 0, true);
        let again = find_path(&grid, IVec2::new(0, 0), IVec2::new(2, 0)).unwrap();
        assert_eq!(again.last(), Some(&IVec2::new(2, 0)));
        assert_eq!(again.len(), 2);
    }

    #[test]
    fn path_grid_invalid_or_overflow_size_is_empty() {
        let overflow = PathGrid::new(i32::MAX, 2);
        assert!(!overflow.is_walkable(0, 0));

        let negative = PathGrid::new(-3, 4);
        assert!(!negative.is_walkable(0, 0));

        let blocked = PathGrid::new_blocked(i32::MAX, 2);
        assert!(!blocked.is_walkable(0, 0));
    }
}
