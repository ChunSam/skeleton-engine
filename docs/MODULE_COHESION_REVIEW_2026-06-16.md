# Module cohesion & coupling review — skeleton-engine v9.3.0

**Date:** 2026-06-16 · **Scope:** full `src/` tree · **Method:** 6-way parallel read-only review
(App/lifecycle · editor · renderer · ECS+serialization · physics+spatial · animation+audio+UI),
each cluster judged through a **fork-friendliness lens** (VISION priority #1 = clear module
boundaries + extension points). Findings cite `file:line` at v9.3.0.

> This is an **architecture/cohesion inspection**, not a bug hunt — almost nothing here is a
> correctness defect (those were handled by the v9.0.0 hardening + the
> `CODE_ANALYSIS_2026-06-16_COVERAGE.md` ledger). These are structural observations: where
> modules do too much, where boundaries leak, and where a forker would get stuck. Many fixes
> are **breaking or architectural** → treat this as a roadmap, not a checklist.

## Executive summary

The engine is **well-decomposed and not a ball of mud.** Module-level cohesion is generally
strong: most files do one thing, the rapier encapsulation is genuine, the two collision systems
are clearly separated, audio/animation are cleanly layered, and the ECS `World` is sectioned
rather than tangled. The robustness scaffolding (panic isolation, scene-reset registration
replay, `HierarchySystem` tail) is production-grade for a "skeleton."

The recurring weaknesses cluster into **6 cross-cutting themes**: (1) a few **god-units** that
grew without being split (App, `render()`, `update()`, `SpriteRenderer`, `tilemap.rs`,
`docked.rs`, `EditorState`); (2) **residual per-frame heap allocations** the 80-finding audit
didn't reach; (3) **backend types (wgpu/rapier/rhai) leaking into the public API**, hard-linking
forks to dependencies; (4) **fork-friction in the extension seams** (4 overlapping `register_*`
calls, no render-pass hook, no inspector-panel registration); (5) **misplaced code** living in
the wrong module; (6) **layer-boundary blur** (layout writing to the render queue, timeline
mutating the camera). None block shipping; they are the agenda for a future architecture pass
(several naturally belong to the next breaking `v10` window).

---

## Cross-cutting themes

### Theme 1 — God-units that outgrew their split (cohesion)
The highest-severity structural debt. Each is individually understandable but forces a forker to
read far more than their task requires.

| Unit | Problem | Recommendation | Breaking? |
|---|---|---|---|
| `App` (`app.rs:94–224`) | ~30 fields across 4 concerns (ECS/schedule, **all** renderers + intermediate textures, window/platform, editor) | Extract a `RenderState` struct owning all renderer fields + intermediate textures; `render.rs` operates on it | no (internal) |
| `App::render()` (`render.rs:183–1022`) | 839-line monolith inlining 7+ passes; forker can't insert a pass without editing it | Each renderer struct records its own pass (`pass.record(&mut enc, …)`); `render()` just sequences | no |
| `schedule::update()` (`schedule.rs:131–516`) | 386-line god-function mixing viewport calc, egui begin/end, schedule, systems, camera, tile-paint, hot-reload, scene transitions | Split into `compute_viewport()`/`run_systems()`/`post_systems()`; move egui begin/end to `egui_pass.rs` | no |
| `SpriteRenderer` (`sprite.rs:48–82`) | 5 concerns: world batching, UI primitives, `ShaderMaterial` pipelines, texture/RT cache, base geometry | Extract `MaterialRenderer` + `TextureCache`; leave sprite batching only | no (internal) |
| `tilemap.rs` (1,555 lines) | 5 concerns: data model, reactive render system, single-terrain autotile, multi-terrain autotile, coord utils | Split `tilemap/{mod,autotile,system}.rs` mirroring `physics/` | no (module split) |
| `editor/ui/docked.rs` (2,003 lines) | SM panel (~434 lines) + Timeline panel (~250) are inline in a catch-all file | Extract `ui/state_machine_panel.rs` + `ui/timeline_panel.rs` (mirror the existing `audio_panel.rs`) | no |
| `EditorState` (`state.rs:100–316`) | 47 flat fields (~35 cfg-gated): tile-paint(9), resize-gizmo(6), SM(3), RT(4)… | Group into sub-structs (`PaintState`, `GizmoResizeState`, `DockedRtState`) | no |

