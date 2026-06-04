# skeleton-engine source-split regression fixes + full review handoff

**Date:** 2026-06-03
**Status:** COMPLETED
**Bead(s):** none
**Epic:** source file split / maintainability refactor
**Chain:** `source-split-refactor` seq `2`
**Parent:** `HANDOFF_source-split-refactor_source-file-split_2026-06-03.md` (seq 1)
**Prior chain:** `HANDOFF_source-split-refactor_source-file-split_2026-06-03.md` > this

---

## Since Last Handoff

Compares parent (seq 1) plan/claims vs what actually happened this session.

- Parent declared the split **COMPLETED** with **local** verification limited to `cargo fmt --check` and `cargo test --lib`. That narrow local bar let real regressions through: **the WASM build was broken and clippy had new warnings**, neither of which `cargo test --lib` exercises. seq 2 caught and fixed them. **Crucially, CI (`.github/workflows/ci.yml`) already runs `cargo clippy --all-targets -- -D warnings` AND `cargo build --target wasm32-unknown-unknown`** — it would have caught everything. The real failure was that seq 1's split stayed uncommitted/unpushed, so CI never ran; the local "done" bar didn't mirror CI.
- Parent's "Where We're Going" step 1 (review dirty worktree, decide whether to commit) → DONE. After fixes, the user committed everything as `61c09f1` and we pushed to `origin/main`.
- Parent's open question "should `FrameContext` remain re-exported from `engine::renderer`?" — resolved as **yes/kept**. BUT see discrepancy below: the inherited worktree did NOT actually contain `FrameContext`.
- **Discrepancy (important):** Parent claimed (lines 59/178/306) that `FrameContext` was already introduced and re-exported from `src/renderer/mod.rs`. The worktree I inherited did **not** have it — `src/renderer/mod.rs:18` was `pub use sprite::SpriteRenderer;` only, and `SpriteRenderer::render` still took 9 individual args (triggering `clippy::too_many_arguments`). seq 2 actually created `FrameContext` and the re-export as part of the clippy fix. Treat parent's FrameContext claims as describing an intended state that was not present.
- Parent's deferred items (`src/app/editor/ui.rs` 963-line further split, `src/audio.rs` 771-line split, `CLAUDE.md` module-dir mention) remain **untouched** — still open for a future session.
- Trajectory: split is now committed + pushed + verified green on native AND wasm. The workstream's "make the split correct and durable" goal is effectively closed; only optional deeper cleanups remain.

## Reference Documents

- `CLAUDE.md` — project quick reference / module map. Read this session; states the invariant "WASM support: `cargo build --target wasm32-unknown-unknown` passes" — the exact invariant seq 1 broke.
- `AGENTS.md`, `ARCHITECTURE.html` — module-map docs updated by seq 1 (not re-edited this session).
- `docs/VISION.md` — feature work needs a playable example; this session was a mechanical fix/review, examples excluded.
- Plan file produced mid-session: `/Users/jkl/.claude/plans/curried-growing-wreath.md` (the severity-ordered fix plan approved before implementation).

## The Goal

Continue the `source-split-refactor` chain: review the (uncommitted) v2 module split for correctness, fix any regressions the split introduced, then commit and push. The user first asked for a full code review, then (via `/grill-me`) for a severity-ordered fix plan, then implementation, then a second full review, then commit + push. Hard constraint throughout: preserve public API and runtime behavior — only restore what the split accidentally dropped (cfg gates, `#[allow]` attributes) plus one approved mechanical refactor.

## Where We Are

