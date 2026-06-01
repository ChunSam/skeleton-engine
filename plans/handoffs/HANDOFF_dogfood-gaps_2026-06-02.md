# Close 4 playable-example dogfooding gaps (v1.1.0) + platformer tileset fix

**Date:** 2026-06-02
**Status:** COMPLETED
**Bead(s):** none
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `dogfood-gaps` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** `none — first in chain`

## Related Handoffs

These are sibling work streams (the individual playable examples that *surfaced*
the gaps closed here), not parents of this chain:

- `plans/handoffs/HANDOFF_platformer-example-game_2026-05-30.md` — original platformer (surfaced A: one-way + tilemap→physics)
- `plans/handoffs/HANDOFF_maze-escape-example-game_2026-05-31.md` — maze (surfaced B: blackboard path caching)
- `plans/handoffs/HANDOFF_scene-flow-ui-interaction_2026-05-31.md` — scene flow (surfaced E: cross-scene state)
- `plans/handoffs/HANDOFF_sokoban-example-game_survivor-example_2026-06-01.md` — most recent prior session (candidate F)

## Reference Documents

- `CLAUDE.md` — project conventions, module map (updated to v1.1.0 this session)
- `docs/VISION.md` — the dogfooding core loop (feature not done until a playable example exercises it)
- `docs/NEXT_WORK.md` — candidate×example matrix; rows A/B/E marked fixed this session
- `docs/HANDOFF.md` — per-phase dev history; new dated entry added this session
- `docs/PATTERNS.md` — ECS borrow workaround, render-layer separation, system order

## The Goal

skeleton-engine is a fork-friendly, genre-agnostic 2D engine validated by playable
examples. The v1.0.0 playable-examples program (A–F) shipped but left **four engine
gaps** recorded in `docs/NEXT_WORK.md` — things the examples couldn't exercise cleanly.
This session closed all four as an **additive v1.1.0**, with each fix validated by
editing the existing example that first surfaced it (no new examples). A follow-on
visual fix gave the platformer a seamless tileset after the Tilemap refactor exposed
that the original art isn't a tileset.

## Where We Are

- **Branch `feat/example-dogfood-gaps`, PR #3 open** (base `main`), pushed through commit `2b622f2`. Working tree clean.
- All four gaps implemented, unit-tested, and example-validated. Version bumped `1.0.0` → `1.1.0` in `Cargo.toml` and `CLAUDE.md`.
- **#1 Blackboard path caching** — `BlackboardValue::Path(Vec<IVec2>)` + `Blackboard::set_path`/`get_path`; enum now `#[non_exhaustive]`. `maze_escape` caches the whole A* path, recomputes only when the player's goal tile changes.
- **#4 Persistent resources** — `World::take_resource_erased`/`insert_resource_erased` (type-erased `Box<dyn Any>` move) + `App::register_persistent::<T>()`. `reload_scene` lifts registered resources out of the old World before `World::new()` and re-inserts last. `scene_flow` dropped its `Arc<Mutex<StatsData>>` workaround.
- **#3 Tilemap→physics** — `PhysicsWorld::add_static_from_tilemap(tilemap, ppu, collider_for)` + `TileCollider` descriptor (`solid`/`solid_with`/`one_way`). `platformer` level is now an ASCII `LEVEL` → `Tilemap` driving both rendering and collision.
- **#2 One-way platforms** — `PhysicsWorld::set_one_way`/`is_one_way` + `CharacterController::request_drop`/`is_dropping` (+ `drop_timer`, `DROP_DURATION=0.2`). `move_character` builds a Rapier `QueryFilter` predicate excluding one-way colliders when ascending/dropping/below-top. `platformer` adds a one-way tile (`'O'`) and an S/Down drop key.
- **Test count: lib 253 + doctest 34 pass** (was 248 lib before; +5: blackboard path roundtrip, erased-resource roundtrip, take-erased-missing, tilemap collider count/coords, one-way descend/ascend/drop).
- **User verified interactively:** maze chase ✅, S-drop ✅, scene stats preserved ✅. Platformer one-way/collision works; only the **tile rendering** needed the tileset fix.
- **Tileset fix:** confirmed via headless dump that tile transforms are contiguous (`x=32/96/160, scale 64`) — NO spacing bug. Generated `examples/games/platformer/assets/platform_tiles.png` (seamless, textured, no per-tile borders).
- `rust-survivors` (`/Users/jkl/Projects/rust-survivors`) rebuilds clean against the `BlackboardValue` enum change.
- wasm: lib + `maze_escape`/`scene_flow` examples build; `platformer` is native-only (imports `rapier2d`; physics is `#[cfg(not(wasm32))]`) — pre-existing.
- The plan file lives at `/Users/jkl/.claude/plans/4-glistening-catmull.md` (approved, full implementation plan).
- `scripting.rs:397` already had a `_ => continue` arm, so the `BlackboardValue` variant addition needed no scripting changes.
- The original `tiles.png` is still present and unused by the platformer now; left in place (harmless) in case the render approach is reverted.
- `maze_escape` reset path uses sentinel `i32::MIN` for `path_goal_x/y` to force recompute on respawn (path cache invalidation).
- Throwaway `examples/_tilecheck.rs` was used twice (pixel-bbox analysis, then headless TilemapSystem dump, then the PNG generator) and **deleted** — not committed.

