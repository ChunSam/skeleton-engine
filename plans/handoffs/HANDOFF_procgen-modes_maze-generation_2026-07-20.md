# Shipped: perfect-maze generator — a third procgen mode over `DungeonMap` (v0.131.0)

**Date:** 2026-07-20
**Status:** COMPLETED (PR #370 merged, `main @ 38d827a`, v0.131.0, clean + green)
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `procgen-modes` seq `2`
**Parent:** `HANDOFF_procgen-modes_cellular-caves_2026-07-20.md` (seq 1)
**Prior chain:** `HANDOFF_procgen-modes_cellular-caves_2026-07-20.md` (seq 1, cellular caves / v0.130.0 / #368) > this (seq 2, mazes / v0.131.0 / #370)

---

## Since Last Handoff

Seq 1 shipped the cellular-cave generator (2nd procgen mode) and closed with a `## Where We're Going` that named the obvious next-mode menu: **maze generation** (recursive-backtracker / growing-tree), drunkard's-walk caves, room-accretion / BSP+cave hybrid, Voronoi, or a fresh non-procgen area. This session is the direct follow-on: the board was empty again, the user was asked, and picked **maze generation**. So seq 1's plan played out exactly as written.

- **Seq 1's "board empty → ASK, don't self-pick" rule held again.** Both channels were still empty (no new EW filed). Asked; user chose the maze mode from a 4-option menu.
- **Seq 1's open question "should `procgen-modes` grow a shared trait/enum once there are 3+ modes?" is now live** — with mazes we are AT three free functions (`generate_bsp_dungeon` / `generate_cellular_cave` / `generate_maze`). Still shipped as a third free function (see Key Decisions); the trait abstraction remains deferred, not yet warranted.
- **Seq 1's `update-branch` risk materialized again** — `origin/main` moved mid-session (the seq-1 handoff docs PR #369 landed remotely while this session's branch was open), so PR #370 went `BEHIND` and needed `gh pr update-branch`. Exactly the strict-checks recurrence seq 1 flagged.
- **Seq 1's ~5h CI runner-backlog did NOT recur** — this session's CI ran promptly (6/6 green, merged ~6 min after `update-branch`). The backlog was a transient GitHub-side condition, as diagnosed.
- **Seq 1's `cargo fmt` reflow trap recurred** — same lesson, same fix (run `cargo fmt` before `verify.sh`; the reflow reds the first gate).
- **The opening-prompt file-existence quirk from seq 1 repeated in mirror image:** the paste prompt told me to read `PLAN_/HANDOFF_procgen-modes_cellular-caves_2026-07-20.md` (seq 1), but those files did NOT exist in the local tree — they were on `origin/main` via #369, which had not been pulled locally (local tip was `f8354fb`/#368). The paste even anticipated "atop the #369 handoff-docs merge", but #369 hadn't reached local `main`. I did not chase the missing files; I executed the meaningful parts of the prompt (board gate, verify, re-prove seq 1, ask for direction) directly.

---

## Reference Documents

- `plans/handoffs/PLAN_procgen-modes_cellular-caves_2026-07-20.md` — seq-1 plan; its Phase 1/2 structure (board gate → if empty, ASK with a concrete procgen-mode menu) is what this session executed.
- `CLAUDE.md` — project conventions + the module map (the mapgen row now describes all three generators).
- `docs/CHANGELOG.md` — 0.131.0 entry is the migration note for this change.

---

## The Goal

The `procgen-modes` chain is a breadth arc: give the engine a family of deterministic, guaranteed-connected map generators that all return the shared `DungeonMap`, so each new mode inherits the pathfinding (`to_path_grid`), FOV (`FovMap::from_path_grid`), and tilemap (`to_tilemap_tiles`) bridges for free. Seq 1 added organic **caves** to the existing **BSP rooms**. This session adds the third and most structurally different mode — a **perfect maze** — completing the classic trio (rooms / caves / mazes). The concrete deliverable: `generate_maze` ships alongside the other two, is deterministic + guaranteed-connected (this time *by construction*, via a spanning tree, not a keep-largest cleanup), and a playable example (`maze_generation`) is the acceptance test. Shipped as v0.131.0 / PR #370.

---

## Where We Are

- **`main @ 38d827a`, package v0.131.0, CLAUDE.md header v1.6.225, clean tree, all gates green.** Local `main` == `origin/main` (verified `git rev-parse`).
- **Engine PR #370 — MERGED** `38d827a` (2026-07-20T12:52:14Z, async auto-merge, CI **6/6**), squash of branch `feat/mapgen-maze-generator` (branch deleted locally + remotely).
- **New public API:** `generate_maze(width: i32, height: i32, seed: u64, &MazeParams) -> DungeonMap` + `MazeParams { braid_chance: f32 }` (default `0.0`). In `src/mapgen.rs`, re-exported from `src/lib.rs` (the `pub use mapgen::{…}` line now includes `generate_maze, MazeParams`).
- **Reuses the existing `DungeonMap`** — no new grid type, mirroring the cave decision. So `to_path_grid()` / `to_tilemap_tiles()` / `FovMap::from_path_grid()` all compose with a maze. `first_room_center()` works because a maze records a **single 1×1 `Room`** at the start junction `(1, 1)`.
- **Guaranteed connected BY CONSTRUCTION** — a recursive-backtracker (DFS) walk over the junction cells builds a spanning tree over the junction graph, so every junction is reached and there is no disconnected pocket to clean up. This is stronger/cheaper than the cave's post-hoc `keep_largest_cavern`.
- **Perfect-maze property:** for `braid_chance == 0.0` there is exactly one path between any two cells, and the floor-cell count is exactly `2·jw·jh − 1` (J junction cells + J−1 carved passage cells, where `jw = (width−1)/2`, `jh = (height−1)/2`). A unit test asserts this exact count — it proves connected AND acyclic in one number.
- **Optional braiding:** `MazeParams { braid_chance }` (0.0..=1.0, clamped) reopens a fraction of dead-end walls into loops after carving. Braiding only ever *removes* walls, so connectivity is preserved at any value; a braided maze has strictly more floor than the perfect one (a unit test asserts both).
- **Deterministic:** same `(w, h, seed, params)` → identical `DungeonMap`. `Rng` (SplitMix64) drives the neighbor-shuffle at each junction and the braid decisions; iteration order is fixed, so the stream is reproducible. A module doctest pins `assert_eq!(maze, generate_maze(…))`.
- **Example `examples/maze_generation.rs`** (≈290 lines, flat/auto-discovered) — renders a perfect maze (dead-end *tips* tinted warm to show the tree's leaves), WASD/arrows walk (walls block), R regenerates from the next seed, **B toggles braiding** (perfect ↔ braided, same seed). Under `HEADLESS_SHOT` it self-checks single-region connectivity (flood-fill from spawn reaches every floor cell) and exits non-zero on failure, then captures a screenshot.
- **9 new unit tests** in `src/mapgen.rs` + a maze clause in the module doctest. Lib test total **1258 → 1267** (mapgen module **17 → 26**: 9 BSP + 8 cave + 9 maze).
- **Both downstream channels still empty** — no new EW filed; the shelf remains exhausted. The next session must ASK again. **The procgen family is now COMPLETE (rooms/caves/mazes)** — the next mode is genuinely optional breadth, not a gap.
- Memory `engine-current-state` bumped to **seq 189** (`main @ 38d827a`, v0.131.0, header v1.6.225); MEMORY.md index hook (which was stale at v0.124.0/seq 177) refreshed to match.

---

## What We Tried / Did (Chronological)

### Chunk 1 — Onboarding, board gate, state verify, re-prove seq 1

1. **Read the referenced seq-1 plan/handoff files → did not exist locally.** `plans/handoffs/PLAN_/HANDOFF_procgen-modes_cellular-caves_2026-07-20.md` were absent from the working tree (local `main` was at `f8354fb`/#368; they had landed on `origin/main` via #369 but weren't pulled). Recognized this as the same "docs landed by a separate flow after this session's snapshot" pattern seq 1 itself hit; did not chase the files.
2. **Kicked off `./scripts/verify.sh` in the background** and ran the board gate + git-state checks in parallel while it compiled.
3. **Board-check gate (both channels):** `../dungeon-merchant/docs/engine-wishlist.md` — EW-007 and EW-008 both `[x]` Verified/Closed 2026-07-16, no `Proposed`/`Acknowledged`/`In-progress`, next free **EW-009**. `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — Open Requests `_None._`. **Both empty.**
4. **Verify green:** `verify.sh` → exit 0 (7 gates; read the real code, not a trailing echo). Re-proved seq 1: `cargo test --lib mapgen` → **17 passed**; `HEADLESS_SHOT=/tmp/cave.png cargo run --example cave_generation` → exit 0, "one connected cavern — 1301 floor cells, spawn at (34, 23)".
5. **Direction ask (AskUserQuestion, Korean):** four options — (1) 미로 생성기 (procgen 3rd, spanning-tree) / (2) 드렁커즈워크 / 방-증식 하이브리드 / (3) 오디오 반응 훅 / (4) 두 번째 캡스톤 게임. Recommended #1 (pattern-faithful, low-risk, completes the trio). **User chose #1 — 미로 생성기.**

### Chunk 2 — Design + implement the maze generator

6. **Read `src/mapgen.rs` in full** to fit the pattern: `DungeonMap { width, height, rooms, tiles(private) }`; helpers `new_walls` (caps at `MAX_PATH_GRID_CELLS`, degenerate/overflow → empty 0×0), `set_floor`, `index`; the cave's `generate_cellular_cave`/`cave_smooth`/`keep_largest_cavern` and its test-suite shape (which I mirrored). Confirmed the tests reuse a module-level `flood_from_spawn` helper I could reuse for the maze connectivity test.
7. **Checked the wiring conventions** in parallel: `Rng` public methods (`shuffle`, `chance`, `range`, `bool`, …); the lib.rs re-export line; and that flat `examples/*.rs` are **auto-discovered** (no `[[example]]` entry — confirmed `cave_generation` isn't in `Cargo.toml`; the 29 `[[example]]` blocks are only for nested/subdir examples). So `maze_generation.rs` needs no `Cargo.toml` edit.
8. **Design locked before coding:** junction cells on odd coordinates (junction `(jx,jy)` at grid cell `(1+2·jx, 1+2·jy)`); `jw=(w−1)/2`, `jh=(h−1)/2` junctions; recursive backtracker with an **EXPLICIT stack** (a maze can nest as deep as every cell → real recursion risks stack overflow, unlike BSP's shallow `max_depth=5`); carve the wall-between + the neighbor junction on each step; spawn = a synthetic 1×1 `Room` at start junction `(1,1)`; `braid_chance` post-pass.
9. **Wrote the feature** (`src/mapgen.rs`, +285 lines): module doc rewritten to "three deterministic generators" + a maze paragraph; `MazeParams` + `Default { braid_chance: 0.0 }`; `generate_maze` (guard `<3×3` → empty; `visited` over junctions; `rng.shuffle(&mut dirs)` then step to first unvisited neighbor; pop on backtrack; optional `braid_dead_ends`; push `Room{1,1,1,1}`); helper `braid_dead_ends` (row-major, dead-end = floor junction with exactly 1 open passage, `rng.chance(chance)` short-circuited so it's only drawn at a real dead end, then `rng.shuffle` + reopen the first still-closed in-bounds wall); +9 unit tests; a maze clause in the module doctest.
10. **Fast targeted check:** `cargo test --lib mapgen` → **26 passed** (all 9 new maze tests green on the first compile). The exact-floor-count test `2·jw·jh−1` passed, confirming the spanning-tree property.

### Chunk 3 — Example + visual confirmation

11. **Wrote `examples/maze_generation.rs`** modeled on `cave_generation` (COLS 45, ROWS 29 [both odd → clean border, no wall margin], CELL 15, `BRAID_CHANCE 0.55`): render walls-as-backdrop + floor cells (dead-end tips warm via `is_dead_end` = floor with exactly 1 orthogonal floor neighbor), WASD/arrows move gated on `is_floor`, R regen (seed+1), **B toggle braid** (rebuild same seed with/without braiding), HUD showing seed/mode/floor/dead-end counts, and a headless `connectivity()` self-check + screenshot.
12. **Ran headless:** `HEADLESS_SHOT=/tmp/maze.png cargo run --example maze_generation` → exit 0, "one connected region — **615 floor cells**, spawn at (1, 1)". 615 == `2·22·14 − 1` exactly (jw=22, jh=14) — the spanning-tree count, live-confirmed.
13. **Eyeballed the screenshot** (`/tmp/maze.png`): a correct 1-wide-corridor perfect maze — dark walls, light passages, 35 warm dead-end tips, yellow player at (1,1), clear HUD ("perfect · 615 floor · 35 dead ends"). No feel-iteration needed (unlike the cave, which took a rule change) — the recursive backtracker looks right out of the box.

### Chunk 4 — Wire, verify, ship

14. **`src/lib.rs` re-export** extended to `generate_maze, MazeParams`.
15. **CLAUDE.md module-map row** updated: intro "two" → "three deterministic generators … BSP + caves + **perfect mazes**"; added the maze generator description (junction spanning tree, guaranteed-connected-by-construction, `2·jw·jh−1`, `braid_chance`) + the `maze_generation` example.
16. **First `verify.sh` HALTED at `cargo fmt --check`** — the reflow trap: 3 diffs (`examples/maze_generation.rs:19` import wrap, `src/mapgen.rs:650` + `:1032` — a hand-wrapped `assert_eq!` fmt wanted multi-line). The background task's "exit 0" was the trailing `echo`, NOT verify's code; read the log to see the real halt.
17. **`cargo fmt`** applied (2 files reformatted) → `cargo fmt --check` clean → re-ran full `verify.sh`, capturing verify's own exit to `/tmp/verify3.exit` → **exit 0** (7 gates). Re-confirmed headless example post-fmt → exit 0.
18. **`/ship` → v0.131.0** (MINOR, new public API): `Cargo.toml` 0.130.0 → 0.131.0; `cargo update -p skeleton-engine` (Locking 0 packages, lock → 0.131.0); CHANGELOG 0.131.0 entry (mirrors the 0.130.0 cave entry's shape); CLAUDE.md header v1.6.224 → v1.6.225. **Re-ran verify after the bump → exit 0** (`/tmp/verify4.exit`).

### Chunk 5 — Land + the BEHIND / update-branch recurrence

19. **`/land-pr` async mode:** branch `feat/mapgen-maze-generator` was created up front (Chunk 2); committed `feat(mapgen): perfect-maze generator over DungeonMap (v0.131.0)` (`3a08de4`, Co-Authored-By + Claude-Session trailers), pushed, opened **PR #370**, armed `gh pr merge 370 --auto --squash`.
20. **`mergeStateStatus: BEHIND`** right after arming — `origin/main` had moved `f8354fb → 9c5b60b` (**#369**, the procgen-modes seq-1 handoff docs, landed via its own flow mid-session). Disjoint files (docs only) → `gh pr update-branch 370` merged main in cleanly, auto-merge stayed armed, CI re-triggered → `BLOCKED` (checks running).
21. **CI ran promptly — NO runner backlog this time.** 5 of 6 checks green within ~2 min (Windows DX12 1m57s, Render lavapipe 1m12s, WASM 38s, Package dry-run 1m11s, Rustdoc 32s); only Test (native) trailed. Polled in the background (30s interval, fail-fast, ~12-min cap).
22. **6/6 green; auto-merged `38d827a` at 12:52:14Z** (~360s after `update-branch`). Synced `main` (`git checkout main && git pull --ff-only` → `38d827a`), deleted `feat/mapgen-maze-generator`, bumped memory to **seq 189**, and refreshed the stale MEMORY.md index hook (was v0.124.0/seq 177) to current.

---

## Key Decisions

- **Reuse `DungeonMap`, do not invent a `MazeMap`.** Same rationale as the cave (seq 1): the type is the bridge to `PathGrid` / `FovMap` / `Tilemap`. A maze returning `DungeonMap` inherits all three for free. This is now a settled chain convention — every procgen mode returns `DungeonMap`.
- **Third free function, NOT a `MapGenerator` trait/enum (yet).** Seq 1's open question was whether 3+ modes warrant an abstraction. At three, the free functions still read cleanly and share no runtime dispatch need (nothing swaps generators at runtime yet). A trait would be premature — deferred until an example/game actually wants to pick a generator dynamically. Kept the surface flat.
- **Connectivity BY CONSTRUCTION (spanning tree), not keep-largest.** A recursive backtracker visits every junction exactly once, carving as it goes — the result is inherently one connected tree, so no post-hoc region cleanup is needed (unlike the cave). This is both simpler and gives the stronger *perfect-maze* guarantee (unique paths). The exact `2·jw·jh−1` floor count is the test that proves it.
- **Explicit stack, not recursion.** BSP recurses safely because `max_depth=5` bounds it. A maze DFS can nest as deep as the junction count (hundreds), so real recursion risks a stack overflow on a large map. An explicit `Vec` stack removes that ceiling. (Documented in-code.)
- **`MazeParams { braid_chance }` — one tunable, default 0.0.** A pure recursive backtracker has no real knobs, so a bare `MazeParams` would be an empty struct. `braid_chance` is a genuinely useful, well-known post-process (dead-end-heavy perfect maze ↔ flowing braided maze) that also demonstrates a range in the example (the B toggle). Default 0.0 = the clean perfect maze. Braiding only removes walls, so it can't break the connectivity guarantee at any value — safe to expose.
- **Braid determinism via short-circuit + fixed traversal.** `if open != 1 || !rng.chance(chance)` draws `rng.chance` only at an actual dead end (row-major order). The number of draws depends on maze structure, but the structure is itself deterministic from the seed, so the whole stream is reproducible. The progressive wall-removal (an earlier braid can de-dead-end a later junction) is intentional and deterministic.
- **Spawn as a synthetic 1×1 `Room` at the start junction `(1,1)`.** Keeps `first_room_center()` uniform across all three generators. `(1,1)` is always floor (the DFS start) and always in-bounds for a ≥3×3 map. Simpler than the cave's centroid computation because the start is fixed.
- **Odd example dimensions (45×29).** For a grid maze, odd width/height means the last junction sits at `width−2` with no leftover wall margin — a symmetric maze bordered on all sides. Even dims work too (border test still passes) but leave a 1-cell wall margin on the right/bottom. Chose odd for a clean look.
- **Dead-end *tip* tinting via a general detector** (`is_dead_end` = any floor cell with exactly 1 orthogonal floor neighbor), not junction-specific. It highlights the tree's leaves, which braiding visibly thins — so the B toggle reads at a glance (35 dead ends perfect → fewer braided).
- **Async auto-merge (no watch-and-confirm gate).** Pure Rust logic fully covered by unit tests that run on CI's native job; the render half was verified locally (headless screenshot eyeballed). No OS-gated/audio/hot-reload code. Matches the standing merge delegation.

---

## Evidence & Data

### The spanning-tree floor count (the defining maze invariant, live-confirmed)

| Map (perfect, braid 0.0) | jw=(w−1)/2 | jh=(h−1)/2 | J=jw·jh | Expected floor `2J−1` | Observed |
|---|---|---|---|---|---|
| Example 45×29 | 22 | 14 | 308 | **615** | **615** (headless output) |
| Test default 41×27 | 20 | 13 | 260 | 519 | asserted `== 2·jw·jh−1` |

The test computes `jw`/`jh` from the map dims and asserts the exact count — a single number that proves connected (all J junctions visited) AND acyclic (exactly J−1 passages). A braided maze breaks the equality upward (more floor); a separate test asserts `braided > perfect` AND still fully connected.

### Test counts

| Point | Lib tests | mapgen module |
|---|---|---|
| Session start (post cave, seq 1) | 1258 | 17 (9 BSP + 8 cave) |
| **Maze shipped (v0.131.0)** | **1267** (+9) | **26** (9 BSP + 8 cave + 9 maze) |

New mapgen tests (all in `src/mapgen.rs::tests`): `maze_same_seed_is_identical`, `maze_different_seeds_differ`, `maze_border_is_all_wall`, `maze_spawn_is_the_start_junction`, `maze_is_a_single_connected_region` (flood-fill across seeds 1/7/42/99/2026), `maze_perfect_floor_count_is_a_spanning_tree` (the exact `2·jw·jh−1`), `maze_braided_is_connected_and_has_more_floor` (braid_chance 1.0), `maze_composes_with_path_grid_and_fov`, `maze_degenerate_sizes_are_safe`. They reuse the module-level `flood_from_spawn` helper and a new local `floor_count` helper.

### Gate history

| Run | Result | Notes |
|---|---|---|
| targeted: `cargo test --lib mapgen` (post-impl) | 0 | 26 passed |
| targeted: headless `maze_generation` from repo | 0 | OK: one connected region, 615 floor cells, spawn (1,1) |
| full verify (first attempt) | **HALTED at `cargo fmt --check`** | 3 reflow diffs (import wrap + 2 multi-line `assert_eq!`) — the trailing-echo masked it; read the log |
| `cargo fmt` + re-verify (`/tmp/verify3.exit`) | 0 | 7 gates, fmt now clean |
| full verify (post-`/ship` bump, `/tmp/verify4.exit`) | 0 | lock + doc re-checked |
| CI #370 | 6/6 | after `update-branch` re-trigger |

### CI #370 — the 6 checks (ran promptly, no backlog)

| Check | Result |
|---|---|
| Test (native) | ✅ (trailed the others by ~1 min) |
| Build (Windows / DX12) | ✅ 1m57s |
| Build (WASM) | ✅ 38s |
| Rustdoc | ✅ 32s |
| Package dry-run | ✅ 1m11s |
| Render tests (lavapipe) | ✅ 1m12s |

Merged `38d827a` at 12:52:14Z, ~360s after `gh pr update-branch`. Contrast seq 1's ~5h runner-backlog — that was transient and did not recur.

### Merge log

| Repo | PR | Commit | What |
|---|---|---|---|
| skeleton-engine | **#370** | `38d827a` | v0.131.0 — `generate_maze` + `MazeParams` + example `maze_generation` |

Intervening remote merge this session: **#369** `9c5b60b` (procgen-modes seq-1 handoff docs, landed by a separate flow → caused #370 to go `BEHIND`).

### Branch / commit lifecycle (PR #370)

| Step | State |
|---|---|
| Branch created (Chunk 2, before impl) | `feat/mapgen-maze-generator` off `main @ f8354fb` |
| Commit | `3a08de4` `feat(mapgen): perfect-maze generator over DungeonMap (v0.131.0)` (+ Co-Authored-By + Claude-Session trailers) |
| Push + PR | `git push -u origin …` → PR #370 (base `main`) |
| Auto-merge armed | `gh pr merge 370 --auto --squash` → immediately `BEHIND` (origin/main moved to `9c5b60b`) |
| Catch-up | `gh pr update-branch 370` → merge-commit on branch, CI re-triggered, `BLOCKED` (checks running) |
| Merged | squash → `38d827a` on `main` at 12:52:14Z; branch auto-deleted remote + `git branch -D` local |

### The direction ask (full menu — the user chose option 1)

- **1 — 미로 생성기 (procgen 3rd)** *(chosen, recommended)*: a third `DungeonMap` generator — spanning-tree (recursive backtracker / Wilson) perfect maze. Guaranteed-connected by construction, Rng-deterministic, composes with `to_path_grid`/`FovMap`/`to_tilemap_tiles`. Completes the procgen trio (rooms/caves/mazes). Pattern-faithful, low-risk.
- **2 — 드렁커즈워크 / 방-증식 하이브리드**: another organic generator (drunkard's walk or room accretion). Also returns `DungeonMap`, deterministic, keep-largest-connected. Procgen-family extension but a less crisp result than a maze.
- **3 — 오디오 반응 훅 (new area)**: beat/amplitude-driven gameplay callbacks over the Audio facade. New subsystem surface, off the procgen path.
- **4 — 두 번째 캡스톤 게임 (new area)**: a second small playable game composing existing subsystems (roguelike is the first). Weighted toward integration/API-wear discovery over new API.

### The three-generator family (how they differ)

| Aspect | `generate_bsp_dungeon` | `generate_cellular_cave` | `generate_maze` |
|---|---|---|---|
| Structure | rectangular rooms + L-corridors | organic cavern | 1-wide perfect maze |
| Connectivity | carve corridors between BSP siblings | keep only the largest flood region | spanning tree over junctions (**by construction**) |
| Unique paths? | no (loops via corridors) | no | **yes** (braid_chance 0) |
| `rooms` field | one per carved leaf (`rooms[0]`=spawn) | one synthetic 1×1 at centroid | one synthetic 1×1 at start junction (1,1) |
| Params | `DungeonParams{min_leaf,max_leaf,min_room,room_margin,max_depth}` | `CaveParams{initial_wall_prob,steps,wall_threshold}` | `MazeParams{braid_chance}` |
| `Rng` usage | throughout recursion | initial random fill only | per-junction dir shuffle + braid decisions |
| Shared | all return `DungeonMap`; border always wall; cap `MAX_PATH_GRID_CELLS`; feed `to_path_grid`/`to_tilemap_tiles`/`FovMap::from_path_grid`; `first_room_center` spawn | | |

### `MazeParams` tuning guide

| Field | Default | Effect |
|---|---|---|
| `braid_chance` | 0.0 | fraction of dead-end junctions (0.0..=1.0, clamped) whose closing wall is reopened into a loop; 0.0 = pure perfect maze (max dead ends, unique paths); higher = fewer dead ends + flowing loops; connectivity holds at any value |

### The maze algorithm (primary artifact)

`generate_maze` (`src/mapgen.rs`): `new_walls` → return if `<3×3` → `jw=(w−1)/2`, `jh=(h−1)/2` → `Rng::new(seed)` → explicit-stack DFS: `visited[jidx(0,0)]=true`, `set_floor(1,1)`, `stack=[(0,0)]`; while `stack.last()`: `rng.shuffle(&mut dirs[4])`, step to first unvisited in-bounds neighbor (carve `set_floor(cx+dx, cy+dy)` [wall-between] + `set_floor(cx+dx*2, cy+dy*2)` [neighbor junction], `visited=true`, push), else pop → if `braid_chance>0` `braid_dead_ends(...)` → push `Room{1,1,1,1}`.

`braid_dead_ends(map, rng, chance, jw, jh)`: row-major over junctions; `open = count of dirs where is_floor(wall-between)`; if `open==1 && rng.chance(chance)`: `rng.shuffle(&mut dirs)`, reopen the first still-closed wall toward an in-bounds neighbor. Only carves interior walls between valid junctions, so the border is never breached.

### The module doctest (what it now pins)

Three generators are deterministic (`assert_eq!` a re-run each) and `maze.is_floor(1,1)` — so a broken start-junction seed, a non-visiting DFS, or a determinism regression reds the doctest.

---

## Code Analysis

- **`generate_maze(width, height, seed, &MazeParams) -> DungeonMap`** (`src/mapgen.rs`, public, re-exported). Deterministic; explicit-stack recursive backtracker. Degenerate/overflow → empty map via `new_walls` (like the other two generators). Guard: `map.width < 3 || map.height < 3` returns the (possibly empty) all-wall map with no room.
- **`MazeParams { braid_chance: f32 }`** + `Default { 0.0 }`. Clamped `0.0..=1.0` at use.
- **`braid_dead_ends`** — private module helper (see Evidence). Short-circuit `rng.chance` keeps determinism; shuffles a fixed `[(i32,i32);4]` for direction order.
- **`DungeonMap`** — unchanged struct; `rooms` may now hold a maze-spawn 1×1 room at `(1,1)`. `index`/`set_floor`/`tiles`/`is_floor`/`is_wall` are the same private/public surface the maze code uses (same module).
- **Junction coordinate mapping:** junction `(jx,jy)` ↔ grid cell `(1+2·jx, 1+2·jy)`; `jidx(jx,jy) = jy*jw + jx`; `jw=(width−1)/2`, `jh=(height−1)/2`. Wall-between two adjacent junctions is the cell at distance 1 in the step direction; the neighbor junction is at distance 2.
- **`examples/maze_generation.rs`** — `make(seed, braid) -> (DungeonMap, IVec2)`, `is_dead_end(map,x,y)` (floor with exactly 1 orthogonal floor neighbor → warm tint), `Demo` system (input → move/regen/toggle-braid → render → HUD), `connectivity(map) -> (reached, total)` (headless self-check). COLS 45 / ROWS 29 / CELL 15 / BRAID_CHANCE 0.55.
- **The verify gate** (`./scripts/verify.sh`): fmt → clippy --all-targets → wasm build (lib+bins) → wasm clippy --lib → test --all-targets → test --doc → rustdoc `-D warnings`. Read its exit from a non-piped call; a background task's trailing `echo`/`;`-chain reports the echo's 0, not verify's.

---

## Files Changed (all in PR #370 / `38d827a`, +284/−6 excl. lock/changelog)

### Source code
- `src/mapgen.rs` (+285) — module doc rewrite ("three deterministic generators" + a maze paragraph); `MazeParams` + `Default`; `generate_maze` + `braid_dead_ends`; +9 unit tests; maze doctest clause. **No existing fn changed.**
- `src/lib.rs` — re-export extended (`generate_maze, MazeParams`).

### Example (acceptance test)
- `examples/maze_generation.rs` (+~290, NEW, flat/auto-discovered) — playable perfect maze with the B-braid toggle + headless connectivity self-check.

### Docs / release
- `docs/CHANGELOG.md` — 0.131.0 entry.
- `Cargo.toml` / `Cargo.lock` — 0.130.0 → 0.131.0.
- `CLAUDE.md` — header v1.6.224 → v1.6.225; mapgen module-map row updated for three generators + the example.

### Memory (not in the PR)
- `engine-current-state.md` — seq **189** tip-line prepend.
- `MEMORY.md` — engine-current-state index hook refreshed (was stale v0.124.0/seq 177 → v0.131.0/seq 189).

---

## User Feedback & Preferences (REQUIRED)

- **The board takes priority; when both channels are empty, ASK — do NOT self-pick.** Reinforced by the opening prompt ("The shelf is exhausted; ask first. Do NOT skip the board gate or assume a direction."). Honored — asked via AskUserQuestion, user picked the maze mode.
- **Terse, menu-style direction.** The user chose "미로 생성기 (procgen 3번째)" from the 4-option menu with no re-plan invited or notes added — the same terse-approval pattern as seq 1.
- **Merge authority is delegated** — squash on green CI, async auto-merge default, no per-PR re-confirm. Honored (armed `--auto` on #370).
- **User-facing reports in Korean; code, docs, commit messages, PR bodies, handoffs (the .md) in English.** Followed throughout (all status updates + the direction question + this report's narration in Korean; artifacts in English).
- **No mid-session interventions this time** — unlike seq 1 (where the user probed the CI stall), this session ran end-to-end from the single direction choice with no further input. Calibration: when the plan is clear and the board is empty→asked→answered, proceed through the full loop autonomously.

---

## Where We're Going

*(The `procgen-modes` chain is live; the procgen *family* is now complete — rooms/caves/mazes — so the next mode is optional breadth, not a gap.)*

1. **Board gate FIRST, every session.** `../dungeon-merchant/docs/engine-wishlist.md` (next free EW-009; EW-001–008 all closed) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly-filed request preempts everything below.
2. **If both empty (likely): ASK — do NOT self-pick.** The concrete menu, now that the trio is done: **drunkard's-walk caves** (a simpler organic alt to the CA cave), **room-accretion / BSP+cave hybrid**, **Voronoi/region maps**, a **generator trait/enum** (the deferred abstraction — worth it if a game wants runtime generator swap or a "random dungeon" that picks a mode), or a fresh **non-procgen** area (byte-source atlas parity `load_atlas_bytes`, audio-reactive hooks, 2nd capstone game). Frame the ask as "family is complete; here's optional breadth vs a new area."
3. **Whatever is chosen, execute the standard loop:** `/add-feature-example` → `/ship` → `/land-pr`. Each new procgen mode returns a `DungeonMap` so it inherits the pathfinding/FOV/tilemap bridges.
4. **This handoff still needs to land** (if the session is closed with commit): its own `docs(handoff)` PR (procgen-modes seq 2), and the memory seq-bump for the handoff belongs to THAT PR's landing (bump to seq 190 after it merges, so `main @ <hash>` points at the handoff merge).

---

## Risks & Blockers

- **The board-empty ASK rule is load-bearing.** The next session will again find both channels empty. It MUST ask, not self-pick — this is the user's standing rule, explicit in every recent opening prompt.
- **Strict-checks `BEHIND` recurrence.** `main` requires 5 checks with `strict=true`, so any branch open while `main` moves goes `BEHIND` and needs `gh pr update-branch <n>` (re-runs CI, auto-merge stays armed). Hit both seq 1 and seq 2. Expect it whenever a handoff-docs PR (or any other PR) lands mid-session.
- **GitHub hosted-runner backlog can recur** (seq 1 saw ~5h; this session saw none). It's GitHub-side and clears on its own; auto-merge rides it out. If it recurs, seq 1's diagnosis table is the playbook (rule out billing/incident/config, then wait; don't force-re-trigger).
- **CLAUDE.md is over the 200-line soft cap** (pre-existing; ~207 now). Not urgent, but a future edit that *adds* a module-map row should trim detail into a `docs/*.md` per the doc rule.
- **`dungeon-merchant` has no CI/branch protection** (private repo w/o Pro) — its board is discipline, not enforcement. `rust-survivors` has auto-merge DISABLED and is paused/deprecated (don't chase compat).
- **2 audio tests can fail on a no-audio box** (carried gotcha) — not hit this session (mapgen has no audio), relevant to any future audio work.

### Discipline gotchas hit this session (all recoverable)

- **`cargo fmt` before `verify.sh`** — the reflow trap bit again: fmt re-wraps hand-written multi-line `assert_eq!`/imports and reds the FIRST gate. Run `cargo fmt` first.
- **Read gate exit from a non-piped call** — a `run_in_background` verify with a trailing `; echo "…$?"` reports the *echo's* 0, not verify's; the halt is only visible in the log. Capture verify's own `$?` to a file (`verify.sh > log 2>&1; echo $? > exit`). zsh's pipe array is `$pipestatus` (1-indexed); `${PIPESTATUS[0]}` is always empty.
- **Referenced plan/handoff files may not be on local `main`** — they can be on `origin/main` via a separate docs PR not yet pulled. Don't chase them; execute the prompt's intent (board gate, verify, ask) directly. `git fetch origin main` + `git log origin/main` reveals what landed remotely.

## Open Questions

- **Should `procgen-modes` grow a shared `MapGenerator` trait/enum now that there are three modes?** Still answered "not yet" this session (kept free functions). Revisit when a game/example wants to pick a generator at runtime or offer a "random dungeon type" — that's the concrete trigger. A `enum MapKind { Bsp, Cave, Maze }` + `generate(kind, w, h, seed)` dispatcher would be the minimal form.
- **Does the maze want a "growing-tree" variant** (a generalization of recursive-backtracker where the frontier-cell pick is tunable: last=backtracker/tight, random=sprawling, mix=in-between)? It's a one-parameter superset of what shipped and would give the maze a texture knob like the cave's. Natural follow-on if a game wants maze "style" control; not requested.
- **Byte-source atlas parity (`load_atlas_bytes`)** remains an open candidate from the asset-root-windows seq-3 open questions — small, safe, closes the single-file-build story. Worth surfacing in the next direction ask alongside the procgen options.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — engine #370 is merged, main is clean at 38d827a (v0.131.0).

cd ~/Projects/skeleton-engine
git log --oneline -3      # expect 38d827a (v0.131.0) at the tip
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels (a new request preempts everything)
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free EW-009; EW-001–008 all closed)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)

# 2. Verify starting state (read the exit code — do NOT pipe or ;-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 3. Re-prove the shipped maze still works
cargo test --lib mapgen                                  # expect 26 passed
HEADLESS_SHOT=/tmp/maze.png cargo run --example maze_generation
# expect: OK: recursive-backtracker maze is one connected region — 615 floor cells, spawn at (1, 1); exit 0

# 4. Key files to read first (not exhaustive)
#   src/mapgen.rs            — all three generators + MazeParams + tests (the pattern to extend)
#   examples/maze_generation.rs / cave_generation.rs / procgen_dungeon.rs — the example shape
#   plans/handoffs/HANDOFF_procgen-modes_cellular-caves_2026-07-20.md      — seq 1 (parent)

# 5. First action: board gate → if empty, ASK for direction. Do NOT self-pick.
#    The procgen FAMILY is complete (rooms/caves/mazes) — frame the ask as optional
#    breadth (drunkard's-walk / room-accretion / Voronoi / a MapGenerator trait) vs a
#    fresh area (byte-source atlas parity / audio-reactive hooks / 2nd capstone game).
```

---

## Session Closed

**Closed at:** 2026-07-20T13:49:13Z
**Code shipped:** PR #370 `38d827a` — v0.131.0 (`generate_maze` + `MazeParams` + example `maze_generation`), merged + on `main`.
**This handoff lands as:** its own `docs(handoff): procgen-modes seq-2` PR (project convention — the seq-1 handoff landed the same way via #369). Memory `engine-current-state` bumps to **seq 190** after this docs PR merges, so `main @ <hash>` points at the handoff merge.
**Session status:** Handed off to next session.
