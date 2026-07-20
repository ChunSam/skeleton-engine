# Shipped: cellular-automata cave generator — a second procgen mode over `DungeonMap` (v0.130.0)

**Date:** 2026-07-20
**Status:** COMPLETED (PR #368 merged, `main @ f8354fb`, v0.130.0, clean + green)
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `procgen-modes` seq `1` (NEW chain — first of a potential "more procgen modes" breadth arc)
**Parent:** none. This session *opened* on the `asset-root-windows` seq-4 continuation prompt, but that chain was already complete, so the work diverged into a new direction (see Since Last Session).

## Related Handoffs (reference only, NOT parent)

- `HANDOFF_asset-root-windows_chain-complete_2026-07-16.md` (seq 4) + its `PLAN_` twin — the chain this session's opening prompt pointed at. Those files did not exist in the working tree when this session started; they landed on `main` via PR #367 (`b9c8ef7`) *after* this session had already branched. The whole `asset-root-windows` arc (seq 1–4: rodio/Windows, asset roots, loud loaders, embedded images, hot-reload, codec coverage) is DONE and verified downstream.

---

## The Goal

The `asset-root-windows` chain was fully complete and **both downstream request channels were empty** (dungeon-merchant's board: EW-007/EW-008 all `[x]` Verified, next free ID EW-009; rust-survivors: `_None._`). Per the user's standing rule — *when the board is empty, do NOT self-pick; ASK for direction* — the session's real first job was the **board-check gate**, then a direction question. The user chose **"절차적 생성 2번째 모드"** (a second procedural-generation mode) from a four-option menu.

The concrete instance: a **cellular-automata cave generator** — the organic-cavern complement to the existing BSP room-and-corridor dungeon. It had to compose cleanly with the breadth-fov arc that was just built out (`Rng`, `PathGrid`, `FovMap`, `mapgen`), which meant **reusing the existing `DungeonMap` type** rather than inventing a parallel one. End state: `generate_cellular_cave` ships alongside `generate_bsp_dungeon`, is deterministic + guaranteed-connected like BSP, and a playable example (`cave_generation`) is the acceptance test. Shipped as v0.130.0 / PR #368.

---

## Where We Are

- **`main @ f8354fb`, package v0.130.0, CLAUDE.md header v1.6.224, clean tree, all gates green.**
- **Engine PR #368 — MERGED** `f8354fb` (2026-07-20T05:02:06Z, async auto-merge, CI **6/6** including `Build (Windows / DX12)` and `Render tests (lavapipe)`).
- **New public API:** `generate_cellular_cave(width: i32, height: i32, seed: u64, &CaveParams) -> DungeonMap` + `CaveParams { initial_wall_prob: f32, steps: u32, wall_threshold: u32 }` (defaults `0.45 / 4 / 5`). In `src/mapgen.rs`, re-exported from `src/lib.rs` (the `pub use mapgen::{…}` line now includes `generate_cellular_cave, CaveParams`).
- **Reuses the existing `DungeonMap`** — no new grid type. So `to_path_grid()` / `to_tilemap_tiles()` / `FovMap::from_path_grid()` all compose with a cave exactly as with a BSP dungeon. `first_room_center()` works for both because a cave records a **single 1×1 `Room`** at the cavern's central cell.
- **Guaranteed connected**, mirroring BSP's invariant: after CA smoothing, `keep_largest_cavern` flood-fills floor regions (4-connectivity), keeps the biggest, and fills every smaller pocket with wall.
- **Deterministic**: same `(w, h, seed, params)` → identical `DungeonMap`. The `Rng` is drawn from **only** during the initial random-rock fill; smoothing + keep-largest are pure. A module doctest pins this (`assert_eq!(cave, generate_cellular_cave(…))`).
- **Example `examples/cave_generation.rs`** (273 lines, flat/auto-discovered) — renders an organic cave (floor cells touching rock tinted darker to accent the outline), WASD/arrows walk (rock blocks), R regenerates from the next seed. Under `HEADLESS_SHOT` it **self-checks single-cavern connectivity** (flood-fill from spawn reaches every floor cell) and exits non-zero on failure, then captures a screenshot.
- **8 new unit tests** in `src/mapgen.rs` + a cave clause in the module doctest. Lib test total **1250 → 1258** (mapgen module 9 → 17 tests).
- **Both downstream channels still empty** — no new EW filed; the shelf remains exhausted. The next session must ASK again.
- Memory `engine-current-state` bumped to **seq 187** (`main @ f8354fb`, v0.130.0, header v1.6.224).

---

## Since Last Session

- **The opening prompt was the `asset-root-windows` seq-4 execution prompt** — "Read `PLAN_/HANDOFF_asset-root-windows_chain-complete_2026-07-16.md` … Execute from Phase 1 … board gate … if empty, ASK." **Those two files did not exist in the working tree.** The latest asset-root-windows docs on disk were the seq-3 embedded-images pair. The commit log showed why: the chain's last two candidates (hot-reload #365, codec coverage #366) had already shipped, and the seq-4 handoff was a *separate* docs PR (#367) not yet merged.
- **So I read the seq-3 plan/handoff instead** (the most recent on disk) to recover the phase structure + candidate list, and confirmed from the commit log + boards that the chain was genuinely complete.
- **Board-check gate: both channels empty** (verified by reading both files directly). No newly-filed request preempted. → direction ask.
- **User picked candidate A (procgen 2nd mode).** I implemented cellular caves, shipped v0.130.0, and merged.
- **Mid-flight surprise:** after branching `feat/cellular-cave` off `main @ 2adfa2e`, `origin/main` moved to `b9c8ef7` — the seq-4 chain-complete handoff (#367) landed via its own flow. Disjoint files (docs only), so `gh pr update-branch` merged it in cleanly and re-triggered CI.
- **The one real friction was CI infrastructure, not code:** the PR's checks sat `queued` for ~5 hours on a GitHub hosted-runner backlog before a runner picked them up (see Evidence). Once running, 6/6 green in ~7 min.

---

## What We Tried / Did (Chronological)

### Chunk 1 — Onboarding, board gate, state verify

1. **Read the referenced seq-4 files → did not exist.** Located the actual latest asset-root-windows docs (seq-3 embedded-images) and read both in full. Recovered the board-gate + options-A–D structure and the chain-invariant constraints.
2. **Board-check gate (parallel reads):** `../dungeon-merchant/docs/engine-wishlist.md` (EW-007/008 both `[x]` Verified/Closed 2026-07-16, incl. EW-008's hot-reload clause verified on v0.129.0; next free EW-009) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). **Both empty.**
3. **State verification:** `./scripts/verify.sh` → exit 0 (7 gates); re-proofs `cargo test --lib -- codec_decode logical_for_changed watch_path_registers` → 5 passed (1250 total lib); `bash scripts/hot_reload_smoke.sh` → PASS (real notify round-trip). (The smoke script lacked the executable bit — ran it via `bash`.)
4. **Direction ask (AskUserQuestion, Korean):** four candidates — (A) procgen 2nd mode / (B) byte-source atlas parity / (C) audio-reactive hooks / (D) 2nd capstone game. Flagged that the seq-3 handoff's two named candidates were **already shipped**, so the options were all genuinely new. **User chose A.**

### Chunk 2 — Design + implement the cave generator

5. **Read `src/mapgen.rs` in full** to fit the pattern: `DungeonMap { width, height, rooms: Vec<Room>, tiles: Vec<Tile> (private) }`; public `tile`/`is_floor`/`is_wall`/`first_room_center`/`to_tilemap_tiles`/`to_path_grid`; private `new_walls` (caps at `MAX_PATH_GRID_CELLS`, handles degenerate/overflow → empty), `set_floor`, `index`. BSP is deterministic via `Rng` and guaranteed-connected via corridor-carving-on-unwind.
6. **Design locked before coding:** reuse `DungeonMap`; connectivity via keep-largest-cavern (not corridor carving); spawn as a synthetic 1×1 `Room` at the cavern centroid so `first_room_center` stays uniform; `Rng` used only in the random fill for determinism.
7. **Wrote the feature** (`src/mapgen.rs`, +346 lines): module doc rewritten to cover both generators; `use std::collections::VecDeque`; `CaveParams` + `Default`; `generate_cellular_cave` (fill → `cave_smooth` ×steps → `keep_largest_cavern`); helpers `cave_smooth` (Moore-8 wall count, OOB=wall) and `keep_largest_cavern` (BFS region label, keep max, fill rest, push 1×1 centroid room); +8 unit tests; a cave clause in the module doctest.
8. **First borrow error:** `cave_smooth`'s neighbor closure captured `map` immutably while the loop mutated `map.tiles` (E0502). Fixed by capturing the local `prev` snapshot + `(w,h)` instead of `map` (an `is_wall_at` closure over the clone).
9. **Fast targeted checks:** `cargo test --lib mapgen` → 17 passed; `cargo test --doc mapgen` → 2 passed (the 48×32 seed-1234 cave doctest has a walkable spawn).

### Chunk 3 — Example + the "cave feel" iteration

10. **Wrote `examples/cave_generation.rs`** modeled on `procgen_dungeon` (COLS 64, ROWS 44, CELL 14): render, WASD move gated on `is_floor`, R regen (seed+1), HUD, and a headless `connectivity()` self-check (BFS reached vs total floor) + screenshot.
11. **First headless render was too open — 82% floor (2307/2816 cells, seed 1).** A single-threshold "wall iff ≥5 wall neighbors" rule dilates toward one open blob. The screenshot confirmed a valid-but-bland cavern.
12. **Switched `cave_smooth` to the classic 4-5 birth/survival hysteresis:** a *floor* cell needs `wall_threshold` wall neighbors to turn wall, but an *existing* wall survives with `wall_threshold - 1`. Sticky walls keep interior structure. → **46% floor (1301 cells)**, twisty passages + chambers, clearly distinct from BSP rectangles. Updated the `cave_smooth` + `CaveParams::wall_threshold` docs to state the birth/survival rule.
13. **Re-ran mapgen tests (17 pass) + re-captured headless** (OK: one connected cavern, 1301 floor cells, spawn (34,23)). Visual confirmed via the screenshot read.

### Chunk 4 — Wire, verify, ship

14. **`src/lib.rs` re-export** extended to `generate_cellular_cave, CaveParams`.
15. **CLAUDE.md module-map row** rewritten: header → "Procedural map generation (two deterministic generators over a shared `DungeonMap` grid — BSP + cellular caves)"; added the cave generator description + `cave_generation` example. (File is 205 lines — already over the 200 soft cap *before* this edit; my edits were in-line table-row replacements adding no new lines, so not a regression I introduced.)
16. **Stray file check:** a `tmp_codec_smoke.rs` appeared in rust-analyzer diagnostics but was absent on disk + untracked (stale diagnostic) — nothing to clean.
17. **`cargo fmt`** (the reflow trap — it re-wrapped a couple of lines) then **`./scripts/verify.sh` → exit 0** (7 gates).
18. **`/ship` → v0.130.0** (MINOR, new public API): `Cargo.toml` 0.129.1 → 0.130.0; `cargo update -p skeleton-engine` (Locking 0 packages, lock → 0.130.0); CHANGELOG 0.130.0 entry; CLAUDE.md header v1.6.223 → v1.6.224. **Re-ran verify after the bump → exit 0.**

### Chunk 5 — Land + the CI runner-backlog incident

19. **`/land-pr`:** branched `feat/cellular-cave`, committed `feat(mapgen): cellular-automata cave generator … (v0.130.0)` (Co-Authored-By + Claude-Session trailers), pushed, opened PR #368, armed `gh pr merge 368 --auto --squash`.
20. **`mergeStateStatus: BEHIND`** right after arming — `origin/main` had moved `2adfa2e → b9c8ef7` (#367, the seq-4 handoff docs). Disjoint files → `gh pr update-branch 368` merged main in cleanly (head became `819b566`), auto-merge stayed armed, CI re-triggered.
21. **CI queued ~5 hours.** Both the initial-push run (`29708521764`) and the update-branch run (`29708548623`) sat `queued` with zero job starts. Diagnosed (see Evidence): NOT billing (public repo = free unlimited), NOT a GitHub incident (status operational), NOT config (Actions enabled, standard runners, no concurrency block) → a **GitHub hosted-runner assignment backlog**. Cancelled the stale duplicate run `29708521764` (on the pre-update SHA, which the PR's required checks don't reference).
22. **Runner picked it up; 6/6 green; auto-merged `f8354fb` at 05:02:06Z.** Synced `main` (`git pull --ff-only`), pruned `feat/cellular-cave`, bumped memory to seq 187.

---

## Key Decisions

- **Reuse `DungeonMap`, do not invent a `CaveMap`.** The whole value of the breadth-fov arc is that `DungeonMap` bridges to `PathGrid` (pathfinding) and `FovMap` (FOV) with one line each. A cave that returns the same type inherits all of that for free — `to_path_grid`, `to_tilemap_tiles`, `FovMap::from_path_grid`, `first_room_center` all just work. A parallel type would have duplicated every bridge.
- **Connectivity via keep-largest-cavern, not corridor carving.** BSP guarantees connectivity by carving corridors between siblings; a CA cave can't do that (no partition tree). The standard, robust fix is to flood-fill floor regions and keep only the largest, filling the rest — which makes the invariant *unconditional* regardless of the CA rule or seed. This is why the connectivity test passes across all seeds even with `steps: 0`.
- **Classic 4-5 birth/survival hysteresis, not a single threshold.** A state-independent "wall iff ≥5 neighbors" rule dilates to ~82% open floor — a bland single blob. Making an existing wall stickier (survives with `threshold-1`) preserves interior rock and yields ~46% floor with twisty passages. The visual difference (two screenshots) drove this; it's the VISION "fix the feel while writing the example" rule in action.
- **Spawn as a synthetic 1×1 `Room` at the cavern centroid.** Keeps `first_room_center()` a valid uniform spawn accessor for both generators without adding a new field/method to `DungeonMap`. The centroid-nearest cell (min squared distance to the region's mean) gives a roomy, central start — better for the example than a scan-order edge cell. Deterministic (ties in `max_by_key`/`min_by_key` break stably).
- **`Rng` only in the random fill.** Everything after (smoothing, flood-fill, centroid) is pure, so determinism is trivially preserved and a golden `assert_eq!` doctest guards it.
- **`CaveParams { initial_wall_prob, steps, wall_threshold }`, single threshold tunable.** `wall_threshold` is the *birth* threshold; survival is derived (`threshold-1`) rather than exposed as a second field — keeps the tuning surface minimal while documenting the rule.
- **Async auto-merge (no watch-and-confirm gate).** Pure Rust logic + unit tests run on CI's native job; the render half was verified locally (headless screenshot); no OS-gated/audio/hot-reload code. Matches the standing merge delegation. The ~5h CI delay was infra, not a gate the change needed.
- **Cancel the stale duplicate CI run, don't force-re-trigger.** With a broad hosted-runner backlog, a fresh push just queues behind everything else — it can't jump the line. The right move was to remove the redundant run (which was on a SHA the PR didn't reference) and wait for auto-merge.

---

## Evidence & Data

### The cave "feel" iteration (why the rule changed)

| Rule | Floor cells (seed 1, 64×44) | % floor | Look |
|---|---|---|---|
| Single threshold ≥5 (state-independent) | 2307 / 2816 | 82% | one big open blob, thin walls |
| **4-5 birth/survival hysteresis (shipped)** | **1301 / 2816** | **46%** | twisty passages + chambers, organic outline |

Screenshots captured to `/tmp/cave.png` (v1) and `/tmp/cave2.png` (shipped) — the second was sent to the user as the deliverable.

### Test counts

| Point | Lib tests | mapgen module |
|---|---|---|
| Session start (post asset-root-windows) | 1250 | 9 |
| **Cave shipped (v0.130.0)** | **1258** (+8) | **17** (9 BSP + 8 cave) |

New mapgen tests: `cave_same_seed_is_identical`, `cave_different_seeds_differ`, `cave_border_is_all_wall`, `cave_spawn_is_central_floor`, `cave_is_a_single_connected_cavern` (flood-fill across 5 seeds), `cave_composes_with_path_grid_and_fov`, `cave_steps_zero_is_still_connected`, `cave_degenerate_sizes_are_safe`.

### Gate history

| Run | Result | Notes |
|---|---|---|
| targeted: `cargo test --lib mapgen` | 0 | 17 passed |
| targeted: `cargo test --doc mapgen` | 0 | 2 passed (cave doctest) |
| targeted: headless `cave_generation` from repo | 0 | OK: 1 connected cavern, 1301 floor cells |
| full verify (pre-ship) | 0 | 7 gates, after `cargo fmt` |
| full verify (post-`/ship` bump) | 0 | lock + doc re-checked |
| CI #368 | 6/6 | after `update-branch` re-trigger |

### CI #368 — the 6 checks (after the ~5h queue)

| Check | Result |
|---|---|
| Test (native) | ✅ 7m24s |
| Build (Windows / DX12) | ✅ 1m46s |
| Build (WASM) | ✅ 50s |
| Rustdoc | ✅ 36s |
| Package dry-run | ✅ 1m13s |
| Render tests (lavapipe) | ✅ 1m10s |

### The CI hosted-runner backlog — diagnosis (ruling out our-side causes)

| Checked | Finding |
|---|---|
| Repo visibility | **PUBLIC** → Actions minutes free & unlimited → **not billing/quota** |
| GitHub-wide status (`githubstatus.com/api/v2/summary.json`) | **operational**, no incident/maintenance → **not a platform outage** |
| Repo Actions permissions (`gh api repos/…/actions/permissions`) | `enabled: true, allowed_actions: all` → **not disabled** |
| `runs-on` labels (`.github/workflows/ci.yml`) | `ubuntu-latest` ×5 + `windows-latest` ×1, **no self-hosted** → not a missing-runner-label |
| `concurrency` in ci.yml | **none** → the two queued runs didn't block each other |
| Elapsed | runs created ~23:49Z, still `queued` at ~04:52Z → **~5h with zero job starts** |

Conclusion: a GitHub-side hosted-runner assignment backlog (these queue-time degradations often aren't posted to the status page). Remedy applied: cancel the stale duplicate run, wait; a fresh push would not jump a broad backlog.

### Merge log

| Repo | PR | Commit | What |
|---|---|---|---|
| skeleton-engine | **#368** | `f8354fb` | v0.130.0 — `generate_cellular_cave` + `CaveParams` + example `cave_generation` |

### The two new functions (primary artifact)

`generate_cellular_cave` (`src/mapgen.rs`): `new_walls` → return if <3×3 → `Rng::new(seed)` → random fill (`rng.chance(initial_wall_prob.clamp(0,1))` per interior cell, border stays wall) → `cave_smooth` ×`steps` → `keep_largest_cavern`.

`cave_smooth(map, wall_threshold)`: clone `map.tiles` to `prev`; `is_wall_at` closure over `prev` + `(w,h)` (OOB → wall); `survive = wall_threshold.saturating_sub(1)`; per interior cell count Moore-8 walls, threshold = `survive` if the cell was wall else `wall_threshold`, write `Wall`/`Floor`.

`keep_largest_cavern(map)`: BFS-label every floor region (4-conn) into `regions: Vec<Vec<usize>>`; `best_id = (0..regions.len()).max_by_key(|i| regions[i].len())` (None → no floor → return, no room); fill every floor cell not in `best_id` with wall; compute the region centroid, pick the cell nearest it, push `Room { x, y, w: 1, h: 1 }`.

### BSP vs cellular cave — how the two generators differ

| Aspect | `generate_bsp_dungeon` | `generate_cellular_cave` |
|---|---|---|
| Structure | rectangular rooms + 1-wide L-corridors | organic cavern, no discrete rooms |
| Connectivity | carve corridors between BSP siblings on unwind | keep only the largest flood-fill region |
| `rooms` field | one `Room` per carved leaf (`rooms[0]` = spawn) | one synthetic 1×1 `Room` at the cavern centroid |
| Params | `DungeonParams { min_leaf, max_leaf, min_room, room_margin, max_depth }` | `CaveParams { initial_wall_prob, steps, wall_threshold }` |
| `Rng` usage | throughout the recursion (splits, room size/pos, corridor order) | only the initial random fill |
| Shared | both return `DungeonMap`, border is always wall, cap at `MAX_PATH_GRID_CELLS`, feed `to_path_grid` / `to_tilemap_tiles` / `FovMap::from_path_grid`, `first_room_center` spawn | |

### CaveParams tuning guide

| Field | Default | Effect |
|---|---|---|
| `initial_wall_prob` | 0.45 | probability a cell starts as rock; higher → more rock, smaller caverns; clamped `0.0..=1.0` |
| `steps` | 4 | CA smoothing passes; more → smoother, fewer stray specks; `0` → raw fill (keep-largest still runs) |
| `wall_threshold` | 5 | birth threshold (floor→wall needs this many of 8 Moore neighbors as wall); survival = `threshold − 1` |

### The direction ask (full menu — the user chose A)

- **A — 절차적 생성 2번째 모드** *(chosen)*: a second map generator (cellular caves / drunkard's-walk). Composes with mapgen/FovMap/PathGrid/Rng; bounded; low risk.
- **B — 바이트 소스 아틀라스**: `load_atlas_bytes` / `TextureAtlas::from_handle`, extending the just-shipped `load_image_bytes` to spritesheets for single-file/wasm/jam builds. Closes a seq-3 open question.
- **C — 오디오 반응 훅**: beat-clock / amplitude-envelope gameplay hooks (rhythm games, audio-reactive visuals). New genre breadth, but CI audio-verification is limited (2 audio tests can fail on a no-audio box).
- **D — 두 번째 캡스톤 게임**: a complete playable example game composing existing subsystems. Most VISION-pure ("the example is the acceptance test") but the largest for one session.

### Branch protection (context for the `update-branch` / recurrence risk)

`main` requires **5 checks, strict=true**: Build (WASM) / Test (native) / Rustdoc / Package dry-run / Render tests (lavapipe). (`Build (Windows / DX12)` also runs but isn't in the required set.) `allow_auto_merge=true`, `delete_branch_on_merge=true` (enabled 2026-07-02). `strict=true` is *why* a branch goes `BEHIND` when `main` moves after the branch was pushed — the fix is `gh pr update-branch <n>` (re-runs CI, auto-merge stays armed), which this session hit when #367 landed mid-flight.

### The module doctest (what it now pins)

Both generators are deterministic (`assert_eq!` a re-run) and a cave's `first_room_center()` cell is walkable floor — so a broken keep-largest (empty region → no room → `unwrap` panic) or a determinism regression reds the doctest.

---

## Code Analysis

- **`generate_cellular_cave(width, height, seed, &CaveParams) -> DungeonMap`** (`src/mapgen.rs`, public, re-exported). Deterministic; `Rng` used only in the fill. Degenerate/overflow → empty map via `new_walls` (like `generate_bsp_dungeon`).
- **`CaveParams { initial_wall_prob: f32, steps: u32, wall_threshold: u32 }`** + `Default` (0.45 / 4 / 5). `wall_threshold` = birth threshold; survival = `-1`.
- **`cave_smooth` / `keep_largest_cavern`** — private module helpers (see Evidence).
- **`DungeonMap`** — unchanged struct; `rooms` now may hold a synthetic 1×1 cave-spawn room. `index`/`set_floor`/`tiles` are private and used by the new helpers (same module).
- **`examples/cave_generation.rs`** — `make(seed) -> (DungeonMap, IVec2)`, `is_edge(map, x, y)` (floor with an orthogonal wall neighbor → outline tint), `Demo` system (input → move/regen → render → HUD), `connectivity(map) -> (reached, total)` (headless self-check). COLS 64 / ROWS 44 / CELL 14.
- **The verify gate** (`./scripts/verify.sh`): fmt → clippy --all-targets → wasm build (lib+bins) → wasm clippy --lib → test --all-targets → test --doc → rustdoc `-D warnings`. Read its exit from a non-piped call (zsh `$pipestatus` is 1-indexed).

---

## Files Changed (all in PR #368 / `f8354fb`, +633/-7)

- `src/mapgen.rs` (+346) — module doc rewrite (both generators); `CaveParams` + `Default`; `generate_cellular_cave` + `cave_smooth` + `keep_largest_cavern`; `use std::collections::VecDeque`; +8 unit tests; cave doctest clause. No existing fn changed.
- `src/lib.rs` — re-export extended (`generate_cellular_cave, CaveParams`).
- `examples/cave_generation.rs` (+273, NEW, flat) — the acceptance-test example.
- `docs/CHANGELOG.md` — 0.130.0 entry.
- `Cargo.toml` / `Cargo.lock` — 0.129.1 → 0.130.0.
- `CLAUDE.md` — header v1.6.223 → v1.6.224; mapgen module-map row rewritten for both generators + the example.
- Memory `engine-current-state.md` — seq **187**.

---

## User Feedback & Preferences (REQUIRED)

- **The board takes priority; when both channels are empty, ASK — do NOT self-pick.** Reinforced this session: the opening prompt explicitly said "Do NOT skip the board gate or assume a direction. The shelf is exhausted; ask first." Honored — asked, user picked A.
- **Terse approvals are normal.** The direction was chosen via the AskUserQuestion menu ("절차적 생성 2번째 모드"); no re-plan invited.
- **Merge authority is delegated** — squash on green CI, async auto-merge default, no per-PR re-confirm. Honored (armed `--auto` on #368).
- **User-facing reports in Korean; code, docs, commit messages, PR bodies, handoffs in English.** Followed throughout.
- **Mid-session the user asked (in Korean) "머지 확인" then "ci 진행 상황 체크" then "왜 지연되고 있는지 이유 알 수 있어" then "일단 대기".** The expectation: when CI stalls, *investigate and explain the root cause* (not just report status), then wait when told. Honored — produced the runner-backlog diagnosis table, then waited.

---

## Where We're Going

*(Paired with `PLAN_procgen-modes_cellular-caves_2026-07-20.md` — that plan is the authority for the next session.)*

1. **Board gate FIRST, every session.** `../dungeon-merchant/docs/engine-wishlist.md` (next free EW-009; EW-001–008 all closed) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly-filed request preempts everything below.
2. **If both empty (likely): ASK — do NOT self-pick.** The shelf is exhausted. But `procgen-modes` is now a live chain with an obvious menu of next modes, so the ask can be concrete: **maze generation** (recursive-backtracker / growing-tree), **drunkard's-walk caves** (a simpler organic alternative), **room-accretion / BSP+cave hybrid**, **Voronoi/region maps**, or a fresh non-procgen area (byte-source atlas parity, audio-reactive hooks, 2nd capstone game).
3. **Whatever is chosen, execute the standard loop:** `/add-feature-example` → `/ship` → `/land-pr`. Each new procgen mode should return a `DungeonMap` (or clearly document why not) so it inherits the pathfinding/FOV/tilemap bridges.

---

## Risks & Blockers

- **GitHub hosted-runner backlog can recur.** This session's PR queued ~5h. It's GitHub-side and clears on its own; auto-merge rides it out. Don't force-re-trigger a broad backlog. If it recurs, the diagnosis table above is the playbook (rule out billing/incident/config, then wait).
- **The board-empty ASK rule is load-bearing.** The next session will again find both channels empty. It MUST ask, not self-pick — this was explicit in the opening prompt and is the user's standing rule.
- **CLAUDE.md is at 205 lines** (over the 200 soft cap, pre-existing). Not urgent, but a future edit that must *add* a row should consider trimming detail into a `docs/*.md` per the doc rule.
- **`dungeon-merchant` has no CI / branch protection** (private repo without Pro) — its board is discipline, not enforcement. `rust-survivors` has auto-merge DISABLED (merge by hand) and is a paused/deprecated project (don't chase compat).
- **2 audio tests can fail on a no-audio box** (carried gotcha) — not hit this session (mapgen has no audio), but relevant to any future audio work.

### Discipline gotchas hit this session (all recoverable)

- **`scripts/hot_reload_smoke.sh` lacks the executable bit** — `./scripts/hot_reload_smoke.sh` gives "permission denied"; run it as `bash scripts/hot_reload_smoke.sh`. (Same likely for other new scripts.)
- **`cargo fmt` before `verify.sh`** — the fmt-reflow trap bit again: after hand-editing `src/mapgen.rs`/the example, fmt re-wrapped a few lines; running `cargo fmt` first keeps `fmt --check` (the first gate) green.
- **Read gate exit codes from a non-piped call** — zsh's pipe-status array is `$pipestatus` (1-indexed); `${PIPESTATUS[0]}` is always empty. Used `cmd > /tmp/x.log 2>&1; echo $?` (or a `run_in_background` task's completion code) throughout.
- **Borrow-checker on snapshot-read CA passes** — a neighbor-reading closure must capture the *cloned snapshot* + dims, not `&map`, or the write loop can't mutate `map.tiles` (E0502). Pattern reusable for any grid double-buffer step.

## Open Questions

- **Should `procgen-modes` grow a shared trait/enum** (e.g. a `MapGenerator` abstraction) once there are 3+ modes, or stay free functions? Free functions are simplest now; revisit if the example/game surface wants to swap generators at runtime.
- **Does a cave want tunable multi-cavern output** (keep the top-K regions + connect them with tunnels) rather than strictly the largest? Not requested; the single-cavern guarantee is the clean default. A `min_caverns` / connect-K option is a natural follow-on if a game wants island-style maps.
- **Byte-source atlas parity (`load_atlas_bytes`)** remains an open candidate from the asset-root-windows chain's seq-3 open questions — small, safe, closes the single-file-build story. Worth surfacing in the next direction ask.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — engine #368 is merged, main is clean at f8354fb (v0.130.0).

cd ~/Projects/skeleton-engine
git log --oneline -3      # expect f8354fb (v0.130.0) at the tip
git status -s             # expect clean

# 1. READ THE BOARD FIRST — both channels (a new request preempts the plan)
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free EW-009; EW-001–008 all closed)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)

# 2. The plan for this session
cat plans/handoffs/PLAN_procgen-modes_cellular-caves_2026-07-20.md

# 3. Verify starting state (read the exit code — do NOT pipe or `;`-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 4. Re-prove the shipped cave still works
cargo test --lib mapgen                                  # expect 17 passed
HEADLESS_SHOT=/tmp/cave.png cargo run --example cave_generation
# expect: OK: cellular-automata cave is one connected cavern — … ; exit 0

# 5. First action: board gate → if empty, ASK for direction (Phase 1/2 of the plan).
#    Do NOT self-pick a feature. Present the procgen-mode menu + non-procgen options.
```