## What We Tried (Chronological)

1. **Grilled the scope** (`/grill-me`): user chose **all 4 gaps**, **validate by editing existing examples**, **#2 = pass-up + press-down drop-through**, **#3 = rebuild platformer level as a real Tilemap**, **#4 = persistent resource registry** (not documenting the Arc pattern). Plan written to `/Users/jkl/.claude/plans/4-glistening-catmull.md`, approved.
2. **#1 first (lowest risk).** Added `Path` variant + `#[non_exhaustive]`. Checked `scripting.rs:397` already had a `_ =>` arm → safe. Rewrote `maze_escape`'s `ComputePathToPlayer` (cache full path + goal tile, `path_idx`) and `FollowPathStep` (advance index by proximity). Reset path on respawn via sentinel `i32::MIN`. Test + clippy pass.
3. **#4 next.** Added type-erased World move API; `App.persistent_resources: Vec<TypeId>` + `register_persistent`. Wrapped `reload_scene` to drain→reinsert. Refactored `scene_flow` (4 scenes + 4 systems): removed `SceneFlowStats` Arc wrapper, made `StatsData` a plain resource, scenes take `new()` with no args, `mark_enter`/`mark_exit` free fns on the resource, `main` does `insert_resource` + `register_persistent` before `set_scene`. Test of erased roundtrip pass.
4. **#3 next.** Read `tilemap.rs` to match `TilemapSystem`'s tile-center math exactly. Added `add_static_from_tilemap`. Rebuilt `platformer`: ASCII `LEVEL` + `level_tiles()`, `spawn_level()`, removed `spawn_platform`+`PLATFORMS`, added `TilemapSystem`. Test asserts collider count + first tile world→physics coords.
5. **#2 last (highest risk).** First test used `controller.grounded` and **failed** — grounded false on descent. Switched test to step-then-read-translation. Failed again: **descent passed through**. Root cause: `move_character` queries `query_pipeline`, which is empty until `step()` runs → added an initial `pw.step(dt)` in the test helper. Then all 3 scenarios (block-from-top / pass-from-below / drop) pass. Also changed `add_static_from_tilemap`'s closure return from `Option<CollisionGroups>` to `Option<TileCollider>` and updated the #3 test.
6. **Verification + docs + v1.1.0 bump.** fmt, lib 253 + doctest 34, clippy `--all-targets -D warnings`, rustdoc `-D warnings`, wasm, rust-survivors rebuild — all green. Updated CHANGELOG/NEXT_WORK/HANDOFF/CLAUDE. Committed `ed4e11b`, pushed, opened PR #3.
7. **User ran platformer → "블럭 분리됨" (blocks separated).** Investigated: quads provably 64px contiguous. Wrote a throwaway `examples/_tilecheck.rs` using the `image` crate to measure per-cell opaque bbox of `tiles.png` → **the original art is discrete object sprites with transparent margins** (ground idx0 = `x[7..30] y[6..31]`, one-way idx8 = `x[7..30] y[3..24]`), not a seamless tileset. Generated a flat-color seamless `platform_tiles.png`, pointed the platformer at it. Committed `ee0a424`.
8. **User ran again → flat boxes with grid lines ("png not applied correctly").** Two issues: (a) the stone tile's 1px full border formed a dark grid line between tiles, (b) flat colors looked like placeholder boxes. Also ran a **headless `TilemapSystem` dump** to PROVE spacing is contiguous (x=32/96/160, scale 64) — the big gaps are the level's intended jump gaps. Regenerated `platform_tiles.png` with no side borders + top-highlight/bottom-shadow + deterministic speckle/grain. Removed throwaway. Committed `2b622f2`.
9. **User asked why not reuse `tiles.png`** — explained the discrete-object-sprite finding and offered 3 options (keep generated / revert render to stretched AtlasSprite + keep helper for collision / import external seamless tileset). Awaiting choice.

