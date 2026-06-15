# D-2 — Editor pathfinding overlay (v8.20.0)

## Goal

Add a toolbar-toggled debug overlay that visualizes the **pathfinding grid** derived from every
`Tilemap` entity in the world: blocked cells filled, walkable cells faintly outlined. Reuses the
exact pattern of the D-1 bounds overlay (`DebugDraw` world-space draws, `EditorSettings`-persisted
toggle), so it is small, low-risk, and unit-testable.

## Why this design

- The editor cannot read a `PathGrid` the game built locally (examples call `find_path` with a
  throwaway grid; no grid lives in the World). Rather than require games to insert a grid resource,
  the overlay **builds a `PathGrid` per-frame from each `Tilemap`** via the real
  `PathGrid::from_tilemap(&tm, |id| id != 0)` — i.e. it visualizes exactly what a game following the
  common "0 = floor / non-zero = wall" convention would navigate. This exercises the actual
  pathfinding subsystem (`PathGrid` + `from_tilemap` + `is_walkable`) with zero example changes.

## Scope (additive, native-only, semver-minor)

- `EditorState.show_pathgrid: bool` (native-only field) + `EditorState::new()` init.
- `EditorSettings.show_pathgrid` with `#[serde(default)]` (old config files still load) + `from_state`/`apply_to`.
- `App::draw_pathfinding_overlay()` — query `Tilemap`s, build a `PathGrid`, draw cells via `DebugDraw`:
  - blocked cell → `rect_filled_z` (red, low alpha)
  - walkable cell → `rect` outline (green, low alpha)
- Toolbar checkbox **"Path"** in `docked_toolbar` (next to Grid / Bounds).
- Per-frame call in `ui/mod.rs` guarded by `show_pathgrid` (mirrors `show_bounds`).
- Version bump 8.19.0 → 8.20.0 (Cargo.toml), CLAUDE.md header, CHANGELOG entry.

## Completion criteria

1. `cargo test --lib` green, +1 new test: a 3×3 tilemap with one blocked center yields
   1 filled rect (blocked) + 8 outline shapes (walkable).
2. Full Gate6 green (`fmt --check`, `clippy --all-targets -D warnings`, wasm lib+bins build,
   `test --all-targets`, `RUSTDOCFLAGS=-D warnings doc --no-deps`, `cargo package`).
3. Toggle persists across editor close/open (covered by the existing `editor_settings_round_trip`
   pattern; `show_pathgrid` added to the round-trip).
4. `rust-survivors` unaffected (no public API removed; purely additive editor-internal).

## Out of scope

- A* path *preview* (click start/goal → draw route). Possible follow-up; not this feature.
- Reading a game-supplied `PathGrid` resource. The from-tilemap derivation is enough for the overlay.
