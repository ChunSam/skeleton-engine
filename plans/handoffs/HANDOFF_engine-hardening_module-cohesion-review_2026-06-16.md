# Module cohesion & coupling review — full src/ fork-friendliness audit (PR #66)

**Date:** 2026-06-16
**Status:** COMPLETED
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `4`
**Parent:** `HANDOFF_engine-hardening_post-v9-priority-loop_2026-06-16.md` (seq 3)
**Prior chain:** `..._v9.0.0-shipped_...` (1) > `..._v9.0.0-merged_...` (2) > `..._post-v9-priority-loop_...` (3) > this (4)

---

## Since Last Handoff

Seq 3 (post-v9-priority-loop) shipped 5 PRs (#61–65, v9.1→v9.3 + 2 docs) and ended with the
tractable autonomous backlog drained. The user re-ran the same `/loop`; when asked for direction
(all remaining items were marginal / need-a-human / breaking / need-direction), they chose:
**"시각검증은 시간이 걸리니 전체 코드 리뷰 + 모듈 응집도 위주 검사를 진행"** — a full code review
weighted to module cohesion. That is this seq. One deliverable PR (#66, docs-only), merged.

## Reference Documents

- `docs/MODULE_COHESION_REVIEW_2026-06-16.md` — **the deliverable** (the full review; read this).
- `docs/CODE_ANALYSIS_2026-06-16_COVERAGE.md` — the 80-finding coverage ledger (seq 3).
- `CLAUDE.md` — module map; `docs/VISION.md` — fork-friendly skeleton priorities (the review lens).

## The Goal

Produce a full-codebase architecture review centered on **module cohesion + coupling**, judged
through fork-friendliness (VISION priority #1 = clear module boundaries + extension points), so the
project has a prioritized roadmap of structural debt. Inspection only — not a refactor.

## Where We Are

- **`main` = `c6804e7` (PR #66 merged), v9.3.0, clean tree.** The review doc + (bundled — see Risks)
  the seq-3 handoff + the progress-doc banner all landed via #66's squash.
- **Method:** 6 parallel read-only Sonnet agents, one per cluster (App/lifecycle · editor ·
  renderer · ECS+serialization · physics+spatial · animation+audio+UI), each emitting severity-tagged
  `[cohesion|coupling|smell|fork-friction]` findings with `file:line` + recommendation, plus a
  "what's good" list. I synthesized into the review doc (themes → per-cluster → preserve → actions).
- **Headline verdict:** the engine is **well-decomposed, not a ball of mud.** Strong module-level
  cohesion, genuine rapier encapsulation, clean audio/animation layering, production-grade robustness.
- **6 cross-cutting themes** (full detail in the doc): (1) god-units (`App` ~30 fields, `render()`
  839 lines, `update()` 386, `SpriteRenderer` 5 concerns, `tilemap.rs` 5 concerns/1555 lines,
  `docked.rs` 2003 lines, `EditorState` 47 fields); (2) residual per-frame allocations the 80-finding
  audit missed; (3) backend types (wgpu/rapier/rhai) leaking into the public API; (4) extension-seam
  fork-friction (4 overlapping `register_*` calls, no render-pass hook, no inspector-panel
  registration, AssetServer hot-reload sidecars); (5) misplaced code; (6) layer-boundary blur.
- **Prioritized action list** in the doc: items 1–4 are **safe additive autonomous follow-ups**;
  items 5–7 are **breaking/architectural → next `v10` design pass with user sign-off.**
- **No code changed** (inspection only).

## What We Tried (Chronological)

1. **Direction check.** Seq-3 backlog was drained of clearly-tractable items; surfaced the fork
   (AskUserQuestion). User redirected to a cohesion-weighted full review (visual-verification work
   deferred because it needs time/a human).
2. **Fan-out.** Launched 6 background read-only Sonnet agents (explicit `model: sonnet`), one per
   cluster, with a shared cohesion/coupling/fork-friendliness rubric + each cluster's file list.
3. **Collected 6 reviews** (auto-woke per completion). Each returned a cluster cohesion assessment +
   5–15 findings + "what's good."
4. **Synthesized** into `docs/MODULE_COHESION_REVIEW_2026-06-16.md`: deduped into 6 cross-cutting
   themes, a per-cluster verdict table, a "preserve — do not refactor" list, and a 7-item
   prioritized action list with breaking-flags.
5. **Shipped** as PR #66 (docs-only), CI 4/4 green, squash-merged; reconciled a local/remote main
   divergence (see Risks).

## Key Decisions

- **Asked for direction rather than autonomously shipping marginal work.** Seq-3's leftover items
  were all marginal/need-human/breaking/need-direction; a 10-second user choice beat churning a
  micro-nit or guessing a feature. (User chose the cohesion review.)
- **Read-only fan-out, 6 clusters, parallel.** Mirrors the seq-3 audit: read-only ⇒ no write
  conflict ⇒ safe to parallelize; per-cluster depth beats one agent over the whole tree.
- **Deliverable = the review doc, NOT applied refactors.** The user asked for a "검사" (inspection).
  Most fixes are breaking/architectural — applying them blind would pre-empt a design decision.
  Synthesized a prioritized, breaking-flagged action list instead so the user can scope execution.
- **Themes over raw findings.** ~50 raw findings deduped into 6 cross-cutting themes — far more
  actionable than a flat list, and surfaces that (e.g.) the per-frame-allocation smell recurs across
  6 modules with one proven fix pattern (`PhysicsSystem` scratch fields).

## Evidence & Data

### Cluster review yields (severity-tagged findings per agent)
| Cluster | HIGH | MED | LOW | Headline issue |
|---|---|---|---|---|
| App + lifecycle | 3 | 5 | 3 | `App` god-struct; `update()`/`render()` god-functions |
| Editor | 2 | 5 | 4 | `docked.rs` 2003-line catch-all; `EditorState` 47 fields |
| Renderer | 2 | 5 | 4 | `SpriteRenderer` 5 concerns; wgpu types leak; no pass hook |
| ECS + serialization | 3 | 5 | 3 | `World` owns reflect registry; 4 `register_*` shapes; AssetServer sidecars |
| Physics + spatial | 2 | 6 | 4 | `tilemap.rs` 5 concerns; `SteeringSystem` 5 Vecs/frame |
| Animation + audio + UI | 3 | 5 | 2 | undocumented 3-system order; BlendTree↔SM conflict; UI per-frame Vecs |

### The prioritized action list (from the doc — what the next session can pick up)
1. **Additive perf pass (Theme 2, non-breaking)** — promote missed scratch buffers to fields: sprite HashSets + `atlas_entries`, UI widget Vecs, tilemap cached-grid clone + preamble, SM `evaluate()` string clone, collision candidates scratch, `body_factory` remove alloc. Behavior-identical + unit-testable. (`SteeringSystem` #76 needs a construction change → small breaking.)
2. **DRY consolidation (Theme 7)** — `compute_mask_raw` (correctness risk), shared `AssetLoadError`, `submit_egui_pass`, scene-graph/pointer-gating helpers.
3. **Module-home moves (Theme 5)** — `SerdeComponentRegistry`→`serde_registry.rs`, `ScriptAsset`→`scripting/`, tile-paint→`ui/tile_paint.rs`, `CameraUniform` dedup, extract SM/Timeline panels from `docked.rs`. Re-export → additive.
4. **Doc + small-API fork-friction (Theme 4 cheap subset)** — animation ordering doc, `JointHandle::raw()`, `register_editable_component` wasm note, `Wander::direction_for`.
5. **Encapsulation tightening (Theme 3)** — wgpu/rapier fields → `pub(crate)` + accessors. **Breaking** → v10.
6. **Architectural extraction (Theme 1)** — `RenderState` out of `App`, split `render()`/`update()`, `SpriteRenderer`→`MaterialRenderer`+`TextureCache`, split `tilemap.rs`. Large arc.
7. **New extension points (Theme 4 big subset)** — `RenderPlugin` hook, `register_inspector_panel`, unified AssetServer watchlist via `HotReloadable`, registration-API unification. Most advances fork-friendliness; some breaking.

## Files Changed

### Docs
- `docs/MODULE_COHESION_REVIEW_2026-06-16.md` — NEW, the deliverable (152 lines).
- (Bundled into #66's squash via the divergence below: `plans/handoffs/HANDOFF_...post-v9-priority-loop...md` + `plans/code-analysis-2026-06-16-progress.md` banner — both seq-3 artifacts.)

### Memory
- `engine-current-state.md` + `MEMORY.md` index — updated to v9.3.0 in seq 3 (still current; add the cohesion review as the latest artifact when next touched).

## User Feedback & Preferences (REQUIRED)

- **"시각검증 가능하려면 시간이 필요하니 전체 코드 리뷰 및 모듈 응집도 위주 검사 진행해줘"** — the explicit task for this seq: do the review now; visual-verification work (per-locale fonts, visual editors) waits for a session with time/display.
- **Re-ran the SAME `/loop` charter** ("opus 감독 / sonnet 실무 / 테스트 후 머지 / handoff / 보고") — wants autonomous batches each ending in merge + handoff + report. Merge authority continues for this loop.
- **Standing:** Korean prose to the user, English artifacts; subagents on Sonnet with explicit `model:`; never tag unprompted; rust-survivors dropped.

## Where We're Going

The review is the deliverable; **executing it is the user's call.** Recommended next, in order:
1. **Action-list items 1–4** (additive perf + DRY + module moves + doc fork-friction) — safe, testable, autonomous-friendly; could be one or a few PRs. **The obvious next autonomous batch.**
2. **Items 5–7** (encapsulation tightening, `App`/`render()` extraction, new extension points) — breaking/architectural → scope a dedicated `v10` design pass with the user.
3. Still-open from seq 3 (unchanged): per-locale fonts (#5, display session), SM/Timeline visual editors, v9.3 tag.

## Risks & Blockers

- **None blocking.** main green + clean at `c6804e7`.
- **Process gotcha (resolved, note for next time):** the seq-3 handoff was committed to **local** main but not pushed; the #66 docs branch was cut from local main, so #66's squash **bundled** the handoff + progress-banner with the cohesion review under #66's message. All content is on origin/main (verified) — nothing lost — but the commit attribution is muddied. **Fix for next time:** push `session: …handoff` commits straight to main (the established pattern), or branch docs PRs from `origin/main`, not unpushed local main.

## Open Questions

- Greenlight action-list items 1–4 as the next autonomous batch? (Default rec: yes, item 1 first.)
- Items 5–7: schedule a `v10` architecture design pass, or leave as documented roadmap?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3        # expect c6804e7 (#66) → 6033a7f (#65) → 1545c95 (#64)
grep -m1 '^version' Cargo.toml   # 9.3.0
git status -s               # clean

# Read first
#   docs/MODULE_COHESION_REVIEW_2026-06-16.md   (the review — themes + prioritized action list)
#   this handoff (seq 4) + its parent (seq 3, the v9.1-9.3 priority loop)

# Verify
./scripts/verify.sh         # full Gate6 — confirm main green

# Next action — pick ONE:
#   (a) Action-list item 1: additive perf pass (promote missed scratch buffers to fields,
#       mirror the PhysicsSystem pattern; behavior-identical + unit-tested). Safest, fully verifiable.
#   (b) Items 2-4 (DRY / module moves / doc fork-friction) — additive.
#   (c) Scope items 5-7 (breaking/architectural) into a v10 design pass with the user.
#   For any code work: implement → Gate6 → unit-test through real handler → PR → merge
#   (re-confirm merge authority in a NEW session).
```

## Session Closed
**Closed at:** 2026-06-16
**Commit:** the `session: module-cohesion-review handoff [engine-hardening]` commit on `main`
**Session status:** Handed off — cohesion review delivered (PR #66); action-list items 1–4 are the next autonomous batch
