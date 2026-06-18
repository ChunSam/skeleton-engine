# Hexagonal tilemap — TilemapProjection::Hexagonal (v0.29.0, C2 of C1/C2)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, 65 tilemap tests pass (4 new hex), windowed-playtested, CI-equivalent gate green; shipped as v0.29.0 on branch `feat/hex-tilemap` (PR pending push/open).**
**Chain:** `engine-hardening` seq `30` · **Parent:** `HANDOFF_engine-hardening_iso-tilemap_2026-06-18.md` (seq 29)
**Origin:** C2 of the user's "C1=isometric, C2=hex" split. **The iso/hex tilemap item from the
seq-25 feature list is now fully done** (both projections shipped).

---

## What shipped
- **`TilemapProjection::Hexagonal`** (`src/tilemap/mod.rs`) — pointy-top hex grid in **odd-r offset**
  coordinates (odd rows shifted right by half a tile), so the rectangular `tiles[row][col]` array
  maps straight on. `tile_size` = hex flat-to-flat width; cell `(0,0)` center at `origin`.
- **Coordinate math** (all three branch on projection now):
  - `cell_center_world`: `x = origin.x + col*ts + (row odd ? ts/2 : 0)`,
    `y = origin.y + row * ts * (√3/2)` (row pitch = 3/4 of hex height).
  - `cell_at_world`: pixel → fractional axial (`size = ts/√3`) → **`axial_round`** (cube-round) →
    odd-r offset (`col = q + (r - (r&1))/2`, `row = r`). Cube-rounding is exact at hex borders
    where independent rounding isn't.
  - `cell_z`: `-1.0` (hex tiles tessellate without overlap, like orthographic).
- **`Tilemap::cell_render_size()`** (NEW) — sprite size per projection: square `tile_size` for
  ortho/iso, **taller** `(tile_size, tile_size·2/√3)` for hex (pointy-top hexes are taller than
  wide). `TilemapSystem` now sizes tiles with this instead of a hardcoded square.
- Module constant `SQRT_3`. `axial_round` free fn (cube rounding; only returns `(q, r)` so the
  largest-error-is-y case is a no-op).
- Example **`hex_tilemap`** + generated `examples/assets/hex_tiles.png` (grass + sand pointy-top
  hexes). Keyboard cell selection (reactive `set_tile`) + mouse hover-pick (`cell_at_world`).

## Verification
- **65 tilemap unit tests pass**, incl. 4 new hex: odd-row center offset; center↔at_world round-trip
  (5×5, offset origin); off-center → nearest hex; hex `cell_z` fixed.
- **Windowed playtest:** pointy-top hex grid renders with correct odd-r tessellation, sand border +
  grass interior, selected sand hex; arrow keys move the selection live ((4,4)→(3,3), reactive
  `set_tile`). Screenshots captured.
- Full `./scripts/verify.sh` green after the version bump.

## Gotchas
- **Art-gen bug caught in playtest:** first `hex_tiles.png` had the sand cell's polygon in absolute
  atlas coords but received local x → sand rendered fully transparent (sand cells showed as holes).
  Fixed by defining the hex polygon in local cell coords for both cells. *Lesson: eyeball the
  generated atlas, and a transparent tile reads as a "hole" not an error.*
- **Sprite must be taller than wide for pointy-top hex** — hence `cell_render_size`. A square
  `tile_size` sprite would clip the top/bottom vertices.
- Synthetic mouse-move still doesn't reach winit (hover readout "outside" in playtest) — picking is
  unit-tested; keyboard input works live. (Same as the iso handoff.)
- `TilemapProjection` is a plain enum (not `#[non_exhaustive]`); adding `Hexagonal` is a 0.x-MINOR
  change. Internal matches (`cell_center_world`/`cell_at_world`/`cell_z`/`cell_render_size`) all
  updated.

## Files changed
- `src/tilemap/mod.rs` (variant + SQRT_3 + cell_center/at_world/z/render_size hex arms + axial_round
  + 4 tests), `src/tilemap/system.rs` (cell_render_size; moved `glam::Vec2` import into the test
  mod), `examples/hex_tilemap.rs` (new), `examples/assets/hex_tiles.png` (new),
  `Cargo.toml`/`Cargo.lock`, `docs/CHANGELOG.md`, `CLAUDE.md`.

## Risks & Blockers
- None. Additive, default unchanged, tests + playtest green.

## Where to go next (seq-25 feature list, all optional)
- **B1** `TilemapAutotile { mode: Single|Multi }` unification + drop ghost `ConnectRule`.
- **B2** UI Tab/focus keyboard navigation.
- Further wasm audio (named buses / crossfade on wasm).
- crates.io publish (irreversible, explicit go).
- (Stretch) flat-top hex variant / autotile support across iso+hex.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.29.0
cargo test --lib tilemap               # 65 pass
cargo run --example hex_tilemap        # arrows move selection; pointy-top hex grid
```
