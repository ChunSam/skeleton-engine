# macOS live window-drag freeze mitigation (deferred item A.1) — merged as v1.2.1

**Date:** 2026-06-03
**Status:** COMPLETED — PR #5 merged to `main` (`6cf4ac0`), v1.2.1 released, branch deleted
**Bead(s):** none (no beads system in this repo)
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `settings-ui` seq `3`
**Parent:** `HANDOFF_settings-ui_engine-ui-fixes_2026-06-02_2.md` (seq 2)
**Prior chain:** `HANDOFF_settings-ui_2026-06-02.md` (seq 1) > `HANDOFF_settings-ui_engine-ui-fixes_2026-06-02_2.md` (seq 2) > this (seq 3)

---

## Since Last Handoff

Parent (seq 2) closed v1.2.0 and said "await a new direction; if continuing engine work, the
deferred items — **macOS latency**, overlay caret, TextInput h-scroll, real fullscreen — or a
fresh NEXT_WORK dogfooding candidate (lighting)." This session is the direct follow-on:

- User picked **bucket A (the deferred follow-ups) first**, then narrowed (via `/grill-me`) to the
  **macOS input-latency** item specifically.
- Grill resolved "latency" → the **live window-drag content freeze**, not the general "half-beat"
  input latency (which stays a separate deferred item).
- Outcome: the freeze is **fixed and merged (v1.2.1)**. One parent deferred item closed; three
  remain (overlay caret, TextInput h-scroll, real fullscreen).
- Parent's open question about whether `AutoNoVsync` helps was already settled (no) in seq 2; this
  session did **not** touch present mode (stayed `AutoVsync` + `frame_latency=1`).

## Reference Documents

- `CLAUDE.md` — project conventions, module map (version header now `v1.2.1`)
- `docs/VISION.md` — dogfooding loop ("a feature/fix isn't done until a playable example exercises it")
- `docs/NEXT_WORK.md` — dogfooding candidate backlog + deferred-item follow-up note (updated this session)
- `docs/PATTERNS.md` — architecture patterns (system order, borrow workaround)
- Approved plan for this session: `~/.claude/plans/a-linked-matsumoto.md`

## The Goal

Take the highest-priority deferred follow-up from the settings-ui dogfooding chain — the macOS
input latency the user felt while QA-playing `settings_menu_game` — and resolve it within a
bounded "known levers" scope. Per the vision's dogfooding loop, prove the fix by making it
observable in a real playable example and measuring it, then document whatever residual remains
at the winit/OS limit. End state: a merged, released engine improvement (v1.2.1) with the example
extended to surface the behavior, and the deferred-items ledger updated.

## Locked Scope (from grill_decision_packet — plan file is gitignored)

The approved plan lives at `~/.claude/plans/a-linked-matsumoto.md` (personal, not in repo), so the
binding scope is captured here for the next session:

- **Goal:** mitigate the macOS live-window-drag content freeze (resize + titlebar move) using known
  winit/wgpu levers; prove via input-path regression tests + instrumentation logging + example play.
- **Primary invariant:** no engine-wide regression — `AutoVsync` + `frame_latency=1` stay default;
  `rust-survivors` builds unchanged; no tearing / battery regression.
- **Success criteria:** the example's animated element keeps updating during resize-drag (was
  frozen); frame-gap log shows measurable before/after; 260 lib tests + clippy clean. **All met.**
- **Non-goals (explicitly out):** present-mode / `frame_latency` changes; the general "half-beat"
  input latency (separate deferred item); other A items (caret / h-scroll / fullscreen); winit fork
  or native Cocoa hooks; a background redraw thread.
- **Stop condition:** apply known levers + measure; if further gains hit winit 0.30 / macOS modal-loop
  limits, document the residual and stop. A measurable resize-drag improvement on the first attempt
  counts as success. → **Triggered: stopped after measurement confirmed the freeze is gone.**

## Where We Are

- Working tree **clean**, on `main` at `6cf4ac0` (= `origin/main`, ahead/behind 0/0). Branch
  `fix/macos-drag-freeze` merged via merge commit and **deleted** (local + remote + pruned).
