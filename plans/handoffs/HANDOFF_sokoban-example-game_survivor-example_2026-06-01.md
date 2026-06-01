# Top-down twin-stick survivor example (candidate F) + `SteeringSystem` O(N²)→O(N) fix

**Date:** 2026-06-01
**Status:** COMPLETED (shipped + interactive play user-confirmed + perf win measured)
**Bead(s):** none (`bd` not installed in this repo)
**Epic:** Playable examples for v1.0.0 dogfooding (`docs/NEXT_WORK.md`)
**Chain:** `sokoban-example-game` seq `3`
**Parent:** `HANDOFF_sokoban-example-game_shooter-example_2026-06-01.md` (seq 2, shooter / candidate D)
**Prior chain:** `HANDOFF_sokoban-example-game_2026-05-31.md` (seq 1, Sokoban) > `..._shooter-example_...` (seq 2, shooter) > this (seq 3, survivor)

---

## Related Handoffs

Sibling streams in the same playable-examples program (reference only, not chain parents):

- `HANDOFF_platformer-example-game_2026-05-30.md` — candidate A.
- `HANDOFF_scene-flow-ui-interaction_2026-05-31.md` / `..._engine-ui-fixes_...` — candidate E.
- `HANDOFF_maze-escape-example-game_2026-05-31.md` — candidate B. The original `Sprite` +
  `CollisionGridSystem`/`SpatialGrid` + `CollisionLayer` AABB pattern this example (via the
  shooter) inherits.
- `PLAN_sokoban-example-game_2026-05-31.md` — the candidate-D plan from seq 2.

## Since Last Handoff

Parent (seq 2) "Where We're Going" said the breadth pass A–E was complete and named **one
optional depth item: candidate F (top-down twin-stick / bullet survival)**, with the open
question "Is candidate F wanted?". This session:

- **F is now done.** User chose to build it (over "land PRs first" / "verify rust-survivors").
- **The parent's predicted-but-uncommitted gap landed for real.** Parent + the candidate-F plan
  hypothesized `SteeringSystem` might be O(N²); this session *proved it with data* (3.67ms @200
  seekers) and fixed it. The "opportunistic, fix only if it bites" decision flipped to "fix" once
  the data contradicted the "won't surface at this scale" prediction.
- **Plan assumption corrected:** the plan claimed `GpuParticleEmitter` needed no cfg-gating
  (wrong — the whole `gpu_particle` module is wasm-gated like `AudioManager`).
- Parent's open question "PR landing strategy (PR #1/#2 bundles)" is **still open** — untouched.
- Parent's "rust-survivors not yet built against the new API" risk is now **closed**: built clean
  against this session's engine change too.

## Reference Documents

