# Seedable `Rng` + `WeightedTable` (v0.123.0) and styled `SceneTransition` + auto scene-swap (v0.124.0)

**Date:** 2026-07-11
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `breadth-fov` seq `4` — **session-close** handoff covering the two features that shipped
*after* the seq-3 handoff (RNG seq 175 + scene-transition seq 176). Fifth & sixth features this chain.
**Parent:** `HANDOFF_breadth-fov_roguelike-capstone_2026-07-11.md` (seq 3, the procgen↔FOV capstone)
**Prior chain:** `listbox-widget` (widget suite; CLOSED)

> One handoff, **two** shipped features. The seq-3 handoff put a two-item shelf up for the next
> session; the user (over one continuous session) chose to burn the whole shelf — first the RNG, then
> the scene transitions, each with "병합 후 이어서 진행" (continue after merge). Both landed. **The shelf
> is now EXHAUSTED** (widget queue + all non-widget breadth candidates shipped), which is why this is
> a single session-close handoff rather than one per feature: the chain has reached a natural end and
> the next session has no pre-baked task.

---

## Related Handoffs

- `HANDOFF_breadth-fov_field-of-view_2026-07-09.md` — seq 1, `FovMap`.
- `HANDOFF_breadth-fov_procgen_2026-07-10.md` — seq 2, `generate_bsp_dungeon`.
- `HANDOFF_breadth-fov_roguelike-capstone_2026-07-11.md` — seq 3, `DungeonMap::to_path_grid` + the
  `roguelike` capstone. **Read it for the shared onboarding/standing state + the shelf this handoff
  drains;** this doc is deliberately concise.
- `PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the READY-but-unfiled EW-007
  pre-design (FloatingText bold/rich). Untouched. Reference only for when the board next fills.

## The Goal

Finish the non-widget breadth pivot (`docs/VISION.md`: breadth-first, each feature proven by a
playable example). The seq-3 handoff left exactly two shelf candidates. This session shipped both:

1. **Seedable deterministic `Rng` + `WeightedTable` loot** (v0.123.0, PR #355, seq 175) — a shared,
   public PRNG so a game reproduces a run/level/loot-stream from a stored seed; and a weighted
   selection primitive for loot/spawn tables.
2. **Styled `SceneTransition` + auto scene-swap** (v0.124.0, PR #356, seq 176) — the styled successor
   to the solid-colour `FadeTransition`: fade/wipe/iris that cover the screen, swap the scene *while
   hidden*, then reveal it, in one call.

## Where We Are

- **Both shipped and merged.** `main @ 7c78bc8`. Package **v0.124.0**, CLAUDE.md header **v1.6.217**.
  Tree clean, all gates green, memory bumped to **seq 176**.
- **Feature 1 — `src/rng.rs` (NEW).**
  - `engine::Rng` — SplitMix64: `new(seed)`, `next_u64`/`next_u32`, `range(lo,hi)` (i32 `[lo,hi)`,
    empty → `lo`), `f32_unit()` (`[0,1)`), `range_f32`, `bool()`, `chance(p)`, `pick(&[T])`,
    `shuffle(&mut [T])` (Fisher–Yates). A **golden-value stream test** pins SplitMix64(0)'s first
    three `u64`s so the constants can't silently drift (a drift would change every seed-derived
    artifact — dungeons, loot, everything).
  - `engine::WeightedTable<T>` — `new()`/`with(item,weight)`/`add` (f32 relative weights; `<=0` or
    non-finite ignored), `pick(&mut Rng)`/`pick_index(&mut Rng)`, `total_weight`/`len`/`is_empty`/
    `items`. The loot-drop / spawn-table primitive, driven by a caller-supplied `Rng` so table+seed
    reproduce the same sequence.
  - **`mapgen` adopted `Rng`** and dropped its own private duplicate `struct Rng` (+ its now-redundant
    RNG-internals test). **Byte-identical dungeons** (same constants + `range`/`bool`/`chance`
    semantics; mapgen's determinism/connectivity tests pass unchanged) — the whole point of sharing.
  - **11 unit + 2 doctests** in `rng.rs`. Example **`loot_table`** (weighted rarity drops + live
    histogram converging to target markers; Space=draw 1, A=draw 100, R=replay identical, N=new seed;
    `HEADLESS_SHOT` pre-rolls 200).
- **Feature 2 — `src/scene_transition.rs` + `src/renderer/transition.rs` (NEW).**
  - `engine::SceneTransition` — `coverage` runs 0→1 (cover) →0 (reveal) across `phase` Out/In/Done;
    `TransitionStyle` (`#[non_exhaustive]`: Fade / WipeLeft/Right/Down/Up / IrisIn / IrisOut);
    `new(style, half_duration)`/`with_color`/`update(dt)`/`just_covered`/`is_done`/
    `covered_at(x,y,aspect)` (a **CPU mirror of the shader geometry**, so logic/tests agree with pixels).
  - **Auto scene-swap, two entry points:** `App::transition_to_scene(scene, style, half_dur)` (from
    `&mut App`) and **`start_scene_transition(world, …)`** (the world-level twin, **callable from a
    system** — the key design need, since a system has no `&mut App`). App covers → swaps the scene at
    the fully-covered midpoint (internal `PendingSceneTransition(Option<SceneCmd>)` resource consumed
    in schedule step 11b via `apply_scene_cmd`) → reveals. `SceneTransition` is **auto-registered
    persistent** in the App ctor (its `TypeId` seeds `persistent_resources`) so the reveal survives the
    mid-transition `reload_scene` world reset; dropped when Done.
  - `TransitionRenderer` — native-only full-screen styled-coverage pass in the **fade slot**
    (`app/render/frame.rs`), all-`vec4` uniform (coverage/style/aspect/softness + rgba) to dodge the
    WGSL vec3-align trap; the shader mirrors `covered_at` with a smoothstep soft edge. **The swap fires
    on wasm too** (state advances); only the overlay is native-only.
  - **5 unit + 1 doctest** + an **integration test** `scene_transition_auto_swaps_at_cover_and_clears_when_done`
    that drives `App::update` **headlessly** (no GPU) to pin the orchestration: no-swap-before-cover →
    swap-at-cover → survives-reset → cleared-when-done. Example **`scene_transition`** (keys 1–7
    transition between four full-screen-colour levels, one per style; `HEADLESS_SHOT` freezes a
    mid-IrisIn still — content shows through the central circular window).
