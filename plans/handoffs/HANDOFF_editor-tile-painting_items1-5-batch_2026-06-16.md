# Remaining-work batch (items 1–5): collider sync, RTL, SM + timeline editors (v8.24→8.27)

**Date:** 2026-06-16
**Status:** COMPLETED (items 2–5 shipped + merged; item 1 DROPPED — see post-session note)

> **Post-session update (2026-06-16):** the user decided **rust-survivors is no longer maintained
> against the engine** (too far diverged since the engine's early days). Item 1 (the pin bump) is
> **dropped, not pending** — do NOT pick it up. The "What We Tried / Where We Are / Evidence" sections
> below record the in-session assessment accurately; the forward sections were updated to reflect the
> drop. See `[[rust-survivors-deprecated]]` memory; CLAUDE.md "Related projects" + the
> `RUST_SURVIVORS_*_PROMPT.md` docs were removed.
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine editor authoring tools + engine breadth
**Chain:** `editor-tile-painting` seq `4`
**Parent:** `HANDOFF_editor-tile-painting_d2-d5-loop_2026-06-15.md`
**Prior chain:** `HANDOFF_editor-tile-painting_v8.11-shipped_2026-06-15.md` (seq 1) > `..._a-g-editor-loop_2026-06-15.md` (seq 2) > `..._d2-d5-loop_2026-06-15.md` (seq 3) > this (seq 4)

## Stale References

All parent (seq 3) identifiers still valid — none removed this session (`draw_pathfinding_overlay`,
`AudioManager::bus_names`, `reset_particle_emitter`, `reset_point_light`, `ensure_ambient_light`,
`PointLight` editor registration, `EditorSettings.show_pathgrid`). This session only ADDED API.

## Since Last Handoff

Parent (seq 3) concluded the D-2→D-5 subsystem-editor loop and listed remaining work under "Where We're
Going": (1) the two LARGE editors (state-machine graph, timeline); (2) parent leftovers (collider sync
while painting, RTL fonts, rust-survivors v8 pin bump).

- The user said **"1, 2 남은 작업 둘다 진행"** (do both remaining groups). I re-scoped into **5 sequential
  items**: 1 = rust-survivors pin bump, 2 = collider sync, 3 = RTL fonts, 4 = state-machine editor,
  5 = timeline editor — and confirmed operating mode (self-merge) + order (1–3 then 4–5) via AskUserQuestion.
- **Items 2–5 all shipped, Gate6-green, self-merged (#56–#59, v8.24–8.27).** Item 1 **PARKED**.
- **The parent's open question — "should the large editors be their own arcs with serialization-round-trip
  validation?" — was answered in practice:** built each large editor as a **list-based MVP** with a
  unit-testable accessor/edit-op core (visual node-graph / time-ruler deferred), because the docked
  cursor-freeze makes autonomous *visual* validation weak. This matched the parent's suggested strategy.
- **Trajectory shift vs the parent's autonomous-loop:** this session was **interactive** (the user was
  present, answered 3 AskUserQuestions) yet still self-paced via `ScheduleWakeup` between items, under the
  same self-merge + report-at-milestones mode the user re-granted for this batch.

## Reference Documents

- `CLAUDE.md` — conventions + Gate6 "Verification" checklist. Header now v8.27.0 (v1.6.32).
- `docs/VISION.md` — "a feature is not done until a playable example exercises it" (items 2–5 are each
  exercised by an existing example: `dig_quest`, `rtl_text` (new), `sm_crossfade`, `timeline_cutscene`).
- `plans/{collider_sync,rtl_fonts,state_machine_editor,timeline_editor}_plan.md` — per-feature plans.
- `docs/CHANGELOG.md` — 8.24.0 → 8.27.0.

## The Goal

Clear the parent handoff's remaining-work list — keep the engine + docked editor broadening, each feature
additive (semver-minor, `rust-survivors` unaffected) and validated through real code paths. The user
delegated ordering + merge to opus judgment, suppressed mid-reports (milestone reports only), and asked
for a `/handoff` + push when the batch finished.

## Where We Are