## Key Decisions

- **`BlackboardValue` made `#[non_exhaustive]`** alongside the new variant: adding a variant to a public enum is technically minor-breaking; the attribute future-proofs further additions. Verified no exhaustive match downstream (`scripting.rs` has `_ =>`; `rust-survivors` has none).
- **`TileCollider` over `Option<CollisionGroups>`** for the tilemap helper closure: needed to express one-way per tile, so a small descriptor with `solid()`/`one_way()` constructors handles both #3 and #2 in one API.
- **One-way via a `HashSet<ColliderHandle>` + `move_character` predicate**, not Rapier collision groups: keeps groups free for normal use; the predicate decides per-frame using ascend/drop/below-top, which groups can't express.
- **Drop-through as a timed window** (`drop_timer`, 0.2s) rather than tracking which platform: simpler, and once the character clears the platform its bottom is below the top so it won't re-collide.
- **Persistent resources re-inserted LAST** in `reload_scene` so they win over engine default re-inserts of the same type.
- **Platformer level reworked to a real Tilemap** (user's grilled choice) — proves `add_static_from_tilemap` actually drives a level, not just a unit test.
- **Generated a procedural seamless tileset** rather than reverting render: keeps the "level is one Tilemap" story intact. This is the open trade-off the user is weighing (see Open Questions).

## Evidence & Data

### Commits (PR #3, branch `feat/example-dogfood-gaps`)

| Hash | Summary |
|---|---|
| `ed4e11b` | feat: close 4 playable-example dogfooding gaps (v1.1.0) |
| `ee0a424` | fix(platformer): use a seamless tileset so Tilemap tiles render contiguous |
| `2b622f2` | fix(platformer): make the seamless tileset textured, drop per-tile borders |

### `tiles.png` per-cell opaque bbox (why it's not a tileset; 128×128, 4×4, 32px cells)

| idx | opaque x | opaque y | note |
|---|---|---|---|
| 0 ground | 7..30 | 6..31 | right-shifted, left gap 7px |
| 1 block | 5..29 | 17..31 | art only in lower half |
| 8 one-way | 7..30 | 3..24 | sits high, ≠ idx0 vertical range |
| 15 | 1..23 | 1..21 | fill 57/1024 (mostly empty) |

### Headless `TilemapSystem` dump (proves contiguity — no spacing bug)

```
pos=(32.0,32.0) scale=(64.0,64.0)
pos=(96.0,32.0) scale=(64.0,64.0)
pos=(160.0,32.0) scale=(64.0,64.0)
```
Quads span 0–64, 64–128, 128–192 → adjacent tiles touch exactly.

### One-way unit test scenarios (`src/physics/world.rs`, ppu=1, platform y=2.0 half 0.5 → top 1.5)

| start_y | desired_y | one_way | drop | result | assert |
|---|---|---|---|---|---|
| 0.5 | +0.6 | yes | no | ~1.0 | blocked from top (< 1.05) |
| 0.5 | +0.6 | no | no | ~1.0 | solid blocks (< 1.05) |
| 3.0 | −1.5 | yes | no | <2.0 | passes up from below |
| 3.0 | −1.5 | no | no | >2.8 | solid blocks from below |
| 1.0 | +0.6 | yes | yes | >1.4 | drops through |
| 1.0 | +0.6 | no | yes | <1.05 | solid ignores drop |

### New public API surface (v1.1.0) — exact signatures

```rust
// src/behavior.rs
#[non_exhaustive] enum BlackboardValue { Bool, Float, Int, Vec2, String, Path(Vec<IVec2>) }
Blackboard::set_path(&mut self, key: &str, v: Vec<IVec2>)
Blackboard::get_path(&self, key: &str) -> Option<&[IVec2]>

// src/ecs/world.rs
World::take_resource_erased(&mut self, type_id: TypeId) -> Option<Box<dyn Any>>
World::insert_resource_erased(&mut self, type_id: TypeId, resource: Box<dyn Any>)

// src/app.rs
App::register_persistent<T: 'static>(&mut self)

// src/physics/world.rs
struct TileCollider { groups: CollisionGroups, one_way: bool }
TileCollider::solid() / solid_with(groups) / one_way()
PhysicsWorld::set_one_way(&mut self, handle: ColliderHandle, one_way: bool)
PhysicsWorld::is_one_way(&self, handle: ColliderHandle) -> bool
PhysicsWorld::add_static_from_tilemap(&mut self, tilemap: &Tilemap, pixels_per_unit: f32,
    collider_for: impl FnMut(u32) -> Option<TileCollider>) -> Vec<(RigidBodyHandle, ColliderHandle)>

// src/physics/character.rs
CharacterController::request_drop(&mut self)   // sets drop_timer = DROP_DURATION (0.2s)
CharacterController::is_dropping(&self) -> bool
```

### Platformer level map (`examples/games/platformer/platformer.rs`, 24×10, tile 64px, origin (0,0))

```
row3 .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .   (goal floats ~col20)
row4 .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  B  B  B  B  B  B  B  B  .   cols15-22 stone
row5 .  .  O  O  O  .  .  .  .  .  .  B  B  B  .  .  .  .  .  .  .  .  .  .   cols2-4 one-way, 11-13 stone
row6 .  .  .  .  .  .  .  B  B  B  .  .  .  .  .  .  .  .  .  .  .  .  .  .   cols7-9 stone
row7 G  G  G  G  G  G  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .  .   cols0-5 ground
```
Tile ids: `G`=1 (atlas idx0 ground), `B`=2 (idx1 stone), `O`=9 (idx8 wood one-way), `.`=0 empty.
Closure: `0 => None; TILE_ONEWAY => TileCollider::one_way(); _ => TileCollider::solid()`.
Player start (160,380) col2 above ground; goal (1312,224) col20 above row4. Big gaps between
runs are intended jump gaps; same-row runs are contiguous.

### scene_flow #4 before/after

- Before: `SceneFlowStats { data: Arc<Mutex<StatsData>> }`, cloned into every `*Scene::new(stats)`, re-`insert_resource`d each `on_enter`.
- After: `StatsData` plain resource; `main` does `app.world.insert_resource(StatsData::default()); app.register_persistent::<StatsData>();`; scenes are `MenuScene::new()` etc.; `mark_enter(world, "Menu")` free fns read `world.resource_mut::<StatsData>()`.

### Verification matrix (final, all green)

| Check | Result |
|---|---|
| `cargo fmt --check` | clean |
| `cargo test --lib` | 253 passed |
| `cargo test` (doctest) | 34 passed (19 ignored) |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --lib` | clean |
| wasm lib + maze/scene_flow examples | build |
| `rust-survivors` rebuild | clean |

## Code Analysis

- `move_character` (`src/physics/world.rs:~565`): screen coords (+y down) → `up = (0,-1)`. New logic computes `char_bottom = shape.compute_aabb(&col_pos).maxs.y`, `moving_down = desired.y > 1e-6`, `drop_active = drop_timer > 0` (then decrements). Predicate: non-one-way → always include; one-way → exclude if `drop_active || !moving_down`, else include only if `char_bottom <= platform_top + 0.05` (`platform_top = collider.compute_aabb().mins.y`).
- `QueryFilter::default().exclude_collider(col).predicate(&pred)` — predicate is `&dyn Fn(ColliderHandle, &Collider) -> bool`. Borrow-safe: captures `&self.one_way_colliders` (shared) alongside the other shared field borrows passed to `move_shape`.
- `TilemapSystem` tile center: `x = origin.x + col*tile_size + tile_size*0.5`, same for y; `scale = splat(tile_size)`, `z=-1.0`. `add_static_from_tilemap` mirrors this exactly so colliders align with sprites.
- `World.resources: HashMap<TypeId, Box<dyn Any>>` — `take_resource_erased`/`insert_resource_erased` move boxes by `TypeId` without knowing the static type (needed because `reload_scene` is generic over registered types).
- `reload_scene` (`src/app.rs:~700`): only `SceneCmd::Replace` resets the World; `Push`/`Pop` keep it. So persistence only matters across `Replace`. Sequence: `apply_scene_cmd(Replace)` → drain scene_stack → `reload_scene` (take persistent → `World::new()` → re-insert defaults → re-insert persistent last) → `new_scene.on_enter`. Persisted resource must already exist in the World before the first `set_scene`.
- `maze_escape` path follow: `ComputePathToPlayer` stores `path` (Path), `path_idx`, `path_goal_x/y`; recompute iff `cached_goal != current goal || idx >= len`. `FollowPathStep` reads `path[idx]`, moves toward `tile_center(wp)`, advances `path_idx` when within `TILE*0.25`. No A* runs in the follow step.
- `find_path` (`src/pathfinding.rs`) excludes the start tile (path[0] is the first step toward goal) — relied on by the index-based follow.
- `add_static_from_tilemap` clamps `ppu` with `.max(f32::MIN_POSITIVE)`; half-extent = `(tile_size*0.5)/ppu`; position = `Vec2::new(center_x, center_y)/ppu`. One-way tiles call `self.one_way_colliders.insert(pair.1)`.
- The one-way predicate is borrow-safe because `&self.one_way_colliders`, `&self.rigid_body_set`, `&self.collider_set`, `&self.query_pipeline` are all disjoint shared field borrows; `controller.inner.move_shape` borrows `controller` mutably (a separate binding).

## Things NOT done / deliberately deferred

- **No row-merging** in `add_static_from_tilemap` — one box per solid tile. Fine for these small maps; flagged for large maps.
- **No new playable example** — the dogfooding rule was satisfied by editing the examples that surfaced each gap (user's choice).
- **Entity Generation v2** and **dependency security follow-up** remain cancelled/archived (out of scope; see `docs/NEXT_WORK.md` alignment table).
- **wasm one-way/physics** untested on wasm because the platformer is native-only by pre-existing cfg-gating — not a regression.

## Files Changed

### Source code
- `src/behavior.rs` — `BlackboardValue::Path` + `#[non_exhaustive]`; `set_path`/`get_path`; `IVec2` import; unit test.
- `src/ecs/world.rs` — `take_resource_erased`/`insert_resource_erased`; 2 tests.
- `src/app.rs` — `persistent_resources` field (both constructors); `register_persistent`; `reload_scene` drain/reinsert.
- `src/physics/world.rs` — `TileCollider`; `one_way_colliders` set; `set_one_way`/`is_one_way`; `add_static_from_tilemap`; one-way predicate in `move_character`; 2 tests.
- `src/physics/character.rs` — `drop_timer`, `DROP_DURATION`, `request_drop`/`is_dropping`.
- `src/physics/mod.rs`, `src/lib.rs` — re-export `TileCollider`.

### Examples
- `examples/games/maze_escape/maze_escape.rs` — path caching in `ComputePathToPlayer`/`FollowPathStep`; respawn invalidation; dropped `BlackboardValue` import.
- `examples/games/scene_flow/scene_flow.rs` — removed `Arc<Mutex>`; `StatsData` persistent resource; scenes `new()` arg-free; `register_persistent` in `main`.
- `examples/games/platformer/platformer.rs` — ASCII `LEVEL` → `Tilemap`; `spawn_level`; one-way tile + S/Down drop; HUD text; uses `platform_tiles.png`.

### Data & assets
- `examples/games/platformer/assets/platform_tiles.png` — generated seamless 4×4 tileset (ground idx0, stone idx1, wood one-way idx8).

### Docs / config
- `Cargo.toml`, `CLAUDE.md` — version 1.1.0; CLAUDE module map rows updated.
- `docs/CHANGELOG.md`, `docs/NEXT_WORK.md`, `docs/HANDOFF.md` — release notes / gaps marked fixed / new dev-history entry.

## User Feedback & Preferences

- Wanted **all 4 gaps** in one pass, validated by **editing existing examples** (not new ones).
- Chose **press-down drop-through** for one-way (not just pass-from-below).
- Chose **rebuild platformer level as a real Tilemap** (the heavier option) over a minimal add-on.
- Chose a **persistent resource registry** API over documenting the `Arc<Mutex>` pattern.
- Said "진행해" to commit/PR without further review once verification passed.
- Reported the platformer visual bug precisely and persistently ("블럭 분리됨", then "png가 정상 적용 안 됨") — cares about the example *looking* right, not just compiling.
- Asked **why not reuse the existing `tiles.png`** — wants the rationale, implying possible preference to keep the original art. **This is unresolved.**

## Where We're Going

1. **Resolve the tileset choice** (user asked why not reuse `tiles.png`). Pick one: (a) keep the generated `platform_tiles.png`; (b) revert platformer *rendering* to stretched `AtlasSprite` per platform but keep `add_static_from_tilemap` for collision; (c) import an external seamless tileset. Apply, re-verify, push.
2. **Get user confirmation** that the platformer renders acceptably after the choice.
3. **Merge PR #3** once visuals are signed off.
4. Optional: update `docs/HANDOFF.md` entry if the tileset approach changes.

## Risks & Blockers

- **Open visual decision** (tileset) blocks PR merge — purely cosmetic, no engine impact.
- `add_static_from_tilemap` emits one collider per solid tile (no row-merging); large maps may want merging later — deferred, noted in CHANGELOG/HANDOFF.
- Procedurally generated tileset is intentionally simple; may not match the polish of other examples' art.

## Open Questions

- Which tileset path does the user want (keep generated / revert render / external asset)? Functionality is identical across all three.

## Quick Start for Next Session

```bash
# Reference docs
#   docs/NEXT_WORK.md (rows A/B/E fixed), docs/VISION.md (dogfooding loop), CLAUDE.md

# Key files to read first
#   src/physics/world.rs        (add_static_from_tilemap, one-way predicate, TileCollider)
#   examples/games/platformer/platformer.rs (LEVEL, spawn_level, drop key)
#   examples/games/scene_flow/scene_flow.rs (register_persistent usage)
#   src/behavior.rs             (BlackboardValue::Path)

# Verify current state
cargo test --lib            # expect 253 passed
cargo clippy --all-targets -- -D warnings
cargo run --example platformer_game   # visual check

# Branch / PR
git switch feat/example-dogfood-gaps   # PR #3 vs main, up to 2b622f2

# Next action
# Resolve the platformer tileset choice the user raised (keep generated platform_tiles.png,
# revert render to stretched AtlasSprite while keeping the collider helper, or import an
# external seamless tileset), apply, re-verify, then push and merge PR #3.
```