### Theme 2 — Residual per-frame heap allocations (smell, perf)
The 80-finding audit promoted several scratch buffers to fields (`PhysicsSystem`, `AnimationSystem`
are exemplary). These hot paths were **missed** and still allocate every frame. All fixes follow
the proven scratch-field pattern; most are additive (internal), **except `SteeringSystem`** (see note).

- `sprite.rs:522` — `live_material_entities` + `seen_new_hashes` `HashSet`s per `render()` (audit #72).
- `sprite.rs:436` — `atlas_entries: Vec` collected per frame (sibling of the already-fixed `draw_entries`).
- `steering.rs:141,169,200,237,268` — 5 `Vec<Entity>` per frame (audit #76). **`SteeringSystem` is a unit struct** → adding fields changes construction; either use `Default`-constructed scratch + keep `SteeringSystem` constructible, or accept a small breaking change (`add_system(SteeringSystem::default())`).
- `state_machine.rs:399–415` — `evaluate()` clones the target-state `String` every transition-frame; return `&str` + clone only at the write site.
- `tilemap.rs:743` — deep-clones the full cached tile grid every dirty frame (≈64KB on a 128² map); diff against `tilemap.tiles` directly / store flat `Vec<u32>`.
- `tilemap.rs:589–605` — `Vec<Entity>` + `HashSet<Entity>` allocated in the always-run preamble even when nothing changed.
- `ui/system/*_pass.rs` (×6) — each widget pass collects a fresh `Vec<Entity>` per frame; give `UiSystem` shared scratch.
- `collision/grid.rs:144` — `candidates_in_aabb` allocates `HashSet`+`Vec` per call (hot for AI queries).
- `ecs/world.rs:683,708` — `query_added`/`query_changed` scan + allocate even when only one of N tracked types is queried; invert index to `HashMap<TypeId, HashSet<Entity>>`.
- `physics/body_factory.rs:265` — `rb.colliders().to_vec()` per `remove_body` (16k tiny Vecs clearing a 128² collider map).
- `ui/panel.rs:160` — `panel.children.clone()` per panel per frame in the layout snapshot.
- `physics/system.rs:285` — full `prev_col_map` HashMap clone per frame (low priority; only with active contacts).

### Theme 3 — Backend types leaking into the public API (coupling / fork-friction)
A fork-friendly skeleton should let a forker swap a backend; exposing the backend's types in the
public surface hard-links them. (Contrast: `BodyHandle`/`ColliderHandle` newtypes do this RIGHT.)

- `render_target.rs:4–9` — `RenderTarget` exposes `pub texture/view/sampler/bind_group: wgpu::*`. Make `pub(crate)` + getters only where needed.
- `lighting.rs:115` — `LightingRenderer.normal_view/width/height` are `pub` (read by `render.rs`). Add accessors, make fields `pub(crate)`.
- `sprite/textures.rs:16` — `texture_layout() -> &wgpu::BindGroupLayout` is `pub`; `RenderTarget::new` should derive its own layout (a pure fn of device) instead of borrowing the sprite pipeline's.
- `physics/world.rs:330,338` — `get_collider`/`get_collider_mut` return raw `rapier2d::Collider` undocumented (vs `rigid_body`'s documented escape hatch). Add the same escape-hatch doc / engine-level wrappers.
- `asset.rs:117` — `ScriptAsset` embeds `rhai::AST` in the **generic** asset module; a forker swapping scripting engines must edit `asset.rs`. Move to `scripting/`.

### Theme 4 — Fork-friction in the extension seams (fork-friction)
These ARE the engine's reason to exist (priority #1). Each gap forces a forker to edit engine internals.

- **Four overlapping registration calls** with inconsistent shapes: `World::register_reflect_named`, `World::register_clone`, `App::register_serde_component` (`impl Into<String>`), `App::register_editable_component` (`&'static str`). Unify the name-param type; document which combination to call. `register_editable_component` is also **partially native-only** (the factory/remover halves) with no doc note.
- **No render-pass plugin hook** — a custom pass (shadows, outlines) requires forking `App::render()`. Add `trait RenderPlugin { fn record(&mut self, ctx, world); }` + `App::add_render_plugin`.
- **No inspector-panel registration** — `docked.rs:457–562` hardcodes `world.get::<Tilemap/ParticleEmitter/PointLight/AnimationStateMachine/Timeline>(sel).is_some()` checks; a new component sub-panel means editing `docked.rs`. Add `register_inspector_panel`.
- **`AssetServer` hot-reload sidecars** (`asset.rs:196`) — 3 named path-sets + 3 `watch_*` methods + 3 `poll_reloads` branches; every new registry needs surgery. The `HotReloadable` trait (v9.3.0) already exists — route the watchlist through it + one `watch_path`.
- `animation/mod.rs` — the mandatory 3-system order (BlendTree→Animation→StateMachine) is only in individual struct docs; add a module-level `//! # Registration order` block.
- `JointHandle` (`physics/world.rs:88`) — missing the `.raw()`/`from_raw()` escape hatch that `BodyHandle`/`ColliderHandle` have.
- `Wander` (`steering.rs`) — deterministic RNG not overridable without forking `SteeringSystem`; expose a `direction_for` fn field.

### Theme 5 — Misplaced code / wrong module home (cohesion)
- `SerdeComponentRegistry` lives in `prefab.rs` but is an independent serde concern (usable without scenes) → `serde_registry.rs`.
- `ScriptAsset` in `asset.rs` is Rhai-specific (see Theme 3).
- Tile-paint logic (`update_tile_paint`, `tile_paint_*`, `apply_paint_cells`) is in `editor/ui/gizmo.rs` — tile painting is not a gizmo op → `ui/tile_paint.rs`.
- `CameraUniform` defined identically in `sprite/geometry.rs:162` AND `gpu_particle.rs:32` → one home in `renderer/`.
- `OffscreenRenderInfo`/`ComponentFactory` type aliases in `app.rs` root, used only in `render.rs`/`editor/state.rs` → move to use sites.
- `register_core_component_metadata` (`core_resources.rs`) hardcodes UI/animation/timeline registrations under a "core" name → move to subsystem init.

### Theme 6 — Layer-boundary blur (coupling)
- `LayoutSystem` (`ui/panel.rs:160,222`) pushes `DrawRect` directly into `UiQueue` (a render resource) — panel backgrounds bypass the `UiOutput`→`submit_output` path every other widget uses. Route through `UiOutput`.
- `TimelineSystem` (`timeline.rs:329`) directly mutates `Camera::position`/`zoom` (the `CameraTarget` rig) — couples timeline to the concrete `Camera` and assumes a single camera. Document the single-camera assumption or emit an event.
- `EditorHistory::undo` (`editor.rs:162`) calls `physics::sync_tilemap_entity_colliders` directly — the editor undo knows about physics. Route through `App::sync_tilemap_colliders` (respects the opt-in).

### Theme 7 — DRY violations (smell)
- `compute_tile_mask` vs `compute_tile_mask_typed` (`tilemap.rs:281–482`) differ by ONE predicate line; the 50-line Blob8 block is copy-pasted → extract `compute_mask_raw(filled: impl Fn(i32,i32)->bool)`. **Correctness risk** (must patch twice).
- `ClipSetError` vs `ParticleConfigError` — structurally identical → shared `AssetLoadError`.
- egui pass (`update_texture`/`update_buffers`/`render`/`free`/`present`) duplicated in `render.rs:447` and `:980` → `submit_egui_pass()` helper.
- Scene-graph pre-collection duplicated in `editor/ui/mod.rs:200` and `:265`; docked pointer-gating duplicated ×3 in `window.rs:258/301/362`.

---

## Per-cluster cohesion verdicts

| Cluster | Verdict | Top issue |
|---|---|---|
| **App + lifecycle** | Good split, but `App` is a god-struct + `update()`/`render()` are god-functions | Theme 1 (extract `RenderState`, split the two big fns) |
| **Editor** | "Good with a known hot spot"; cleanly removable (1 field) | `docked.rs` 2003-line catch-all; `EditorState` 47 fields |
| **Renderer** | Well-decomposed at the pass level; `SpriteRenderer` overloaded | 5-concern sprite struct; wgpu types leak; no pass hook |
| **ECS + serialization** | Strong file-level cohesion; **not** a ball of mud | `World` owns reflect registry; 4 `register_*` shapes; AssetServer sidecars |
| **Physics + spatial** | Best-encapsulated cluster (rapier newtypes); `tilemap.rs` overloaded | `tilemap.rs` 5 concerns; `SteeringSystem` 5 Vecs/frame |
| **Animation + audio + UI** | Well-stratified leaf layer | undocumented 3-system order; BlendTree↔SM conflict; UI per-frame Vecs |

---

## What's GOOD — preserve, do not "refactor"

These are genuine strengths a refactor must not regress:
- **Rapier encapsulation** — `BodyHandle`/`ColliderHandle`/`CollisionGroups` opaque newtypes with documented `.raw()` escape hatches and the "why vs `CollisionLayer`" rationale in-source. The model for Theme 3.
- **`PhysicsSystem` + `AnimationSystem` scratch-buffer pattern** — the model for Theme 2.
- **Two collision systems cleanly separated** (`physics/` rapier vs `collision/` SpatialGrid), zero cross-dep; `tilemap.rs` has zero physics imports (opt-in coupling only).
- **`World` is sectioned, not a god-object**; `Events`/`Commands` are minimal exemplars; `world_registrars` scene-reset replay is sound.
- **Panic isolation + schedule policy + `HierarchySystem` tail** — production-grade, documented, tested.
- **`UiOutput` accumulator** decouples widget passes from the renderer (testable in isolation).
- **Audio module split** (`bus`/`positional`/`ducking`/`effects`/`playback` as focused `impl` blocks; cfg-gated once).
- **`docked_rt.rs`** pure, tested, editor-independent geometry helpers; **`EditorState`** isolation (remove editor = delete 1 field); **`audio_panel.rs`/`data_table_panel.rs`** are the panel template; **`sort.rs`** isolated + tested.
- **RT-isolation fix** (each offscreen target its own command buffer) + **shared `fullscreen_quad.wgsl`**.
- **`scripting/` thread-local encapsulation** — Rhai never leaks into the ECS layer.

---

## Prioritized action list

Sequenced by value × (low risk first). None are required to ship; this is the roadmap.

1. **Additive perf pass (Theme 2, non-breaking subset)** — promote the missed scratch buffers to fields: `sprite.rs` HashSets + `atlas_entries`, `ui` widget Vecs, `tilemap` cached-grid clone + preamble, `state_machine` string clone, `collision` candidates scratch, `body_factory` remove alloc. Each behavior-identical + unit-testable. *(`SteeringSystem` #76 deferred — needs a construction change.)*
2. **DRY consolidation (Theme 7)** — `compute_mask_raw` (correctness risk), shared `AssetLoadError`, `submit_egui_pass` helper, scene-graph/pointer-gating helpers. Mostly internal.
3. **Module-home moves (Theme 5)** — `SerdeComponentRegistry`→`serde_registry.rs`, `ScriptAsset`→`scripting/`, tile-paint→`ui/tile_paint.rs`, `CameraUniform` dedup, panel extraction from `docked.rs`. Re-export for back-compat → additive.
4. **Doc + small-API fork-friction (Theme 4 cheap subset)** — animation module ordering doc, `JointHandle::raw()`, `register_editable_component` wasm note, `Wander::direction_for`. Additive.
5. **Encapsulation tightening (Theme 3)** — make wgpu/rapier fields `pub(crate)` + accessors. **Breaking** (public field removals) → batch into the next breaking window.
6. **Architectural extraction (Theme 1)** — `RenderState` out of `App`, split `render()`/`update()`, `SpriteRenderer`→`MaterialRenderer`+`TextureCache`, split `tilemap.rs`. Large; internal-mostly but high-effort. A dedicated arc.
7. **New extension points (Theme 4 big subset)** — `RenderPlugin` hook, `register_inspector_panel`, `AssetServer` unified watchlist via `HotReloadable`, registration-API unification. These most advance the fork-friendliness mission; design carefully (some breaking).

**Recommendation:** items 1–4 are safe autonomous follow-ups (additive, testable); items 5–7 are
breaking/architectural and belong to a scoped `v10` design pass with user sign-off.
