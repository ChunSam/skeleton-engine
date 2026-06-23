# egui overlay submission consolidated into one helper (v0.63.1)

**Date:** 2026-06-23
**Status:** COMPLETED
**Bead(s):** none (engine work tracked via the dungeon-merchant wishlist board + the untracked code-quality scan)
**Epic:** Code-quality findings backlog (`docs/CODE_QUALITY_FINDINGS_2026-06-23.md`)
**Chain:** `codequality-backlog` seq `3` (Task 3 — the LAST of the user-ordered 1→2→3 run)
**Parent:** seq 2 = `HANDOFF_dialogue-style_2026-06-23.md` (DialogueStyle, v0.63.0)
**Next:** the 1→2→3 run is COMPLETE → final report, then read the wishlist board (ASK if empty).

---

## The Goal

The scan's **P2 egui-submission-duplication** finding: the egui renderer lifecycle (update texture deltas → update buffers → record the render pass → submit → free textures → restore the renderer) was duplicated near-identically in `frame.rs` (the final surface overlay) and `docked.rs` (the docked-editor placeholder), differing only in callback handling — so a future egui change had to be applied twice and the two paths' callback behavior could drift. This task consolidates them into one helper.

## Where We Are

- **main @ `2974b02`, package v0.63.1, CLAUDE.md header v1.6.135, clean + green.** PR #224 merged (squash).
- **New `egui_pass::submit_egui(render: &mut RenderState, gpu: &GpuContext, view: &TextureView, guard_callbacks: bool)`** — the single egui-submission flow. It `take()`s `render.egui_renderer` + `render.egui_output`, builds the `ScreenDescriptor` from `gpu.config` + ppp, updates texture deltas, updates buffers, records the pass (guarded), submits its own encoder, frees freed textures, restores the renderer. No-op when either is absent.
- **`guard_callbacks` captures the two call sites' only difference:** `frame.rs::present_egui` passes **`true`** (paint callbacks unsupported on the final overlay → skipped with a warn, for render-pass lifetime safety); `docked.rs` passes **`false`** (the placeholder UI never produces callbacks → records directly). Logic is byte-identical to the two inlined blocks.
- **`present_egui` is now a 3-line delegate**; the ~36-line inlined block in `docked.rs` is now one call.
- **`egui_render_pass` is now private** to `egui_pass` (only `submit_egui` calls it). **`paint_jobs_contain_callbacks` stays `pub(super)`** — it has a dedicated `#[cfg(test)]` unit test in `app.rs` (`egui_callback_jobs_are_detected_before_unsafe_render_pass`).
- Verify gate green (`VERIFY_EXIT=0`); CI #224 4/4 green.
- **Native smoke** (the egui overlay path is CI-unverifiable) — see Evidence.

## Public API surface

**None.** All items are `pub(super)`/private inside `crate::app`. No public API change, no behavior change — a pure internal consolidation.

## Key Decisions

- **One helper with a `bool` policy param, not two helpers or a closure.** The finding suggested "an explicit callback policy parameter if the two paths need different behavior" — exactly `guard_callbacks`. The two paths differ only in whether callbacks are guarded, so a `bool` is the minimal honest abstraction.
- **`submit_egui` takes `&mut RenderState` + `&GpuContext`** (rather than the four sub-fields) — it owns the full take/restore of `egui_renderer`/`egui_output`, so passing the struct keeps the call sites one line. `RenderState = crate::app::render_state::RenderState`, `GpuContext = crate::renderer::GpuContext`; `egui_pass` imports both.
- **`paint_jobs_contain_callbacks` kept `pub(super)`** (not privatized with `egui_render_pass`) — it's unit-tested directly from `app.rs`. `egui_render_pass` had no such test and no other caller, so it became private.
- **`pub(super)` = `pub(in crate::app)`** reaches `app::render::{frame,docked}` (grandchildren), which is why the call sites can use `submit_egui` — same visibility the pre-existing `egui_render_pass`/`paint_jobs_contain_callbacks` imports relied on.

## Evidence & Data

| Hash | PR | Version | Summary |
|---|---|---|---|
| `2974b02` | #224 | v0.63.1 | egui submission → `submit_egui` |

**Native smoke** (`/tmp/egui_overlay.png`): ran `gpu_particles`, toggled the **F1** debug overlay (key code 122) — the egui **"엔진 통계"/Engine Stats** panel (FPS 60.2, ms 16.62, entities 1) **+ "인스펙터"/Inspector** panel (tabs, snap checkbox, new-entity, search, save/load) both render correctly through the shared `submit_egui`; the GPU particles still draw; clean log (no errors/panics). Confirms the surface-overlay path (`guard_callbacks = true`) is behavior-preserving. (The docked path, `guard_callbacks = false`, is the F2 placeholder; same code, only the bool differs.)

**Verify gate:** `VERIFY_EXIT=0` (fmt / clippy `--all-targets -D warnings` / wasm lib+bins build / `test --all-targets` / rustdoc). CI #224 4/4 green. Net `7 files changed, 75 insertions(+), 73 deletions(-)` (incl. the version/CHANGELOG paperwork) — roughly a wash in lines but the duplicated logic now lives once.

## Files Changed

- `src/app/egui_pass.rs` — `submit_egui` added; `egui_render_pass` → private; `paint_jobs_contain_callbacks` kept `pub(super)`; imports `RenderState`/`GpuContext`.
- `src/app/render/frame.rs` — `present_egui` delegates to `submit_egui(.., true)`; dropped the `egui_render_pass`/`paint_jobs_contain_callbacks` imports.
- `src/app/render/docked.rs` — the inlined block → `submit_egui(.., false)`; swapped its import.
- `CLAUDE.md` header v1.6.135; `Cargo.toml`/`Cargo.lock` (0.63.1); `docs/CHANGELOG.md` (0.63.1 entry).

## Risks & Blockers

- **CI is ubuntu-only** — the egui overlay render is macOS-smoke-verified; green CI does not exercise the overlay. The change is byte-identical logic relocated, so risk is low, but the smoke confirms it.
- No new tests (it's a behavior-preserving move; the existing `app.rs` callback-detection test still covers `paint_jobs_contain_callbacks`, and the smoke covers the integrated path).

## Reusable Gotchas & Recipes

- **`pub(super)` in `crate::app::egui_pass` = `pub(in crate::app)`** → reachable from `app::render::frame`/`docked` (deeper descendants), not just the immediate parent. That's why a helper in `egui_pass` can back both render submodules.
- **Before privatizing a `pub(super)` helper, grep for `#[cfg(test)]` users** — `paint_jobs_contain_callbacks` had a test in `app.rs` (a different module), so it had to stay `pub(super)` even though no non-test code calls it directly anymore.
- **macOS F-key codes:** F1 = `key code 122`, F2 = `120` (the docked-editor toggle). Used to drive the egui overlay for the smoke.

## Quick Start for Next Session

The **1→2→3 code-quality-backlog run is COMPLETE** (seq 80 GPU-particles-HDR, seq 81 DialogueStyle, seq 82 egui-dedup). Remaining scan items are all **P3 polish** (wgpu bind-group/pass boilerplate helpers, DebugDraw constants, frame-pacing config) + the still-untracked `docs/CODE_QUALITY_FINDINGS_2026-06-23.md` itself (decide: commit as its own docs PR, or address + drop).

```bash
git -C /Users/jkl/Projects/skeleton-engine log --oneline -3   # tip 2974b02 (v0.63.1)
cat ../dungeon-merchant/docs/engine-wishlist.md               # read the board FIRST; ASK if empty
```
