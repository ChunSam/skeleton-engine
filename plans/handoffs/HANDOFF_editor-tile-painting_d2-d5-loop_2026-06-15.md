# D-2→D-5 editor /loop: 4 subsystem-editor features shipped (v8.20→8.23)

**Date:** 2026-06-15
**Status:** COMPLETED (all 4 commissioned features shipped + merged; loop concluded)
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine editor authoring tools
**Chain:** `editor-tile-painting` seq `3`
**Parent:** `HANDOFF_editor-tile-painting_a-g-editor-loop_2026-06-15.md`
**Prior chain:** `HANDOFF_editor-tile-painting_v8.11-shipped_2026-06-15.md` (seq 1) > `HANDOFF_editor-tile-painting_a-g-editor-loop_2026-06-15.md` (seq 2) > this (seq 3)

## Related Handoffs

- `HANDOFF_editor-gui-arc_engine-wide-bug-hunt_2026-06-15.md` — editor-gui-arc seq 3. Built the docked
  editor, gizmo, `EditorCmd`/`EditorHistory`, inspector — the infrastructure every feature here extends.
- `HANDOFF_deferred-candidates_feature-loop_2026-06-15.md` — the reusable feature-loop/subagent recipe
  (plan → implement → Gate6 → unit-test → PR → merge) this loop reused.
- `HANDOFF_lit-dungeon-lighting_example-and-engine-fixes_2026-06-05.md` — built `PointLight`/`AmbientLight`
  + the `lit_dungeon` example that D-5's lighting editor targets.

## Stale References

All parent identifiers still valid and used this session (`Tilemap::cell_at_world`/`cell_center_world`/
`set_tile`/`from_tilemap`, `EditorCmd`, `EditorSettings`, `DebugDraw::rect`/`rect_filled_z`,
`SerdeComponentRegistry`, `register_default_components`, `component_factories`/`component_removers`,
`update_tile_paint`). None removed. No stale references.

## Since Last Handoff

Parent (seq 2) concluded the autonomous **A–G editor loop** at 7 features (v8.13–8.19) and deferred the
remaining **category-D subsystem editors**, listing them under "Where We're Going":
(1) small/feasible — audio mixer (needs `bus_names()`), pathfinding overlay; (2) viewport — lighting
editor, particle tuner; (3) LARGE — state-machine graph, timeline.

