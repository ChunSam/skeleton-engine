//! Field of view — recursive shadowcasting on a 2D grid.
//!
//! [`FovMap`] answers "which cells can an observer at `origin` see, given walls that block
//! sight?" It is a plain owned helper (like [`PathGrid`] or
//! [`InputBuffer`](crate::input_buffer::InputBuffer)) — a game keeps one in a system's state or
//! a resource and recomputes it when the observer moves. It is **not** an ECS component and has
//! no system; the game drives it.
//!
//! Genre-agnostic vision: roguelike field-of-view, stealth sight cones, top-down fog-of-war,
//! or tactics line-of-sight all reduce to "opaque cells cast shadows".
//!
//! Two queries, computed from the same opaque grid:
//! - [`FovMap::compute`] runs **recursive shadowcasting** over all 8 octants — the classic
//!   symmetric-ish area FOV. It fills the `visible` set (cleared each call) and accumulates the
//!   `revealed` set (cells seen at least once — the fog-of-war "explored" memory).
//! - [`FovMap::line_of_sight`] is a single **point-to-point** Bresenham check ("can A see B?"),
//!   the natural primitive for a stealth guard's sight line.
//!
//! ```
//! use engine::FovMap;
//! use glam::IVec2;
//!
//! // 5×5 open room with a single wall two cells north of the observer.
//! let mut fov = FovMap::new(5, 5);
//! fov.set_opaque(2, 1, true);
//! fov.compute(IVec2::new(2, 3), 5);
//!
//! assert!(fov.is_visible(2, 3));  // the observer's own cell
//! assert!(fov.is_visible(2, 1));  // the wall itself is seen…
//! assert!(!fov.is_visible(2, 0)); // …but the cell directly behind it is shadowed
//! ```

use glam::IVec2;

use crate::pathfinding::{PathGrid, MAX_PATH_GRID_CELLS};

/// One of the 8 shadowcasting octants: the linear transform mapping octant-local `(dx, dy)`
/// offsets into world offsets. Bundling the four multipliers keeps [`FovMap::cast_light`] under
/// the argument-count lint and names the transform.
#[derive(Clone, Copy)]
struct Octant {
    xx: i32,
    xy: i32,
    yx: i32,
    yy: i32,
}

/// The 8 octant transforms (RogueBasin recursive-shadowcasting multiplier table).
const OCTANTS: [Octant; 8] = [
    Octant {
        xx: 1,
        xy: 0,
        yx: 0,
        yy: 1,
    },
    Octant {
        xx: 0,
        xy: 1,
        yx: 1,
        yy: 0,
    },
    Octant {
        xx: 0,
        xy: -1,
        yx: 1,
        yy: 0,
    },
    Octant {
        xx: -1,
        xy: 0,
        yx: 0,
        yy: 1,
    },
    Octant {
        xx: -1,
        xy: 0,
        yx: 0,
        yy: -1,
    },
    Octant {
        xx: 0,
        xy: -1,
        yx: -1,
        yy: 0,
    },
    Octant {
        xx: 0,
        xy: 1,
        yx: -1,
        yy: 0,
    },
    Octant {
        xx: 1,
        xy: 0,
        yx: 0,
        yy: -1,
    },
];

/// A grid of sight-blocking (`opaque`) cells plus the `visible` / `revealed` sets computed from an
/// observer position. Row-major; `(x, y)` with `x` in `0..width` and `y` in `0..height`.
///
/// Build one directly with [`new`](Self::new) + [`set_opaque`](Self::set_opaque), or straight from
/// a [`PathGrid`] via [`from_path_grid`](Self::from_path_grid) (a non-walkable cell blocks sight).
/// Then call [`compute`](Self::compute) each time the observer moves and read
/// [`is_visible`](Self::is_visible) / [`is_revealed`](Self::is_revealed) while rendering.
#[derive(Clone, Debug)]
pub struct FovMap {
    /// Grid width in cells.
    pub width: i32,
    /// Grid height in cells.
    pub height: i32,
    opaque: Vec<bool>,
    visible: Vec<bool>,
    revealed: Vec<bool>,
}

impl FovMap {
    /// A `width × height` map with every cell transparent, nothing visible, nothing revealed.
    ///
    /// A non-positive dimension — or a `width × height` that exceeds [`MAX_PATH_GRID_CELLS`] (or
    /// overflows) — yields an empty `0 × 0` map with a logged error (every query reads out of
    /// bounds) rather than a panic or a multi-gigabyte allocation.
    pub fn new(width: i32, height: i32) -> Self {
        let (width, height, size) = Self::sized(width, height);
        Self {
            width,
            height,
            opaque: vec![false; size],
            visible: vec![false; size],
            revealed: vec![false; size],
        }
    }

