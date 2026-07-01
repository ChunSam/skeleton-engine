# Game-feel + UI breadth run — 5 shipped features (FloatingText, InputBuffer, SpriteTrail, docked-split, ProgressBar)

**Date:** 2026-07-02
**Status:** COMPLETED (5 PRs shipped + merged; tree clean, all green)
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `game-feel-ui-breadth` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** none — first in chain

---

## Related Handoffs

- `HANDOFF_editor-ux_scene-tree-menu-doc-audit_2026-07-01.md` — editor-ux seq 9. **This session opened from that paste prompt** but did NOT continue editor-ux: seq 9 declared editor-UX self-pick breadth *fully exhausted* and said "next work needs an external signal or a user-specified direction — ASK, don't grind." This session did exactly that (asked, user picked new work), so it starts a **new chain**, not an editor-ux continuation. Seq 9 is reference-only.

## Reference Documents

- `CLAUDE.md` — project agent reference (module map + verify gate + conventions); edited by every PR this session (module-map rows + header version). Now at exactly 200 lines, header **v1.6.196**, package **v0.107.0**.
- `docs/VISION.md` — the feature+example loop this session ran 4×: "a feature is not done until a small playable example exercises it; the example is the acceptance test; if the API feels awkward while writing the example, fix the API first."
- `docs/CHANGELOG.md` — entries 0.104.0 → 0.107.0 (this session).
- Memory: `engine-current-state.md` (tip = seq 141, `30d9d8b`), `MEMORY.md` index line, `egui-editor-emoji-glyph-set.md` (glyph tofu gotcha, still relevant), `merge-authority-delegated.md` (squash on green CI, no re-confirm — used 5× this session).

## The Goal

Keep extending the engine's **breadth** (a hackable, fork-friendly, genre-agnostic MIT 2D skeleton per `docs/VISION.md`) now that the editor-UX self-pick set is complete and the Dungeon-Merchant wishlist board is empty. When the board is empty, the agreed protocol (from editor-ux seq 8/9) is to **ASK the user for a direction** rather than grind marginal tweaks. This session did that: the user chose "new engine feature + example," then a batch of specific features, then accepted a recommendation. End state: 4 new public capabilities (3 game-feel primitives + 1 UI widget) + 1 supporting refactor, each shipped through the full VISION loop and merged.

## Where We Are

- **main @ `30d9d8b`** (package **v0.107.0**, CLAUDE.md header **v1.6.196**), tree clean, all green.
- **5 PRs merged this session** (all squash-merged on green CI per delegated authority):
  - **#316 v0.104.0** — `FloatingText` + `FloatingTextSystem` (rising/fading combat text). `src/floating_text.rs`.
  - **#317 v0.105.0** — `InputBuffer` (buffered jump + coyote time), a pure-logic helper. `src/input_buffer.rs`.
  - **#318 v0.106.0** — `SpriteTrail` + `SpriteTrailGhost` + `SpriteTrailSystem` (motion afterimage). `src/sprite_trail.rs`.
  - **#319 v0.106.1** — behavior-preserving split of `docked.rs` (1619 lines) → `docked/` (9 files). Refactor.
  - **#320 v0.107.0** — `ProgressBar` read-only UI gauge widget. `src/ui/progress_bar.rs` + `src/ui/system/progress_bar_pass.rs`.
- **Memory seqs 137→141** (global engine-history counter): 137 FloatingText, 138 InputBuffer, 139 SpriteTrail, 140 docked-split, 141 ProgressBar.
- **New lib unit tests this session: +26** (FloatingText 6, InputBuffer 9, SpriteTrail 6, ProgressBar 5) — docked-split added 0 (tests moved verbatim). Plus **7 new doctests** (2+2+2+1).
- **Every feature verified headlessly** — a `HEADLESS_SHOT=path` capture rendered the real path with no display (the GUI playtest is blocked on this locked/remote macOS box). All 4 feature captures visually confirmed the effect.
- **Render tests unchanged at 11** — no new render tests added; the docked split's existing docked render tests (`editor_docked_renders_headless` + rename/reparent/inline-rename variants) stayed green, proving the split preserved behavior.
- **The whole session ran the same land-loop per PR:** branch → `cargo fmt` → verify 7 gates → `/ship` paperwork → commit → push → PR → `gh pr checks --watch` (background) → CI 5/5 → squash-merge → `git pull --ff-only` + prune → memory seq bump.
- **2 audio tests are `--skip`ped locally every run** (`play_tone_reports_playing_then_finished_when_audio_device_exists`, `stop_on_drained_sink_is_immediate`) — no audio device on this box; CI gates them. Standing pattern.
- **The docked split was authored/verified across the 2026-07-01→07-02 midnight boundary** (the date rolled during that PR); no impact.