- **This session built the parent's tiers (1) + (2): D-2 pathfinding overlay, D-3 audio mixer, D-4
  particle tuner, D-5 lighting editor** — all 4 shipped, Gate6-green, self-merged (#52–#55, v8.20–8.23).
- **The parent's open question — "add `AudioManager::bus_names()`?" — was answered YES** and shipped
  (D-3). Deduped union of `channel_buses` values ∪ `bus_volumes` keys, exactly as the parent proposed.
- **Tier (3) LARGE editors (state-machine graph, timeline) remain deferred** — the user scoped this loop
  to D-2~D-5 explicitly, leaving the two multi-session visual arcs for a separate commission.
- **Validation strategy held:** unit-test through the real handler (the parent's core lesson). No GUI
  playtest this session — the cursor-freeze ceiling + display availability make it low-value; every
  feature was validated by driving its real code path in a test.
- **Trajectory:** same breadth-first, one-feature-per-cycle, merge-authorized autonomous loop as the
  parent — but scoped (4 named features) rather than open-ended (A–G), and concluded by completion
  rather than a milestone `AskUserQuestion`.

## Reference Documents

- `CLAUDE.md` — conventions + the Gate6 "Verification" checklist (the pre-commit bar). Header now v8.23.0.
- `docs/VISION.md` — "a feature is not done until a playable example exercises it" (each D editor is
  exercised by opening F2 on an existing example: `maze_escape`/`diagonal_pathing`, `audio_ducking`,
  `data_particles`/`gpu_particles`, `lit_dungeon`).
- `plans/{pathfinding_overlay,audio_mixer,particle_tuner,lighting_editor}_plan.md` — one plan +
  completion-criteria doc per feature (written before implementing, per the user's standing preference).
- `docs/CHANGELOG.md` — per-version record, 8.20.0 → 8.23.0.

## The Goal

The user ran `/loop d-2~d-5 진행 opus 판단으로 진행. gate6 후 셀프 머지. 중간 보고 생략하고 완료시 일괄
보고. 셀프 테스트 가능하면 테스트 진행하고, 모든 작업 마무리 되면 handoff 스킬 사용해서 문서화 하고 푸시까지
진행.` — an autonomous mandate to build the four remaining feasible category-D subsystem editors
(D-2…D-5) in an opus-chosen order, **with merge authority delegated for this session**, mid-reports
suppressed, self-testing where possible, and a `/handoff` + push at the end. Goal: round out the
in-game docked editor's authoring tools so a designer can inspect pathfinding, balance audio buses,
tune particles, and place/edit lights — each feature additive (semver-minor, keeps `rust-survivors`
unaffected) and validated through real code paths.

## Where We Are

- **`main` = `5b137e7` (v8.23.0)**, clean tree. 4 feature PRs (#52–#55) merged + all branches deleted.
- **Shipped this loop (all merged, Gate6-green, additive, native-gated editor surface):**
  - **D-2 v8.20.0 (#52)** — Pathfinding-grid overlay. Toolbar **Path** toggle → for each `Tilemap`,
    builds `PathGrid::from_tilemap(tm, |id| id != 0)` and shades cells via `DebugDraw`: blocked filled
    red (`rect_filled_z`), walkable outlined green (`rect`). `App::draw_pathfinding_overlay`. Persisted
    in `EditorSettings.show_pathgrid` (`#[serde(default)]`). +1 test.
  - **D-3 v8.21.0 (#53)** — Audio bus mixer. New public `AudioManager::bus_names()` (sorted/deduped
    union of assigned + volume-only buses, via pure `collect_bus_names`). New bottom-panel **Audio** tab
    (`bottom_tab == 2`) with one volume slider per bus → `set_bus_volume`. New module
    `src/app/editor/ui/audio_panel.rs`. +3 tests.
  - **D-4 v8.22.0 (#54)** — Particle live-tuner. Inspector **Particle Tuner** section (entities with a
    `ParticleEmitter`): drag editors for `emit`/`spawn_rate`/`lifetime`/`velocity`/`velocity_spread`/
    `size` + r/g/b/a color drags; mutates in place (live while sim runs). **Reset to Default** →
    `App::reset_particle_emitter` (preserves texture). +2 tests.
  - **D-5 v8.23.0 (#55)** — Lighting editor. Inspector **Point Light** section (drags for
    color/radius/intensity/light_height + Reset → `App::reset_point_light`); `PointLight` registered as
    an editor component (Add/Remove + "+ Add"), so select-entity-then-add places a light at its
    `Transform`. **Ambient Light** section edits the global `AmbientLight` resource (color/intensity),
    `App::ensure_ambient_light` inserts a default first. +3 tests.
- **Tests: 586 lib tests pass** (was 577 at loop start; **+9** across the four features). `cargo test
  --all-targets` clean every cycle.
- **`rust-survivors` impact:** none. D-3 adds one public method (`bus_names`); everything else is
  editor-internal/native-only. No public API removed or changed.

## What We Tried (Chronological)

1. **Onboarding (seq-2 handoff)** — verified `git log -1` = `13f1873` atop `452488c`, `cargo test --lib`
   = 577, read the four key editor files (`state.rs`, `editor.rs`, mapped `gizmo.rs`/`docked.rs` by
   signature to save context). Surfaced a scope+merge-authority `AskUserQuestion`; user rejected it to
   add context, then chose **D-2~D-5 / self-merge / suppress mid-reports** and issued the `/loop`.
2. **Survey before scoping** — read `src/audio/bus.rs`, grepped `PathGrid`/`PointLight`/`Timeline` public
   APIs and which examples exercise each subsystem (lighting=`lit_dungeon`, pathfinding=`maze_escape`/
   `diagonal_pathing`, audio=`audio_ducking`, timeline=`timeline_cutscene`). Confirmed each D editor can
   be exercised by F2 on an existing example — no new example needed. Organized the work list for the user.
3. **D-2 pathfinding overlay (#52)** — reused the D-1 bounds-overlay pattern exactly (`DebugDraw`
   world-space, `EditorSettings`-persisted toggle, per-frame call in `ui/mod.rs` gated on the flag).
   Snapshot `Tilemap`s (clone) to release the world borrow before `DebugDraw`. Built `PathGrid` per-frame
   so it visualizes the real subsystem with zero example changes. Unit test: 3×3 map, center blocked →
   8 outline shapes + 1 filled rect. 577 → 578.
4. **D-3 audio mixer (#53)** — added `AudioManager::bus_names()` backed by a pure free fn
   `collect_bus_names(channel_buses, bus_volumes)` (so the merge/sort/dedup logic is testable without an
   audio device). New `audio_panel.rs` mirrors the data-table panel's collect-then-apply borrow pattern.
   Bottom panel selector went from a 2-way `if/else` to a 3-way `match` (Assets | Data Tables | Audio).
   578 → 581.
5. **D-4 particle tuner (#54)** — found `ParticleEmitter` does NOT derive `Reflect`, so the reflect-grid
   inspector can't show it → built a dedicated **Particle Tuner** section (mirrors the `Tilemap`-only Tile
   Paint precedent). Used flat sequential `ui.horizontal` rows (not nested closures) to keep the `&mut
   ParticleEmitter` borrow disjoint per row. Added a testable `App::reset_particle_emitter` (the slider
   wiring itself is direct egui field editing). 581 → 583.
6. **D-5 lighting editor (#55)** — registered `PointLight` as an editor component (factory + remover) so
   "place a light" = select entity + add component (works around the cursor-freeze that blocks viewport
   click-to-place). Added **Point Light** + global **Ambient Light** inspector sections, with testable
   `App::reset_point_light` and `App::ensure_ambient_light`. Factory test calls the closure via disjoint
   field borrows (`app.editor` vs `app.world`) — NOT `.clone()` (a `Box<dyn Fn>` isn't `Clone`). 583 → 586.
7. **Concluded** — all 4 features done; ran `/handoff` (this file) + push. No milestone question this time
   (scope was explicitly D-2~D-5; completion = done).

## Key Decisions

- **Merge authority delegated for THIS session only** — the user chose "gate6 후 셀프 머지". Re-confirmed
  because the parent loop's authorization did NOT carry over (a new session). Each feature self-merged
  after Gate6: `git merge --no-ff` branch → push → PR auto-MERGED → `git branch -d` + delete remote.
- **One feature per wake-up cycle, fresh-ish context, self-paced via `ScheduleWakeup`** (120s cache-warm
  delays). No external event to monitor → no `Monitor`; the `/loop` prompt re-fires each cycle.
- **Build `PathGrid` from `Tilemap` per-frame** (D-2) rather than require games to insert a `PathGrid`
  resource — visualizes the real subsystem with zero example changes. Rejected: a `PathGridResource` the
  game must populate (more invasive, no example uses it).
- **Dedicated inspector sections, not `Reflect` retrofits** (D-4, D-5) — `ParticleEmitter`'s
  `Option<Arc<str>>` + `pub(crate) timer` and the desire for per-field ranges make a hand-built section
  cleaner than deriving `Reflect`. Mirrors the existing Tile Paint section precedent.
- **Light placement via add-component, not viewport click** (D-5) — the docked cursor-freeze makes true
  click-to-place unreliable; registering `PointLight` as an editor component makes "select entity → add
  light at its Transform" a one-click path that's also unit-testable.
- **Pure helper + device-guarded test for audio** (D-3) — `collect_bus_names` is a free fn so the logic
  has an always-running headless test; the `AudioManager::new()`-backed test guards with `else { return }`
  (skips on CI without a device), matching the existing audio tests.
- **`color_rgb_drags` vs `color_rgba_drags`** — lights ignore alpha, so the Point Light / Ambient colors
  expose only r/g/b; particle colors keep the alpha channel (used for fade-to-transparent).
- **Reset buttons preserve identity where it matters** — `reset_particle_emitter` keeps the texture;
  `reset_point_light` is a full default (lights have no comparable "asset" field).
- **Self-test = unit test through the real handler, no GUI playtest** — consistent with the parent's
  lesson; the cursor-freeze + display-sleep ceiling makes GUI smoke low-value for these.

## Evidence & Data

### Feature → version → PR → commit → tests

| Feat | Ver | PR | Merge commit | Headline | New tests | Lib total |
|---|---|---|---|---|---|---|
| D-2 | 8.20.0 | #52 | `7231184` | pathfinding-grid overlay | 1 | 578 |
| D-3 | 8.21.0 | #53 | `1da4a91` | audio bus mixer + `bus_names()` | 3 | 581 |
| D-4 | 8.22.0 | #54 | `d8c144b` | particle live-tuner | 2 | 583 |
| D-5 | 8.23.0 | #55 | `5b137e7` | lighting editor (PointLight+AmbientLight) | 3 | 586 |

### Commit log (newest first)

```
5b137e7 Merge #55 (D-5 lighting editor, 8.23.0)
e21bd9a feat(editor): lighting editor — PointLight + AmbientLight (8.23.0)
d8c144b Merge #54 (D-4 particle tuner, 8.22.0)
6441ab5 feat(editor): particle live-tuner inspector section (8.22.0)
1da4a91 Merge #53 (D-3 audio mixer, 8.21.0)
f7874e3 feat(editor): audio bus mixer panel + AudioManager::bus_names (8.21.0)
7231184 Merge #52 (D-2 pathfinding overlay, 8.20.0)
6cfb102 feat(editor): pathfinding-grid overlay (8.20.0)
13f1873 session: a-g editor-feature loop handoff [editor-tile-painting]  ← seq-2 handoff (loop start)
```

### Gate6 (run before EVERY commit — all green each cycle)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo build --target
wasm32-unknown-unknown` (lib+bins) · `cargo test --all-targets` · `RUSTDOCFLAGS="-D warnings" cargo doc
--no-deps` · `cargo package --locked --allow-dirty`. Final lib test count **586**.

### New tests by feature

| Test | Feature | Always runs? |
|---|---|---|
| `pathfinding_overlay_shades_blocked_and_walkable_cells` | D-2 | yes |
| `collect_bus_names_merges_sorts_and_dedups` | D-3 | yes (pure) |
| `collect_bus_names_empty_is_empty` | D-3 | yes (pure) |
| `bus_names_round_trips_through_audio_manager` | D-3 | **no — device-guarded** |
| `reset_particle_emitter_restores_defaults_keeping_texture` | D-4 | yes |
| `reset_particle_emitter_no_emitter_is_noop` | D-4 | yes |
| `reset_point_light_restores_defaults` | D-5 | yes |
| `ensure_ambient_light_inserts_default_once` | D-5 | yes |
| `pointlight_registered_as_editor_component` | D-5 | yes |

### Reusable engineering gotchas (hit this session)

- **`Box<dyn Fn>` (`ComponentFactory`) is NOT `Clone`.** To invoke a registered factory in a test/handler,
  use **disjoint field borrows**: `if let Some(f) = app.editor.component_factories.get(name) { f(&mut
  app.world, e); }` — `app.editor` and `app.world` are different fields of `app`, so the immutable map
  borrow + mutable world borrow coexist. `.clone()` triggers a `noop_method_call` clippy lint AND won't
  compile.
- **`ParticleEmitter` does not derive `Reflect`** — its `Option<Arc<str>>` texture + `pub(crate) timer`
  aren't `ReflectValue`-shaped. Use a dedicated inspector section, not the reflect grid.
- **`AudioManager` needs a real audio device** (`_stream: OutputStream` field) — `AudioManager::new()`
  returns `Option`; tests guard with `let Some(a) = AudioManager::new() else { return; }`. Extract pure
  logic (`collect_bus_names`) into a free fn for an always-running headless test.
- **`cargo fmt` reformats freshly-added test asserts** (e.g. wraps `assert!(app.world.get::<T>(e)
  .is_none())` across lines) — broke one `Edit` (string-not-found). Run `cargo fmt`, then **re-Read**
  the file before further edits to that region.
- **The `Edit` tool requires a fresh `Read` after a turn boundary or after fmt touches a file** — hit
  "File has not been read yet" / "File has been modified since read" on `Cargo.toml`, `CHANGELOG.md`,
  `docked.rs`, `editor.rs`. Re-Read the target slice before editing.
- **Nested egui closures capturing `&mut em` can conflict** — use flat *sequential* `ui.horizontal(|ui|
  …)` rows (each FnOnce closure reborrows disjoint fields and releases on return) instead of nesting
  `ui.horizontal` inside `ui.add(Grid…)` with the same `&mut`.
- **rust-analyzer phantoms persisted all session** (cfg-inactive, unlinked-file, `expected ColliderHandle
  found ColliderHandle` E0308, `missing field show_pathgrid` E0063 right after adding it, `never used`
  before wiring, `unused import` before the `super::` caller recompiles). ALL cleared by `cargo
  check`/`clippy`. Trust the compiler, not the IDE snapshot.
- **Local merge auto-closes the PR** — `git merge --no-ff` a pushed branch into `main` + push → GitHub
  marks the PR MERGED after ~2–3s; then `git branch -d` + `git push origin --delete`.

## Code Analysis

- **`App::draw_pathfinding_overlay`** (`editor.rs`): snapshot `Vec<Tilemap>` via `world.query::<Tilemap>()
  .map(|(_,tm)| tm.clone())` (release the world borrow before `DebugDraw`); per map build
  `PathGrid::from_tilemap(tm, |id| id != 0)`, iterate `0..grid.height × 0..grid.width`, `center =
  tm.cell_center_world(y,x)`, `half = tile_size*0.5`; walkable → `dbg.rect`, blocked → `rect_filled_z(.,.,.,0.0)`.
- **`AudioManager::bus_names`** (`src/audio/bus.rs`): `collect_bus_names(&self.channel_buses,
  &self.bus_volumes)` — pure fn: `channel_buses.values().cloned().chain(bus_volumes.keys().cloned())
  .collect()`, then `sort(); dedup()`. `bus_volumes`/`channel_buses` are private `HashMap`s; this is the
  only public enumerator.
- **`audio_mixer_panel_body`** (`audio_panel.rs`): `match world.resource::<AudioManager>()` → snapshot
  `Vec<(String,f32)>` of `(name, bus_volume(name))`; render `Slider::new(&mut v, 0.0..=1.0)
  .fixed_decimals(2)` per bus; collect changed `(name, v)`; apply via `world.resource_mut::<AudioManager>()
  .set_bus_volume(name, v)` (collect-then-apply — registry can't be borrowed mutably while iterating).
- **`particle_tuner_grid` / `color_rgba_drags`** (`docked.rs`): `world.get_mut::<ParticleEmitter>(sel)`;
  flat `ui.horizontal` rows of `DragValue` with per-field ranges (`spawn_rate 0..=2000`, `lifetime
  0..=60 s`, `size 0..=512`, `velocity_spread 0..=10000`). `color_rgba_drags` iterates `[("r",&mut c.r),
  ("g",&mut c.g),("b",&mut c.b),("a",&mut c.a)]` (disjoint field borrows in one array literal — legal).
- **`point_light_grid` / `color_rgb_drags` / `ambient_light_control`** (`docked.rs`): point light drags
  `radius 0..=4000`, `intensity 0..=10`, `light_height 0.01..=2.0`; ambient calls `app
  .ensure_ambient_light()` then `world.resource_mut::<AmbientLight>()` for color+intensity.
- **`App::reset_particle_emitter`**: `let texture = em.texture.clone(); *em = ParticleEmitter::default();
  em.texture = texture;`. **`App::reset_point_light`**: `*light = PointLight::default();`.
  **`App::ensure_ambient_light() -> bool`**: inserts `AmbientLight::default()` if absent, returns whether
  it inserted.
- **`PointLight`** (`components.rs`): cross-platform component `{ color: Color, radius: f32, intensity:
  f32, light_height: f32 }`, default `{ WHITE, 200.0, 1.0, 0.15 }`. **`AmbientLight`** (`resources.rs`):
  resource `{ color: Color, intensity: f32 }`, default `{ WHITE, 0.1 }`; `App::new` does NOT insert it.
- **`Color`** (`color.rs`): public `r/g/b/a: f32` + `to_array() -> [f32;4]`. **`Tilemap`** (`tilemap.rs`):
  `#[derive(Clone)]`, public `tiles`/`tile_size`/`origin`, `dims()`, `cell_center_world(row,col)`.
- **Bottom-panel dispatch** (`docked.rs`): `match app.editor.bottom_tab { 1 => data_table_panel_body,
  2 => audio_mixer_panel_body, _ => assets_tab_body }`.

## Files Changed

### Source — editor module (the surface touched repeatedly)
- `src/app/editor/state.rs` — `EditorState.show_pathgrid` field + `new()` init (D-2).
- `src/app/editor.rs` — `EditorSettings.show_pathgrid` (+`from_state`/`apply_to`); `App::{
  draw_pathfinding_overlay, reset_particle_emitter, reset_point_light, ensure_ambient_light}`;
  `register_default_components` now registers `PointLight` factory+remover; all 9 new tests in
  `editor_cmd_tests`.
- `src/app/editor/ui/docked.rs` — toolbar **Path** toggle (D-2); bottom-tab **Audio** + `match` dispatch
  (D-3); **Particle Tuner** + `particle_tuner_grid`/`color_rgba_drags` (D-4); **Point Light** + **Ambient
  Light** sections + `point_light_grid`/`color_rgb_drags`/`ambient_light_control` (D-5).
- `src/app/editor/ui/mod.rs` — per-frame `draw_pathfinding_overlay` call (D-2); `audio_panel` module +
  `audio_mixer_panel_body` re-export (D-3).
- `src/app/editor/ui/audio_panel.rs` — **NEW** (D-3): `audio_mixer_panel_body`.

### Source — engine
- `src/audio/bus.rs` — `AudioManager::bus_names()` + pure `collect_bus_names` + 3 tests (D-3).

### Docs / plans / version
- `Cargo.toml` (8.19.0 → 8.23.0 over four bumps), `CLAUDE.md` (header v1.6.24 → v1.6.28 / v8.23.0 +
  editor & audio module-map rows), `docs/CHANGELOG.md` (8.20.0–8.23.0).
- `plans/{pathfinding_overlay,audio_mixer,particle_tuner,lighting_editor}_plan.md` — per-feature plan +
  completion criteria.

## User Feedback & Preferences (REQUIRED — never omit)

- **The /loop mandate (issued once, user away the whole loop):** "d-2~d-5 진행 opus 판단으로 진행. gate6
  후 셀프 머지. 중간 보고 생략하고 완료시 일괄 보고. 셀프 테스트 가능하면 테스트 진행하고, 모든 작업 마무리
  되면 handoff 스킬 사용해서 문서화 하고 푸시까지 진행." → autonomous, opus-ordered, **merge-authorized for
  this session**, mid-reports suppressed, self-test where possible, `/handoff` + push at the end.
- **Pre-loop:** the user **rejected the first `AskUserQuestion`** to add context ("clarify these
  questions"), then asked **"루프 진행 할건데 작업 할 내용 정리해서 알려줘"** (organize the work list before
  looping). Lesson: present the scoped plan as prose for confirmation before kicking off a loop.
- **Scope chosen:** D-2~D-5 (the four feasible D editors); **self-merge** (option ⓐ), not PR-only;
  mid-reports suppressed.
- **Standing (from memory):** Korean prose to the user, English code/docs/handoff; subagents on Sonnet
  (set explicit model — `claude-fable-5` dies as a subagent); never drop `CLAUDE.md` content to hit the
  ≤200-line limit (move detail to `docs/*.md`).
- **Earlier this arc (carried):** "opus가 세부 계획 세우고 완료 시점까지 명시 하여 시작" — write a detailed
  plan + completion criteria FIRST, then implement (the per-feature `plans/*_plan.md` pattern, honored).

## Where We're Going

The four feasible D editors are done. Remaining (commission separately if wanted):

1. **D — LARGE (separate multi-session arcs):** state-machine graph editor (`AnimationStateMachine`
   nodes/transitions), timeline editor (`Timeline`/`Track`/`Keyframe`). Heavily visual, weak autonomous
   visual validation — the kind deferred at the parent's milestone. Need a different validation strategy.
2. **Parent leftovers:** collider sync while painting (`sync_static_from_tilemap` on tile edits), RTL /
   per-locale fonts, **rust-survivors v8.x pin bump + smoke** (engine moved 8.11 → 8.23; all additive →
   bump the git-rev pin and smoke-test the game).
3. **Optional polish on the shipped D editors:** A* path *preview* (click start/goal → draw route) on the
   pathfinding overlay; per-channel volume / mute-solo on the audio mixer; "save tuned config to
   `ParticleConfigSet` RON" on the particle tuner; radius gizmo handle on point lights.

## Risks & Blockers

- **None blocking.** `main` green + clean at `5b137e7` (v8.23.0).
- **Synthetic-input playtest ceiling (recurring, unchanged):** macOS CGEvent clicks can't reliably move
  the docked editor's frozen viewport cursor; precise viewport interaction isn't reproducible. Validate
  editor logic by unit test; GUI confirms only that UI *renders*. This is why no GUI smoke ran this loop.
- **rust-survivors pin is now 12 minor versions behind** (8.11 → 8.23) — all additive, so low-risk, but
  the bump + smoke is unverified work waiting in the wings.

## Open Questions

- Should the large D editors (state-machine graph, timeline) be their own multi-session arcs with a
  different validation strategy (golden-file serialization round-trips instead of GUI), since autonomous
  visual validation is weak?
- Is the "place light via add-component" UX acceptable long-term, or is a viewport click-to-place worth
  solving the cursor-freeze for (would benefit lights, particles, and prefab spawning alike)?

## Quick Start for Next Session

```bash
# No beads in this repo.
# Reference: CLAUDE.md (Gate6), docs/VISION.md, plans/*_plan.md (per-feature plans).

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 5b137e7 (v8.23.0)
cargo test --lib                # 586 pass
./scripts/verify.sh             # full Gate6

# Key files to read first (not exhaustive — explore adjacent code too)
#   src/app/editor.rs                 (App editor methods + EditorSettings + editor_cmd_tests — all 9 new tests)
#   src/app/editor/ui/docked.rs       (toolbar + inspector sections + panel dispatch + tuner/light helpers)
#   src/app/editor/ui/audio_panel.rs  (D-3 mixer panel — newest module)
#   src/audio/bus.rs                  (AudioManager::bus_names + collect_bus_names)
#   src/app/editor/state.rs           (EditorState fields)

# Playtest tooling (macOS, display must be awake): /tmp/tile_paint_playtest/input
#   F2 keycode = 120; cursor freezes outside the central viewport rect.

# Next action — pick ONE:
#   (a) rust-survivors v8.x pin bump + smoke (lowest-risk, all engine changes additive), OR
#   (b) commission a LARGE D editor (state-machine graph / timeline) as its own arc with a
#       serialization-round-trip validation strategy (GUI validation is weak autonomously), OR
#   (c) parent leftovers: collider sync while painting / RTL fonts.
#   For ANY editor feature: write plans/<name>_plan.md (criteria) → implement → Gate6 → unit-test
#   through the real handler → PR → merge. Merge authority was THIS-session-scoped; re-confirm before
#   self-merging in a NEW session.
```

## Session Closed
**Closed at:** 2026-06-15
**Commit:** `5b137e7` (v8.23.0, D-5) + this handoff's `session:` commit
**Session status:** Handed off — D-2→D-5 editor loop concluded at 4 features (all commissioned work shipped)
