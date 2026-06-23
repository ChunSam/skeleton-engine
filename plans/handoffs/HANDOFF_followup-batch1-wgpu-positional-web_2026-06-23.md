# Follow-up Batch 1 — `wgpu` re-export + `positional_audio` shipped to the web (v0.57.0)

**Date:** 2026-06-23
**Status:** COMPLETE + merged. main @ `58694eb` (code #210), package **v0.57.0**, clean tree, full gate green.
**Chain:** post-umbrella follow-up run (parent: `HANDOFF_p1-p4-carried-run_2026-06-23.md`, the P1→P4 carried run, seq 12). This is **Batch 1 of 2** of the user-selected follow-up work.
**Auto:** false (user `/goal`: do P2+P3 as batch 1 → handoff → merge; then P1→P5→P4 as batch 2 → final handoff → merge → completion report; intermediate reports minimized).

> The umbrella's "Where We're Going" listed five follow-up candidates. The user asked for a priority-ordered list, then chose: **batch 1 = P2 + P3** (low-risk quick wins), **batch 2 = P1 → P5 → P4** (the HDR/render arc). This doc covers batch 1.

---

## What shipped (this batch)

| Item | What | Files |
|---|---|---|
| **P3** | **`pub use wgpu;`** at the crate root | `src/lib.rs` |
| **P2** | **`positional_audio` shipped to the web** (web/ harness over the existing wasm entry) | `examples/positional_audio/web/{build.sh,index.html}` |

### P3 — `wgpu` re-export

`pub use wgpu;` in the convenience re-exports block (`src/lib.rs`). Rationale: `wgpu::TextureFormat`
**already** appears in the public API — `App::create_render_target_with_format` and
`App::load_image_with_format` both take one (shipped in the carried P4/seq-67). So a game already had
to match the engine's `wgpu` version; the re-export just removes the redundant *direct* `wgpu`
dependency. The doc comment states the semver implication explicitly: because `wgpu` is part of the
public surface, a `wgpu` **major** bump is a breaking change for games that name re-exported types —
but that coupling already existed; this only makes it usable without a second dep.

- **Gotcha (rustdoc, carried from the carried-run):** `App` is re-exported at the crate root
  (`src/lib.rs:82`), so an explicit-path intra-doc link `[`App::foo`](crate::app::App::foo)` trips
  `redundant_explicit_links` under `-D warnings`. Use the **shorthand** `[`App::foo`]`. (Isolated the
  doc gate first — `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps` exit 0 — before piling P2 on top.)

### P2 — `positional_audio` to the web

The example **already had** its `#[cfg(target_arch="wasm32")] #[wasm_bindgen] pub fn
run_positional_audio()` entry point (and a comment saying a `web/` harness "would call this") — so
P2 was harness-only: `build.sh` (`cargo build --release --example` + `wasm-bindgen --target web`) and
`index.html` (canvas `id="game-canvas"` tabindex=0, Start button unlocks `AudioContext` + focuses the
canvas, `?autostart=1` for a headless render smoke). **Modeled on `examples/audio_facade/web/`**
(the closest sibling — same `Audio` facade, same canvas+Start+autostart shape). `pkg/` is gitignored.

- **No headless smoke added** (deliberate): `audio_facade` set the precedent of an audio web example
  shipping without a dedicated smoke (positional is by-ear / visual-readout). `?autostart=1` is wired
  for a future render smoke. The example-level gate is the wasm **bundle build**; native real-play was
  verified at the P2 carried landing (seq-9).

## Verification

- `./scripts/verify.sh` → **exit 0** (read from `/tmp/verify_b1.log`, not piped): fmt / clippy
  `-D warnings` / wasm lib+bins / `test --all-targets` / `cargo doc -D warnings`.
- `cargo build --release --example positional_audio --target wasm32-unknown-unknown` → exit 0;
  `examples/positional_audio/web/build.sh` emits `pkg/positional_audio.js` + `_bg.wasm`.
- **CI (#210):** Build (WASM) / Rustdoc / Test (native) / Package dry-run — all pass. Squash-merged.

## Batch 2 scoping (already done — read before starting it)

While batch 1's CI ran, I read the render path to scope batch 2 (P1→P5→P4). **Key findings that shape
batch 2** (recorded here so the next step doesn't re-derive them):

- **The main render path is uniformly `gpu.config.format` (surface).** `src/app/render/frame.rs`
  draws the scene into `render_view` — which is `post_renderer.target_view` (the post-process
  intermediate) when post-process is enabled — via the sprite pass (step 2), UI-primitive pass
  (step 2.7, `sr.render_ui_primitives_from_slices`), GPU-particle pass (step 2.8, native), and render
  plugins (step 3). **Text is drawn AFTER post-process** (step 4.7) directly onto `scene_target`, so
  text is NOT in the post intermediate.
- **The post-process intermediate texture is created with the surface format** (`post_process.rs`,
  `create_target(..., surface_format)`), so over-bright values are clamped at store time — a real
  tonemap needs the intermediate to be **HDR (`Rgba16Float`)**.
- **Sprites already format-match** via the carried-P4 cache (`SpriteRenderer::extra_sprite_pipelines`,
  `ensure_sprite_pipeline` / `sprite_pipeline_for(format)`, free fn `build_sprite_pipeline`,
  `src/renderer/sprite.rs:359-388`). **UI-primitive, material (`MaterialRenderer`), and GPU-particle
  pipelines do NOT** — they're surface-only (the carried-P4 CHANGELOG says so).

- **⇒ P1 does NOT need a main-path HDR rewrite, and the user's order P1→P5→P4 stands (no reorder).**
  My stated P1 was "*the engine directly tonemaps the HDR RT*". The bounded, faithful implementation:
  **make the post-process intermediate optionally `Rgba16Float` + add a tonemap operator + exposure to
  the post shader.** A **sprite-only** over-bright example works with just the existing sprite cache.
  Plan (designed to code level):
  1. `Tonemap` enum (None/Reinhard/AcesFilmic), public, `#[non_exhaustive]`, `Default=None`.
  2. `PostProcessConfig` += `hdr: bool` (default false), `exposure: f32` (default **1.0**),
     `tonemap: Tonemap` (default None) → defaults are byte-identical.
  3. `PostProcessUniforms`: **repurpose the existing `pad0: vec2` → `exposure: f32` + `tonemap: u32`**
     (size stays 48 B / 16-B aligned; mirror in `post_process.wgsl`).
  4. `PostProcessRenderer`: split **`intermediate_format`** (target texture, HDR when `hdr`) from
     **`output_format`** (pipeline color target, stays surface). Add `intermediate_format()`. The bind
     group's sample type is already `Float { filterable: true }` (works for `Rgba16Float`).
     `setup_post_renderer` must **reconfigure** when the desired intermediate format changes (hdr
     toggled).
  5. `post_process.wgsl`: order = sample (chroma) → **× exposure** → bloom/vignette (in HDR) →
     **tonemap (Reinhard / ACES filmic)** → existing grade → clamp.
  6. `src/app/render/frame.rs`: compute `scene_format = if !docked && use_post {
     pr.intermediate_format() } else { gpu.config.format }`; pass it to the sprite + render-plugin
     passes (they key pipelines off `ctx.format`). **Also format-match the UI-primitive pipeline** in
     `SpriteRenderer` (small, mirrors the sprite cache; both share `sprite_pipeline_layout`) so
     debug-draw / UI don't mismatch in HDR mode. **GPU particles + shader-materials in an HDR post
     scene are out of scope for P1** (document it; particles are native+niche, materials are P5's
     concern) — the P1 example has none, so their `if !empty` / `if has_emitters` guards skip them.
  7. Example `tonemap` (sprite-only over-bright scene; toggle hdr + cycle None/Reinhard/ACES +
     exposure ±) — without tonemap the bright sprites clip to white; with ACES they roll off smoothly.

- **P5 = `MaterialRenderer` per-format pipeline cache + the offscreen path renders materials into the
  RT format.** NB: `src/app/render/offscreen.rs` currently renders **sprites only** (`sr.render`,
  `format: rt_format`). Confirm where `MaterialRenderer::render` is invoked (main path vs offscreen)
  before scoping — P5 likely also has to *add* material rendering to the offscreen pass, not just
  format-match it.
- **P4 = ship `hdr_render_target` to the web** — **VALIDATE `Rgba16Float` as a render target on the
  wasm WebGL2 backend** (`wgpu` is built with the `webgl` feature; float-renderable needs
  `EXT_color_buffer_float`). Add a fallback (e.g. detect + fall back to 8-bit, or document) if
  unsupported.

## Where We're Going

- **Batch 2 (P1 → P5 → P4)** is the next and final work; scoping above. Each lands as its own code PR
  (per-feature MINOR bump + example + native GPU smoke — CI can't verify GPU), then ONE final wrap-up
  handoff covers all three, merge, then the Korean completion report.
- Verify each render feature with a **native real-play smoke** (windowed app + screenshot on macOS);
  green CI does NOT verify GPU output.

## Related

- Parent: `HANDOFF_p1-p4-carried-run_2026-06-23.md` (the carried P1→P4 run; same naming collision on
  "P1..P4" — note this follow-up's P1/P4/P5 are a DIFFERENT numbering, the umbrella's five
  "Where We're Going" candidates).
- Code PR: #210 (`58694eb`).
