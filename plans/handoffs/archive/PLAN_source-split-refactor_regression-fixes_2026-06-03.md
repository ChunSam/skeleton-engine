# Source-split follow-ups: local verification bar, finish the deep review, optional further splits

**Date:** 2026-06-03
**Status:** PLANNED
**Bead(s):** none
**Epic:** source file split / maintainability refactor
**Chain:** `source-split-refactor` seq `2`
**Context:** See `HANDOFF_source-split-refactor_regression-fixes_2026-06-03.md` for full session data, the regression fixes, CI evidence, and the public-API diff. Parent: `HANDOFF_source-split-refactor_source-file-split_2026-06-03.md`.

---

## Problem Statement

The v2 source split (seq 1) introduced regressions — a broken WASM build (`E0432`/`E0609`) and new clippy warnings — that seq 2 found, fixed, committed (`61c09f1`), and pushed. **CI is now green on `main`.** The root cause was process, not infra: CI already runs `cargo clippy --all-targets -- -D warnings` and `cargo build --target wasm32-unknown-unknown`, but seq 1 declared "done" on a narrow LOCAL bar (`cargo fmt --check` + `cargo test --lib`) while the work sat unpushed, so CI never ran. Two remaining gaps: (1) the local "done" bar doesn't mirror CI, so the same trap can recur; (2) two large split files — `src/app/editor/ui.rs` (963 lines) and `src/ecs/world.rs` (909 lines) — were never read end-to-end during review (low risk: green build/clippy/tests/CI + byte-identical public API, but untested code paths could still hide a behavior change).

## Key Findings

- CI (`.github/workflows/ci.yml`) ALREADY enforces clippy `-D warnings`, wasm build, rustdoc `-D warnings`, and `cargo package --dry-run`. → **drives Phase 1** (don't re-add to CI; close the *local* gap).
- seq-1 regressions were invisible to `cargo test --lib` (wasm-only + clippy-only) → **drives Phase 1** (local bar must include clippy --all-targets -D warnings + wasm build).
- Public API is byte-identical to pre-split HEAD except deliberate sprite changes (−2 dead wrappers, +`FrameContext`); rust-survivors unaffected. See handoff Evidence. → bounds Phase 2/3 (no API churn expected).
- `editor/ui.rs` (963) and `ecs/world.rs` (909) were NOT read line-by-line this session. → **drives Phase 2**.
- `editor/ui.rs` is near the 1000-line ceiling; `audio.rs` is 771 lines (parent-deferred). → **drives Phase 3** (optional).
- The split was a clean *move* (no duplicate symbols, no logic edits found in the files that WERE read). → Phase 2 is a confirmation pass, not a rewrite.

## Anti-Goals (What NOT To Do)

- **Do NOT re-add wasm build / clippy to CI** — already present and green. Phase 1 is about the local bar + docs only.
- **Do NOT change public API** in any phase. The split's whole premise (and parent's hard constraint) is API/behavior preservation. If a review finds a real API drift, STOP and ask.
- **Do NOT restore the deleted `render_ui_*_from_slice` wrappers** or revert `FrameContext` — both were deliberate, approved, and CI-green.
- **Do NOT touch `examples/`** — out of scope for this chain; the wasm example failures are expected (native-only deps).
- **Do NOT rewrite `editor/ui.rs`/`ecs/world.rs` while "reviewing"** — Phase 2 reads and confirms; only fix concrete defects found.

## Plan

### Phase 1: Document the local "done" bar (CI-equivalent)

**Goal:** Make future local sessions run the same checks CI does, so a refactor can't be declared "done" on a narrow local check again.

**Why this approach:** The regression root cause was a local/CI mismatch, not missing CI. The cheapest durable fix is to write the bar down where agents read it (`CLAUDE.md`, `AGENTS.md`) and optionally provide a one-command runner.

- Add a short "Verification (run before declaring done)" block to `CLAUDE.md` and/or `AGENTS.md` listing exactly: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --target wasm32-unknown-unknown`, `cargo test --all-targets`.
- Note the gotcha: wasm `--all-targets` fails on native-only examples (`platformer_game`/`mp_server`/`gpu_particles`); use plain `cargo build --target wasm32` (lib+bins) or `--lib` for the wasm gate.
- Keep it ≤200 lines per the doc-length rule in `CLAUDE.md`; if `CLAUDE.md` would exceed 200, put the detail in `docs/PATTERNS.md` and leave a one-line pointer.
- (Optional) Add a `scripts/verify.sh` or `Makefile`/`justfile` `verify` target running the 4 commands in order, and reference it from the docs.

**Files:** `CLAUDE.md`, `AGENTS.md` (doc edits); optional new `scripts/verify.sh`.
**Validates with:** the documented commands all pass on current `main` (they do — CI is green). If adding a script, run it end-to-end and confirm exit 0.
**Rollback:** revert the doc/script edits; no code impact.

### Phase 2: Finish the deep review of the two large unread files

**Goal:** Close the only review gap — confirm `editor/ui.rs` and `ecs/world.rs` are faithful moves with no split-induced behavior change.

**Why this approach:** Everything else was read end-to-end this session and verified; these two (the largest split outputs) were skipped. Tests/CI pass, so any defect is on an untested path — exactly what a read catches that CI doesn't.

- Read `src/app/editor/ui.rs` (963) end-to-end. Confirm: cfg gates intact (it is native-only editor UI; `update_editor_ui` is called every frame and internally gated), no dropped `#[allow]`, no accidental `pub` widening, gizmo/inspector/scene-save logic unchanged vs `git show HEAD:src/app.rs` equivalents.
- Read `src/ecs/world.rs` (909) end-to-end. Parent split it by moving tests only, so production code should be unchanged vs HEAD — verify with `git show HEAD:src/ecs/world.rs` diff that only the `#[cfg(test)] mod tests` block left and `mod tests;` was added.
- For each file, spot-check the public symbol set against the handoff's "identical" finding (already confirmed at module-group level; this is a fn-body sanity read).
- If a concrete defect is found (dropped gate, behavior change, stale comment/import from the move), fix it surgically and re-run the Phase 1 verification set.
- If clean, record "reviewed, no findings" — the review is then complete for the whole split.