- **Both additive** — new modules/renderer/re-exports; `FadeTransition` and every existing API
  untouched. No breaking change (MINOR bumps are the pre-1.0 norm).

## What We Tried (Chronological)

### Feature 1 — RNG (PR #355)

1. **Designed the surface** around what `mapgen` already needed (SplitMix64 `range`/`bool`/`chance`)
   plus the loot use-case (`WeightedTable`, `pick`/`shuffle`, `f32_unit`). Kept it `rand`-free and
   thread-RNG-free so seeds fully determine the stream.
2. **Ported `mapgen`'s private SplitMix64 verbatim** into `Rng` so the shared type is bit-for-bit the
   old generator; deleted the private `struct Rng` + its redundant internals test. Confirmed
   byte-identical output via mapgen's unchanged determinism/connectivity tests.
3. **Golden-value stream test** — hard-codes SplitMix64(0)'s first three `u64`s
   (`16294208416658607535`, `7960286522194355700`, `487617019471545679`) so a future refactor that
   changes the constants fails loudly instead of silently reshuffling every seed-derived artifact.
4. **Wrote `examples/loot_table.rs`** (rarity histogram converging to markers, R=replay proves
   determinism). Built + headless.
5. **Verify.** **One gate trip:** clippy `useless_use_of_format` — `format!("{}", r.name)` on a
   `&'static str`. Fixed to pass `r.name` directly. Re-ran → green.
6. **/ship** 0.122.0 → **0.123.0** + **/land-pr Async** → PR **#355** → squash `ec55ab8`.

### Feature 2 — scene transitions (PR #356)

