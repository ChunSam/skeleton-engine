# procgen↔FOV composition bridge + `roguelike` capstone shipped (v0.122.0)

**Date:** 2026-07-11
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `breadth-fov` seq `3` (the non-widget breadth pivot; **third** feature this chain)
**Parent:** `HANDOFF_breadth-fov_procgen_2026-07-10.md` (seq 2, the BSP dungeon feature)
**Prior chain:** `listbox-widget` (widget suite; CLOSED)

> Board (`../dungeon-merchant/docs/engine-wishlist.md`) was **empty** at session start (next free
> ID EW-007, unfiled). Per the onboarding rule I asked for direction; the user picked the
> **procgen↔FOV capstone** off the shelf the seq-2 handoff left. This is that third self-pick — and
> it closes the loop by composing this chain's two prior features (seq-1 `FovMap` + seq-2
> `generate_bsp_dungeon`) into one playable slice.

---

## Related Handoffs

- `HANDOFF_breadth-fov_field-of-view_2026-07-09.md` — seq 1, `FovMap` (the FOV half of the capstone).
- `HANDOFF_breadth-fov_procgen_2026-07-10.md` — seq 2, `generate_bsp_dungeon` (the procgen half).
  Read it for the shared onboarding/standing state; **this handoff is deliberately concise**.
