# Field-of-view / fog-of-war (`FovMap`) shipped (v0.120.0)

**Date:** 2026-07-09
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `breadth-fov` seq `1` (NEW chain — the non-widget breadth pivot)
**Parent:** `HANDOFF_listbox-widget_switch-widget_2026-07-09.md` (seq 3, prior chain `listbox-widget`)
**Prior chain:** `listbox-widget` (ListBox seq 1 → Stepper seq 2 → Switch seq 3) — CLOSED. Seq 3's
"Where We're Going" §3 called it: the self-pick **widget** queue is exhausted, so recommend a
**non-widget breadth pivot** or pausing over another marginal widget. This session executed exactly
that pivot — the first link of a new `breadth-fov` chain.

---

## Related Handoffs

- `plans/handoffs/PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the
  READY-but-unfiled EW-007 pre-design (FloatingText bold/rich). Untouched this session; still the
  top candidate the moment the game files EW-007. Reference only.

## Since Last Handoff

Parent (listbox-widget seq 3) planned four next-actions; here is what happened to each:

- **§1 "Land THIS handoff (seq 3) → bump memory to seq 168"** → already done before this session
  opened (the paste-prompt confirmed PR #348 merged, memory at seq 168, `main @ 95fbf4b`). This
  session did NOT re-bump; it confirmed `git log` (95fbf4b/#348 + b2d8719/#347) and clean/ahead.
- **§2 "Read the board FIRST; serve EW-007 if filed"** → board read first thing; **still EMPTY**
  (EW-007 unfiled; EW-004/005/006 Verified+archived). Nothing to serve.
- **§3 "If board empty → ASK; widget queue exhausted, recommend non-widget pivot / pause"** → asked
  via `AskUserQuestion` (대기 / 비위젯 브레드스 피벗 / 한계적 위젯). User picked **비위젯 브레드스
  피벗**. Then asked a second question with 4 verified-absent candidates (FOV / procgen / RNG+loot /
  scene-transition-fx); user picked my recommendation, **Field of View**.
- **§4 "Hygiene: next trim ~seq 172"** → not due yet.

## Reference Documents

- `CLAUDE.md` — module map (now has the `FovMap` row) + header v1.6.213 / package v0.120.0.
- `.claude/skills/add-feature-example/SKILL.md` — the feature+example pass this followed.
- `docs/VISION.md` / `docs/NEXT_WORK.md` — breadth-first loop; the A–H playable-examples program is
  marked "done for v1.0.0", so a non-widget pivot needs a self-picked target (I proposed four).

## The Goal

Widen the engine's breadth per `docs/VISION.md` (a forkable, genre-agnostic 2D skeleton, each
feature proven by a small playable example). The widget suite is mature and its clean self-pick gaps
are used up, so this session pivoted to a **non-widget** gap: **grid field of view / fog-of-war**, a
genre-agnostic vision primitive (roguelike FOV, stealth sight lines, top-down fog-of-war) the engine
lacked entirely. Verified absent before starting (`lit_dungeon` does lighting, NOT sight occlusion).

## Where We Are

- **Shipped and landing.** PR **#349** (`feat/fov`) with async auto-merge armed (`--auto --squash`).
  Package **v0.120.0**, CLAUDE.md header **v1.6.213**. main tip → `9405b8d` on merge.
- `src/fov.rs` (NEW, ~290 lines incl. tests) — `FovMap` (a plain owned helper like `PathGrid` /
  `InputBuffer`, **NOT** an ECS component or system; the game keeps one and recomputes on move):
  - row-major `opaque` / `visible` / `revealed` grids.
  - **`compute(origin, radius)`** — recursive shadowcasting over 8 octants (RogueBasin multiplier
    table). An opaque cell is itself lit then shadows everything behind it; Euclidean
    `dx²+dy²≤radius²`. Clears `visible` each call, accumulates `revealed` (fog-of-war memory);
    `radius ≤ 0` lights only the origin.
  - **`from_path_grid`** — a `PathGrid`'s non-walkable cell → opaque, coords 1:1 (so a grid from
    `PathGrid::from_tilemap` carries straight over).
  - **`line_of_sight(a, b)`** — point-to-point Bresenham sight check, endpoints excluded (observer
    can see the wall it looks at); the companion primitive for stealth, independent of `compute`.
  - `new` caps cells at `MAX_PATH_GRID_CELLS` (reused from pathfinding) — over-cap/overflow → empty
    map + logged `error!`, mirroring `PathGrid`. Also `set_opaque` / `is_opaque` / `is_visible` /
    `is_revealed` / `clear_visible` / `reset`.
  - **9 unit tests + 2 doctests.**
- `src/lib.rs` — `pub mod fov;` + `pub use fov::FovMap;` (both in the alphabetical slot between
  `floating_text` and `gpu_particle`).
- `examples/fov.rs` (NEW) — playable top-down dungeon (ASCII map, defensively parsed): WASD/arrows
  move an observer one cell (walls block move + sight), `+`/`-` grow/shrink the radius, Esc quits.
  Cells in sight render bright / explored-but-unseen dim / never-seen black (a dark backdrop rect
  guarantees intentional black regardless of the clear colour); gems are drawn only when in sight
  (bright) or a dim ghost once discovered. HUD "gems discovered N/7 · radius R". `HEADLESS_SHOT`.
- `CLAUDE.md` module-map row + header; `docs/CHANGELOG.md` 0.120.0; `Cargo.toml`/`Cargo.lock` bumped.
- **Full verify gate green** (all 7 checks) — ran the full gate to completion 3× (once green after
  two gate trips, once post-fix, once post-ship) + the doc gate twice.
- Headless capture (`/tmp/fov.png`) eyeballed: the lit FOV forms an organic diamond radiating from
  the yellow observer, bounded by tan walls (occlusion correct), one cyan gem discovered (1/7), rest
  of the dungeon in black fog. Clean.
- **Additive** — a new module + one re-export; no existing API changed, no OS-gated code, no
  wasm-affecting example (native example, but the wasm gate is lib+bins only).

## What We Tried (Chronological)

1. **Onboarding** (paste-prompt): read the seq-3 handoff, confirmed `git log` (95fbf4b/#348 +
   b2d8719/#347), tree clean + up to date, memory seq 168 already current → **no re-bump**.
   `cargo test --lib switch` → 24 green (18 switch + 6 incidental). Read the key files (switch.rs,
   switch_pass.rs, focus_pass.rs) + adjacent (VISION.md, NEXT_WORK.md).
2. **Board empty → asked direction** → user chose non-widget pivot; **verified four candidate gaps
   absent** (`grep` for FOV/shadowcast, ldtk/tiled, RNG/loot, SceneTransition wipe/iris): FovMap
   absent (only dialogue `line_fully_revealed` + `FadeTransition` = solid-colour alpha fade only);
   procgen absent entirely; seedable RNG/loot absent (`rand 0.8` internal only). Asked a 4-option
   question → **Field of View** (my recommendation).
3. **Designed against the example first** (skill step 1): read `pathfinding.rs` (`PathGrid` gives
   `width`/`height`/`is_walkable` — clean `from_path_grid`), `debug_draw.rs`, and the ideal template
   `examples/diagonal_pathing.rs` (UiQueue+DrawRect grid, TextQueue HUD, camera at origin) +
   `ui_switch.rs` (the `save_screenshot_headless` idiom).
4. **Wrote `src/fov.rs`** — ported RogueBasin recursive shadowcasting; bundled the 4 octant
   multipliers into an `Octant` struct to stay under the too-many-args lint. `cargo test --lib fov`
   → **8/9 green**; the 9th (`degenerate_and_out_of_bounds_are_safe`) **failed** because
   `FovMap::new(i32::MAX, 2)` does NOT overflow 64-bit `usize` → it tried to allocate ~13 GB. Added
   a `sized` helper capping at `MAX_PATH_GRID_CELLS` (the same footgun `PathGrid` guards) → 9/9.
5. **Wrote `examples/fov.rs`** — hit a borrow conflict (held `InputState` while reaching
   `resource_mut::<ShouldQuit>`); refactored to gather input intents into locals, then act after the
   borrow drops. Fixed two 31-char map rows to 30. First headless shot showed the FOV working but
   the fog read as the (light) headless clear colour → added a dark backdrop rect so fog reads as
   intentional black. Re-shot → clean.
6. **CLAUDE.md module row** + verify. **Two gate trips**, both fixed:
   (a) `cargo fmt --check` reflow (the known trap — hand-wrapped `OCTANTS` / long `DrawRect` line /
   the `sized` closure) → `cargo fmt`.
   (b) `RUSTDOCFLAGS=-D warnings` **redundant-explicit-links**: `[MAX_PATH_GRID_CELLS](crate::…)`
   and `[PathGrid](crate::…)` are redundant because both are imported → dropped the explicit paths
   (kept `[InputBuffer](crate::input_buffer::InputBuffer)`, which is NOT imported, so its path is
   required). Full gate green after.
7. **`/ship`** 0.119.0 → **0.120.0** (Cargo.toml + `cargo update -p skeleton-engine` + CHANGELOG
   0.120.0 + CLAUDE.md header v1.6.212 → v1.6.213). Re-ran the full gate → green.
8. **`/land-pr` Async:** branch `feat/fov` → commit `af70611` → push → PR **#349** →
   `gh pr merge 349 --auto --squash` (armed; `mergeStateStatus: BLOCKED` = checks running).

## Key Decisions

- **Recursive shadowcasting (RogueBasin 8-octant), not raycasting or permissive FOV.** The classic,
  well-behaved algorithm — an opaque cell is lit (you see the wall) then shadows behind it. Ported
  faithfully with f32 slopes; the octant multipliers live in a const `[Octant; 8]` table.
- **A plain owned helper, NOT an ECS component/system.** Mirrors `PathGrid` / `InputBuffer`: the
  game keeps a `FovMap` in a system's state (or a resource) and calls `compute` on move. No editor
  wiring, no reflect/clone/serde registration, no system — a much smaller footprint than a widget.
- **`revealed` accumulates (fog-of-war memory); `visible` clears each `compute`.** The two-set model
  is the standard fog-of-war shape: currently-visible vs ever-explored. `reset` wipes both (new
  level).
- **`from_path_grid` = the integration seam.** Non-walkable → opaque, coords 1:1, so the same walls
  that block movement block sight and a `PathGrid::from_tilemap` grid carries over with no rework.
- **`line_of_sight` is a separate primitive.** Area FOV (shadowcasting) and point LOS (Bresenham)
  are distinct genre needs (roguelike reveal vs stealth "can the guard see me"); shipped both.
  Endpoints excluded so the observer sees the wall it looks at and its own opaque cell never blocks.
- **`MAX_PATH_GRID_CELLS` cap reused, not a new constant.** FOV maps and path grids are the same
  scale; my own degenerate test caught that `i32::MAX×2` doesn't overflow 64-bit usize → a ~13 GB
  alloc footgun. Capping at the existing pub const fixes it and keeps the two grid types consistent.
- **Euclidean radius (`dx²+dy²≤radius²`), `≤` not `<`.** Gives a rounded FOV where `(radius,0)` is
  lit — intuitive "sight radius N".
- **Example draws a dark backdrop rect** so never-seen cells read as intentional black regardless of
  the window/headless clear colour (the headless capture's clear is lighter than the windowed
  `clear_color`). Also: defensive ASCII-map parsing (missing/out-of-bounds cell → treated safely),
  so a ragged row can't panic.
- **Rejected (for now):** LDtk/Tiled import (bigger, adds a parse dependency), procedural map
  generation, and seedable-RNG+loot-table — all verified-absent and offered as candidates; FOV was
  the cleanest single-algorithm + one-example fit. The other three remain on the shelf.

## Evidence & Data

### Version / PR

| Item | Value |
|---|---|
| Package version | 0.119.0 → **0.120.0** (MINOR — new public API) |
| CLAUDE.md header | v1.6.212 → **v1.6.213** |
| PR | **#349**, async auto-merge armed (`--auto --squash`) |
| main tip | `9405b8d` (was `95fbf4b` #348) |
| Commit (pre-squash) | `af70611` on `feat/fov` |
| Memory global seq | **169** (code PR); this handoff PR will be seq **170** |
| Async landing | 16th unattended auto-merge |

### Tests (9 unit + 2 doctests, all green)

| File | Count | Coverage |
|---|---|---|
| `src/fov.rs` | 9 | open-field-within-radius, wall-shadow, radius-0-origin-only, revealed-accumulates-visible-clears+reset, from_path_grid→opaque, LOS-blocked-by-wall, LOS-ignores-endpoints, degenerate/overflow-safe, pillar-shadow-widens |
| doctests | 2 | module doc (wall shadow) + `from_path_grid` |

Full gate: `[verify] all checks passed ✓` (3 full runs). CI: 5/5 required checks (armed auto-merge).

### The feature+example signature (add-feature-example)

| File | What |
|---|---|
| `src/fov.rs` (new) | `FovMap` + shadowcasting + `line_of_sight` + `sized` cap + 9 tests |
| `src/lib.rs` | `pub mod fov;` + `pub use fov::FovMap;` |
| `examples/fov.rs` (new) | playable fog-of-war dungeon + `HEADLESS_SHOT` |
| `CLAUDE.md` | module-map row + header v1.6.213 |
| `docs/CHANGELOG.md` + `Cargo.toml` + `Cargo.lock` | 0.120.0 (via `/ship`) |

## Files Changed

### Source (new)
- `src/fov.rs` — `FovMap` + recursive shadowcasting + `line_of_sight` + 9 tests + 2 doctests.
- `examples/fov.rs` — playable top-down fog-of-war dungeon + headless self-check.

### Source (modified)
- `src/lib.rs` — `pub mod fov;` + `pub use fov::FovMap;`.

### Docs / release
- `CLAUDE.md` — module-map `FovMap` row + header v1.6.213 / v0.120.0.
- `docs/CHANGELOG.md` — 0.120.0 entry.
- `Cargo.toml`, `Cargo.lock` — 0.120.0.

### Memory (not in git)
- `engine-current-state.md` — seq-169 bump (code PR) + seq-170 bump (this handoff PR, on merge).

## User Feedback & Preferences

- Opened via a paste prompt (listbox-widget seq-3 continuation) that pre-empted the deferred
  wrap-up (already done) and told me to confirm state, read the board, and — if empty — ASK +
  recommend a non-widget pivot over another marginal widget. Narrated the 5-step onboarding in
  Korean, then waited for go-ahead.
- When the board was empty I asked twice (direction, then target). The user chose the pivot and then
  my recommended target (FOV) — consistent with the standing read that the user values continued
  visible breadth progress, as long as the pick is clean, verified-absent, and low-risk.
- Standing preferences (from memory): user-facing reports in **Korean**, agent-to-agent/code/docs in
  English; **merge authority delegated** (squash on green CI, no per-session re-confirm); **async
  auto-merge is the default landing** for CI-verifiable changes; always pass an explicit `model` to
  subagents.

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `breadth-fov` seq 1), async
   auto-merge. On merge, bump memory to **seq 170** pointing at the handoff merge hash. This is the
   recorded "deferred wrap-up done at seq-N start" cadence — next session's opening step.
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). If a
   request is filed (EW-007+), serve it priority-order. EW-007 (FloatingText bold/rich) has a READY
   pre-design note — serve same-day per that plan.
3. **If the board is still empty → ASK for direction.** The self-pick **widget** queue stays
   exhausted. **Non-widget breadth candidates still on the shelf** (all verified-absent this
   session, offered but not picked): **procedural map generation** (BSP / cellular-automata → a
   `Tilemap`; pairs with FOV for a roguelike slice), **seedable/deterministic RNG + `WeightedTable`
   loot** (small, foundational, makes procgen/FOV runs reproducible), and **scene-transition
   effects** (extend the existing solid-colour `FadeTransition` to wipe/iris + auto scene-swap
   orchestration). Recommend one of these (or pausing) over another marginal widget. A `FovMap`
   follow-up (a ready-made `FovSystem`/component wrapper, or directional/cone FOV for stealth) is
   possible if a game asks — but keep it a plain helper unless there's real demand.
4. **Hygiene:** tip line healthy. Next trim due ~**seq 172** (keep current chain + one prior). Use
   the proven Python surgical edit, not a hand-edit of the giant line.

## Risks & Blockers

- None for the shipped feature (additive, green, verified). CI (5 required checks) fully covers it.
- Minor, unhit: 2 audio tests can fail locally on a no-audio-device box (passed here; verify green).

## Open Questions

- None blocking. The one lean choice (plain helper, no `FovSystem`/component) is deliberate and
  documented; add a system wrapper only if a game asks.

## Quick Start for Next Session

```bash
# No beads — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 169/170), breadth-fov chain seq 1.

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 9405b8d (#349) or later
cargo test --lib fov                                          # 9 tests green (+ 2 doctests)

# See it run
HEADLESS_SHOT=/tmp/fov.png cargo run --example fov            # or: cargo run --example fov

# Key files (if extending FOV or adding another non-widget feature)
#   src/fov.rs                    — FovMap + recursive shadowcasting + line_of_sight
#   examples/fov.rs               — the playable fog-of-war acceptance test
#   src/pathfinding.rs            — PathGrid (the from_path_grid seam) + MAX_PATH_GRID_CELLS
#   .claude/skills/add-feature-example/SKILL.md — the feature+example pass

# Next action
#   Read ../dungeon-merchant/docs/engine-wishlist.md. If a request is filed, serve it priority-order
#   (EW-007 has a READY pre-design note). If STILL empty, ASK — the widget queue is exhausted;
#   recommend a non-widget target (procgen / seedable-RNG+loot / scene-transition-fx) or pausing.
```

## Session Closed

**Closed at:** 2026-07-09
**Session status:** Handed off — this handoff lands as its own `docs(handoff)` PR (chain
`breadth-fov` seq 1), async auto-merge. The memory **seq-170 bump** (updating `main @` to the
handoff merge hash) is the next session's opening wrap-up, per the recorded cadence. Code state at
close: `main @ 9405b8d`, v0.120.0, tree clean, all gates green.
