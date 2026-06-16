# v10 breaking architecture pass — 8 PRs shipped (v9.6.0 → v10.0.0 in progress)

**Date:** 2026-06-16
**Status:** PAUSED — loop ended by user ("루프 종료") after item G; v10 arc 6/8 items done.
**Bead(s):** none (beads unavailable in this repo)
**Epic:** skeleton-engine — engine hardening / architecture
**Chain:** `engine-hardening` seq `9`
**Parent:** `HANDOFF_engine-hardening_session-summary-v9.5.1_2026-06-16.md` (seq 8)

> This session: drained the last additive items (v9.6.0/v9.6.1), then scoped + executed most of the
> **v10 breaking architecture pass**. Read the plan `plans/V10_BREAKING_PASS_PLAN_2026-06-16.md`
> alongside this — it has the full per-item breaking-surface/effort/risk analysis.

## What shipped this session (8 PRs, all merged + CI-green)

Starting state: `main` @ `1aec558` (v9.5.1). Ending state: **`main` @ `5d695ab`, v10.0.0 (in progress), 729 lib tests, clean + CI-green.**

| PR | Ver | What |
|---|---|---|
| #71 | 9.6.0 | **`RenderPlugin`** trait + `App::add_render_plugin` — fork-friendly custom render-pass hook (additive). `FrameContext` gained `pub format`. Example `render_plugin` (vignette). |
| #72 | 9.6.1 | `Wander::direction_fn`/`with_direction_fn` (swap wander RNG w/o forking); fixed 2 long-broken doctests (`register_serde_component`/`register_editable_component`); **added `cargo test --doc` to CI + verify.sh** (doctests were silently rotting). |
| #73 | 10.0.0 | **v10 PR1 (item I):** split `tilemap.rs` (1620L) → `tilemap/{mod,autotile,system}.rs`; removed unused `cell_display_uv`; **fixed `verify.sh` exec bit** (was `100644` in git). |
| #74 | 10.0.0 | **v10 PR2 (items A/B/C):** `RenderTarget`/`LightingRenderer` wgpu fields → `pub(crate)` + escape-hatch accessors; `RenderTarget::new` builds its own bind-group layout (drops borrowed `texture_layout`; removed `SpriteRenderer::texture_layout()`). |
| #75 | 10.0.0 | **v10 PR3 (item J, breaking):** `UiSystem`/`SteeringSystem` unit structs → reused scratch fields; all `add_system(X)` → `add_system(X::default())` (22 in-repo sites migrated). Killed 11 per-frame `Vec<Entity>` allocs (closes #76). |
| #76 | 10.0.0 | **v10 PR4 (item K):** `ScriptAsset`/Rhai decoupled from `asset.rs` → `scripting/{asset,loading}.rs` + new `ScriptRegistry` World resource (hot-reload via `HotReloadable`). `asset.rs` now rhai-free. `engine::ScriptAsset` re-exported at crate root. |
| #77 | 10.0.0 | **v10 PR5 (item E):** extracted 14 renderer/texture/egui fields from the `App` god-struct into internal `RenderState` (`src/app/render_state.rs`). `gpu`+`world` stay on `App`. |
| #78 | 10.0.0 | **v10 item G:** split `schedule::update()` (386L) → `compute_viewport()`/`run_systems()`/`post_systems()` + egui begin/end → `egui_pass.rs`. Order byte-equivalent (test-guarded). |

## Where We're Going (remaining v10 work — needs a decision)

**`main` is green + clean at v10.0.0 (in progress).** Two planned items NOT done:

1. **Item F — split `render()` (839L): SKIPPED (supervisory call).** Rationale: its primary stated goal
   ("a forker can't insert a render pass without editing `render()`") is **already met by the
   `RenderPlugin` hook shipped in #71**. What's left is internal readability of a *working* function —
   and F is the ONE item CI cannot verify (no GPU test; submit-order/texture-routing/cfg mistakes
   compile fine but render wrong). High render-path risk for low marginal value. The user re-fired the
   loop after I flagged this + recommended skipping F, which I took as acceptance. **Reconsider only
   with a real visual-playtest plan.**