**Files:** read-only unless a defect is found; then the specific file(s).
**Validates with:** `git diff HEAD~? -- src/ecs/world.rs` shows only test-move; `cargo clippy --all-targets -- -D warnings` + `cargo test --all-targets` stay green after any fix.
**Rollback:** revert any fix; the files are already CI-green as-is.

### Phase 3 (optional): Carryover splits + doc polish

**Goal:** Address parent's deferred maintainability items, only if Phase 2 didn't surface higher-priority work.

**Why this approach:** These are pure maintainability nice-to-haves the parent explicitly deferred; do them only when nothing more important is pending.

- Consider splitting `src/app/editor/ui.rs` (963, near 1000) into cohesive sub-passes (inspector panel / gizmo / scene save-load), mirroring the existing private-submodule pattern. Preserve `update_editor_ui`'s call signature and cfg gating.
- Consider splitting `src/audio.rs` (771) by responsibility (playback / positional / bus-mixer / fades) — lowest priority.
- Optionally add the new module directories to `CLAUDE.md`'s module map (parent updated only `AGENTS.md` + `ARCHITECTURE.html`).
- Each split is a mechanical move under the same constraints: no public API change, run the full Phase 1 verification after.

**Files:** `src/app/editor/ui.rs` (+ new `src/app/editor/ui/*.rs`), `src/audio.rs` (+ new `src/audio/*.rs`), `CLAUDE.md`.
**Validates with:** Phase 1 verification set green; public symbol diff vs HEAD unchanged.
**Rollback:** revert the split commit(s); these are independent and isolated.

## Dependencies & Order

- Phase 1 is independent and should go first (it defines the verification used to validate Phases 2–3).
- Phase 2 is independent of Phase 1 but should use Phase 1's command set.
- Phase 3 depends on Phase 2 (don't start optional splits until the review confirms the current split is clean). Phase 3's two splits are independent of each other and parallelizable in worktrees.

## Risks & Mitigations

- **Phase 2 finds a real behavior change** (low, given green tests/CI). Mitigation: fix surgically, add a regression test, re-run full verification; if it's an API change, STOP and ask the user.
- **Phase 3 split reintroduces the seq-1 class of bug** (medium if done carelessly). Mitigation: Phase 1's documented bar — run clippy `-D warnings` + wasm build locally before commit, then push so CI confirms.
- **`editor/ui.rs` split changes egui frame timing/behavior** (low). Mitigation: keep it a pure move; verify the editor still drives via `update_editor_ui` unchanged; manual smoke per `docs/VISION.md` if behavior is in doubt.

## Success Criteria

- **Minimum viable:** Phase 1 done — local verification bar documented; `main` stays CI-green.
- **Full:** Phases 1–2 done — both large files read end-to-end with "no findings" (or defects fixed), making the entire v2 split reviewed; verification bar documented.
- All phases: `cargo clippy --all-targets -- -D warnings`, `cargo build --target wasm32-unknown-unknown`, `cargo test --all-targets` green; public symbol diff vs pre-split HEAD unchanged (except the already-approved sprite delta).

## Quick Start

```bash
# Restore full context
cat plans/handoffs/PLAN_source-split-refactor_regression-fixes_2026-06-03.md
cat plans/handoffs/HANDOFF_source-split-refactor_regression-fixes_2026-06-03.md

# Confirm green starting state
git log --oneline -3            # 61c09f1 == origin/main
gh run list --branch main --limit 3   # latest CI = success
cargo clippy --all-targets -- -D warnings && cargo build --target wasm32-unknown-unknown && cargo test --all-targets

# Key files
sed -n '1,40p' .github/workflows/ci.yml   # the bar Phase 1 mirrors
sed -n '1,60p' CLAUDE.md                   # where Phase 1 writes the local bar
sed -n '1,80p' src/app/editor/ui.rs        # Phase 2 target (963 lines)
sed -n '1,80p' src/ecs/world.rs            # Phase 2 target (909 lines)

# First concrete action (Phase 1)
# Add a "Verification (run before declaring done)" block to CLAUDE.md listing:
#   cargo fmt --check
#   cargo clippy --all-targets -- -D warnings
#   cargo build --target wasm32-unknown-unknown
#   cargo test --all-targets
```
