# Vertical shooter playable example + one-shot `ParticleBurst` (candidate D)

**Date:** 2026-06-01
**Status:** COMPLETED (shipped + interactive play confirmed)
**Bead(s):** none (`bd` not installed in this repo)
**Epic:** Playable examples for v1.0.0 dogfooding (`docs/NEXT_WORK.md`)
**Chain:** `sokoban-example-game` seq `2`
**Parent:** `HANDOFF_sokoban-example-game_2026-05-31.md` (seq 1, Sokoban) + paired `PLAN_sokoban-example-game_2026-05-31.md` (the candidate-D plan executed this session)
**Prior chain:** seq 1 shipped Sokoban (candidate C) + `History<T>`; its plan pointed at candidate D as the next work.

---

## Related Handoffs

Sibling work streams in the same playable-examples program (independent, reference only — not chain parents):

- `HANDOFF_platformer-example-game_2026-05-30.md` — candidate A.
- `HANDOFF_scene-flow-ui-interaction_2026-05-31.md` / `..._engine-ui-fixes_...` — candidate E.
- `HANDOFF_maze-escape-example-game_2026-05-31.md` — candidate B. **Closest code precedent** for the shooter: same persistent-`Sprite` + `CollisionGridSystem`/`SpatialGrid` + `CollisionLayer` AABB pattern the shooter copied.
- `HANDOFF_sokoban-example-game_2026-05-31.md` — **direct parent** (seq 1).

## Reference Documents

- `PLAN_sokoban-example-game_2026-05-31.md` — the 4-phase plan executed this session (the candidate-D plan).
- `CLAUDE.md` — engine module map; updated this session (particle row).
- `docs/VISION.md` — the feature+example dogfooding loop ("a feature is not done until a small example exercises it in real play").
- `docs/NEXT_WORK.md` — candidate table; updated this session (D done, A–E breadth complete, F remaining).
- `docs/PATTERNS.md` — architecture patterns (borrow workaround, render-layer separation, system order).
- `docs/HANDOFF.md` — per-phase dev history; shooter session entry appended this session.

## The Goal

Ship candidate **D (simple vertical shooter)** under the dogfooding loop: a small playable shooter that for the first time stresses `engine::Pool` (bullet churn), `Timer`-driven spawning (fire cooldown + enemy waves), `SpatialGrid`/`CollisionLayer` under many simultaneous entities, and the audio bus mixer — together, the combination most likely to surface real friction. Close whatever engine gap surfaces (per the maze/sokoban precedent) by patching the engine before release. End state: `cargo run --example shooter_game` is an actual playable shooter, and a fork user gets a one-shot particle burst from the engine without hand-rolling it.