## What We Tried (Chronological)

1. **Onboarding + board check.** Read editor-ux seq 9 handoff. Read `../dungeon-merchant/docs/engine-wishlist.md` → **ACTIVE EMPTY (EW-004)**, last edit 2026-06-25 (unrelated). Per protocol, used `AskUserQuestion` for direction (options: new feature+example / new example game / refactor / stop). User chose **"새 엔진 기능 + 예제"**.
2. **Verified candidate gaps before proposing.** Grepped `src/` to confirm which game-feel/UI ideas were genuinely unimplemented: camera-shake ALREADY exists (`Camera::shake(strength, duration)`, `shake_offset()`) → excluded. Confirmed gaps: floating combat text, input buffering+coyote, UI progress bar, sprite trail. Presented 4 options; user chose **플로팅 데미지/전투 텍스트**.
3. **#316 FloatingText.** Modeled on `HitFlash` (transient game-feel component + user-added system, no serde, clone-registered). `FloatingText{text,color,velocity,size,lifetime,fade}` + private `elapsed`; `FloatingTextSystem` ages/drifts(default −Y rise)/fades-alpha/draws via `TextQueue`+`Camera::world_to_screen` (screen-space fallback if no Camera)/**despawns the whole entity** on expiry. Free `spawn_floating_text(world,pos,ft)` + `App::spawn_floating_text{,_colored}`. **Key call:** clone-registered but NOT editor-add (it despawns its own entity → a footgun if hand-authored on a kept entity; precedent: `Timer`). Example `floating_text`: Space pops numbers over 3 targets, click at cursor, headless auto-fires. 6 unit + 2 doctests. 7/7 gates, CI 5/5, merged.
4. **User asked for the next-work list.** Provided a categorized list (A game-feel primitives / B UI widgets / C big subsystems / D refactor / E new example game) with verified-gap status.
5. **User said "2, 3 진행"** = do priority-2 (A-group game-feel) + priority-3 (D refactor). Ran a **3-PR batch**:
   - **#317 InputBuffer.** Pure-logic helper (driven like `Timer`/`Tween`, NOT a system/component). Input buffering + coyote time. **Design iteration:** first draft used a "remaining buffer" representation with tick-BEFORE-consume — realized a `buffer_secs=0` / `coyote_secs=0` config would silently make the jump *never* fire (a footgun). Rewrote to a "time-since-press" representation with **tick LAST**: `since_press<=buffer_secs`=live, `grounded||coyote>0`=eligible. Ticking last means a 0-length window disables forgiveness *without* the footgun (a grounded press still jumps on the exact frame). 9 unit + 2 doctests. Example `input_buffer` (kinematic box on a platform with a gap: walk off for coyote, tap early for buffer).
   - **#318 SpriteTrail.** `SpriteTrail{interval,lifetime,start_alpha}`+timer on a moving Sprite+Transform; `SpriteTrailGhost`=emitted fading copy; `SpriteTrailSystem` 2-pass: snapshot a ghost per due source (full `Sprite` clone at `z-0.1` behind the source, **one ghost/source/frame** = no slow-frame storm, ghosts carry no `SpriteTrail` = no ghost-of-ghost) → fade each ghost alpha→0 + despawn. A ghost fades only ITSELF → **safe on a kept entity** (unlike FloatingText). clone+editor-add registered like YSort. 6 unit + 2 doctests. **Test gotcha fixed:** a ghost spawned in pass-1 gets ticked by pass-2 the same frame; a test using `dt=10.0` made the ghost fade instantly → rewrote the despawn test to spawn (interval 0) then `remove_component::<SpriteTrail>` to stop emission, then tick with realistic dt.
   - **#319 docked.rs split.** See Code Analysis + Evidence for the method. 9 files, largest 341 lines (was 1619). Behavior-preserving; tests moved verbatim.
