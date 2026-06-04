# skeleton-engine: local verification bar + audio/editor source splits (Phase 1–3)

**Date:** 2026-06-04
**Status:** COMPLETED
**Bead(s):** none (`bd` not installed)
**Epic:** source file split / maintainability refactor
**Chain:** `source-split-refactor` seq `3`
**Parent:** `HANDOFF_source-split-refactor_regression-fixes_2026-06-03.md` (seq 2)
**Prior chain:** `HANDOFF_source-split-refactor_source-file-split_2026-06-03.md` (seq 1) > `HANDOFF_source-split-refactor_regression-fixes_2026-06-03.md` (seq 2) > this

---

## Since Last Handoff

This session executed the seq-2 PLAN (`PLAN_source-split-refactor_regression-fixes_2026-06-03.md`) end-to-end. Comparing parent's "Where We're Going" (5 optional follow-ups) vs reality:

- Parent step 1 (document the LOCAL CI-equivalent verify bar) → **DONE** (Phase 1): block in `CLAUDE.md` + `AGENTS.md` + new `scripts/verify.sh`.
- Parent step 2 (deep line-by-line review of `editor/ui.rs` 963 + `ecs/world.rs` 909) → **DONE** (Phase 2): both confirmed faithful moves, **no findings**.
- Parent step 3 (optional split of `editor/ui.rs`) → **DONE** (Phase 3): extracted shortcuts + gizmo into `src/app/editor/ui/`.
- Parent step 4 (optional split of `audio.rs` 771) → **DONE** (Phase 3): `src/audio/` by responsibility.
- Parent step 5 (mention new module dirs in `CLAUDE.md`) → **DONE**: audio row updated in both `CLAUDE.md` and `AGENTS.md`.
- Parent open question "should the local bar be documented?" → **answered yes** (Phase 1). "Split `editor/ui.rs` now or at 1000 lines?" → **split now** (conservative). "Is `FrameContext` public re-export OK long-term?" → **still open** (out of scope this session).
- Parent risk "Phase 3 split reintroduces the seq-1 class bug (dropped cfg gate)" → **did NOT materialize**: verify.sh (incl. wasm build + clippy `-D warnings`) green; objective attribute audit shows all cfg gates preserved.
- Trajectory: the `source-split-refactor` epic is now effectively **complete** — every deferred maintainability item is done, no file over the 1000-line ceiling, public API unchanged.

## Reference Documents

- `CLAUDE.md` — project quick reference / module map; now carries the Verification block (Phase 1).
- `AGENTS.md` — Codex/agent quick ref; mirror of the Verification block.
- `docs/PATTERNS.md` — core architecture patterns (unchanged this session).
- `.github/workflows/ci.yml` — the authoritative bar Phase 1 mirrors locally.
- `docs/VISION.md` — feature work needs a playable example; this session was mechanical refactor + docs, examples excluded.

## The Goal

Continue the `source-split-refactor` chain by executing the seq-2 plan: (1) close the LOCAL verification gap that let the seq-1 regression through, by documenting a CI-equivalent "done" bar where agents read it; (2) finish the only outstanding review gap (two large split files never read end-to-end); (3) optionally finish the parent's deferred file splits. Hard constraint throughout: **preserve public API and runtime behavior** — the split's whole premise is API/behavior preservation.

## Where We Are