- All work is **committed and pushed**. `origin/main` is at `61c09f1 refactor: split large engine source files` (author ChunSam, committed by the user mid-session at 23:36). Working tree is clean.
- `git push` result: `c755239..61c09f1 main -> main` (2 commits: `5f75a91` + `61c09f1`).
- **CI is GREEN on `61c09f1`**: `gh run list --branch main` shows the push run `completed/success` in 5m53s — all jobs (native clippy `-D warnings` + test, wasm build, rustdoc, package dry-run) passed. The authoritative check now validates the fixed state.
- **WASM lib build restored**: `cargo build --target wasm32-unknown-unknown` finishes clean (was failing with E0432 + E0609).
- **Native build/clippy clean**: `cargo build --all-targets` and `cargo clippy --all-targets` produce 0 warnings.
- **WASM lib clippy clean**: `cargo clippy --lib --target wasm32-unknown-unknown` produces 0 warnings.
- **Tests green**: `cargo test` → 269 lib + 32 integration (20 ignored) passed, 0 failed.
- `src/app/editor.rs` — restored `#[cfg(not(target_arch = "wasm32"))]` on 6 items + `#[allow(clippy::enum_variant_names)]` and `#[derive(Debug)]` on `EditorCmd`.
- `src/app.rs:19` — `use editor::EditorHistory;` now cfg-gated to native; also added `FrameContext` to the `crate::renderer` import group.
- `src/asset/async_loading.rs` — split the import so `decode_image_with_state` is native-gated, `magenta_fallback` stays unconditional.
- `src/asset/hot_reload.rs` — `decode_image_with_state`, `compile_script_file`, `asset_key` all native-gated (rustfmt reordered them).
- `src/app/schedule.rs:33` — `write_crash_log` re-gated to native (hidden dead_code regression surfaced after E0432/E0609 cleared).
- `src/app/render.rs:84` — `ensure_intermediate_texture` re-gated to native (same class of hidden regression).
- `src/renderer/sprite.rs` — new `pub struct FrameContext<'a>`; `render` and `render_ui_primitives_from_slices` now take `&mut FrameContext` instead of 4 separate wgpu args; inline `mod tests` (was at the TOP of the file, line ~38, before the struct) extracted to `src/renderer/sprite/tests.rs`.
- `src/renderer/sprite/ui_primitives.rs` — deleted 2 unused `pub` wrappers (`render_ui_rects_from_slice`, `render_ui_images_from_slice`); refactored `render_ui_primitives_from_slices` to `FrameContext`.
- `src/renderer/mod.rs:18` — `pub use sprite::{FrameContext, SpriteRenderer};` (added FrameContext).
- `src/app/render.rs` — 3 call sites (lines ~283, ~381, ~446) construct `&mut FrameContext { device, queue, view, encoder }` inline.
- `src/renderer/sprite/tests.rs` — new file holding all 5 sprite tests; all 5 pass under `renderer::sprite::tests`.
- Public API verified byte-identical to pre-split (HEAD `5f75a91`) for app/asset/ecs::world/physics::world/scripting/ui::system; only `sprite` differs (deliberate: −2 dead wrappers, +`FrameContext`).

## What We Tried (Chronological)

