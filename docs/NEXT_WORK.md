# Next Work — candidates and alignment

> Status: living document. Derived from `docs/VISION.md` (reset 2026-05-29).
> This lists what to build next and why, under the vision's core loop:
> **a feature is not done until a small, playable example game in `examples/`
> exercises it in real play.**

## Context

`examples/` now separates top-level feature demos from playable example games under
`examples/games/`. The first playable examples are the platformer
(`cargo run --example platformer_game`), scene-flow game
(`cargo run --example scene_flow_game`), and maze-escape
(`cargo run --example maze_escape_game`), which start closing the previous validation gap.
The active direction remains: widen the feature set breadth-first, and prove each feature
with a small playable example.

## Candidate feature × playable-example pairs

Each candidate pairs an example game with the engine capability it validates/extends and
the API gaps it is likely to surface.

| # | Example game | Engine capability validated/extended | Likely gaps to surface |
|---|--------------|----------------------------------------|------------------------|
| **A** | Platformer (jump, run, platforms) ✅ done | `CharacterController`, `move_character`, physics platforms/sensors, `AnimationStateMachine`, atlas animation, camera follow | surfaced + **fixed (v1.1.0)**: one-way platforms (`set_one_way` + `CharacterController::request_drop`, drop-through in `move_character`); tilemap↔physics binding (`PhysicsWorld::add_static_from_tilemap` + `TileCollider`). Platformer level is now a single `Tilemap`. |
| **B** | Top-down maze escape (chasing enemies) ✅ done | `PathGrid`/`find_path`, `BehaviorTree`, `SpatialGrid` collision (`examples/games/maze_escape/maze_escape.rs`) | surfaced + fixed: `BehaviorTree`/`Sequence`/`Selector`/`Inverter`/`AlwaysSucceed`/`BehaviorSystem` were not re-exported from `engine::`; `SpatialGrid` was trapped inside `CollisionGridSystem` (now mirrored to a `World` resource each frame); no `PathGrid::from_tilemap` (added). **Fixed (v1.1.0)**: added `BlackboardValue::Path(Vec<IVec2>)` + `Blackboard::set_path`/`get_path`; `ComputePathToPlayer` now caches the whole path and recomputes only when the player's goal tile changes. |
| **C** | Sokoban (box pushing) ✅ done | discrete grid logic, multi-level progression, undo/redo, `save`/`load` progress (`examples/games/sokoban/sokoban.rs`) | surfaced + fixed: no reusable game-facing undo — only the editor had a private command history; added genre-agnostic `History<T>` snapshot undo/redo (`src/history.rs`, re-exported from `engine::`). `save`/`load_or_default` reused unchanged (no friction). Immediate-mode `DebugDrawQueue` filled rects render board state without ECS entity churn. |
| **D** | Simple shooter (bullets, waves) | `ParticleEmitter`, `Timer`, collision layers, audio buses | pooling/spawn bursts, perf; complements rust-survivors |
| **E** | Scene-flow game (menu → play → result) ✅ done | `SceneCmd` Push/Replace/Pop, UI buttons, `GameState`, scene-owned systems, explicit entity cleanup | surfaced + **fixed (v1.1.0)**: `App::register_persistent::<T>()` preserves resources across the `Replace` World reset; `scene_flow_game` dropped its `Arc<Mutex<_>>` workaround. |
| **F** | Skeletal-animation showcase character ✅ done | NEW: 2D cutout skeletal animation (`src/skeletal.rs`, `examples/skeletal_puppet.rs`) | surfaced + fixed `HierarchySystem` depth-3 cap; scale-vs-attachment-size rule noted in `docs/SKELETAL.md` |

## Done

- **A — Platformer** (`platformer_game`): tile collision, gravity, jump, moving platforms.
- **B — Maze escape** (`maze_escape_game`): PathGrid + BehaviorTree + SpatialGrid.
- **C — Sokoban** (`sokoban_game`): discrete grid, undo/redo (`History<T>`), multi-level + save.
- **D — Simple shooter** (`shooter_game`): pooled bullets (`Pool`), `Timer` fire/wave cadence,
  `SpatialGrid`/`CollisionLayer` hit detection, score/lives + restart, explosion particles, audio buses.
  - **Engine gap closed:** `ParticleEmitter` was continuous-only → added the additive one-shot
    `ParticleBurst` component (+ `ParticleEmitter::for_burst()`); `ParticleSystem` drains it and
    retires the emitter. Re-exported as `engine::ParticleBurst`. Unit tests in `src/particle.rs`.
  - **Surfaced-but-not-a-gap:** `Pool` worked for bullet churn via `remove_resource`/reinsert per
    system; released bullets strip `Sprite`/`Collider`/`CollisionLayer` so they leave the renderer
    and grid. No new pooling API was needed.
