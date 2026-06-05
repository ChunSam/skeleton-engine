# skeleton-engine — Full Codebase Analysis

> Generated 2026-06-05 by a multi-agent full-codebase pass (15 subsystem readers +
> 6 cross-cutting review lenses + synthesis). Findings cite `file:line`. Severity:
> HIGH = likely to bite a real game / data-loss / security; MEDIUM = real but narrow;
> LOW = polish. See `docs/HANDOFF.md` for which items have since been fixed.

> **Resolution status (2026-06-06): all 30 issues addressed.** The findings below
> are the original as-of-analysis snapshot and are intentionally left unedited.
> - HIGH #1–#6, MEDIUM #9/#10/#14/#16/#17/#21, LOW #24/#25/#26 → PR #7 (non-breaking sweep).
> - MEDIUM #11 (Color), #13 (PhysicsWorld resource) → PR #8 (v3.0.0 breaking batch).
> - MEDIUM #7/#8/#18/#19 (perf), #20/#22/#23 (robustness), #12/#27/#29/#30 (additive API)
>   → PR #9. **#15 was a false positive** — the point-light radius was already correct;
>   locked with a contract test, no math change.
> - LOW #28 (JointHandle newtype) → v4.0.0 breaking change (the only item that forces a
>   major bump, so done last).
> Git log is the source of truth for exact commits.

## 1. Executive summary

skeleton-engine is a wgpu-based, MIT-licensed, genre-agnostic 2D game engine (~36 KLOC
across ~15 subsystems plus ~9 KLOC of examples) built on a custom archetype ECS, with
rapier2d physics, Rhai scripting, an egui-based editor/inspector, and full native +
wasm32 targeting. Maturity is **broad and structurally sound, but uneven in polish**:
the foundational layers (ECS, physics, save/crypto, scheduler, hierarchy) are genuinely
solid and well-tested, while several higher-level subsystems (editor undo/redo, audio
fade lifecycle, animation edge-cases) carry real correctness bugs. The architecture
honors the "skeleton" VISION goal — clean one-directional layering (ECS → data →
subsystems → App), no circular dependencies, a faithful gameplay-writes / renderer-reads
seam.

Four most important takeaways:
1. A handful of real HIGH-severity bugs cluster in the native-only editor and at
   scene-transition boundaries (`panicked_systems` leak, broken redo paths, hardcoded
   component-removal).
2. The topological scheduler is not dogfooded — built-in systems publish no
   `SystemLabel`, so the engine's own documented ordering requirements live in tribal
   knowledge.
3. Cross-cutting ergonomic friction (three incompatible color representations; a
   confusing screen-vs-world coordinate convention that the engine's own examples get
   wrong) erodes the "pleasant to ship a 2D game" goal.
4. The test suite is respectable (285 passing) but blind exactly where it matters most —
   scene transitions, wasm runtime behavior, and example rendering are all unverified,
   which is why off-screen-render bugs shipped in `loading_bar`/`minimap`.

Security posture is good for a fork-it skeleton: correct AEAD save encryption, an
honestly-scoped Rhai sandbox, bounded network queues. None of the issues are
architectural dead-ends; they're fixable defects and missing extension-point polish.

## 2. Architecture overview

Layering (verified one-directional, no cycles):

```
ecs/ (+ reflect leaf trait)          ← archetype storage, queries, scheduler, Commands, Events
   ↓
data layer (no upward deps)          ← components, camera, resources, asset, scene, timer,
   ↓                                    tween, timeline, hierarchy, prefab, atlas, locale
subsystems (read &World only)        ← renderer, physics, ui, animation, skeletal, audio,
   ↓                                    behavior/steering/pathfinding, collision, tilemap,
                                        particle, scripting, input, network
   ↓
App (sole orchestrator)              ← owns World + renderers + winit loop + scene stack + schedule
```

The data layer has zero upward dependencies on `app` or concrete renderer types, and the
renderer reads only data components with no imports of physics/scripting/behavior/ui/audio
— the render-layer separation is faithfully upheld. `App` is a ~50-field god-object but
decomposed into focused submodules, keeping each file locatable.

