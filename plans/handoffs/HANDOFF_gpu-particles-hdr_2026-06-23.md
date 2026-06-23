# GPU particles under HDR post — the last format-matched scene pass (v0.62.2)

**Date:** 2026-06-23
**Status:** COMPLETED
**Bead(s):** none (engine work is tracked via the dungeon-merchant wishlist board + the untracked code-quality scan)
**Epic:** Code-quality findings backlog (`docs/CODE_QUALITY_FINDINGS_2026-06-23.md`)
**Chain:** `codequality-backlog` seq `1` (Task 1 of a user-ordered 1→2→3 run: GPU-particles-HDR → DialogueStyle → egui-dedup)
**Prior chain:** seq 79 (`fix/ui-pointer-capture`, v0.62.1) closed the scan's **P1 UI pointer capture**; this closes the scan's **P1 HDR** tail.

---

## The Goal

The untracked `docs/CODE_QUALITY_FINDINGS_2026-06-23.md` flagged that HDR post-process still depended on surface-format pipelines. seq 76 (v0.59.0) format-matched the material + UI passes, leaving **GPU particles as the only scene pass skipped under HDR post** (a warn-once, particles invisible). This task closes that — the GPU-particle render pipeline is now built per target format, so **every scene pass renders under HDR**.

## Where We Are

- **main @ `9d69921`, package v0.62.2, CLAUDE.md header v1.6.133, clean + green.** PR #220 merged (squash).
- HDR post (`PostProcessConfig::hdr`) renders the scene into an `Rgba16Float` intermediate. The GPU-particle render pipeline was bound to the surface format at `GpuParticleRenderer::new`, so `frame.rs` skipped particles whenever `scene_format != gpu.config.format` (warn-once).
- **Fix = the established per-format pipeline cache pattern** (sprite v0.56.0 `extra_sprite_pipelines`, material/UI v0.59.0): `GpuParticleRenderer` gained `base_format`, a stored `render_pipeline_layout`, and `extra_render_pipelines: HashMap<TextureFormat, RenderPipeline>`. The render pipeline descriptor was extracted to a free fn `build_particle_render_pipeline(device, shader, layout, format)`, shared by `new()` (surface format) and `ensure_render_pipeline(device, format)` (lazy build + cache for a non-surface format, recompiling the render shader once per format).
- `render()` gained a `target_format` param and selects the pipeline via `render_pipeline_for(format)` (base for the surface format, else the cached extra, falling back to base to never panic mid-frame).
- `frame.rs` (Step 2.8): removed the `hdr_skip_particles` branch + warn-once; now calls `gpr.ensure_render_pipeline(&gpu.device, scene_format)` (via `as_mut()`) then `gpr.render(..., scene_format, clip_scale)`. **Surface-format path = cache no-op, byte-identical.**
- `gpu_particles` example: `H` toggles `PostProcessConfig { enabled, hdr, tonemap: AcesFilmic }`; HUD shows `HDR post: ON (ACES) / off`.
- Verify gate green (`VERIFY_EXIT=0`); CI #220 4/4 green.
- **Native GPU smoke done** (CI is ubuntu, no GPU render verification) — see Evidence.

## Public API surface (this change)

```rust
// GpuParticleRenderer is App-managed (users only attach a GpuParticleEmitter component).
// New:
pub fn ensure_render_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat);
// Changed signature (added target_format before clip_scale):
pub fn render(&self, queue, view, encoder, world, width, height,
              target_format: wgpu::TextureFormat, clip_scale: Vec2);
```

No user-facing API change for normal use (attach `GpuParticleEmitter` + optionally set `PostProcessConfig`). Versioned **PATCH** (v0.62.2): a fix completing HDR; the only signature change is to an internally-managed renderer.

## Key Decisions