- `PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the READY-but-unfiled
  EW-007 pre-design. Untouched. Reference only.

## The Goal

Continue the non-widget breadth pivot (`docs/VISION.md`: breadth-first, each feature proven by a
playable example). A procgen↔FOV capstone was one of the shelf candidates the seq-2 handoff put up:
compose the two features this chain shipped into a real roguelike slice (a seeded dungeon explored
under fog-of-war). It dogfoods both, is low-risk (mostly an example), and — per the VISION rule that
the example is the acceptance test — writing it surfaced an awkward composition seam worth fixing.

## Where We Are

- **Shipped and landing.** PR **#353** (`feat/roguelike-capstone`) with async auto-merge armed.
  Package **v0.122.0**, CLAUDE.md header **v1.6.215**. main tip → the squash of #353 on merge.
- **`src/mapgen.rs` — new public method `DungeonMap::to_path_grid(&self) -> PathGrid`.** Walkable
  cells = the dungeon's `Tile::Floor` cells, coords 1:1. It is the direct bridge to enemy
  pathfinding (`find_path(&map.to_path_grid(), …)`) **and**, via `FovMap::from_path_grid`, to
  field-of-view — one line each, same layout, so the walls that block movement block sight. Symmetric
  with the existing `to_tilemap_tiles`; **no new coupling** (`mapgen` already imported `pathfinding`,
  the import just widened `MAX_PATH_GRID_CELLS` → `{PathGrid, MAX_PATH_GRID_CELLS}`).
  - **2 new unit tests** (`to_path_grid_walkable_matches_floor`; `fov_from_dungeon_sees_the_spawn_room`
    = the capstone seam pinned in a test) + **1 doctest** → `mapgen` now 10 unit + 2 doctests.
- **`examples/roguelike.rs` (NEW) — the capstone.** A seeded `generate_bsp_dungeon` explored under
  `FovMap` fog-of-war: cells in sight render bright, explored-but-unseen dim, never-seen black; rooms
  tinted warmer than the corridors linking them; a gem hidden in every non-spawn room (`rooms[1..]`
  centers), revealed only when it falls in sight. WASD/arrows move, **+/-** torch radius, **R**
  descends to a fresh always-connected dungeon (new seed, blank fog). `HEADLESS_SHOT`.
  - The whole composition is one line: `FovMap::from_path_grid(&map.to_path_grid())`.
- `CLAUDE.md` — mapgen row (adds `to_path_grid` + names `roguelike`) + fov row (names the capstone) +
  header; `docs/CHANGELOG.md` 0.122.0; `Cargo.toml`/`Cargo.lock` bumped.
- **Full verify gate green** (all 7 checks; ran three times — see the doc-link trip below).
- **Additive** — one new method + one example, no existing API changed, no OS-gated/wasm-affecting code.

## What We Tried (Chronological)

1. **Onboarded** (git `62d4400`/#352 confirmed, `cargo test --lib mapgen`=8, `--lib fov`=9 green),
   read the board (empty), **asked direction** via AskUserQuestion → user picked the capstone.
2. **Read `PathGrid`'s constructor surface** to size the composition seam. Found `FovMap::from_path_grid`
   already exists but takes a `PathGrid`, while `mapgen` yields a `DungeonMap` — so composing meant
   either a manual double-loop or the heavyweight `to_tilemap_tiles → Tilemap → PathGrid::from_tilemap`
   3-hop. **Decided to add `DungeonMap::to_path_grid()`**: it's genuinely reusable (dungeon → A*
   navigation is a real roguelike need, not just FOV plumbing), makes FOV one line, and is symmetric
   with `to_tilemap_tiles`. This upgrades the capstone from "just an example" to "example + a small,
   well-justified composition API" (MINOR).
3. **Wrote the method + tests**, ran `cargo test --lib mapgen` → 10 green, doctests 2 green.
4. **Wrote `examples/roguelike.rs`** (merges the `fov` + `procgen_dungeon` render idioms: dark
   backdrop + z-layers, warm/cool room/corridor tint, bright/dim/black fog). Built + headless capture
   → lit warm spawn room + cool corridors around the yellow player, rest in black fog, HUD
   `gems 0/14 · explored 9%`. Reads clean.
5. **CLAUDE.md rows + `cargo fmt` (pre-empt the reflow trap) + verify.** **One gate trip:** the doc
   build (`RUSTDOCFLAGS=-D warnings`) flagged `redundant_explicit_links` — adding `PathGrid` to the
   `use` meant every `[`PathGrid`](crate::pathfinding::PathGrid)` in `mapgen.rs` (incl. a *pre-existing*
   one in `to_tilemap_tiles`' doc) became redundant. Fixed all three → bare `[`PathGrid`]` (left the
   `FovMap` links explicit — `FovMap` is *not* imported into `mapgen.rs`, referenced by full path).
   Re-ran doc → clean, then full gate → green.
6. **`/ship`** 0.121.0 → **0.122.0** (Cargo.toml + `cargo update -p skeleton-engine` + CHANGELOG +
   CLAUDE.md header v1.6.214 → v1.6.215). Re-ran the full gate (background) → green (exit 0).
7. **`/land-pr` Async:** branch `feat/roguelike-capstone` → commit `125287f` → push → PR **#353** →
   `gh pr merge 353 --auto --squash` (armed; `autoMerge.enabledAt` set, state BLOCKED = checks running).

## Key Decisions

- **Add `to_path_grid()` rather than keep the example self-contained.** The VISION loop says fix an
  awkward seam the example surfaces. The 3-hop `DungeonMap→Tilemap→PathGrid` (or a manual double-loop)
  was that awkwardness. `to_path_grid` earns its place on its own merit — a game wants a `PathGrid`
  from a generated dungeon for enemy A* regardless of FOV — and makes *both* pathfinding and FOV fall
  out in one line. Lowest-coupling place for it: a method on `DungeonMap` in `mapgen.rs` (already
  depends on `pathfinding`; the reverse would couple `pathfinding`/`fov` to `mapgen`).
- **The example DOES wrap itself in FOV (unlike `procgen_dungeon`).** Seq-2's demo deliberately
  showed the *full* dungeon (FOV would hide the very thing procgen produces). The capstone's job is
  the opposite — prove the *composition*, i.e. discovery under fog — so here the fog is the point.
  The two examples are complementary, not redundant.
- **Gem per non-spawn room, at `rooms[1..]` centers.** Room centers are guaranteed floor and (rooms
  being connected) reachable, so every gem is fair game — a clean "explore to discover" objective
  without extra placement logic. Spawn room (`rooms[0]`) is left empty (you start there).
- **Kept `FovMap` links in `mapgen.rs` docs explicit** (full path) since `FovMap` isn't imported;
  only the newly-redundant `PathGrid` links were made bare. Minimal, correct diff.

## Evidence & Data

| Item | Value |
|---|---|
| Package version | 0.121.0 → **0.122.0** (MINOR — new public API) |
| CLAUDE.md header | v1.6.214 → **v1.6.215** |
| PR | **#353**, async auto-merge armed |
| main tip | `62d4400` (#352) → squash of #353 on merge |
| Commit (pre-squash) | `125287f` on `feat/roguelike-capstone` |
| Memory global seq | code PR = seq **173**; this handoff PR = seq **174** |
| Async landing | 19th unattended auto-merge |

### Tests (10 unit + 2 doctests on `mapgen`, all green)

| File | New tests | Coverage |
|---|---|---|
| `src/mapgen.rs` | 2 unit + 1 doctest | walkability mirrors floor across the whole grid; a `FovMap::from_path_grid(&map.to_path_grid())` sees the spawn cell + every adjacent floor cell; doctest shows the two-line pathfinding+FOV bridge |

Full gate: `[verify] all checks passed ✓` (3×; the middle run caught the doc-link regression). CI: 5/5
required (armed auto-merge).

## Files Changed

### Source
- `src/mapgen.rs` (modified) — `to_path_grid()` + 2 unit tests + doctest; import widened to `{PathGrid,
  MAX_PATH_GRID_CELLS}`; module-doc bridge note; 3 redundant `PathGrid` doc-links made bare.
- `examples/roguelike.rs` (NEW) — the procgen↔FOV capstone (flat path → Cargo auto-discovers, no
  `Cargo.toml` change).

### Docs / release
- `CLAUDE.md` — mapgen + fov module-map rows + header v1.6.215 / v0.122.0.
- `docs/CHANGELOG.md` — 0.122.0 entry.
- `Cargo.toml`, `Cargo.lock` — 0.122.0.

### Memory (not in git)
- `engine-current-state.md` — seq-173 bump (code PR) + seq-174 bump (this handoff PR, on merge).

## User Feedback & Preferences

- The user opened with a thorough onboarding script + **"wait for my go-ahead before executing."** I
  narrated all 5 onboarding steps, then — board empty — used AskUserQuestion (recommendation first),
  and the answer *was* the go-ahead. Read: on an empty board the user wants a steer-with-recommendation,
  not silence and not a pre-implemented guess.
- Standing preferences (memory): user-facing reports **Korean**, agent-to-agent/code/docs English;
  **merge authority delegated** (squash on green CI, no re-confirm); **async auto-merge is default**;
  always pass explicit `model` to subagents.

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `breadth-fov` seq 3), async auto-merge.
   On merge, bump memory to **seq 174** pointing at the handoff merge hash.
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). If filed
   (EW-007+), serve priority-order (EW-007 FloatingText bold/rich has a READY pre-design note).
3. **If the board is still empty → ASK.** Widget queue stays exhausted. **Two non-widget shelf
   candidates remain** (both verified-absent): **seedable/deterministic RNG + `WeightedTable` loot**
   (small, foundational — a shared public RNG would let `mapgen` drop its private SplitMix64, and
   `to_path_grid` already gives the dungeon→nav bridge such loot tables would sit alongside) and
   **scene-transition effects** (extend the solid-colour `FadeTransition` to wipe/iris + auto
   scene-swap). The procgen↔FOV capstone is now DONE, so it leaves the shelf. Recommend one of the
   two remaining over another marginal widget.
4. **Hygiene:** the memory tip line is long (seq 153→174). A trim is **due now** (~seq 174–175):
   keep the current `breadth-fov` chain + one prior, push the tail into `[[engine-history-archive]]`
   via the proven Python surgical edit. Do this at the seq-174 bump.

## Risks & Blockers

- None for the shipped feature (additive, green, verified; CI's 5 checks fully cover it).
- Minor, unhit: 2 audio tests can fail locally on a no-audio-device box (passed here).

## Open Questions

- None blocking. `to_path_grid`'s coupling choice (method on `DungeonMap`, not a ctor on `FovMap`/
  `PathGrid`) is deliberate and documented.

## Quick Start for Next Session

```bash
# No beads — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 173/174), breadth-fov chain seq 3.