    /// Builds a map whose opaque cells are the [`PathGrid`]'s **non-walkable** cells — the common
    /// case where the same walls that block movement also block sight. The coordinate systems match
    /// 1:1, so a grid built via [`PathGrid::from_tilemap`] carries straight over.
    ///
    /// ```
    /// use engine::{FovMap, PathGrid};
    /// let mut grid = PathGrid::new(3, 3);
    /// grid.set_walkable(1, 1, false); // a wall
    /// let fov = FovMap::from_path_grid(&grid);
    /// assert!(fov.is_opaque(1, 1));
    /// assert!(!fov.is_opaque(0, 0));
    /// ```
    pub fn from_path_grid(grid: &PathGrid) -> Self {
        let mut map = Self::new(grid.width, grid.height);
        for y in 0..map.height {
            for x in 0..map.width {
                if !grid.is_walkable(x, y) {
                    if let Some(i) = map.index(x, y) {
                        map.opaque[i] = true;
                    }
                }
            }
        }
        map
    }

    /// Marks `(x, y)` as sight-blocking (`opaque = true`) or transparent. Out-of-bounds is a no-op.
    pub fn set_opaque(&mut self, x: i32, y: i32, opaque: bool) {
        if let Some(i) = self.index(x, y) {
            self.opaque[i] = opaque;
        }
    }

    /// Whether `(x, y)` blocks sight. Out-of-bounds reads `false` (no recorded wall).
    pub fn is_opaque(&self, x: i32, y: i32) -> bool {
        self.index(x, y).map(|i| self.opaque[i]).unwrap_or(false)
    }

    /// Whether `(x, y)` was visible from the last [`compute`](Self::compute). Out-of-bounds `false`.
    pub fn is_visible(&self, x: i32, y: i32) -> bool {
        self.index(x, y).map(|i| self.visible[i]).unwrap_or(false)
    }

    /// Whether `(x, y)` has ever been visible since the last [`reset`](Self::reset) — the
    /// fog-of-war "explored" memory. Out-of-bounds reads `false`.
    pub fn is_revealed(&self, x: i32, y: i32) -> bool {
        self.index(x, y).map(|i| self.revealed[i]).unwrap_or(false)
    }

    /// Clears the visible set (keeps `revealed` and `opaque`). [`compute`](Self::compute) does this
    /// for you; call it directly to blank the view without recomputing.
    pub fn clear_visible(&mut self) {
        self.visible.iter_mut().for_each(|v| *v = false);
    }

    /// Clears both the visible and revealed sets (keeps the `opaque` walls) — e.g. on entering a
    /// new level, so the explored memory starts blank.
    pub fn reset(&mut self) {
        self.clear_visible();
        self.revealed.iter_mut().for_each(|v| *v = false);
    }

    /// Recomputes the visible set for an observer at `origin` seeing out to `radius` cells
    /// (Euclidean: a cell is in range when `dx² + dy² ≤ radius²`). Clears the previous visible set
    /// first, always marks `origin` visible, and folds every newly-seen cell into `revealed`.
    ///
    /// Uses recursive shadowcasting over 8 octants: an opaque cell is itself lit (you see the wall)
    /// and then casts a shadow over everything behind it. `radius ≤ 0` lights only `origin`.
    pub fn compute(&mut self, origin: IVec2, radius: i32) {
        self.clear_visible();
        self.light(origin.x, origin.y); // the observer always sees its own cell
        if radius <= 0 {
            return;
        }
        for oct in OCTANTS {
            self.cast_light(origin, 1, 1.0, 0.0, radius, oct);
        }
    }

