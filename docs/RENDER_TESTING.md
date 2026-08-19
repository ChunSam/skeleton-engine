# Render testing — CI-verifying the GPU render path

`tests/render.rs` renders the engine's **real** render path headlessly (no window, no surface, no
display) and asserts on the read-back pixels, so a shader / pipeline / projection regression is
caught by an automated test instead of slipping through to a release.

Before this, every GPU pass (sprite / text / lighting / letterbox / post) was exercised *only* by
local macOS shell smokes (`scripts/headless_screenshot_smoke.sh`, `lighting_cap_smoke.sh`), because
CI is ubuntu-only with **no GPU**. The fix has two halves:

> ⚠️ **Those smokes were deleted on 2026-08-19** along with the examples they drove, so
> `tests/render.rs` is no longer a *supplement* to them — it is the engine's entire render
> verification, on CI and locally alike. Weigh that when deciding whether a render change needs a
> new case here.

1. **A software GPU on CI** — the `render` job in `.github/workflows/ci.yml` installs Mesa
   **lavapipe** (a CPU Vulkan driver), so `wgpu` gets an adapter and the real render path runs on
   the runner.
2. **Render tests** — `tests/render.rs` drives the path via `App::screenshot_headless` and asserts.

## How it works

- **Driver:** [`App::screenshot_headless(frames)`](../src/app/headless.rs) builds a headless
  `GpuContext` (offscreen `Rgba8UnormSrgb` target, no surface), runs the app's systems + the full
  render path per frame, and reads the final frame back as tightly-packed RGBA8. `App::new()` builds
  no winit `EventLoop`, so this is callable from a plain `#[test]`.
- **Probe + gate:** `render_or_skip(app, frames)` first probes `GpuContext::new_headless` for an
  adapter:
  - no adapter → prints `[render-test] SKIP …` and the test no-ops (green on a GPU-less box and in
    the GPU-less `test` job) — **unless** `SKELETON_REQUIRE_GPU=1`, which turns it into a **panic**.
  - adapter present → prints `[render-test] adapter=<name> backend=<…>` and renders.
- **Two CI guards** make a silent no-op impossible: the `render` job sets `SKELETON_REQUIRE_GPU=1`
  (missing adapter → hard fail) **and** greps the log for `[render-test] adapter=` (confirms a real
  render happened).

## Renderer-tolerant assertions — *not* pixel-exact goldens

A PNG golden rendered on macOS-Metal can never byte-match ubuntu-lavapipe, and a lavapipe-only golden
drifts with the runner's Mesa version (flaky + binary blobs in the repo). So the tests assert
**structural invariants**, always **relative to a sampled background pixel**, with generous
thresholds — never an absolute RGB value (sRGB store + FP/AA differ ±1 LSB across backends). This
runs identically on local Metal and CI lavapipe, yet still catches real regressions: wrong color, a
dropped pass, a blank frame, a broken letterbox. (A tolerant pixel-golden for one ultra-stable scene
could be added later; deferred.)

Current tests (see `tests/render.rs` for the authoritative list — it grows with each editor/render
feature). **Scene invariants:** `red_quad_reads_red` (sprite color + placement), `hud_text_non_blank`
(glyph pass, injects the bundled DejaVu Sans for runner-independent fonts),
`lighting_cap_lights_more_when_raised` (`LightingConfig::max_lights` drives the GPU lighting pass),
`design_resolution_letterboxes` (design-resolution scale+center projection),
`hidden_component_suppresses_sprite` (the `Hidden` marker skips the sprite pass). **Editor overlay/docked
UI** (via the headless editor-capture path — see `CLAUDE.md`): `editor_overlay_renders_headless`,
`editor_toast_renders_headless`, `editor_docked_renders_headless`, `editor_docked_inline_rename_renders_headless`,
`editor_docked_scene_tree_rename_renders_headless`, `editor_docked_scene_tree_reparent_renders_headless`.

## Running

```bash
# Local (any machine with a GPU — Metal/Vulkan/DX). Skips cleanly if no GPU is present.
cargo test --test render -- --nocapture          # prints [render-test] adapter=…

# Require a GPU (fail instead of skip) — what CI does:
SKELETON_REQUIRE_GPU=1 cargo test --test render
```

The render tests are **not** `#[ignore]`d, so they also run automatically under
`scripts/verify.sh` / `cargo test --all-targets` on a developer machine with a GPU.

## Adding a render test

1. Build an `App`, set a `WindowConfig` (size + a deliberately-distinct `clear_color`), and add the
   scene (spawn sprites / insert resources / add a system that pushes `DrawText`/`DrawRect`).
2. `let Some((w, h, px)) = render_or_skip(&mut app, frames) else { return; };`
3. Assert **relative to a sampled background pixel** with wide thresholds (use `px_rgb`,
   `region_mean`, `count_far_from`). Never assert an absolute RGB.
4. If the scene draws text, insert `FontData(DejaVuSans bytes)` **before** rendering — on native an
   empty `FontData` falls back to the runner's (sparse) system fonts.

## The lavapipe CI job

ubuntu-latest, apt `mesa-vulkan-drivers libvulkan1 vulkan-tools` (+ the usual build deps). The
lavapipe ICD path is globbed into `$GITHUB_ENV` as `VK_ICD_FILENAMES` (the filename varies by Mesa
version). The engine calls a raw `instance.request_adapter` with `Backends::all()` — it does **not**
honor `WGPU_BACKEND` / `WGPU_ADAPTER_NAME` — so the adapter is constrained at the **Vulkan-loader**
level, not via wgpu env vars. No GL software driver is installed, so lavapipe is the only adapter
wgpu can pick; no `xvfb` is needed (the headless path is surfaceless). A `vulkaninfo --summary | grep
llvmpipe` step fails fast if the ICD didn't resolve.

**If `request_device` ever fails as "too old"** on the runner's stock Mesa, add the newer-Mesa PPA
before the install: `sudo add-apt-repository -y ppa:kisak/kisak-mesa && sudo apt-get update`.

## Scope notes

- Keep render tests off the **GPU-particle** path — it's the only consumer of compute + `STORAGE`
  buffers and only activates with a `GpuParticleEmitter` (which the test scenes don't spawn). lavapipe
  supports compute, so a GPU-particle render test is a viable future stretch, not part of the core gate.
- The egui pass is inert headless (`submit_egui` no-ops without a `DebugUi`).
