# Isometric tilemap — TilemapProjection (v0.28.0, C1 of C1/C2)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, 61 tilemap tests pass (4 new iso), windowed-playtested, CI-equivalent gate green; shipped as v0.28.0 on branch `feat/iso-tilemap` (PR pending push/open).**
**Chain:** `engine-hardening` seq `29` · **Parent:** `HANDOFF_engine-hardening_wasm-save-verify_2026-06-18.md` (seq 28)
**Origin:** C1 of the user's "C1=isometric, C2=hex" split of the seq-25 iso/hex-tilemap item. **C2
(hex) is the immediate next step.**

---

## What shipped
- **`TilemapProjection`** enum (`src/tilemap/mod.rs`): `Orthographic` (default) / `Isometric`.
  Added a `projection` field to `Tilemap` + `with_projection` builder. Default = `Orthographic`,
  so **every existing tilemap is byte-identical**.
- **Projection-aware coordinates** (`cell_center_world` / `cell_at_world` branch on `projection`):
  - Isometric = 2:1 diamond: `w = tile_size`, `h = tile_size/2`; cell `(0,0)` center at `origin`;
    `center = origin + ((col-row)*w/2, (col+row)*h/2)`.
  - Isometric picking inverts that (`a = relx/(w/2) = col-row`, `b = rely/(h/2) = col+row`,
    `col=(a+b)/2`, `row=(b-a)/2`) and **rounds** to the nearest cell — diamond cells tessellate, so
    independent rounding is exact.
- **`Tilemap::cell_z(row, col)`** — render depth: `-1.0` (orthographic, behind sprites) /
  `row + col` (isometric painter's order, back-to-front; higher z draws on top).
- **`TilemapSystem`** now positions tiles via `cell_center_world` + `cell_z` at the one
  `spawn_tile_entity` call site (handles both projections; orthographic output unchanged).
- Re-exported `TilemapProjection` at the crate root.
- Example **`iso_tilemap`** (`examples/iso_tilemap.rs`) — diamond grid (water border + grass),
  keyboard cell selection (reactive `set_tile` paints the selected cell water), mouse hover-pick
  (`cell_at_world` on `Camera::screen_to_world(InputState::cursor())`). Generated diamond atlas
  `examples/assets/iso_tiles.png` (pure-Python `zlib` PNG: grass + water 2:1 diamonds).

## Verification
- **61 tilemap unit tests pass**, incl. 4 new: iso cell centers form a diamond; iso
  center↔at_world round-trip (4×4, offset origin); iso off-center pick → nearest diamond; iso/ortho
  `cell_z`.
- **Windowed playtest:** the diamond grid renders correctly (tessellation + water-border/grass +
  depth order); arrow keys move the selected water diamond (4,4)→(2,3) live (reactive `set_tile` +
  `cell_center_world` placement). Screenshots captured.
- Full `./scripts/verify.sh` green after the version bump.

## Gotchas / notes
- **Synthetic mouse-move doesn't reach winit** (same class as synthetic clicks): the live mouse
  hover-pick readout stayed "(outside grid)" under `CGWarpMouseCursorPosition`/osascript. Not a bug
  — `cell_at_world` iso picking is unit-tested (round-trip + off-center + bounds). Keyboard input
  (`key code N`) DOES reach winit, so the selection demo is live-verified.
- **z model for iso:** tiles use depth z = `row+col` (positive, ascending). This differs from the
  orthographic "all at -1, behind everything" model — iso scenes depth-sort *everything*, so a game
  placing units on an iso floor should give them a depth-based z too. Documented on `cell_z`.
- Iso art convention: a square sprite (`scale = tile_size`) whose art is a diamond of width
  `tile_size`, height `tile_size/2`, centered; transparent corners overlap harmlessly.

## Files changed
- `src/tilemap/mod.rs` (enum + field + builder + coord/z branches + 4 tests), `src/tilemap/system.rs`
  (`spawn_tile_entity` uses cell_center_world/cell_z), `src/lib.rs` (export), `examples/iso_tilemap.rs`
  (new), `examples/assets/iso_tiles.png` (new), `Cargo.toml`/`Cargo.lock`, `docs/CHANGELOG.md`,
  `CLAUDE.md`.

## Risks & Blockers
- None. Additive, default unchanged, tests + playtest green.

## Where to go next
- **C2 — hex tilemap:** add `TilemapProjection::Hexagonal` (pointy- or flat-top). Decide axial vs
  offset coords; `cell_center_world` = hex layout, `cell_at_world` = cube-round picking; `cell_z`
  likely by row (rows overlap in pointy-top). New example + diamond/hex art. Mirror this PR's shape.
- Then (seq-25 list): B1 TilemapAutotile mode unify, B2 UI Tab/focus, further wasm audio, crates.io.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.28.0
cargo test --lib tilemap               # 61 pass
cargo run --example iso_tilemap        # arrows move selection; diamond grid
```