1. **First full review (compile-level).** Ran `cargo check --all-targets` (passed), `cargo clippy --all-targets` (native, found warnings), then `cargo clippy --all-targets --target wasm32-unknown-unknown` → **build failed**: `E0432` (unresolved `decode_image_with_state`) and `E0609` (no field `component_factories`). Concluded the split broke the documented wasm invariant. Confirmed each dropped `#[allow]`/cfg against `git show HEAD:<file>`.
2. **Root-caused the wasm errors.** `decode_image_with_state` is `#[cfg(not(target_arch="wasm32"))]` in `image_loading.rs:117` but was imported unconditionally by `async_loading.rs:7` and `hot_reload.rs:4`. `component_factories` is a native-only `App` field but `editor.rs:141` (`register_component`) accessed it without a cfg gate — because the whole editor block lost its gates in the split.
3. **Quantified the clippy regressions vs HEAD.** `git stash` + clippy on original showed 0 of {too_many_arguments, items-after-test-module, enum_variant_names}. Original `sprite.rs` had **5** `#[allow(clippy::too_many_arguments)]`; only 1 survived the split (`sort.rs:50`). Original `EditorCmd` had `#[allow(clippy::enum_variant_names)]` + `#[derive(Debug)]`, both dropped.
4. **`/grill-me` severity-ordered planning.** Asked 2 rounds of bounded questions; locked: (a) fix all 3 tiers, (b) clippy via param-struct refactor not `#[allow]` restore, (c) extract sprite tests to `tests.rs`, (d) delete the 2 dead wrappers. Researched callers first: `SpriteRenderer` is `pub` via `engine::renderer::sprite` but NOT in crate-root re-exports, and rust-survivors uses neither `SpriteRenderer` nor `.render(` directly → no downstream break. Wrote plan to `/Users/jkl/.claude/plans/curried-growing-wreath.md`, approved via ExitPlanMode.
5. **Tier 1 (wasm) implementation.** Restored cfg gates in `editor.rs` (6 items), `app.rs:19`, split imports in `async_loading.rs`, gated all 3 imports in `hot_reload.rs`. Re-ran `cargo build --target wasm32` → E0432/E0609 gone, but **2 new dead_code warnings appeared** (`ensure_intermediate_texture`, `write_crash_log`) that were hidden behind the earlier hard errors. Verified both were `#[cfg(not(target_arch="wasm32"))]` in HEAD, re-gated them. Re-ran → wasm build fully clean.
6. **Tier 2 (clippy struct refactor).** Created `FrameContext<'a> { device, queue, view, encoder }`, re-exported it, changed `render` + `render_ui_primitives_from_slices` signatures, deleted the 2 dead wrappers, updated 3 call sites. Used a minimal-risk technique: bind `let device = ctx.device; ... let encoder = &mut *ctx.encoder;` at function top so the bodies are unchanged. Native build passed.
7. **Tier 3 (sprite test extraction).** Wrote `src/renderer/sprite/tests.rs` (copied body + moved the `#[cfg(test)]` imports `file_texture_aliases`, `sorted_ui_primitives`, `UiPrimitiveKind` into it with `super::textures::`/`super::ui_primitives::` paths). Removed the inline `mod tests { … }` (≈195 lines) via a verified Python splice (Edit was impractical for a contiguous 195-line delete), added `#[cfg(test)] mod tests;` next to the other mod decls.
8. **Full verification.** `cargo fmt`, native build+clippy (0 warnings), wasm lib build+clippy (0 warnings), `cargo test` (269+32 pass), all 5 sprite tests pass by name.
9. **Second full review (logic/API).** Read `render.rs`, `window.rs`, `physics/world.rs`, `scripting.rs` end-to-end; confirmed cfg gating and render-pass ordering intact. Ran a per-module `pub` symbol diff vs HEAD → identical except deliberate sprite changes. Confirmed no accidental visibility widening/narrowing.
10. **Commit + push.** Discovered the user had already committed the whole worktree as `61c09f1` (working tree clean). Verified my fixes were in HEAD (cfg gates ×6, FrameContext, tests.rs, dead wrappers gone), then `git push origin main`.

## Key Decisions

- **Fix all 3 severity tiers**, not just the critical wasm break (user choice in grill-me).
- **clippy `too_many_arguments` → param-struct refactor, not `#[allow]` restore** (user choice). Rationale: cleaner than re-adding suppression; safe because the methods are de-facto internal.
- **Delete the 2 unused `pub` wrappers** rather than struct-ify or `#[allow]` them (user choice). They had 0 callers, aren't in crate-root re-exports, and rust-survivors doesn't use them.
- **Bundle only the 4 wgpu handles** into `FrameContext` (not width/height/layer_mask) — that alone drops 9→6 args, under the threshold, and is reusable across the 2 functions.
- **Minimal-risk refactor body technique**: re-bind `ctx.device`/`queue`/`view`/`&mut *ctx.encoder` to locals at the top so function bodies stay byte-for-byte unchanged → no behavior change.
- **Extract sprite tests to a sibling `tests.rs`** for consistency with the other split modules (which all already put tests in a separate file) and to fix "items after a test module".
- **Treated the wasm example failures as out of scope** — `platformer_game`/`mp_server`/`gpu_particles` fail wasm clippy because they use native-only deps (`rapier2d`, `tungstenite`, `GpuParticleEmitter`); pre-existing, examples not touched.
- **Pushed directly to `main`** — the user explicitly asked, the commit already existed on main (made by the user), working tree clean, and the repo's history shows direct-to-main is the norm.

## Evidence & Data

### WASM build: before vs after Tier 1

| Stage | `cargo build --target wasm32-unknown-unknown` |
| --- | --- |
| Inherited worktree (seq 1 output) | FAIL — `E0432` (×2: `async_loading.rs:7`, `hot_reload.rs:4`), `E0609` (`editor.rs:141`) |
| After 1a–1d (cfg/import gates) | builds, but 2 `dead_code` warnings (`ensure_intermediate_texture`, `write_crash_log`) |
| After re-gating those 2 fns | clean, 0 warnings |

Verbatim wasm errors from the inherited worktree (primary evidence):

