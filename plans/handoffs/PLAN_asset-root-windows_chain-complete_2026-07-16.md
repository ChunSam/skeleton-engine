# Plan: board-check gate, then a new direction (the `asset-root-windows` chain is complete)

**Date:** 2026-07-16
**Status:** PLANNED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `asset-root-windows` seq `4`
**Context:** See `HANDOFF_asset-root-windows_chain-complete_2026-07-16.md` for this session's data (hot-reload #365 + codec coverage #366 + board note dm#30), and the parents for the chain's `resolve()`/identity constraints.

---

## Problem Statement

The `asset-root-windows` chain is **complete** — 6 engine PRs served the whole rust-survivors + dungeon-merchant packaged-build bug report, down to the last EW-008 acceptance clause (hot-reload under an asset root, v0.129.0) and the last untested codec (vorbis/`.ogg` decode coverage, v0.129.1). Both downstream request channels are **empty of open engine work**, and the self-pick shelf tied to this chain is exhausted. There is therefore **no pre-decided feature to build next.** The next session's job is to run the board-check gate and, if nothing is filed, get a direction from the user before writing any code. This plan is unusual for that reason: its Phase 1 is a gate + decision, not a code change. See Where We're Going in the handoff.

## Key Findings

*(Conclusions from this session — raw data in the handoff.)*

- **The chain is fully served; nothing is pending.** Engine `main @ 2adfa2e` (v0.129.1), clean; dm #30 merged; EW-007/008 are `Shipped` awaiting the GAME's verify (their action, not the engine's). → **drives Phase 1** (confirm, don't re-do).
- **Both request channels are empty of NEW engine work** (dm next free EW-009; rust-survivors `_None._`). → **drives Phase 1**: the gate must run first; a filed request preempts everything.
- **The self-pick shelf is exhausted** — every widget and every asset-root candidate has shipped. → **drives Phase 2**: if the board is empty, ASK the user rather than assume a direction.
- **Standing directive: board first, then ASK.** The user has repeatedly chosen the next direction from an options list when the shelf is empty (embedded-images "A", codec "the remaining"). → **drives Phase 2**: present concrete candidates, let the user pick.
- **Prior handoffs floated candidate NEW areas, none started:** more procgen modes, audio-driven gameplay hooks, a 2nd capstone game, tilemap streaming. → **drives Phase 3** (pre-scoped so a chosen direction starts fast).
- **The engine's release loop is mature and consistent:** `/add-feature-example` (or `/add-ui-widget`) → `/ship` → `/land-pr`, with the judgment-gate distinction (watch-and-confirm for CI-unverifiable behavior, async for CI-verifiable). → **drives Phase 3** (whatever is chosen uses this loop).

## Anti-Goals (What NOT To Do)

- **Do NOT assume a next feature and start coding it.** The chain is closed; there is no queued work. Skipping the board gate + the direction ask is the one wrong move here.
- **Do NOT re-open or "polish" the `asset-root-windows` chain.** It is served end to end. The two noted loose ends (a wasm `load_image_bytes` smoke; byte-source atlas/`_with_format` parity) are unrequested — do them only if a use case appears, not as busywork.
- **Do NOT wire the hot-reload smoke into CI** (notify latency → flaky). The two unit tests are the deterministic guards; the smoke stays a manual/scripted proof.
- **Do NOT tighten the codec test to exact sample counts** — lossy codecs make that fragile. Rate/channels/non-empty is the right level.
- **Do NOT treat "MERGED" on a dungeon-merchant PR as verification** — that repo has no CI/branch protection.

## Plan

### Phase 1: Board-check gate + confirm clean starting state

**Goal:** Determine whether a new downstream request exists; if so, it becomes the real work and this plan yields to it.

**Why this approach:** The standing directive is board-first, and a filed request always preempts self-picked work. Confirming the clean tree also guards against acting on a stale checkout.

