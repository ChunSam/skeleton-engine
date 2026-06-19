# Engine-hardening refactor sweep (P3–P5) + 64-tile hex blob atlas (v0.43.0 → v0.43.5)

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — 6 PRs (#146–#151) merged + green; `main` @ `79524ef`, package **v0.43.5**, tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `44`
**Parent:** `HANDOFF_engine-hardening_focus-ring-style_2026-06-19.md` (seq 43)

> This session shipped one feature (the **64-tile hex blob autotile atlas**, v0.43.0) plus a
> **5-step behavior-preserving module-split sweep** (P3–P5 of a code-review-driven refactor plan,
> v0.43.1–v0.43.5) that broke up the five biggest/most-mixed-responsibility `src/` files. Public
> API and runtime behavior unchanged throughout; **883 lib tests held at every step**.

---

## Since Last Handoff (vs seq-43)

Seq-43 left the backlog at: 64-tile hex atlas, per-OS gamepad input, crates.io publish. The user
picked the **hex atlas** first, then handed me an **external code-review document** (a structural
refactor plan for `src/`) and asked me to (1) fact-check it, then (2) execute it via a `/goal`.

## The Goal (user `/goal`, still the active directive until this handoff)

> "p3 부터 권장 실행 순서대로 진행. 모든 계획 진행하고 테스트 후 머지까지 한 뒤에 핸드오프 하고 완료 보고."

i.e. run the refactor plan in my recommended risk order (P3 → P4 → P1 → P2 → P5), each split
verified + shipped + merged, then write this handoff and report done.

## Where We Are

- `main` @ **`79524ef`** (PR #151, v0.43.5), tree clean, all CI green.
- **Six PRs merged this session:**
  | PR | what | version |
  |---|---|---|
  | #146 | **feat:** 64-tile hex blob autotile atlas + `gen_hex_autotile_sheet` + `hex_blob_autotile{,_flat}` examples (also folds in a `docs/PATTERNS.md` recipe) | v0.43.0 |
  | #147 | **P3 refactor:** extract bottom-of-file `mod tests` → `tests.rs` siblings (5 files) | v0.43.1 |
  | #148 | **P4 refactor:** `network.rs` (1372) → `src/network/` (7 submodules) | v0.43.2 |
  | #149 | **P1 refactor:** `app/render.rs` (1200) → `src/app/render/` (6 submodules) | v0.43.3 |
  | #150 | **P2 refactor:** `app/editor.rs` (1509) → `src/app/editor/` (8 submodules) | v0.43.4 |
  | #151 | **P5 refactor:** `renderer/text.rs` (1102) → `text/` (5) + decompose `sprite.rs` `render()` | v0.43.5 |
- **Root-file line counts after the sweep:** `network.rs` 1372→**99**, `app/render/mod.rs` **18**
  (was render.rs 1200), `app/editor.rs` 1509→**43**, `renderer/text.rs` 1102→**17**, `sprite.rs`
  825→**355**.
- **Public API unchanged** (`engine::*` re-exports + `engine::network::*` consts + `engine::renderer::*`
  all verified) and **every `#[cfg(target_arch = "wasm32")]` boundary preserved** (the wasm build is
  the gate that proves this — see Gotchas).

## What We Did (chronological)

1. **64-tile hex blob atlas (feature, v0.43.0, #146).** The `hex_6`/`hex_6_flat` autotile constructors
   existed since v0.39.0 but had no 64-tile atlas (examples used a 2-tile interior/edge rule). Wrote
   `examples/gen_hex_autotile_sheet.rs` — a deterministic procedural generator (the hex sibling of
   `gen_autotile_sheet`) that draws a regular hexagon per cell with exposed-edge outlines, **tile index
   == the 6-bit Hex6/Hex6Flat neighbor mask**. Produced `examples/assets/hex_autotile{,_flat}.png`
   (pointy 512×592, flat 592×512). Added full-blob examples `hex_blob_autotile`/`hex_blob_autotile_flat`.
   **Verified geometry tile-by-tile (masks 0/1/2/4/8/16/32/63) AND in-window screenshots both
   orientations** (rim + carved holes outlined correctly). Also committed a `docs/PATTERNS.md` recipe
   ("Make a hardcoded constant configurable (default-preserving resource)", the `FocusRingStyle`
   idiom) on the same branch.
2. **Reviewed the refactor plan before executing.** Fact-checked it against the codebase — line counts
   all exact; symbols all present. Found corrections (see Key Decisions). Recommended risk order
   P3→P4→P1→P2→P5.
3. **Executed the sweep, one PR per priority.** P3 I did by hand (a mechanical Python splitter). P4/P1/P2/P5
   I **delegated to Opus subagents** with precise specs (concern→file map, cfg map, visibility rules,
   mandatory R1-safe verify-green, no-cheats), to conserve main-thread context. After each subagent
   returned, I **independently re-ran the full verify gate** (R1-safe) before `/ship` + commit + push +
   PR + CI + squash-merge.

## Key Decisions

- **Plan corrections applied (from my review):**
  - **`engine::SpriteRenderer` does NOT exist** at the crate root (only `engine::renderer::SpriteRenderer`);
    the plan said to "preserve" it — there was nothing to preserve. (Compile-probed: E0425.)
  - **`RenderState` was already in `src/app/render_state.rs`** (not inside render.rs) — so P1 moved
    methods/free-fns only, never RenderState.
  - **`sprite.rs` was already partly split** (6 submodules incl. tests) — lowest-value target; I told the
    P5 subagent to split `render()` only if cleanly separable (it was) and otherwise leave it.
- **One PR + one PATCH per priority** (0.x cadence; behavior-preserving refactor = PATCH, like the seq-41
  cleanups). v0.43.1 … v0.43.5. CHANGELOG entries are all `### Changed (internal)`.
- **Subagents for the bulky splits, gate as the safety net.** Each subagent had to reach a green
  `./scripts/verify.sh` (R1-safe) — which includes the wasm build (the only thing that compiles the
  native-only `cfg` paths in editor/network/render). I reviewed each diff + re-verified before merge.
- **Visibility pattern for cross-submodule items:** raise `pub(super)` → `pub(in crate::app)` (or
  `pub(in crate::renderer)`) when a moved item is used across the new module boundary; never widen to
  `pub`; keep existing `pub` items `pub`. (Applies to P1/P2/P5; the compiler/clippy enforces the minimum.)
- **`component_registry`/`loading` module decls left UN-gated in P2** (their cross-platform public API —
  `register_editable_component`/`register_serde_component`/`load_*` — must compile on wasm), with the
  native-only contents keeping their own inner `#[cfg(not(wasm32))]`. This was the highest-risk call.

## Evidence & Data

- **Verify gate (R1-safe, exit read from a fresh file) green before every merge:** P3, P4, P1, P2, P5
  each `EXIT=0` + `all checks passed ✓`, **883 lib tests** every time (no test lost or added by any
  split — pure moves).
- **CI 4/4 green on every PR** (Build WASM · Package dry-run · Rustdoc · Test native).
- **Hex blob:** tile-level bit↔edge verification + two in-window screenshots (sent to user).

## Files Changed (high level)

- **#146:** `examples/gen_hex_autotile_sheet.rs` (new), `examples/hex_blob_autotile{,_flat}.rs` (new),
  `examples/assets/hex_autotile{,_flat}.png` (new), `docs/PATTERNS.md` (recipe), CLAUDE.md module-map row.
- **#147:** `state_machine.rs`/`autotile.rs`/`dialogue/mod.rs`/`prefab.rs`/`save.rs` → each grew a
  `tests.rs` child.
- **#148:** `src/network/` = `event`/`native`/`wasm_impl`/`system`/`remote_entities`/`snapshot`/`tests`.
- **#149:** `src/app/render/` = `mod`/`frame`/`docked`/`offscreen`/`post_lighting`/`debug_draw` (+ a few
  unused imports dropped from `src/app.rs`).
- **#150:** `src/app/editor/` += `history`/`settings`/`prefab`/`overlays`/`component_registry`/`loading`/
  `util`/`tests`.
- **#151:** `src/renderer/text/` = `queue`/`cache`/`rich_text`/`renderer`/`tests`; `src/renderer/sprite/`
  += `collect`/`batch`/`draw`.
- Per-PR `/ship` paperwork: `Cargo.toml` + `Cargo.lock` + `docs/CHANGELOG.md` + `CLAUDE.md` header.

## User Feedback & Preferences

- Picked the hex atlas, then "p3 부터 … 머지까지 … 핸드오프 … 완료 보고" (the `/goal`). Authorized
  push→PR→merge for the whole sweep up front.
- Standing: Korean for user-facing reports; English for code/docs/handoffs/subagent prompts. Merge is
  standing-delegated (squash on green CI, direct instruction not an AskUserQuestion option). Use subagents
  for parallel/bulky work with an explicit `model` (Opus here, given difficulty). Honest gap-naming valued.

## Where We're Going

1. **Remaining engine-hardening backlog (unchanged):** **per-OS (Win/Mac) gamepad input** + the deferred
   analog-stick Y-sign hardware test (see `gilrs-macos-xbox-no-input` memory); **crates.io publish**
   (irreversible, needs explicit go; publish `engine_reflect_derive` too).
2. **Optional further refactors:** `frame.rs` (726) and `component_registry.rs` (333) / `tests.rs` files
   are the largest remaining post-sweep, but all are now single-concern and readable — no pressing need.
3. **Optional:** the seq-43 focus-ring extensions (corner radius / pulse), only if a use-case appears.

## Risks & Blockers

- **None outstanding** — all 6 PRs merged green, tree clean, public API + behavior + wasm all preserved.
- **crates.io is irreversible** — do not publish without an explicit user go.
- CLAUDE.md is ~218 lines (over the 200 soft guideline, pre-existing) — trim into a `docs/*.md` if it grows.

## Cross-cutting Gotchas (expensive-to-rediscover)

1. **rust-analyzer diagnostics are STALE/WRONG after a multi-edit subagent refactor — trust `cargo`, not
   the IDE.** This bit a false alarm **four times** this session: "unlinked file" (P4), "method
   step_frame_once is private" E0624 (P1), "duplicate definitions" E0592 (P2), and "Syntax Error" (P5).
   Every one was disproven by an independent `cargo build`/`cargo fmt --check`/full gate. **Always verify
   a refactor with the real compiler before trusting (or panicking at) the squiggles.**
2. **The P5 "syntax error" was an edition mismatch:** the code uses `gen` as an identifier (`let gen = …`,
   `|_k, gen|`), valid in **edition 2021** (this crate) but a reserved keyword in 2024 — rust-analyzer
   parsed it as 2024 and flagged a bogus syntax error. `cargo fmt --check`/`cargo build` accept it. (If the
   crate ever moves to edition 2024, those `gen` identifiers need `r#gen`.)
3. **`gh pr checks <n> --watch` exits immediately with "no checks reported" if run before CI registers**
   (~15–30 s after `gh pr create`). It returns exit 0, so a naive merge-after-watch then fails with
   "BLOCKED / requirements not met" (hit once on #149). Fix: poll `gh pr checks` until a check appears,
   THEN `--watch`.
4. **R1 (verify exit-code) held all session** — read the exit from a *fresh* file, never trust a
   background wrapper's "exit 0". `cargo fmt --check` diffs (long array literals, `use` blocks) were the
   usual first-run failures; `cargo fmt` then re-verify.
5. **Doubly-backgrounding a watch (`cmd & ; echo` inside a `run_in_background`) detaches the real watcher**
   — the harness notifies on the launcher's exit, not the watch's. Run the watch directly as the
   background command.
6. **The wasm build is the ONLY gate for native-only `cfg` correctness** in editor/network/render —
   `cargo test --all-targets` is native-only. A cross-platform method mis-placed into a native-gated module
   passes native tests but fails `cargo build --target wasm32-unknown-unknown`. Never skip the wasm step.

## Process / Versioning Notes

- 0.x cadence: behavior-preserving refactor = PATCH (v0.43.1…v0.43.5); the hex atlas (new
  capability/assets+examples) = MINOR (v0.43.0). Each PR: `/ship` four-edit set + squash-merge on green CI.
- Subagent pattern that worked: precise spec (exact line ranges or concern map + cfg map + visibility rule),
  mandatory R1-safe verify-green-before-return, explicit "no `#[allow]`, no test deletion, no API change",
  report diff-stat + judgment calls. Then orchestrator re-verifies + ships + merges. Keeps the risky git
  ops and final verification in the main thread.

---

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #146 (`fc8363e`, v0.43.0) → #147 (`d5ed372`) → #148 (`4652956`) → #149 (`6532a1b`) →
#150 (`f83b235`) → #151 (`79524ef`, v0.43.5); this handoff committed separately.
**Session status:** Goal met — refactor sweep complete, merged, handed off.
