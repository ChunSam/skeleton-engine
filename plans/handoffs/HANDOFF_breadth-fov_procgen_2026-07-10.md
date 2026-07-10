# Procedural BSP dungeon generation (`mapgen`) shipped (v0.121.0)

**Date:** 2026-07-10
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `breadth-fov` seq `2` (the non-widget breadth pivot; **second** feature this session)
**Parent:** `HANDOFF_breadth-fov_field-of-view_2026-07-09.md` (seq 1, this chain — the FOV feature)
**Prior chain:** `listbox-widget` (widget suite; CLOSED)

> Same session as the FOV feature (seq 1). After FOV landed, the user said **"비위젯 후보 진행"**
> (proceed with a non-widget candidate) — so I picked the highest-value item off the shelf the FOV
> handoff left (procgen / seedable-RNG+loot / scene-transition-fx): **procedural dungeon generation**,
> which composes directly with the just-shipped `FovMap`. This is that second self-pick.

---

## Related Handoffs

- `HANDOFF_breadth-fov_field-of-view_2026-07-09.md` — seq 1, the FOV feature (`FovMap`). Shares this
  session's onboarding/standing state; read it for the full board/queue context. **This handoff is
  deliberately concise** — it does not repeat that shared context.
- `PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the READY-but-unfiled EW-007
  pre-design. Untouched. Reference only.

## The Goal

Continue the non-widget breadth pivot (`docs/VISION.md` breadth-first, each feature proven by a
playable example). Procedural map generation was one of three verified-absent candidates the seq-1
FOV handoff put on the shelf. It's a genre-defining roguelike capability, self-contained, and
**composes with the FOV shipped minutes earlier** (a seeded procedural dungeon you explore under
fog-of-war = a real roguelike slice), so it was the strongest next self-pick.

## Where We Are

- **Shipped and landing.** PR **#351** (`feat/mapgen`) with async auto-merge armed. Package
  **v0.121.0**, CLAUDE.md header **v1.6.214**. main tip → `30017b0` on merge.
- `src/mapgen.rs` (NEW, ~430 lines incl. tests) — BSP dungeon generator, a plain owned grid like
  `FovMap`/`PathGrid` (NOT an ECS type):
  - **`generate_bsp_dungeon(width, height, seed: u64, &DungeonParams) -> DungeonMap`** — recursively
    splits the map into partitions (`bsp_build`), carves a room per leaf (`carve_room`), and connects
    the two children of each split with an L-corridor (`carve_corridor`) while unwinding — each
    subtree returns a *representative room* for its parent to connect. **Guarantees connectivity**
    (the tree connects every sibling → every room reachable).
  - **Deterministic** — a private `Rng` (SplitMix64), seeded only by `seed`; never `rand`'s thread
    RNG. Same `(w, h, seed, params)` → byte-identical `DungeonMap` (test-pinned). A game stores just
    the seed to regenerate a level / reproduce a run.
  - `DungeonMap { width, height, rooms: Vec<Room> }` (+ private `tiles: Vec<Tile>`): `tile`/`is_floor`
    /`is_wall` (OOB reads `Wall`), `first_room_center`, `to_tilemap_tiles(floor_id, wall_id) ->
    Vec<Vec<u32>>` (build a `Tilemap`/`PathGrid`/`FovMap` from the same layout — the composition
    seam). `Tile` (Wall/Floor), `Room { x, y, w, h }` (+ `center`/`contains`), `DungeonParams`
    (min/max leaf, min_room, room_margin, max_depth; `Default` suits ~48×32). `new_walls` caps cells
    at `MAX_PATH_GRID_CELLS` (over-cap/overflow → empty map + `error!`, like `FovMap`/`PathGrid`).
  - **8 unit tests + 1 doctest**, incl. the key `all_rooms_are_connected` BFS flood fill.
- `src/lib.rs` — `pub mod mapgen;` + `pub use mapgen::{generate_bsp_dungeon, DungeonMap,
  DungeonParams, Room, Tile};` (alphabetical slot between `locale` and `material`).
- `examples/procgen_dungeon.rs` (NEW) — renders the full generated dungeon over a dark backdrop
  (walls = backdrop showing through; floor cells drawn, **rooms tinted warmer than the corridors**
  that link them, so the BSP structure is visible); WASD/arrows walk an explorer (walls block), **R**
  regenerates with the next seed (a fresh always-connected dungeon), HUD shows seed + room count.
  `HEADLESS_SHOT`. Headless capture eyeballed: 15 connected rooms + corridors, player at spawn, clean.
- `CLAUDE.md` module row + header; `docs/CHANGELOG.md` 0.121.0; `Cargo.toml`/`Cargo.lock` bumped.
- **Full verify gate green** (all 7 checks; ran twice — one clippy fix).
- **Additive** — new module + one re-export, no existing API changed, no OS-gated/wasm-affecting code.

## What We Tried (Chronological)

1. **User: "비위젯 후보 진행."** Picked procgen from the FOV handoff's shelf (over seedable-RNG+loot
   and scene-transitions) — highest value + composes with FOV. Stated the pick, did not re-ask.
2. **Designed against the example first** (skill step 1): the example renders the full dungeon (so
   you SEE the procgen structure — FOV would hide the very thing procgen produces, so NO fog here;
   FOV is the *composition* story, not the demo), player walks, R regenerates.
3. **Wrote `src/mapgen.rs`** — BSP with a self-contained SplitMix64 PRNG (decided NOT to extract a
   separate `engine::Rng` feature — that's a distinct shelf candidate; procgen's PRNG is an impl
   detail). Two compile errors caught by `cargo test --lib mapgen`: (a) **E0503** borrow-order —
   `map.width - 2` used inside the `bsp_build(&mut map, …)` arg list → hoisted to
   `let (iw, ih) = (map.width - 2, map.height - 2);` before the call; (b) **cast-then-`<` parse
   ambiguity** in `Rng::chance` — `(… as f32 / … as f32) < p` parsed `f32<p…>` as generics → bound
   the cast to a `let unit` first. Then 8/8 tests green incl. connectivity.
4. **Wrote `examples/procgen_dungeon.rs`** — first render drew wall cells at `WALL=0.09` linear,
   which the sRGB surface shows as ~0.34 (mid-gray) — same headless-clear-is-lighter effect as FOV.
   Switched to the FOV example's proven approach: a **dark backdrop rect** + draw only floor cells
   (walls = backdrop). Re-shot → clean dark dungeon, rooms/corridors pop.
5. **CLAUDE.md row** (next to the FOV row) + `cargo fmt` (pre-empt the reflow trap) + verify. **One
   gate trip**: clippy `int_plus_one` on a test assertion `r.x + r.w <= m.width - 1` → `< m.width`.
   Full gate green after.
6. **`/ship`** 0.120.0 → **0.121.0** (Cargo.toml + `cargo update -p skeleton-engine` + CHANGELOG +
   CLAUDE.md header v1.6.213 → v1.6.214). Re-ran the full gate → green.
7. **`/land-pr` Async:** branch `feat/mapgen` → commit → push → PR **#351** →
   `gh pr merge 351 --auto --squash` (armed; BLOCKED = checks running).

## Key Decisions

- **BSP (binary space partition), not cellular-automata caves.** BSP produces recognizable *rooms +
  corridors* with clear spawn points and a clean tree-based connectivity guarantee — the quintessential
  "dungeon." (Cave-style CA remains a possible future param/mode.)
- **Connectivity is structural, not post-hoc.** Corridors are carved on the recursion *unwind*: each
  internal node joins its two children's representative rooms, and a subtree with no room returns
  `None` (the parent just forwards the other child). So all *actual* rooms stay in one connected
  component — proven by the `all_rooms_are_connected` flood-fill test, not assumed.
- **Self-contained SplitMix64 PRNG, NOT a separate `engine::Rng` feature.** Determinism (seed →
  identical dungeon) is the headline property, but the PRNG is a private impl detail here. A public
  seedable-RNG + `WeightedTable` resource is a *distinct* shelf candidate (see Where We're Going) —
  extracting it now would blur two features. When it ships, `mapgen` can adopt it (non-breaking).
- **A plain owned grid, not an ECS type** — mirrors `FovMap`/`PathGrid`. No editor/reflect/serde
  wiring. A game keeps the `DungeonMap`, reads `is_floor`/`rooms`, and/or calls `to_tilemap_tiles` to
  spin up a `Tilemap`/`PathGrid`/`FovMap` — the composition seam (esp. with the seq-1 FOV).
- **The example shows the FULL dungeon (no fog).** A procgen demo should reveal the generated
  structure, so it does NOT wrap itself in FOV — rooms are tinted warmer than corridors to make the
  BSP layout legible. FOV composition is documented, not demoed (the demo's job is to prove *procgen*).
- **`MAX_PATH_GRID_CELLS` cap reused** (third grid type to do so, after PathGrid/FovMap) — consistent
  degenerate/overflow handling, no new constant.
- **Rejected for now:** cave/CA generation mode, weighted room contents, multi-level/stairs — all
  clean future extensions; kept the first cut to one well-tested algorithm + one example.

## Evidence & Data

| Item | Value |
|---|---|
| Package version | 0.120.0 → **0.121.0** (MINOR — new public API) |
| CLAUDE.md header | v1.6.213 → **v1.6.214** |
| PR | **#351**, async auto-merge armed |
| main tip | `30017b0` (was `17a3fad` #350) |
| Commit (pre-squash) | on `feat/mapgen` |
| Memory global seq | **171** (code PR); this handoff PR will be seq **172** |
| Async landing | 18th unattended auto-merge |

### Tests (8 unit + 1 doctest, all green)

| File | Count | Coverage |
|---|---|---|
| `src/mapgen.rs` | 8 | same-seed-identical, different-seeds-differ, border-all-wall, rooms floor+in-bounds+min-sized, **all-rooms-connected (BFS flood fill)**, to_tilemap_tiles-matches-grid, degenerate/overflow-safe, PRNG deterministic+range-bounded |
| doctest | 1 | module doc (same seed → identical; border is wall) |

Full gate: `[verify] all checks passed ✓` (twice). CI: 5/5 required (armed auto-merge).

## Files Changed

### Source (new)
- `src/mapgen.rs` — BSP generator + `DungeonMap`/`Room`/`DungeonParams`/`Tile` + private `Rng` + 8 tests.
- `examples/procgen_dungeon.rs` — playable full-dungeon demo (walk + regenerate) + headless self-check.

### Source (modified)
- `src/lib.rs` — `pub mod mapgen;` + the 5-type re-export.

### Docs / release
- `CLAUDE.md` — module-map row + header v1.6.214 / v0.121.0.
- `docs/CHANGELOG.md` — 0.121.0 entry.
- `Cargo.toml`, `Cargo.lock` — 0.121.0.

### Memory (not in git)
- `engine-current-state.md` — seq-171 bump (code PR) + seq-172 bump (this handoff PR, on merge).

## User Feedback & Preferences

- **"비위젯 후보 진행"** — a terse "proceed with a non-widget candidate," delegating the pick. I chose
  procgen decisively (stated the pick + reasoning, did NOT re-ask via AskUserQuestion) — the read is
  the user wants momentum, not another question round, once they've set the lane. Consistent with the
  seq-1 pattern (they value visible breadth progress on clean, verified-absent, low-risk picks).
- Standing preferences (from memory): user-facing reports in **Korean**, agent-to-agent/code/docs in
  English; **merge authority delegated** (squash on green CI); **async auto-merge is the default**;
  always pass an explicit `model` to subagents.

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `breadth-fov` seq 2), async auto-merge.
   On merge, bump memory to **seq 172** pointing at the handoff merge hash (next session's opening
   wrap-up, per the recorded cadence — this session did seq 170's wrap-up at its own start).
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). If a
   request is filed (EW-007+), serve it priority-order (EW-007 FloatingText bold/rich has a READY
   pre-design note).
3. **If the board is still empty → ASK for direction.** The widget queue stays exhausted. **Two
   non-widget candidates remain on the shelf** (both verified-absent, offered but not yet picked):
   **seedable/deterministic RNG + `WeightedTable` loot** (small, foundational — makes procgen/FOV
   runs reproducible with a *shared public* RNG; `mapgen` could then drop its private SplitMix64) and
   **scene-transition effects** (extend the solid-colour `FadeTransition` to wipe/iris + auto
   scene-swap orchestration). A **procgen ↔ FOV capstone example** (a seeded dungeon explored under
   fog-of-war, i.e. compose seq-1 + seq-2 into one playable roguelike slice) is also a strong,
   low-risk pick that dogfoods both. Recommend one of these over another marginal widget.
4. **Hygiene:** tip line growing (seq 153–172). A trim is due ~**seq 174–175** (keep the current
   `breadth-fov` chain + one prior). Use the proven Python surgical edit into `[[engine-history-archive]]`.

## Risks & Blockers

- None for the shipped feature (additive, green, verified; CI's 5 checks fully cover it).
- Minor, unhit: 2 audio tests can fail locally on a no-audio-device box (passed here).

## Open Questions

- None blocking. The one lean choice (private PRNG rather than a shared `engine::Rng`) is deliberate
  and documented; revisit if/when the RNG feature ships (then `mapgen` adopts it non-breakingly).

## Quick Start for Next Session

```bash
# No beads — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 171/172), breadth-fov chain seq 2.

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 30017b0 (#351) or later
cargo test --lib mapgen                                       # 8 tests green (+ 1 doctest)
cargo test --lib fov                                          # seq-1 FOV, 9 tests green

# See it run
HEADLESS_SHOT=/tmp/procgen.png cargo run --example procgen_dungeon   # or drop HEADLESS_SHOT to play
HEADLESS_SHOT=/tmp/fov.png cargo run --example fov

# Key files
#   src/mapgen.rs                 — BSP generator + DungeonMap/Room/DungeonParams/Tile
#   src/fov.rs                    — the seq-1 FOV it composes with (from_path_grid / compute)
#   examples/procgen_dungeon.rs   — the playable acceptance test
#   .claude/skills/add-feature-example/SKILL.md — the feature+example pass

# Next action
#   Read ../dungeon-merchant/docs/engine-wishlist.md. If filed, serve priority-order. If STILL empty,
#   ASK — shelf: seedable-RNG+WeightedTable-loot / scene-transition wipe-iris / a procgen↔FOV capstone.
```

## Session Closed

**Closed at:** 2026-07-10
**Session status:** Handed off — this handoff lands as its own `docs(handoff)` PR (chain `breadth-fov`
seq 2), async auto-merge. The memory **seq-172 bump** (updating `main @` to the handoff merge hash) is
the next session's opening wrap-up. Code state at close: `main @ 30017b0`, v0.121.0, tree clean,
all gates green. This session shipped **two** non-widget breadth features (FOV seq 1 + procgen seq 2).
