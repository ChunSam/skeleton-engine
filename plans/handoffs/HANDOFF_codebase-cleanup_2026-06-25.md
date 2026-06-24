# Codebase cleanup pass 2: extract the frame.rs render() god-function (v0.68.3) + split the dialogue grab-bag (v0.68.4)

**Date:** 2026-06-25
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `codebase-cleanup` seq `2`
**Parent:** `HANDOFF_codebase-cleanup_2026-06-24.md` (seq 1)
**Prior chain:** `HANDOFF_codebase-cleanup_2026-06-24.md` > this

---

## Since Last Handoff

Seq 1 (2026-06-24) ended with the board ACTIVE EMPTY and three cleanups landed (resources.rs #241, world.rs #242, CLAUDE.md trim #243, + the handoff #244). Its "Where We're Going" said: read the wishlist board first; if still empty, ASK; and flagged the *marginal* remaining split candidates (focus_pass.rs 922, timeline.rs 832) as "only split if a future change makes one unwieldy", concluding "no other 1000+-line non-test source files remain."

This session followed that exactly and then went one level deeper than seq 1 had looked:

- Board re-checked → still ACTIVE EMPTY (EW-004 next) → ASKED → user chose **continue cleanup**.
- **Seq 1's "largest files" table was by total `wc -l`, which hid two real targets it didn't analyze:** (1) `frame.rs` is 770 lines of **pure code** (no `#[cfg(test)]`) and its `render()` is a single **~680-line god-function** — worse than any god-file but invisible to a line-count-only scan; (2) `dialogue/mod.rs` (730) is a genuine **grab-bag** of 3 concerns. Both were picked up this session.
- Seq 1's open question "is it time to return to feature work, or a 3rd cleanup?" → user picked a **3rd (and 4th) cleanup**. The board-empty→ASK directive held again.
- No risks from seq 1 materialized; the `/tmp/ecs_backup.md` memory backup from seq 1 was not needed.

## Reference Documents

- `CLAUDE.md` — agent quick reference (module map, verification gate, conventions). Header now **v1.6.148**; the dialogue + ECS-world module-map rows describe these splits.
- `docs/VISION.md` — the forkable-skeleton north star + the feature+example acceptance loop.
- `docs/PATTERNS.md` — core architecture patterns (incl. the render-target-format-aware pipeline-cache pattern).
- `../dungeon-merchant/docs/engine-wishlist.md` — the downstream game↔engine board (EW-NNN). Read FIRST each session.
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — the LIVE per-seq state. **Pending update**: bump to seq 95 (main @ this handoff's merge) — see "Where We're Going".

## The Goal

Keep `skeleton-engine` a clean, fork-friendly **skeleton** (`docs/VISION.md`). With the wishlist board empty (no queued feature work), the user elected a continued **codebase-cleanup** pass: find and land the highest-value behavior-preserving cleanups, each as a merged PR on green CI, mirroring the prior resources.rs/world.rs split PATCHes. This session's objective became two such refactors, landed back-to-back as separate PRs: (A) de-god-function the render orchestration, and (B) de-grab-bag the dialogue module. The acceptance bar throughout: **no public API change, no behavior change, verify gate green, CI green** — with the extra judgment gate that GPU/native-only code CI can't run needs a real-behavior confirmation before merge.

## Where We Are

- **main @ `5d06dca`**, working tree **clean**, no open PRs (both code PRs merged). Package **v0.68.4**, CLAUDE.md header **v1.6.148**.
- **#245 (A) MERGED** (`90011fa`, v0.68.3) — `refactor(render): extract docked scene-target prep out of render()`. The native-only docked-RT management block (~150 lines) moved verbatim from `frame.rs::render()` into a new associated fn `App::prepare_docked_scene_view(render, editor, window, gpu) -> Option<wgpu::TextureView>` in `src/app/render/docked.rs`. `render()`: 705 → 572 lines in the file (~133 fewer). 2 files: `docked.rs` +152, `frame.rs` +9/−145.
- **#246 (B) MERGED** (`5d06dca`, v0.68.4) — `refactor(dialogue): split dialogue/mod.rs grab-bag into themed submodules`. The 730-line `dialogue/mod.rs` → `model.rs` (451) + `style.rs` (93) + `system.rs` (174) + `mod.rs` (44, re-export coordinator). `tests.rs` +6 (one import). `tree.rs`/`vars.rs` untouched.
- **Final line counts**: `frame.rs` 634, `docked.rs` 286; `dialogue/{mod 44, model 451, style 93, system 174}`.
- The verify gate (`./scripts/verify.sh`) ran green (`VERIFY_EXIT=0`) **6 times** this session: A baseline (implicit via check), A post-extract, A post-bump; B post-split (FAILED→fixed→re-run), B post-doc-fix, B post-bump. **931 lib tests** unchanged throughout; doctests pass.
- **A was verified beyond CI** (it's GPU + native + editor-only, which ubuntu CI cannot exercise) via two independent methods: a **token-equivalence diff** and a **parent-commit native screenshot comparison** (details below). No regression.
- **B is CI-verifiable** (pure logic + the existing dialogue unit + doc tests) — green CI fully covers it; no native smoke needed.
- The split is **semver-invisible** for both: every public path (`engine::DialogueBox`, `crate::dialogue::*`, `App::render` internals) is unchanged; downstream consumers need no change.
- Two memory seqs are **pending** (to bump after THIS handoff PR merges): seq 94 = A (#245), seq 95 = B (#246).
- Board remains **ACTIVE EMPTY** (EW-004 next).

## What We Tried (Chronological)

1. **Onboarding + board check.** Read parent handoff (seq 1) + `../dungeon-merchant/docs/engine-wishlist.md` → board ACTIVE EMPTY (EW-001/002/003 all Verified/archived). Confirmed main @ `f23e5e5` (#244 handoff merge — one commit past the `656926a` the parent body named, exactly as expected since a handoff is written before its own PR merges). Ran baseline `verify.sh` → green. ASKED user → **continue cleanup**.
2. **Fresh health scan (went deeper than seq 1).** `grep TODO/FIXME/HACK/XXX` → 0; `#[allow]` → 43 (unchanged, justified). Largest *pure-code* files (after subtracting `#[cfg(test)]` boundaries): `docked.rs` 959, `audio_wasm.rs` 882, `window.rs` 844, **`frame.rs` 770 (no tests at all)**, `dialogue/mod.rs` 730. `app.rs` 1049 is ~340 code + tests (seq 1 was right to skip it). **`frame.rs::render()` mapped to a single ~680-line fn** (only `present_egui` 5-line helper beside it) → the real worst-offender.
3. **Picked targets + asked.** Presented A (frame.rs god-function, highest value but GPU/CI-unverifiable) vs B (dialogue grab-bag, CI-verifiable, lower risk). User picked **"both, A first then B."**
4. **A: implement.** Read `render()` fully. Found `src/app/render/` already holds 4 extracted-helper files (`docked.rs`/`offscreen.rs`/`post_lighting.rs`/`debug_draw.rs`) using an **associated-fn pattern** (`Self::foo(&mut self.render, …, gpu)` — disjoint borrows, NOT `&mut self`) — e.g. `render_offscreen_targets(&mut self.render, &mut self.world, gpu)` already takes `&mut self.render` + `gpu` simultaneously, proving the borrow shape. Extracted the docked block (frame.rs 33–182) into `docked.rs::prepare_docked_scene_view(render: &mut RenderState, editor: &mut EditorState, window: Option<&Window>, gpu: &GpuContext)`, replacing the inline `let docked_render_view = { … }` with a 1-line call. `cargo fmt` + `cargo check --lib` green first try; dead_code warning was a stale rust-analyzer artifact (both fn + call are `#[cfg(not(wasm32))]`).
5. **A: verify the un-CI-able runtime.** Built `hello_sprite` (a single centered spinning sprite). Synthetic F2 (`osascript … key code 120`) → first attempt screenshot showed the *non-docked* window (F2 not delivered). Diagnostic: sent Escape → app quit → **synthetic keys ARE delivered**; second F2 attempt → docked editor rendered. Captured the window precisely via `osascript … get {position, size} of front window` (→ 560,135,800,632 logical × 2 retina), cropped → **full docked editor renders** (toolbar/panels/central viewport with correct clear-color). The central viewport showed no sprite — to rule out my change, built the **parent commit (`f23e5e5`) binary in a `git worktree`** and ran the same smoke → **byte-identical docked render, also no sprite** → confirmed pre-existing (the 800×600 scene projected into the narrow docked RT), not my regression.
6. **A: token-equivalence proof.** Normalized the original frame.rs block (HEAD~1, lines 38–178) and the extracted fn body: strip comments, **remove whitespace FIRST** (the original hand-wrapped `self`/`.window` across lines, so a per-line `s/self.//` missed them — a normalization bug I hit and fixed), then strip `self.`/`.as_ref()`/`&mut *`. Result: **char-for-char identical except ONE `cargo fmt` trailing comma** (multi-line `(a, b,)` → single-line `(a, b)`, semantically identical). This is a stronger refactor proof than any single screenshot.
7. **A: ship + land.** `/ship` 0.68.2→0.68.3 (Cargo.toml/lock, CHANGELOG, CLAUDE.md header v1.6.147). Re-verify green. Branch `refactor/extract-docked-scene-view`, PR **#245**, CI 4/4 (native 4m39s), mergeStateStatus CLEAN → squash-merge → sync main.
8. **B: implement.** Validated the split is test-safe (the seq-1 world.rs lesson): `grep`ed `tests.rs`/`tree.rs`/`vars.rs` for DialogueBox private fields (`elapsed`/`full`)/private methods (`is_available`/`pending_choices_raw`)/struct-literal construction → all matches were comments, a RON string literal, or `-> DialogueBox` return types (no private access). Kept `DialogueChoice`+`DialogueBox` in ONE file (`model.rs`) because `DialogueBox::visible_choices` calls the **private** `DialogueChoice::is_available`. Wrote `model.rs`/`style.rs`/`system.rs`, rewrote `mod.rs` as a 44-line coordinator. `cargo check --lib` green.
9. **B: verify FAILED twice, fixed.** (1) `cargo test --all-targets` → 4× `E0433: cannot find type LocaleResource` in `tests.rs` — it had inherited `LocaleResource` through `mod.rs`'s private `use … LocaleResource;` via `use super::*`; the split removed that import. Fixed by adding `use crate::locale::LocaleResource;` to `tests.rs` (World/Events/TextQueue/ViewportSize were already imported locally per-test, so only LocaleResource broke). (2) Re-verify → `cargo doc -D warnings` → 5× "public documentation for `dialogue` links to private item `model`/`style`/…" — my new mod doc used intra-doc links `[`model`]` to the private submodules. Fixed by switching those to plain code spans. Third verify → green.
10. **B: ship + land.** `/ship` 0.68.3→0.68.4 + the dialogue module-map row in CLAUDE.md. Branch `refactor/split-dialogue-module`, PR **#246**, CI 4/4 (native 5m42s), CLEAN → squash-merge → sync main.
11. **Wrap.** User chose "write handoff + close." This doc.

## Key Decisions

- **A's extraction target = the docked block specifically.** It's the single most self-contained chunk of `render()` (a `let x = { … }` native-only island), touches only `self.render`/`self.editor`/`self.window`/`gpu` (disjoint), and has a sibling helper (`present_docked_placeholder`) already in `docked.rs` with the same signature shape — so it *joins an existing file + pattern* rather than inventing structure. One focused extraction (not a full pass-by-pass shredding of `render()`) keeps the diff trivially reviewable.
- **Associated fn, not `&mut self` method.** `render()` holds `gpu = self.gpu.as_mut()` across its whole body; a `self.prepare_docked_scene_view(gpu)` call would alias `self`. The established `src/app/render/*` pattern (and the now-extracted fn) take disjoint field borrows as explicit params, composing with the live `gpu` borrow.
- **A is GPU/native/editor-only → green CI is NOT sufficient** (CLAUDE.md gate). Did the token-equivalence diff + a parent-vs-child native docked screenshot comparison *before* merging. The diff proves all states behave identically (stronger than a one-frame screenshot); the parent comparison empirically rules out regression and attributes the "no sprite in docked viewport" to pre-existing projection behavior.
- **B keeps `DialogueChoice` + `DialogueBox` together in `model.rs`.** Splitting them would expose the private `DialogueChoice::is_available` (called by `DialogueBox::visible_choices`) across a module boundary, forcing a `pub(crate)` widening. Co-locating preserves encapsulation with zero visibility changes.
- **`advance`/`choose` free fns → `model.rs`** (not `mod.rs` or `system.rs`). They're world-level operations on the box model (delegating to `DialogueBox::advance_with`/`choose_visible`), so they belong with the model; `mod.rs` stays a pure re-export coordinator (mirrors the resources.rs precedent).
- **Module named `model.rs`, not `box.rs`.** `box` is a Rust keyword — `mod box;` is a syntax error.
- **Both PATCH (0.68.3 / 0.68.4).** Pre-1.0 rule: internal refactor, no API/behavior change = PATCH. Mirrors #241 (0.68.1) / #242 (0.68.2).
- **Two separate PRs, not one.** "One PR = one coherent change." A (render) and B (dialogue) are unrelated subsystems; landing separately keeps each diff and its verification self-contained.

## Evidence & Data

### Commits landed this session (main)

| Hash | PR | Type | Bump | Summary |
|---|---|---|---|---|
| `90011fa` | #245 | refactor(render) | 0.68.2→0.68.3 | extract docked scene-target prep out of render() |
| `5d06dca` | #246 | refactor(dialogue) | 0.68.3→0.68.4 | split dialogue/mod.rs grab-bag into themed submodules |

### A — frame.rs render() line accounting (PR #245: 6 files, +172 / −148)

| File | Δ | Note |
|---|---|---|
| `src/app/render/frame.rs` | +9 / −145 | `render()` 705→572 in-file; block → 1-line call |
| `src/app/render/docked.rs` | +152 | new `prepare_docked_scene_view` (+ `use … EditorState`) |
| Cargo.toml/lock, CHANGELOG, CLAUDE.md | +14 | v0.68.3 paperwork |

### B — dialogue split line accounting (PR #246: 9 files, +753 / −706)

| File | Lines (final) | Contents |
|---|---:|---|
| `src/dialogue/mod.rs` | 44 | doc + `mod` decls + `pub use` re-exports + `#[cfg(test)] mod tests` |
| `src/dialogue/model.rs` (new) | 451 | `DialogueChoice` + `DialogueBox` + `advance`/`choose` |
| `src/dialogue/style.rs` (new) | 93 | `DialogueStyle` + `Default` |
| `src/dialogue/system.rs` (new) | 174 | private `DrawItem` + `DialogueSystem` |
| `src/dialogue/tests.rs` | +6 | one `use crate::locale::LocaleResource;` + comment |

### Verify gate runs (all eventually `VERIFY_EXIT=0`)

| When | Result | Note |
|---|---|---|
| A baseline | 0 | 931 tests, fmt/clippy/wasm/doc clean |
| A post-extract | 0 | behavior preserved |
| A post-bump (0.68.3) | 0 | lock + doc rebuilt |
| B post-split | **101** | `tests.rs` lost `LocaleResource` (4× E0433) |
| B post-LocaleResource-fix | **101** | `mod.rs` doc linked private modules (5× rustdoc error) |
| B post-doc-fix | 0 | green |
| B post-bump (0.68.4) | 0 | green |

### CI results

| PR | Test (native) | Build (WASM) | Rustdoc | Package dry-run | mergeState |
|---|---|---|---|---|---|
| #245 | pass 4m39s | pass 49s | pass 38s | pass 1m2s | CLEAN → squash |
| #246 | pass 5m42s | pass 44s | pass 46s | pass 1m2s | CLEAN → squash |

### A — token-equivalence (the core correctness proof)

Normalized original block (frame.rs@HEAD~1 L38–178) vs extracted fn body: identical char-for-char **except a single `cargo fmt` trailing comma** at the `if let (Some(er), Some(old_id)) = (egui_renderer.as_mut(), editor.docked_texture_id.take())` site (original wrapped the tuple multi-line → trailing comma; fmt put mine single-line → none). Semantically identical.

### A — docked smoke artifacts (native, /tmp, session-local)

- `/tmp/window_exact.png` — child (this-change) docked editor, full window.
- `/tmp/parent_exact.png` — parent-commit docked editor, **identical render**.
- Window bounds both runs: logical `560,135 / 800,632` → retina ×2.

## Code Analysis

- **`prepare_docked_scene_view` signature:** `#[cfg(not(target_arch = "wasm32"))] pub(in crate::app) fn prepare_docked_scene_view(render: &mut RenderState, editor: &mut EditorState, window: Option<&winit::window::Window>, gpu: &GpuContext) -> Option<wgpu::TextureView>`. Returns `None` when not docked / RT warming up → caller's downstream passes target the surface unchanged → **common path structurally byte-identical**.
- **Field types threaded** (from `App`): `render: RenderState`, `editor: EditorState`, `window: Option<Arc<Window>>` (→ `self.window.as_deref()` = `Option<&Window>`), `gpu: Option<GpuContext>` (→ `self.gpu.as_mut()`, reborrowed `&` at the call).
- **`render()` is still a long pass-sequence** (~545 lines): clear → sprites → debug→UiQueue → UI primitives → GPU particles → render plugins → bloom → post → lighting → HUD/text → fade → docked surface clear → submit → egui. Further extraction is possible but each pass is short and the sequence reads top-to-bottom; the docked island was the one genuinely-tangled block. Not worth shredding further now.
- **Dialogue private surface preserved:** `DialogueBox.elapsed`/`.full` (private, only touched by DialogueBox's own impl), `DialogueChoice::is_available` (private, called by `DialogueBox::visible_choices` — same module), `pending_choices_raw` (private), `DrawItem` (private, system.rs-only). `DialogueSystem` reads only public API (`speaker`/`portrait` pub fields + public methods), so the model↔system split needed no widening.
- **`DialogueBox` derives `Serialize/Deserialize`** over private `elapsed`/`full` (no `#[serde(skip)]` — they round-trip in scene RON; the `tests.rs` RON literal at L136 exercises this). Derive moves with the struct, so the split is serde-transparent.

## Gotchas & Discoveries (this session)

### Tooling / harness
- **A backgrounded `./scripts/verify.sh > log 2>&1; echo "VERIFY_EXIT=$?"` masks a real failure.** The compound command's exit (reported by the background-task completion notification) is the **`echo`'s** 0, not verify.sh's. B's first verify actually exited **101** (the real value was inside the task's stdout file as `VERIFY_EXIT=101`, and the failure text was in the `.log`), yet the task summary said "exit code 0." **Fix: background `./scripts/verify.sh > log 2>&1` with NO trailing `; echo`** — then the task's own exit code == verify.sh's. (Foreground `; echo "$?"` is fine; this only bites backgrounded runs. A cousin of the documented "don't pipe a gate's exit code" trap.)
- **Token-equivalence diff = the right proof for a verbatim refactor** (reusable technique). Normalize both the original block and the extracted body — `grep -v` comment lines, then **`tr -d '[:space:]'` BEFORE** any `sed 's/self.//'` (the original was hand-wrapped, so `self` and `.field` sit on different lines; a per-line `sed` misses them — I hit this exact bug and the first diff was all-noise). Then strip the mechanical renames (`self.`/`.as_ref()`/`&mut *`). A clean diff proves identical behavior in ALL states — stronger than a single-frame screenshot.
- **Synthetic macOS key delivery to a non-bundled binary works, but verify it.** `osascript -e 'tell application "System Events" to key code 120'` (F2) reached the window only after `set frontmost of (first process whose name contains "hello_sprite")`. First F2 silently no-op'd; a control test (`key code 53` = Escape → app quit) confirmed keys ARE delivered, so the retry succeeded. Capture the window precisely with `osascript … get {position, size} of front window` × the retina scale (2×), then `sips -c H W --cropOffset Y X`, rather than guessing crop coords.
- **rust-analyzer flagged the new `prepare_docked_scene_view` as `dead_code` transiently** — both the fn and its only caller are `#[cfg(not(wasm32))]`; the warning was a stale-index artifact between the two edits. `cargo check --lib` (the real arbiter) was clean. Don't trust an IDE diagnostic over a build.

### Rust / engine
- **A `use super::*` glob silently inherits the parent module's PRIVATE `use` imports.** `dialogue/tests.rs` compiled for months using `LocaleResource` with no local import — it came from `mod.rs`'s private `use crate::locale::LocaleResource;` through the glob. Moving the types out of `mod.rs` removed that import and broke `tests.rs` (4× E0433). The world.rs seq-1 lesson ("check the test file doesn't name a relocated private item") generalizes: also check names the test inherits via the parent's glob, not just private items it spells directly.
- **A public module doc can't intra-doc-link a private submodule under `-D warnings`.** `mod.rs`'s new doc used `[`model`]`/`[`style`]`/… → 5× "public documentation for `dialogue` links to private item." `cargo test --all-targets` does NOT catch this (it's a `cargo doc` gate, which runs LAST in verify) — so a split can pass tests and still fail the doc build. Use plain code spans for private-module names; qualify cross-module item links (`crate::DialogueSystem`).
- **`main` was one commit past what the parent handoff's body named** (`f23e5e5` vs `656926a`) — expected, because a handoff is written *before* its own `docs(handoff)` PR merges, so the recorded tip always lags the actual tip by that one merge. Not drift; don't "fix" it.

## Files Changed

### Source — A (#245)
- `src/app/render/frame.rs` — the `#[cfg(not(wasm32))] let docked_render_view = { … }` block (~150 lines) replaced by a `Self::prepare_docked_scene_view(&mut self.render, &mut self.editor, self.window.as_deref(), gpu)` call. wasm fallback (`= None`) unchanged.
- `src/app/render/docked.rs` — new native-only `prepare_docked_scene_view` beside `present_docked_placeholder`; added `use crate::app::editor::EditorState;`.

### Source — B (#246)
- `src/dialogue/mod.rs` — reduced 730→44 lines: doc + `mod {model,style,system,tree,vars}` + `pub use` re-exports + `#[cfg(test)] mod tests`.
- `src/dialogue/model.rs` — **new**; `DialogueChoice` + `DialogueBox` + `advance`/`choose`. `DialogueSystem` doc-links qualified `crate::DialogueSystem`.
- `src/dialogue/style.rs` — **new**; `DialogueStyle` + `Default`. `ViewportSize`/`DialogueSystem` doc-links qualified.
- `src/dialogue/system.rs` — **new**; private `DrawItem` + `DialogueSystem`.
- `src/dialogue/tests.rs` — +`use crate::locale::LocaleResource;` (lost glob-inherited import).

### Docs / paperwork (both PRs)
- `Cargo.toml`/`Cargo.lock` (0.68.2→0.68.3→0.68.4), `docs/CHANGELOG.md` (0.68.3 + 0.68.4 entries), `CLAUDE.md` (header v1.6.146→147→148 + dialogue + ECS-world module-map rows).

### Memory (pending — see Where We're Going)
- `~/.claude/projects/-Users-jkl-Projects-skeleton-engine/memory/engine-current-state.md` — bump to seq 95, main @ this handoff's merge.

## User Feedback & Preferences

- **Session opener (pasted):** read seq-1 handoff, continue from "Where We're Going"; board-empty → ASK directive.
- When the board was empty and offered options, user chose **"코드베이스 정리 계속"** (continue cleanup) — a 4th consecutive cleanup over feature work.
- For the cleanup target, user chose **"둘 다 (A 먼저, 그 다음 B)"** — both, A first then B, as two separate PRs.
- At wrap, user chose **"핸드오프 쓰고 마무리 (추천)"** — write the handoff + close.
- Standing prefs (honored): user-facing reports in **Korean**, agent-to-agent/code/docs in **English**; merge authority **standing-delegated** (squash on green CI, no re-confirm) but the OS/GPU-CI-unverifiable judgment gate overrides blanket delegation; always pass explicit `model` to subagents; run `cargo fmt` before verify; never mask a gate's exit code.

## Where We're Going

1. **This handoff lands as its own `docs(handoff)` PR** (branch `docs/handoff-seq2-cleanup`, commit `docs(handoff): codebase-cleanup seq 2 …`, no package bump). After it merges:
2. **Bump the memory** (`engine-current-state.md`): add **seq 94** (A, #245, v0.68.3, `90011fa`) and **seq 95** (B, #246, v0.68.4, `5d06dca`) to the recent-seqs list, update the `main @ <hash>` (→ the handoff merge) + version + CLAUDE.md-header pointers, and trim the oldest seq to `[[engine-history-archive]]` to keep the file compact. The seq-bump belongs to THIS handoff PR's landing (the session tip).
3. **Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004 next). If a new EW-NNN was filed → VISION feature+example loop. If still empty → **ASK** the user (cleanup is now thin — see below).
4. **If a 5th cleanup is requested:** the codebase is now genuinely clean. No 1000+-line non-test files; `frame.rs::render()` is no longer a god-function. The only *marginal* candidates left are cohesive single-systems (`timeline.rs` 832, `focus_pass.rs` ~250 code, `audio_wasm.rs` 882, `window.rs` 844) — splitting any of these is shaping, not de-god-ifying; do it only if a future change makes one unwieldy. Lean toward telling the user cleanup has reached diminishing returns and feature work is the higher-value path.
5. **If feature work:** follow the VISION loop — a new capability isn't done until a small playable `examples/` game exercises it; `/add-feature-example` → `/ship` → `/land-pr`.

## Risks & Blockers

- **None blocking.** Tree clean, CI green, both code PRs merged.
- A's docked-mode runtime is CI-invisible (ubuntu/no-GPU). This session closed that gap with the token-equiv proof + parent screenshot comparison; future edits to `prepare_docked_scene_view` need the same native-docked check (F2 on any windowed example), since CI still won't catch a docked-RT regression.
- The `/tmp/*.png` smoke artifacts + any `/tmp/parent-wt` worktree are session-local (worktree was `git worktree remove`d). Not needed going forward.

## Open Questions

- **None blocking.** All verify failures this session were resolved (LocaleResource import, private-module doc links).
- Open-but-not-urgent: is cleanup done? After 4 consecutive cleanups the answer is "effectively yes" — the next board-empty session should weigh feature work first. The user's call.

## Quick Start for Next Session

```bash
# 1. Read the downstream wishlist board FIRST (standing directive)
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY? EW-004 next → ASK if empty

# 2. Confirm engine state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4        # tip should be the seq-2 handoff merge (this PR), above 5d06dca (#246)
git status -s               # clean

# 3. Key files this session touched (read if continuing cleanup)
#    src/app/render/frame.rs    — render() now ~545 lines (was ~680); pass-sequence
#    src/app/render/docked.rs   — prepare_docked_scene_view + present_docked_placeholder
#    src/dialogue/{mod,model,style,system}.rs   — the split

# 4. Verify (read the exit code; do NOT append `; echo` to a backgrounded run — it masks the real exit)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"   # foreground: fine. background: drop the echo.

# 5. Memory: bump engine-current-state to seq 95 AFTER this handoff PR merges (main @ handoff merge)

# Next action:
#   Read ../dungeon-merchant/docs/engine-wishlist.md. New EW → implement via VISION loop.
#   Still empty → ASK; note cleanup has reached diminishing returns (lean feature work).
```

---

## Session Closed

**Closed at:** 2026-06-25 (KST)
**Commit:** lands via a `docs(handoff)` PR (this file)
**Session status:** Handed off — engine work (#245, #246) merged to `main`; this handoff is the session record. Memory seq-bump (94/95) pending this PR's merge.