7. **First design used only `App::transition_to_scene`** (`&mut App`). Writing the example surfaced
   the awkward seam (VISION rule): **gameplay triggers a transition from a system**, which has no
   `&mut App`. Fixed by adding the **world-level `start_scene_transition(world, …)`** + an internal
   `PendingSceneTransition` resource; `App::transition_to_scene` now just delegates to it.
8. **The scene swap resets the world** (`reload_scene`), which would drop the mid-flight
   `SceneTransition` and freeze the screen covered. Fixed by **auto-registering `SceneTransition`
   persistent** in the App ctor so the reveal survives the reset. (`PendingSceneTransition` is consumed
   *before* the reset, so it needs no persistence.)
9. **Renderer** — all-`vec4` uniform ([[wgsl-vecn-uniform-alignment-trap]]); the fragment shader
   recomputes coverage per style, mirroring `covered_at` with a soft edge.
10. **Verify — three gate trips**, all caught locally:
    - (a) `covered_at` is **degenerate at the exact center/corner** for iris (IrisOut radius 0,
      IrisIn radius 1). The endpoint unit test sampled those exact points and flapped → rewrote the
      grid to **cell-centre samples** `(i+0.5)/10.0` (0.05..0.95, never 0.5/0/1).
    - (b) **wasm dead-code** — `TransitionStyle::shader_index` is only called from the native-gated
      renderer, so it's dead on wasm → `#[cfg(not(target_arch = "wasm32"))]` on the method.
    - (c) **2 broken intra-doc links** — `[`SceneTransition::with_color`]` needed the `crate::` path;
      `[`TransitionStyle::shader_index`]` is `pub(crate)` (unlinkable in public docs) → de-linked to
      plain text.
    - Plus a **headless framing fix** (not a gate): the headless capture renders at the engine default
      1280×720 (a `set_scene` swap drops `WindowConfig` before the headless read), but the example
      content was 660×420 → sized the example window to 1280×720 to match, then switched the headless
      still to IrisIn coverage 0.5 so the level shows through the central window.
11. **/ship** 0.123.0 → **0.124.0** + **/land-pr Async** → PR **#356** → squash `7c78bc8` (watched via
    background `gh pr checks --watch`).

## Key Decisions

- **`Rng` is a straight port of `mapgen`'s SplitMix64, not a new algorithm.** Sharing the *exact*
  generator makes the mapgen refactor provably behavior-preserving (byte-identical dungeons) and lets
  the determinism tests double as the port's proof. A golden test locks the constants going forward.
- **`WeightedTable::pick` takes a `&mut Rng` rather than owning one.** One RNG threads through a
  frame's rolls (loot + spawns + procgen) so a single seed reproduces the whole sequence; an owned
  per-table RNG would fragment the stream.
- **World-level `start_scene_transition` is the primary API; `App::transition_to_scene` delegates.**
  Gameplay lives in systems, and a system can't reach `&mut App`. The resource-based handshake
  (`PendingSceneTransition` consumed at cover) is the only way to swap the scene from inside the
  schedule. This was the one real design tension the example surfaced.
