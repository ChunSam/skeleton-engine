# Headless screenshot mode: render to PNG with no window/display (+ HEADLESS_SHOT example env var), proven by a remote monitor-off test

**Date:** 2026-06-28
**Status:** COMPLETED
**Bead(s):** none (bd unavailable in this repo)
**Epic:** none
**Chain:** `headless-screenshot` seq `1`
**Parent:** none (new work stream)
**Related handoffs:** `HANDOFF_hardcoding-audit_2026-06-27.md` (the session this conversation *began* as — its Tier-2 work `#255–#258` is already captured + merged; that session's inability to screenshot GPU features with the display locked is exactly the gap this feature closes).

---

## Since Last Handoff

This conversation started as `hardcoding-audit` seq 2 ("tier2 진행"): the physics/timing Tier-2 bundle (`#255` one_way_tolerance v0.72.0, `#256` set_solver_iterations v0.73.0, `#257` FrameConfig::max_dt v0.74.0) + the wrap docs (`#258`). **All of that is already done, merged, and handed off** in `HANDOFF_hardcoding-audit_2026-06-27.md`. Do NOT re-handoff it.

After that handoff merged, the user asked a question that kicked off a **new** work stream and this handoff:
1. "클라우드 세션에서 진행하면 스크린샷 테스트 가능한지 확인해줘" → I researched + proved (a throwaway surfaceless wgpu probe) that headless offscreen Metal render works on this Mac.
2. "3을 구현하면 모니터가 꺼져있어도 테스트가 가능해?" → confirmed yes (offscreen has no display dependency), proven with the probe.
3. "진행해줘, 헤드리스 스크린샷 모드 엔진에 추가" → built it (`#259`, v0.75.0).
4. "현재 맥북 잠금 상태로 모니터 꺼져있고 모바일에서 원격 접속 중 … 기존 디스플레이 필요하던 테스트를 새 기능 사용해서 테스트 진행" → used it for real: captured the previously-unscreenshottable examples with the monitor off + shipped the `HEADLESS_SHOT` env var (`#260`, v0.75.1).

## The Goal

Give the engine a way to render a frame to a PNG **with no window, no surface, and no display**, so GPU rendering can be pixel-verified with the monitor off/asleep/locked and from a remote session — the exact case that blocked the `solver_iterations`/`frame_dt_cap` visual playtests last session (macOS `screencapture`/`osascript` need the screen on + unlocked). Then prove it by using it remotely.

## Where We Are

- **main @ `5f80193`** (v0.75.1), CLAUDE.md header **v1.6.159**, tree **clean**, **no open PRs**.
- **Two PRs merged this work stream** (squash, branches deleted, CI 4/4 each):
  - **`#259`** (`b28ca1c`, v0.75.0 MINOR) — headless screenshot mode (the engine feature).
  - **`#260`** (`5f80193`, v0.75.1 PATCH) — `HEADLESS_SHOT` env var on 3 examples (example-only, no library change).
- Memory `engine-current-state.md` bumped to **seq 105–106**; description + body header + recent-seqs list updated to main @ `5f80193` / v0.75.1.
- The downstream wishlist board (`../dungeon-merchant/docs/engine-wishlist.md`) remains **ACTIVE EMPTY** (next free ID EW-004).

## Public API added (`#259`, native-only, additive)

- `App::save_screenshot_headless(frames, path) -> Result<(), String>` — renders `frames` frames headlessly and saves the final frame as a PNG. No `image` dep needed by the caller.
- `App::screenshot_headless(frames) -> (u32, u32, Vec<u8>)` — same but returns tightly-packed sRGB RGBA8 (row stride `w*4`).
- `GpuContext::new_headless(width, height)` — surfaceless GPU context (adapter requested with `compatible_surface: None`), renders into an offscreen `Rgba8UnormSrgb` color texture (`RENDER_ATTACHMENT | COPY_SRC`).
- `GpuContext::read_headless_rgba() -> (u32, u32, Vec<u8>)`, `headless_view()`, `is_headless()`.
- `App::init_gpu_renderers(&GpuContext)` — `pub(in crate::app)`; factors the window-independent renderer init (sprite/text renderers, pre-GPU render targets, `RenderCapabilities`) out of `finish_init`, shared with the headless path.
- **Example interface (`#260`):** `solver_iterations`/`one_way_tolerance`/`frame_dt_cap` accept `HEADLESS_SHOT=<path>` (+ `HEADLESS_FRAMES=<n>`) → screenshot headlessly and exit instead of `app.run()`. No-op unless set.

## What We Tried (Chronological)

1. **Researched the cloud/headless question against the repo, not from memory.** Found: the existing "headless" smokes (`bloom_web_smoke.sh`, `centered_text_smoke.sh`, …) render via **headless Chrome + SwiftShader WebGL2** and either read a `document.title` verdict or capture a PNG — but that's the **wasm/WebGL** path, not native. The native render path (`frame.rs`) targets the surface and `GpuContext::new` requires a `&Window`.
2. **Proved the primitive first.** Wrote a throwaway `examples/headless_probe.rs`: `Instance::new` → `request_adapter(compatible_surface: None)` → offscreen `Rgba8Unorm` texture → clear pass → `copy_texture_to_buffer` → `map_async` + `poll(wait)` → PNG. **Ran on this Mac → `ADAPTER: Apple M1 Pro | backend=Metal`, valid 64×64 PNG, readback pixel correct.** Deleted the probe after. This de-risked the whole feature.
3. **Scoped the real change** by grepping every `surface`/`config` use: almost everything reads `gpu.config.{format,width,height}` (not `surface` directly) — only `frame.rs:115` (acquire), `:559` (present), `docked.rs:21`, and `context.rs` (configure/clear) touch `surface`. So: make `surface` `Option`, keep `config` populated in headless mode, branch the few direct uses.
4. **`#259` implementation** (`GpuContext` surfaceless → render() branch → factor init → App methods → example + smoke). Details in Key Decisions + Files.
5. **Verified `#259` two ways I *could* in a locked/asleep state:** the `headless_screenshot` example produced a correct PNG (3 quads + text) AND — critically — I re-ran a **windowed** example (`basic`) and screenshotted its window to confirm the render() acquire/present refactor didn't break windowed rendering (cyan rotating sprite rendered fine).
6. **`#260` + the real remote test.** Added `HEADLESS_SHOT` to the 3 physics/timing examples; ran them with the **MacBook locked + monitor off** (confirmed: `screencapture` returned an all-black frame); captured 3 PNGs; **sent them to the user's mobile via `SendUserFile`**. `solver_iterations` is the headline: the 2-iter chain visibly sags (links gapped, stretch 1.07) vs the taut 16-iter chain (links flush, 0.02) — the exact shot blocked in the prior session.

### Debugging notes from the build
- **The probe needed 2 wgpu-29 fixes before it ran:** `RenderPassDescriptor` missing `multiview_mask`, and `PollType::Wait` is a struct variant (compiler suggested `{submission_index, timeout}`) → switched to `PollType::wait_indefinitely()`. I verified the exact wgpu-29 type names from the crate source (`~/.cargo/registry/.../wgpu-29.0.3/src`) before writing, which caught `TexelCopy*` (not `ImageCopy*`) and `Instance::new` by-value up front.
- **The smoke's PNG check took 3 tries:** `head -c8 | grep "PNG"` (grep on binary unreliable) → `od -An -tx1 -N4 | grep "89 50 4e 47"` (failed — `od` uses *double*-space separators, my pattern had single) → `file "$SHOT" | grep "PNG image"` (works). Lesson: use `file`, not hand-rolled magic-byte matching.
- **Windowed re-verification specifics:** ran `cargo run --example basic`, positioned its window at `{150,120}` size `{960,540}` via `osascript`, `screencapture -R150,120,960,575` → confirmed the cyan rotating sprite renders centered (the render() refactor didn't regress the windowed path). An earlier `hello_sprite` attempt mis-cropped (window larger than the capture region); `basic` (textureless colored sprite, fixed 960×540) was the cleaner check.
- **`screenshot_headless` rebuilds a fresh `GpuContext` each call** and installs it on the app — call it **instead of** `run()`, not after (it's documented on the method).

## The render() branch (read before editing the render path)

`render()` (`src/app/render/frame.rs`, now `pub(in crate::app)`) is the ~570-line hot path. The headless change is two surgical branches; the entire middle (offscreen RTs, scene pass, HDR post, lighting, fade, render plugins, egui) is **unchanged** and works off `final_view`/`scene_target`:

- **Acquire** (was `gpu.surface.get_current_texture()`): now
  ```rust
  let (surface_frame, suboptimal, final_view) = match &gpu.surface {
      Some(surface) => { /* get_current_texture → Success/Suboptimal/Err; create_view; (Some(frame), subopt, view) */ }
      None => (None, false, gpu.headless_view()),
  };
  ```
  The `Err(e) => return Err(e)` arm only exists on the windowed branch; headless never returns an error from acquire.
- **Present** (was `frame.present(); if suboptimal { gpu.reconfigure() }`): now
  ```rust
  if let Some(window) = &self.window { window.pre_present_notify(); }
  if let Some(frame) = surface_frame { frame.present(); if suboptimal { gpu.reconfigure(); } }
  ```
  Headless: no window (`self.window` is `None`), no present, no reconfigure — the frame stays in the offscreen texture for read-back.

`docked.rs::present_docked_placeholder` got a `let Some(surface) = &gpu.surface else { return Ok(()) };` guard — the docked editor is windowed-only, so headless never reaches it; the guard is just to compile with `surface: Option`.

## Key Decisions

- **`GpuContext.surface` → `Option`, not a separate headless context type.** `config` stays populated in both modes, so the ~30 `gpu.config.*` read sites are untouched; only the acquire/present/configure/clear branch. `GpuContext` is **not re-exported** at the crate root (`grep` confirmed) → the field-type change is **not a public-API break** (so still a clean MINOR for the additive API).
- **Offscreen format = `Rgba8UnormSrgb`** so the read-back bytes are directly RGBA8 sRGB for a PNG — no channel swizzle (a `Bgra8` surface would need a swap), and an sRGB target matching the usual surface encoding. The renderers are built with `gpu.config.format`, so the offscreen texture MUST be that same format.
- **Reuse the REAL render path, not a parallel renderer.** `render()` was made `pub(in crate::app)` and branches its frame acquire (surface texture vs `headless_view()`) + present (present vs no-op). This guarantees the screenshot matches what the window draws (offscreen RTs, HDR post, lighting, fade, plugins all run). The alternative — a slim headless renderer — would duplicate orchestration and drift.
- **Factor `init_gpu_renderers` out of `finish_init`.** The renderer init (sprite/text/RT/`RenderCapabilities`) is window-independent; egui/IME need a window and **stay** in `finish_init`. Headless calls only the factored part. `update` is already `pub(super)` (visible in the `app` subtree), so the headless loop can call it.
- **Row-padding in read-back.** `copy_texture_to_buffer` requires 256-byte-aligned `bytes_per_row` → `padded = unpadded.div_ceil(256)*256`; read-back unpads each row back to `w*4`. (For w=480/1280 it happens to already be aligned, but the code handles arbitrary widths.)
- **`#260` is example-only → PATCH (v0.75.1).** No library change; the env branch is a no-op unless `HEADLESS_SHOT` is set, so `cargo run --example …` is unchanged.
- **Did NOT block the merge on a fully-green local verify** for `#260`: 2 audio-device tests fail in the locked/remote state (no audio device). Confirmed environmental (fail identically on clean main, pass on CI) → verified all 945 non-audio tests + doc gate locally, let CI gate the audio tests. This is the "match verification to what's checkable" principle, not a narrowed bar.

## Evidence & Data

### Commits landed (main)
| Hash | PR | Bump | Summary |
|---|---|---|---|
| `b28ca1c` | #259 | 0.74→0.75.0 MINOR | headless screenshot mode |
| `5f80193` | #260 | 0.75.0→0.75.1 PATCH | HEADLESS_SHOT env var on 3 examples |

### CI (all 4/4 SUCCESS)
| PR | Test (native) | WASM | Rustdoc | Package |
|---|---|---|---|---|
| #259 | 3m37s | pass | pass | pass |
| #260 | 4m56s | pass | pass | pass |

### Headless render results (captured with the monitor OFF)
- Probe: `ADAPTER: Apple M1 Pro | backend=Metal | type=IntegratedGpu`; readback first pixel `[51,153,229,255]` vs expected `[51,153,230,255]` (off-by-1 = sRGB rounding).
- `headless_screenshot` example: 480×320 PNG, **32,558 non-background pixels**, `HEADLESS_SCREENSHOT: PASS`. Visual: 3 overlapping quads (correct z-order) + title + centered subtitle.
- `solver_iterations` headless (600 frames): left "low — 2 iters, stretch: 1.07 links" (links visibly gapped/sagging), right "high — 16 iters, stretch: 0.02 links" (links flush/taut). Heavy red weight hangs much lower on the left.
- `one_way_tolerance` headless (120 frames): player resting on the one-way platform, HUD "grounded: yes".
- `frame_dt_cap` headless (120 frames): both markers + HUD render (note: in headless, `dt` passed is a fixed `1/60` and the system's Instant-based `real` is tiny, so the cap's real-time divergence isn't shown — it just proves the example renders).
- Monitor-off proof: `screencapture -x` returned an **all-black** frame at capture time.

### The environmental audio-test failure (IMPORTANT, recorded)
- `cargo test --all-targets` fails 2 tests in the locked/remote macOS state: `audio::tests::play_tone_reports_playing_then_finished_when_audio_device_exists` and `audio::tests::stop_on_drained_sink_is_immediate` (no usable audio output device).
- **Confirmed environmental:** `git stash` → both fail identically on clean `main`; `#260` CI native test (ubuntu) passed → they pass on CI.
- Workaround to verify a non-audio change in that state: `cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate` (all 945 others pass) + run the doc gate manually, then rely on CI for the audio tests.

## Gotchas & Discoveries

- **Offscreen render has ZERO display dependency.** `compatible_surface: None` + no window → nothing touches the windowing system; the GPU renders to memory regardless of monitor power/lock (same reason GPU compute/video-encode run with the screen off). This is *why* the feature works monitor-off; the prior session's failures were entirely in the window-capture tooling, not rendering.
- **wgpu 29.0.3 API specifics** (the engine does no texture read-back elsewhere, so these were newly needed — verify against the crate source, names changed across versions):
  - `PollType::Wait` is a **struct variant** (`{submission_index, timeout}`) → use the helper `wgpu::PollType::wait_indefinitely()`.
  - `RenderPassDescriptor` needs `multiview_mask: None`.
  - Copy types are `TexelCopyTextureInfo` / `TexelCopyBufferInfo` / `TexelCopyBufferLayout` (the old `ImageCopy*`/`ImageDataLayout` names are gone).
  - `Instance::new` takes the descriptor **by value** (not `&`).
  - `device.poll(...)` returns `Result<PollStatus, PollError>` → `.unwrap()`; `request_adapter` returns `Result` (`.expect`).
- **`IntegrationParameters` is NOT `#[non_exhaustive]`** (carried over from the prior Tier-2 work) → struct-update syntax satisfies clippy `field_reassign_with_default`.
- **The backgrounded `verify.sh > log; echo $?` gotcha bit AGAIN this session** — a background task's reported exit is the trailing `echo`'s, masking real failures. **Always grep the LOG for `all checks passed` / errors**, never trust the task exit on a `…; echo` command. (Also caught a real clippy fail on the Tier-2 PR2 earlier.)
- **macOS screen lock vs display sleep both block window-capture** (`screencapture` returns the lock screen or black; `osascript` can't position an off-screen window) and can't be undone headlessly. The offscreen path sidesteps all of it.
- **`SendUserFile` is the right delivery for a remote/mobile user** — render headlessly, then push the PNGs to their phone.

## Files Changed

### #259 (headless screenshot mode)
- `src/renderer/context.rs` — `surface: Option`, `headless_color: Option<Texture>`, `new_headless`, `create_headless_color`, `is_headless`, `headless_view`, `read_headless_rgba`; branched `resize`/`reconfigure`/`clear`.
- `src/app/render/frame.rs` — `render()` made `pub(in crate::app)`; branch frame acquire (surface vs `headless_view`) + present (present+reconfigure vs no-op).
- `src/app/render/docked.rs` — guard `present_docked_placeholder` for `surface: None` (docked is windowed-only; just compiles).
- `src/app/window.rs` — extracted `init_gpu_renderers(&GpuContext)`; `finish_init` calls it then does egui/IME.
- `src/app.rs` — `#[cfg(not(wasm32))] mod headless;`.
- `src/app/headless.rs` — **new**: `App::screenshot_headless` / `save_screenshot_headless`.
- `examples/headless_screenshot.rs` — **new** (renders 3 quads + text, self-checks non-blank, never opens a window).
- `scripts/headless_screenshot_smoke.sh` — **new** (native GPU render smoke, no display/Chrome).
- `CLAUDE.md` — app-entry module-map row.

### #260 (HEADLESS_SHOT env var)
- `examples/{solver_iterations,one_way_tolerance,frame_dt_cap}.rs` — `HEADLESS_SHOT`/`HEADLESS_FRAMES` branch before `app.run()`.

### Paperwork (both PRs)
- `Cargo.toml`/`Cargo.lock` 0.74.0→0.75.0→0.75.1; `docs/CHANGELOG.md` (0.75.0 + 0.75.1); `CLAUDE.md` header v1.6.157→v1.6.159.
- Memory `engine-current-state.md` — seq 105–106.

## User Feedback & Preferences

- Asked the capability question ("can a cloud session screenshot test?") expecting **verification, not assertion** — I wrote a probe and proved it on-device before answering. This user consistently values "확인해줘" = actually check/run it.
- Chose **"Ship (PR로 머지)"** for the `HEADLESS_SHOT` enhancement (offered ship / extend-to-more-examples / revert).
- Standing prefs honored: user-facing reports in **Korean**, code/docs/PRs in **English**; merge authority delegated (squash on green CI); `cargo fmt` before verify; never trust a masked gate exit.

## Where We're Going

1. **This handoff lands as a `docs(handoff)` PR** (branch `docs/handoff-headless-screenshot`, no package bump) — then session closes.
2. **Next session: read `../dungeon-merchant/docs/engine-wishlist.md` FIRST** (ACTIVE EMPTY, EW-004 next). New EW → VISION feature+example loop. Still empty → ASK.
3. **Natural follow-ups for the headless feature** (offer if the user wants more):
   - Add `HEADLESS_SHOT` to more visual examples (bloom, particles_showcase, dialogue_*, tilemap/iso/hex) for broader golden-image coverage.
   - **CI golden-image tests**: run the headless path on CI with a software adapter (Mesa lavapipe / `force_fallback_adapter: true`) + Xvfb-free (no window needed) → compare against committed reference PNGs. This would make GPU rendering CI-verifiable for the first time (currently CI only compiles GPU code).
   - Scripted input injection (set `InputState` programmatically) so interactive examples can be captured in a specific state, not just their settled state.
4. **Remaining Tier-2 hardcoding knobs** (`docs/HARDCODING_AUDIT_2026-06-26.md` "Open — Tier 2 (remaining)"): `desired_maximum_frame_latency`, StickNav deadzone (`UiConfig`), `SLIDER_STEP_FRAC`, network `READ_TIMEOUT`, editor `APP_ID`; hardest `MAX_LIGHTS` (dynamic WGSL uniform array).

## Risks & Blockers

- **None blocking.** Tree clean, both merges green, no open PRs.
- The headless GPU **runtime** path is NOT CI-verified (ubuntu CI has no GPU) — the local `headless_screenshot_smoke.sh` is the gate, like every GPU smoke in this repo. A future edit to the headless render/read-back path needs a local GPU smoke.
- In a **locked/remote macOS session, audio runtime tests cannot pass locally** (no audio device). Don't mistake this for a regression; verify with the `--skip` pattern + CI. (Also: audio output remains ear-only / CI-unverifiable — the standing judgment gate.)

## Quick Start for Next Session

```bash
# 1. Downstream board FIRST (standing directive)
cat ../dungeon-merchant/docs/engine-wishlist.md   # ACTIVE EMPTY? EW-004 next → ASK if empty

# 2. Confirm state
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -4        # tip = this handoff merge, above 5f80193 (#260)
git status -s               # clean

# 3. Headless screenshot any example (works monitor-off / remote):
HEADLESS_SHOT=/tmp/out.png cargo run --example solver_iterations          # or one_way_tolerance / frame_dt_cap
cargo run --example headless_screenshot /tmp/out.png                       # the dedicated demo
scripts/headless_screenshot_smoke.sh                                       # native GPU smoke (no display/Chrome)

# 4. Verify (in a locked/remote session, audio tests fail environmentally):
cargo fmt && ./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"
#   If only the 2 audio-device tests fail (locked/remote, no audio device): re-run with
#   cargo test --all-targets -- --skip play_tone_reports_playing_then_finished_when_audio_device_exists --skip stop_on_drained_sink_is_immediate
#   then run the doc gate; let CI gate the audio tests. (BACKGROUNDED verify must NOT append `; echo` — read the LOG.)
```

---

## Session Closed

**Closed at:** 2026-06-28 (KST)
**Commit:** lands via a `docs(handoff)` PR (this file).
**Session status:** Handed off — headless screenshot mode (`#259`, v0.75.0) + `HEADLESS_SHOT` example env var (`#260`, v0.75.1) merged to `main`; proven by a real monitor-off remote capture delivered to the user's mobile; memory bumped to seq 105–106. (The earlier Tier-2 work in this same conversation is captured in `HANDOFF_hardcoding-audit_2026-06-27.md`.)
