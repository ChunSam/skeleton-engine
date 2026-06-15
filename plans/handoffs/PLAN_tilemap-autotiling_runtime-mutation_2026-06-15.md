# PLAN — Tilemap autotiling + runtime mutation (v8.2.0)

**Date:** 2026-06-15
**Chain:** `tilemap-autotiling` seq 1
**Branch:** `feat/v8.2-tilemap-autotiling`
**Status:** APPROVED — implementing
**Vision tie-in:** `docs/VISION.md` (genre-agnostic 2D breadth, fork-friendly, validate via a playable example), `docs/NEXT_WORK.md` (this closes the deferred candidates *tilemap autotiling* + *runtime tilemap mutation*).

---

## 1. Goal & acceptance

| Item | Detail |
|---|---|
| **Feature** | (1) runtime tile mutation API with render + collision auto-reflect; (2) neighbor-bitmask autotiling. |
| **Acceptance** | Playable example `dig_quest` — digging an adjacent tile updates the **autotiled outline immediately + updates collision so the player can move into the dug space**; reach the gem = win / `R` reset. "Works in real play" is the bar for done (VISION). |
| **Version** | **minor v8.2.0** — all additive (no `Tilemap` struct field changes; see §2). |

## 2. Key design decisions (all non-breaking)

| Decision | Rationale |
|---|---|
| **Autotile rules live in a separate component `TilemapAutotile`, not a new `Tilemap` field.** | `Tilemap` has all-public fields → adding a field breaks struct-literal construction (semver-breaking). A sibling component on the same entity is **0-breaking** and matches the ECS style. |
| **Change detection: `TilemapSystem` diffs a per-entity cached `tiles` snapshot (no `version`/`dirty` field on `Tilemap`).** | Same reason — can't add a field to `Tilemap`. The system holds the last-seen `tiles` and diffs per frame. Negligible for small 2D maps (perf note kept). |
| **Logical/display separation.** | With autotile, a non-zero `tiles` value = "filled terrain"; the *display* atlas id is computed from the neighbor mask. Without autotile, behavior is unchanged (`value-1` = atlas id). |
| **Neighborhood `Edge4` (16-tile) and `Blob8` (47-tile) both supported generically; shipped default + example use `Edge4`.** | Mask computation differs only in 4 vs 8 bits → shared code. A legible 47-blob sheet is hard to generate procedurally; `Edge4`/16-tile draws borders procedurally and is legible at low asset cost. `Blob8`/47 is supported by the same code path for forks who supply a sheet + mask map (or a follow-up). |
| **Incremental collision via new `sync_static_from_tilemap` + `TileColliderIndex`.** | Keep `add_static_from_tilemap` (one-shot) unchanged (platformer unaffected). The sync API diffs desired-set vs index and adds/removes only changed cells. Empty index on first call = full build, so the example uses sync from the start. Reuses `remove_body` (v8.1.6 already purges one_way set + refreshes the query pipeline). |
| **PathGrid runtime sync is out of scope.** | `dig_quest` has no enemies/pathfinding. Defer to the example that needs it ("fix only the gap the example hits"). |

## 3. New public API

```rust
// src/tilemap.rs — additive (methods + new component; no Tilemap field change)
impl Tilemap {
    pub fn set_tile(&mut self, row: usize, col: usize, value: u32) -> bool; // bounds-checked; true if changed
    pub fn get_tile(&self, row: usize, col: usize) -> Option<u32>;
    pub fn cell_at_world(&self, world_pos: Vec2) -> Option<(usize, usize)>;  // world → cell (digging/editing)
    pub fn cell_center_world(&self, row: usize, col: usize) -> Vec2;          // cell → world center
    pub fn dims(&self) -> (usize, usize); // (rows, cols); jagged → max col
}

pub enum Neighborhood { Edge4, Blob8 }

pub struct ConnectRule { /* default: any non-zero connects to any non-zero */ } // v1 = single terrain

pub struct TilemapAutotile {
    pub neighborhood: Neighborhood,
    pub mask_to_tile: std::collections::HashMap<u8, u32>, // mask → display atlas id
    pub oob_filled: bool,                                  // treat out-of-bounds neighbors as filled (default false)
    pub connect: ConnectRule,
}
impl TilemapAutotile {
    pub fn edge_16(base_atlas_id: u32) -> Self; // identity mask→(base+mask), matches the generated sheet
    pub fn blob_47(base_atlas_id: u32) -> Self; // standard blob layout (fork-facing)
}

// pure function — primary unit-test target
pub fn compute_tile_mask(tiles: &[Vec<u32>], row: usize, col: usize,
                         nb: Neighborhood, oob_filled: bool) -> u8;
```

```rust
// src/physics/world/tile_collider.rs — additive
pub struct TileColliderIndex { /* HashMap<(usize,usize),(BodyHandle,ColliderHandle)> */ }
impl TileColliderIndex { pub fn new() -> Self; pub fn len(&self) -> usize; pub fn is_empty(&self) -> bool; }

impl PhysicsWorld {
    /// Diff `tilemap` against `index` → add/remove only changed cells. Empty index = full build.
    pub fn sync_static_from_tilemap(
        &mut self, tilemap: &Tilemap, pixels_per_unit: f32,
        collider_for: impl FnMut(u32) -> Option<TileCollider>,
        index: &mut TileColliderIndex,
    );
}
```