```text
error[E0432]: unresolved import `super::image_loading::decode_image_with_state`
  --> src/asset/async_loading.rs:7:28   (also src/asset/hot_reload.rs:4:5)
   note: found an item that was configured out
   --> src/asset/image_loading.rs:117:15  (#[cfg(not(target_arch = "wasm32"))])
error[E0609]: no field `component_factories` on type `&mut app::App`
  --> src/app/editor.rs:141:14
   = note: available fields are: `world`, `systems`, ... and 26 others
```

### Dropped `#[allow]` accounting (vs `git show HEAD`)

| File / item | Original | After split | seq 2 resolution |
| --- | --- | --- | --- |
| `sprite.rs` `too_many_arguments` | 5 attrs | 1 (`sort.rs`) | refactor 2 fns to `FrameContext`; delete 2 dead fns; sort.rs unchanged |
| `EditorCmd` `enum_variant_names` + `Debug` | present | dropped | restored both |

### Final verification (all green)

| Command | Result |
| --- | --- |
| `cargo build --all-targets` (native) | clean |
| `cargo clippy --all-targets` (native) | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib) | clean |
| `cargo clippy --lib --target wasm32-unknown-unknown` | 0 warnings |
| `cargo test` | 269 lib + 32 integration (20 ignored), 0 failed |
| `cargo test --lib renderer::sprite::tests` | 5 passed |

### Public `pub` symbol diff vs pre-split HEAD (`5f75a91`)

| Module group | Result |
| --- | --- |
| app, asset, ecs::world, physics::world, scripting, ui::system | identical ✓ |
| sprite | `−render_ui_images_from_slice`, `−render_ui_rects_from_slice`, `+FrameContext` (all deliberate) |

### Known non-issue: wasm `--all-targets` example failures

`cargo clippy --all-targets --target wasm32` fails compiling examples `platformer_game` (`rapier2d`, `engine::{CharacterController,PhysicsBody,PhysicsSystem,PhysicsWorld,TileCollider,TriggerEvent}`), `mp_server` (`tungstenite`), `gpu_particles` (`engine::GpuParticleEmitter`). These are native-only features; examples were never wasm targets. Pre-existing, unrelated to this work, no example files modified.

### Git state at handoff time

| Item | Value |
| --- | --- |
| Branch | `main` |
| `origin/main` | `61c09f1` (pushed this session) |
| Working tree | clean |
| `61c09f1` stat | 50 files changed, 7221 insertions(+), 6662 deletions(−) |
| `61c09f1` author | ChunSam <jkl9203@gmail.com>, Wed Jun 3 23:36 2026 |
| `bd` | not installed |

### Initial inherited worktree state (start of session)

At session start the split was uncommitted on `main`:

```text
 M AGENTS.md
 M ARCHITECTURE.html
 M src/app.rs
 M src/asset.rs
 M src/ecs/world.rs
 M src/physics/world.rs
 M src/renderer/sprite.rs      # note: NO src/renderer/mod.rs here → FrameContext absent
 M src/scripting.rs
 M src/ui/system.rs
?? src/app/… src/asset/… src/ecs/world/… src/physics/world/…
?? src/renderer/sprite/… src/scripting/… src/ui/system/…
```

`src/renderer/mod.rs` was NOT modified at start — direct evidence that seq 1 had not actually added the `FrameContext` re-export it claimed. `cargo check --all-targets` (native) passed; `cargo test --lib` = 269 pass. The breakage was wasm-only + clippy-only, invisible to seq 1's verification set.

### Pedantic clippy sweep (engine-wide, pre-existing — do NOT chase)

Ran `cargo clippy --lib -- -W clippy::pedantic` to distinguish refactor regressions from background noise. These are **whole-engine style preferences that predate the split** — explicitly NOT in scope, listed so the next session doesn't mistake them for new problems:

| Count | Lint |
| ---: | --- |
| 433 | missing `#[must_use]` on method |
| 101 | doc item missing backticks |
| 54 | missing `#[must_use]` on `-> Self` |
| 45 | `u32 as f32` precision loss |
| 41 | could be `let…else` |
| 31 | `f64 as f32` truncation |
| 30/26 | missing `# Panics` / `# Errors` doc sections |

No `TODO`/`FIXME`/`unimplemented!`/`todo!` markers exist in any new module file.

### grill-me decision provenance

Two bounded-question rounds before planning; chosen options:

| Question | Chosen |
| --- | --- |
| Severity tiers to fix | **All 3** (wasm + clippy + sprite test placement) |
| clippy `too_many_arguments` approach | **Bundle args into a struct (real refactor)** — not `#[allow]` restore, not crate-level allow |
| sprite inline `mod tests` | **Extract to `tests.rs`** (sibling-consistent) |
| 2 unused `pub` wrappers | **Delete** — not struct-ify, not `#[allow]` |

Pre-question research that made the delete/refactor safe: `grep` showed `render_ui_*_from_slice` have 0 engine callers; `SpriteRenderer` is reachable via `engine::renderer::sprite` but absent from crate-root `pub use`; rust-survivors has no `SpriteRenderer` / `.render(` usage.

### Severity-ordered fix map (the approved plan, in case the plan file is unavailable)

Plan file `/Users/jkl/.claude/plans/curried-growing-wreath.md` lives outside the repo; this is the durable copy.

| Tier | Symptom | Files / sites |
| --- | --- | --- |
| 1 (critical) | wasm build `E0432`/`E0609` + 2 hidden `dead_code` | `app/editor.rs` (6 cfg + 2 attrs), `app.rs:19`, `asset/async_loading.rs:7`, `asset/hot_reload.rs:4-6`, `app/schedule.rs:33` (`write_crash_log`), `app/render.rs:84` (`ensure_intermediate_texture`) |
| 2 (medium) | clippy `too_many_arguments` ×4, `enum_variant_names` ×1 | `renderer/sprite.rs` (`FrameContext` + `render`), `renderer/sprite/ui_primitives.rs` (delete 2 + refactor 1), `renderer/mod.rs:18`, `app/render.rs` call sites ~283/~381/~446. enum_variant_names resolved by Tier 1's `EditorCmd` attr restore. |
| 3 (low) | clippy "items after a test module" | `renderer/sprite.rs` inline `mod tests` (≈L38) → `renderer/sprite/tests.rs` |

### Downstream relationship

`rust-survivors` (`/Users/jkl/Projects/rust-survivors`) consumes this crate as `engine`. Confirmed it does NOT reference `SpriteRenderer`, the deleted wrappers, or call `.render(` directly — so the only public-surface change this session (sprite) has zero downstream impact. No rust-survivors changes needed.

## Code Analysis

- `FrameContext<'a> { device: &Device, queue: &Queue, view: &TextureView, encoder: &mut CommandEncoder }` — defined in `src/renderer/sprite.rs`, re-exported via `src/renderer/mod.rs`. Constructed inline at each call site so the `&mut encoder` borrow ends immediately after the call (no conflict with later `enc.finish()`/`queue.submit`).
- `SpriteRenderer::render(&mut self, ctx: &mut FrameContext, world, width, height, layer_mask)` and `render_ui_primitives_from_slices(&mut self, ctx, rects, images, width, height)` — both now ≤6 args; bodies unchanged via top-of-fn local re-binds.
- `editor.rs` items needing native gating (all access native-only `App` fields like `component_factories`, or feed `cmd_history`): `EditorCmd`, `EditorHistory`+impl, `entity_to_def`, `snap_to_grid`, `register_default_components`, `register_component`. `mod ui;` stays unconditional because `update_editor_ui` is called every frame from `schedule.rs:254` and is internally cfg-gated.
- The two hidden dead_code fns (`ensure_intermediate_texture` in render.rs, `write_crash_log` in schedule.rs) are only called from native-gated code paths, so they must themselves be native-gated — exactly as in HEAD.
- `render.rs` render-pass order is intact: offscreen RT passes → clear → sprite → UI primitives → GPU particles (native) → text → post-process → lighting (native) → fade (native) → submit → egui.
- All split modules now keep `#[cfg(test)] mod tests;` at the END (idiomatic). sprite.rs was the only outlier (inline mod at top) — now fixed.
- `window.rs::finish_init` correctly handles native vs wasm text-renderer creation (wasm skips `TextRenderer` when `font_bytes` empty, because cosmic-text panics shaping with no font). `poll_gilrs` uses the collect-then-process borrow workaround (gather events into a Vec, then mutate `GamepadState`). Both verified intact post-split.
- `physics/world.rs` keeps `PhysicsWorld` field encapsulation: rapier sets are `pub(crate)`, exposed only through typed accessors (`rigid_body`/`rigid_body_mut`/`get_collider`/`collision_groups`/…). `one_way_colliders: HashSet<ColliderHandle>` drives `move_character`'s one-way logic. No accessor lost in the split.
- `scripting.rs` root keeps `ScriptRunner`/`ScriptingSystem`/`ScriptingLimits` public; internal `scope`/`started` are `pub(crate)`. Rhai API registration lives in `scripting/api.rs`, execution in `scripting/execution.rs` — public surface unchanged.
- No duplicate symbol definitions across any split (checked physics group for copy-paste artifacts) — the split was a clean move, not a partial copy.