**Native vs wasm:** physics/audio/gpu_particle/lighting/fade are cleanly
`#[cfg(not(wasm32))]`-gated at module + field level; the wasm GPU init is async via
`spawn_local` + thread-local `PENDING_GPU`. The one leak: `src/save.rs` and
`src/asset/script_loading.rs` call `std::fs` unconditionally, failing at runtime on wasm
instead of being gated.

**VISION fit:** mostly met — clear boundaries, single-method `System` trait, real
exercised extension points (`register_persistent`, `register_event`,
`register_component`, `Reflect`, `Scene`, `ShaderMaterial`, `register_render_target`).
Misses at the margins: no Plugin/pass registry on `App`, the editor ignores its own
`register_component`, and the scheduler exposes no built-in labels.

## 3. Subsystem health table

| Subsystem | Maturity | LOC | Top concern |
|---|---|---|---|
| ECS core | solid | ~2036 | No `query_mut`; forces collect-then-`get_mut` in nearly every system |
| App / main loop | ok | ~2146 | `panicked_systems` never cleared across scene transitions (HIGH); god-object, no pass registry |
| Editor / Reflect / DebugUi | ok | ~1235 | Broken undo/redo + hardcoded 4-type component removal (HIGH) |
| Renderer core | ok | ~1963 | Render-pass-per-batch; `params_buffers` GPU leak; per-frame WGSL string clones |
| Renderer advanced | ok | ~1050 | Point-light radius ~½ size; GPU particle capacity hardcoded 4096 |
| Physics | solid | ~1489 | `one_way_colliders` not cleaned on `remove_body` → stale handle reuse |
| Animation + Skeletal | solid | ~1301 | `1.0/fps` div-by-zero / infinite spin on `fps<=0`; single-`if` frame advance lags on big dt |
| AI (behavior/steering/pathfinding) | ok | ~1062 | Steering last-writer-wins (no blending); A* has no closed-set guard |
| Assets + Scripting | ok | ~1552 | Per-frame full Rhai AST clone + 4×`Arc<Mutex>` per scripted entity; `load_script` unguarded fs on wasm |
| UI subsystem | solid | ~2044 | CheckBox toggles on press vs Button on release; ScrollView has no item-click event |
| Audio | ok | ~806 | `update()` not driven by any built-in system (silent broken fades); no SFX caching |
| Input | ok | ~1241 | `InputMap` keyboard-only/single-binding; `MouseButton` not re-exported |
| Gameplay infra | solid | ~2628 | `save`/`load` not wasm-gated; duplicate-tag silent overwrite in `spawn_scene_def` |
| Collision/tilemap/particle/camera/atlas/network | ok | ~2943 | `RenderLayer(i32)` vs `layer_mask(u32)` clamp folds negatives to bit 0; `camera.zoom=0`→NaN |
| Examples (dogfooding) | ok | ~9288 | `loading_bar`/`minimap` render off-screen (broken acceptance tests); coverage gaps |

## 4. Top strengths

- Genuinely clean, verified layering with no circular dependencies — the core VISION goal actually delivered.
- Robust ECS foundation — generation-checked `Entity` handles reject stale access on every mutation path (`world/tests.rs:438`); dense archetype iteration; documented borrow workarounds.
- System panic isolation — every user system runs inside `catch_unwind` + configurable `SystemPanicPolicy`.
- Correct, well-tested save encryption — ChaCha20-Poly1305 AEAD, fresh CSPRNG nonce per write, header/length validation before slicing, tamper/wrong-key → `SaveError::Corrupted` (not panic), honest threat-model docs.
- Offscreen render-target isolation — each RT submits its own command buffer so per-target camera writes don't bleed into the main pass.
- High example breadth — ~23 subsystems each have an interactive example; most are real playable games with win/lose states.
- Bounded, DoS-resistant network layer — per-message size caps, bounded queues, malformed-frame handling without panics, native + wasm.
- Adversarial testing where present — save tamper-detection, schedule policy through real `app.update()`, depth-5 hierarchy, physics rotation sync, timeline NaN edges.