6. **User asked "남은 후보중 추천하는 작업 뭐야?"** — I recommended the **UI ProgressBar widget** (clear gap in the widget set, low risk, existing DrawRect corner_radius/border support, proven cadence) over the game-feel-combo example game and the app.rs refactor (deprioritized: don't do two refactors back-to-back).
7. **User said "추천 작업 진행해".** → **#320 ProgressBar.** Read-only gauge modeled on `Slider` minus interaction. `progress_bar_pass.rs` mirrors `label_pass` (non-interactive: `world/viewport/output/scratch` signature). 5 unit + 1 doctest. Example `ui_progress` (health/loading/XP bars, rounded+bordered, live %). Headless capture = 3 gauges at 78/15/6% matching labels.
8. **User: "handoff … 푸시 해줘"** → this handoff (new chain seq 1), to be committed + pushed.

## Key Decisions

- **Asked for direction, then recommended.** Board empty → `AskUserQuestion` (per editor-ux seq 9 protocol). Later, when asked for a recommendation, gave ONE clear pick (ProgressBar) with reasoning + a runner-up, not an exhaustive survey.
- **FloatingText despawns the entity; SpriteTrail does not.** FloatingText is a dedicated ephemeral entity (like a particle) → despawn is correct and clean. But that makes it a footgun to hand-author in the editor → NOT editor-add-registered. SpriteTrail's ghosts are separate entities that fade themselves; the SpriteTrail source is untouched → SAFE on a kept entity → editor-add-registered like YSort. This asymmetry is deliberate and documented in both module docs + CLAUDE.md rows.
- **InputBuffer ticks LAST, uses time-since-press.** Chosen specifically so a zero-length window disables forgiveness without preventing a grounded jump (no footgun). The recommended per-frame order is `set_grounded → press → try_consume → tick`. Consuming clears both windows → one press = one jump (no mid-air double-jump).
- **InputBuffer / ProgressBar are NOT camera-shake / Slider clones.** Camera shake already existed (excluded). ProgressBar deliberately has no input path (it's the read-only complement to the interactive Slider).
- **docked split: mod.rs keeps the orchestrator + re-exports.** `update_docked_ui` (the panel dispatcher) stays in `mod.rs`; the `pub(in crate::app)` tab fns are re-exported from `mod.rs` so the parent `ui/mod.rs` import (`docked::{...}`) is **unchanged**. Shared helpers (entity_kind classifier, context-menu dispatch) became `pub(super)`. This keeps the split a pure move with no call-site churn outside `docked/`.
- **Tests moved verbatim with their code.** `icon_tests`→`entity_kind.rs`, `context_action_tests`→`context_menu.rs`, `swatch_tests`→`inspector_tab.rs`. Each test module's `use super::{...}` still resolves because `super` is now the co-located submodule. Zero test edits = strong evidence behavior is preserved.
- **docked-split is a PATCH (0.106.1), not a MINOR.** Pure internal refactor, no public-API change → PATCH per the pre-1.0 ship rule; CHANGELOG uses `### Changed (internal)`.
- **Each feature is its own coherent PR.** No bundling (one PR = one change). The 3-item "2,3 진행" batch landed as 3 separate PRs (#317/#318/#319), each verified + CI'd + merged independently off fresh main.
- **CLAUDE.md kept at exactly 200 lines every PR** by reclaiming a line before adding a module-map row (lossless merges: the WASM-support blockquote 2→1 line; the Conversation-language parenthetical trim; the doc-Language bullet 3→2 lines).

## Evidence & Data

### PRs this session

| PR | Version | Type | Summary | Tests |
|---|---|---|---|---|
| #316 | v0.104.0 | feat | FloatingText + FloatingTextSystem (rising/fading combat text) | +6 unit, +2 doctest |
| #317 | v0.105.0 | feat | InputBuffer (buffered jump + coyote time), pure-logic helper | +9 unit, +2 doctest |
| #318 | v0.106.0 | feat | SpriteTrail + SpriteTrailGhost + SpriteTrailSystem (afterimage) | +6 unit, +2 doctest |
| #319 | v0.106.1 | refactor | split docked.rs (1619 lines) → docked/ (9 files) | 0 (moved verbatim) |
| #320 | v0.107.0 | feat | ProgressBar read-only UI gauge widget | +5 unit, +1 doctest |

### Commit log (this session, newest first)

| Hash | PR | Subject |
|---|---|---|
| `30d9d8b` | #320 | feat(ui): ProgressBar — read-only health/mana/loading/XP gauge widget (v0.107.0) |
| `abb09cc` | #319 | refactor(editor): split docked.rs (1619 lines) into a docked/ directory module (v0.106.1) |
| `e603b41` | #318 | feat(render): SpriteTrail — fading afterimage behind a moving sprite (v0.106.0) |
| `26b15dd` | #317 | feat(input): InputBuffer — buffered jump + coyote time (v0.105.0) |
| `96cc47c` | #316 | feat(feature): FloatingText — rising, fading combat text + FloatingTextSystem (v0.104.0) |

### Version / header progression

| After PR | package | CLAUDE.md header | memory seq | main @ |
|---|---|---|---|---|
| (start) | 0.103.0 | v1.6.191 | 136 | 45f0633 |
| #316 | 0.104.0 | v1.6.192 | 137 | 96cc47c |
| #317 | 0.105.0 | v1.6.193 | 138 | 26b15dd |
| #318 | 0.106.0 | v1.6.194 | 139 | e603b41 |
| #319 | 0.106.1 | v1.6.195 | 140 | abb09cc |
| #320 | 0.107.0 | v1.6.196 | 141 | 30d9d8b |

### docked.rs split — file breakdown (behavior-preserving)

| File | Lines | Concern |
|---|---|---|
| `docked/mod.rs` | ~188 | `update_docked_ui` orchestrator + module decls + `pub(in crate::app)` re-exports |
| `docked/toolbar.rs` | 124 | top toolbar (`docked_toolbar`, `pub(super)`) |
| `docked/entity_kind.rs` | 247 | `EntityKind`/`entity_kind`/`entity_type_icon`/`sorted_entity_list` (`pub(super)`) + `icon_tests` |
| `docked/context_menu.rs` | 214 | `EntityContextAction`/`entity_context_menu`/`editor_apply_entity_context_action` (`pub(super)`) + `context_action_tests` |
| `docked/entities_tab.rs` | 202 | `entities_tab_body` |
| `docked/scene_tab.rs` | 158 | `scene_tab_body` + `DragEntity` |
| `docked/inspector_tab.rs` | 341 | `inspector_tab_body` + `uv_rect_to_egui` + `swatch_tests` |
| `docked/assets_tab.rs` | 41 | `assets_tab_body` |
| `docked/save_load.rs` | 159 | `save_load_controls`/`do_save_scene_with_list`/`do_save_scene`/`do_load_scene` |

### Reusable: the split method (for the next large-file split, e.g. app.rs)

1. **Map sections** with a grep of top-level items (`^(pub|fn|struct|enum|impl|const) `) + line numbers.
2. **Compute boundaries in Python:** for each anchor line (item signature substring), walk BACKWARD over contiguous preceding `///`-doc / `#[attr]` lines to find the section's true start; a section = `[true_start, next_true_start)`. Run this as a **dry run first** (print boundaries) before writing.
3. **Extract with a Python script:** `seg(a,b)` slices lines; each submodule gets a `//!` doc + `use super::*;` header + the extracted range; bump shared helpers to `pub(super)` via asserted single-occurrence `str.replace`. `mod.rs` = header + `mod` decls + `pub(in crate::app) use` re-exports + orchestrator. Then `os.remove` the old file.
4. **Compile-iterate.** The only real bug was **one off-by-one boundary** (`seg(12,175)` should have been `seg(12,174)`) that left a dangling `/// Toolbar contents…` doc comment in `mod.rs` (error: "expected item after doc comment"). Fixed by trimming the trailing section-header + doc line.
5. **Beware compile-truncation warnings.** After the hard error, the build reported 4 spurious "unused import" warnings (audio_mixer_panel_body, data_table_panel_body, EntitySortMode, entity_matches_filter) — these were artifacts of the compiler stopping analysis at the hard error. Once the error was fixed, the rebuild was clean (0 warnings). **Do not chase warnings emitted alongside a hard error until the error is fixed.**
6. **Cross-`ui`-sibling imports** (`super::grid_overlay::draw_editor_grid`, `super::lighting_panel::ambient_light_control`) stayed in `mod.rs`; `super::` from `mod.rs` is still `ui`, so those paths were unchanged. Submodules using them inherit via `use super::*`.

### InputBuffer semantics (the tested contract)

Recommended per-frame order: `set_grounded(g)` → `if pressed { press() }` → `if try_consume() { jump }` → `tick(dt)` (**last**).
- `press()` → `since_press = 0`. Live while `since_press <= buffer_secs`.
- `set_grounded(true)` → `coyote = coyote_secs`. Eligible while `grounded || coyote > 0`.
- `tick(dt)` → `since_press += dt`; `if !grounded { coyote -= dt }`.
- `try_consume()` → if `is_buffered() && is_coyote_available()`, set `since_press=∞`, `coyote=0`, return true.
- `DEFAULT_BUFFER_SECS = 0.12`, `DEFAULT_COYOTE_SECS = 0.10`. Windows clamp to `>= 0`.

### Headless captures (all visually confirmed; render-path with no display)

| Example | Frames | What the capture showed |
|---|---|---|
| `floating_text` | 70 | color-coded numbers rising over 3 targets, older ones higher + more transparent (fade), CRIT/heal/miss distinguished |
| `input_buffer` | 40 | box mid-jump above the platform, HUD: grounded/coyote/buffer ms + jump count + last-kind |
| `sprite_trail` | 60 | bright box leading a fading arc of 13 ghosts |
| `ui_progress` | 30 | 3 rounded+bordered gauges (green health 78%, blue loading 15%, gold XP 6%) with matching % labels |

### Verify gate (run identically each PR; all 7 green, 2 audio `--skip`ped)

`cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo build --target wasm32-unknown-unknown` · `cargo clippy --target wasm32 --lib -- -D warnings` · `cargo test --all-targets` (+ the 2 audio skips) · `cargo test --doc` · `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`. CI ran **5/5** each PR: Build (WASM) · Package dry-run · Render tests (lavapipe) · Rustdoc · Test (native).

### Recurring clippy lint hit twice

`self.frame % N == 0` → clippy `manual_is_multiple_of` (Rust 1.95) → use `self.frame.is_multiple_of(N)`. Hit in the `floating_text` and `input_buffer` examples' headless auto-fire timers. Reflex fix.

### Unit test names (the tested contracts — a future edit to these modules must keep them green)

- **FloatingText (6):** `rises_and_despawns_when_finished`, `fades_alpha_toward_zero`, `no_fade_keeps_full_alpha`, `projects_through_the_camera`, `progress_and_is_finished_accessors`, `works_without_a_camera_resource`.
- **InputBuffer (9):** `grounded_press_jumps_immediately`, `coyote_allows_a_late_press`, `coyote_expires`, `buffered_press_fires_on_landing`, `buffer_expires_if_landing_is_too_late`, `grounded_keeps_coyote_full`, `no_double_jump_in_the_air`, `zero_windows_still_allow_a_grounded_jump`, `negative_windows_clamp_to_zero`.
- **SpriteTrail (6):** `emits_a_ghost_after_the_interval`, `ghost_snapshots_position_and_sits_behind`, `ghost_starts_at_the_start_alpha_and_fades`, `ghost_despawns_when_faded`, `a_ghost_never_spawns_ghosts`, `progress_and_is_finished_accessors`.
- **ProgressBar (5):** `fraction_clamps_without_mutating_the_field`, `new_clamps_the_initial_value`, `builders_set_colors_radius_border`, `serde_roundtrip_keeps_value_and_style`, `reflect_roundtrip`.
- **docked-split:** no new tests; the moved modules still run `icon_tests::*` (7), `context_action_tests::*` (6), `swatch_tests::*` (2) under their new paths (e.g. `app::editor::ui::docked::entity_kind::icon_tests::…`) + the 4 docked render tests in `tests/render.rs`.

### AskUserQuestion option sets used (reusable when the board is empty)

- **Q1 (direction, board empty):** 새 엔진 기능+예제 [chosen] / 새 예제 게임 / 코드 건강성 리팩터 / 지금은 중단. → picked feature+example.
- **Q2 (which feature):** 플로팅 데미지/전투 텍스트 [chosen] / 입력 버퍼링+코요테 / UI 프로그레스 바 / 스프라이트 트레일 — all four pre-verified as genuine gaps (camera-shake excluded because it already exists). User later picked the other three across the session too.
- **When asked for a recommendation** (not an AskUserQuestion) — gave ONE pick (ProgressBar) with 4 reasons + a runner-up + explicitly-deprioritized items (2D shadows: too big; app.rs refactor: don't stack refactors). User accepted. Pattern: a reasoned single recommendation beats a menu when the user asks "what do you recommend."

### API-awkwardness check (VISION rule: fix the API if the example is awkward)

For all 4 features the example was clean to write — no API changes were forced by the example. FloatingText's `impl App` helpers (`spawn_floating_text{,_colored}`) are co-located in `src/floating_text.rs` (a top-level module can `impl crate::app::App`), keeping the feature self-contained/fork-friendly rather than scattering the helper into `src/app/`. The one design change was internal (InputBuffer's tick-last redesign, made *before* the example, to avoid the zero-window footgun).

## Code Analysis

- **`FloatingTextSystem::run`** — `let camera = world.resource::<Camera>().copied();` (Camera is `Copy`) releases the borrow before `query2_mut::<FloatingText,Transform>()`; collect `DrawText`s + finished entities during the query, then `push` to `TextQueue` + `despawn` after (the standard collect-then-apply borrow workaround).
- **`SpriteTrailSystem::run`** — 2 passes: `query3_mut::<SpriteTrail,Transform,Sprite>()` collects `(Transform, Sprite, lifetime, base_alpha)` snapshots; spawn ghosts; then `query2_mut::<SpriteTrailGhost,Sprite>()` fades + collects finished; despawn. `GHOST_Z_OFFSET = 0.1` (ghost drawn behind; higher `Transform.z` = in front, per YSort).
- **`InputBuffer`** — pure struct, no ECS. Fields `buffer_secs, coyote_secs, since_press (f32::INFINITY init), coyote, grounded`. All accessors + `new(buffer,coyote)` + `Default`.
- **`ProgressBar`** — `{value, bg_color, fill_color, corner_radius, border, border_color}`; `Reflect` exposes `value`/`corner_radius`/`fill_color`/`bg_color`; serde `#[serde(default)]` (value serializes — it's the authored field). `fraction()` clamps `0..=1` without mutating `value`.
- **`progress_bar_pass::run(world, viewport, output, scratch)`** — mirrors `label_pass` (no input/capture). `node_layout(world, entity, viewport) -> (pos, size, z, visible)`; pushes a bg `DrawRect` (`with_corner_radius(r).with_z(z)`), a fill rect (`fill_w = size.x * bar.fraction()`, skipped if 0, `z + UI_SUBLAYER_Z_STEP`), and an optional border outline (`with_border`, `z + 2*step`). `UI_SUBLAYER_Z_STEP = 0.001`.
- **`DrawRect`** (`src/renderer/ui.rs`) — `with_z` / `with_corner_radius` / `with_border`; `0,0` fast path. The UI SDF pipeline (`ui.wgsl`) renders rounded/bordered rects; sprite pipeline untouched.
- **UI widget registration touchpoints** (mirror for any new widget): `src/ui/mod.rs` (`pub mod` + `pub use`), `src/lib.rs` (`pub use ui::{...}`), `src/ui/system.rs` (`mod X_pass` + scratch field + call in `run`), `src/app/core_resources.rs` (`register_reflect_named` + `register_clone` + serde `registry.register`), `src/app/editor/component_registry.rs` (`register_component` + `register_component_remover`).
- **Non-ECS logic-helper registration** (FloatingText/SpriteTrail = components → clone-registered; InputBuffer = pure helper like Timer/Tween → NOT registered at all). Component clone registration lives in `core_resources.rs::register_core_component_metadata`.

## Files Changed

### New source modules
- `src/floating_text.rs` — FloatingText + FloatingTextSystem + spawn helpers + `impl App` helpers (#316).
- `src/input_buffer.rs` — InputBuffer pure-logic helper (#317).
- `src/sprite_trail.rs` — SpriteTrail + SpriteTrailGhost + SpriteTrailSystem (#318).
- `src/ui/progress_bar.rs` — ProgressBar widget (#320).
- `src/ui/system/progress_bar_pass.rs` — the render pass (#320).

### Refactor (#319) — split, no behavior change
- `src/app/editor/ui/docked.rs` **deleted** → `src/app/editor/ui/docked/{mod,toolbar,entity_kind,context_menu,entities_tab,scene_tab,inspector_tab,assets_tab,save_load}.rs`.

### Wiring (touched by multiple PRs)
- `src/lib.rs` — `pub mod` + `pub use` for each new type (FloatingText, InputBuffer, SpriteTrail, ProgressBar + their `DEFAULT_*` consts + `spawn_floating_text`).
- `src/app/core_resources.rs` — `register_clone` for FloatingText/SpriteTrail; reflect+clone+serde for ProgressBar.
- `src/app/editor/component_registry.rs` — editor add/remove for SpriteTrail + ProgressBar.
- `src/ui/mod.rs`, `src/ui/system.rs` — ProgressBar module + pass wiring.

### Examples
- `examples/floating_text.rs`, `examples/input_buffer.rs`, `examples/sprite_trail.rs`, `examples/ui_progress.rs` (all flat → Cargo auto-discovers, no Cargo.toml change).

### Release paperwork (per PR)
- `Cargo.toml` / `Cargo.lock` (version), `docs/CHANGELOG.md` (entry), `CLAUDE.md` (header + module-map row, kept at 200 lines).

### Memory (outside repo)
- `engine-current-state.md` — bumped 136→141 (in-place Python `str.replace`; the tip line is ~77k chars, too big for the Edit read-gate).
- `MEMORY.md` — index line updated per seq.

## User Feedback & Preferences (REQUIRED)

- **"새 엔진 기능 + 예제"** (AskUserQuestion) — when the board was empty, chose new breadth features over a new example game / refactor / stop. Signal: wants continued breadth growth.
- **"플로팅 데미지/전투 텍스트"** then **"2, 3 진행"** — picked specific features from offered lists. "2, 3" mapped to my priority-numbered list (priority-2 = A-group game-feel, priority-3 = D refactor); I interpreted A-group as BOTH input-buffer AND sprite-trail and stated that plan before executing — user did not object.
- **"남은 후보중에 추천하는 작업 뭐야?"** then **"추천 작업 진행해"** — explicitly asked for MY recommendation, then accepted it. Signal: trusts a well-reasoned single recommendation; don't over-survey. Give one clear pick + why.
- **"handoff … 푸시 해줘"** — wants the handoff captured AND pushed (not left uncommitted). Same pattern as prior sessions.
- **Standing (memory + prior seqs):** merge authority delegated (squash on green CI, no re-confirm — express merge as a direct instruction, never an AskUserQuestion option); user-facing reports **Korean**, repo artifacts **English**; never push to main directly (branch + PR); `cargo fmt` before verify (fmt --check reflows long lines + reds gate 1); read a gate exit non-piped or via `$pipestatus` (1-indexed) NOT `${PIPESTATUS[0]}`; 2 audio tests fail locally → `--skip`, CI gates.
- **Momentum + trust:** the user drove a long, productive session (5 merges) with terse directions ("2,3 진행", "추천 작업 진행해"), delegating judgment. Keep shipping via the proven loop; surface a recommendation when asked rather than a menu.

## Where We're Going

1. **Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (was ACTIVE EMPTY / EW-004 as of 2026-06-25). A real game request there is the highest-value work — jump on it.
2. **If the board is still empty, ASK for a direction** (per the standing protocol) or offer a recommendation if asked. Candidates that remain **verified-unimplemented**:
   - **UI widgets (continue the set):** tooltip (hover → popup), dropdown/combobox, radio-button group, tab container. The editor's egui has these but the *game* UI system doesn't. Tooltip/dropdown are the most common asks.
   - **New example game (VISION capstone):** combine this session's game-feel primitives (FloatingText + InputBuffer + SpriteTrail + HitFlash + `Camera::shake`) into one small playable — validates they compose and surfaces API awkwardness. Overlaps `juice_demo` somewhat.
   - **Big subsystems (deliberate, larger):** 2D dynamic shadows (pairs with `LightingRenderer`, native-only), tilemap chunking / fog-of-war, particle collision.
3. **`app.rs` (1063 lines) slim/refactor** — the next `/split-module` candidate (God-struct). Was deliberately deferred this session ("don't do two refactors back-to-back"); now spaced out enough to be reasonable.

## Risks & Blockers

- **GUI playtest is blocked** (locked/remote macOS screen). Every feature this session was verified only via `HEADLESS_SHOT` captures + unit tests. Gesture/interaction paths (a real click, a held key, a drag) are NOT headless-testable — a human click-through on a real display is the only way to catch an egui/input wiring regression. This is a standing limitation, not new.
- **docked-split is native-only** (`#![cfg(not(target_arch = "wasm32"))]`) → CI ubuntu native runner compiles it; no OS-gated-CI risk. But note CI is ubuntu-only, so any future `#[cfg(target_os = "macos")]` work still needs a local both-branch build + hardware check.
- **`engine-current-state.md` tip line is ~77k chars** (one line) — too big for the Edit tool's read-gate; every memory bump uses an in-place Python `str.replace`. It keeps growing; a future cleanup should trim the oldest per-seq detail into `engine-history-archive.md` (last done 2026-06-20) before it becomes unwieldy.

## Open Questions

- **None blocking.** Strategic only: what next, given the board is empty and both editor-UX and this game-feel/UI-widget run are substantial? That's for the user (see Where We're Going).
- **Should the game-feel primitives get a combined capstone example?** They've each only been exercised in isolation. A combined demo would be the truest VISION acceptance test but partially overlaps `juice_demo`. Deferred pending user interest.

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -6          # tip = 30d9d8b #320 ProgressBar (v0.107.0)
git status -s                 # clean

# Board FIRST (was ACTIVE EMPTY → ASK or recommend a direction if still empty)
sed -n '53,56p' ../dungeon-merchant/docs/engine-wishlist.md

# Verify (2 audio tests fail locally — env; read exit via echo $?, NOT ${PIPESTATUS[0]} in zsh)
cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
cargo clippy --all-targets -- -D warnings

# Key files from this session (the reusable patterns)
#   src/floating_text.rs / src/sprite_trail.rs  — transient game-feel component + user-added system (HitFlash pattern)
#   src/input_buffer.rs                         — pure-logic helper (Timer/Tween pattern), tick-last
#   src/ui/progress_bar.rs + system/progress_bar_pass.rs — a full new UI widget + its render pass (mirror label_pass)
#   src/app/editor/ui/docked/                   — the 9-file split; mod.rs = orchestrator + re-exports

# Next action
#   Check the board. If empty, ASK for a direction or recommend one (tooltip/dropdown UI widget,
#   a game-feel capstone example, or the app.rs split). Ship via /add-feature-example or
#   /split-module → the land-loop (branch → verify 7 gates → /ship → PR → CI 5/5 → squash-merge → memory bump).
```

## Session Closed

**Closed at:** 2026-07-02
**Commit:** to be landed via the `docs(handoff)` PR (this session's tip is `30d9d8b`; the handoff lands on top)
**Session status:** Handed off to next session