- **All Phase 1–3 work is complete and verified green.** As of handoff, changes are committed and pushed (see Session Closed footer); before commit they sat uncommitted in the working tree on `main`.
- **Phase 1 docs:** `CLAUDE.md` gained a `## Verification (run before declaring done)` section (file 153→178 lines, under the 200 cap). `AGENTS.md` gained a `### Verification` subsection + replaced the vague "Run default verification" bullet (104→121 lines).
- **`scripts/verify.sh`** (new, executable): runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --target wasm32-unknown-unknown`, `cargo test --all-targets` in order; ran end-to-end → exit 0.
- **Phase 2 — `ecs/world.rs` (909):** `git diff 5f75a91..61c09f1` is a **single hunk** — the 534-line `#[cfg(test)] mod tests` block relocated to `tests.rs`, replaced by `mod tests;`. Production code byte-identical; native `par_query*` methods still `#[cfg(not(target_arch = "wasm32"))]`-gated. No accidental `pub` widening.
- **Phase 2 — `editor/ui.rs` (963):** at pre-split `5f75a91` this code was inline in `App::update` (`app.rs:916`, editor block from line 1065). The split lifted 957 lines verbatim into `update_editor_ui`; the diff is exactly **one line** — `if let Some(ctx) = &egui_ctx` → `if let Some(ctx) = egui_ctx` (required by the new `egui_ctx: &Option<egui::Context>` parameter; behavior-identical via match ergonomics). Call site `schedule.rs:255` correct.
- **Phase 3 — audio split:** `src/audio.rs` 771 → parent `audio.rs` (64) + `types.rs` (75) + `source.rs` (70) + `playback.rs` (329) + `positional.rs` (61) + `effects.rs` (21) + `bus.rs` (88) + `tests.rs` (98). `AudioManager` struct stays in `audio.rs` (resolves `crate::audio::AudioManager` natively); `pub use types::{AudioChannelState, AudioEffect};` re-exports the public types.
- **Phase 3 — editor split:** `src/app/editor/ui.rs` 963 → `ui/mod.rs` (767) + `ui/shortcuts.rs` (72) + `ui/gizmo.rs` (145). `ui.rs` became `ui/mod.rs` (directory module) so `mod shortcuts; mod gizmo;` resolve.
- **Module map updated:** audio row → `src/audio.rs`, `src/audio/` in both `CLAUDE.md` and `AGENTS.md`. (Editor `ui/` is app-internal, not listed in either map.)
- **Public API:** unchanged. `lib.rs:61 pub use audio::{AudioChannelState, AudioEffect, AudioManager};` intact. Audio public symbol set = **24 identical** (verified by extraction+diff). Editor changes are all internal (`pub(in crate::app)`).
- **Combined verify (whole integrated tree):** `./scripts/verify.sh` → exit 0 (fmt, clippy `-D warnings`, wasm32 build, `test --all-targets` 269 lib + integration, 0 failed).
- **Worktrees cleaned:** both agent worktrees removed, temp branches `worktree-agent-a2e0a2ca3add365bc` / `worktree-agent-a93c2bf283891e9f6` deleted.
- **Code review run (`/code-review`):** objective normalized-diff method (not agent fan-out). Verdict: **0 findings**.

## What We Tried (Chronological)

