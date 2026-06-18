# Autotile API unification + ConnectRule removal (v0.30.0, B1)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, 65 tilemap tests pass, CI-equivalent gate green; shipped as v0.30.0 on branch `refactor/autotile-unify` (PR pending push/open).**
**Chain:** `engine-hardening` seq `31` · **Parent:** `HANDOFF_engine-hardening_hex-tilemap_2026-06-18.md` (seq 30)
**Origin:** B1 of the seq-25 feature list ("TilemapAutotile { Single|Multi mode } unify + drop ghost ConnectRule"). User: "b1.b2 진행하고 완료되면 머지까지".

---

## What shipped (a contained breaking refactor, behavior-preserving)
The two separate autotile component types are now **one**, matching the dispatch `TilemapSystem`
already did internally (its private `AutotileMode {None,Single,Multi}`):
- **`AutotileMode`** enum (public): `Single { mask_to_tile }` / `Multi { rules: Vec<TerrainRule> }`.
- **`TilemapAutotile { neighborhood, oob_filled, mode }`** — was `{ neighborhood, mask_to_tile,
  oob_filled, connect }`. Builders: `edge_16`/`blob_47` (Single, unchanged signatures), **new
  `multi_edge_16(&[(terrain, base)])`** (Multi), `with_oob_filled`, `rule_for` (pub(super)).
- **Removed `MultiTerrainAutotile`** (folded into `TilemapAutotile` + `AutotileMode::Multi`).
- **Removed `ConnectRule`** — a do-nothing unit-struct marker + `connect` field that the docs called
  a "future extension point" but which never did anything (the real ghost).
- `TilemapSystem` now clones the single optional `TilemapAutotile` and matches on `.mode` (dropped
  the old "two components, Multi-wins precedence" logic — there's one component now).
- `TerrainRule`, `Neighborhood`, `compute_tile_mask`, `compute_tile_mask_typed` unchanged.

## Migration (what callers change)
- `MultiTerrainAutotile::edge_16(&t)` → **`TilemapAutotile::multi_edge_16(&t)`**.
- Reading the single-terrain map: `at.mask_to_tile` → `match at.mode { AutotileMode::Single { mask_to_tile } => .. }`.
- **Single-terrain users (`TilemapAutotile::edge_16(..).with_oob_filled(..)`) are unaffected** —
  `dig_quest` needed no change. Only `multi_terrain` changed one call + import.

## Verification
- **65 tilemap unit tests pass** (single + multi + iso + hex). Test rewrites: a `single_map()`
  helper extracts the `AutotileMode::Single` map for the edge/blob assertions; the old
  "multi-takes-precedence-over-single (both components present)" test became
  `multi_terrain_mode_resolves_via_terrain_base` (one component now).
- Full `./scripts/verify.sh` green after the version bump.
- Examples updated: `multi_terrain` (import + `multi_edge_16` call), doc mentions in
  `multi_terrain`/`gen_multiterrain_sheet`.

## Notes / gotchas
- `TilemapProjection`/`AutotileMode` are plain enums (not `#[non_exhaustive]`) — fine for 0.x.
- `rule_for` is `pub(super)`; the `tests` submodule (descendant of `autotile`) can still call it.
- This is the first deliberately-breaking change since the iso/hex additions; justified by 0.x
  MINOR and the small real-world call surface (no forker call sites in-repo except the one example).

## Files changed
- `src/tilemap/autotile.rs` (unify types, drop ConnectRule/MultiTerrainAutotile, multi_edge_16,
  rule_for, test fixes + single_map helper), `src/tilemap/system.rs` (mode dispatch),
  `src/lib.rs` + `src/tilemap/mod.rs` (exports: -ConnectRule -MultiTerrainAutotile +AutotileMode),
  `examples/games/multi_terrain/multi_terrain.rs`, `examples/gen_multiterrain_sheet.rs` (doc),
  `Cargo.toml`/`Cargo.lock`, `docs/CHANGELOG.md`, `CLAUDE.md`.

## Risks & Blockers
- None. Behavior-preserving (tests green), breaking surface tiny + migrated.

## Where to go next
- **B2 — UI Tab/focus keyboard navigation** (next, this session): Tab/Shift-Tab cycle focus across
  focusable UI widgets (Button/TextInput/Slider/CheckBox), focus ring, Enter/Space activate. See
  `src/ui/`. Then: further wasm audio, crates.io publish.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.30.0
cargo test --lib tilemap               # 65 pass
cargo run --example multi_terrain      # multi_edge_16 path
```