## 5. Prioritized issues

### HIGH

1. **`panicked_systems` never cleared across scene transitions** — `src/app/schedule.rs:186,214` + `src/app/scenes.rs:67-91`. The panic-disabled-index set is never cleared. On `Replace` (refills from index 0) or `Pop` (`truncate`), stale indices silently suppress the new scene's systems. Fix: clear on Replace/Pop/reload; for Pop prune entries `>= new_len`; log on clear.
2. **Editor undo/redo broken in three paths** — `src/app/editor.rs:91-106`, `editor/ui/gizmo.rs:96-133`. CreateEntity redo doesn't re-push; DeleteEntity redo despawns `*selected` not the recorded target; multi-entity gizmo drag pushes only one `MoveEntity`. Fix: symmetric redo; track actual targets; push a command per moved entity.
3. **Editor component removal is a hardcoded 4-type match** — `src/app/editor/ui/mod.rs:532-547`. Components added via `register_component` get a "✕" button that does nothing. Fix: type-erased remove closure in the registry, or hide the button when no remover exists.
4. **`params_buffers` GPU memory leak (no eviction)** — `src/renderer/sprite.rs:72,569-585`. Per-entity `(Buffer, BindGroup)` map grows monotonically. Fix: prune dead entities at end of `render()`; clear on reload.
5. **Animation div-by-zero / infinite spin on `fps <= 0`** — `src/animation/system.rs:29,61`. `fps < 0` → negative `frame_dur` → crossfade `while` loop hangs. Fix: guard `fps <= 0.0`, validate in the constructor.
6. **Examples render entirely off-screen** — `examples/loading_bar.rs:64-97`, `examples/minimap.rs:163-184`. Content placed at negative/centered coords against a top-left pixel projection. Fix: reposition into positive screen pixels via `ViewportSize`.

### MEDIUM

7. Per-frame Rhai AST deep-clone + 4×`Arc<Mutex>` per scripted entity — `src/scripting/execution.rs:46,53-58`. Fix: `Arc<rhai::AST>` + per-runner reusable buffers.
8. Sprite renderer opens a new render pass per texture-run and per material — `src/renderer/sprite.rs:626-647,665-688`. Fix: one pass for the whole sorted stream.
9. `save`/`load`/`delete`/`load_script` not wasm-gated → runtime failure with green CI — `src/save.rs:77-130`, `src/asset/script_loading.rs:12-43`.
10. Built-in systems publish no `SystemLabel` — `src/ecs/schedule.rs`. Fix: `pub const` labels for ordering-sensitive built-ins.
11. Color represented three incompatible ways — `[f32;4]`, `[u8;4]`, `[f32;3]`. Fix: a `Color` newtype accepting `impl Into<Color>`.
12. Coordinate convention confusing and mis-used in flagship examples. Fix: anchor enum / `DrawText::centered` + docs.
13. PhysicsWorld access asymmetric with SpatialGrid — `src/physics/system.rs:24-33`. Fix: mirror PhysicsWorld to a World resource.
14. No mutable-iteration query — `src/ecs/world.rs:306-338`. Fix: add `query_mut`/`query2_mut`.
15. Point-light radius unit mismatch — `src/renderer/lighting.rs:97,148`. Fix: unify CPU/shader space; add a visual-size test.
16. Physics one-way collider handle leak on body removal — `src/physics/world/body_factory.rs:239`.
17. Continuous particle emitter under-emits on slow frames — `src/particle.rs:204-213`. Fix: `while timer >= interval`.
18. A* has no closed-set guard — `src/pathfinding.rs:173-201`.
19. CollisionGridSystem deep-clones two HashMaps every frame — `src/collision/grid.rs:197`. Fix: `Arc<SpatialGrid>`.
20. Audio `update()` not driven by any built-in system; no SFX caching — `src/audio/playback.rs:122,188`.
21. CheckBox toggles on press while Button fires on release — `src/ui/system/checkbox_pass.rs:32`.
22. `RenderLayer(i32)` vs `layer_mask(u32)` clamp folds negatives to bit 0 — `src/renderer/sprite/sort.rs:88-94`.

