# Engine-wide hardening batch (v9.0.0) — reviewed, reconciled, and MERGED to main

**Date:** 2026-06-16
**Status:** ✅ DONE — v9.0.0 squash-merged to `main` (`7e48794`); concurrent-session history reconciled; full review + CI green
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / robustness pass
**Chain:** `engine-hardening` seq `2`
**Parent:** `HANDOFF_engine-hardening_v9.0.0-shipped_2026-06-16.md` (seq 1)
**Prior chain:** none — `engine-hardening` is its own stream (forked from the `editor-tile-painting` chain when the user said "limit으로 멈춘 작업 다시 진행")

## The Goal

Close out the engine-hardening batch (80 findings from `docs/CODE_ANALYSIS_2026-06-16.md`, WU1–14): **reconcile the two concurrent sessions that raced the iteration-3 repair, code-review the full v9.0.0 stack, create a PR, verify tests, and merge to `main`.** Seq 1 left the work "implemented, Gate6-green, but NOT merged — reconciliation pending." Seq 2 (this session) finished that.

## Where We Are

- **`main` = `7e48794` (v9.0.0).** PR **#60** squash-merged 2026-06-16 (76 files, +4890/−437). Branch `chore/engine-hardening-2026-06-16` deleted (remote + local); redundant `chore/hardening-opus-2026-06-16` worktree/branch removed. Clean tree.
- **Squash merge collapsed the interleaved dual-session history into ONE clean commit** — resolving the `e96529d` "foreign/duplicate commit" concern (seq 1's #1 open item). `main` now reads `7e48794` → `42de46c` with no interleaving visible.
- **v9.0.0 (major) confirmed correct** — breaking changes present (`#[non_exhaustive]` GamepadButton/GamepadAxis, `SolidTiles::Only` Vec→HashSet, `NetworkSystem` unit→struct, `SerdeComponentEntry` +field, MSRV 1.92→1.95, blob_47 atlas-index reorder).
- **Independent full-stack code review run** (Sonnet, read-only) before merge. It **confirmed the high-risk changes sound** and surfaced **1 real regression + 3 doc gaps**, all fixed in commit `fd83587` (now squashed into `7e48794`).
- **CI green 4/4** on PR #60: Test (native) 4m19s, Build (WASM) 1m55s, Package dry-run 1m13s, Rustdoc 49s. Local full gate also green (fmt, clippy --all-targets -D warnings, test --all-targets = 698 lib + integration / 0 fail, doc -D warnings, wasm lib+bins).
- **Cross-session memory updated** (`engine-current-state.md`) from "NOT merged / reconcile pending" → "MERGED via PR #60".

## Since Last Handoff (vs seq 1)

Seq 1's `## Where We're Going` had 2 blocking items + optional follow-ups. Status now:

1. **"Reconcile the concurrent session FIRST"** → ✅ DONE. The two sessions (seq-1 author on the shared `main`-working-tree branch, and THIS session which had branched an isolated `chore/hardening-opus-2026-06-16` worktree to do WU12/WU14 in parallel) **converged**: the shared branch (`chore/engine-hardening-2026-06-16`) ended up containing the complete work (WU1–14 + v9.0.0). My parallel opus branch was therefore **redundant and discarded**. The interleaved `e96529d` checkpoint was collapsed by the squash merge.
2. **"Then push / open PR / merge to main"** → ✅ DONE. PR #60 → squash-merge → `7e48794`.
3. **"(optional) update progress-doc checkboxes"** → left as-is (CHANGELOG + the squashed commit are authoritative; the progress doc + both seq-1/seq-2 handoffs are committed to main).
4. **Seq 1's open question "is v9.0.0 right?"** → resolved: YES (the review independently confirmed the breaking set; no accidental extra breaks).

**New this session that seq 1 did not cover:** the code review caught a genuine regression seq-1's Gate6 missed — the Tilemap `generation` dirty-guard skips rebuilds on **direct `pub tiles` field mutation** (not just `set_tile`), which `dig_quest`'s reset (`tm.tiles = initial_tiles()`) hits → stale render. Fixed.

## Reference Documents

- `docs/CODE_ANALYSIS_2026-06-16.md` — the 80-finding audit (Appendix A = severity/file:line/effort/breaking).
- `docs/CHANGELOG.md` — the 9.0.0 entry = authoritative human record (+ my added blob_47 migration note).
- `plans/handoffs/HANDOFF_engine-hardening_v9.0.0-shipped_2026-06-16.md` — seq 1: full implementation detail, WU→findings matrix, concurrent-session forensics.
- `plans/code-analysis-2026-06-16-progress.md` — execution plan + prior RESUME-STATE note.
- `CLAUDE.md` — Gate6 checklist.

## What We Tried (Chronological)

**Full arc of THIS session (disambiguating the two-session shape — important):** THIS session is the one that *started* the whole batch: (a) authored `docs/CODE_ANALYSIS_2026-06-16.md` (the 80-finding audit, via a 14-subsystem fan-out Workflow + adversarial-verify pass); (b) ran the `/loop` — planned WU1–14, implemented **Iter 1 (WU1–4, `0d6da75`)** + **Iter 2 (WU5–7,13, `f5260ef`)** supervising Sonnet sub-agents, launched Iter 3; (c) **hit a usage limit mid-Iter-3**, wrote a RESUME-STATE note into `plans/code-analysis-2026-06-16-progress.md` (`613b9c6`), paused. A **second/concurrent Claude session** then resumed the same tree, did Iter 3–5 + the v9.0.0 finalize on the shared branch, and wrote the **seq-1 handoff** (so seq 1's "THIS session" = that other session; its "concurrent session" = me). When I resumed (steps below), I worked in parallel on an isolated worktree, found the other session had already completed everything, discarded my redundant branch, then reviewed + merged the combined result. The two lineages thus **converged on one branch** and squash-merged. (So commits `0d6da75`/`f5260ef`/`93f2275` are mine; `92e8141`/`a6aefdb`/`305d6d3`/`f4894ea`/`e96529d` are the other session's; both are now one squashed commit on main.)

1. **Re-survey after session-limit resume.** Found the shared working tree had advanced beyond my iteration-3 partial (another session repaired it + added `prev_col_map`, fixed pathfinding, removed `spawned_ids`). Confirmed via `git status`/`git log` it was a *live concurrent editor* (a file changed between my `grep` and my `Read`, with only a commit in between).
2. **Surfaced concurrency to the user** (AskUserQuestion). User: "다른세션…일단 작업 후 나중에 병합" (proceed, merge later).
3. **Isolated my work in a git worktree.** Created `chore/hardening-opus-2026-06-16` at `a6aefdb` in `../skeleton-engine-opus` to avoid clobbering the shared tree; did WU14 (serde_json→dev-deps, MSRV 1.95, wasm clippy, web-sys `ErrorEvent`) + WU12 safe subset (Focused→release_all, panic-policy doc) + clippy/doc-link cleanups there; ran the full gate green on that branch.
4. **Discovered the other session had finished everything on the shared branch** (`305d6d3` "batch 4 WU12+WU14", then `f4894ea` "release v9.0.0" with CHANGELOG/REFERENCE). My opus branch became redundant → **cleaned it up** (`git worktree remove` + `git branch -D`).
5. **User asked: review the full stack, reconcile duplication, PR, test, merge.**
6. **Independent code review** of `git diff main...HEAD` (74 files) via a Sonnet read-only reviewer focused on the high-risk areas. Result: 1 HIGH regression + 1 HIGH→doc + 1 MED→doc + 2 LOW; everything else (network close-flag/Drop, gpu-particle slots, `stepped_this_iteration`, inspector TypeId, audio fades, blob_47 correctness, breaking-change completeness) verified **sound**.
7. **Triaged + fixed** (commit `fd83587`): the Tilemap regression (real) + 3 doc clarifications; left #5 (documented footgun) as-is. Re-ran full gate → green.
8. **Created PR #60** (`gh pr create --base main`), pushed the branch.
9. **Watched CI to completion** (`gh pr checks 60 --watch`) → 4/4 pass.
10. **Squash-merged** (`gh pr merge 60 --squash --delete-branch`) → `7e48794`; synced local main; deleted local feature branch; updated the state memory.

## Key Decisions

- **Squash merge (not merge-commit / rebase).** Collapses the interleaved dual-session commits (incl. `e96529d`) into one clean `v9.0.0` commit — directly resolves the seq-1 duplication concern. The PR body recommends it.
- **Tilemap regression fix = `bump_generation()` escape hatch + doc + fix the example, NOT making `tiles` private.** `tiles`/`tile_size`/`origin` are widely **read** externally (pathfinding, editor, tile_collider) and `dig_quest` reassigns `tiles` wholesale — making them private would break reads and is a bigger break. Additive `bump_generation()` + a field doc + fixing `dig_quest`'s reset path preserves the perf win and the public surface. (`set_tile` still bumps automatically.)
- **3 review findings downgraded to doc-only:** blob_47 atlas migration note (the new masks are mathematically correct; the old were broken→tile-0, so it's an acceptable 9.0.0 break w/ a migration note); WASM `Drop` not emitting `Disconnected` (documented asymmetry — pushing it post-drop is pointless since nothing drains after the resource is gone); text cache `caret_byte` omission (correct — scroll recomputed from the buffer; added a comment).
- **Discarded my parallel opus branch rather than merge it.** Once the shared branch had the complete work, my branch added no net value; keeping two lineages would only complicate the merge.
- **Reviewed the canonical `main`-lineage branch, not my redundant one** — the review/merge target is what ships.

## Evidence & Data

### Final state
| Item | Value |
|---|---|
| `main` HEAD | `7e48794` feat(engine)!: hardening v9.0.0 — 80-finding analysis (WU1–14) (#60) |
| Version | 9.0.0 (Cargo.toml + CHANGELOG + CLAUDE.md header v1.6.33) |
| PR | #60 (squash-merged, branch deleted) |
| Merge diff | 76 files, +4890 / −437 |
| Lib tests | 698 pass / 0 fail (hardening start 603 → +95) + integration smoke (pathfinding/timeline/behavior/save) |

### CI (PR #60) — all green
| Check | Result |
|---|---|
| Test (native) | ✅ 4m19s |
| Build (WASM) | ✅ 1m55s |
| Package dry-run | ✅ 1m13s |
| Rustdoc | ✅ 49s |

### Code-review findings (commit `fd83587`)
| # | Sev | Finding | Resolution |
|---|---|---|---|
| 1 | HIGH | Tilemap `generation` fast-path skips rebuild on **direct field mutation** (`tm.tiles = …`, `tiles[r][c]=…`, `tile_size`/`origin`) → stale render. Hits `dig_quest` reset. | Added `Tilemap::bump_generation()` + field docs; `dig_quest.rs` reset now calls it. |
| 2 | HIGH→doc | blob_47 atlas tile-index mapping changed; users who adapted to the old (buggy) indices break. | CHANGELOG migration note (regenerate/reorder atlas to canonical Blob-47). |
| 3 | MED→doc | WASM `Drop` nulls `on_close` before `close()` → no `NetworkEvent::Disconnected` (native emits it). | Documented the asymmetry on the WASM `Drop` impl. |
| 4 | LOW | Text cache key omits `caret_byte` (correct, but opaque). | Added a clarifying comment on `is_single_line`. |
| 5 | LOW | `update_position` cancels an active `fade_volume` on the same channel. | Left as documented behavior (separate channels for positional vs fade). |

### Verified SOUND by the review (do NOT re-investigate)
Network native close-flag race (`Arc<AtomicBool>` + 5ms read-timeout, Acquire/Release — correct); `Drop` double-Disconnected (exactly-once); gpu-particle `frame_cursor` (local, reset per frame, disjoint slots); `stepped_this_iteration` double-step guard (reset in `about_to_wait`, no-op on 2nd call); inspector TypeId write-back (`comp_fields_entity` mid-frame guard correct); `RemoteEntities::get_or_spawn` is_alive re-spawn; audio fade guards (`to_stop` deferred-removal + `fades.contains_key`); WASM `buffered_amount` guard (single-threaded, no race); **blob_47 new 47-mask table independently re-derived correct**; `serde_json`→dev-dep (no `src/` usage); breaking-change set complete (no accidental extra public-API break beyond the documented 5 + the noted `RenderTarget::clear_color` add).

## Code Analysis (this session's fix)

- **`Tilemap::bump_generation(&mut self)`** (`src/tilemap.rs`, after `set_tile`): `self.generation = self.generation.wrapping_add(1)`. `set_tile` bumps automatically; this is the escape hatch for direct `pub tiles`/`tile_size`/`origin` mutation. The reactive `TilemapSystem` fast-path is `if tm.generation == cached_generation && dims == cached_dims { continue }` — so without a bump the change is never re-rendered.
- **`dig_quest.rs` reset** (~line 152): `tm.tiles = initial_tiles(); tm.bump_generation();`.
- Field doc added to `Tilemap::tiles` warning that direct mutation needs `bump_generation()`.

## Reusable Engineering Gotchas (this session)

- **Detect a concurrent editor by content-changing-under-you:** a file differed between a `grep` and a `Read` seconds apart, with only a `git commit` in between → proof another process is live-editing the shared tree. `git status` showing files modified *after* your own checkpoint commit is the confirmation.
- **For "work in parallel, merge later" on a shared working tree, use a `git worktree` on a separate branch** — branch-switching alone does NOT isolate the working directory (both sessions share one dir); a worktree gives a separate dir + branch. Cost: a fresh `target/` (full recompile) and being disciplined with absolute paths. Clean up with `git worktree remove` + `git branch -D` if it turns out redundant.
- **Squash-merge is the clean fix for interleaved dual-session / checkpoint-noisy history** — collapses N messy commits into one, so a "foreign commit in my history" concern simply disappears on `main`.
- **`gh pr checks <n> --watch --interval 30`** blocks until all checks finish (exit 0 = all pass) — the clean "verify tests then merge" primitive (foreground `sleep` is blocked, but `--watch` is real work and allowed). Native test job ≈ 4–5 min here; WASM/doc/package faster.
- **The safety classifier can be transiently unavailable** ("claude-opus-4-8 …temporarily unavailable, auto mode cannot determine safety") — it blocks *mutating* tools (Write/Edit/Bash) but read-only ops still work; just retry the write after a moment.
- **`git worktree` shares the object store**, so a branch created in a worktree is visible/mergeable from the main checkout; deleting the worktree does NOT delete the branch (and vice-versa) — clean up both.
- (Carried, still true) per-iteration `test --lib`+`clippy --lib` misses `fmt`/`clippy --all-targets`/doc/wasm/example breakage — run the full Gate6 before "done". rust-analyzer phantoms clear on `cargo check`. Subagents reliable with precise prompts + explicit `model: sonnet`.

## Files Changed (this session)

- **`fd83587` (review fixes, in `main` via squash):** `src/tilemap.rs` (`bump_generation` + field doc), `examples/games/dig_quest/dig_quest.rs` (reset bump), `docs/CHANGELOG.md` (blob_47 migration note), `src/network.rs` (WASM Drop doc), `src/renderer/text.rs` (cache-key comment).
- **Earlier WU14/WU12 work on the now-discarded `chore/hardening-opus-2026-06-16` branch** (NOT in main — the other session's equivalents shipped instead): serde_json→dev-deps, MSRV 1.95, wasm clippy, web-sys ErrorEvent, Focused→release_all, panic-policy doc, clippy/doc-link fixes. Preserved in reflog (`7130366`) if ever needed.
- **Memory:** `engine-current-state.md` (status → MERGED), `MEMORY.md` index unchanged.

## User Feedback & Preferences (REQUIRED — never omit)

- **"다른세션…일단 작업 후 나중에 병합"** — parallel session running; proceed in isolation, merge later. (→ worktree isolation, then squash-merge.)
- **"커밋 중복 확인 요청 … 전체 커밋 코드 리뷰 진행하고, 통합해서 pr 생성 하고 테스트 확인 후 머진 진행"** — review all commits, resolve the `e96529d` duplication, PR, verify tests, merge. (All done.)
- **v9.0.0 (breaking) — user asked me to confirm agreement; I endorsed it.**
- **This `/handoff` ask: "허고 푸쉬"** — write the handoff AND push it.
- **Standing:** Korean prose to the user; English code/docs/handoff. Subagents on Sonnet with explicit `model:`. Push/merge is normally the user's call (explicitly authorized here).

## Where We're Going

The batch is shipped. Nothing is blocking. Optional follow-ups (all noted in CHANGELOG / seq 1):

1. **Tag the release** `v9.0.0` if the project resumes tagging (tags lapsed after v4.3.0 — do NOT tag unprompted).
2. **Full `HotReloadable` trait** — shipped as a `macro_rules!` dedup; the full trait was deferred. Could be a v9.1.0 additive item.
3. **REFERENCE.html full refresh** — its top blockquote is stale ("v5.0.0 기준"); only a v9.0.0 section was appended.
4. **Line-by-line 80-finding audit vs Appendix A** — the review confirmed coverage sound by area, but a 1:1 tick against every audit # would confirm "all 80" for marketing/claims (the analysis itself predicted ~80% strictly "addressed").
5. Unrelated editor-chain follow-ups (SM visual node-graph, timeline time-ruler) live on the `editor-tile-painting` chain, not here.

## Risks & Blockers

- **None blocking.** The dual-session reconciliation (seq 1's sole real blocker) is resolved by the squash merge. `main` is green on CI.
- Minor: the WASM `Drop` no-`Disconnected` asymmetry is documented, not fixed (untestable in CI; low impact). The `HotReloadable` trait + REFERENCE refresh are deferred, not regressions.

## Open Questions

- Tag `v9.0.0`? (default: no, tagging lapsed.)
- `HotReloadable` full trait now (v9.1.0) or leave the macro dedup?
- Worth a line-by-line Appendix-A audit to claim 100% of 80 findings?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3        # expect 7e48794 (v9.0.0, #60) → 42de46c
git status -s               # clean
grep -m1 '^version' Cargo.toml   # 9.0.0
# The batch is MERGED. No reconciliation pending. References:
#   docs/CHANGELOG.md (9.0.0 entry), docs/CODE_ANALYSIS_2026-06-16.md (Appendix A),
#   seq-1 handoff (full impl detail), CLAUDE.md (Gate6).
# Optional next: v9.0.0 tag (only if asked), HotReloadable trait (v9.1.0), REFERENCE.html refresh,
#   or a fresh feature per docs/VISION.md.
./scripts/verify.sh         # confirm main still green if resuming engine work
```
