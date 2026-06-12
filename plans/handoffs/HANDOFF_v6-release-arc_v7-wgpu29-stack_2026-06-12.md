# v7.0.0 shipped in one evening session — wgpu 29 / glyphon 0.11 / egui 0.34 stack, CI toolchain 1.95, RUSTSEC-2026-0002 closed, playtest caught a real latent bug, consumer migrated

**Date:** 2026-06-12
**Status:** COMPLETED — PR #20 merged, engine at v7.0.0 (`a3369ee`), rust-survivors migrated (`960b43c`, local commit, push is the user's). The structural-debt queue is now EMPTY (the wgpu dep-major was the last known item). Next session starts fresh: new feature work per VISION, user picks.
**Bead(s):** none (`bd` not installed — exit 127, seventh session running; tracked via in-session Task tools #1–#5, all completed)
**Epic:** v6-release-arc — continuation: this session executed "Where We're Going" item 3 (wgpu/glyphon dep-major) and item-4 deletion from the parent
**Chain:** `v6-release-arc` seq `2`
**Parent:** `HANDOFF_v6-release-arc_v5.1-to-v6-shipping_2026-06-12.md` (seq 1)
**Prior chain:** `HANDOFF_v6-release-arc_v5.1-to-v6-shipping_2026-06-12.md` > this

---

## Stale References

Identifiers/facts from the parent that no longer hold (parent was written this morning; this session changed them):

- `cargo +1.88.0` (parent's Quick Start and gate commands throughout) — **CI pin is now 1.95.0**; gate with `cargo +stable` while local stable == 1.95 (see `ci-toolchain-pin` memory, rewritten this session)
- `wgpu::SurfaceError` — type no longer exists (wgpu 29 removed it; surface acquisition now returns the `wgpu::CurrentSurfaceTexture` enum)
- rust-survivors "main = `801f334`, NOT pushed" — superseded: the user pushed their work between sessions; game HEAD before this session's bump was `0c42012` (their WIP commits "Flip actor sprites by movement direction", "Add stage tilemap backgrounds" landed); our new local-only commit is `960b43c`
- Game test baseline "206/0" — now **213/0** (user WIP added 7 tests between sessions; re-capture before judging, as the parent warned)
- Engine version "6.0.0" / `77b4465` — now **7.0.0** / `a3369ee`
- "egui_pass.rs unsafe transmute" (parent's render contract notes) — the unsafe block is GONE (`RenderPass::forget_lifetime()`)
- Parent's open item "fable5-as-subagent retest" — **permanently deleted by user decision**, not pending anymore

All other parent identifiers (builtin_tail_count, merge semantics, `Fade::stop_fade`, `is_clip_finished`, `path_arc`, HierarchySystem tail mechanics) verified still present and unchanged.

## Since Last Handoff

- Parent's "Where We're Going" listed: (1) user pushes game, (2) new feature work, (3) optional wgpu/glyphon dep-major, (4) optional fable5 retest, (5) optional recurring cloud review. The user picked **3** and explicitly killed **4** ("4번은 완전 삭제하고, 2번 진행" — option numbering in my menu differed from parent's list; my option 2 = the dep-major).
- Item 1 partially resolved itself: the user pushed rust-survivors between sessions (parent's `801f334` is now ancestry; their WIP source commits landed too).
- Parent's "Open Questions" #2 (wgpu/glyphon: schedule or keep accepting RUSTSEC risk?) — **answered: scheduled and shipped this session.** RUSTSEC-2026-0002 is closed, the SECURITY_HARDENING doc updated with strikethroughs + RESOLVED note.
- Parent's risk "REFERENCE.html drift" — no new drift: REFERENCE.html has zero version-specific wgpu/egui mentions (grep verified), so the dep bump needed no REFERENCE edit. CHANGELOG carries everything.
- Parent's risk "classifier merge checkpoint" — held exactly as documented: one fresh "머지 확인" unlocked the PR #20 merge, zero friction.
- The parent's playtest procedure (osascript/caffeinate/screencapture) was used again and **caught a real crash bug** that all compile/test gates missed — the strongest validation of the example-as-acceptance-test doctrine yet.
- New process learning the parent didn't have: the user rejects monolithic multi-step automation commands at permission prompts; splitting into small single-purpose commands was accepted immediately.

## Reference Documents

- `CLAUDE.md` (doc v1.6.3, updated this session) — module map, verification gates; header now says wgpu 29 / MSRV 1.92 / CI pin 1.95.0
- `docs/CHANGELOG.md` `## 7.0.0` — the canonical migration guide for this release (dep table, API breaks, the textures_delta fix)
- `docs/SECURITY_HARDENING_2026_05.md` — RUSTSEC-2026-0002 closure recorded (strikethrough + RESOLVED block)
- `docs/VISION.md` — the feature loop that governs what comes next (queue is empty)
- Memory: `engine-current-state` (rewritten 2× this session, final = v7 shipped), `ci-toolchain-pin` (fully rewritten for 1.95), `new-model-subagent-incompat` (retest permanently dropped), MEMORY.md (3 index lines updated)
- Parent handoff — the entire v5.1→v6 arc evidence (verification doctrine, classifier protocol, agent fleet patterns)

## The Goal

Clear the engine's last known structural debt: the renderer dependency stack was 7 wgpu majors behind (22 vs 29), which pinned glyphon at 0.6 and therefore `lru` at 0.12.5 — the unfixable-in-place `RUSTSEC-2026-0002` advisory that had been archived as accepted risk since 2026-05-29. The user chose this over new feature work when offered the menu. Scope grew once, by necessity not choice: egui 0.34's MSRV (1.92) forced the CI toolchain pin from 1.88.0 to 1.95.0 — there was no alternative stack (full matrix proof in Evidence). End state required: identical runtime behavior, all gates green, wasm + native playtests verified, consumer migrated.

## Where We Are

- **Engine `main` = `a3369ee`** "Merge pull request #20" (v7.0.0) — pushed, merged after CI 4/4 + user "머지 확인", branch `feat/v7-wgpu29-stack` deleted local+remote.
- **Branch state**: `main` + `docs/english-conversion` (not ours, preserved per standing rule). `git fetch --prune` this session also removed 5 stale remote-tracking refs left from the v6 arc's merges (analysis/full-review, cleanup/v5.1.3, feat/v6-breaking-api, fix/v5.1.1, fix/v5.1.2).
- **rust-survivors = `960b43c`** "deps: bump skeleton-engine pin 6.0.0 -> 7.0.0 (a3369ee)" — committed locally, **NOT pushed** (standing rule). 2 files only: `crates/game/Cargo.toml:20` rev + `Cargo.lock` (756+/180−). User WIP (22 unstaged paths) untouched.
- **Zero game source changes were needed** — measured before the work (zero grep hits for egui/debug_ui/DebugUi/ctx() in `crates/game/src/`) and confirmed by the compiler after the bump.
- Engine lib tests **391 → 393** (the +2 are `egui_delta_tests` regression tests for the playtest-caught bug).
- **Dependency stack shipped**: wgpu 22 → 29.0.3 (`webgl` feature unchanged), glyphon 0.6 → 0.11.0 (cosmic-text 0.18.2), egui/egui-wgpu/egui-winit 0.29 → 0.34.3, winit floor 0.30 → 0.30.13 (no major), transitive `lru` 0.12.5 → **0.16.4** (≥ 0.16.3 → RUSTSEC-2026-0002 no longer applies, verified in Cargo.lock before the code migration even started).
- **Toolchain**: CI yml pins ×4 → `dtolnay/rust-toolchain@1.95.0`; `rust-version = "1.92"` (true MSRV, driven by egui 0.34; cosmic-text 0.18 needs 1.89). 1.95.0 chosen over 1.92-minimum because it equals current local stable → local/CI rustfmt+clippy match exactly, killing the divergence trap the old `ci-toolchain-pin` memory existed for. Old codebase had **zero** new-lint churn under 1.95 (fmt + clippy clean with no code changes).
- **The migration churn**: ~70 lib compile errors across 13 files, all in `src/renderer/*` + `src/app/*` — fixed by one sonnet agent working in-place on the branch (no worktree: single agent, warm build cache; details in Evidence).
- **`src/app/egui_pass.rs` unsafe transmute REMOVED** — wgpu 29's `RenderPass::forget_lifetime()` is the supported replacement for egui-wgpu's `RenderPass<'static>` requirement.
- **Verification caught 2 agent errors + 1 latent engine bug** (the headline of this session — see What We Tried 7–10 and Evidence).
- `GpuContext::clear()` now returns `Result<(), String>` (SurfaceError gone) — this IS pub API (`pub use context::{GpuContext, GpuContextError}` in renderer/mod.rs:1,16), contrary to the migration agent's claim; documented as breaking in CHANGELOG.
- Surface-acquire semantics preserved exactly: `Success(t) | Suboptimal(t)` both render+present (matches wgpu 22 where suboptimal was a bool on the frame); `Lost | Outdated` → `gpu.reconfigure()` (render.rs:697-700 same shape as before).
- **wasm_smoke PASS** — 41913-byte screenshot (v6 reference was 41962B), eyeballed: coin_race HUD text crisp under glyphon 0.11, player/coins geometry correct, WebSocket connect logged.
- **Windowed playtests**: lit_dungeon (point-light falloff renders, torch HUD, post-fx key available, FPS 58-60), F1 inspector overlay (panicked → fixed → renders, Entities/Assets/Scene tabs visible, skrifa text crisp), sm_crossfade (caught mid-blend at 0.58, two-frame shader mix visually distinct from the hard-switch left panel).
- CHANGELOG `## 7.0.0` written (dep table + per-item migration + the Fixed section); CLAUDE.md header v1.6.3; Cargo.toml version 7.0.0.
- All 5 in-session tasks completed; no open work items anywhere.

## What We Tried (Chronological)

1. **Onboarding per the 5-step paste-prompt protocol**: read parent handoff, `bd list` (exit 127 again), verified both repo states matched parent exactly, read VISION/CHANGELOG-6.0.0/engine-current-state, adjacent-explored NEXT_WORK.md (candidates A–F all done — second source confirming the empty queue), Cargo.toml dep pins, SECURITY doc's glyphon section. Presented 4 numbered options and waited:
   1. 신규 기능 작업 (VISION loop, user picks genre → candidate table on request)
   2. wgpu/glyphon 메이저 마이그레이션 (RUSTSEC 해소, wasm_smoke mandatory)
   3. 주간 예약 리뷰 루틴 설치 (proven template, one cloud session per run)
   4. fable5 서브에이전트 재테스트 (성공 시 `model:` 강제 해제)
   The onboarding also surfaced the stale remote-tracking refs from the v6 arc's merges (noted then; pruned later in the same session after the PR #20 cleanup).
2. **User: "4번은 완전 삭제하고, 2번 진행"** — first action was the deletion: `new-model-subagent-incompat` memory rewritten ("Retesting is permanently dropped by user decision (2026-06-12)"), `engine-current-state` line replaced. The explicit-`model:`-on-subagents rule is now standing policy, not a workaround awaiting removal.
3. **Version research via crates.io API**: first attempt FAILED silently — crates.io rejects requests without a User-Agent header (classic gotcha; retried with `-A "skeleton-engine-dep-research"` and everything worked). Built the version + dependency-requirement matrix (Evidence table 1). Conclusion forced itself: **only one coherent stack exists** — RUSTSEC fix needs glyphon ≥0.10; glyphon 0.10 = wgpu 28 which has NO egui-wgpu release (0.33→wgpu 27, 0.34→wgpu 29); so wgpu 29 + glyphon 0.11 + egui 0.34 is the only option. No scope decision to ask the user about.
4. **MSRV check changed the plan**: crates.io `rust_version` fields — egui/egui-wgpu 0.34.3 = **1.92**, cosmic-text 0.18 = 1.89, wgpu 29.0.3 = 1.87. CI pinned 1.88.0 → toolchain bump unavoidable. Chose **1.95.0 (= local stable)** over 1.92-minimum: kills the local/CI fmt divergence permanently (until stable moves), zero extra installs. Verified old code is lint-clean under 1.95 BEFORE committing (fmt --check, clippy -D warnings, full test suite: 391/0, wasm, doc — all green). **Commit A `9fe5cff`** (ci.yml ×4 + rust-version only, 9 insertions).
5. **Parallel research agent** (sonnet, background, 91K tokens / 50 tool uses / 377s) compiled a wgpu 23→29 + glyphon 0.7→0.11 + egui 0.30→0.34 breaking-changes cheat sheet from upstream changelogs while I did commit A. Verdict on its accuracy after the fact: compile-break list ~90% useful; 3 claims wrong or N/A (SurfaceTexture::present removal — false, code compiles; egui 0.34 panel deprecations — never surfaced under -D warnings; InstanceDescriptor "constructors only" — struct literal with `display: None` works). Its real value was the [SILENT-BEHAVIOR] list — egui 0.34 skrifa font backend + text size 12.5→13.0 went straight into the playtest checklist.
6. **Dep bump + error enumeration in main session**: bumped Cargo.toml (uncommitted), `cargo check --all-targets` → **70 errors / 13 files**, классified (Evidence table 3). Pre-dispatch risk triage by direct grep: engine shaders have NO integer `@location`, NO `var<push_constant>`, NO `set_viewport`, NO `device.poll` → 4 of the cheat sheet's top-5 silent risks structurally absent. `lru` already resolved to 0.16.4 in the lock → RUSTSEC closure confirmed before any code fix.
7. **One migration agent in-place** (sonnet, 137K tokens / 127 tool uses / 803s — no worktree: a single agent on an interlocked compile-error set gains nothing from isolation and loses the warm cache). Prompt carried: full error inventory, behavior-preservation constraints (sRGB-first, AutoVsync, latency 1, WebGL2 limits, Backends::GL on wasm), pub-API shape rules, the explicit instruction to replace the egui_pass unsafe with `forget_lifetime()`, the docs-ban, and the full gate list. Agent delivered **commit `6109796`**, all gates green, 391/0, and a per-file + silent-decisions report. NOTE: tried to send a mid-flight addendum — **SendMessage does not exist in this environment** (Agent tool description mentions it; ToolSearch finds nothing) — converted the addendum into a post-hoc diff-review checklist instead.
8. **Verify-before-trust diff review caught two deviations** from the agent's "IDENTICAL BEHAVIOR" claims: (a) `egui_wgpu::RendererOptions::default()` = `{msaa_samples: 0, dithering: true}` vs the old positional args `(None, 1, false)` — msaa 0 verified harmless (egui-wgpu clamps `.max(1)` at renderer.rs:400) but **dithering true is a real silent visual change** → explicit options struct restored `dithering: false` (**commit `0010f64`**); (b) the claim "`GpuContext::clear()` is not exposed in lib.rs so no public API break" is **false** — `pub mod renderer` + `pub use context::{GpuContext, ...}` make it public; CHANGELOG documents the break. Confirmed-correct list: Suboptimal-frame presenting, Lost/Outdated→reconfigure, set_text trailing `Align=None` (per-line align loop after set_text preserved), entry_point Some-wrapping, immediate_size 0 ≡ empty push_constant_ranges.
9. **wasm_smoke + playtest round**: smoke PASS (eyeballed screenshot OK). The user rejected my monolithic 25-line osascript playtest command twice (first rejection → asked for a status report; after "중단된 부분부터 이어서 진행해" rejected the same shape again, then said **"플레이 테스트 너가 해"**) — re-split into 5 small single-purpose commands (launch / place window / screenshot / keys+screenshot / quit) which were all accepted. lit_dungeon shots 1-2 verified the lighting pass (ambient vignette; point-light radial falloff around the moving player; torch HUD 75%, FPS 58).
10. **F1 → CRASH → root cause → fix `c279588`**: pressing F1 (debug overlay on) panicked the app inside egui-wgpu 0.34.3 renderer.rs:669 "Tried to update a texture that has not been allocated yet". Root cause (by code reading, both engine and egui-wgpu source): the engine runs `ctx.begin_pass/end_pass` EVERY frame (egui_ctx is Some whenever the DebugUi resource exists — overlay visibility only gates panel drawing); `schedule.rs:304` **overwrote** `self.egui_output` each frame; `render.rs` consumes it at :614-615 — AFTER the surface-acquire early return at :209. One skipped render (surface Lost/Outdated/Timeout — e.g. the osascript window resize in my own playtest) drops that frame's `textures_delta` forever. Under egui 0.29/ab_glyph the next font-atlas change re-sent the FULL image → self-healing, bug invisible for the engine's whole life. Under 0.34/skrifa the atlas updates are incremental per-glyph partials (`pos: Some`) → losing the initial full set makes egui-wgpu's `self.textures.remove(&id).expect(...)` panic on the next partial. **Fix**: `merge_textures_delta()` helper — pending deltas append old→new instead of being overwritten (paint jobs still replaced; egui's `TexturesDelta::append` handles set+free merge). +2 regression tests.
11. **A pipe masked a real clippy failure — caught and fixed**: the "final" gate suite printed `warning: build failed, waiting for other jobs to finish...` between FMT-OK and CLIPPY-OK yet exited 0 — my `cargo clippy ... | tail -1` pipeline reported tail's exit code, not cargo's (no pipefail). Re-ran unpiped: **clippy error `items_after_test_module`** (the new test mod sat before `write_crash_log`). Moved tests to EOF, amended (`c279588` final form), and re-ran the whole suite with `set -o pipefail` → 393/0 green. Lesson recorded in the rewritten `ci-toolchain-pin` memory: pipefail in every gate pipe.
12. **F1 retest after fix**: same resize-then-F1 sequence that crashed before → Inspector overlay renders (Entities/Assets/Scene tabs, entity list, New Entity button), process alive, Esc-clean exit. sm_crossfade playtest: D-key acceleration, screenshot caught blend **0.58** mid-transition — left panel hard-snapped, right panel visibly mixing two frames.
13. **Release**: CHANGELOG `## 7.0.0`, SECURITY_HARDENING strikethrough+RESOLVED, CLAUDE.md v1.6.3 header, Cargo.toml 7.0.0, lock refresh; release suite with pipefail green (36 ok-lines, 393/0); **commit `aa0646e`**; push; **PR #20**; CI 4/4 (table in Evidence); user "머지 확인" → merge `a3369ee`; branch + prune cleanup.
14. **Consumer migration** (same flow as v5→v6, refined): noticed game HEAD moved (`0c42012` — user pushed between sessions, baseline drift expected) → pre-bump baseline **213/0** captured WITH their 22-path WIP → pin `77b4465..` → `a3369eef3ac632069b434b1ff4b0bbe8b4158466` → `cargo update -p skeleton-engine` (also moved yazi 0.1.6→0.2.1, zeno 0.2.3→0.3.3 transitively) → clippy clean, **zero source breaks** (as predicted by the zero-grep measurement) → tests 213/0 = baseline exact → surgical 2-file commit `960b43c`, NO push.

Sub-threads worth knowing (finer grain):

15. **Background-gate + wakeup mechanics**: every long suite ran as one `run_in_background` Bash chain; completion notifications resumed the turn. ScheduleWakeup used 3× as hang-fallbacks only (270s while in-cache waiting on the first 1.88 baseline; 1800s during the migration agent; 1200s during the game baseline build) — every primary signal arrived first; one stale wakeup fired post-hoc and was correctly no-op'd against current state. `gh pr checks 20 --watch --interval 30` in background = the single CI completion notification, same as the parent arc.
16. **Enumeration artifact**: the raw 70-error log lives at `~/.claude/jobs/c538aa0e/tmp/wgpu29-check-raw.txt` (written via `tee` during the bump check; referenced verbatim in the migration-agent prompt so the agent never re-derived it). Job tmp dirs outlive the session only until job deletion — the inventory table in this file is the durable copy.
17. **wasm_smoke preconditions checked before running**: `wasm-bindgen` crate in the new lock = 0.2.122, installed `wasm-bindgen-cli` = 0.2.122 — exact match, no reinstall (the script hard-requires the match). `examples/assets/blend_locomotion.png` already existed from the parent arc's playtest → `gen_blend_sheet` prerun skipped for sm_crossfade.
18. **Mid-session status report** (after the user's first command rejection + "세션 진행 상황 보고"): delivered as two tables (done-items with commit hashes / risk-checklist outcomes) + a remaining-work list with the playtest decision explicitly offered three ways (I run it / user runs it / skip to docs+PR with the coverage gap named). User answered "중단된 부분부터 이어서 진행해" then "플레이 테스트 너가 해" — the delegation was never in question, only the command granularity.
19. **In-session task ledger** (Task tools, since bd is absent): #1 toolchain commit A → #2 dep bump + migration → #3 gates/wasm_smoke/playtest → #4 release docs + PR → #5 consumer migration. All five completed; #2's verification catches and #3's F1 bug were handled inside their tasks rather than spawning new ones.

## Key Decisions

- **CI pin 1.95.0 (= local stable) instead of 1.92 (minimum MSRV)** — pin-to-minimum preserves the local/CI rustfmt divergence that has bitten this repo before (memory `ci-toolchain-pin`); pin-to-local-stable makes `cargo +stable` CI-faithful today. The trap re-arms when local stable moves past 1.95; the rewritten memory documents both escape paths (install 1.95.0, or deliberately re-pin).
- **`rust-version = "1.92"` ≠ CI pin 1.95.0, intentionally** — rust-version is the honest consumer-facing MSRV (what deps force); the CI pin is a gate-reproducibility choice. They serve different contracts.
- **wgpu 29 + glyphon 0.11 + egui 0.34 was announced, not asked** — the compatibility matrix leaves literally no alternative (RUSTSEC fix ⇒ glyphon ≥0.10 ⇒ wgpu ≥28; egui-wgpu has no wgpu-28 release). When research collapses the option space to one, presenting a menu is theater.
- **v7.0.0 (major), justified twice over** — `DebugUi::ctx() → &egui::Context` and `RenderTarget`'s pub wgpu fields put both bumped ecosystems in the public API; plus `GpuContext::clear()`'s forced signature change; plus MSRV. Any one suffices under the project's semver discipline.
- **One in-place agent, not a parallel worktree fleet** — dep-major churn is one interlocked compile-error set (nothing compiles until everything does); the v5.1/v6 parallel-worktree pattern applies to disjoint modules, not this. In-place also reuses the warm target dir (a worktree would cold-rebuild the whole new dep tree).
- **Verify-before-trust held against my own agents again** — 2 wrong claims out of one migration report (dithering "identical behavior", clear() "not public"), consistent with the parent arc's measured 13–30% finding-error rates. The doctrine stays mandatory.
- **`dithering: false` restored** — behavior preservation beats upstream defaults; the engine's pre-0.34 call passed `false` explicitly, so silent enabling is a regression no matter how subtle the visual delta.
- **The egui delta fix merges rather than processes-immediately** — alternative (apply texture updates in step_frame as they're produced) was rejected: it needs gpu+renderer access in the schedule path and breaks the set→paint→free contract when paint is skipped. Append-and-consume-later keeps one consumption site.
- **fable5 subagent retest permanently dropped** (user order) — explicit `model:` on every Agent/agent() call is now standing policy; ~53 clean sonnet-forced runs of evidence, zero re-litigations to come.
- **glam 0.28 deliberately NOT bumped** (0.33 is current) — it's re-exported in the pub API (`pub use glam::{IVec2, Mat4, Vec2, Vec3}`) so a bump is its own breaking window; nothing forces it; out of scope.
- **Playtest commands split small after user pushback** — the monolithic launch+place+screenshot+keys+quit script was rejected twice at the permission prompt; 5 single-purpose commands were all approved. Process note for every future macOS playtest.

## Evidence & Data

### Version/compat matrix (researched via crates.io API — needs `-A` User-Agent header!)

| Crate | Pinned (old) | Latest stable | Latest's requirements | Chosen |
|---|---|---|---|---|
| wgpu | 22 | 29.0.3 (2026-05-02) | rust 1.87 | **29** |
| glyphon | 0.6 | 0.11.0 (2026-04-13) | wgpu ^29.0.0, lru ^0.16.2, cosmic-text ^0.18, winit ^0.30.12 | **0.11** |
| egui-wgpu | 0.29 | 0.34.3 (2026-05-27) | wgpu ^29.0.1, egui ^0.34.3 | **0.34** |
| egui-winit | 0.29 | 0.34.3 | egui ^0.34.3, winit ^0.30.13 | **0.34** (default-features=false kept) |
| winit | 0.30 | 0.30.13 (no major) | — | floor → **0.30.13** |
| glam | 0.28 | 0.33.1 | — | **kept 0.28** (pub API re-export; separate window) |

Why no alternative stack: glyphon 0.10 (the first lru-fixed release) targets wgpu **28**; egui-wgpu jumped 27→29 with nothing for 28 (0.33.3 → wgpu ^27.0.1; 0.34.x → wgpu ^29). Older glyphon (0.9 = wgpu 25) predates the lru fix → doesn't close RUSTSEC.

### MSRV table (the scope-changing discovery)

| Crate@version | rust_version |
|---|---|
| wgpu 29.0.3 | 1.87.0 |
| glyphon 0.11.0 | (none declared) |
| **egui 0.34.3 / egui-wgpu 0.34.3** | **1.92** ← the forcing constraint |
| cosmic-text 0.18.0 | 1.89 |
| lru 0.16.3 | 1.70.0 |

Local toolchains at session start: stable-1.95.0 (active), 1.88.0, 1.79.0. CI was `dtolnay/rust-toolchain@1.88.0` ×4 jobs.

### Compile-error inventory (the bump, before fixing — 70 errors / 13 files)

| Count | Error |
|---|---|
| 20 | mismatched types (Instance::new by-ref, etc.) |
| 12 | missing field `multiview_mask` in `RenderPassDescriptor` |
| 12 | missing field `depth_slice` in `RenderPassColorAttachment` |
| 7 | `PipelineLayoutDescriptor` has no field `push_constant_ranges` (→ `immediate_size: u32`) |
| 6 | `RenderPipelineDescriptor` has no field `multiview` (→ `multiview_mask`) |
| 4 | glyphon/cosmic-text call-site arg-count changes (set_text/set_rich_text gained trailing `Option<Align>`) |
| 2 | moved value `default_attrs` (cosmic-text 0.18 `Attrs` lost `Copy`) |
| 2 | `?` on non-Try (`get_current_texture` no longer returns Result) |
| 2+2 | `SurfaceError` not found (type removed in wgpu 29) |
| 1 | `InstanceDescriptor` no `Default` (filled: flags/memory_budget_thresholds/backend_options/`display: None`) |
| 1 | `DeviceDescriptor` missing `experimental_features` + `trace` (request_device lost its trace param) |
| 1 | request_adapter returns `Result` (was Option) |

By file: text.rs 10, gpu_particle.rs 10, lighting.rs 9, app/render.rs 8, sprite.rs 7, post_process.rs 7, fade.rs 7, context.rs 7, sprite/material.rs 5, ui_primitives.rs 2, app/window.rs 2, app/egui_pass.rs 2, editor/ui/gizmo.rs 1.

### Commit / PR ledger

| Commit | Content | Verification |
|---|---|---|
| `9fe5cff` | chore(toolchain)!: CI 1.88.0→1.95.0 ×4, rust-version 1.92 | 1.95 fmt/clippy/test/wasm/doc green, 391/0, zero lint churn |
| `6109796` | feat(deps)!: the migration (agent) — 15 files, +725/−477 incl. lock | agent-run full gates, 391/0 |
| `0010f64` | fix(deps): dithering=false restored (verification catch) | clippy + fmt |
| `c279588` | fix(app): merge_textures_delta + 2 regression tests (playtest catch; amended once for `items_after_test_module`) | 393/0, clippy clean unpiped, F1 retest alive |
| `aa0646e` | chore(release): 7.0.0 — version/CHANGELOG/SECURITY/CLAUDE.md | release suite w/ pipefail, 36 ok-lines |
| `a3369ee` | Merge PR #20 (merge-commit, user "머지 확인") | CI 4/4 |
| game `960b43c` | pin 6.0.0→7.0.0, 2 files (+756/−180 lock) | clippy clean, 213/0 = baseline exact |

### PR #20 CI timing (first run on the 1.95 pin, cold dep cache)

| Check | Result | Duration |
|---|---|---|
| Build (WASM) | pass | 1m48s |
| Test (native) | pass | 9m11s (cold cache with the new dep tree; was 1m51s–2m52s warm on v6-era runs) |
| Rustdoc | pass | 1m50s |
| Package dry-run | pass | 3m45s |

### Verification outcomes (agent claims vs. my review — the session's error-rate data point)

| # | Migration-agent claim | Verdict | Evidence |
|---|---|---|---|
| 1 | `RendererOptions::default()` ≡ old args | **WRONG** | default = `{msaa_samples: 0, dithering: true, ...}` (egui-wgpu renderer.rs:224-233) vs old `(None, 1, false)`; dithering differs |
| 2 | msaa 0 vs 1 | OK (after my check) | `count: options.msaa_samples.max(1)` at renderer.rs:400 — clamped, equivalent |
| 3 | `GpuContext::clear()` "not exposed → no pub API break" | **WRONG** | `pub mod context` + `pub use context::{GpuContext, GpuContextError}` (renderer/mod.rs:1,16) |
| 4 | Suboptimal frames still presented | CONFIRMED | render.rs: `Success(t) \| Suboptimal(t)` both yield the frame |
| 5 | Lost/Outdated → reconfigure preserved | CONFIRMED | render.rs:697-700 same match shape, new enum variants |
| 6 | set_text `Align=None` preserves alignment | CONFIRMED | per-line `for line in &mut buf.lines` align loop runs after set_text, unchanged |
| 7 | "all runtime behaviors match wgpu 22 exactly" | 2 exceptions above; rest held | — |

Research-agent cheat sheet accuracy: ~90% of compile-breaks real; 3 wrong/N-A claims (present() removal, egui panel deprecations, InstanceDescriptor constructors-only). The [SILENT-BEHAVIOR] sections were the high-value part.

### The F1 panic (raw, for future grep)

```
thread 'main' (11548327) panicked at .../egui-wgpu-0.34.3/src/renderer.rs:669:18:
Tried to update a texture that has not been allocated yet.
```

Mechanism: engine egui frame runs every tick (`schedule.rs` — egui_ctx Some whenever DebugUi resource exists); `egui_output` overwritten at end of step_frame; consumed in render() AFTER the `get_current_texture` early return; egui-wgpu 0.34 partial update does `self.textures.remove(&id).expect(...)`. Trigger in practice: osascript window resize → one Outdated frame → initial full font-atlas set dropped → first F1 lays out panel text → partial glyph update → panic. Pre-0.34 the bug existed but ab_glyph's full-image re-sends masked it permanently.

### Playtest evidence (procedure per `playtest-windowed-examples` memory)

| Test | Result |
|---|---|
| wasm_smoke (coin_race, headless Chrome DPR=2) | PASS — connect + render, screenshot 41913B (v6 ref: 41962B), HUD/geometry eyeballed correct |
| lit_dungeon shot1 | lighting pass alive: ambient darkness + vignette, HUD text crisp, FPS 60, torch-out state ("Press R to retry") |
| lit_dungeon shot2 (R + D/W + E) | point-light radial falloff around player, torch 75%, FPS 58 |
| lit_dungeon F1 (pre-fix) | **PANIC** (egui-wgpu :669) — process dead |
| lit_dungeon F1 (post-fix, same resize-first sequence) | Inspector overlay renders (Entities/Assets/Scene tabs), alive, Esc-clean exit |
| sm_crossfade (D held) | blend **0.58** mid-transition captured; right panel = visible two-frame mix, left = hard switch; speed 0.95 |
| Synthetic-input notes | letter keys via `key down "d"` + `keystroke "r"` fine; **F1 = `key code 122` WORKS** (new data — arrows remain dead per old memory); Esc = `key code 53` fine |

### Agent fleet (this session)

| Agent | Model | Tokens | Tool uses | Duration | Output |
|---|---|---|---|---|---|
| dep-migration research | sonnet | 91K | 50 | 377s | breaking-changes cheat sheet (~90% accurate) |
| wgpu29 migration | sonnet | 137K | 127 | 803s | commit `6109796`, gates green, report w/ 2 wrong claims |

Both with explicit `model: sonnet` per standing policy. No worktree isolation (deliberate — see Key Decisions).

### Consumer (rust-survivors) migration measurements

| Check | Value |
|---|---|
| Game HEAD at migration | `0c42012` (user pushed since parent handoff; parent's 801f334 in ancestry) |
| User WIP | 22 unstaged paths (incl. survivor/{data,locale,sfx,stage}.rs) — untouched |
| Pre-bump baseline (with WIP) | `test -p game --lib` **213/0** (parent recorded 206 — user WIP added 7) |
| Breaks after pin bump | **0** (predicted by zero egui/wgpu grep hits; clippy exit 0) |
| Post-bump tests | **213/0** = baseline exact |
| Transitive movement | yazi 0.1.6→0.2.1, zeno 0.2.3→0.3.3 (via cargo update -p skeleton-engine) |
| Commit | `960b43c`, 2 files (+756/−180), **NOT pushed** |

### Gate-run ledger (every full-suite execution this session)

| Run | Toolchain | Tree state | Result |
|---|---|---|---|
| baseline | 1.88.0 | main @ b6bd781, old deps | green, **391/0** |
| pre-commit-A | stable 1.95 | old deps, ci.yml+rust-version edited | green, 391/0, **zero lint churn** (the decisive data for the 1.95 pin) |
| post-migration | stable 1.95 | new deps @ 6109796+0010f64 | green, 391/0 |
| post-F1-fix | stable 1.95 | + c279588 (pre-amend) | "green" — **but the clippy step was masked by `\| tail -1`** (no pipefail); `warning: build failed, waiting for other jobs` in the transcript was the tell; unpiped re-run = exit 101 `items_after_test_module` |
| post-amend | stable 1.95 | c279588 final | clippy clean unpiped, **393/0** |
| release | stable 1.95, `set -e -o pipefail` | + aa0646e (version 7.0.0) | green, 36 `test result: ok` lines, 393/0 |
| PR #20 CI | 1.95.0 (dtolnay pin) | merged tree | 4/4 (timings above) |

### Silent-risk triage (cheat sheet top-5 × this engine — all checked by direct grep before dispatch)

| Risk (from research) | Engine verdict |
|---|---|
| [wgpu 25/WebGL] `device.poll(Wait)` returns Timeout instead of blocking | **N/A** — zero `device.poll`/`Maintain` use (only rodio's unrelated `c.poll()` in network.rs:601) |
| [wgpu 28] push constants → immediates; WGSL `var<push_constant>` dead | **N/A** — zero push-constant use; layouts passed `&[]` (→ `immediate_size: 0`) |
| [wgpu 29] integer `@location` needs explicit `@interpolate(flat)` | **N/A** — 5 shaders, 23 `@location`s, none integer |
| [wgpu 27] `set_viewport` min==max rejected; buffer-offset alignment enforced | **N/A** — zero `set_viewport` calls |
| [egui 0.34] skrifa fonts + text 12.5→13.0 + tessellator changes | **REAL** — debug-UI-only; verified visually via F1 overlay + accepted; dithering default ALSO flipped (caught separately, restored false) |

### Condensed cheat-sheet (the research agent's output lives only in this conversation — engine-relevant extract)

- wgpu 23: `entry_point` → `Option<&str>`; resources gain `Clone` (Arc-wrapping now double-counts — kept ours anyway for API stability)
- wgpu 24: `request_adapter` → `Result`; instance-descriptor reshuffles begin
- wgpu 25: `Maintain` → `PollType` (N/A here)
- wgpu 26: `Limits` binding-size fields u32 → u64 (no engine comparisons broke)
- wgpu 27: buffer-mapping guards lose lifetimes; validation tightenings (N/A per triage)
- wgpu 28: `SamplerDescriptor.mipmap_filter` → `MipmapFilterMode`; immediates rename; `RenderPassDescriptor.multiview_mask` appears
- wgpu 29: `SurfaceError` GONE → `CurrentSurfaceTexture` enum; `InstanceDescriptor` loses `Default`; `bind_group_layouts`/`VertexState::buffers` element types become `Option<_>`; `DepthStencilState` fields become `Option` (engine uses no depth — untouched)
- glyphon 0.7–0.11: public API stable for our usage; all churn was transitive wgpu + cosmic-text (`Attrs` non-Copy at 0.18, set_text gains `Option<Align>`)
- egui 0.30→0.34: `Rounding`→`CornerRadius` (engine unaffected — no direct use), `Context::style()`→`global_style()` (unaffected), `wants_pointer_input`→`egui_wants_pointer_input` (one site, gizmo.rs), skrifa fonts (visual only)
- Research-agent misses for the record: `SurfaceTexture::present()` NOT removed; egui 0.34 panel deprecations never fired under `-D warnings`; `InstanceDescriptor` struct literal fine with `display: None`

### Raw snippets (primary evidence — expensive to re-derive)

The F1 fix, consumer side (schedule.rs step_frame tail):

```rust
let textures_delta = merge_textures_delta(
    self.egui_output.take().map(|(_, pending, _)| pending),
    full_output.textures_delta,
);
self.egui_output = Some((paint_jobs, textures_delta, ppp));
```

The disproving quote for agent-claim #1 (egui-wgpu 0.34.3 renderer.rs:224-233 — `RendererOptions::default()`):

```rust
impl Default for RendererOptions {
    fn default() -> Self {
        Self {
            msaa_samples: 0,            // engine passed 1 — clamped equal by .max(1) at :400
            depth_stencil_format: None,
            dithering: true,            // engine passed FALSE — the real silent change
            predictable_texture_filtering: false,
        }
    }
}
```

The egui-wgpu panic site (renderer.rs:660-669, partial update path):

```rust
let (texture, origin, bind_group) = if let Some(pos) = image_delta.pos {
    let Texture { .. } = self
        .textures
        .remove(&id)
        .expect("Tried to update a texture that has not been allocated yet.");
```

wgpu 29 surface acquire, as shipped (render.rs):

```rust
let (frame, _suboptimal) = match gpu.surface.get_current_texture() {
    wgpu::CurrentSurfaceTexture::Success(t) => (t, false),
    wgpu::CurrentSurfaceTexture::Suboptimal(t) => (t, true),   // still rendered+presented
    e => return Err(e),
};
// ... step_frame:
Err(wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated) => {
    if let Some(gpu) = &self.gpu { gpu.reconfigure(); }
}
```

The unsafe removal (egui_pass.rs, after):

```rust
let mut rpass_s = rpass.forget_lifetime();
er.render(&mut rpass_s, paint_jobs, screen_desc);
```

wgpu 29 init descriptors, as shipped (context.rs — the canonical reference for any future wgpu work):

```rust
let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
    backends: /* wasm: Backends::GL, native: Backends::all() — unchanged */,
    flags: wgpu::InstanceFlags::default(),
    memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
    backend_options: wgpu::BackendOptions::default(),
    display: None,   // works on macOS/wasm; Linux/Wayland untested — see Risks
});
// request_adapter: .ok_or(AdapterNotFound) → .map_err(|_| AdapterNotFound)
let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
    label: Some("main device"),
    required_features: wgpu::Features::empty(),
    required_limits: /* native: default(), wasm: downlevel_webgl2_defaults() — unchanged */,
    experimental_features: wgpu::ExperimentalFeatures::disabled(),
    memory_hints: wgpu::MemoryHints::default(),
    trace: wgpu::Trace::Off,   // was the second positional arg `None`
}).await?;
```

The v7-major pub-API evidence (the exact lines that forced the version call):

```
src/debug_ui.rs:37      pub fn ctx(&self) -> &egui::Context          // egui in pub API
src/renderer/render_target.rs:4-9   pub texture/view/sampler: wgpu::*, pub bind_group: Arc<wgpu::BindGroup>
src/renderer/mod.rs:1,16  pub mod context; pub use context::{GpuContext, GpuContextError};  // clear() IS public
```

PR: https://github.com/ChunSam/skeleton-engine/pull/20 — body shape reused from the arc convention: summary + dep table + per-commit table + breaking list + verification section (wasm_smoke bytes, playtest results, the silent-risk triage).

crates.io research recipe (the User-Agent gotcha):

```bash
# 403/empty WITHOUT -A; works with any UA string:
curl -s -A "dep-research" "https://crates.io/api/v1/crates/{crate}"            # versions + max_stable + rust_version
curl -s -A "dep-research" "https://crates.io/api/v1/crates/{crate}/{ver}/dependencies"  # req ranges
```

### Playtest command recipe that survived the permission boundary (5 small steps — the monolithic version was rejected twice)

```bash
# 1. launch (detached) — capture PID
./target/debug/examples/lit_dungeon_game >/tmp/v7_lit.log 2>&1 & echo "PID=$!"
# 2. place + focus (System Events by unix id); caffeinate -t N = the sleep substitute
osascript -e 'tell application "System Events" to tell (first process whose unix id is PID)
set frontmost to true
set position of front window to {100, 100}
set size of front window to {960, 720}
end tell'
# 3. screenshot
caffeinate -u -t 2; screencapture -x -R 100,100,960,720 /tmp/shot.png
# 4. keys: keystroke "r" / key down "d" ... delay ... key up "d" / F1 = key code 122 / Esc = key code 53
# 5. quit check: ps -p PID || echo "exited via Esc cleanly"
```

Screenshots on disk: `/tmp/wasm_smoke.png` (41913B), `/tmp/v7_lit_shot{1,2}.png`, `/tmp/v7_lit_f1.png` (post-fix overlay), `/tmp/v7_sm_blend.png` (blend 0.58). `/tmp/v7_lit_shot3.png` is the crash artifact (captured a terminal window because the game had died — that mis-aimed screenshot was the bug's first symptom).

### Gate-suite shape that is now canonical (pipefail!)

```bash
set -e -o pipefail
cargo +stable fmt --check
cargo +stable clippy --all-targets -- -D warnings
cargo +stable build --target wasm32-unknown-unknown   # lib+bins; NEVER --all-targets on wasm
cargo +stable test --all-targets                      # 393/0 lib expected
RUSTDOCFLAGS="-D warnings" cargo +stable doc --no-deps
```
Without pipefail, `cargo ... | tail -1` masked a real clippy failure once this session (`items_after_test_module`) — the suite printed green while clippy was exit 101.

## Code Analysis

- `merge_textures_delta(pending: Option<egui::TexturesDelta>, newer: egui::TexturesDelta) -> egui::TexturesDelta` — `pub(super)` in src/app/schedule.rs; append old→new via `TexturesDelta::append`; tests at module bottom (`egui_delta_tests` — clippy requires test mods LAST in file: `items_after_test_module`).
- `App.egui_output: Option<(Vec<egui::ClippedPrimitive>, egui::TexturesDelta, f32)>` (app.rs:178) — producer schedule.rs end-of-step, consumer render.rs:614 `self.egui_output.take()`; the take site is after the surface-acquire early return at render.rs:209 — that ordering is WHY merge (not overwrite) is load-bearing.
- wgpu 29 surface flow: `surface.get_current_texture()` → `wgpu::CurrentSurfaceTexture` enum (`Success(t)`/`Suboptimal(t)` carry frames; `Timeout`/`Outdated`/`Lost`/`Validation` don't); `frame.present()` still exists (research-agent claim of removal was wrong); render() now returns `Result<(), wgpu::CurrentSurfaceTexture>` (private fn).
- wgpu 29 descriptor fields filled engine-wide: `RenderPassDescriptor.multiview_mask: None`, `RenderPassColorAttachment.depth_slice: None`, `PipelineLayoutDescriptor.immediate_size: 0` + `bind_group_layouts: &[Some(&bgl)]`, `RenderPipelineDescriptor.multiview_mask: None` + `entry_point: Some("vs_main")`, `DeviceDescriptor.experimental_features: ExperimentalFeatures::disabled()` + `trace: Trace::Off`, `InstanceDescriptor{flags, memory_budget_thresholds, backend_options, display: None}` (struct literal, no Default impl).
- egui-wgpu 0.34: `Renderer::new(&device, format, RendererOptions{msaa_samples: 1, dithering: false, ..Default::default()})` (window.rs:424-435, with the why-comment); `RenderPass::forget_lifetime()` feeds `Renderer::render` (egui_pass.rs, unsafe gone).
- egui 0.34 renames hit: `Context::wants_pointer_input()` → `egui_wants_pointer_input()` (gizmo.rs:10).
- cosmic-text 0.18: `Attrs` non-Copy → `parse_rich_text`/`rich_attrs` take `&Attrs` (one `.clone()` at the copy site); `set_text/set_rich_text(font_system, text, &attrs, Shaping::Advanced, None)` — trailing arg is `Option<Align>`, None preserves the engine's per-line align loop.
- wgpu resources are `Clone` (internally refcounted) since 24 — `RenderTarget.bind_group: Arc<wgpu::BindGroup>` kept as-is for API stability (double-refcount is harmless: Arc drop → inner drop → wgpu count drop).
- Engine shader surface (5 .wgsl: sprite 17 @location, post_process 2, gpu_particle_render 2, fullscreen_quad 2, gpu_particle_compute 0): no integer locations / push constants → the wgpu 27-29 validation tightenings are structurally N/A here.

## Files Changed

### Engine (branch feat/v7-wgpu29-stack → main via PR #20)
- `.github/workflows/ci.yml` — 4× toolchain pin 1.88.0 → 1.95.0
- `Cargo.toml` — deps (wgpu 29, glyphon 0.11, egui trio 0.34, winit 0.30.13), rust-version 1.92, version 7.0.0; `Cargo.lock` regenerated
- `src/renderer/context.rs` — InstanceDescriptor literal, request_adapter Result, DeviceDescriptor new fields, CurrentSurfaceTexture matches, clear() → Result<(), String>
- `src/app/render.rs` — render() returns CurrentSurfaceTexture on error, acquire match (Success|Suboptimal present), step_frame Lost|Outdated arm, pass fields
- `src/app/egui_pass.rs` — unsafe transmute → `forget_lifetime()`, pass fields
- `src/app/window.rs` — `RendererOptions{msaa_samples:1, dithering:false, ..}` with rationale comment
- `src/app/schedule.rs` — `merge_textures_delta` + call site + `egui_delta_tests` (×2) at EOF
- `src/app/editor/ui/gizmo.rs` — `egui_wants_pointer_input()` rename
- `src/renderer/{sprite,post_process,fade,lighting,gpu_particle,text}.rs`, `src/renderer/sprite/{material,ui_primitives}.rs` — descriptor-field churn per the inventory; text.rs also cosmic-text 0.18 Attrs/set_text changes

### Docs & meta
- `docs/CHANGELOG.md` — `## 7.0.0` (~57 lines: dep table prose, MSRV, API breaks, the Fixed entry, Changed notes)
- `docs/SECURITY_HARDENING_2026_05.md` — RUSTSEC-2026-0002 items struck through + RESOLVED block pointing at CHANGELOG 7.0.0
- `CLAUDE.md` — header v1.6.3: package 7.0.0, "(wgpu 29, MSRV 1.92, CI pin Rust 1.95.0)"

### rust-survivors (commit `960b43c`, local only)
- `crates/game/Cargo.toml:20` — rev `77b4465...` → `a3369eef3ac632069b434b1ff4b0bbe8b4158466`; `Cargo.lock`

### Memory
- `ci-toolchain-pin.md` — fully rewritten (1.95 pin, +stable guidance, stable-drift re-arm warning, pipefail lesson; old 1.88 content gone; editor/ui/mod.rs fmt gotcha retained)
- `new-model-subagent-incompat.md` — retest permanently dropped (user decision 2026-06-12)
- `engine-current-state.md` — v7 shipped state (rewritten 2×: in-progress → final)
- `MEMORY.md` — 3 index lines updated (toolchain, engine state, subagent policy)

## User Feedback & Preferences (REQUIRED — never omit)

Complete input timeline (verbatim):

| # | Input | Meaning / handling |
|---|---|---|
| 1 | paste prompt (seq-1 onboarding protocol, 5 steps + wait) | followed; presented 4 numbered options and waited |
| 2 | "4번은 완전 삭제하고, 2번 진행" | compound: PERMANENTLY delete the fable5-retest option (memory updated, not just skipped) + go on the wgpu migration. "완전 삭제" = make it never come back |
| 3 | (rejects monolithic playtest command) "세션 진행 상황 보고" | wanted a status report before allowing GUI automation; delivered full table-form report in Korean |
| 4 | "중단된 부분부터 이어서 진행해" | resume from the interruption point — re-offered the same command shape… |
| 5 | (rejects again) "플레이 테스트 너가 해" | …rejection was about the COMMAND SHAPE, not the delegation: splitting into 5 small single-purpose commands got instant approval. Lesson: at permission boundaries, granular > monolithic |
| 6 | "머지 확인" | the standing per-merge unlock token — worked first try, PR #20 merged |
| 7 | "/handoff" | close the session |

- **Checkpoint cadence stays calibrated**: zero mid-execution corrections on the technical work itself across the whole session; the only friction was command granularity at permission prompts.
- The user reads status tables happily (the mid-session 진행 상황 보고 used commit/gate tables; no complaints, immediately followed by "이어서 진행해").
- Korean prose / English artifacts unchanged. Merge stays user-confirmed; game-repo pushes stay user-owned (commit yes, push no).
- The user pushed rust-survivors between sessions themselves — their push rhythm is their own; never nag about unpushed game commits.

## Where We're Going

1. **User pushes rust-survivors `960b43c`** (their schedule, with their WIP commits).
2. **New feature work** — VISION loop (feature + playable example in `examples/`). The candidate queue is empty AND the structural-debt queue is empty — first fully-clean slate since the project started. User picks genre/feature; build the candidate table (feature × example × expected API gaps) on request.
3. **Optional process**: weekly scheduled cloud review routine — the one-time template from the parent arc is proven; needs a fresh RemoteTrigger (one-time routines auto-disable) and `clear_mcp_connections` hygiene.
4. **Toolchain watch** (not work, awareness): when local stable moves past 1.95.0, either `rustup toolchain install 1.95.0` and gate with `+1.95.0`, or deliberately re-pin CI (4 sites) — `ci-toolchain-pin` memory has both paths.
5. **Possible future window** (no urgency): glam 0.28 → 0.3x is the only remaining old-major dep, and it's pub-API-exposed → would be its own breaking release if ever wanted.

## Risks & Blockers

- **egui 0.34 visual drift in the debug UI is real but unaudited in detail** — skrifa hinting + default text size 12.5→13.0 reflow panels; F1 overlay verified renders correctly, but the user hasn't eyeballed the editor/inspector themselves. If a panel looks cramped later, it's the font metrics, not a regression.
- **`InstanceDescriptor.display: None` untested on Linux/Wayland** — fine on macOS (Metal) and wasm (GL), and CI is headless; a Linux consumer creating a surface might need the display handle. Flag, don't fix.
- **rust-survivors `960b43c` local-only** — remote pin still says 6.0.0 until the user pushes. Game tree carries 22-path WIP; pre-change baseline capture (now 213, drifts with their WIP) + explicit-path staging remain mandatory.
- **The 1.95 pin re-arms the fmt-divergence trap when stable moves** — recorded in memory; cheap to handle, easy to forget.
- v7.0.0 is breaking on the remote: forks following main break on MSRV + the API changes; CHANGELOG ## 7.0.0 is the guide.

## Open Questions

- Weekly scheduled reviews — wanted, or stay on-demand? (Carried from parent; still unanswered, still non-blocking.)
- None blocking — the release is fully closed.

## Quick Start for Next Session

```bash
# Current state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3 main     # a3369ee = v7.0.0 merge; clean; pushed
cd /Users/jkl/Projects/rust-survivors
git log --oneline -2          # 960b43c = v7 pin bump — committed, NOT PUSHED (user's call)
git status -s | head          # user WIP (22 paths) — strictly untouchable

# Canonical context
# - docs/CHANGELOG.md ## 7.0.0            (migration guide — consumer-tested, zero breaks)
# - this file                              (the v7 session)
# - parent handoff                         (the v5.1→v6 arc; verification doctrine, agent patterns)
# - memory engine-current-state            (current), ci-toolchain-pin (1.95 + pipefail)

# Verify engine state (CI pin = 1.95.0 = local stable right now)
cd /Users/jkl/Projects/skeleton-engine
set -o pipefail
cargo +stable clippy --all-targets -- -D warnings && cargo +stable test --lib
# Expect: clean, 393 lib tests / 0 failed

# Verify game state (baseline drifts with user WIP — capture before judging)
cd /Users/jkl/Projects/rust-survivors
cargo +stable test -p game --lib   # Expect 213/0 unless user WIP moved it

# Next action
# 1) Confirm user pushed 960b43c (or leave it — their repo), then
# 2) NEW FEATURE WORK per docs/VISION.md — user picks genre/feature; both queues
#    (candidates AND structural debt) are empty for the first time.
#    Offer the candidate table (feature × playable example × expected API gaps).
```

## Session Closed
**Closed at:** 2026-06-12
**Commit:** see `session: v7-wgpu29-stack [v6-release-arc]` on engine main (handoff file only — all code/doc work was merged during the session via PR #20; game `960b43c` local per the user's push rule)
**Session status:** Handed off to next session