    /// Whether `from` has a clear line of sight to `to`: a Bresenham line between them crosses no
    /// opaque cell (endpoints excluded, so an observer can "see" the wall it is looking at, and its
    /// own cell being opaque never blocks). The natural primitive for a stealth sight check;
    /// independent of [`compute`](Self::compute) (reads only the `opaque` grid).
    pub fn line_of_sight(&self, from: IVec2, to: IVec2) -> bool {
        let (mut x, mut y) = (from.x, from.y);
        let dx = (to.x - x).abs();
        let dy = -(to.y - y).abs();
        let sx = if x < to.x { 1 } else { -1 };
        let sy = if y < to.y { 1 } else { -1 };
        let mut err = dx + dy;
        loop {
            if x == to.x && y == to.y {
                return true;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
            if x == to.x && y == to.y {
                return true; // reached the target — the last cell is the endpoint, not a blocker
            }
            if self.is_opaque(x, y) {
                return false;
            }
        }
    }

    /// Marks `(x, y)` visible and revealed if it is in bounds.
    fn light(&mut self, x: i32, y: i32) {
        if let Some(i) = self.index(x, y) {
            self.visible[i] = true;
            self.revealed[i] = true;
        }
    }

    /// Recursive shadowcasting for one octant (RogueBasin's algorithm). `row` is the scan distance
    /// to start at; `start`/`end` are the slopes bounding the still-lit angular sweep. A run of
    /// opaque cells narrows the sweep; a child scan handles the sub-sweep before the blocker.
    fn cast_light(
        &mut self,
        origin: IVec2,
        row: i32,
        mut start: f32,
        end: f32,
        radius: i32,
        oct: Octant,
    ) {
        if start < end {
            return;
        }
        let radius_sq = radius * radius;
        for j in row..=radius {
            let dy = -j;
            let mut dx = -j - 1;
            let mut blocked = false;
            let mut new_start = 0.0f32;
            while dx <= 0 {
                dx += 1;
                let cx = origin.x + dx * oct.xx + dy * oct.xy;
                let cy = origin.y + dx * oct.yx + dy * oct.yy;
                // Slopes of this cell's left and right edges (dy is always ≤ -1 here, so the
                // denominators are never zero).
                let l_slope = (dx as f32 - 0.5) / (dy as f32 + 0.5);
                let r_slope = (dx as f32 + 0.5) / (dy as f32 - 0.5);
                if start < r_slope {
                    continue;
                } else if end > l_slope {
                    break;
                }
                if dx * dx + dy * dy <= radius_sq {
                    self.light(cx, cy);
                }
                if blocked {
                    if self.is_opaque(cx, cy) {
                        new_start = r_slope;
                        continue;
                    } else {
                        blocked = false;
                        start = new_start;
                    }
                } else if self.is_opaque(cx, cy) && j < radius {
                    // A blocker mid-sweep: recurse on the sub-sweep to its left, then keep scanning
                    // this row from the blocker's right edge.
                    blocked = true;
                    self.cast_light(origin, j + 1, start, l_slope, radius, oct);
                    new_start = r_slope;
                }
            }
            if blocked {
                break;
            }
        }
    }

    /// Resolves `(width, height)` to a usable `(width, height, cell_count)`, collapsing a
    /// non-positive, overflowing, or over-cap size to an empty `(0, 0, 0)` with a logged error.
    fn sized(width: i32, height: i32) -> (i32, i32, usize) {
        if width <= 0 || height <= 0 {
            return (0, 0, 0);
        }
        match (width as usize).checked_mul(height as usize) {
            Some(size) if size <= MAX_PATH_GRID_CELLS => (width, height, size),
            other => {
                let size = other
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "overflow".into());
                log::error!(
                    "FovMap {width}×{height} ({size} cells) exceeds MAX_PATH_GRID_CELLS \
                     ({MAX_PATH_GRID_CELLS}); creating an EMPTY map."
                );
                (0, 0, 0)
            }
        }
    }

