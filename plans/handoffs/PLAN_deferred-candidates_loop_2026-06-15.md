# PLAN — deferred-candidate loop (autonomous /loop, opus-supervised)

**Date:** 2026-06-15
**Chain:** `deferred-candidates` seq 1
**Mode:** autonomous `/loop` (dynamic self-paced). opus plans + gates + commits; sonnet implements.
**Goal:** work down the deferred breadth candidates (NEXT_WORK 2026-06-09 audit + tilemap-arc deferrals), each as its own additive minor-version feature PR, to completion.

## Governance / completion

- Each candidate = independent branch off `main` → CI-green PR. **No self-merge** — PRs await the user's "머지 확인" (standing rule). The loop keeps building the next candidate while PRs accumulate.
- **Gate6 per candidate:** `cargo fmt --check`, `clippy --all-targets -D warnings`, `cargo build --target wasm32` (lib+bins), `cargo test --all-targets`, `RUSTDOCFLAGS=-D warnings cargo doc`, `cargo package --locked`.
- VISION: a feature isn't done until a playable example exercises it (or, for pure ergonomic helpers, until an existing example is refactored onto it). Fix awkward API before release.
- **Failure rule:** if a candidate fails 3+ times at a point, report and pause for the user.
- **Overall completion point:** candidates 1–5 each open as a CI-green PR. Then stop the loop + notify.

## Ordered candidates

| # | Candidate | Version | Validation | Done = |
|---|---|---|---|---|
| 1 | Ergonomic helpers: `World::with_resource_mut` + `CharacterController::top_down()` | 8.3.0 | refactor `dig_quest` (no new example) | both APIs + dig_quest using them + Gate6 + PR/CI |
| 2 | Audio ducking / sidechain (`AudioBus` duck) | 8.4.0 | `settings_menu` ducks BGM under dialogue | ducking API + example + Gate6 + PR/CI |
| 3 | Diagonal pathfinding (8-dir A* + corner-cut rule) | 8.5.0 | extend/new from `maze_escape` | `find_path` diagonal option + example + Gate6 + PR/CI |
| 4 | Multi-terrain autotiling (`ConnectRule` extension) | 8.6.0 | grass/water/sand example | per-terrain connection + example + Gate6 + PR/CI |
| 5 | Save migration (versioned schema) | 8.7.0 | old→new save example | migration API + example + Gate6 + PR/CI |

**Out of scope (deferred to dedicated arcs — too large/narrow for this loop):** data-driven anim/particle assets (RON; large, overlaps editor), editor tile-painting (editor-internal; reuses `set_tile` + reactive system), RTL/per-locale fonts (font-system rework).

## Candidate 1 detail (v8.3.0 — ergonomic helpers)

Both gaps were surfaced by the tilemap arc's `dig_quest` (awkwardness #3 and #5).

- `World::with_resource_mut<R, F>(&mut self, f: F) -> bool where F: FnOnce(&mut R, &mut World)` — temporarily removes resource `R`, runs `f(&mut r, self)` (giving simultaneous `&mut R` + `&mut World`), re-inserts; returns `false` if `R` absent. Hides the `remove_resource`/`insert_resource` dance.
- `CharacterController::top_down() -> Self` — like `new()` but snap-to-ground + autostep disabled (the defaults are platformer-tuned and cause top-down wall-sticking).
- Refactor `dig_quest` `move_character` + `sync_colliders` blocks onto `with_resource_mut`, and the player onto `top_down()`. This is the acceptance test.
- No new public re-exports (both are methods on already-exported types).

## Verification cadence

After each candidate's implementation: opus runs Gate6 independently (never trusts agent self-report or IDE diagnostics — stale `ColliderHandle` E0308 phantoms are routine; trust `cargo check`). `cargo clean -p skeleton-engine` before any GUI re-playtest. Native playtest where a new example warrants it ([[playtest-windowed-examples]]; swift CGEvent keyhold tool at `/tmp/dig_quest_playtest/keyhold` — `just_pressed` actions work as taps, `is_pressed` movement needs holds).

## Engine conventions (carry-over)

Coordinate origin = TOP-LEFT (`Camera.position` = viewport top-left, Y down; `DrawText` screen-space top-left) — place example maps at POSITIVE world coords. Patches non-breaking; features = minor bump. Korean prose to user, English code/docs.