1. **Confirmed green starting state.** `git log` (HEAD `e0bad0f`), `gh run list --branch main` (61c09f1 CI = success, e0bad0f run in progress), then ran clippy `--all-targets -D warnings` + wasm build + `test --all-targets` in background → exit 0. Baseline green.
2. **Phase 1 — wrote the local bar.** Read `.github/workflows/ci.yml` (CI order: fmt → clippy `-D warnings` → `test --all-targets` → release build → wasm build job → rustdoc `-D warnings` → package dry-run). Added the Verification section to `CLAUDE.md` (before Project direction) + `AGENTS.md`, created `scripts/verify.sh`, `chmod +x`, ran it → exit 0. Confirmed doc line counts under 200.
3. **Phase 2 — `ecs/world.rs`.** `git diff 5f75a91 61c09f1 -- src/ecs/world.rs` → single hunk (`@@ -905,534 +905,5 @@`), test-only move. Read the full production file; native parallel-query impl gate intact. Confirmed 1 hunk via `grep -c '^@@'`.
4. **Phase 2 — `editor/ui.rs`.** Found `update_editor_ui` is NEW in `61c09f1` (didn't exist at `5f75a91`). Traced the pre-split source: editor UI was inline in `App::update` (`5f75a91:src/app.rs:916`, editor block at line 1065, 8-space indent). Diffed the extracted 957-line slice → exactly 1 line changed (`&egui_ctx`→`egui_ctx`). Verified call site `schedule.rs:255`.
5. **Phase 3 — dispatched two concurrent Sonnet worktree agents** (per launch-prompt instruction + user's aggressive-subagent preference): one to split `audio.rs`, one to extract shortcuts+gizmo from `editor/ui.rs`. Each instructed to run the 4 checks directly (worktrees branch from HEAD, which lacks `scripts/verify.sh`), commit in its worktree, and report branch + symbol diff.
6. **Editor agent returned** (`feb2140`, branch `worktree-agent-a93c2bf...`): `ui/mod.rs` (767) + `shortcuts.rs` (72) + `gizmo.rs` (145), all 4 checks green. Independently verified from the branch: shortcuts.rs verbatim, call sites correct (shortcuts cfg-gated at mod.rs:73-74, gizmo ungated at mod.rs:765 after SelectedEntity sync).
7. **Audio agent returned** (`9305d50`, branch `worktree-agent-a2e0a2ca...`): 7 files, 24 public symbols identical, all 4 checks green (269 tests, 9 audio).
8. **Integrated both into main tree** via `git checkout <branch> -- <paths>` (audio: `src/audio.rs src/audio`; editor: `src/app/editor/ui`), `rm -f src/app/editor/ui.rs` (stale monolith from the rename), `git reset` to leave everything unstaged for review.
9. **Combined verify** on the integrated tree → exit 0. Updated module maps. Cleaned up worktrees + branches.
10. **Code review** (`/code-review`, user request). Used objective method: normalized logic-line diff + attribute audit + whitespace-insensitive body diff. Result: pure moves, 0 findings.

## Key Decisions

- **Executed the optional Phase 3** (the plan marks it optional) because the launch prompt explicitly provisioned it: "use the Agent tool to run them concurrently in worktrees." Honored that directive.
- **Worktree isolation for the two split agents** — each gets its own `target/`, so parallel `cargo` runs don't contend on the build lock. Model = Sonnet (memory: user prefers aggressive subagent use on Sonnet).
- **Agents ran the 4 checks directly, not `scripts/verify.sh`** — worktrees branch from HEAD (`e0bad0f`), which doesn't yet contain the uncommitted `verify.sh`. Avoided needing to commit Phase 1 first.
- **Integration via `git checkout <branch> -- <paths>` + `git reset`**, not merge/cherry-pick — leaves all changes uncommitted in the working tree for a single coherent review, no merge commits on `main`. Rejected: cherry-pick (requires clean tree; Phase 1 changes were dirty).
- **Editor split is a CONSERVATIVE extraction** — only the two self-contained blocks (keyboard shortcuts, gizmo) that reference solely `self`/ctx/`dt`. Left the egui `Window` closures and borrow-state precomputation in `update_editor_ui`. Rejected: extracting the inspector window (deep closure borrows precomputed locals → real refactor risk, behavior change). `ui.rs`→`ui/mod.rs` directory form chosen by the agent so submodules resolve.
- **Audio split by responsibility** — playback / positional / effects / bus+fades / types / source. Private helpers used across new module boundaries promoted to **`pub(super)`/`pub(crate)`** (minimum that compiles); NOT plain `pub` → public API unchanged.
- **Code review done inline with an objective method, not ~20 review agents** — uncommitted+untracked files don't surface cleanly to worktree agents, and a normalized diff *proves* "no logic changed" for a pure-move refactor better than fresh-eyes prose. Recall-mode risk (dropped cfg gate) covered by a dedicated attribute audit.
- **Direct-to-main commit + push** — repo norm (parent seq 2 did the same; git log shows `session:` commits on main), and the user explicitly asked ("커밋 푸쉬 해줘").

## Evidence & Data

### Final verification (integrated tree — all green)

| Command | Result |
| --- | --- |
| `cargo fmt --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | 0 warnings |
| `cargo build --target wasm32-unknown-unknown` (lib+bins) | clean |
| `cargo test --all-targets` | 269 lib + integration, 0 failed |
| `./scripts/verify.sh` (all four, integrated) | exit 0 |

### Audio split — logic & API preservation

| Check | Result |
| --- | --- |
| Normalized logic-line diff (HEAD vs 7 split files) | 430 = 430 lines; **1 diff**: `fades: HashMap<String, Fade>` → `HashMap<String, types::Fade>` (path qualifier, same type) |
| `#[cfg]`/`#[allow]`/`#[derive]` attribute multiset | **identical** (`allow(dead_code)`×1, `cfg(test)`×1, two `derive`×1) |
| Public `pub fn/struct/enum` symbols | **24 identical** |
| Visibility additions | only `pub(crate)` on 3 state helpers, `pub(super)` on `play_internal`/`effective_volume`/`effective_volume_params`/`spatial_params`/`PannedSource`/`Fade` fields — none in public API |

### Editor split — logic & API preservation

| Check | Result |
| --- | --- |
| Normalized logic-line diff (HEAD ui.rs vs mod+shortcuts+gizmo) | 629 → 633; **+4 only**: 2 new method sigs + 2 call sites. No logic line removed/changed |
| `#[cfg(not(target_arch = "wasm32"))]` count | 24 → 28 (+4 = shortcuts.rs `use`×2 + `impl` + gizmo.rs `use`; original block gate → call-site gate, net 0) |
| `#[cfg(target_arch = "wasm32")]` count | 4 → 4 (unchanged) |
| gizmo body vs HEAD lines 825–961 (`diff -w`) | **identical** (internal native/wasm branches intact) |
| shortcuts body vs HEAD lines 72–133 (`diff -w`) | **identical** |
| mod.rs native gates retained | 20 (inspector / scene-save / component-mgmt region preserved) |

### File sizes (before → after)

| File | Before | After |
| --- | ---: | ---: |
| `src/audio.rs` (+ `src/audio/`) | 771 (1 file) | 64 + 6 submodules + tests (largest `playback.rs` 329) |
| `src/app/editor/ui.rs` (+ `ui/`) | 963 (1 file) | `mod.rs` 767 + `shortcuts.rs` 72 + `gizmo.rs` 145 |
| `CLAUDE.md` | 153 | 178 |
| `AGENTS.md` | 104 | 121 |

### Agent runs

| Agent | Branch | Commit | Outcome |
| --- | --- | --- | --- |
| audio split (Sonnet, worktree) | `worktree-agent-a2e0a2ca3add365bc` | `9305d50` | 7 files, 24 symbols identical, 4 checks green, ~6.5min |
| editor split (Sonnet, worktree) | `worktree-agent-a93c2bf283891e9f6` | `feb2140` | mod+shortcuts+gizmo, 4 checks green, ~5.4min |

Both branches deleted after `git checkout`-integration.

## Code Analysis

- **`src/audio.rs` (parent, 64 lines):** `pub struct AudioManager { ... }` (def at line 45), `mod {bus,effects,playback,positional,source,types};`, `pub use types::{AudioChannelState, AudioEffect};`, `#[cfg(test)] mod tests;`. The `fades` field is now typed `types::Fade`.
- **Cross-module `impl AudioManager`** spread across `playback.rs`/`positional.rs`/`effects.rs`/`bus.rs` — the exact idiom already used by `scripting/api.rs` (`impl ScriptingSystem`) and `asset/async_loading.rs` (`impl AssetServer`). Private methods crossing module boundaries are `pub(super)`/`pub(crate)`.
- **`AudioManager::update(&mut self, dt)`** lives in `playback.rs` and drives fades/bus; the helpers it needs (`Fade` fields, bus/effect helpers) are visibility-promoted so it still compiles + behaves identically (verified by logic-line diff + green tests).
- **`src/app/editor/ui/mod.rs`:** keeps `pub(in crate::app) fn update_editor_ui(&mut self, egui_ctx: &Option<egui::Context>, dt: f32)` (signature unchanged) + the egui Stats/Inspector windows + staging-apply + SelectedEntity sync; declares `mod gizmo; mod shortcuts;`.
- **`shortcuts.rs`:** `#[cfg(not(target_arch = "wasm32"))]`-gated module — `fn handle_editor_shortcuts(&mut self, ctx: &egui::Context)` (undo/redo/copy/paste). Called as `#[cfg(not(target_arch = "wasm32"))] self.handle_editor_shortcuts(ctx);` (call-site gate preserves wasm compile).
- **`gizmo.rs`:** ungated module (`fn update_editor_gizmo(&mut self, egui_ctx: &Option<egui::Context>)`) containing both native and wasm cfg branches; native-only imports (`snap_to_grid`, `EditorCmd`) are `#[cfg(not(target_arch = "wasm32"))]`-gated. Called unconditionally after SelectedEntity sync — same position as the original inline block.

## Files Changed

### Source code
- `src/audio.rs` — reduced to parent module (struct def + mod decls + re-exports + tests decl).
- `src/audio/{types,source,playback,positional,effects,bus}.rs` — NEW; `impl AudioManager` split by responsibility + helper types.
- `src/app/editor/ui.rs` — DELETED (renamed to `ui/mod.rs`).
- `src/app/editor/ui/mod.rs` — NEW (was `ui.rs`, minus the two extracted blocks, plus `mod` decls + 2 calls).
- `src/app/editor/ui/{shortcuts,gizmo}.rs` — NEW; extracted `impl App` methods.

### Docs
- `CLAUDE.md` — added `## Verification (run before declaring done)`; audio module-map row → `src/audio.rs`, `src/audio/`.
- `AGENTS.md` — added `### Verification` subsection; replaced vague verify bullet; audio row → `src/audio.rs`, `src/audio/`.

### Tests
- `src/audio/tests.rs` — NEW; 9 audio tests moved verbatim from inline `mod tests`.

### Scripts
- `scripts/verify.sh` — NEW (executable); CI-equivalent 4-check runner with wasm-gotcha note.

### Handoff
- `plans/handoffs/HANDOFF_source-split-refactor_verify-bar-and-splits_2026-06-04.md` — this file.

## User Feedback & Preferences (REQUIRED — never omit)

- **User works in Korean; wants conversational responses in Korean.** (Handoff artifact stays English per the project doc-language rule.)
- Started the session with a paste prompt: "Execute the plan starting at Phase 1 … Do NOT onboard, explore, or ask questions. The plan has everything. Build." → wanted decisive execution, no clarifying questions.
- Explicitly provisioned subagents: "For Phase 3's two independent splits … use the Agent tool to run them concurrently in worktrees."
- "코드리뷰 진행하고 문제점 있나 확인" — wanted a real code review of the changes, with a problems verdict.
- "/handoff 하고 커밋 푸쉬 해줘" — wanted a handoff THEN commit + push (this session's close).
- Memory: prefers **aggressive subagent use for parallel work, on the Sonnet model**.
- Memory/parent: **direct-to-main commit + push is the established repo norm**; values thoroughness and double-checking (parent asked for a second full review).
- Doc-language rule: English prose in docs to cut tokens; Korean kept only for the beginner glossary + gitignored one-off prompts.

## Where We're Going

The `source-split-refactor` epic is essentially complete. Remaining items are small and optional:

1. **Resolve the parent's open question:** should `engine::renderer::FrameContext` stay a public re-export, or become crate-internal for a tighter fork-friendly surface? (No code depends on it externally; rust-survivors doesn't use it.)
2. **Optional further extraction of `ui/mod.rs` (767)** — if it ever approaches 1000, the egui `Window::new("Inspector")` closure (~470 lines) is the next candidate, but it needs the precomputed borrow-state, so it's a real refactor, not a move. Defer until actually needed.
3. **Nothing else pending** for this chain — consider archiving the chain's handoffs/PLANs to `plans/handoffs/archive/` once confirmed.

## Risks & Blockers

- **Low.** All work committed + pushed + CI will validate on push (CI enforces clippy `-D warnings` + wasm build + rustdoc + package dry-run). Local `verify.sh` already green.
- wasm `--all-targets` will always "fail" on the 3 native-only examples (`platformer_game`/`mp_server`/`gpu_particles`) — `verify.sh` correctly gates wasm on lib+bins. Don't treat the example failures as a regression.
- The audio `pub(crate)`/`pub(super)` promotions are crate-internal only; if a future change tightens them, re-run the symbol diff to confirm no public-API drift.

## Open Questions

- `FrameContext` public re-export long-term (carried from parent seq 2; not addressed this session).
- Whether to archive the `source-split-refactor` chain now that the epic is complete.

## Quick Start for Next Session

```bash
# Restore context (bd unavailable)
cat plans/handoffs/HANDOFF_source-split-refactor_verify-bar-and-splits_2026-06-04.md
cat plans/handoffs/HANDOFF_source-split-refactor_regression-fixes_2026-06-03.md   # parent (seq 2)

# Confirm green, clean starting state
git status -s                    # expect clean
git log --oneline -5
./scripts/verify.sh              # CI-equivalent: fmt, clippy -D warnings, wasm build, test --all-targets
gh run list --branch main --limit 3   # CI on the pushed commit

# Key files (the split outputs)
sed -n '1,40p' src/audio.rs              # parent module: struct + mod decls + re-exports
ls src/audio/                            # types/source/playback/positional/effects/bus/tests
ls src/app/editor/ui/                    # mod.rs (767) + shortcuts.rs + gizmo.rs
sed -n '1,30p' CLAUDE.md                 # the Verification block (Phase 1)

# Next action
# The source-split epic is complete. If continuing: resolve the FrameContext
# public-re-export question (engine::renderer::FrameContext) or archive this
# chain's handoffs. Otherwise start a new chain for the next feature.
```

## Session Closed
**Closed at:** 2026-06-04 19:08:13 KST
**Commit:** `b97071a` (refactor work) + the `session:` commit carrying this handoff
**Session status:** Handed off to next session — committed and pushed to `origin/main`

## Post-Handoff Follow-up (rustdoc CI fix)

After pushing, CI on `31b4af8` **failed the Rustdoc job** (native/wasm/test/package all passed). Root cause: the audio split moved `AudioChannelState` into `src/audio/types.rs`, breaking its `[`AudioManager`]`/`[`AudioManager::stop`]` intra-doc links (`AudioManager` is in the parent `audio` module, out of scope in `types.rs`). CI's `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` gate caught it — but `scripts/verify.sh` and the Phase 1 verify blocks had **omitted the rustdoc gate**, so the local bar wasn't truly CI-equivalent (the exact gap class Phase 1 targets).

**Fix (`9aebd6a`, pushed, CI green):**
- `src/audio/types.rs` — absolute-path links: `[`AudioManager`](crate::audio::AudioManager)` and `[`AudioManager::stop`](crate::audio::AudioManager::stop)` (same rendered text, no import needed).
- `scripts/verify.sh` + `CLAUDE.md` + `AGENTS.md` — added the `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` gate so the local bar now covers **5** checks (fmt, clippy, wasm, test, **rustdoc**).
- Local `./scripts/verify.sh` (with rustdoc) → exit 0; CI run `26945631687` on `9aebd6a` → **success** (all 4 jobs green).

**Lesson for next session:** when splitting a module, same-module intra-doc links (`[`Type`]`/`[`Type::method`]`) break if the referenced item stays in the parent — use absolute-path links or run the rustdoc gate locally. `verify.sh` now includes it.