Re-export from `lib.rs` / `physics/mod.rs`: `TilemapAutotile`, `Neighborhood`, `ConnectRule`, `TileColliderIndex`, `compute_tile_mask`.

## 4. Phased implementation

| Phase | Content | Files | Tests |
|---|---|---|---|
| **P1 — mutation + reactive render** | `set_tile`/`get_tile`/`cell_at_world`/`cell_center_world`/`dims`. Rework `TilemapSystem` to a **per-cell entity map + cached `tiles` diff** → spawn/despawn/update `UvRect` for changed cells only. Non-mutating maps behave identically (first frame spawns all). | `src/tilemap.rs` | set_tile bounds/return, diff updates only changed cells, value→0 despawns, coord round-trip |
| **P2 — autotiling** | `Neighborhood`/`TilemapAutotile`/`compute_tile_mask`/`edge_16`/`blob_47`. `TilemapSystem` uses `mask_to_tile[mask]` when `TilemapAutotile` present. **A changed cell also dirties its 4/8 neighbors** for UV recompute. | `src/tilemap.rs` (+ optional `src/tilemap/autotile.rs`) | each neighbor config → mask value, edge_16 identity, oob_filled true/false, neighbor propagation |
| **P3 — incremental collision** | `TileColliderIndex` + `sync_static_from_tilemap`; reuse `remove_body`. | `src/physics/world/tile_collider.rs`, `world.rs` | empty index = same count as add_static, removing a cell removes exactly 1 collider, re-add, one_way retained |
| **P4 — asset generator** | `gen_autotile_sheet.rs` — Edge4/16-tile sheet, procedurally drawing borders per mask (legible). Mirror `gen_platform_tiles.rs`. | `examples/gen_autotile_sheet.rs` | (eyeball output) |
| **P5 — playable example** | `examples/games/dig_quest/` — see §5. | new dir | (real-play verification) |
| **P6 — docs/version** | CHANGELOG 8.2.0, CLAUDE.md module-map rows (autotile/sync), NEXT_WORK.md candidate entry, REFERENCE.html (optional). | docs | — |

**Parallelizable:** P3 (collision) and P4 (asset gen) are independent of P1/P2 (different files) — can run concurrently with the P1→P2 chain. P5 depends on P1–P4. P2 depends on P1's reactive system.

## 5. Example spec — `dig_quest`

- **Loop:** destructible terrain. Arrow-key move; dig key toward an adjacent dirt tile → `set_tile(r,c,0)`. Autotiled outline updates instantly; `sync_static_from_tilemap` removes the collider so the player enters the dug space. Reach gem `◆` = win, `R` = reset.
- **Validates:** runtime mutation → render (autotile) + collision, **both** live, proven in real play.
- **Fix the API if it feels awkward** (VISION) — the dig-input → cell → two-system-update call site is the litmus test.
- **Asset:** 16-tile sheet from `gen_autotile_sheet`.
- **Physics:** player via `CharacterController` (platformer pattern); dirt = solid tiles.

## 6. Verification & gates (CLAUDE.md)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo build --target wasm32-unknown-unknown   # lib+bins
cargo test --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
# = ./scripts/verify.sh
```
- Play verification: native run + screenshot (playtest harness). **`cargo clean -p skeleton-engine` before any GUI re-playtest** (stale-binary trap).
- Autotile + collision are mostly unit-testable without a GPU (pure `compute_tile_mask` + index diff); only the rendered result needs eyeballing.

## 7. Scope boundaries (deferred — do not re-litigate)

- **Multi-terrain autotiling** (distinct tile types each autoconnecting): v1 = single terrain (non-zero connects to non-zero). `ConnectRule` is the extension point.
- **Blob8/47 sheet setup:** code path supported; shipped sheet/example use Edge4. 47-blob asset is fork/follow-up.
- **PathGrid runtime sync:** out of scope (no enemies in the example).
- **Editor tile painting:** the docked editor reusing `set_tile` + the reactive system for a paint mode is a natural follow-up (editor-arc synergy), but this arc fixes only the gap the example hits.

## 8. Risks

- **Diff-based change detection cost:** per-frame full-grid compare. Negligible for small 2D maps; if profiling shows cost, a future minor adds a `version` field (with a constructor change).
- **Autotile neighbor propagation:** a cell change affects neighbors' UVs — missing neighbors in the dirty set breaks the outline. Locked by P2 tests.
- **Collision sync kind change:** v1 leaves cells present in both old/new untouched (groups/one_way change not reflected). Dig only adds/removes → harmless; noted in a test comment.

## 9. Deliverables

6 new public APIs (§3) + re-exports, reactive `TilemapSystem`, incremental collision sync, `dig_quest` example + asset generator, unit tests (P1–P3), CHANGELOG/CLAUDE.md/NEXT_WORK updates. **v8.2.0, non-breaking.**

## 10. Governance

Interactive session (not autonomous `/loop`). Opus supervises + runs `./scripts/verify.sh` independently + commits between phases; Sonnet subagents implement (explicit `model:` always — [[new-model-subagent-incompat]]). Never self-merge; PRs wait for the user's "머지 확인". Korean prose to the user, English code/docs.