### LOW

23. `spawn_scene_def` silently overwrites duplicate tags — `src/prefab.rs:301-303`.
24. `camera.zoom == 0` → NaN projection — `src/camera.rs:78,88,101-103`; also missing `world_to_screen`.
25. Timeline all-NaN-keyframe track panics — `src/timeline.rs:112` (`rposition(...).unwrap()`).
26. `Timer::repeating(0.0)` fires every frame — `src/timer.rs:43-58`.
27. `MouseButton` not re-exported; examples reach into internal/winit paths + Korean-only doc comments — `src/lib.rs:48`.
28. rapier `ImpulseJointHandle` leaks through the public API — `src/physics/mod.rs:10`.
29. `ReflectValue` is a closed enum (no `#[non_exhaustive]`, no `I32`) — `src/reflect.rs:8`.
30. Incomplete Rhai resource limits — `src/scripting/api.rs:9-12`.

## 6. Cross-cutting assessment

- **Architecture / fork-friendliness** — strongest dimension: verified one-directional layering, no cycles, real exercised extension points. Weak at the seams a forker extends: no built-in scheduler labels, no plugin/pass registry, editor ignores `register_component`.
- **API ergonomics** — strong consistent core undermined by three color representations, the screen-vs-world confusion, PhysicsWorld/SpatialGrid asymmetry, the missing mutable query, and `MouseButton` not being re-exported.
- **Correctness** — hot paths are defensively coded and foundations sound; real bugs cluster in the native-only editor and at boundaries. The synthesis recheck downgraded four overstated claims (scripting `get_mut().unwrap()` is guarded by an earlier `get`; the audio "dead branch" is reachable; wasm save/load returns `Err` not panic; `last_dt` is set before `update`).
- **Performance** — dominated by collect-then-`get_mut` allocations (mostly benign at 2D entity counts). High-leverage: render-pass-per-batch, Rhai AST clone churn, SpatialGrid deep-clone, audio per-`play()` disk reads, A* missing closed-set.
- **Testing** — 285 passing with depth on ECS/crypto/schedule/hierarchy/physics, but blind on scene transitions, `register_persistent` round-trip, executable wasm tests (build-only gate), and example rendering (examples only compile, never run).
- **Security** — good for a fork-it skeleton; the May 2026 hardening is genuinely applied. Two robustness gaps: wasm `std::fs` failure with no compile signal, and op-limit being the only Rhai DoS guard. No memory-unsafety; the single `transmute` (egui) is guarded.

## 7. Examples / dogfooding coverage

The "example is the acceptance test" loop holds in breadth but is structurally weak in
verification — CI compiles examples but never runs them, and they're windowed event-loop
binaries. Features lacking a proving example: `Prefab`/`spawn_entity_def`/`SceneDef`,
Rhai scripting, `DebugUi` custom panels, `Reflect`/`ReflectValue`, gamepad, positional
audio (`play_at`/`play_spatial`), `Tween`, hot-reload, physics `add_prismatic_joint`.

## 8. Recommended next steps

Fix first — correctness bugs that silently break forked games:
1. Clear `panicked_systems` on transitions (#1) + first scene-transition tests.
2. Repair editor undo/redo + component-removal (#2, #3).
3. Guard float/hang hazards: animation `fps<=0` (#5), `camera.zoom=0` (#24), all-NaN timeline (#25), `Timer::repeating(0.0)` (#26).
4. Plug leaks: `params_buffers` eviction (#4), one-way handle cleanup (#16).
5. Fix off-screen examples (#6) + add a headless smoke-harness.

Then — fork-friendliness & ergonomics: wasm-gate save/load (#9); publish `SystemLabel`
constants (#10); `Color` newtype (#11) + coordinate anchor helpers (#12); mirror
PhysicsWorld to a resource (#13) + `query_mut` (#14).

Then — performance & polish: single sprite pass (#8), `Arc<rhai::AST>` (#7),
`Arc<SpatialGrid>` (#19), audio caching + `AudioSystem` (#20), A* closed-set (#18); add
proving examples for the untested extension points.
