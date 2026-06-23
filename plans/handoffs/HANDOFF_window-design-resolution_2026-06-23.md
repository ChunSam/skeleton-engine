# Wishlist display arc — EW-002 window mode control + EW-003 design resolution/letterbox (v0.61.0 → v0.62.0)

**Date:** 2026-06-23
**Status:** COMPLETED
**Bead(s):** none (this repo tracks engine work via the dungeon-merchant wishlist board, not beads)
**Epic:** Engine ⇄ game wishlist board (`../dungeon-merchant/docs/engine-wishlist.md`)
**Chain:** `wishlist-display` seq `1`
**Parent:** `none — first in chain`
**Prior chain:** none — first in chain (the prior session's `HANDOFF_followup-batch2-hdr-render-arc_2026-06-23.md` is a *predecessor in time* but a different work stream — the HDR/render-format arc — not a parent of this display/windowing work)

---

## The Goal

The dungeon-merchant game session filed two new requests on the shared wishlist board (both 2026-06-23): a fixed-aspect display setup for a UI authored in a 1280×720 logical space. This session shipped both, each as its own merged PR + a runnable example (the VISION "the example is the acceptance test" loop): **EW-002** (window mode control — non-resizable / 16:9 aspect lock / fullscreen) and **EW-003** (fixed design/virtual resolution + scale-to-window letterbox). Together they let the game keep authoring at 1280×720 and present correctly at any 16:9 window (incl. the 1080p release build) by changing only the window size. The board's ACTIVE list is now empty.

## Where We Are

- **main @ `1c19873`, package v0.62.0, CLAUDE.md header v1.6.131, clean + green, no open PRs.** Both features merged.
- **EW-002 = seq 77, PR #216, v0.61.0** (`b83b38d`). **EW-003 = seq 78, PR #217, v0.62.0** (`1c19873`).
- **EW-002 — `WindowOptions` resource** (`src/resources.rs`): `{ resizable: bool (default true), mode: WindowMode (Windowed | BorderlessFullscreen), lock_aspect: Option<f32> }`. Opt-in, inserted before `App::run()` (the `ImeConfig` pattern). Applied in `src/app/window.rs`: `resumed()` adds `with_resizable` + `with_fullscreen(Some(Fullscreen::Borderless(None)))`; a native `WindowEvent::Resized` handler re-derives `want_h = round(width / ratio)` and `request_inner_size` when it differs by >1px to hold `lock_aspect` (converges in one step — the corrected size's next `Resized` already matches; no feedback loop). Aspect-lock is native-only (wasm canvas size is owned by HTML).
- **EW-003 — `DesignResolution` + `Letterbox` resources** (`src/resources.rs`): `DesignResolution { width, height }` is the opt-in game-facing resource; `Letterbox { clip_scale: Vec2, px_scale: f32, px_offset: Vec2 }` is the computed transform inserted every frame by `compute_viewport` (`src/app/schedule.rs`), identity when no `DesignResolution` / in the docked editor.
- **The letterbox is a coordinate transform, NOT an offscreen render-target + blit.** A centered letterbox = a pure clip-space scale (no translation). `camera::apply_letterbox(clip_scale, proj)` post-multiplies it onto each scene projection and returns `proj` unchanged when `(1,1)` → OFF-path byte-identical.
- **Threading:** `SpriteRenderer::render`, `render_ui_primitives_from_slices`, `GpuParticleRenderer::render`, and `LightingRenderer::update` each gained a `clip_scale` param; the text renderer (`src/renderer/text/renderer.rs`) maps design→window via `px_scale`/`px_offset` composed with `DisplayScaleFactor`; the cursor handler (`src/app/window.rs`) maps the window cursor into design space via `Letterbox::window_to_design`. Offscreen RTs pass `Vec2::ONE`.
- **`ViewportSize` now reports the design size** (1280×720) when `DesignResolution` is set, so `screen_to_world` + cursor hit-testing + game systems all work in design space.
- **Examples:** `examples/window_mode.rs` (EW-002, 16:9 aspect-lock demo) and `examples/design_resolution.rs` (EW-003, a 1280×720 UI letterboxed to any window). Both flat (no Cargo.toml registration).
- **Verify gate green** at each step (`VERIFY_EXIT=0`); CI #216 + #217 both 4/4 green.
- **macOS real-window verification done** for both (CI can't exercise a render/window path) — see Evidence.
- **Unit tests added:** `Letterbox`/`apply_letterbox` math (6 tests in `resources.rs` `letterbox_tests` + 2 in `camera.rs`).
- **Untracked `docs/CODE_QUALITY_FINDINGS_2026-06-23.md`** still sits on the tree, deliberately excluded from both PRs (a separate scan, unrelated to this work).

## Public API surface (this arc)

```rust
// EW-002 — insert before App::run(); absent = a normal resizable windowed window.
pub struct WindowOptions { pub resizable: bool, pub mode: WindowMode, pub lock_aspect: Option<f32> }
pub enum WindowMode { Windowed /*default*/, BorderlessFullscreen }
// Non-resizable (most robust fixed aspect):
app.world.insert_resource(WindowOptions { resizable: false, ..Default::default() });
// 16:9 lock (resizable, snaps back on drag; native-only):
app.world.insert_resource(WindowOptions { lock_aspect: Some(16.0/9.0), ..Default::default() });
// Borderless fullscreen on the current monitor (config-time):
app.world.insert_resource(WindowOptions { mode: WindowMode::BorderlessFullscreen, ..Default::default() });

// EW-003 — insert before App::run(); absent = ViewportSize == window size, no scaling.
pub struct DesignResolution { pub width: f32, pub height: f32 }   // ::new(w, h)
app.world.insert_resource(DesignResolution::new(1280.0, 720.0));
// Computed every frame (read-only data; identity when no DesignResolution / in docked editor):
pub struct Letterbox { pub clip_scale: Vec2, pub px_scale: f32, pub px_offset: Vec2 }
// Letterbox::compute(dw, dh, ww, wh) -> Self ; Letterbox::window_to_design(p) -> Vec2 ; Letterbox::IDENTITY
pub fn engine::camera::apply_letterbox(clip_scale: Vec2, proj: Mat4) -> Mat4;  // identity short-circuit
```

All re-exported from the `engine` crate root.

## What We Tried (Chronological)

1. **Session start** — user: "마지막 핸드오프 보고 다음 작업 확인". Read memory (seq 76, v0.60.0, "next: read the wishlist board then ASK") + the board. Found a NEW P1 (EW-002) and a companion P2 (EW-003) freshly filed. Asked the user which to take → chose EW-002.
2. **EW-002 design decision** — adding fields to `WindowConfig` would break ~70 full-literal call sites (the seq-60 constraint). Chose a separate `WindowOptions` resource (the `ImeConfig` precedent) → non-breaking. EW-002 itself explicitly allowed "a field and/or a resource".
3. **EW-002 impl** — `WindowMode` enum + `WindowOptions` struct (manual `Default` with `resizable: true` since `#[derive(Default)]` gives `false`, the opposite of winit's default). Applied in `window.rs`. Lib compiled clean (winit `with_resizable`/`with_fullscreen`/`request_inner_size` all exist in wgpu-29's winit).
4. **EW-002 verify** — `./scripts/verify.sh` green. macOS check: drove the example to distorted sizes (1400×500 r=2.8, 1500×480, 1500×470 r=3.19) via `osascript ... set size of window 1`, read back via `get size` → all corrected to inner ~16:9 (1.777). Screenshot confirmed the in-window readout "1500 x 844 (ratio 1.777)".
5. **EW-002 land** — `/land-pr`: branch → `/ship` v0.61.0 → commit (excluding the findings doc) → PR #216 → CI 4/4 → squash-merge `b83b38d` → memory seq 77.
6. **User: "EW-003 이어서 진행해줘"** → started EW-003.
7. **EW-003 investigation** — launched an Explore subagent (sonnet) to map how EVERY render pass sources its screen dimensions (sprite/UI/text/particle/post/lighting/egui) + whether any calls `set_viewport`/`set_scissor`. Key findings: sprites/UI/particles/lighting all build projection from `ViewportSize` (`logical_w/h`); text uses physical `gpu.config` for the glyphon resolution + scales by `DisplayScaleFactor`; **no `set_viewport`/`set_scissor` anywhere**; bars come free from the existing full-surface clear.
8. **EW-003 approach evaluation** — considered three strategies (see Key Decisions). Chose Strategy C (coordinate transform / clip-space letterbox scale), rejecting A (design-res offscreen RT + blit — more plumbing, upscaled-softer) and B (per-pass `set_viewport` — messy post-process interaction).
9. **EW-003 impl** — added `DesignResolution`/`Letterbox`, `camera::apply_letterbox`, threaded `clip_scale` into 4 renderers, text scale/offset, cursor remap, `compute_viewport` integration. Fixed signature churn at all call sites (frame.rs ×4, offscreen ×1).
10. **EW-003 clippy fix** — `gpu_particle::render` hit 8 args → `#[allow(clippy::too_many_arguments)]` (mirrors the wgpu pass arg set).
11. **EW-003 verify** — gate green. macOS check at 3 aspects (16:9 fills, wide pillarbox, tall letterbox) — identical scaled layout. Discovered the example's world sprites were occluded by the opaque UI canvas-background rect (UI draws AFTER sprites) → dropped the sprites, UI-only example (matches the acceptance scope).
12. **EW-003 cursor verify attempt** — wanted a screenshot proving the cursor round-trip; no `cliclick` and no pyobjc `Quartz` on the box → fell back to the `window_to_design` round-trip unit test (it's the exact inverse of the render transform).
13. **EW-003 land** — `/ship` v0.62.0 → commit → PR #217 → CI 4/4 → squash-merge `1c19873` → board EW-003 Shipped → memory seq 78.

## Key Decisions

- **EW-002 + EW-003 both delivered as separate opt-in resources, not `WindowConfig` fields.** A `WindowConfig` field add breaks ~70 full-literal example call sites (documented since seq 60). The `ImeConfig` separate-resource pattern keeps both non-breaking and makes the OFF path byte-identical.
- **EW-003 = Strategy C (coordinate transform), NOT an offscreen RT (Strategy A) or per-pass `set_viewport` (Strategy B).**
  - **Insight that made C clean:** a *centered* letterbox is a *pure clip-space scale* — no translation — because clip space is centered at 0 and the content is centered. So `clip_scale = (content_w/win_w, content_h/win_h)`, one axis always `1.0`. Post-multiplying this scale onto any pass's projection letterboxes it uniformly, regardless of how that pass computes NDC.
  - **C beats A on quality:** C rasterizes at native window resolution (crisp text/UI at 1080p); A renders at 720 into an RT then upscales (softer). For a UI-heavy game shipping at 1080, crisp matters. C is what modern engines do (reference-resolution scaling).
  - **C beats B on correctness:** B (per-pass `set_viewport`) interacts badly with the post-process intermediate (bars baked into the intermediate, post effects over bars). C bakes the letterbox into the projections, which flow through the post-process fullscreen blit naturally.
  - **Strategy comparison (the three considered):** A = render the whole frame into a design-res offscreen RT, then a letterbox blit to the surface (centralized, but needs a new blit pipeline + redirecting text/post/lighting `gpu.config` dims to design res; upscaled-softer; most plumbing). B = per-pass `set_viewport` into the letterbox rect (native content, but ~6 pass injections + a messy post-process intermediate interaction). C (chosen) = a clip-space scale baked into each projection + a text/cursor px transform (no new GPU resources, native-res crisp, composes with post/lighting; cost = signature threading across 5 renderers, guarded to a no-op when off).
- **`clip_scale` passed as an explicit param, not read from a World resource by the renderers.** Sprites/particles are used by BOTH the main surface pass AND offscreen RTs; a global resource would wrongly letterbox offscreen RTs. Explicit param = main pass passes the computed scale, offscreen passes `Vec2::ONE`.
- **Text uses `px_scale`/`px_offset` (logical px), glyphon resolution stays the full physical window.** This renders text crisply at native resolution in the letterbox region (vs. an RT upscale). Composes with the existing `DisplayScaleFactor`.
- **`DesignResolution` ignored in the docked editor.** The editor owns the viewport (the central panel rect). `compute_viewport` only computes an active `Letterbox` in the non-docked branch.
- **EW-002 fullscreen is config-time only** (`WindowMode::BorderlessFullscreen`). A runtime F11-style toggle needs access to the engine-private window from game systems — a separate future request.
- **EW-003 example is UI-only.** World sprites were occluded by the opaque UI background rect (render order: clear → sprites → UI → text). The acceptance scope is DrawRect/DrawText/cursor, so UI-only is both clean and representative (the game is UI-heavy).

## Evidence & Data

**Commits this session:**

| Hash | PR | Version | Summary |
|---|---|---|---|
| `b83b38d` | #216 | v0.61.0 | EW-002 `WindowOptions` (resizable / fullscreen / aspect lock) |
| `1c19873` | #217 | v0.62.0 | EW-003 `DesignResolution` (design res + letterbox) |

**EW-002 macOS aspect-lock verification** (requested distorted size → engine-corrected inner size):

| Requested (outer) | Ratio | Corrected (outer) | Inner ≈ | Inner ratio |
|---|---|---|---|---|
| 1400 × 500 | 2.80 | 1400 × 820 | 1400 × 788 | 1.777 |
| 1500 × 480 | 3.13 | 1500 × 876 | 1500 × 844 | 1.777 |
| 1500 × 470 | 3.19 | 1500 × 876 | 1500 × 844 | 1.777 |

(Initial 1280×720 → outer 1280×752 confirms a ~32px title-bar offset; 16:9 = 1.778.)

**EW-003 macOS letterbox verification** (same 1280×720 design canvas, 3 window aspects):

| Window | Aspect vs 16:9 | Result | Bars |
|---|---|---|---|
| 1280 × 720 | exact | canvas fills window | none |
| 1460 × 520 | wider (2.81) | canvas centered | pillarbox L/R |
| 760 × 900 | taller (0.84) | canvas centered | letterbox T/B |

All three: identical layout (border, 4 corner markers, centered title, crosshair, readout) — uniformly scaled. Screenshots at `/tmp/dr_exact_1280x720.png`, `/tmp/dr_wide_1600x520.png`, `/tmp/dr_tall.png` (and `/tmp/wm_shot.png` for EW-002).

**`Letterbox::compute` math (reference cases, all unit-tested):**

```
compute(Dw, Dh, Ww, Wh):
  s = min(Ww/Dw, Wh/Dh)                # uniform fit scale
  content = (Dw*s, Dh*s)
  clip_scale = (content_w/Ww, content_h/Wh)   # one axis == 1.0 (centered)
  px_scale  = s
  px_offset = ((Ww-content_w)/2, (Wh-content_h)/2)
window_to_design(p) = (p - px_offset) / px_scale   # cursor inverse
```

**Verify gate:** `VERIFY_EXIT=0` on every run (fmt / clippy --all-targets / wasm lib+bins build / test --all-targets / rustdoc). CI #216 + #217: Build(WASM), Package dry-run, Rustdoc, Test(native) all pass (native test ≈ 3.5–5 min).

## Render dimension map (from the Explore subagent — the basis for Strategy C)

Each scene pass's coordinate→NDC source, and whether it needed letterbox threading:

| Pass | File:line | Dimension source | Letterbox applied via |
|---|---|---|---|
| Sprite | `sprite.rs:451` | `ViewportSize` (`logical_w/h`) → `camera.view_proj(w,h)` | `clip_scale` param → `apply_letterbox` |
| UI primitives (DrawRect/UiImage) | `ui_primitives.rs:123` | `ViewportSize` → `ortho(0,w,h,0)` | `clip_scale` param → `apply_letterbox` |
| GPU particles | `gpu_particle.rs:315` | `ViewportSize` → `camera.view_proj` | `clip_scale` param → `apply_letterbox` |
| Lighting | `lighting.rs:136` | `ViewportSize` → `light_position_ndc` | `clip_scale` param → ndc×scale, radius×kx |
| Text (glyphon) | `text/renderer.rs:147,179` | `gpu.config` (physical) + `DisplayScaleFactor` | `px_scale`/`px_offset` on positions/sizes |
| Post-process / lighting blit | `post_process.rs`, `lighting.rs` | `gpu.config` intermediate; UV-space fullscreen | none (letterbox flows through the blit) |
| egui overlay/docked | `frame.rs:24`, `docked.rs:62` | `gpu.config` + `pixels_per_point` | none (editor uses the real window) |

`ViewportSize` is computed once per frame in `compute_viewport` (`schedule.rs:267`); the render-time read is `frame.rs:384-390` (`logical_w/h`). The clear pass (`frame.rs:361`) clears the full surface → letterbox bars = `WindowConfig::clear_color`.

## Code Analysis

- **`camera::apply_letterbox(clip_scale: Vec2, proj: Mat4) -> Mat4`** — `if clip_scale == Vec2::ONE { proj } else { Mat4::from_scale(Vec3::new(kx, ky, 1.0)) * proj }`. The identity short-circuit keeps the OFF path byte-identical (no matrix multiply).
- **Render dimension sources (from the Explore map):** sprite `view_proj(w,h)` (sprite.rs:451), UI ortho (ui_primitives.rs:123), gpu particle `view_proj` (gpu_particle.rs:315), lighting `light_position_ndc` (lighting.rs:136) all use `logical_w/h` = `ViewportSize`. Text uses `gpu.config.width/height` for the glyphon `Viewport` + `DisplayScaleFactor` for positions (text/renderer.rs:147,179). Post-process/lighting intermediates are sized to `gpu.config` (physical); their final passes are UV-space fullscreen blits.
- **Lighting under letterbox:** `light_position_ndc` returns full-clip NDC over the design rect; multiply position by `clip_scale` and radius by `clip_scale.x` (radius is a width-fraction → window-fraction = design-fraction × kx). Position alignment is essential; radius is correct for the width axis.
- **`compute_viewport` (schedule.rs)** reads `DesignResolution` via `.copied()` (drops the immutable World borrow before the `insert_resource` calls), computes `(ViewportSize, Letterbox)` via an `apply_design` closure used by both the native non-docked branch and the wasm branch; the docked branch keeps the central-rect viewport + identity letterbox.
- **`WindowOptions` aspect-lock** lives in `window.rs`'s `Resized` arm, native-only; reads `WindowOptions` from World, computes `want_h`, calls `window.request_inner_size`. `self.window` is `Some` on both native (finish_init) and wasm.
- **Import gotcha:** `src/app.rs` imports resources by NAME (`resources::{ ... }`), not a glob — so every new re-exported resource (`WindowOptions`, `WindowMode`, `DesignResolution`, `Letterbox`) must be added there too, or the app-module files (schedule.rs, window.rs, frame.rs use `super::*` / named imports) won't see it. This bit twice during impl (the first lib build failed on `WindowOptions` not in scope).
- **Borrow discipline in `compute_viewport`:** `gpu = self.gpu.as_ref()` (immutable) coexists with `self.world.resource::<DesignResolution>().copied()` (immutable, disjoint field) and the later `self.world.insert_resource(...)` (mutable) — `.copied()` drops the design borrow before the inserts. The `apply_design` closure captures `design` (Copy) and is reused by the native non-docked branch + the wasm branch.

## Files Changed

### Source code
- `src/resources.rs` — `WindowMode` + `WindowOptions` (EW-002); `DesignResolution` + `Letterbox` (EW-003) with `compute`/`window_to_design`/`IDENTITY`.
- `src/camera.rs` — `apply_letterbox` free fn (+ 2 tests).
- `src/lib.rs` — re-export `WindowMode`, `WindowOptions`, `DesignResolution`, `Letterbox`.
- `src/app.rs` — import the new resources into the app-module namespace (named import, not glob).
- `src/app/window.rs` — apply `WindowOptions` (resizable/fullscreen + Resized aspect-lock); cursor → design-space remap.
- `src/app/schedule.rs` — `compute_viewport` applies `DesignResolution` → `ViewportSize` + inserts `Letterbox`.
- `src/app/render/frame.rs` — read `Letterbox.clip_scale`, pass to the 4 scene-pass calls.
- `src/app/render/offscreen.rs` — pass `Vec2::ONE` (offscreen RTs never letterbox).
- `src/renderer/sprite.rs`, `src/renderer/sprite/ui_primitives.rs`, `src/renderer/gpu_particle.rs`, `src/renderer/lighting.rs` — `clip_scale` param + apply to projection / light NDC. (`gpu_particle` got `#[allow(clippy::too_many_arguments)]`.)
- `src/renderer/text/renderer.rs` — read `Letterbox`, scale/offset positions/sizes/bounds.

### Tests
- `src/resources.rs` `letterbox_tests` — 6 tests (same-aspect/taller/wider, window_to_design round-trip, identity no-op, degenerate divide-by-zero).
- `src/camera.rs` — `apply_letterbox` identity + clip-scale tests.

### Examples
- `examples/window_mode.rs` — EW-002 16:9 aspect-lock demo.
- `examples/design_resolution.rs` — EW-003 1280×720 letterbox demo (UI-only).

### Release paperwork
- `Cargo.toml` / `Cargo.lock` (0.60.0 → 0.61.0 → 0.62.0), `docs/CHANGELOG.md` (0.61.0 + 0.62.0 entries), `CLAUDE.md` (header v1.6.131 + module-map row).

### Game repo (board, uncommitted in dungeon-merchant — game session commits it)
- `../dungeon-merchant/docs/engine-wishlist.md` — EW-002 + EW-003 → Shipped, with usage snippets + known edge.

## User Feedback & Preferences

- **Drives engine work via the wishlist board.** Session opened with "마지막 핸드오프 보고 다음 작업 확인" — the established cadence: read memory → read `../dungeon-merchant/docs/engine-wishlist.md` → act on the highest-priority open item, ASK when the board is empty.
- **Picks the next item, then "이어서 진행해줘"** (proceed) — for a wishlist feature this means implement AND land (the full `/land-pr` loop), as established by EW-002 landing with the user watching.
- **Merge authority is standing-delegated** (squash on green CI, no per-session re-confirm) — used for both #216 and #217.
- **Korean for user-facing reports; English for everything written** (code, docs, handoffs, board replies) — followed throughout.
- **`/handoff 하고 푸시 해줘`** — wants the handoff committed + pushed (a `docs(handoff)` PR per the per-seq cadence).

## Where We're Going

1. **Next session: read the wishlist board first** (`../dungeon-merchant/docs/engine-wishlist.md`). ACTIVE is now **empty** (next free ID **EW-004**); EW-001/002/003 all Verified/Shipped.
2. **If the board is still empty → ASK the user for direction** (don't self-assign backlog). Possible offers if asked: the deferred items in `docs/CODE_QUALITY_FINDINGS_2026-06-23.md` (untracked scan — UI pointer-capture sharing, dialogue style config, egui-submission dedup), GPU particles under HDR post, a runtime fullscreen (F11) toggle (EW-002 follow-on).
3. **Game-side verification pending:** EW-002 + EW-003 are `Shipped` on the board, awaiting the game session to integrate (`WindowOptions { lock_aspect: Some(16.0/9.0) }` + `DesignResolution::new(1280.0, 720.0)`) and mark `Verified`.

## Risks & Blockers

- **EW-003 known edge:** `TextAlign::Center`/`Right` *without explicit bounds* aligns within the full window, not the design rect (glyphon's viewport = physical window). Mitigation documented on the board: pass bounds, or use anchored `DrawText::centered`. If the game hits mis-centered text under `DesignResolution`, this is why.
- **Lighting under letterbox** scales position by `clip_scale` + radius by `clip_scale.x`; radius is exact on the width axis but slightly anisotropic under a non-square letterbox. Lighting is native-only + niche; acceptable for v1.
- **No cursor-injection tooling on this box** (no `cliclick`, no pyobjc `Quartz`) — automated cursor-position tests aren't possible; rely on the `window_to_design` unit test + visual round-trip reasoning.
- **CI is ubuntu-only** — the window/render paths here are macOS-verified manually; don't assume green CI verified the windowed/letterbox behavior.

## Reusable Gotchas & Recipes

These cost time to rediscover — captured for any future window/render-path work.

- **Driving a winit window from the shell on macOS (for CI-unverifiable render checks):** resize via `osascript -e 'tell application "System Events" to tell process "<binname>" to set size of window 1 to {W, H}'`, read back with `get size of window 1` / `get position of window 1`, then `screencapture -o -R"x,y,w,h" out.png`. The process name is the example binary name (e.g. `design_resolution`). First-call focus can fail (empty position) — `activate` + `set frontmost ... to true` first, then `sleep 0.4`. macOS reports the OUTER window size; the title bar is ~28–32px (so inner height ≈ outer − 32). `screencapture -C` includes the cursor.
- **No cursor-injection tool on this box** — neither `cliclick` nor pyobjc `Quartz` (`CGWarpMouseCursorPosition`) is installed. Cursor-mapping correctness must be unit-tested (the `window_to_design` round-trip), not screenshotted.
- **`cargo fmt` BEFORE the verify gate** — `cargo fmt --check` reflows hand-wrapped lines and reds the FIRST gate (`[[cargo-fmt-reflow-trap]]`). Run `cargo fmt` first, then `./scripts/verify.sh > /tmp/v.log 2>&1; echo $?` (read the exit code from the log, never pipe the gate).
- **Adding an arg to a renderer can trip clippy `too_many_arguments` (>7)** — `gpu_particle::render` went 7→8 → needs `#[allow(clippy::too_many_arguments)]`. `sprite::render` (6→7) and `render_ui_primitives_from_slices` (6→7) stayed at the 7 boundary, OK.
- **Render order is clear → sprites → UI primitives → text** — an opaque `DrawRect` (UI) covers world sprites. The `design_resolution` example originally had world sprites hidden behind its opaque canvas-background rect; dropped them. If a demo needs both, use a low-z background *sprite* (not a UI rect), or a transparent UI background.
- **`WindowConfig` is built by full struct-literal in ~70 examples** — adding a field is a ~70-site breaking change. The standing pattern (used by `ImeConfig`, and now `WindowOptions` + `DesignResolution`) is a separate opt-in resource. Keep this in mind for any future window/display config.
- **The untracked `docs/CODE_QUALITY_FINDINGS_2026-06-23.md`** is a separate scan (not from this work) — it was deliberately staged-out of both PRs (`git add` specific files, never `git add -A`). It still sits uncommitted; decide its fate (commit as its own docs PR, or address its findings) in a future session.

## Open Questions

- None blocking. (Game-side `Verified` is pending but is the game session's court.)

## Quick Start for Next Session

```bash
# 1. Read the wishlist board FIRST (drives engine work)
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE empty? next ID EW-004 → ASK

# 2. Confirm current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -3   # tip should be 1c19873 (v0.62.0)
grep -m1 '^version' Cargo.toml                                # 0.62.0

# 3. Key files for this arc (if a follow-on lands)
#   src/resources.rs                  — WindowOptions / WindowMode / DesignResolution / Letterbox
#   src/camera.rs                     — apply_letterbox
#   src/app/schedule.rs               — compute_viewport (ViewportSize + Letterbox)
#   src/app/window.rs                 — WindowOptions apply + Resized aspect-lock + cursor remap
#   src/app/render/frame.rs           — clip_scale threaded into scene passes
#   examples/window_mode.rs, examples/design_resolution.rs

# 4. Verify gate (authoritative — read the exit code, never pipe it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 5. Manual render/window checks (CI can't): run the examples on macOS, resize, observe
cargo run --example window_mode        # drag → snaps to 16:9
cargo run --example design_resolution  # drag → letterboxed, identical layout

# Next action: read the wishlist board; if ACTIVE is empty, ASK the user for direction
#              (offer the CODE_QUALITY_FINDINGS items or an EW-002 runtime-fullscreen follow-on).
```