### Files read end-to-end this session (review coverage)
- Fully read: `app.rs`, `app/schedule.rs`, `app/scenes.rs`, `app/core_resources.rs`, `app/editor.rs`, `app/render.rs`, `app/window.rs`, `asset.rs`, `asset/image_loading.rs`, `asset/async_loading.rs`, `asset/hot_reload.rs`, `physics/world.rs`, `scripting.rs`, `renderer/sprite.rs` (top), `renderer/sprite/ui_primitives.rs`, `renderer/mod.rs`.
- NOT read line-by-line (low risk — green build/clippy/tests, identical public API): `app/editor/ui.rs` (963), `ecs/world.rs` (909), `ecs/world/tests.rs` (527), `app/assets.rs`, the ui/system/*_pass.rs widget files, physics submodule bodies, sprite/sort.rs|geometry.rs|textures.rs|material.rs.

## Files Changed

### Source code (all within commit `61c09f1`)
- `src/app/editor.rs` — restored 6 native cfg gates + `EditorCmd` `#[allow(enum_variant_names)]`/`#[derive(Debug)]`.
- `src/app.rs` — cfg-gated `use editor::EditorHistory`; added `FrameContext` to `crate::renderer` import.
- `src/asset/async_loading.rs` — native-gated `decode_image_with_state` import; `magenta_fallback` unconditional.
- `src/asset/hot_reload.rs` — native-gated `decode_image_with_state`, `compile_script_file`, `asset_key`.
- `src/app/schedule.rs` — native-gated `write_crash_log`.
- `src/app/render.rs` — native-gated `ensure_intermediate_texture`; 3 call sites use `FrameContext`.
- `src/renderer/sprite.rs` — new `FrameContext`; `render` signature; inline `mod tests` removed → `mod tests;`.
- `src/renderer/sprite/ui_primitives.rs` — deleted 2 dead wrappers; `render_ui_primitives_from_slices` uses `FrameContext`.
- `src/renderer/mod.rs` — `pub use sprite::{FrameContext, SpriteRenderer};`.

### Tests
- `src/renderer/sprite/tests.rs` — NEW; 5 sprite tests moved out of `sprite.rs` (imports re-pathed via `super::textures`/`super::ui_primitives`).

### Plan / handoff
- `/Users/jkl/.claude/plans/curried-growing-wreath.md` — severity-ordered fix plan (approved).
- This handoff.

### Config
- None. No Cargo/dependency changes.

## User Feedback & Preferences (REQUIRED — never omit)

- User works in Korean; wants responses in Korean. (Handoff artifact stays English per doc-language rule.)
- "이전에 리팩토링 작업 했는데 코드 전체 살펴보고 문제있는 부분 알려줘" — wanted a full-codebase review of the prior refactor, not just a narrow check.
- Used `/grill-me` and asked for a **severity-ordered** fix plan ("치명도 순서대로 수정 계획 세워줘").
- In grill-me chose: fix all 3 tiers; clippy via real struct refactor (not `#[allow]`); extract sprite tests to `tests.rs`; **delete** the dead wrappers.
- After the plan, expected implementation to proceed (approved via plan mode), then asked for a **second** full review ("전체 소스코드 다시 한번 보고 문제점 없는지 알려줘") — values thoroughness/double-checking.
- "커밋 하고 푸쉬 해줘" — explicitly authorized commit + push (and had already committed `61c09f1` themselves).
- Invoked `/handoffplan` to close the session with a plan for the next one.
- Memory note: user prefers aggressive subagent use for parallel work (Sonnet) — but the harness in this session disallows spawning agents unless explicitly asked, so work was done inline.

## Where We're Going

The core regression-fix goal is DONE and pushed. Remaining items are **optional follow-ups** carried over from parent + this session (these become the PLAN's phases):

1. **Close the LOCAL verification gap (CI already covers wasm+clippy — do NOT re-add it there).** Document in `CLAUDE.md`/`AGENTS.md` that a refactor is "done" only after the CI-equivalent runs locally: `cargo clippy --all-targets -- -D warnings`, `cargo build --target wasm32-unknown-unknown`, `cargo test --all-targets`. Root cause was a local bar of just `fmt --check` + `test --lib` while the work sat unpushed. Cheap, high-leverage.
2. **Deep line-by-line review of the 2 large unread files**: `src/app/editor/ui.rs` (963 lines) and `src/ecs/world.rs` (909 lines). Build/clippy/tests/public-API/CI are green, so risk is low, but neither was read end-to-end this session.
3. **Optionally split `src/app/editor/ui.rs`** (963 lines, near the 1000 ceiling) — parent's deferred item.
4. **Optionally split `src/audio.rs`** (771 lines, confirmed) — parent's lower-priority deferred item.
5. **Optionally mention new module dirs in `CLAUDE.md`** (parent only updated `AGENTS.md` + `ARCHITECTURE.html`).

## Risks & Blockers

- **Recurrence risk:** CI already enforces clippy `-D warnings` + wasm build, so any regression that reaches `main` is caught. The residual risk is purely LOCAL — declaring "done" on a narrow local check (`fmt --check` + `test --lib`) while work is unpushed, exactly what bit seq 1. Phase 1's local-bar doc closes that. Low severity now that the fix is pushed and CI-green.
- The 195-line inline test removal in `sprite.rs` was done with a verified Python `re.subn` splice (with `assert n == 1` guards), not the Edit tool, because a contiguous 195-line delete is impractical for exact-match Edit. If re-running anything similar, prefer the same guard-with-assert approach over `sed`.
- `cargo fmt` reordered the `hot_reload.rs` imports after my edits (cosmetic, expected) — don't be surprised by that diff vs the raw edit.
- Low overall: everything is committed, pushed, and green on native + wasm lib.
- `src/app/editor/ui.rs` and `src/ecs/world.rs` were NOT read line-by-line; a subtle behavior change from the split could still hide there (tests pass, so any such change is untested-path only).
- The parent handoff contains inaccurate `FrameContext` claims (see "Since Last Handoff") — don't trust seq-1's public-surface notes without re-checking the code.
- wasm `--all-targets` will always "fail" on the 3 native-only examples; don't treat that as a regression. Use `--lib` for wasm gating checks.

## Open Questions

- wasm build + clippy `-D warnings` are ALREADY in CI (`.github/workflows/ci.yml`). Open: should the **local** "done" bar be documented in `CLAUDE.md`/`AGENTS.md` (Phase 1), and/or add a `make verify`/`xtask` convenience that runs the CI-equivalent locally?
- Should `editor/ui.rs` be split now or left until it actually crosses 1000 lines?
- Is the `FrameContext` public re-export (`engine::renderer::FrameContext`) acceptable long-term, or should sprite render plumbing be made fully private (crate-internal) for the fork-friendly API surface?

## Quick Start for Next Session

```bash
# Restore context
cat plans/handoffs/HANDOFF_source-split-refactor_regression-fixes_2026-06-03.md
cat plans/handoffs/HANDOFF_source-split-refactor_source-file-split_2026-06-03.md   # parent (note: FrameContext claims are stale)

# Confirm clean, green starting state (everything is already committed + pushed)
git status -s
git log --oneline -3            # expect 61c09f1 at top, == origin/main
cargo build --target wasm32-unknown-unknown    # must stay clean
cargo clippy --all-targets                      # 0 warnings
cargo test --lib                                # 269 pass

# Key files for the most likely next work (deep review / verification hardening)
sed -n '1,80p' src/app/editor/ui.rs
sed -n '1,80p' src/ecs/world.rs
sed -n '1,40p' CLAUDE.md          # the wasm invariant lives here

# Confirm CI is green on the pushed commit (it was, this session)
gh run list --branch main --limit 3

# Next action
# Phase 1: document the LOCAL "done" bar (CI-equivalent: clippy --all-targets -D warnings,
# build --target wasm32, test --all-targets) in CLAUDE.md/AGENTS.md. CI already enforces it
# post-push; this stops the local-only narrow-check failure that caused seq 1. See PLAN Phase 1.
```
