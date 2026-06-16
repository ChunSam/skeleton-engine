//! Smoke tests for the pathfinding module.
//!
//! Uses only `PathGrid`, `find_path`, and `find_path_diagonal` — no window,
//! GPU, or event-loop required.
//!
//! Path-reconstruction note: the engine returns the path from the first step
//! *after* start up to and including goal (start itself is excluded).

use engine::{find_path, find_path_diagonal, IVec2, PathGrid};

/// Adjacent cells — path contains just the goal.
#[test]
fn find_path_adjacent() {
    let grid = PathGrid::new(3, 3);
    let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(1, 0));
    assert!(path.is_some());
    let path = path.unwrap();
    // Reconstruction loop excludes start; the only step is the goal itself.
    assert_eq!(path.last(), Some(&IVec2::new(1, 0)));
}

/// Path must navigate around a wall of blocked cells.
#[test]
fn find_path_blocked_only_route() {
    // 5×1 grid with cell (2,0) blocked — no alternate route exists.
    let mut grid = PathGrid::new(5, 1);
    grid.set_walkable(2, 0, false);
    let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(4, 0));
    assert!(
        path.is_none(),
        "expected None when wall blocks the only row"
    );
}

/// Blocked goal cell: expect None.
#[test]
fn find_path_blocked_goal() {
    let mut grid = PathGrid::new(4, 4);
    grid.set_walkable(3, 3, false);
    let path = find_path(&grid, IVec2::new(0, 0), IVec2::new(3, 3));
    assert!(path.is_none());
}

/// When start == goal the path is a single-element vec containing that cell.
#[test]
fn find_path_same_start_and_goal() {
    let grid = PathGrid::new(4, 4);
    let path = find_path(&grid, IVec2::new(2, 2), IVec2::new(2, 2));
    assert!(path.is_some());
    let path = path.unwrap();
    assert_eq!(path.len(), 1);
    assert_eq!(path[0], IVec2::new(2, 2));
}

/// Diagonal pathfinding: path ends at goal on an open grid.
#[test]
fn find_path_diagonal_open_grid() {
    let grid = PathGrid::new(5, 5);
    let path = find_path_diagonal(&grid, IVec2::new(0, 0), IVec2::new(4, 4));
    assert!(path.is_some());
    assert_eq!(path.unwrap().last(), Some(&IVec2::new(4, 4)));
}

/// PathGrid::is_walkable returns false for out-of-bounds coords (no panic).
#[test]
fn is_walkable_oob_returns_false() {
    let grid = PathGrid::new(3, 3);
    assert!(!grid.is_walkable(-1, 0));
    assert!(!grid.is_walkable(0, 99));
}