- **PR #5 merged.** All 4 CI checks passed: Test (native) 2m21s, Build (WASM) 29s, Rustdoc 36s,
  Package dry-run 4m55s. `mergeStateStatus: CLEAN`.
- **v1.2.1** in `Cargo.toml` (line 3), `Cargo.lock`, `CLAUDE.md` header (line 3); `docs/CHANGELOG.md`
  has a `## 1.2.1` section (was authored as `## Unreleased` then promoted at merge-decision time).
- `cargo test --lib` = **260 passed** (unchanged from seq 2; this fix added no unit tests — latency
  is not unit-testable, proof was instrumentation + manual play, per agreed proof bar).
- `cargo fmt`, native `cargo clippy --all-targets -- -D warnings` = clean.
- **wasm** `cargo clippy --target wasm32-unknown-unknown --lib --example settings_menu_game -- -D warnings`
  = clean (was **red** before this session on pre-existing toolchain drift; fixed here).
- `cargo build --target wasm32-unknown-unknown` (lib + example) = passes.
- `rust-survivors` (`/Users/jkl/Projects/rust-survivors`) `cargo check` = clean against engine 1.2.1
  (additive, no public API change).
- **Engine fix** (`src/app.rs`): `RedrawRequested` body extracted into new `fn step_frame(&mut self, event_loop: &ActiveEventLoop)`; called from both `RedrawRequested` and inline from `WindowEvent::Resized` (native-gated).
- **`pre_present_notify`** added: `self.window.pre_present_notify()` immediately before `frame.present()` in the main render path (`src/app.rs` ~2611).
- **Instrumentation**: `step_frame` does `if dt > 0.033 { log::debug!("frame gap {:.1}ms (drag/stall?)", dt*1000.0) }`.
- **Logger gap discovered + fixed**: the repo had `log = "0.4"` but **no logger backend anywhere** — `log::debug!` was a silent no-op on native. Added `env_logger` as a **native-only dev-dependency** and `env_logger::try_init()` (native-gated) at the top of `settings_menu_game::main`.
- **Example** (`examples/games/settings_menu/settings_menu.rs`): added `SpinnerSystem` + `SPINNER_FRAMES`, a `dt`-driven bottom-left spinner pushed into all 3 scenes before `UiSystem`.
- **Pre-existing wasm clippy drift fixed** (clippy 1.95.0): `src/lib.rs` (run_demo casts), `src/app.rs` (PENDING_GPU thread_local), `src/asset.rs` (WASM_ASYNC_QUEUE thread_local + type_complexity).
- Measurement done: `RUST_LOG=engine=debug` drag session logged only 3 `frame gap` lines (91.0, 65.6, 35.4 ms) — **no sustained gaps** → freeze gone, sustained drag smooth.
- Frame-gap threshold = `0.033` s (~30 fps). Spinner color `[120,230,160,255]` (green), font 18, text `"{frame}  live {:.1}s (drag the window)"`.
- All 3 example scenes (Title / Settings / Dialogue) push `SpinnerSystem::new()` — Title after `LocalizationSystem`, Settings/Dialogue after `LayoutSystem` — each immediately before `UiSystem`.
- Branch cleanup: `gh pr merge 5 --merge --delete-branch` (remote + local deleted) + `git fetch --prune` (removed stale `remotes/origin/fix/macos-drag-freeze` ref). Verified: no `*macos-drag*` refs remain.
- No memory files (`~/.claude/.../memory/`) were written this session; existing project/feedback memories already cover the dogfooding loop, doc-language rule, and subagent preference.

## What We Tried (Chronological)

1. **Orientation.** Read parent seq-2 handoff. Presented next-direction options (deferred A items vs fresh dogfooding). User chose **A first**.
2. **`/grill-me` (multi-round, Korean).** Pinned scope before planning:
   - Round 1: which A item → **macOS 입력 지연**; proof bar → **예제 플레이 + 테스트 둘 다**.
   - Round 2: target symptom → **창 드래그 중 멈춤** (not general half-beat latency); test half → **입력 경로 회귀 + 계측 로그**; stop condition → **알려진 레버 적용 후 한계면 문서화·종료**.
   - Round 3: drag type → **리사이즈 + 이동 둘 다**; proof surface → **settings_menu에 움직이는 요소 추가** (not a new example).
   - Emitted `grill_decision_packet` (plan_allowed: true), wrote plan `~/.claude/plans/a-linked-matsumoto.md`, ExitPlanMode → approved.