- `docs/VISION.md` — the feature+example dogfooding loop ("a feature is not done until a small
  example exercises it in real play"; "fix the API before release if it feels awkward").
- `docs/NEXT_WORK.md` — candidate table; updated this session (F → Done, breadth+depth complete).
- `docs/HANDOFF.md` — per-phase dev history; candidate-F entry prepended this session.
- `docs/PATTERNS.md` — architecture patterns (borrow "collect then act", render-layer separation).
- `CLAUDE.md` — module map; steering row added this session.
- Plan file: `/Users/jkl/.claude/plans/read-plans-handoffs-handoff-sokoban-exam-silly-perlis.md`
  (the approved candidate-F plan, updated mid-session with grill-me decisions).

## The Goal

Ship candidate **F (top-down twin-stick survival)** under the dogfooding loop — the one optional
*depth* item after the A–E breadth pass. Where D (shooter) first stressed `Pool`/`SpatialGrid`/
`Timer`, F stresses **steering under many simultaneous entities** (`Seek` + `SteeringSystem`), the
**native-only GPU particle path** (`GpuParticleEmitter`) the CPU `ParticleSystem` never touches,
and **`ProfilerData` perf visibility**. It complements `rust-survivors` (a survivors-like) directly.
End state: `cargo run --example survivor_game` is a real playable game, and any engine friction it
surfaces is fixed before F is called done. It surfaced a genuine O(N²) in `SteeringSystem`, now
fixed and measured.

## Where We Are

- **DONE and committed.** Single commit this session: `e991ca0` on branch `feat/shooter-example`.
  Working tree clean. **Not pushed; no PR** (per the "no push/PR unless asked" rule).
- **New example:** `examples/games/survivor/survivor.rs` (~874 lines incl. blanks/comments).
- **Cargo entry:** `Cargo.toml` `[[example]] name = "survivor_game"` /
  `path = "examples/games/survivor/survivor.rs"` (mirrors the shooter entry).
- **Engine change:** `src/steering.rs` — `SteeringSystem::run` rewritten from per-entity
  `query().find()` self-scans to O(1) `world.get`/`get_mut(entity)`; +1 regression test
  (`many_seekers_each_advance_toward_shared_target`). Behavior-identical.
- **Tests:** `cargo test --lib` → **248 passed** (was 247 at session start; +1 steering test).
  `cargo test --lib steering` → 5 passed.
- **Lint/format:** `cargo clippy --lib --example survivor_game` = **0 warnings**; `cargo fmt
  --check` clean.
- **wasm:** `cargo build --target wasm32-unknown-unknown` for both `--lib` and
  `--example survivor_game` = ok, 0 warnings (after target-gating the GPU thruster).
- **Sibling:** `cargo build` in `/Users/jkl/Projects/rust-survivors` = clean (no breakage from the
  steering change).
- **Interactive play user-confirmed:** "1-5까지 테스트 이상없음" (move/chase/shoot-kill-explode/
  death-gameover/restart all working).
- **Perf measured in real play:** `steer` (SteeringSystem avg from `ProfilerData`) **3.67ms@200
  before** the fix → **0.48ms@200, 1.36ms@600 after** (linear = O(N) confirmed). `frame` held
  15–17ms (vsync cap) throughout, including 600 enemies (3× the design target).
- **Systems (in order):** `CollisionGridSystem::new(64.0)` → `PlayerSystem` (move/aim/fire/debug
  keys) → `BulletSystem` → `SpawnSystem` → `SeekSystem` (retarget) → `SteeringSystem` (engine;
  moves enemies) → `ThrusterSystem` → `CollisionSystem` → `ParticleSystem` → `HudSystem`.
- **Docs updated:** `docs/NEXT_WORK.md`, `docs/HANDOFF.md`, `CLAUDE.md`.
- **Sonnet review:** one read-only correctness pass; no real defects; one latent tidy applied.

## What We Tried (Chronological)

1. **Onboarding + baseline.** Read parent handoff + `docs/NEXT_WORK.md` + `docs/VISION.md`,
   confirmed the candidate-F building blocks exist and are re-exported (`engine::{Seek, Flee,
   Arrive, Wander, SteeringSystem, SteeringVelocity}`, `engine::GpuParticleEmitter`,
   `ProfilerData`). Verified green base: `cargo test --lib` = 247.
2. **Plan-mode planning + grill-me.** Wrote a plan; user ran `/grill-me`. Locked 3 decisions:
   (1) keyboard twin-stick shooter [not mouse-aim / not auto-fire / not bullet-hell dodge],
   (2) engine changes opportunistic-only, (3) perf bar moderate (~100-200 enemies) + thruster-only
   GPU. Updated the plan to match, then `ExitPlanMode` approved.
3. **Wrote the example** modeled wholesale on `examples/games/shooter/shooter.rs`. Native build
   passed first try; clippy clean; fmt applied.
4. **wasm build FAILED** — `error[E0432]: unresolved import engine::GpuParticleEmitter`. Discovered
   the plan's claim was wrong: `src/lib.rs:14` `#[cfg(not(wasm32))] pub mod gpu_particle;` and
   `:74` gate the re-export — the *whole module* is native-only, exactly like `AudioManager`.
   Fixed by target-gating the thruster (import, emitter creation, and `update_thruster_emit`
   native fn + wasm no-op stub). wasm then green.
5. **Startup smoke run** (background, ~8s) → no panic, exit on SIGTERM. Window opened.
6. **Sonnet read-only review subagent** (per user preference). Verdict: pool lifecycle, claimed/
   spent dedup, borrow patterns, thruster cfg-gating, spawn arithmetic, system order all clean.
   One [BUG] flagged (spawn_timer ticked during GameOver — harmless, matches shooter precedent)
   and one [RISK] that was actually already guarded by the `claimed` filter. Applied the tidy:
   `SpawnSystem` now Playing-gates the timer tick (consistent with elapsed/fire_timer).
7. **First interactive test (user):** 1–5 fine, but **#6 impossible** — single-life + seekers means
   you die before ~100 enemies accumulate, so the perf target can't be reached in normal play.
   This is a real example-design friction (not an engine gap).
8. **Added debug perf affordances:** `G` toggles invulnerability (god mode), `B` spawns +25 (later
   +50) enemies, HUD shows `[GOD]` + a debug hint line. Lets the user reach the perf scale on
   demand without changing the locked single-life design.
9. **Perf observation round 1:** user reported `frame` 15–16ms steady at 200+. **Did NOT conclude**
   — checked the engine: `prof.frame_ms = dt*1000` and present mode is `AutoVsync`
   (`src/renderer/context.rs:126`), so `frame_ms` *saturates at the ~16.7ms vsync cap* and can't
   reveal CPU headroom.
10. **Added per-system `steer` readout** to the HUD from `ProfilerData.systems` (the
    `SteeringSystem` `avg_us`) — the honest "does steering bite?" number.
11. **Perf observation round 2:** user reported **`steer` 3.67ms @200**. This is ~30× the expected
    cost (simple seek math for 200 entities should be tens of µs), confirming the O(N²) self-lookup
    *does* bite. My earlier "won't surface at this scale" prediction was wrong.
12. **Asked the user** (decision was theirs given the locked opportunistic rule vs new data). User
    chose **"fix now."**
13. **Fixed `SteeringSystem`** — replaced all 4 `query().find(|(e,_)| *e==entity)` scan sites
    (Seek, Flee, Arrive reads + the SteeringVelocity transform-apply pass) with
    `world.get`/`get_mut(entity)`. Wander already used `get_mut`. Added a 64-seeker regression test.
14. **Verified the fix:** lib tests 248 pass, clippy 0, fmt clean, wasm green, **rust-survivors
    builds clean**. User re-measured: **`steer` 0.48ms @200** (7.6× faster).
15. **500+ stress test** (user asked). Split `NATURAL_CAP=200` (natural spawn) from `MAX_ENEMIES=600`
    (debug `B` hard cap) so balance is unchanged but `B` can push to 600. User measured **`steer`
    1.36ms @600, frame 16-17ms** — 2.83× time for 3× entities = linear = O(N) confirmed at scale.
16. **Docs + commit.** Updated 3 docs, committed `e991ca0`, confirmed all 6 files in the commit and
    tree clean.

## Key Decisions

- **Twin-stick shooter, keyboard-only** (WASD move + Arrow-key 8-way *aimed* fire). Rejected:
  mouse-aim (camera/coords complexity), auto-fire-at-nearest (less "twin-stick"), bullet-hell dodge
  (furthest from the shooter precedent). Firing requires holding an arrow (aim = trigger).
- **Single life + spawn-grace** (1s), endless, score = survival time + kills. Rejected the shooter's
  3-lives — survival framing fits "how long do you last".
- **Engine fix = data-driven, not pre-committed.** Locked rule was "opportunistic"; only flipped to
  "fix" after `steer 3.67ms@200` data contradicted the prediction. The user made the final call.
- **The fix is additive + behavior-identical** (`query().find` → `world.get`), with a regression
  test, so `rust-survivors` is unaffected (verified). No public API change.
- **`steer` (per-system ProfilerData), not `frame_ms`, is the perf signal** — `frame_ms` is
  vsync-capped (`AutoVsync`) and saturates ~16.7ms. Surfacing the per-system avg in the HUD is
  itself good `ProfilerData` dogfooding.
- **Debug `G`/`B` keys** added so the perf scale is reachable despite single-life play. Kept the
  locked single-life design; the keys are an observation aid, not a balance change.
- **`NATURAL_CAP=200` split from `MAX_ENEMIES=600`** so natural play feel is unchanged while `B` has
  stress headroom.
- **GPU thruster target-gated** (not the plan's "no gating") because `gpu_particle` is wasm-absent.
- **God mode persists across restart** (debug toggle, not run state) — no reset in `restart_game`.

## Evidence & Data

### Perf: SteeringSystem `steer` avg (from `ProfilerData.systems`)

| Enemies | Before fix (O(N²)) | After fix (O(N)) | Notes |
|---|---|---|---|
| 200 | **3.67 ms** | **0.48 ms** | 7.6× faster; before = ~30× expected cost |
| 600 | (~33 ms projected; never run) | **1.36 ms** | 2.83× time for 3× entities = linear |
| `frame` (all) | 15–16 ms | 16–17 ms | vsync-capped (`AutoVsync` ≈16.7ms); 60fps held even @600 |

Scaling check: O(N²) would give 9× from 200→600 (≈4.3ms from 0.48 base, or ≈33ms from the 3.67
base); observed 2.83× (0.48→1.36) confirms O(N).

### Verification commands (all green)

- `cargo build --example survivor_game` → ok.
- `cargo clippy --lib --example survivor_game` → 0 warnings.
- `cargo fmt --check` → clean.
- `cargo test --lib` → **248 passed** (247 + 1 steering test). `cargo test --lib steering` → 5.
- `cargo build --target wasm32-unknown-unknown --lib` and `--example survivor_game` → ok, 0 warn.
- `cd /Users/jkl/Projects/rust-survivors && cargo build` → clean.
- Startup smoke run → alive ~8s, no panic.
- Interactive play → user-confirmed (1–5 working).

### grill-me locked decisions (provenance for the build)

| Question | Options offered | Chosen | Consequence |
|---|---|---|---|
| Core loop | twin-stick shooter / auto-fire survivors / bullet-hell dodge | **twin-stick shooter** | keyboard-only, Arrow-aim = trigger |
| Engine scope | opportunistic / commit steering fix / example-only | **opportunistic** | flipped to "fix" once data showed the gap |
| Perf bar | moderate+thruster / heavy stress+GPU / gameplay-first | **moderate + thruster** | ~100-200 target, GPU = thruster only |

(The "opportunistic" choice + the locked "won't surface at moderate scale" prediction were
*overturned by measurement* — `steer 3.67ms@200` — and the user then chose to fix.)

### Spawn ramp + batch (SpawnSystem)

- `spawn_interval(elapsed) = (0.55 - elapsed*0.006).max(0.18)` — cadence shrinks over the run.
- `batch = (3 + elapsed_secs/12).min(8).min(NATURAL_CAP - alive)` — grows slowly, capped.
- `deactivate_bullet` strips `Sprite`/`Collider`/`CollisionLayer`/`Velocity`/`Bullet` so a pooled
  bullet leaves both the renderer and the collision grid; `fire_bullet` re-adds them on acquire.

### Survivor design constants (for tuning / regression)

- Window `820×820`, clear `[0.03,0.04,0.07,1.0]`, `Camera::new(Vec2::ZERO,1.0)` (world==screen).
- Player: `PLAYER_SPEED 300`, `PLAYER_HALF 16`, center spawn, clamped to arena.
- Bullets: `BULLET_SPEED 620`, `BULLET_HALF 5`, `BULLET_POOL_CAP 96`, `FIRE_COOLDOWN 0.14`,
  `BULLET_MARGIN 24` (release when off-arena by margin), fired along the 8-way aim dir.
- Enemies: `ENEMY_HALF 14`, speed `90..=140`, spawn from arena edges (`edge_spawn`).
- Caps: `NATURAL_CAP 200` (SpawnSystem), `MAX_ENEMIES 600` (debug `B` hard cap).
- `START_GRACE 1.0` spawn-grace invuln. Single life (contact → GameOver).
- Layers (disjoint bits): `LAYER_PLAYER 1<<0`, `LAYER_ENEMY 1<<1`, `LAYER_BULLET 1<<2`.
  `CollisionGridSystem::new(64.0)`.
- Thruster GPU emitter: `spawn_rate 140`, `lifetime 0.5`, `velocity_spread 36`, blue colors,
  `emit` toggled by movement.
- Explosion: `ParticleEmitter::for_burst()` + `ParticleBurst{remaining:16}`, lifetime 0.4,
  spread 180.

### Controls

Move WASD · Aim+Fire Arrow keys (hold = repeat) · `R` restart · `Esc` quit ·
**Perf-debug:** `G` toggle invulnerability · `B` spawn +50 enemies (up to `MAX_ENEMIES`).

## Code Analysis

- **`SteeringSystem` (`src/steering.rs`) before:** each of its 5 phases (Seek, Flee, Arrive, Wander,
  transform-apply) collected entity IDs, then per entity did `world.query::<T>().find(|(e,_)| *e ==
  entity)` — a full component-set scan per lookup → O(N²) over N steering entities. **After:** direct
  `world.get::<T>(entity)` / `world.get_mut::<T>(entity)` O(1) access. Wander already used `get_mut`.
- **`ProfilerData` (`src/resources.rs:407`):** `{ systems: Vec<SystemProfile>, render, frame_ms }`.
  `frame_ms = dt*1000` (set in `app.rs` per frame). `SystemProfile { name, last_us, avg_us }`;
  `avg_us` is a 60-frame EMA. The HUD reads `systems.iter().find(|s| s.name=="SteeringSystem").avg_us`.
- **Present mode `AutoVsync`** (`src/renderer/context.rs:126`) → `frame_ms` saturates at the refresh
  interval (~16.7ms @60Hz); it cannot show CPU headroom, only frame *drops* (when CPU > budget).
- **GPU particles native-only:** `src/lib.rs:14` `#[cfg(not(wasm32))] pub mod gpu_particle;` and
  `:74` gate the re-export. The renderer-side collection/upload in `app.rs:~2410` is also
  `#[cfg(not(wasm32))]`. So the *component itself* is wasm-absent — exactly like `AudioManager`.
- **`Pool` (`src/pool.rs`):** `acquire(world, setup)` reuses/ spawns + runs setup; `release(e,world)`
  adds `Pooled` marker + queues (despawns on overflow past capacity). It does NOT strip other
  components — the example's `deactivate_bullet` does that so released bullets leave renderer + grid.
- **Borrow pattern throughout:** "collect entity IDs, drop the query borrow, then `get_mut`/despawn/
  `insert_resource`" (the `docs/PATTERNS.md` workaround). `Pool` is `remove_resource`'d then
  re-`insert_resource`'d within each system that touches it.

## Files Changed (commit `e991ca0`)

### Source code
- `examples/games/survivor/survivor.rs` — NEW (~874 lines). The survivor example.
- `src/steering.rs` — `SteeringSystem` O(N²)→O(N) lookup fix (4 scan sites) + 1 regression test.
- `Cargo.toml` — `[[example]] survivor_game` entry.

### Docs
- `docs/NEXT_WORK.md` — F moved backlog→Done with gap+numbers; "Breadth + depth pass complete".
- `docs/HANDOFF.md` — candidate-F dated entry prepended; stale "Current status" line updated
  (A–F shipped, 248 tests).
- `CLAUDE.md` — added a `src/steering.rs` module-map row (notes the O(1) per-entity lookup).

## User Feedback & Preferences

- **Built F over alternatives** ("Build candidate F" chosen vs "Land PRs first" / "Verify
  rust-survivors").
- **Ran `/grill-me`** to pin scope before building — wanted the core-loop ambiguity, engine-scope,
  and perf-bar resolved first. Mid-grill asked the questions **in Korean** ("한글로 다시 물어봐").
- **Wanted to verify interactively themselves** — reported "1-5까지 테스트 이상없음", and that #6
  couldn't be tested because they game-over before 100 enemies.
- **Proactively asked for the 500+ stress test** ("500 마리 이상 부하테스트도 해볼까?").
- **Made the engine-fix call** when presented with the data (chose "지금 수정").
- **Korean is the working language;** docs stay English per `doc-language-rule`. Subagents on
  **Sonnet**, used aggressively (carried from parent preference; honored with the review pass).
- Commit style: `Co-Authored-By: Claude Opus 4.8`. Do not push / open PR unless asked.

## Where We're Going

The playable-examples program (A–F) is now **complete** per `docs/NEXT_WORK.md` — breadth + the one
depth item. No further planned candidate. Possible next actions, none committed:

1. **PR / landing strategy (carried open from seq 2).** `e991ca0` sits on `feat/shooter-example`,
   unpushed. PRs #1/#2 already target `main` and bundle the whole unmerged stack (english-conversion
   + A–E + now F). Decide merge order; push + extend a PR if wanted.
2. **Optional deeper perf pass** (only if desired): at 600 enemies the *new* dominant frame cost is
   render (600 sprites) + collision grid, not steering. Could profile those next — but the program
   goal is met; this is beyond F.
3. **Optional polish on survivor** feel/balance (enemy speed, spawn ramp) — gameplay was confirmed
   fine, so not required.

## Risks & Blockers

- **PR #1/#2 are large overlapping bundles against `main`** — review/merge-order burden (unchanged
  from seq 2; not blocking).
- **No `bd`** — tracking is the plan file + TaskCreate, not beads.
- Tool-degradation incident from seq 2 did **not** recur this session.

## Open Questions

- **PR landing order** (carried, still open) — does the `docs/english-conversion` stack merge to
  `main` first, then these rebase, or keep bundling?
- **Is the examples program truly closed for v1.0.0?** F was the last named candidate; confirm no
  further depth items are wanted.

## Quick Start for Next Session

```bash
# This work is DONE in commit e991ca0 (branch feat/shooter-example, unpushed).
cat plans/handoffs/HANDOFF_sokoban-example-game_survivor-example_2026-06-01.md   # this file

# Play the shipped example (perf-debug: G invuln, B +50 enemies):
cargo run --example survivor_game     # WASD move, Arrows aim+fire, R restart, Esc quit

# Key files to read first:
#   examples/games/survivor/survivor.rs   — the example
#   src/steering.rs                       — the O(N) fix + regression test
#   docs/NEXT_WORK.md                     — program status (A–F done)

# Verify current state:
cargo test --lib && cargo clippy --lib --example survivor_game   # expect 248 tests, 0 warnings
cargo build --target wasm32-unknown-unknown --example survivor_game

# Next action (pick one — none auto-committed):
#   (a) Decide PR/landing strategy for the unmerged stack, then push + PR if wanted, OR
#   (b) Confirm the examples program is closed for v1.0.0 and move to a new work stream.
```

## Session Closed
**Closed at:** 2026-06-01
**Commit:** feature work in `e991ca0`; this handoff committed as the session commit below.
**Session status:** Handed off to next session
