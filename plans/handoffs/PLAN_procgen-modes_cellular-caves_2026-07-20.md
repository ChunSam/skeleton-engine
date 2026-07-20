# Plan: next procgen mode (or a fresh direction) — board-gated selection then ship

**Date:** 2026-07-20
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `procgen-modes` seq `1`
**Context:** See `HANDOFF_procgen-modes_cellular-caves_2026-07-20.md` for this session's data (the cellular-cave ship, design decisions, CI-backlog incident, and the reusable `DungeonMap`/keep-largest pattern).

---

## Problem Statement

The `procgen-modes` chain is now live: seq 1 shipped the **cellular-automata cave generator** (v0.130.0, PR #368), a second map generator over the shared `DungeonMap` alongside BSP. Both downstream request channels are **empty** (dungeon-merchant EW-001–008 all closed, next free EW-009; rust-survivors `_None._`), and the self-pick shelf is exhausted. So — exactly as at the start of the session that produced this plan — the next session's first job is the **board-check gate**, then, if empty, a **direction ask** (do NOT self-pick; this is the user's standing, explicitly-restated rule). This plan makes that selection concrete and, for the most likely branch (another procgen mode), pre-loads the implementation so the chosen work starts immediately.

## Key Findings

*(Conclusions from seq 1 — raw data in the handoff.)*

- **Reusing `DungeonMap` is the winning pattern** — a generator that returns it inherits `to_path_grid` / `to_tilemap_tiles` / `FovMap::from_path_grid` / `first_room_center` for free. → **every new procgen mode should return `DungeonMap`** unless there's a strong reason not to. → drives Phase 3.
- **Connectivity via keep-largest-region is unconditional** — flood-fill floor, keep the biggest, fill the rest. Works for any organic generator regardless of rule/seed (cave's `keep_largest_cavern` proves it). → reusable for maze/drunkard's-walk/Voronoi. → drives Phase 3.
- **Determinism = draw from `Rng` only where randomness is intrinsic**, keep the rest pure, and pin it with a golden `assert_eq!` doctest + unit test. → drives Phase 3.
- **The board-empty rule is ASK, never self-pick** — reinforced verbatim in seq 1's opening prompt. → drives Phase 1/2.
- **`procgen-modes` has an obvious next-mode menu** (maze, drunkard's-walk, room-accretion/hybrid, Voronoi) plus non-procgen options (byte-source atlas parity, audio-reactive hooks, 2nd capstone game). → drives Phase 2's ask.
- **CI can queue for hours on a GitHub hosted-runner backlog** (seq 1 saw ~5h). Async auto-merge rides it out; don't force-re-trigger. → drives Risks.

## Anti-Goals (What NOT To Do)

- **Do NOT self-pick a feature when both channels are empty.** Present options and let the user choose. This is the user's explicit standing rule and was restated in seq 1's prompt.
- **Do NOT invent a parallel map type** (a `CaveMap`/`MazeMap`). Return `DungeonMap` so the pathfinding/FOV/tilemap bridges compose. Only diverge with a documented reason.
- **Do NOT skip the board gate.** A newly-filed EW/rust-survivors request preempts everything below and becomes the real work.
- **Do NOT force-re-trigger CI on a hosted-runner backlog** (empty commit / close-reopen won't jump a broad queue). Arm async auto-merge and wait; use the handoff's diagnosis table if it stalls.
- **Do NOT abandon determinism or the connectivity guarantee** for a new generator — they are the chain's invariants and what the tests assert.

## Plan

### Phase 1: Board-check gate + clean-state verification

**Goal:** Confirm there is no filed downstream request (which would preempt), and that `main` is green before any new work.

**Why this approach:** The board takes priority over self-picked work; a new request is the real job. And a new mode should start from a proven-clean tree.

- Read `../dungeon-merchant/docs/engine-wishlist.md` (expect next free **EW-009**; EW-001–008 all `[x]`) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (expect `_None._`). If a NEW request exists → **serve it instead of Phase 2/3** (it becomes the work; follow its acceptance criteria + the standard loop).
- Confirm clean state: `git log --oneline -3` (expect `f8354fb` v0.130.0 tip), `git status -s` (clean).
- `./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?` → expect 0 (read the exit non-piped; zsh `$pipestatus` is 1-indexed).
- Re-prove seq 1's ship: `cargo test --lib mapgen` (expect 17 passed); `HEADLESS_SHOT=/tmp/cave.png cargo run --example cave_generation` (expect "OK: … one connected cavern …", exit 0).

**Files:** none modified (read-only gate).
**Validates with:** both boards read; verify exit 0; the cave re-proof passes. Success = you can state definitively whether a request is filed and that the tree is green.
**Rollback:** n/a.

### Phase 2: Direction ask (only if both channels empty)

**Goal:** Let the user choose the next direction — do not self-pick.

**Why this approach:** Board-empty → ASK is the standing rule. `procgen-modes` gives a concrete menu, so the ask is specific rather than open-ended.

- Present via `AskUserQuestion` (Korean), recommending nothing over the user's interest. Candidate options:
  - **Maze generation** — recursive-backtracker or growing-tree perfect maze → `DungeonMap` (corridors = floor, walls between). Deterministic, inherently connected (spanning tree). Strong, bounded, distinct from both rooms and caves.
  - **Drunkard's-walk caves** — a simpler organic generator (random-walk carve to a target floor ratio); pairs with keep-largest. Contrast to the CA cave.
  - **Room-accretion / BSP+cave hybrid** — place rooms then connect with cave-like tunnels, or run CA inside BSP leaves.
  - **Non-procgen:** byte-source atlas parity (`load_atlas_bytes`), audio-reactive hooks, or a 2nd capstone game (see the handoff's "direction ask" menu for the full descriptions).
- If the user names something off-menu, take it. If they defer to a recommendation, suggest **maze generation** (most distinct third structure; clean spanning-tree connectivity; obvious example).

**Files:** none.
**Validates with:** a chosen direction. Success = a single, agreed next feature.
**Rollback:** n/a.

### Phase 3: Implement the chosen mode via the standard loop

**Goal:** Ship the chosen feature with a playable example, verified and merged.

**Why this approach:** The VISION loop — the example is the acceptance test — and the reusable `DungeonMap`/keep-largest/determinism pattern from seq 1 make a new procgen mode a well-trodden path.

- Run `/add-feature-example` → `/ship` → `/land-pr` (the loop seq 1 used cleanly).
- **If a procgen mode:** add `generate_<mode>(w, h, seed, &<Mode>Params) -> DungeonMap` in `src/mapgen.rs`; re-export in `src/lib.rs`; seed from `engine::Rng`; guarantee connectivity (maze = spanning tree is already connected; organic = `keep_largest_cavern`-style flood-fill); record a spawn `Room` so `first_room_center` works; write an `examples/<mode>_generation.rs` modeled on `cave_generation` (render, WASD walk, R regen, headless connectivity self-check); add unit tests (determinism, border, connectivity across seeds, `to_path_grid`/`FovMap` composition, degenerate sizes) + a doctest clause; update the CLAUDE.md mapgen row.
  - **Maze specifics:** recursive-backtracker on a cell grid where walls sit between cells; carve on a randomized DFS using `Rng::shuffle` for neighbor order; the result is a *perfect maze* (already fully connected, no keep-largest needed). Spawn = any cell (e.g. `(1,1)`).
  - **Drunkard's-walk specifics:** carve from a start cell via a random walk (`Rng`-picked direction each step) until floor ≥ a target fraction of interior; then `keep_largest_cavern` to guarantee connectivity + record the spawn.
- **If non-procgen:** follow that feature's natural shape (byte-atlas → mirror `load_image_bytes`; audio → the `Audio` facade; capstone → compose existing subsystems). Same loop.
- Async auto-merge on green CI (no judgment gate for pure-Rust + example work; verify the render half locally via the headless screenshot).

**Files:** `src/mapgen.rs` (+ new generator/params/tests) or the relevant subsystem; `src/lib.rs` (re-export); `examples/<name>.rs` (new); `CLAUDE.md` (row); `docs/CHANGELOG.md` + `Cargo.toml` + `Cargo.lock` (via `/ship`).
**Validates with:** `./scripts/verify.sh` exit 0; new unit tests + doctest pass; headless example self-checks; CI 6/6; PR merged.
**Rollback:** the feature is additive — revert the branch; no existing signature changes.

## Dependencies & Order

- **Phase 1 → Phase 2 → Phase 3, strictly sequential.** Phase 2 only runs if Phase 1 finds both channels empty; a filed request replaces Phase 2/3.
- **Within Phase 3**, if the user picks *multiple* modes (unlikely in one session), they're independent and could be parallelized in worktrees — but one coherent PR per mode.

## Risks & Mitigations

- **User picks a direction that doesn't fit `DungeonMap`** (e.g. audio hooks). *Likely: medium.* Mitigation: that's fine — the chain tag is a convenience, not a constraint; follow the feature's own shape and still ship via the standard loop.
- **CI hosted-runner backlog recurs** (~5h in seq 1). *Likely: low-medium.* Mitigation: async auto-merge rides it out; don't force-re-trigger; use the handoff's diagnosis table (rule out billing/incident/config, then wait); cancel any stale duplicate run from an `update-branch`.
- **`main` moves mid-flight** (a docs/handoff PR lands) → branch goes `BEHIND` (strict checks). *Likely: medium.* Mitigation: `gh pr update-branch <n>` (auto-merge stays armed), or branch off freshly-pulled main.
- **A new generator ships disconnected/degenerate maps.** *Likely: low.* Mitigation: the connectivity + degenerate-size unit tests are mandatory (copy the cave test shape); the example's headless self-check is a second guard.

## Success Criteria

- **Minimum viable:** Phase 1 completed — boards checked, state green, cave re-proof passes; and either a filed request is being served OR the user has chosen a direction.
- **Full:** the chosen feature is implemented with a playable example (the acceptance test), `./scripts/verify.sh` exit 0, new tests green, and the PR merged to `main` with the version bumped and memory seq advanced.
- **Invariant preserved (if a procgen mode):** returns `DungeonMap`, deterministic (golden test), guaranteed-connected, `first_room_center` valid; no existing signature changed.

## Quick Start

```bash
# Restore context
cat plans/handoffs/HANDOFF_procgen-modes_cellular-caves_2026-07-20.md

# BOARD-CHECK GATE (before anything) — a filed request preempts this plan
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free EW-009; EW-001–008 all closed)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)

# Confirm clean state (read the exit code — do NOT pipe or `;`-chain it)
git log --oneline -3     # expect f8354fb (v0.130.0)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# Re-prove seq 1's ship
cargo test --lib mapgen                                   # expect 17 passed
HEADLESS_SHOT=/tmp/cave.png cargo run --example cave_generation   # expect OK, exit 0

# Key files for a procgen-mode Phase 3
#   src/mapgen.rs             — the two existing generators + the DungeonMap/keep-largest pattern to copy
#   src/lib.rs               — the mapgen re-export line
#   examples/cave_generation.rs — the example template (render + WASD + R + headless self-check)

# First concrete action (Phase 1): read BOTH boards. If empty → AskUserQuestion with the
# procgen-mode menu (do NOT self-pick). If a request is filed → serve it.
```