- **`SceneTransition` auto-persistent, `PendingSceneTransition` not.** The transition must outlive the
  world reset (it's mid-reveal); the pending command is consumed *before* the reset, so persisting it
  would be wrong (it'd re-fire). Consumed-then-reset ordering is load-bearing.
- **`covered_at` as a CPU mirror of the shader.** Lets the swap timing + endpoint behavior be unit-
  and integration-tested with no GPU (CI has none for logic tests), and keeps pixels honest against
  the logic. The degeneracy at iris endpoints is inherent (radius 0/1), handled by never sampling them.
- **One session-close handoff, not two.** Both features are the same shelf-draining arc, landed back-
  to-back in one session, and leave the chain at a clean end. Splitting would duplicate the standing
  state for no benefit.

## Evidence & Data

| Item | Value |
|---|---|
| Feature 1 version | 0.122.0 → **0.123.0** (MINOR — new public `Rng`/`WeightedTable`) |
| Feature 2 version | 0.123.0 → **0.124.0** (MINOR — new public `SceneTransition` + `start_scene_transition`) |
| CLAUDE.md header | v1.6.215 → v1.6.216 (#355) → **v1.6.217** (#356) |
| PRs | **#355** (`ec55ab8`), **#356** (`7c78bc8`) — both async auto-merge |
| main tip | `d644988` (#354) → `ec55ab8` (#355) → **`7c78bc8`** (#356) |
| Memory global seq | RNG code = seq **175**; scene-transition code = seq **176**; this handoff PR = seq **177** |
| Async landings | #355 = 22nd, #356 = 23rd unattended auto-merge |

### Tests

| File | Tests | Coverage |
|---|---|---|
| `src/rng.rs` | 11 unit + 2 doctests | golden stream stability; range/`f32_unit` bounds; `chance` extremes + rate; pick/shuffle permutation; weighted distribution ~9:1; deterministic replay |
| `src/mapgen.rs` | (unchanged) | determinism + connectivity still green after the `Rng` swap = byte-identical proof |
| `src/scene_transition.rs` | 5 unit + 1 doctest | phase progression; `just_covered`/`is_done` edges; `covered_at` per style at cell-centre samples |
| `src/app.rs` | 1 integration | `scene_transition_auto_swaps_at_cover_and_clears_when_done` drives `App::update` headlessly (swap timing + persistence + cleanup) |

Full gate green for both PRs (fmt / clippy `-D warnings` / wasm lib+bins build / wasm `--lib` clippy /
`test --all-targets` / `test --doc` / rustdoc `-D warnings`). CI 5/5 required on each.

## Files Changed

### Source
- `src/rng.rs` (NEW) — `Rng` + `WeightedTable` + 11 unit + 2 doctests.
- `src/mapgen.rs` (modified) — dropped private `struct Rng`, `use crate::rng::Rng;`; behavior-preserving.
- `src/scene_transition.rs` (NEW) — `SceneTransition`/`TransitionStyle`/`TransitionPhase`/
  `start_scene_transition`/`PendingSceneTransition` + 5 unit + 1 doctest.
- `src/renderer/transition.rs` (NEW) — `TransitionRenderer` (native-only overlay pass).
- `src/app/render/frame.rs` — transition pass in the fade slot (native cfg).
- `src/app/schedule.rs` — step 11b: advance the transition, swap at cover, drop when done.
- `src/app/scenes.rs` — `App::transition_to_scene` (delegates to the world-level fn).
- `src/app/render_state.rs` — `transition_renderer` field (native cfg).
- `src/app.rs` — seed `persistent_resources` with `SceneTransition`'s `TypeId` + the integration test.
- `src/lib.rs` — `pub mod rng;` + `pub use rng::{Rng, WeightedTable};`; `pub mod scene_transition;` +
  the `start_scene_transition`/`SceneTransition`/`TransitionPhase`/`TransitionStyle` re-exports.
- `src/renderer/mod.rs` — `pub mod transition;`.
- `examples/loot_table.rs` (NEW), `examples/scene_transition.rs` (NEW).

### Docs / release
- `CLAUDE.md` — rng row + mapgen adoption note (#355); SceneTransition row (#356); headers.
- `docs/CHANGELOG.md` — 0.123.0 + 0.124.0.
- `Cargo.toml`, `Cargo.lock` — 0.123.0 → 0.124.0.

### Memory (not in git)
- `engine-current-state.md` — seq 175 (#355) + seq 176 (#356) bumps + `main @ 7c78bc8` / v0.124.0 /
  v1.6.217; **SHELF EXHAUSTED** noted. `MEMORY.md` hook rewritten to match. This handoff PR = seq 177.

## User Feedback & Preferences

- **On an empty board the user wants a steer-with-recommendation, then drives.** Twice this session
  the answer to "what next?" was a terse pick ("1", "병합 후 이어서 진행") — the user treats a well-scoped
  shelf as a menu and expects the agent to execute each choice end-to-end (ship + land + memory)
  without re-confirming mid-arc.
- Standing (memory): user-facing reports **Korean**, agent-to-agent/code/docs **English**; **merge
  authority delegated** (squash on green CI, no re-confirm); **async auto-merge is default**; always
  pass explicit `model` to subagents; `cargo fmt` before verify; read gate exit codes **non-piped**
  (zsh `$pipestatus` is 1-indexed — never `${PIPESTATUS[0]}`).

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `breadth-fov` seq 4), async auto-merge.
   On merge, bump memory to **seq 177** pointing at the handoff merge hash. (No package bump — docs-only.)
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). Still empty
   at this session's close (next free ID **EW-007**, unfiled). If filed → serve priority-order (EW-007
   FloatingText bold/rich has a READY pre-design note).
3. **If the board is still empty → ASK.** **The shelf is now EXHAUSTED** — the widget self-pick queue
   AND all three non-widget breadth candidates (procgen↔FOV capstone / RNG / scene-transition) shipped.
   There is **no pre-baked next task**. Propose a NEW breadth area with a recommendation, e.g.:
   - **More procgen modes** — cave/cellular-automata caverns, multi-level dungeons with stairs
     (composes with `mapgen`/`FovMap`/`Rng` already on the shelf).
   - **Audio-driven gameplay hooks** — beat/onset events a game can gate spawns/effects on.
   - **A second capstone game** — a fuller playable slice dogfooding more subsystems at once.
   - **Tilemap streaming** — load/unload chunks for maps larger than one screen/allocation.
4. **No hygiene due.** The memory tip line sits at the correct `current chain + one prior session`
   boundary (breadth-fov 169–176 + listbox-widget 163–168; ≤162 archived). The trim that was due at
   seq 174 was done; **no trim needed at seq 177** — re-check only if the chain grows past ~seq 178
   without a session boundary.

## Risks & Blockers

- None for either shipped feature (additive, green, verified; CI's 5 checks fully cover both).
- The scene-transition **overlay is native-only** by design (wasm has no `TransitionRenderer` pass) —
  the *swap* still happens on wasm, but a web game gets an instant cut, not an animated wipe. Documented
  on `App::transition_to_scene`. If a web game needs the visual, that's a follow-up (port the pass to
  the wasm render path).

## Open Questions

- None blocking. The one real design choice (world-level trigger + persistent resource for the swap)
  is deliberate and documented; the wasm overlay gap is a known, documented limitation.

## Quick Start for Next Session

```bash
# No beads — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 175/176/177), breadth-fov chain seq 4 (session-close).

git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 7c78bc8 (#356) or the seq-177 handoff squash
cargo test --lib rng                                          # 11 unit green (+ 2 doctests)
cargo test --lib scene_transition                             # 5 unit green (+ 1 doctest)

# See the two examples run (drop HEADLESS_SHOT to play)
HEADLESS_SHOT=/tmp/loot.png cargo run --example loot_table
HEADLESS_SHOT=/tmp/transition.png cargo run --example scene_transition

# Key files
#   src/rng.rs                    — Rng (SplitMix64) + WeightedTable
#   src/scene_transition.rs       — SceneTransition + start_scene_transition (world-level trigger)
#   src/renderer/transition.rs    — the native-only overlay pass
#   ../dungeon-merchant/docs/engine-wishlist.md — the board (read FIRST)

# Next action
#   Read the board. If filed, serve priority-order. If STILL empty, ASK — the SHELF IS EXHAUSTED,
#   so propose a NEW area (more procgen modes / audio-driven hooks / 2nd capstone / tilemap streaming).
```

## Session Closed

**Closed at:** 2026-07-11
**Session status:** Handed off. Two features shipped and merged this session (RNG #355 v0.123.0 seq
175 + scene-transition #356 v0.124.0 seq 176); this handoff lands as its own `docs(handoff)` PR (chain
`breadth-fov` seq 4). Code state at close: **`main @ 7c78bc8`, v0.124.0, tree clean, all gates green,
memory at seq 176** (→ seq 177 on this handoff's merge). The `breadth-fov` chain has now shipped
**four** non-widget breadth features (FOV seq 1 · procgen seq 2 · procgen↔FOV capstone seq 3 · RNG +
scene-transition seq 4) — **and the self-pick shelf is EXHAUSTED**, so the next session must read the
board and, if empty, ASK for a new direction.