- **E — Scene-flow / UI interaction** (`scene_flow_game`): menus, pause, transitions.
- **F — Top-down twin-stick survival** (`survivor_game`): WASD move + Arrow-key 8-way aimed
  fire, `engine::Seek`/`SteeringSystem` seeker enemies, pooled bullets, `SpatialGrid` hits,
  `GpuParticleEmitter` player thruster, `ProfilerData` perf HUD. Debug `G`/`B` keys to reach
  the perf scale on demand.
  - **Engine gap closed:** `SteeringSystem` was **O(N²)** — each entity self-looked-up via
    `query().find()` full scans. The example surfaced it with data (steer **3.67ms @ 200**
    seekers, ~30× expected, quadratic). Fixed to O(1) `world.get`/`get_mut(entity)` access
    (behavior-identical, additive) + a regression test in `src/steering.rs`. After: **0.48ms
    @200, 1.36ms @600** (linear), ~60fps held at 600 (3× the design target). `rust-survivors`
    rebuilt clean (no breakage).
  - **Surfaced-but-not-a-gap:** `GpuParticleEmitter` is native-only (`cfg(not(wasm32))`, like
    `AudioManager`) — the thruster is target-gated, wasm renders nothing. `frame_ms` is
    vsync-capped (`AutoVsync`) so per-system `ProfilerData.systems` (the HUD `steer` readout)
    is the real CPU-cost signal.

## Breadth + depth pass complete

A–E (breadth) and **F** (depth: steering + many entities + GPU particles) have all shipped
under the dogfooding loop. No remaining planned candidate — the playable-examples program is
done for v1.0.0.

## Coverage follow-up — UI / i18n / audio cluster (v1.2.0)

A subsystem-coverage audit found ~10 shipped subsystems with no *playable-game* coverage (only
standalone demos or none). The densest, most universally-needed cluster — UI depth + localization
+ audio buses — is now closed by a new playable example:

- **G — Settings + Dialogue** (`settings_menu_game`, `examples/games/settings_menu/`): Title →
  Settings → Dialogue. First playable-game use of `TextInput`, `Slider`, `CheckBox`, `ScrollView`,
  `Panel`/`LayoutSystem`, rich/multiline `Label`, `LocaleResource` (EN/KO/ES), and `AudioManager`
  buses + `AudioEffect` low-pass. `Settings`/locale/`AudioManager` persist across `SceneCmd::Replace`.
  - **Engine gap closed:** `LocaleResource` had no reactive binding, so switching locale meant
    manually re-resolving every widget's text. Added the additive `LocalizedText` component +
    `LocalizationSystem` (targets `Label`/`Button`/`CheckBox`); the example switches language with
    one `set_locale` call and the whole UI retranslates. Unit tests in `src/ui/localized.rs`.
  - **Documented gaps (not fixed):** runtime per-locale font switching is unsupported
    (`TextRenderer` font is fixed at init; `LocaleData.font` is dead) → non-Latin relies on native
    system-font fallback, absent on Linux CI / wasm; `LocaleData.direction` (RTL) is metadata only
    and not wired into alignment. Both await a future dedicated example.
  - **Deferred-item follow-up:** the macOS input-latency item is now *largely* addressed — the
    live window-drag freeze is gone (frame step factored into `App::step_frame`, driven inline
    from `Resized`, plus `pre_present_notify`); content keeps animating through both resize and
    titlebar-move drags. Residual: a one-frame lag at the *start* of a drag (documented limitation,
    CHANGELOG → Unreleased). Still deferred from this cluster: overlay caret, `TextInput`
    horizontal scroll, real OS fullscreen.

Remaining never-in-a-game subsystems (candidates for later dogfooding cycles, none scheduled):
2D lighting (`PointLight`/normal-map), `BlendTree1D`, `Timeline`/cutscene, `PostProcessConfig`,
physics joints, `RenderTarget`/`OffscreenCamera` in real play, networking.

## Alignment check — previously "planned" items vs the reset vision

Vision criteria: (1) fork-friendly skeleton, (2) genre-agnostic 2D, breadth-first,
(3) validate via playable examples, (4) semver after v1.0.

| Planned item | Nature | Alignment | Verdict |
|--------------|--------|-----------|---------|
| **Entity Generation v2** (`docs/ENTITY_GENERATION_V2_PLAN.md`) | correctness/safety, breaking | Fits the fork-friendly/learning goal, but it is neither breadth nor example-validated; it is a v2-only breaking change | **Cancelled (archived)** — removed from planned work; design preserved in the archived doc for a possible future v2.0.0. |
| **Dependency security follow-up** (glyphon→lru `RUSTSEC-2026-0002`, `paste` unmaintained) | maintenance/hygiene | Needed for a trustworthy forkable engine, but it is a renderer/wgpu-major migration: high-risk, non-breadth, non-example | **Cancelled (archived)** — removed from planned work; recorded as accepted/known risk in `docs/SECURITY_HARDENING_2026_05.md`. |
| **2D skeletal animation** | new feature | Directly fits genre-agnostic 2D breadth and is naturally validated by a playable example | **Done** — implemented as candidate **F** (`src/skeletal.rs`, `examples/skeletal_puppet.rs`). |

**Takeaway:** of the three pre-existing planned items, only skeletal animation matched the
current breadth-first + dogfooding priority and is now done. The other two were cancelled
from planned work and archived: Entity Generation v2's design is preserved for a possible
v2.0.0, and the dependency advisories are recorded as accepted/known risk.
