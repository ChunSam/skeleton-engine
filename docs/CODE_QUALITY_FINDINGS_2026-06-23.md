# Code quality findings — hardcoding and duplication

**Date:** 2026-06-23  
**Scope:** full `src/` engine tree, with `examples/` scanned only for low-priority noise  
**Method:** read-only repository scan for hardcoded values, duplicated render/UI patterns,
large maintenance hotspots, panics, and obvious DRY violations. `cargo check --all-targets`
passed after the scan. No code changes were made for this report.

## Executive summary

The engine is already split into many focused modules, and the previous large-file split work
reduced the worst `App` and `SpriteRenderer` concentration. The remaining code-quality risks
are less about raw line count and more about **shared behavior being implemented separately**:
UI hit-testing is repeated across widget passes, egui submission is duplicated between render
paths, and several wgpu layout/pass helpers are copied with small variations.

The highest-priority items are behavioral, not cosmetic:

1. UI pointer capture is not shared across widget types, so a covered button can still fire.
2. HDR post-processing currently skips several render features because some pipelines are
   still compiled only for the surface format.
3. egui overlay submission has two near-identical implementations with different callback
   behavior.

Hardcoded values are mostly acceptable in examples and debug-only code. The clearest engine
configuration gap is `DialogueSystem`, where layout, colors, font sizes, portrait placement,
and viewport fallback are embedded directly in engine code.

## Resolution log

> Added 2026-06-23 (after the scan). Every actionable finding below has been addressed; this
> document is kept as the found→fixed record. The recommended order was followed; the only
> "won't fix" is the intentional example hardcoding.

| Finding | Priority | Resolved by |
|---|---|---|
| UI pointer capture duplicated by widget type | P1 | #219 (v0.62.1) — shared `PointerCapture` map in `UiSystem` |
| HDR post depends on surface-format pipelines | P1 | #213 (v0.59.0, material + UI format-matching) + #220 (v0.62.2, GPU particles under HDR) |
| egui overlay submission duplicated | P2 | #224 (v0.63.1) — single `egui_pass::submit_egui` |
| Dialogue UI layout/style hardcoded | P2 | #222 (v0.63.0) — opt-in `DialogueStyle` resource |
| DebugDraw fixed visual constants | P3 | #226 (v0.63.2) — named constants in `debug_draw.rs` |
| Frame-pacing policy fixed in the window layer | P3 | #226 (v0.63.2) — private `frame_pacing` constants |
| wgpu bind-group/pass boilerplate repeated | P3 | #227 (v0.63.3) — `renderer::common` helpers |
| Example hardcoding | Low | Won't fix — intentional for runnable demos (see below) |

No public API was removed or renamed in any of these changes; each is additive or a
behavior-preserving internal refactor, per the "avoid public API removal" guardrail.

## Findings

### P1 — UI pointer capture is duplicated by widget type

**Files:** `src/ui/system/button_pass.rs`, `src/ui/system/checkbox_pass.rs`,
`src/ui/system/slider_pass.rs`, `src/ui/system/text_input_pass.rs`,
`src/ui/system/scroll_pass.rs`

The button pass already documents the core issue:

```rust
// TODO: a button beneath a *different* widget type (e.g. a Panel) can still fire here;
// shared pointer-consumption across widget kinds is a broader concern left for a future pass.
```

Each widget pass performs its own entity collection and hit-test logic. Because there is no
shared "topmost interactive widget consumed this pointer" decision, one widget kind can miss
that another widget kind should have captured the click.

**Risk:** real UI behavior bug. A hidden or covered button can emit `ButtonClicked` when a
different widget is visually above it.

**Recommended fix:** add a shared pointer-capture helper owned by `UiSystem` or `InputSnapshot`.
Compute the topmost interactive/capturing UI entity once, then let widget passes query that
result. Add a regression test with a higher-z `Panel`, `TextInput`, or `ScrollView` covering a
button.

### P1 — HDR post-process still depends on surface-format pipelines

**Files:** `src/app/render/frame.rs`, `src/renderer/sprite/draw.rs`,
`src/renderer/post_process.rs`

HDR scenes render into a non-surface intermediate format, but several render paths still assume
`gpu.config.format`. The current code explicitly skips:

- UI primitive rendering under HDR post-process.
- GPU particle rendering under HDR post-process.
- `ShaderMaterial` entries when the material pipeline is not compiled for the current scene
  target format.

**Risk:** feature combinations silently disappear when HDR is enabled.

**Recommended fix:** cache format-specific pipelines keyed by `wgpu::TextureFormat`:

