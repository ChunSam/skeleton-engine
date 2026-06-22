# HDR / linear render targets via a format-matched sprite pipeline (v0.56.0)

**Date:** 2026-06-23
**Status:** COMPLETED + merged. main @ `249c5f2`, package **v0.56.0**, clean tree, full gate green, CI green, squash-merged (#207).
**Bead(s):** none (bd unavailable)
**Epic:** post-audit feature work — the `/goal` P1→P4 carried-direction run (this = **P4**, the final phase)
**Chain:** `standalone-4365aa4a` seq `11`
**Parent:** `HANDOFF_ron-registry-pub_2026-06-23.md` (seq 10, P3)
**Auto:** false (P1→P4 `/goal`: each phase test→handoff→merge; **this is the last phase — all four done**)

> NB the memory `engine-current-state` engine-wide seq for this work is **seq 74**. Also closes the seq-67-deferred "HDR render target needs a format-matched pipeline" item.

---

## The Goal

P4 (final) of the session goal: the carried direction **HDR / linear render-target** — the riskiest of the four. Render targets were locked to the surface format (`create_render_target` used `gpu.config.format`), and the single `SpriteRenderer` had one format-bound pipeline, so an `OffscreenCamera` rendering into a non-surface-format RT (e.g. `Rgba16Float` HDR) would hit a wgpu color-target format mismatch. seq-67 explicitly deferred this as "needs a format-matched pipeline." This session delivered it.

## Where We Are

- **main @ `249c5f2`, package v0.56.0, CLAUDE.md header v1.6.125, clean tree, `./scripts/verify.sh` → exit 0.** PR #207 squash-merged on green CI; branch `feat/hdr-render-target` deleted.
- **`App::create_render_target_with_format(name, w, h, format)`** (`src/app/assets.rs`) — additive; `create_render_target` now delegates to a private `create_render_target_impl(.., Option<format>)` with `None` (= surface format). `pending_render_targets` carries `Option<wgpu::TextureFormat>` (the surface format isn't known until GPU init; `None` resolves to it in `window.rs`).
- **`RenderTarget`** stores `format: wgpu::TextureFormat` (`pub(crate)`) + `pub fn format()`.
- **Format-matched sprite pipeline** (`src/renderer/sprite.rs`): `SpriteRenderer` gains `base_format`, `sprite_pipeline_layout` (kept so the cache can rebuild), and `extra_sprite_pipelines: HashMap<TextureFormat, RenderPipeline>`. The inline base-pipeline build was extracted to a free fn **`build_sprite_pipeline(device, shader, layout, format)`** (used for both the base format and the cache). `render()` reads `ctx.format`, calls `ensure_sprite_pipeline` (builds + caches for a new format — recompiles the sprite shader once per distinct format), selects the matching pipeline via `sprite_pipeline_for`, and passes it to **`record_draw_pass(.., sprite_pipeline, ..)`** (`src/renderer/sprite/draw.rs` — signature gained the pipeline param; the `set_pipeline(&self.pipeline)` became `set_pipeline(sprite_pipeline)`).
- **Offscreen pass** (`src/app/render/offscreen.rs`): the `OffscreenRenderInfo` tuple (`src/app.rs`) gained the RT's `format`; the sprite render's `FrameContext.format` is now the RT's format (was `gpu.config.format`).
- **UI + material pipelines stay surface-format** — `render()` (the offscreen entry) is **sprite-only** (no UI pass; UI/materials are separate, surface-targeted), so only the sprite pipeline needed format-keying.
- **Example** `examples/hdr_render_target.rs` (native-only flat) — two `OffscreenCamera`s render an over-bright off-screen scene (sprite colours **> 1.0**: dim bg 0.5, mid 2.5, core 6.0) into an `Rgba16Float` "hdr" RT and a default 8-bit "ldr" RT; two on-screen monitor sprites display them with an **exposure multiply** (the display sprite's `Color`). `↑`/`↓` adjust exposure.
- **Paperwork**: CLAUDE.md render-target module-map row (rewritten) + header bump; CHANGELOG 0.56.0; Cargo.lock → 0.56.0.
- **Memory** `engine-current-state` → seq 74; `MEMORY.md` index refreshed.

## What We Tried (Chronological)

1. **Mapped the renderer.** Key facts: `create_render_target` → `gpu.config.format`; one `SpriteRenderer` built once with the surface format; the offscreen pass (`offscreen.rs:111`) passed `gpu.config.format` to the sprite `FrameContext`; the sprite pipeline binds in `sprite/draw.rs:74` (`self.pipeline`); `render()` is **sprite-only** (no UI). The `MaterialRenderer::new` **consumes** the sprite `shader` + `camera_layout`, but the `pipeline_layout` (shared sprite+UI) is only borrowed → keep it for the cache, recompile the shader on demand.
2. **Implemented the engine change** top-down: `RenderTarget.format` + getter → `pending_render_targets: Option<format>` + `create_render_target_with_format` → `SpriteRenderer` fields + `build_sprite_pipeline` + `ensure_sprite_pipeline`/`sprite_pipeline_for` + `render()` wiring → `record_draw_pass` param → `OffscreenRenderInfo` + offscreen `FrameContext.format`. **Lib compiled clean** on the first try after wiring (the borrow plan held: ensure (`&mut self`) → select (`&self`) → `record_draw_pass(&self, pipe)` — two coexisting immutable borrows).
3. **Wrote the example**, fixed two compile errors (`Sprite.texture` wants `Arc<str>` → `.into()`; `RenderLayer` is `i32` not `u32`). Full gate **exit 0** (fmt trap didn't bite).
4. **First smoke → both monitors BLACK.** Root cause: **`Camera::position` is the view's TOP-LEFT corner**, not the center (`view = [pos.x, pos.x + w/zoom] × [pos.y, pos.y + h/zoom]`, Y down). My `OffscreenCamera` at `SCENE` framed *past* the scene; and the main camera at `ZERO` put the monitors (placed at ±205 world x) off-screen / mis-placed. **Sprites are centered** at `Transform.position` (unit quad ±0.5), but the **camera is corner-anchored** — the two conventions differ.
5. **Fixed framing:** `OffscreenCamera` position = `SCENE - (RT_W/2, RT_H/2)` (center the scene in the frame); main camera at `ZERO` → view `[0,WIN_W]×[0,WIN_H]`, monitors centered at `(210,215)` / `(610,215)`. **Second smoke → exactly right:** at exposure 0.20 the HDR monitor shows three distinct levels (dim bg / olive mid / **white core**); the LDR monitor **collapses** mid+core into one flat gray (both clamped to 1.0 at store). Process alive, stderr 0, no format-mismatch panic.
6. **`/ship`** (0.55→0.56, lock, CHANGELOG, CLAUDE.md), re-verify exit 0. **`/land-pr`** — commit `e8e1b81`, push, PR #207, CI, squash-merge, sync.

## Key Decisions

- **Sprite pipeline only — not UI/material.** The offscreen `render()` entry is sprite-only, so only the sprite pipeline needs format-keying. UI/material pipelines (surface-targeted, separate passes) are left untouched — smaller blast radius, no offscreen UI/material support (documented; rare need).
- **Lazy per-format cache, surface fast-path untouched.** `extra_sprite_pipelines` is empty in the common single-format case; `ensure_sprite_pipeline` is a no-op for the base format and on cache hits. No per-frame cost for the 99% case; an HDR RT pays one shader recompile + pipeline build on first use.
- **Recompile the shader in the cache builder** rather than storing the sprite `ShaderModule` — `MaterialRenderer::new` consumes it, and a recompile-per-new-format (rare) is far simpler than threading shared ownership.
- **Tone-mapping is the game's job.** The facade-style honest scope: the engine makes a non-surface-format RT *render correctly*; turning HDR values into a displayable image (exposure/tonemap) is left to the game. The example shows the simplest tonemap (a constant exposure multiply via the display sprite's `Color`, which is f32 + un-clamped so >1.0 reaches the shader) — no new engine pipeline, and it makes the HDR-vs-8-bit difference visible.
- **MINOR bump v0.56.0** (additive API).

## Reusable Gotchas & Patterns (carry forward)

- **`Camera::position` is the view's TOP-LEFT corner, not the center** (`view = [pos.x, pos.x + width/zoom] × [pos.y, pos.y + height/zoom]`, **Y increases downward**) — see `src/camera.rs` doc. But **sprites are centered** at `Transform.position` (unit quad ±0.5). Mixing the two (placing a camera as if centered) silently frames the wrong region → a **black render target**. To center a scene in an `OffscreenCamera`: `Camera::new(scene_center - Vec2::new(rt_w/2, rt_h/2), zoom)`. For the main camera at `ZERO`, the visible world is `[0,win_w]×[0,win_h]` — place on-screen sprites with **positive** coords inside that box.
- **A black render target = the offscreen camera framed off the scene** (the scene didn't land in the captured region), *not* a pipeline failure. Check the camera framing (corner-anchored!) before suspecting the format/pipeline.
- **Sprite `Color` is f32 and `to_array()` does NOT clamp** — values > 1.0 reach the shader (only the u8 conversion clamps). This is what makes an HDR demo possible (over-bright sprites) and is the simplest exposure-tonemap knob (display sprite colour = exposure).
- **`MaterialRenderer::new` consumes the sprite `shader` + `camera_layout`** (`sprite.rs:~295`) — anything needing the sprite shader after that must recompile it (cheap) or be built before. The `pipeline_layout` is only borrowed, so it's safe to keep.
- **Borrow pattern for a lazily-cached resource used in a `&self` method:** ensure (`&mut self`) first, then select a `&T` (immutable borrow), then call the `&self` consumer passing the `&T` — two coexisting immutable borrows compile cleanly.
- **macOS synthetic key: ArrowDown = 125, ArrowUp = 126.** Region-capture via `screencapture -x -o -R<x>,<y>,<w>,<h>` from `get {position, size} of front window` — the only legible way to read a small window's content.

## Files Changed

- `src/renderer/render_target.rs` — `format` field + `format()`.
- `src/renderer/sprite.rs` — `base_format`/`sprite_pipeline_layout`/`extra_sprite_pipelines` fields; `build_sprite_pipeline` free fn; `ensure_sprite_pipeline`/`sprite_pipeline_for`; `render()` wiring (`ctx.format` → ensure → select → pass).
- `src/renderer/sprite/draw.rs` — `record_draw_pass` gains `sprite_pipeline: &RenderPipeline`; `set_pipeline(sprite_pipeline)`.
- `src/app/render/offscreen.rs` — thread `rt.format` into the tuple + `FrameContext.format`.
- `src/app/assets.rs` — `create_render_target_with_format` + `create_render_target_impl`.
- `src/app.rs` — `OffscreenRenderInfo` + `pending_render_targets` carry the format.
- `src/app/window.rs` — pending RT loop resolves `Option<format>`.
- `examples/hdr_render_target.rs` — new (native-only flat).
- `CLAUDE.md` (render-target row + header v1.6.125 + v0.56.0), `docs/CHANGELOG.md` (0.56.0), `Cargo.lock`.
- Memory `engine-current-state` → seq 74; `MEMORY.md`.

## Where We're Going

- **P4 done + merged — the `/goal` P1→P4 carried-direction run is COMPLETE** (P1 named tone channels + low-pass + settings_menu; P2 positional facade; P3 RonRegistry pub; P4 HDR render target). All four landed as code PR + own docs(handoff) PR, each with a native real-play smoke.
- **Next session: read the wishlist board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`, ACTIVE empty, next ID EW-002). If empty, ASK for direction — the carried backlog is now exhausted.
- **Possible follow-ups surfaced this run** (offer, not committed): ship `positional_audio` / `hdr_render_target` to the web (`/ship-wasm-example`); a real tonemap/bloom engine pass (this run left tonemapping to the game); offscreen UI/material pipeline format-matching (left surface-only); a `wgpu` re-export so games naming `wgpu::TextureFormat` for `create_render_target_with_format` / `load_image_with_format` don't need a direct `wgpu` dep.

## Risks & Blockers

- **None blocking.** main clean + green at v0.56.0; all four phases verified by native smoke.
- **HDR RT display needs game-side tonemapping** — without it, sampling an `Rgba16Float` RT and outputting to the (clamping) surface clips >1.0 values. Documented; the example demonstrates the simplest exposure tonemap. A general engine tonemap/bloom pass is a future feature.
- **Offscreen UI/material into a non-surface format is unsupported** (those pipelines stay surface-format; the offscreen pass is sprite-only). Documented; extend if a game needs it.

## Quick Start for Next Session

```bash
git checkout main && git pull --ff-only        # expect the seq-11 handoff docs PR or later
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?" >> /tmp/verify.log; grep VERIFY_EXIT /tmp/verify.log

# The P1→P4 carried backlog is DONE. Read the wishlist board first:
cat ../dungeon-merchant/docs/engine-wishlist.md    # ACTIVE empty, next ID EW-002
# If empty, ASK for direction (or offer a follow-up from this run's "Where We're Going").
```

---

## Session Closed (P4 — final)

**Closed at:** 2026-06-23
**Code work:** HDR/linear render targets + format-matched sprite pipeline + `hdr_render_target` example landed via PR **#207** (v0.56.0, merge `249c5f2`).
**Landing:** this handoff lands on `main` via its own `docs(handoff)` PR. Memory `engine-current-state` at seq 74. **The P1→P4 `/goal` run is complete.**