    /// Row-major index for `(x, y)`, or `None` when out of bounds.
    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || y < 0 || x >= self.width || y >= self.height {
            return None;
        }
        Some((y * self.width + x) as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_field_sees_everything_within_radius() {
        let mut fov = FovMap::new(11, 11);
        fov.compute(IVec2::new(5, 5), 3);
        // Cells within Euclidean radius 3 are visible…
        assert!(fov.is_visible(5, 5)); // origin
        assert!(fov.is_visible(8, 5)); // dist 3
        assert!(fov.is_visible(5, 2)); // dist 3
        assert!(fov.is_visible(7, 7)); // dist² = 8 ≤ 9
                                       // …and cells beyond it are not.
        assert!(!fov.is_visible(9, 5)); // dist 4
        assert!(!fov.is_visible(8, 8)); // dist² = 18 > 9
    }

    #[test]
    fn a_wall_casts_a_shadow_behind_it() {
        // Observer at (3,3); a single wall two cells north at (3,1).
        let mut fov = FovMap::new(7, 7);
        fov.set_opaque(3, 1, true);
        fov.compute(IVec2::new(3, 3), 6);
        assert!(
            fov.is_visible(3, 2),
            "the cell in front of the wall is seen"
        );
        assert!(fov.is_visible(3, 1), "the wall itself is seen");
        assert!(
            !fov.is_visible(3, 0),
            "the cell directly behind the wall is shadowed"
        );
    }

    #[test]
    fn radius_zero_lights_only_the_origin() {
        let mut fov = FovMap::new(5, 5);
        fov.compute(IVec2::new(2, 2), 0);
        assert!(fov.is_visible(2, 2));
        assert!(!fov.is_visible(2, 1));
        assert!(!fov.is_visible(3, 2));
    }

    #[test]
    fn revealed_accumulates_across_moves_but_visible_does_not() {
        let mut fov = FovMap::new(11, 3);
        // Look from the far left…
        fov.compute(IVec2::new(1, 1), 2);
        assert!(fov.is_visible(2, 1));
        assert!(!fov.is_visible(8, 1));
        // …then from the far right.
        fov.compute(IVec2::new(9, 1), 2);
        assert!(!fov.is_visible(2, 1), "visible is cleared each compute");
        assert!(fov.is_visible(8, 1));
        // Both spots stay revealed (explored memory persists).
        assert!(fov.is_revealed(2, 1));
        assert!(fov.is_revealed(8, 1));

        // reset() wipes the explored memory too.
        fov.reset();
        assert!(!fov.is_revealed(2, 1));
        assert!(!fov.is_revealed(8, 1));
    }

    #[test]
    fn from_path_grid_maps_blocked_to_opaque() {
        let mut grid = PathGrid::new(4, 4);
        grid.set_walkable(2, 2, false);
        let fov = FovMap::from_path_grid(&grid);
        assert_eq!(fov.width, 4);
        assert_eq!(fov.height, 4);
        assert!(fov.is_opaque(2, 2));
        assert!(!fov.is_opaque(0, 0));
    }

    #[test]
    fn line_of_sight_blocked_by_a_wall_between() {
        let mut fov = FovMap::new(7, 3);
        fov.set_opaque(3, 1, true);
        let a = IVec2::new(0, 1);
        let b = IVec2::new(6, 1);
        assert!(!fov.line_of_sight(a, b), "a wall between blocks sight");
        // Remove it → clear line.
        fov.set_opaque(3, 1, false);
        assert!(fov.line_of_sight(a, b));
    }

    #[test]
    fn line_of_sight_ignores_the_endpoints() {
        // The target cell being opaque must not report "blocked" — you can see the wall you look at.
        let mut fov = FovMap::new(5, 1);
        fov.set_opaque(4, 0, true); // the endpoint itself
        assert!(fov.line_of_sight(IVec2::new(0, 0), IVec2::new(4, 0)));
        // Same cell → trivially visible.
        assert!(fov.line_of_sight(IVec2::new(2, 0), IVec2::new(2, 0)));
    }

    #[test]
    fn degenerate_and_out_of_bounds_are_safe() {
        // Empty map: every query is false, compute does not panic.
        let mut empty = FovMap::new(0, 0);
        empty.compute(IVec2::new(0, 0), 4);
        assert!(!empty.is_visible(0, 0));

        // Overflowing dimensions collapse to an empty map rather than allocating/panicking.
        let huge = FovMap::new(i32::MAX, 2);
        assert_eq!((huge.width, huge.height), (0, 0));

        // Out-of-bounds origin: no panic, in-bounds cells simply stay dark.
        let mut fov = FovMap::new(5, 5);
        fov.compute(IVec2::new(-3, 10), 4);
        assert!(!fov.is_visible(0, 0));
        // Out-of-bounds queries read false.
        assert!(!fov.is_visible(100, 100));
        assert!(!fov.is_opaque(-1, -1));
    }

    #[test]
    fn pillar_shadow_widens_with_distance() {
        // A single pillar to the east should hide a growing wedge of cells behind it, but leave
        // cells off the shadow line visible.
        let mut fov = FovMap::new(11, 11);
        fov.set_opaque(6, 5, true); // one cell due east of the observer
        fov.compute(IVec2::new(5, 5), 8);
        assert!(fov.is_visible(6, 5), "the pillar is seen");
        assert!(
            !fov.is_visible(7, 5),
            "the cell directly behind the pillar is shadowed"
        );
        // A cell well off the pillar's line stays visible.
        assert!(fov.is_visible(7, 8));
    }
}