- **`main` = `2ee9cee` (v8.27.0)**, clean tree. 4 feature PRs (#56–#59) merged + branches deleted.
- **Item 1 — rust-survivors pin bump: PARKED.** `rust-survivors` HEAD (`960b43c`) pins engine **7.0.0**
  (`rev a3369ee`), and its tree has **37 files of uncommitted WIP** (incl. `ENGINE_MIGRATION_NOTES.md`
  with an "Applied Status 2026-06-15" v7.0.0 section, `Cargo.toml`/`Cargo.lock`, and source files
  `crates/game/src/survivor/{data,character,bgm,stage}.rs` + `main.rs`). A 7.0.0→8.27.0 bump crosses the
  v8.0.0 BREAKING window — but a read-only grep found **zero direct use** of the v8.0.0-breaking APIs, so
  it's near-clean (pin + `Cargo.lock`, maybe the `engine_reflect_derive` dev-dep). Parked to avoid
  entangling the bump with the user's uncommitted work; "push is the user's" standing rule.
- **Item 2 — collider sync (v8.24.0, #56):** `TilemapColliders` opt-in component + `SolidTiles` rule +
  `sync_tilemap_entity_colliders` free fn + `App::sync_tilemap_colliders`. Editor Tile Paint resyncs on
  stroke + undo/redo. `dig_quest` refactored onto it. +4 tests.
- **Item 3 — RTL fonts (v8.25.0, #57):** `ExtraFonts` multi-font resource + `TextAlign::Auto`/`End` +
  bundled OFL Noto Sans Hebrew + `rtl_text` example. **Key finding: bidi shaping already worked.** +3 tests.
- **Item 4 — state-machine editor MVP (v8.26.0, #58):** `AnimationStateMachine` accessors + edit ops +
  docked **State Machine** list panel. +6 tests.
- **Item 5 — timeline editor MVP (v8.27.0, #59):** `Track<T>` keyframe accessors + edit ops + docked
  **Timeline** panel (playback + per-track keyframe list). +4 tests.
- **Tests: 603 lib tests pass** (was 586 at session start; **+17**). `cargo test --all-targets` clean each cycle.
- **`rust-survivors` impact:** none from items 2–5 (all additive: new components/resources/methods/enum
  variants; nothing removed). The only `rust-survivors` action is the parked pin bump itself.

## What We Tried (Chronological)

1. **Item 1 assessment (read-only).** Opened `rust-survivors`; found HEAD pins **7.0.0** (not 8.x) and a
   dirty tree (37 WIP files). Grepped for the v8.0.0 breaking surface (`AnimationSystem {`/`EntityDef {`/
   `TextInput {`/`Slider {` literals, particle texture, `Reflect` derive) — **all clean**. Surfaced that
   reality contradicts the "additive, low-risk" framing → **parked** rather than mutate uncommitted WIP.
2. **Item 2 collider sync (#56).** Read `sync_static_from_tilemap` + `dig_quest`'s hand-rolled sync
   (`tile_index` resource + `with_resource_mut::<PhysicsWorld>` + `|v| (v!=0).then(TileCollider::solid)`).
   Extracted that exact pattern into a `TilemapColliders` component + free fn, wired the editor paint
   path, and **refactored dig_quest onto the new API** (VISION: API from real usage). 586→590.
3. **Item 2 wasm gotcha.** `App::sync_tilemap_colliders` sat in a non-gated `impl App` block but called
   `crate::physics` (native-only) → **E0433 on wasm**. Added `#[cfg(not(target_arch = "wasm32"))]` to the
   method (its only call site, `commit_paint_stroke`, is already editor/native-gated). Re-ran wasm → green.
4. **Item 3 RTL discovery.** Explored `src/renderer/text.rs`: the renderer uses **`Shaping::Advanced`**
   (cosmic-text + rustybuzz) → **bidi/RTL shaping already works**. Reframed "RTL fonts" as font *coverage*
   + *alignment*, and checkpointed scope via AskUserQuestion (premise was wrong + needs a binary asset).
   User chose the **full deliverable** (incl. bundled font + example).
5. **Item 3 build (#57).** Added `TextAlign::Auto`/`End` (`to_glyphon → Option<Align>`), `ExtraFonts`
   resource (loaded in `window.rs`; `build_font_system` extracted for a headless test), downloaded an OFL
   **Noto Sans Hebrew** (16KB) + OFL license into `assets/fonts/`, wrote `examples/rtl_text.rs`. 590→593.
6. **Item 4 state-machine editor MVP (#58).** `AnimationStateMachine`'s `states`/`current`/`params` were
   **private with no read accessors** → added `state_names`/`state`/`state_count`/`param_names`/`param`
   + edit ops `set_current_state`/`set_state_clip`/`remove_state`/`remove_transition`, then a docked list
   panel. Shipped list MVP; deferred visual node-graph. 593→599.
7. **Item 5 timeline editor MVP (#59).** Same shape: `Track<T>`'s `keyframes` was **private** → added
   `keyframes`/`len`/`remove`/`set_time`/`clear`, then a docked `Timeline` panel (playback + generic
   per-track keyframe list `timeline_track_ui<T>`). Shipped list MVP; deferred visual time-ruler. 599→603.
8. **Batch close.** All 5 items resolved → this `/handoff` (seq 4) + push.

## Key Decisions

- **Item 1 parked, not forced.** The pin bump was framed as "additive, low-risk"; reality (pinned at
  7.0.0 + 37 files of uncommitted WIP incl. active migration docs) contradicted that. Modifying the
  user's dirty tree across a breaking boundary would entangle my changes with theirs and violate "push is
  the user's." Assessed read-only, parked, surfaced. **Rejected:** doing the bump on top of the WIP.
- **Large editors as list-based MVPs, not visual graphs.** A drawn node-graph (SM) / time-ruler (timeline)
  with draggable nodes/keyframes is unvalidatable autonomously (docked cursor-freeze blocks the drag).
  Built a serializable-by-construction, **unit-testable accessor/edit-op core** + a functional list panel;
  the visual layer is a noted follow-up. This is exactly the parent's suggested validation strategy.
- **RTL = coverage + alignment, not from-scratch shaping.** Discovered `Shaping::Advanced` already shapes
  RTL; the deliverable became `ExtraFonts` (script fallback) + `TextAlign::Auto`/`End`. **Checkpointed the
  scope** (AskUserQuestion) because the premise was wrong AND it needs a binary font asset decision.
- **API extracted from real usage (dig_quest → TilemapColliders).** `dig_quest`'s sync rule mapped 1:1 to
  `SolidTiles::NonZero` + `PPU`, so refactoring it onto the new component both proved the API in real play
  and deleted hand-rolled code — the VISION ideal.
- **Hebrew over Arabic for the bundled RTL font.** Smaller (16KB vs Arabic's larger glyph set) and still
  demonstrates RTL direction + alignment; Arabic works identically once a game supplies a Noto Arabic blob.
- **`remove_state` refuses the active/last state and prunes inbound transitions** — keeps the machine valid
  (can't orphan `current`, can't empty it, no dangling edges).

## Evidence & Data

### Item → version → PR → merge commit → tests

| Item | Ver | PR | Merge | Headline | New tests | Lib total |
|---|---|---|---|---|---|---|
| 1 rust-survivors | — | — | — | **PARKED** (user WIP; near-clean bump) | — | — |
| 2 collider sync | 8.24.0 | #56 | `26afc69` | `TilemapColliders` + editor/runtime sync | 4 | 590 |
| 3 RTL fonts | 8.25.0 | #57 | `1e9f59b` | `ExtraFonts` + `Auto`/`End` + Noto Hebrew | 3 | 593 |
| 4 SM editor MVP | 8.26.0 | #58 | `c4a4866` | `AnimationStateMachine` API + list panel | 6 | 599 |
| 5 timeline editor MVP | 8.27.0 | #59 | `2ee9cee` | `Track` keyframe API + Timeline panel | 4 | 603 |

### Commit log (newest first)

```
2ee9cee Merge #59 (item 5 timeline editor MVP, 8.27.0)
6bbd7fc feat(editor): timeline editor MVP + Track keyframe API (8.27.0)
c4a4866 Merge #58 (item 4 SM editor MVP, 8.26.0)
e8cba6b feat(editor): state-machine editor MVP + AnimationStateMachine API (8.26.0)
1e9f59b Merge #57 (item 3 RTL fonts, 8.25.0)
3c0ead5 feat(text): RTL support — multi-font loading + reading-direction alignment (8.25.0)
26afc69 Merge #56 (item 2 collider sync, 8.24.0)
c75b5bc feat(physics): TilemapColliders + collider sync on tile mutation (8.24.0)
16f50cd session: d-2~d-5 editor-loop handoff [editor-tile-painting]  ← seq-3 handoff (this session's start)
```

### Gate6 (run before EVERY commit — all green each cycle)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo build --target
wasm32-unknown-unknown` (lib+bins) · `cargo test --all-targets` · `RUSTDOCFLAGS="-D warnings" cargo doc
--no-deps` · `cargo package --locked --allow-dirty`. Final lib test count **603**.

### New tests by item

| Item | Tests |
|---|---|
| 2 | `solid_tiles_rules`, `tilemap_colliders_sync_adds_and_removes`, `sync_tilemap_entity_colliders_free_fn`, `sync_tilemap_entity_colliders_missing_component_is_false_path` |
| 3 | `text_align_to_glyphon_mapping`, `build_font_system_loads_extra_fonts` (loads the real bundled Hebrew font → +1 db face), `build_font_system_skips_empty_blobs` |
| 4 | `state_accessors_report_states_and_transitions`, `set_current_and_set_clip`, `remove_state_prunes_inbound_transitions`, `remove_state_refuses_last_state`, `remove_transition_by_index`, `param_accessors` |
| 5 | `track_keyframes_accessor_is_sorted`, `track_remove_keyframe`, `track_set_time_resorts`, `track_clear` |

### Reusable engineering gotchas (this session)

- **`physics` is native-only (`#[cfg(not(target_arch = "wasm32"))]`).** Any `App`/free-fn in a *non-gated*
  module that references `crate::physics` must itself be cfg-gated, or the **wasm lib build fails E0433**
  ("cannot find `physics` in `crate`"). The native build + native tests pass — only the wasm gate catches
  it. (Hit on `App::sync_tilemap_colliders`.)
- **`cargo fmt` reformats freshly-added test asserts** (wraps long `assert!(...)`), which then breaks a
  follow-up `Edit` ("String to replace not found"). After `cargo fmt`, **re-Read** the region before the
  next Edit. Recurs every cycle.
- **A bundled binary asset must be `git add`-ed before `cargo package`** includes it. The Hebrew font +
  OFL license were staged before the package gate (the existing `DejaVuSans.ttf` is referenced by
  `DEFAULT_FONT`'s `include_bytes!`, so `assets/fonts/` is in the package).
- **`Box<dyn Fn>` is not `Clone`** (carried) — invoke a registered closure via *disjoint field borrows*.
- **rust-analyzer phantoms persisted** (`expected ColliderHandle, found ColliderHandle` E0308; `missing
  field` E0063 right after adding it; `cannot find type` at lines I'd already edited). ALL cleared by
  `cargo check`. Trust the compiler, not the IDE snapshot.
- **cosmic-text already shapes bidi/RTL** with `Shaping::Advanced` — don't rebuild shaping; the gap is
  font coverage (`ExtraFonts`) + reading-direction alignment (`Align::End` / `set_align(None)`).
- **An existing `crate::locale::TextDirection` enum exists** (discovered during item 3, not used) — relevant
  if a future per-locale-font feature wants to branch on direction.

## Code Analysis

- **`TilemapColliders`** (`src/physics/world/tile_collider.rs`): `{ pixels_per_unit: f32, solid:
  SolidTiles, index: TileColliderIndex (private) }`; `new(ppu, solid)`, `sync(&mut PhysicsWorld,
  &Tilemap)` (calls `sync_static_from_tilemap(tm, ppu, |id| self.solid.collider_for(id), &mut self.index)`
  — disjoint `self.solid`/`self.index` borrows), `collider_count()`. **`SolidTiles`**: `NonZero` |
  `Only(Vec<u32>)`; `collider_for(id) -> Option<TileCollider>` (`solid.then(TileCollider::solid)`).
- **`sync_tilemap_entity_colliders(world, entity) -> bool`** (free fn): clone `Tilemap` out, then
  `world.with_resource_mut::<PhysicsWorld>(|physics, world| if let Some(tc) =
  world.get_mut::<TilemapColliders>(entity) { tc.sync(physics, &tm) })`. Returns the `with_resource_mut`
  bool (whether `PhysicsWorld` was present). **`App::sync_tilemap_colliders`** = thin native-only wrapper.
- **`TextAlign`** (`src/renderer/text.rs`): added `End` (→ `cosmic_text::Align::End`) + `Auto` (→ `None`).
  `to_glyphon(self) -> Option<Align>`; render: `line.set_align(d.align.to_glyphon())`. **`build_font_system(
  font_data, extra_fonts) -> FontSystem`**: `FontSystem::new()` + `db_mut().load_font_data` for the main
  blob and each non-empty extra. **`ExtraFonts(Vec<Vec<u8>>)`** (`resources.rs`) read in `window.rs`.
- **`AnimationStateMachine`** (`src/animation/state_machine.rs`): private `states: HashMap<String,
  AnimState>` / `current: String` / `params`. New: `state_names()` (sorted), `state(name) ->
  Option<&AnimState>`, `state_count()`, `param_names()`, `param(name)`, `set_current_state`,
  `set_state_clip`, `remove_state` (refuses `==current` / `len()<=1` / missing; prunes inbound
  transitions via `retain(|t| t.to != name)`), `remove_transition(from, index)`. `AnimState.transitions`
  + `AnimTransition.{to,conditions,crossfade_duration}` already public.
- **`Track<T: Clone + Lerp>`** (`src/timeline.rs`): private `keyframes: Vec<Keyframe<T>>`. New:
  `keyframes() -> &[Keyframe<T>]`, `len()`, `remove(index) -> Option<Keyframe<T>>`, `set_time(index, time)`
  (mutate + `sort_by(total_cmp)`), `clear()`. `Timeline` is a component with public tracks
  `position/rotation/scale/color/alpha/zoom` + `duration/time/looping/playing`.
- **Editor panels** (`src/app/editor/ui/docked.rs`): `state_machine_panel` + `timeline_panel` follow the
  snapshot→collect-intents→apply (SM) / single-`get_mut`-disjoint-fields (timeline) borrow patterns.
  `timeline_track_ui<T>(ui, label, &mut Track<T>, fmt: impl Fn(&T)->String)` is generic over all 6 tracks.

## Files Changed

### Item 2 — collider sync
- `src/physics/world/tile_collider.rs` — `TilemapColliders` + `SolidTiles` + `sync_tilemap_entity_colliders`
  + 4 tests. `src/physics/{world.rs,mod.rs}` + `src/lib.rs` — exports. `src/app/editor.rs` —
  `App::sync_tilemap_colliders` (cfg-gated) + PaintTiles undo/redo re-sync. `src/app/editor/ui/gizmo.rs` —
  `commit_paint_stroke` sync hook. `examples/games/dig_quest/dig_quest.rs` — refactor onto the API.

### Item 3 — RTL fonts
- `src/renderer/text.rs` — `TextAlign::Auto`/`End`, `to_glyphon → Option`, `build_font_system` + 3 tests.
  `src/resources.rs` — `ExtraFonts`. `src/app/window.rs` — load `ExtraFonts`. `src/lib.rs` — export.
  `examples/rtl_text.rs` (NEW). `assets/fonts/NotoSansHebrew-Regular.ttf` + `NotoSansHebrew-OFL.txt` (NEW).

### Item 4 — SM editor
- `src/animation/state_machine.rs` — accessors + edit ops + 6 tests. `src/app/editor/ui/docked.rs` —
  State Machine section + `state_machine_panel`/`cond_summary`/`param_display`. `src/app/editor/state.rs` —
  `sm_add_state_name`.

### Item 5 — timeline editor
- `src/timeline.rs` — `Track` accessors + edit ops + 4 tests. `src/app/editor/ui/docked.rs` — Timeline
  section + `timeline_panel`/`timeline_track_ui`.

### Docs / plans / version
- `Cargo.toml` (8.23.0 → 8.27.0 over 4 bumps), `CLAUDE.md` (header v1.6.28 → v1.6.32 + module-map rows),
  `docs/CHANGELOG.md` (8.24.0–8.27.0), `plans/{collider_sync,rtl_fonts,state_machine_editor,timeline_editor}_plan.md`.

## User Feedback & Preferences (REQUIRED — never omit)

- **"1, 2 남은 작업 둘다 진행"** — do both remaining work groups from the seq-3 handoff (large editors +
  parent leftovers). I re-scoped into 5 sequential items + confirmed.
- **Operating mode (AskUserQuestion): "d-2~d-5와 동일 (셀프 머지)"** — self-merge after Gate6, report at
  milestones, autonomous. **Order: "1–3 먼저, 그 다음 4–5".**
- **RTL scope (AskUserQuestion): "RTL 폰트 자산까지 번들링 (예제 완성)"** — the FULL deliverable including a
  bundled OFL RTL font asset + a complete example (not just the alignment/multi-font infrastructure).
- The user **engaged with checkpoints** (answered 3 AskUserQuestions, didn't reject them) — so a tight,
  recommended-default AskUserQuestion at a genuine fork is welcomed, not friction. (Contrast: in the
  pre-loop discussion of the prior session they rejected an over-broad question.)
- **Standing:** Korean prose to the user, English code/docs/handoff; **`rust-survivors` push is the
  user's** (local changes only); never drop `CLAUDE.md` content to hit ≤200 lines; subagents on Sonnet
  with explicit `model:` ([[new-model-subagent-incompat]]).

## Where We're Going

1. **rust-survivors pin bump — DROPPED (do not do).** Post-session the user decided rust-survivors is no
   longer maintained against the engine. Engine changes are validated on their own (Gate6 + in-repo
   `examples/`); rust-survivors is out of scope. See `[[rust-survivors-deprecated]]`.
2. **Large-editor iteration 2 (visual):** SM **node-graph** rendering (positioned state boxes + drawn
   transition edges, current highlighted) and timeline **time-ruler** (horizontal scale + draggable
   keyframe dots + playhead). Both are egui-painter work; drag-editing stays weakly validatable
   autonomously (cursor-freeze) — pair with unit-tested layout/hit-test helpers.
3. **Editor depth:** add-transition + condition editing in the SM panel; add-keyframe + value/easing
   editing in the Timeline panel; live parameter editing.
4. **Persistence:** serde for `AnimationStateMachine` / `Timeline` so editor edits survive scene save/load
   (currently neither is serde-registered).
5. **Per-locale fonts:** auto-select a font from the active `LocaleResource` (the deferred RTL option C;
   note the existing `crate::locale::TextDirection`).

## Risks & Blockers

- **None blocking.** `main` green + clean at `2ee9cee` (v8.27.0).
- **rust-survivors dropped** (post-session) — no longer a gate or a task; out of the engine's scope.
- **Autonomous visual-validation ceiling (recurring):** the docked cursor-freeze blocks reliable
  drag-editing playtests, so the SM/timeline *visual* editors can't be autonomously verified — hence the
  list-based MVPs + unit-tested cores. A human eyeball of the panels (and `cargo run --example rtl_text`
  for RTL rendering) is worth doing with display access.

## Open Questions

- For the visual SM node-graph / timeline ruler: store node/keyframe-dot positions where? (A new editor-
  only layout component, auto-layout each frame, or positions on the data model?) Auto-layout avoids new
  serde state but loses manual arrangement.
- Should `AnimationStateMachine` / `Timeline` gain serde + `register_serde_component` so the new editor
  edits persist in scene saves (today they're runtime-only, lost on save/load)?

## Quick Start for Next Session

```bash
# No beads in this repo.
# Reference: CLAUDE.md (Gate6), docs/VISION.md, plans/*_plan.md (per-feature plans).

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 2ee9cee (v8.27.0)
cargo test --lib                # 603 pass
./scripts/verify.sh             # full Gate6

# Key files to read first (not exhaustive — explore adjacent code too)
#   src/app/editor/ui/docked.rs        (state_machine_panel + timeline_panel + the 6 inspector sections)
#   src/animation/state_machine.rs     (SM accessors + edit ops + tests)
#   src/timeline.rs                    (Track keyframe accessors + edit ops + tests)
#   src/physics/world/tile_collider.rs (TilemapColliders + sync_tilemap_entity_colliders)
#   src/renderer/text.rs               (TextAlign::Auto/End, ExtraFonts, build_font_system)

# rust-survivors: DROPPED — no longer maintained against the engine (do not bump/sync). See
#   the [[rust-survivors-deprecated]] memory.

# Next action — pick ONE:
#   (a) SM node-graph / timeline time-ruler visual editors (iteration 2), OR
#   (b) SM/Timeline serde persistence so editor edits survive scene save/load, OR
#   (c) a new feature + example per docs/VISION.md.
#   For ANY engine/editor feature: plans/<name>_plan.md (criteria) → implement → Gate6 → unit-test
#   through the real handler → example → PR → merge. Merge authority was THIS-batch-scoped; re-confirm
#   before self-merging in a NEW session.
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** `2ee9cee` (v8.27.0, item 5) + this handoff's `session:` commit
**Session status:** Handed off — items 2–5 shipped (v8.24–8.27), item 1 (rust-survivors) parked on user WIP.