2. **Item H — `SpriteRenderer` → `MaterialRenderer` + `TextureCache`: NOT done (loop ended first).**
   Internal cohesion (the 5-concern sprite struct). Medium risk: it's a contained ownership extraction
   but touches the core sprite render path (same no-GPU-CI-test blind spot as F, though a layout
   mis-wire usually *panics on startup* → catchable by just running a sprite example). If pursued:
   delegate the split (agent C's analysis in the plan has the field-by-concern breakdown) → **run a
   sprite-example smoke test (run + screenshot + eyeball) before merging**, since CI won't catch a
   visual regression. Effort L (~180 LOC).

**Also pending:** v10.0.0 is **in progress** on `main` (one CHANGELOG section accumulating). When the
arc is declared complete, drop the "(in progress)" from the `## 10.0.0` heading. Tagging is on-request
only (lapsed after v4.3.0).

## Key Decisions (this session)

- **v10 = one accumulating release.** First v10 PR bumped Cargo.toml 9.6.1→10.0.0; subsequent PRs keep
  10.0.0 and append to the one `## 10.0.0` CHANGELOG section. Each PR still bumps the CLAUDE.md header
  doc-version (v1.6.40 → v1.6.48).
- **The v10 breaking surface turned out tiny.** A 4-way read-only analysis found **no item has any
  external/forker call site in `examples/`**; most "architectural" value (E/F/G/H/I) is internal-only.
  The genuine public breaks: J (unit-struct construction), A (RenderTarget fields), K (ScriptAsset
  module path, bridged), `cell_display_uv` removal. All trivial migrations.
- **Skipped F** (above) — stopped at the high-risk/low-value line rather than burn effort + take a
  visual-regression risk CI can't guard.
- **`gpu` stays on `App` in RenderState (E).** Deliberate: keeping it disjoint from the grouped
  renderers avoids the destructuring churn that grouping gpu+renderers would force (agent B flagged it).

## Reusable Gotchas (important for next session)

- **Stale mid-edit rustc diagnostics fired 6×** this session (E0061/E0308/E0423/E0609 in the exact files
  an agent was editing) while the agent's final state was clean. **EVERY time, `cargo check
  --all-targets` was green** — the diagnostics were snapshots from mid-edit, resolved before the agent
  finished. **Always independently `cargo check`/Gate6 after agent work; trust neither the agent's
  "green" claim alone NOR the stale diagnostics.** (Also: agents have twice self-reported "cargo fmt
  PASS" while `cargo fmt --check` actually failed — always run `--check` explicitly in your re-verify.)
- **Agent worktree surprise (PR5):** a long-running general-purpose agent (no `isolation` set) did its
  work in a `.claude/worktrees/agent-*` worktree, NOT the main tree — my main tree was clean and the
  changes "missing." Fix: `git -C <worktree> add -A && commit`, then `git checkout -b <branch> &&
  git cherry-pick <sha>` into the main tree, then `git worktree remove --force` + delete the branch.
  Check `git worktree list` + `git status` if an agent's changes seem absent.
- **`verify.sh` was committed `100644` (non-executable)** in git all along (`core.fileMode=false` hid
  it locally). `./scripts/verify.sh` fails "permission denied" on a fresh clone. Fixed in #73 via
  `git update-index --chmod=+x` (a plain `chmod`+`git add` does NOT record the bit when
  `core.fileMode=false`). Run Gate6 with `bash scripts/verify.sh` if the bit is ever lost again.
- **CI disk-space flake:** PR3's "Test (native)" failed with `No space left on device` on the runner
  (infra, not code — the test steps never ran; same-commit WASM/Rustdoc/Package passed). `gh run rerun
  <id> --failed` fixed it. Inspect `gh run view --job <id>` annotations before assuming a code failure.
- **`gh pr checks <n> --watch | tail` masks the real exit code** (you get tail's 0). Read the actual
  per-check pass/fail lines, or capture `${PIPESTATUS[0]}`. For Gate6, `bash verify.sh > log; echo $?`
  then `grep 'all checks passed' log`.
- **rust-analyzer phantoms persist** (ColliderHandle E0308, inactive-cfg, unlinked-file) — routine.
- **Loop mechanics:** opus scopes each item inline → ONE background Sonnet impl agent (explicit
  `model: sonnet`) → on completion opus reviews diff + re-runs full Gate6 + bumps version/CHANGELOG/
  CLAUDE + branch/commit/PR + `gh pr checks --watch` + squash-merge. Read-only analysis fanned out to
  4 concurrent agents.

## Risks & Blockers

- **None.** main green + clean at `5d695ab` (v10.0.0 in progress).

## Open Questions

- Finish v10 with **item H** (SpriteRenderer split, + sprite smoke test)? Or declare v10 done at item G?
- **Item F** (render split) — leave skipped (RenderPlugin covers the fork need), or pursue with a full
  render-mode playtest?
- Drop "(in progress)" from the 10.0.0 CHANGELOG heading + (optionally) tag v10.0.0 when the arc closes?

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -8        # 5d695ab(#78 G) ... 113f815(#73 PR1) ... 4a4d488(#72 v9.6.1)
grep -m1 '^version' Cargo.toml   # 10.0.0
git status -s               # clean
./scripts/verify.sh         # main green (729 lib tests) — exec bit now fixed

# Read first
#   plans/V10_BREAKING_PASS_PLAN_2026-06-16.md   (per-item analysis; F/H detail)
#   THIS handoff (seq 9)
#   docs/CHANGELOG.md  → the accumulating ## 10.0.0 section

# Next action — needs a DECISION:
#   (a) Item H (SpriteRenderer→MaterialRenderer+TextureCache) — delegate split, then a sprite-example
#       smoke test before merge (CI has no GPU test). Then close v10.
#   (b) Reconsider item F (render split) only with a real visual playtest across render modes.
#   (c) Declare v10 done at item G — drop "(in progress)" from the 10.0.0 CHANGELOG heading.
#   Standing: no breaking changes beyond the v10 plan without sign-off; Korean prose / English
#   artifacts; Sonnet subagents w/ explicit model:; re-verify Gate6 independently after agent work;
#   merge authority was granted for the loop (re-confirm in a new session).
```

## Session Status
PAUSED — loop ended by user after item G. v10 arc 6/8 done (#73–#78), F skipped, H remaining. main green + clean at v10.0.0 (in progress).