- `cd ~/Projects/skeleton-engine`; `git log --oneline -3` (expect `2adfa2e` v0.129.1) and `git status -s` (clean).
- Read `../dungeon-merchant/docs/engine-wishlist.md` — check for any request newer than EW-008 (next free is **EW-009**), and note whether the game has moved EW-007/008 to `Verified` (informational; that's their action).
- Read `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` — check "Open Requests" (currently `_None._`).
- Run `./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"` and re-prove this session's work (`cargo test --lib codec_decode`, the two hot-reload unit tests, `./scripts/hot_reload_smoke.sh`).
- **Branch outcome:** if a NEW request is filed → stop this plan, scope that request as its own work (use `/add-feature-example` or the appropriate loop) and treat it as the session's real Phase 1+. If both channels are empty → go to Phase 2.

**Files:** none (read-only gate).
**Validates with:** a clear statement of "board has X" or "board empty"; `VERIFY_EXIT=0`; this session's tests still green.
**Rollback:** n/a.

### Phase 2: If the board is empty, ASK the user for a direction

**Goal:** Get an explicit direction before writing code, because the self-pick shelf is exhausted.

**Why this approach:** The user has consistently picked the next area from an options list when nothing is queued; assuming a direction risks building the wrong thing. This is a decision phase, not a build phase.

- Present the candidate NEW areas below via `AskUserQuestion` (recommend one, but let the user choose or propose their own). Keep each option one concrete sentence.
- **Candidate A — More procgen modes.** Extend `src/mapgen.rs` (which currently has BSP `generate_bsp_dungeon`): a cellular-automata cave generator and/or multi-level dungeons with stairs. Composes with the existing `FovMap`/`PathGrid`/`Rng`; example-driven (cf. `roguelike`, `procgen_dungeon`). Highest-continuity with the recent breadth work.
- **Candidate B — Tilemap streaming.** Chunked/large tilemaps that load/unload by camera position; extends `src/tilemap/`. A genuinely new subsystem; more design surface.
- **Candidate C — A 2nd capstone game.** A small playable game under `examples/games/` (like `coin_race`/`roguelike`) that exercises breadth in real play; surfaces API-awkwardness to fix (the VISION loop).
- **Candidate D — Audio-driven gameplay hooks.** Beat/onset or amplitude-reactive components on top of the mature audio stack. Least-defined API surface; scope carefully before committing.
- If the user proposes something else, take that; the candidates are a menu, not a constraint.

**Files:** none (decision).
**Validates with:** an explicit chosen direction (or a filed request from Phase 1 that supersedes this).
**Rollback:** n/a.

### Phase 3: Execute the chosen direction via the standard release loop

**Goal:** Ship the chosen work as a merged PR, following the engine's established loop.

**Why this approach:** The loop is consistent and proven this session (twice). The judgment-gate distinction determines the merge mode.

- Use `/add-feature-example` (engine feature + a playable example that IS the acceptance test) or `/add-ui-widget` (if somehow a widget), per the work.
- Follow the VISION rule: **the example is the acceptance test; if the API feels awkward writing it, fix the API before release.**
- Run the full verify gate (`cargo fmt` first — the reflow trap), read exit from a non-piped call.
- `/ship` for the version/CHANGELOG/CLAUDE.md paperwork (pre-1.0: MINOR for a feature, PATCH for a fix/test-only).
- `/land-pr`: **async auto-merge** if green CI fully verifies the change; **watch-and-confirm** if it's a judgment gate (windowed playtest, audio playback, hot-reload, a wasm smoke — anything CI can't exercise). See the handoff's merge-mode table.
- Bump `engine-current-state` memory seq after the merge lands.

**Files:** per the chosen direction.
**Validates with:** verify exit 0, CI green, the example demonstrates the feature in real play.
**Rollback:** revert the feature commit; the loop is per-PR so each is independently revertable.

## Dependencies & Order

- **Phase 1 → Phase 2 → Phase 3, strictly sequential and gated.** Phase 2 only runs if Phase 1 finds the board empty; Phase 3 only runs after a direction is chosen (by a filed request in Phase 1, or by the user in Phase 2).
- No parallelism at the plan level — the direction must be settled first. Once a direction is chosen, sub-work within it may parallelize (per that work's own shape).

## Risks & Mitigations

- **The next session skips the gate and builds a self-picked feature.** *Likely: low, but it's the main failure mode.* Mitigation: Phase 1 is explicitly the board gate; this plan's whole point is "don't assume a direction."
- **A newly-filed request is subtle or large.** *Likely: low.* Mitigation: scope it as its own work (its own plan if big); don't force it into this plan's phases.
- **The user wants to leave breadth entirely** (e.g. a refactor, a docs pass, or pausing). *Likely: medium.* Mitigation: Phase 2's ask is open — the candidates are a menu, and "something else / nothing right now" is a valid answer.
- **Acting on a stale checkout.** *Likely: low.* Mitigation: Phase 1 confirms `git status` clean + tip `2adfa2e` before anything.

## Success Criteria

- **Minimum viable:** the board gate ran, its result is stated, and either (a) a filed request is being served, or (b) the user has been asked for a direction — no code written on an assumed direction.
- **Full:** a direction is chosen and shipped as a merged PR via the standard loop, with the example demonstrating it in real play; `engine-current-state` bumped to the new seq.
- **Invariant:** the completed `asset-root-windows` chain is not re-opened or "polished"; both trees stay clean; no regression to the 1250 lib-test baseline.

## Quick Start

```bash
# Restore context
cat plans/handoffs/HANDOFF_asset-root-windows_chain-complete_2026-07-16.md

# BOARD-CHECK GATE (Phase 1 — before any code)
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free EW-009; EW-007/008 Shipped, awaiting the game's verify)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)

# Confirm starting state (read the exit code — do NOT pipe or ;-chain it)
cd ~/Projects/skeleton-engine
git log --oneline -3     # expect 2adfa2e (v0.129.1)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# Re-prove this session's work
cargo test --lib codec_decode
cargo test --lib -- asset::tests::logical_for_changed asset::tests::watch_path_registers
./scripts/hot_reload_smoke.sh

# First concrete action (Phase 1): read both request channels.
#   - If a NEW request exists → serve it (it becomes the real work; this plan yields).
#   - If both empty → Phase 2: ASK the user for a direction (candidates A–D in the plan).
#   Do NOT start coding a self-picked feature without the gate + the ask.
```