The known predicted gap (from the parent handoff's pre-gathered API inventory): `ParticleEmitter` is **continuous-only** (`spawn_rate`/`emit`), so hit/explosion *bursts* have no clean API.

## Where We Are

- **DONE and shipped.** Branch `feat/shooter-example` (based on the then-current `feat/sokoban-example` tip `48abab6`). Three commits this session:
  - `70dd796 feat: add vertical shooter playable example + one-shot ParticleBurst` — the main work (7 files: example, particle engine change, lib re-export, Cargo entry, 3 docs). Note: docs were **re-applied and amended into this commit** after they were silently lost during a tool-degradation window (see What We Tried #2).
  - `7b3bd75 fix(shooter): prevent double-release when a bullet overlaps two enemies` — the one real bug a subagent review caught.
- **PR #2 OPEN against `main`:** https://github.com/ChunSam/skeleton-engine/pull/2 — user explicitly chose `main` (same call as Sokoban PR #1), so the diff vs main bundles the whole unmerged stack (english-conversion + A–D). 13 commits ahead of main.
- Working tree clean. All commits pushed to `origin/feat/shooter-example`.
- **Interactive play CONFIRMED by the user this session** ("게임은 테스트 정상적으로 진행됨"). This is the one item Sokoban/maze left unverified — for the shooter it is now verified.

## Engine change (the gap closed)

**File:** `src/particle.rs` (+ `src/lib.rs` re-export).

- **Before:** `ParticleEmitter` only emitted continuously (`spawn_rate` per sec gated by `emit: bool`). No way to say "emit N now."
- **After (purely additive — no field added to `ParticleEmitter`, full back-compat):**
  - `ParticleEmitter::for_burst()` — constructor preset: `emit=false`, `spawn_rate=0.0`, short `lifetime` (0.5), radial `velocity_spread` (160), warm explosion colors. Because `emit=false`/`spawn_rate=0`, the continuous path (step 2 of `ParticleSystem`) skips it.
  - `ParticleBurst { remaining: u32 }` — new component. Attach alongside `ParticleEmitter` on a **dedicated one-shot entity**.
  - `ParticleSystem` step 3 (new): for each `(Transform, ParticleEmitter)` that also has `ParticleBurst`, emit `remaining` radial particles (random angle over `TAU`, speed `0.2..=1.0 × velocity_spread.max_element()`), then **`despawn` that emitter entity**. Continuous emitters (no `ParticleBurst`) are never despawned.
  - Shared private helper `spawn_particle(...)` factored out so continuous + burst paths build particles identically.
  - Re-exported as `engine::ParticleBurst` (`src/lib.rs:82`: `pub use particle::{Particle, ParticleBurst, ParticleEmitter, ParticleSystem};`).
- **Type aliases added** to satisfy `clippy::type_complexity`: `EmitterSnapshot` (existing) and new `BurstSnapshot` (the 8-tuple the burst query collects). Without the alias clippy flagged the inline `Vec<(Entity, Vec2, f32, Vec2, [f32;4], [f32;4], Vec2, Option<String>)>`.
- **Tests (2 new, in `src/particle.rs`):**
  - `burst_emits_count_then_retires_emitter`: spawn emitter+`ParticleBurst{remaining:8}`, run one tick → exactly 8 `Particle` entities exist AND the emitter entity is `!is_alive` (retired).
  - `continuous_emitter_unaffected_by_burst_path`: continuous emitter (`emit=true`, high `spawn_rate`, no `ParticleBurst`) → ≥1 particle AND emitter still `is_alive` (not despawned).

## What We Tried (Chronological)

1. **Executed the 4-phase plan in-session (not phase-by-phase commits).** Read the plan + parent handoff + the 5 Quick-Start source files (`src/particle.rs`, `src/pool.rs`, `src/timer.rs`, `examples/games/maze_escape/maze_escape.rs`, `src/audio.rs`), confirmed the base was green (245 lib tests at start), then wrote the engine change + the full example, then verified. Tasks tracked via TaskCreate (4 phase tasks), all completed.
2. **TOOL-DEGRADATION INCIDENT (important).** Mid-session the Bash/Read tools entered a window where calls returned **stale or fabricated-looking output** and several Edit/Write calls **reported success but did not persist** (notably the three doc edits to `NEXT_WORK.md`, `HANDOFF.md`, `CLAUDE.md`). I over-scheduled wakeups during this. **Recovery:** once tools stabilized I re-verified EVERYTHING against real command output from scratch. This is how two problems were caught: (a) the doc edits had never landed — `git diff` showed the docs unmodified and the first commit had only 4 files; (b) the bullet double-release bug. Lesson for next session: **after any tool flakiness, re-run `git diff --stat` / `git show --stat HEAD` to confirm what actually persisted before trusting prior "success" results.**
3. **Subagent correctness review (Sonnet, per user request "서브에이전트 적극 사용 … 모델은 sonnet").** A read-only review agent found **one real bug**: `CollisionSystem` deduplicated enemies (a `claimed` HashSet so one enemy can't die twice in a frame) but **not bullets** — a single bullet whose AABB overlapped two adjacent enemies got pushed into `bullet_kills` twice and thus `deactivate_bullet`+`pool.release` ran on it twice (a double-release into the `Pool`). Other findings were NITs/low-risk patterns; the burst logic reviewed clean.
4. **Fixed the double-release** by adding a `spent: HashSet<Entity>` for bullets alongside `claimed` for enemies; a bullet already in `spent` is skipped (`examples/games/shooter/shooter.rs` CollisionSystem). Committed `7b3bd75`.
5. **Re-applied the lost docs** via a second Sonnet subagent (it adapted to the actual file structures, which differed from my assumed headings), then staged + `git commit --amend --no-edit` into `70dd796` and `--force-with-lease` pushed. Confirmed HEAD now has all 7 files.
6. **Interactive play.** Ran `cargo run --example shooter_game`; user confirmed it tested fine.

## Key Decisions

- **Persistent ECS `Sprite` entities, NOT Sokoban's immediate-mode `DebugDrawQueue`.** A particle-heavy action game has many moving sprites; the maze pattern (real `Transform`+`Sprite` entities, `CollisionGridSystem` rebuilding `SpatialGrid` each frame) fits. This was a deliberate divergence from the C precedent, flagged in the plan's Anti-Goals.
- **Route bullets through `engine::Pool` (don't hand-roll spawn/despawn).** The whole point of D was to exercise pooling. `Pool` has no accessor that yields `&mut Pool` while you also need `&mut World`, so each system that fires/releases does `remove_resource::<Pool>().unwrap()` → use → `insert_resource(pool)`. **Released bullets are fully deactivated** — `deactivate_bullet` strips `Sprite`/`Collider`/`CollisionLayer`/`Velocity`/`Bullet` so a pooled bullet vanishes from BOTH the renderer (no Sprite) and the collision grid (no Collider); reacquire re-adds them in the `acquire` setup closure. **No new pooling API was needed** — recorded as "surfaced-but-not-a-gap."
- **Additive particle API, no `ParticleEmitter` field change.** Keeps `rust-survivors` and the existing `examples/particle/particle_demo.rs` literal `ParticleEmitter { ... }` construction compiling unchanged. The one-shot lives entirely in a new component + constructor + system step.
- **Audio is native-only + best-effort.** The engine's `AudioManager` is `#[cfg(not(target_arch = "wasm32"))]`. ALL audio wiring in the example (the `use engine::AudioManager`, the setup block, the `play_tone` helper) is target-gated; a `#[cfg(target_arch="wasm32")] fn play_tone(...) {}` no-op stub keeps call sites uniform. This keeps the **wasm example build green**. On native, missing audio device → `AudioManager::new()` returns `None` → silent, never panics.
- **No physics (rapier2d).** AABB overlap via `SpatialGrid`/`Collider` like maze; rapier is overkill and complicates wasm.
- **PR base = `main`** (user's explicit call) → bundles the 13-commit unmerged stack. Same decision as Sokoban PR #1.

## Evidence & Data

### Verification commands run (all green)

- `cargo build --example shooter_game` → ok.
- `cargo clippy --lib --example shooter_game` → **0 warnings** (after fixing: `type_complexity` via `BurstSnapshot` alias; unused `dt` → `_dt` in HudSystem; gating `AudioManager`).
- `cargo fmt --check` → clean (ran `cargo fmt` to apply).
- `cargo test --lib` → **247 passed**, 0 failed (was 245 at session start; +2 new particle tests).
- `cargo test --lib particle` → 2 passed (`burst_emits_count_then_retires_emitter`, `continuous_emitter_unaffected_by_burst_path`).
- `cargo build --target wasm32-unknown-unknown --lib` → ok, 0 warnings.
- `cargo build --target wasm32-unknown-unknown --example shooter_game` → ok, **0 warnings** (after the audio gating fix; before it: `error[E0432]: unresolved import engine::AudioManager`).
- Native startup smoke-run → alive 7s, **no panic**, exit 0.
- **Interactive play → user-confirmed working.**

### Files changed (commit `70dd796` + `7b3bd75`)

| File | Change |
|---|---|
| `examples/games/shooter/shooter.rs` | NEW (~629 lines). The shooter example. |
| `src/particle.rs` | `ParticleEmitter::for_burst()`, `ParticleBurst` component, `ParticleSystem` step 3, `spawn_particle` helper, `BurstSnapshot` alias, 2 tests. (~211 lines added net.) |
| `src/lib.rs` | re-export `ParticleBurst`. |
| `Cargo.toml` | `[[example]] name = "shooter_game"` / `path = "examples/games/shooter/shooter.rs"`. |
| `docs/NEXT_WORK.md` | D → Done (with sub-bullets: engine gap closed + Pool surfaced-not-a-gap); E → Done; "Breadth pass complete" note; backlog now only F. |
| `docs/HANDOFF.md` | New session entry for the shooter. |
| `CLAUDE.md` | Module-map row: `ParticleEmitter, ParticleSystem, ParticleBurst (one-shot burst + ParticleEmitter::for_burst())`. |

### Shooter design constants (for regression / tuning)

- Window: `720 × 900` portrait, clear `[0.04, 0.05, 0.09, 1.0]`. `Camera::new(Vec2::ZERO, 1.0)` (world == screen, y down).
- Player: `PLAYER_SPEED 430`, `PLAYER_HALF 20`, `PLAYER_Y = H-90`, clamped to field.
- Bullets: `BULLET_SPEED 760` (upward, negative y), `BULLET_HALF 5`, `BULLET_POOL_CAP 64`, `FIRE_COOLDOWN 0.16`s. Released when `y < -BULLET_HALF*4`.
- Enemies: `ENEMY_HALF 18`, speed `80..=150` downward, despawn when `y > H + ENEMY_HALF*2` (slipping past the bottom costs no life — only contact does).
- Waves: `WAVE_INTERVAL 1.3`s; count `(3 + waves/3).min(6)` (saturates at wave 9).
- Lives `START_LIVES 3`, `INVULN_TIME 1.2`s after a hit.
- Collision layers (disjoint bits): `LAYER_PLAYER 1<<0`, `LAYER_ENEMY 1<<1`, `LAYER_BULLET 1<<2`. `CollisionGridSystem::new(64.0)`.
- Explosion: `ParticleEmitter::for_burst()` + `ParticleBurst{remaining:18}`, lifetime 0.45, spread 190, on a one-shot entity at the dead enemy's position.

### System order in `main()`

`CollisionGridSystem::new(64.0)` (rebuilds `SpatialGrid` resource first) → `PlayerSystem` (input/move/fire) → `BulletSystem` (advance + release offscreen) → `WaveSystem` (timer-spawn enemies) → `EnemySystem` (advance + despawn offscreen) → `CollisionSystem` (reads the grid built this frame) → `ParticleSystem` (drains bursts) → `HudSystem` (score/lives/controls text). CollisionSystem depends on the grid being rebuilt earlier the same frame.

### Controls

Move WASD/Arrows · Fire Space (held = repeat, gated by `FIRE_COOLDOWN`) · `R` restart (only meaningful at game-over but works anytime) · `Esc` quit.

### The bug that was fixed (commit `7b3bd75`)

`CollisionSystem`: `claimed: HashSet<Entity>` deduped enemies, but a single bullet overlapping two adjacent enemies in one frame was added to `bullet_kills` twice (the `break` exits only the inner `query_aabb` loop). At apply time `pool.release(bullet)` ran twice = double-release. Fix: added `spent: HashSet<Entity>`; `if spent.contains(bullet) { continue; }` at the top of the bullet loop, and `spent.insert(*bullet)` when a kill is recorded. One bullet → at most one enemy → released once.

### `ParticleBurst` / `for_burst` public API (full signatures, `src/particle.rs`)

```rust
// Existing — unchanged (continuous emitter). No new field added.
pub struct ParticleEmitter {
    pub spawn_rate: f32,        // per sec; 0 + emit=false => burst-only entity
    pub lifetime: f32,
    pub velocity: Vec2,
    pub velocity_spread: Vec2,  // burst path uses .max_element() as speed radius
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub size: Vec2,
    pub texture: Option<String>,
    pub emit: bool,
    pub(crate) timer: f32,
}

impl ParticleEmitter {
    pub fn for_burst() -> Self;  // emit=false, spawn_rate=0, short life, radial spread
}

// NEW — additive one-shot marker. Attach with ParticleEmitter on a dedicated entity.
pub struct ParticleBurst {
    pub remaining: u32,          // particles emitted in one tick, then emitter despawned
}
// Re-exported: engine::ParticleBurst  (src/lib.rs)
```

Usage (from the shooter's `spawn_explosion`):

```rust
let e = world.spawn();
world.add_component(e, Transform { position: pos, ..Default::default() });
let mut em = ParticleEmitter::for_burst();
em.color_start = color; em.lifetime = 0.45; em.velocity_spread = Vec2::splat(190.0);
world.add_component(e, em);
world.add_component(e, ParticleBurst { remaining: 18 });
// ParticleSystem next tick: emits 18 radial particles, then despawns `e`.
```

### Exact fixes applied to reach green (for reference)

- `error[E0432]: unresolved import engine::AudioManager` (wasm example) → split the import: gameplay types in the plain `use engine::{...}`, then `#[cfg(not(target_arch="wasm32"))] use engine::AudioManager;`. Gated the setup block and added a wasm no-op `play_tone` stub.
- `clippy::type_complexity` on the burst query's inline 8-tuple `Vec<...>` → added `type BurstSnapshot = (...)`.
- `clippy` unused `dt` in `HudSystem::run` → renamed to `_dt`.
- `clippy` unused marker structs (`Player`/`Bullet`/`Enemy` are tag-only, queried but never field-read) → `#[allow(dead_code)]` on each (matches maze_escape's marker convention).
- Borrow-checker: the bullet/enemy "advance" loops collect `Vec<Entity>` first, then `filter_map` with `world.get` — the maze "collect then get_mut" pattern — to avoid holding the query borrow across `get_mut`.

## Code Analysis

- `Pool::acquire` reuses a released entity (removing the `Pooled` marker) or spawns a new one, runs the `setup` closure. `Pool::release` adds the `Pooled` marker and pushes to the free deque (despawns on overflow past capacity). It does **not** strip other components — the example does that explicitly in `deactivate_bullet`, which is essential: otherwise pooled bullets would still render/collide.
- `restart_game`: returns all active `Bullet` entities to the pool (deactivate + release — no leak), despawns all `Enemy` entities, recenters the player, resets `Shooter` fields and both timers. `debug_assert!(pool.available_count() <= pool.capacity())` as a sanity check.
- `ParticleSystem` borrow pattern: collects an owned snapshot from `query2` first, then mutates the world (spawns particles / despawns the emitter) — the standard "collect then act" borrow workaround from `docs/PATTERNS.md`.
- The remove/reinsert `Pool` pattern is a latent panic risk if anything between `remove_resource` and `insert_resource` panics (the pool would be missing for the next `.unwrap()`). Acceptable for an example; noted by the review subagent.

## User Feedback & Preferences

- **"서브에이전트 적극 사용해서 동시 작업 가능한 부분 작업해. 서브에이전트 모델은 sonnet으로 사용"** — user wants subagents used aggressively for parallelizable work, on the **Sonnet** model. Honored: ran a Sonnet review agent + a Sonnet doc-reapply agent, in parallel with main-thread work. (This caught the double-release bug and recovered the lost docs.)
- User confirmed interactive play themselves ("게임은 테스트 정상적으로 진행됨") — they will run the GUI to verify feel.
- A stray message (`/tmp/fontmaker-editor-issue-smoke`) was **"잘못된 전달" (mis-sent)** — unrelated to this project; no action taken. Ignore in future.
- Korean is the user's working language; doc prose stays English per `doc-language-rule` (token cost). Commit style: `Co-Authored-By: Claude Opus 4.8`. PR body ends with the Claude Code generated-with line. PR base = `main` chosen knowingly.

## Where We're Going

The breadth-first example pass (A–E) is now complete; the docs declare it so. The only remaining backlog item is the **optional depth** candidate:

- **F — Top-down twin-stick / bullet survival** (`docs/NEXT_WORK.md`): steering (`SteeringSystem`/`Seek`/`Wander` already exist), many entities, GPU particles (`GpuParticleEmitter`, native-only). Would stress perf/`ProfilerData` and the GPU particle path the CPU `ParticleSystem` doesn't touch. This is a depth item, not required for the breadth pass. Complements `rust-survivors` directly.

No other planned follow-on. If F is skipped, the playable-examples program is effectively done for v1.0.0.

## Open Questions

- **PR landing strategy.** PR #2 (and #1) target `main`, bundling the whole unmerged stack (english-conversion docs + platformer + scene-flow + maze + sokoban + shooter). Should the `docs/english-conversion` stack merge to `main` independently first, then these rebase? User has accepted the bundle for now, but the two large mixed PRs (#1, #2) overlap heavily and will need ordered merging.
- **Is candidate F wanted?** Breadth is done. F is the only depth item; user hasn't committed to it.
- **`rust-survivors` impact.** `ParticleBurst` is purely additive (no `ParticleEmitter` field change), so no breakage expected — but not yet built against the sibling game repo (`/Users/jkl/Projects/rust-survivors`). Worth a `cargo build` there if/when convenient.

## Risks & Blockers

- **PR #1 and #2 are large overlapping bundles** against `main` — review/merge-order burden. Not blocking but needs a plan.
- **Tool-degradation recurrence.** This session hit a window of fabricated/stale tool output and silently-dropped edits. If it recurs: re-verify persistence with `git diff`/`git show --stat` and re-read files from disk before trusting any "success."
- No beads tooling (`bd` absent) — phase tracking is the plan file + TaskCreate, not `bd`.

## Quick Start for Next Session

```bash
# This work is DONE and in PR #2. To pick up the program:
cat plans/handoffs/HANDOFF_sokoban-example-game_shooter-example_2026-06-01.md  # this file

# Sanity-check the shipped shooter:
cargo run --example shooter_game        # move WASD, fire Space, R restart, Esc quit

# Re-verify green base before any new work:
cargo build && cargo clippy --lib && cargo test --lib   # expect 247 passing

# If continuing the examples program, the only remaining item is candidate F
# (top-down twin-stick / bullet survival) — see docs/NEXT_WORK.md backlog.
# Steering (engine::Seek/Wander/SteeringSystem) and GPU particles
# (engine::GpuParticleEmitter, native-only) already exist to build on.
```

## Session Closed

Candidate D (shooter) shipped: `shooter_game` example + additive `engine::ParticleBurst` one-shot particle API, all checks green INCLUDING user-confirmed interactive play. Commits `70dd796` + `7b3bd75` on `feat/shooter-example`, pushed, PR #2 against `main`. Subagents (Sonnet) used per user request — caught the bullet double-release bug and recovered docs lost to a tool glitch. Breadth pass A–E complete; only optional candidate F remains.

## Session Closed
**Closed at:** 2026-06-01
**Commit:** (this handoff committed below; feature work in `70dd796` + `7b3bd75`)
**Session status:** Handed off to next session