- **Mirror the sprite/material/UI per-format cache, not a redesign.** The pattern was already established three times; GPU particles were simply the pass that hadn't adopted it. Lazy build, cached, paid once per distinct format, never per frame; the surface-format fast path is untouched.
- **Pass `target_format` as an explicit param** (like sprite's `ctx.format`), not a World resource — the renderer is driven by `frame.rs` which knows `scene_format`. (Offscreen RTs don't render GPU particles — the offscreen pass is sprite-only — so there's no second caller to thread.)
- **`render()` stays `&self`; ensure is a separate `&mut self` call** before it (exactly like `ensure_sprite_pipeline`), so `frame.rs` borrows `self.render.gpu_particle_renderer.as_mut()` for ensure, then `&...` for the draw.
- **Exercise via the existing `gpu_particles` example with an `H` toggle**, not a new example — the demo already drives GPU particles; adding an HDR toggle makes the combo a one-keypress visual check.
- **PATCH not MINOR** — it's a fix (a feature that was silently skipped now works); no new user-facing API.

## Evidence & Data

| Hash | PR | Version | Summary |
|---|---|---|---|
| `9d69921` | #220 | v0.62.2 | GPU particles render under HDR post |

**Native GPU smoke** (`/tmp/gpu_hdr_off.png`, `/tmp/gpu_hdr_on.png`):
- HDR **off**: HUD `HDR post: off`, yellow particles render at screen center.
- HDR **on** (`H` → key code 4): HUD `HDR post: ON (ACES)`, particles **still render** (ACES tonemap visibly applied — edge darkening). Example log empty (no wgpu validation / format-mismatch / panic).

**Verify gate:** `VERIFY_EXIT=0` (fmt / clippy `--all-targets -D warnings` / wasm lib+bins build / `test --all-targets` / rustdoc). CI #220: Build(WASM), Package dry-run, Rustdoc, Test(native) all green.

## Files Changed

- `src/renderer/gpu_particle.rs` — `base_format` / `render_pipeline_layout` / `extra_render_pipelines` fields; `build_particle_render_pipeline` free fn; `ensure_render_pipeline` + `render_pipeline_for`; `render()` `target_format` param.
- `src/app/render/frame.rs` — Step 2.8: removed HDR skip + warn-once; `ensure_render_pipeline(scene_format)` + `render(..., scene_format, ..)`.
- `examples/gpu_particles.rs` — `H` toggles HDR post (ACES); HUD line.
- `CLAUDE.md` — post_process module-map row (GPU particles no longer the HDR exception); header v1.6.133.
- `Cargo.toml` / `Cargo.lock` (0.62.2), `docs/CHANGELOG.md` (0.62.2 entry).

## Risks & Blockers

- **CI is ubuntu-only** — this render path is macOS-GPU-verified manually; green CI does not verify the HDR-particle render. Don't assume green CI covered it.
- A future pass with **another non-surface RT format** for particles (e.g. an HDR offscreen RT that also renders GPU particles) would just hit the cache and build a pipeline for it — no further work, but untested (no current caller renders particles into an offscreen RT).

## Reusable Gotchas & Recipes

- **macOS synthetic key into a winit window:** `osascript -e 'tell application "System Events" to set frontmost of process "<bin>" to true'` (the `set frontmost of process …` form — NOT a `set frontmost` inside a `tell process` block, which errors `-10006`), then `key code 4` for `H` (raw key code, not `keystroke`). Window opened at ~(320,105) despite a `set position` request that errored; read it back with `get position of window 1` and `screencapture -x -o -R<x>,<y>,<w>,<h>`.
- **HDR needs BOTH `enabled` + `hdr`** on `PostProcessConfig` (the post pass runs on `enabled`; `hdr` picks the `Rgba16Float` intermediate). The example's `H` sets both.
- **Per-format pipeline cache pattern** (now used 4×: sprite, material, UI, GPU particles): store `base_format` + the pipeline layout + a `HashMap<TextureFormat, RenderPipeline>`; `ensure_*` builds+caches lazily (re-`include_str!` the shader, cheap), `*_for` returns base or cached-or-base. The surface-format fast path stays a direct field access.

## Quick Start for Next Session

This was Task 1 of the user's 1→2→3 backlog run. **Next = Task 2: DialogueStyle resource** (extract `src/dialogue/mod.rs` hardcoded layout/colors/font-sizes/portrait-placement/viewport-fallback into a `DialogueStyle` resource, defaults matching current literals; exercise via a dialogue example). Then **Task 3: egui submission dedup** (`frame.rs` + `docked.rs` → `egui_pass.rs`).

```bash
git -C /Users/jkl/Projects/skeleton-engine log --oneline -3   # tip 9d69921 (v0.62.2)
grep -n "DialogueStyle\|fallback viewport\|portrait" src/dialogue/mod.rs   # Task 2 starting point
```