- UI primitive pipeline per target format.
- GPU particle pipeline per target format.
- `ShaderMaterial` pipeline cache including target format in the key.

This should follow the existing sprite pipeline pattern rather than changing public APIs.

### P2 — egui overlay submission is duplicated and can diverge

**Files:** `src/app/render/frame.rs`, `src/app/render/docked.rs`,
`src/app/egui_pass.rs`

`frame.rs` and `docked.rs` both perform the same egui renderer lifecycle: update texture delta,
update buffers, begin render pass, render, free texture delta, and submit. One path has callback
guarding while the other calls into the pass helper more directly.

**Risk:** future egui renderer changes must be applied twice; callback behavior can differ
between the final overlay path and docked preview/warm-up paths.

**Recommended fix:** move the shared submission flow into `egui_pass.rs`, with an explicit
callback policy parameter if the two paths need different behavior.

### P2 — Dialogue UI layout and style are hardcoded

**File:** `src/dialogue/mod.rs`

`DialogueSystem` currently embeds the default presentation directly in engine code:

- fallback viewport size
- portrait size and x/y placement
- text and choice offsets
- font sizes
- text, speaker, and choice colors

**Risk:** a game must edit engine source to make normal dialogue style changes. This conflicts
with the fork-friendly goal because the default behavior is useful, but the style is not an
extension point.

**Recommended fix:** add a `DialogueStyle` resource or style field with defaults matching the
current literals. Keep the current visual result as the default to avoid behavior changes.

### P3 — wgpu bind-group and render-pass boilerplate is repeated

**Files:** `src/renderer/lighting.rs`, `src/renderer/post_process.rs`,
`src/renderer/texture.rs`, `src/renderer/sprite/ui_primitives.rs`,
`src/renderer/sprite/draw.rs`, `src/app/egui_pass.rs`

Several modules define nearly identical sampled-texture + filtering-sampler bind group layouts
or render-pass descriptors. The code is not broken, but the repeated patterns increase the cost
of wgpu upgrades and make subtle changes easy to apply inconsistently.

**Recommended fix:** add private renderer helpers for common layout entries and simple
load/store render-pass descriptors. Keep these helpers internal; this is not a public API need.

### P3 — DebugDraw uses fixed visual constants

**File:** `src/app/render/debug_draw.rs`

Debug rendering uses fixed values for z depth, line thickness, minimum length, thickness floor,
and circle segments.

**Risk:** low. It is debug-only, but the output can become too thin, too thick, or too coarse
under unusual camera zooms or world scales.

**Recommended fix:** first promote the literals to named constants. If real games need control,
add a `DebugDrawConfig` resource later.

### P3 — Frame pacing policy is fixed in the window layer

**File:** `src/app/window.rs`

The refresh-rate fallback and clamp are currently fixed to a normal desktop range. The comment
explains the intent, and this is a reasonable default, but it is still a hardcoded policy.

**Risk:** low for games, moderate for editor/tool forks that may want low-power idle rendering,
30 FPS caps, or unusual display behavior.

**Recommended fix:** expose the policy through `WindowConfig` or an internal `FramePacingConfig`
with defaults matching the current behavior.

### Low priority — example hardcoding is mostly intentional

**Files:** `examples/`

Examples contain many hardcoded asset paths, websocket URLs, spawn positions, and gameplay
numbers. This is expected for runnable demos and should not be mixed with engine refactors
unless a value is copied into multiple examples as a reusable convention.

## Duplication cleanup candidates

These are good follow-up tasks after the P1/P2 behavior fixes:

1. Extract shared UI widget pass scratch buffers and pointer candidate collection.
2. Extract egui submission helper from `frame.rs` and `docked.rs`.
3. Extract renderer bind-group layout helpers for sampled texture + filtering sampler.
4. Extract render-pass descriptor helpers for common clear/load/store cases.
5. Keep RON registry wrappers thin; `src/ron_registry.rs` already covers the important shared
   pattern, so the remaining animation/particle wrappers are acceptable.

## Recommended implementation order

1. Fix shared UI pointer capture and add regression tests.
2. Make HDR render pipelines target-format-aware.
3. Consolidate egui overlay submission into `egui_pass.rs`.
4. Add configurable dialogue style defaults.
5. Do renderer boilerplate cleanup as an internal refactor.
6. Convert low-risk hardcoded debug/window values into named constants or config fields.

Avoid public API removal or renames during this pass. If a fix requires changing public API,
stop and batch it into a separately approved breaking-design step.