3. **Root-cause read.** `Resized` handler (app.rs:2701) only called `gpu.resize(size)` — no inline draw. winit parks `about_to_wait → request_redraw → RedrawRequested` during the macOS modal drag loop. `pre_present_notify` was never called. → fix = inline frame on Resized + present hint.
4. **Implementation.** Extracted `step_frame`, called from Resized (native); added `pre_present_notify`; added frame-gap `log::debug!`; added `SpinnerSystem` to the example, wired before `UiSystem` in all 3 scenes (scenes reset the world + clear systems on `SceneCmd::Replace`, so the spinner lazily re-spawns its own label each scene via `world.is_alive`). Builds + 260 tests + native clippy + wasm build all green.
5. **First measurement run — FAILED to capture.** `RUST_LOG=engine=debug cargo run` (background) produced **zero** `frame gap` lines. Diagnosed: **no logger backend installed** (only `log` facade) → `log::debug!` silently dropped on native. (Earlier wasm builds were fine; this was purely a native logging gap.)
6. **Logger fix.** Added `env_logger = "0.11"` under `[target.'cfg(not(target_arch = "wasm32"))'.dev-dependencies]` (target-gated so wasm never compiles it) + `#[cfg(not(target_arch = "wasm32"))] let _ = env_logger::try_init();` in `main`. Rebuilt; wasm build re-verified clean.
7. **Second measurement run — user dragged.** Captured 3 lines: `91.0ms`, `65.6ms` (same second, drag/startup), `35.4ms` (26 s later, threshold-grazing). No sustained gap stream → confirmed sustained drag is smooth; residual is a transient start-of-drag hitch.
8. **fontmaker cross-project investigation** (user asked: "did I already solve this in fontmaker (Swift)?"). Located `/Users/jkl/Projects/fontmaker`, read `plans/handoffs/HANDOFF_sheet-editor-performance_2026-05-31.md` + `SheetCanvasView.swift`. Their fix was **redraw-cost** (per-pixel `Path` over ~1.4M pixels → raster image cache + `sheetVersion` dirty flag + no-op stroke skip). **Concluded: not applicable** — our wgpu redraw is already cheap (proven by the absence of sustained gaps); our residual is present/reconfigure timing, not redraw cost.
9. **Decision: stop** (per stop condition). User confirmed "종료로 마무리".
10. **wasm clippy discovery.** While verifying for PR, found wasm clippy `-D warnings` was red on **5** pre-existing drift lints (not just the `as f32` I'd initially named). User approved fixing all in this PR.
11. **Wrap-up.** Version bump 1.2.1, fixed all 5 clippy sites, full re-verify, commit, push, PR #5, watched CI to green, merged with `--delete-branch`, pruned, synced main.

## Key Decisions

- **Continuation, not new chain.** This session executes a named next-action from parent seq 2 (the deferred macOS latency item), so chain `settings-ui` seq 3.
- **Scope = drag-freeze only, both resize + move.** Rejected the broader "all macOS latency" framing; the general half-beat latency stays a separate deferred item.
- **Proof surface = extend `settings_menu`, not a new example.** Keeps example count down; the spinner is `dt`-driven and visible in all scenes.
- **`env_logger` as a target-gated *dev*-dependency**, not a runtime dep and not in the library. Rejected putting a gated `eprintln!` in the engine hot path (less idiomatic); rejected an un-gated dev-dep (would try to compile env_logger for wasm and risk breaking the wasm build gate). Keeps `log::debug!` idiomatic in the engine, RUST_LOG works as documented.
- **Fix ALL 5 wasm clippy drift lints, not just the `as f32`.** User's approved intent was a green wasm CI gate; the as-f32 was only my incomplete characterization. All 5 are pre-existing, wasm-cfg-gated, mechanical.
- **`type_complexity` via `#[allow(...)]`** on the thread_local (rejected a public type alias — it's an internal wasm queue, not worth surfacing).
- **Version 1.2.1 (patch bump).** Additive bugfix/perf, no breaking change. Authored CHANGELOG as `## Unreleased` first, promoted to `## 1.2.1` only after the user confirmed the bump (decision deliberately deferred out of the plan's scope).
- **Merge commit (`--merge`), not squash.** Mirrors PR #3/#4 in this repo.
- **Documented residual honestly + corrected my own doc.** I had pre-written CHANGELOG "Known limitations" as "titlebar move drag still freezes." Measurement + user report showed move-drag also keeps animating; rewrote it to "one-frame lag at the *start* of a drag (both kinds)."

## Evidence & Data

### Frame-gap measurement (`RUST_LOG=engine=debug`, full drag session)
| Timestamp | Gap | Interpretation |
|---|---|---|
| 16:49:29Z | 91.0 ms | start-of-drag / startup spike |
| 16:49:29Z | 65.6 ms | start-of-drag / startup spike (same second) |
| 16:49:55Z | 35.4 ms | isolated, threshold(33ms)-grazing single frame |

Key signal: **only 3 gaps, none sustained.** A modal-loop park would log gaps spanning the whole
drag. Continued dragging produced **no** >33 ms frames → smooth. Matches user report exactly:
"첫 이동은 늦게 따라오고, 이후 지속적인 이동에는 정상." (Did NOT run the optional 16 ms-threshold
re-check to disambiguate startup vs drag-start — user chose to stop.)

### fontmaker vs skeleton-engine (why the Swift fix doesn't port)
| | fontmaker (Swift/SwiftUI) | skeleton-engine (Rust/wgpu) |
|---|---|---|
| Symptom | lag throughout drag/resize | freeze gone; transient start-of-drag hitch only |
| Root cause | per-frame redraw cost (~1.4M-px `Path`) | present/reconfigure timing (redraw already cheap) |
| Fix | raster image cache + `sheetVersion` dirty flag | inline render on Resized + `pre_present_notify` |
| Transferable? | No — our redraw isn't the bottleneck | — |

### Pre-existing wasm clippy drift fixed (clippy 1.95.0, all `#[cfg(target_arch="wasm32")]` code)
| Site | Lint | Fix |
|---|---|---|
| `src/lib.rs:160` (run_demo) ×2 | `unnecessary_cast` (f32→f32) | `(v.width as f32, v.height as f32)` → `(v.width, v.height)` |
| `src/app.rs:14` (PENDING_GPU) | thread_local can be const | `= const { std::cell::RefCell::new(None) }` |
| `src/asset.rs:133` (WASM_ASYNC_QUEUE) | thread_local can be const | `= const { RefCell::new(VecDeque::new()) }` |
| `src/asset.rs:132` (WASM_ASYNC_QUEUE) | `type_complexity` | `#[allow(clippy::type_complexity)]` on the static |

### Residual documented in CHANGELOG `## 1.2.1 → Known limitations` (verbatim)
> A one-frame lag remains at the **start** of a live drag (both resize and titlebar move): the
> window content tracks the cursor a beat late on the first movement, then follows normally for
> the rest of the drag. The hard freeze is gone — content keeps animating throughout both drag
> kinds on the tested macOS (15.x / Darwin 25) — but this residual start-of-drag latency is a
> macOS/winit present-timing artifact left as a documented limitation per the "known levers"
> scope (deeper fixes — background redraw thread / native Cocoa hooks — were out of scope).

### Commit / PR
| Item | Value |
|---|---|
| Fix commit | `919c485` fix(app): mitigate macOS live-resize drag freeze (v1.2.1) |
| Merge commit | `6cf4ac0` Merge pull request #5 |
| PR | #5 → base `main`, head `fix/macos-drag-freeze` (deleted) |
| Diff stat | 9 files, +303 / −50 (Cargo.lock +133 = env_logger transitive deps) |

### Verification matrix (final, all green)
| Check | Result |
|---|---|
| `cargo test --lib` | 260 passed |
| `cargo fmt` / native `clippy --all-targets -D warnings` | clean |
| wasm `clippy --lib --example settings_menu_game -D warnings` | clean (was red) |
| wasm build (lib + example) | passes |
| `rust-survivors` cargo check | clean (additive) |
| CI (Test/WASM/Rustdoc/Package) | all pass, CLEAN |

### env_logger wiring (the bit that made instrumentation actually work)
The repo had `log = "0.4"` but **no backend**, so all `log::*` was dropped on native. Fix, kept
wasm-safe via target-gating so the wasm build/clippy gates are untouched:
```toml
# Cargo.toml — native-only dev-dependency
[target.'cfg(not(target_arch = "wasm32"))'.dev-dependencies]
env_logger = "0.11"
```
```rust
// examples/games/settings_menu/settings_menu.rs — top of main()
#[cfg(not(target_arch = "wasm32"))]
let _ = env_logger::try_init();
```
`try_init()` (not `init()`) avoids a panic if a backend is ever already set. `Cargo.lock` grew +133
lines from env_logger transitives (anstyle, anstream, env_filter, regex, regex-automata) — native
dev-only, absent from the published lib and wasm.

### Exact verification commands (all green, re-runnable)
```bash
cargo test --lib                                                   # 260 passed
cargo fmt && cargo clippy --all-targets -- -D warnings             # native, clean
cargo clippy --target wasm32-unknown-unknown --lib \
    --example settings_menu_game -- -D warnings                    # wasm, clean (was red)
cargo build --target wasm32-unknown-unknown --lib \
    --example settings_menu_game                                   # wasm build, passes
( cd /Users/jkl/Projects/rust-survivors && cargo check )           # downstream, clean
```

## Code Analysis

- **`App::step_frame(&mut self, event_loop: &ActiveEventLoop)`** (`src/app.rs`): computes `dt` (`min(0.1)` clamp), frame-gap log, `update(dt)`, `ShouldQuit` check (`event_loop.exit(); return;`), `PendingResize` handling, `render()` with `SurfaceError::Lost|Outdated` → `reconfigure()`. The single source for a frame; called from `RedrawRequested` and `Resized`.
- **`WindowEvent::Resized`** (`src/app.rs:2701`): `gpu.resize(size)` then `#[cfg(not(target_arch="wasm32"))] self.step_frame(event_loop);`. wasm unchanged (no modal-loop issue there).
- **present sites:** `src/app.rs` ~2611 (main render; `pre_present_notify` added here) and `src/renderer/context.rs:190` (clear path; no window handle there, left as-is — only the initial solid clear).
- **Borrow note:** in `render()`, `gpu = self.gpu.as_mut()` is dead by the present point (NLL), so `self.window` (disjoint field) is freely accessible for `pre_present_notify`.
- **Scene reset semantics** (`src/app.rs:890` `apply_scene_cmd`): `SceneCmd::Replace` drains scene stack, `self.systems.clear()` (896), `reload_scene()` (world reset), then `new_scene.on_enter`. So entities AND systems are per-scene → `SpinnerSystem` is pushed by each scene and lazily re-spawns its label (checks `world.is_alive`).
- **System order in example:** spinner pushed after `LocalizationSystem`/`LayoutSystem` but **before `UiSystem`**, so `UiSystem` consumes the updated `Label.text` the same frame. `Label.text` is a public field (`src/ui/label.rs:8`); updated via `world.get_mut::<Label>(e)`.
- **Versions:** winit 0.30.13, wgpu 22.1.0. `present_mode: AutoVsync`, `desired_maximum_frame_latency: 1` (unchanged — explicitly out of scope).
- **`SPINNER_FRAMES: [&str;4] = ["|","/","-","\\"]`**, advanced by `((self.elapsed*8.0) as usize) % 4`; node `UiNode::new(12.0,-12.0,260.0,26.0).with_anchor(Anchor::BottomLeft).with_z(0.99)`.
- **`SpinnerSystem { label: Option<Entity>, elapsed: f32 }`**: each frame `elapsed += dt`; if `self.label` is `None` or `!world.is_alive(e)`, spawn a fresh `UiNode + Label` and store the entity; then set `Label.text` via `world.get_mut::<Label>(e)`. The liveness check is what makes it survive a `SceneCmd::Replace` world reset without per-scene bookkeeping. `Anchor::BottomLeft` resolves to `(0, vh-h)` then adds the node's `(x,y)`, so `(12,-12)` insets 12 px up-and-right from the bottom-left corner (mirrors the `node.rs` test at line 93).
- **Move-drag surprise:** the plan predicted titlebar-move would stay frozen (no `Resized`, modal loop). Empirically on this macOS the spinner kept animating during move too → `RedrawRequested` is evidently still serviced during move on this winit 0.30.13 + Darwin 25 combo. So the pre-written "move drag freezes" limitation was **wrong** and was corrected to "one-frame start-of-drag hitch (both kinds)".

### fontmaker artifacts read (for the cross-project check)
- `/Users/jkl/Projects/fontmaker/plans/handoffs/HANDOFF_sheet-editor-performance_2026-05-31.md` — root cause line: "the main lag source was interactive redraw cost in `SheetCanvasView`" → `Canvas` iterated ~1.4M pixels per redraw filling a `Path` per black pixel.
- `/Users/jkl/Projects/fontmaker/editor/Sources/FontmakerSheetEditorApp/SheetCanvasView.swift` — confirmed SwiftUI `Canvas` (CPU), no Metal/`CAMetalLayer`/`displayLink`/window-level present trick. Nothing transferable to a wgpu engine.

## Files Changed

### Source code
- `src/app.rs` — `step_frame` extraction; `Resized` inline frame (native); `pre_present_notify`; frame-gap `log::debug!`; PENDING_GPU `const {}`.
- `src/lib.rs` — removed unnecessary f32 casts in `run_demo` (wasm-only).
- `src/asset.rs` — WASM_ASYNC_QUEUE `const {}` + `#[allow(clippy::type_complexity)]`.

### Example
- `examples/games/settings_menu/settings_menu.rs` — `SpinnerSystem` + `SPINNER_FRAMES`; pushed in all 3 scenes (Title/Settings/Dialogue) before `UiSystem`; `env_logger::try_init()` (native) in `main`.

### Build / docs
- `Cargo.toml` — version 1.2.0→1.2.1; `env_logger = "0.11"` native dev-dependency.
- `Cargo.lock` — version bump + env_logger transitive deps.
- `CLAUDE.md` — header v1.2.0→v1.2.1.
- `docs/CHANGELOG.md` — new `## 1.2.1` (Fixed/Added/Known limitations).
- `docs/NEXT_WORK.md` — deferred-item follow-up note (macOS latency largely addressed; residual + remaining deferred).

### Plan (gitignored / personal)
- `~/.claude/plans/a-linked-matsumoto.md` — approved implementation plan for this session.

## User Feedback & Preferences

- **Korean conversation, English docs/artifacts** (project doc-language rule). Held throughout.
- Chose to **grill before planning** ("a먼저 진행 하는 계획 진행") — wanted scope pinned, not a fast plan.
- **QA by playing on macOS, reports precisely per-symptom:** "드래그 해도 스피너는 계속 돌고있음... 창 리사이즈가 첫이동은 늦게 따라오고, 이후 지속적인 이동에는 정상." This precision drove the start-of-drag-vs-sustained distinction.
- Brought a **cross-project lead** ("fontmaker 프로젝트에서 동일한 문제 해결한적있음... swift로 개발한 부분이라 rust에서도 동일하게 적용 될지 모르겠지만 한번 살펴봐") — expects me to actually check, not hand-wave.
- Asked a side question: **"rust에서 mac 프로그램 app 빌드하는 전용 빌더를 준비해놓지는 않았어?"** → answered No (only WASM scripts); flagged `cargo-bundle`/`cargo-packager` as a separate future task.
- **Decisive on closing:** "종료로 마무리"; "CI 통과하면 머지하고 브랜치 정리해줘". Comfortable delegating the merge + cleanup once CI is green.
- Pragmatic about scope/stop conditions (consistent with seq 2's "나중으로 미룸" style).

## Where We're Going

- **Remaining deferred items** from the settings-ui chain (none scheduled): overlay caret (renderer caret-quad — glyphon has no quad pipeline today), `TextInput` horizontal scroll (single-line clips long text; IME at `max_len` uncommittable), real OS fullscreen (checkbox is preference-only).
- **macOS `.app` bundler** — user-raised, separate task: `cargo-bundle` or `cargo-packager` + `[package.metadata.bundle]` + `Info.plist` (icon, `NSHighResolutionCapable`). Not started.
- **Optional latency depth** (only if user wants): lower frame-gap threshold to ~16 ms and re-measure to confirm whether the start-of-drag hitch is swapchain reconfigure vs OS modal-loop entry. Deeper fixes (background redraw thread / native Cocoa) were declared out of scope.
- **Fresh dogfooding candidates** (`docs/NEXT_WORK.md`): 2D lighting (`PointLight`/normal-map — most visually striking untested), `BlendTree1D`, `Timeline`/cutscene, `PostProcessConfig`, physics joints, `RenderTarget` in real play, networking.
- Direction is **user-gated** — await next instruction.
- Recommendation if asked: 2D lighting is the highest-value *new* dogfooding subsystem; among the leftover deferred items, `TextInput` horizontal scroll is the most self-contained (no new renderer pipeline), the overlay caret is the biggest (needs a glyphon quad path), real fullscreen is the smallest.

## Risks & Blockers

- **None blocking.** Work is merged and released.
- **clippy toolchain drift** will keep surfacing new lints on wasm-only code as the toolchain advances (this session fixed clippy 1.95's batch). Watch on future PRs.
- Residual start-of-drag hitch is a **winit 0.30 / macOS limitation**, not a regression — documented.
- `Resized` now drives a full `step_frame` (update + render) per resize event. On a very heavy game this adds per-resize-event update cost; acceptable here (dt-clamped, GPU render cheap) but worth remembering if a future game reports resize-time CPU spikes.
- The instrumentation `log::debug!` only fires if a downstream binary installs a `log` backend. The example wires `env_logger`; other binaries (incl. `rust-survivors`) won't see it unless they do too.

## Open Questions

- Is the start-of-drag hitch swapchain reconfigure cost or OS modal-loop entry latency? Not disambiguated (the 16 ms-threshold re-run was offered but the user chose to stop). Non-blocking.
- Would a `.app` bundle (with `Info.plist` `NSHighResolutionCapable` etc.) change any of the residual drag behavior? Unknown — bundling was raised by the user as a separate task and not investigated here. Non-blocking.

## Quick Start for Next Session

```bash
# On main, v1.2.1 merged.
cd /Users/jkl/Projects/skeleton-engine
git switch main && git pull          # expect HEAD = 6cf4ac0 or later
cargo test --lib                     # expect 260 passed

# See the drag-freeze fix + instrumentation in real play:
RUST_LOG=engine=debug cargo run --example settings_menu_game
#   bottom-left spinner stays animating during resize AND titlebar-move drags;
#   "frame gap …ms" logs only at drag start (residual hitch), not sustained.

# Reference docs
#   docs/VISION.md, docs/NEXT_WORK.md, CLAUDE.md
#   plans/handoffs/HANDOFF_settings-ui_engine-ui-fixes_2026-06-02_2.md (parent)

# Key files to read first
#   src/app.rs (step_frame, Resized, present sites)
#   examples/games/settings_menu/settings_menu.rs (SpinnerSystem, env_logger)
#   docs/CHANGELOG.md (## 1.2.1 — Known limitations)

# Next action (await user direction). Most likely candidates:
#   (a) next deferred item — overlay caret / TextInput h-scroll / real fullscreen
#   (b) macOS .app bundler (cargo-bundle / cargo-packager) — user-raised, not started
#   (c) fresh dogfooding subsystem — 2D lighting is the highest-value untested one
```

## Session Closed
**Closed at:** 2026-06-02T17:12Z
**Commit:** 212eca3 (amended)
**Session status:** Handed off to next session