git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect the #353 squash or later
cargo test --lib mapgen                                       # 10 unit green (+ 2 doctests)

# See the capstone run
HEADLESS_SHOT=/tmp/roguelike.png cargo run --example roguelike # or drop HEADLESS_SHOT to play

# Key files
#   src/mapgen.rs                 — to_path_grid() (the composition bridge) + DungeonMap
#   src/fov.rs                    — FovMap::from_path_grid (the other half)
#   examples/roguelike.rs         — the capstone acceptance test
#   ../dungeon-merchant/docs/engine-wishlist.md — the board (read FIRST)

# Next action
#   Read the board. If filed, serve priority-order. If STILL empty, ASK — shelf now:
#   seedable-RNG+WeightedTable-loot / scene-transition wipe-iris. Also: trim the memory tip line.
```

## Session Closed

**Closed at:** 2026-07-11
**Session status:** Handed off — this handoff lands as its own `docs(handoff)` PR (chain `breadth-fov`
seq 3), async auto-merge. The memory **seq-174 bump** (updating `main @` to the handoff merge hash,
+ the due tip-line trim) is the wrap-up. Code state at close: `main @ 62d4400` + PR #353 armed,
v0.122.0, tree clean on the branch, all gates green. This chain has now shipped **three** non-widget
breadth features (FOV seq 1 + procgen seq 2 + the procgen↔FOV capstone seq 3).
