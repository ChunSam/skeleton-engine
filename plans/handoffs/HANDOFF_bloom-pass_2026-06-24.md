# Real multi-pass bloom post-process pass

**Date:** 2026-06-24
**Status:** COMPLETED (code PR #231 merged; this handoff lands as its own `docs(handoff)` PR)
**Bead(s):** none (`bd` unavailable; engine work tracked via the dungeon-merchant wishlist board)
**Epic:** none — a user-chosen feature from the empty-queue "Where We're Going" offers
**Chain:** `bloom-pass` seq `1` (new chain — NOT a continuation of `patterns-doc`, which is COMPLETE)
**Parent:** none
**Next:** nothing queued. Wishlist board ACTIVE is empty (next free ID **EW-004**) → next session read `../dungeon-merchant/docs/engine-wishlist.md` FIRST, then ASK for direction if still empty.

---

## The Goal

The session started on a clean+green `main` with the queue genuinely empty (wishlist ACTIVE empty, `patterns-doc` chain done). Per the standing rule I read the board first, confirmed it empty, and ASKED the user for direction, offering the un-queued candidates the prior handoff left (a real bloom pass; an engine-level format-renderability query; hearing the audio web demos). The user picked **Bloom 패스 (recommended)**.

Deliverable: **a real multi-pass bloom post-process effect** — replacing (when opted in) the cheap inline 4-tap approximation the post shader has always done — shipped with a playable example (the VISION acceptance test).

## Where We Are

- **main @ `397649d`, package v0.64.0, CLAUDE.md header v1.6.139, clean + green, no open PRs.** Code PR **#231** merged (squash, `feat(renderer)`); branch deleted; main synced.
- **New feature, fully wired + verified.** A genuine bloom pass: bright-pass → half-res separable Gaussian blur (ping-ponged) → additive composite onto the scene intermediate, before post-process.

## Public API surface

Additive (→ MINOR). Two new fields on the already-public `PostProcessConfig`:

- `bloom: bool` — opt into the real multi-pass bloom. Default `false` ⇒ the cheap inline 4-tap runs (OFF path **byte-identical** to pre-0.64.0). Requires `enabled: true`.
- `bloom_iterations: u32` — number of separable blur iterations (each = one H + one V pass); glow width. Clamped to `0..=8`, default `4`.

`BloomRenderer` is `pub(crate)` (internal — no new public type). `bloom_threshold` / `bloom_intensity` (pre-existing) now drive both the inline and the real bloom.

## How it works (the shipped design)

**Pipeline position:** the bloom pass runs only when `use_post && cfg.bloom`, on the scene intermediate (`render_view` = `PostProcessRenderer::target_view`), as **Step 3.5** in `src/app/render/frame.rs` — after the scene/UI/particle/render-plugin passes (Step 2–3), before the post-process composite (Step 4).

**Three stages** (`src/renderer/bloom.rs` + `src/renderer/shaders/bloom.wgsl`), all sharing one bind-group layout (input texture + sampler + uniform):
1. `fs_prefilter` — bright-pass: keep luminance above `bloom_threshold` (soft knee, same shape as the old inline 4-tap). Scene → texture A (half res).
2. `fs_blur` — separable 9-tap Gaussian, one axis per pass, ping-ponged A→B→A `bloom_iterations` times. Result always lands in A.
3. `fs_composite` — additive blend (`One,One` color; `Zero,One` alpha to preserve scene alpha) of A onto the scene intermediate (`LoadOp::Load`).

**Post shader interplay:** `PostProcessUniforms` gained a `bloom_enabled: u32` flag; the post shader skips its inline 4-tap when `bloom_enabled == 1` (the real pass already composited the glow) — avoids double bloom. `make_uniforms` sets it from `cfg.bloom`.

**Format-matching (key decision):** the bloom pipelines/textures are built for the **scene intermediate format** (`Rgba16Float` under HDR, else surface), so bloom works under HDR post. This is **NOT** the per-target-format pipeline *cache* pattern (sprite/material/UI/GPU-particle `HashMap<TextureFormat, Pipeline>`). Bloom — like the post-process and lighting renderers — has **one target per frame**, so a format change just **rebuilds the renderer** (`reconfigure`, mirroring `setup_post_renderer`/`setup_lighting`). So it is **NOT the "5th instance"** that the `patterns-doc` PATTERNS section says would trigger abstracting the cache — different pattern (single-target reconfigure vs multi-target-per-frame cache). Recorded in the bloom module docs + the CLAUDE.md row.

**Wiring files:** `setup_bloom_renderer` in `src/app/render/post_lighting.rs` (lazy create / resize / reconfigure-on-format-change); `bloom_renderer` field on `RenderState`; setup call + Step 3.5 in `frame.rs`. Cross-platform (un-gated; post-process runs on native + wasm).

## What We Tried (process)

1. **Onboarding** — read the prior handoff, the wishlist board (empty), the engine-state memory; confirmed clean+green `main @ ec2b0fa` v0.63.3. ASKED for direction → user chose bloom.
2. **Read the pipeline** — `post_process.rs` (found the existing "bloom" is a fake inline 4-tap in `post_process.wgsl`, not a real pass), `frame.rs` (Step 2–5 sequence, `render_view`/`scene_format`/`use_post` derivation), `post_lighting.rs` (`setup_*` reconfigure pattern), `renderer/common.rs` (the shared pass/sampler/layout helpers), `tonemap.rs` (example template).
3. **Implemented** the shader + renderer + config fields + post-shader flag + wiring + example + CLAUDE.md row + 3 unit tests.
4. **Verify gate green** (`VERIFY_EXIT=0`) — twice (before and after the `/ship` bump).
5. **Native GPU smoke caught a real bug** (see Gotchas) — fixed — re-ran clean.
6. **Landed via `/land-pr`** → `/ship` v0.64.0 → PR #231 → CI 4/4 green → squash-merge.

No dead ends in the design; the one defect was the WGSL layout drift, caught by the smoke.

## Key Decisions

- **Real bloom is opt-in (`bloom: false` default), inline 4-tap retained.** Keeps the OFF path byte-identical (non-breaking) and leaves a cheap option. `bloom: true` skips the inline to avoid double bloom.
- **Self-contained additive pass, post bind-group unchanged.** Bloom composites directly onto the scene intermediate, so the post-process bind group / shader layout were untouched (only a `bloom_enabled` flag added to the uniform). Cleaner than threading a second bloom texture into the post bind group.
- **Single-target reconfigure, NOT the per-format pipeline cache.** Deliberate — bloom has one target per frame. So the `patterns-doc` "5th instance ⇒ abstract the cache" trigger does **not** fire here.
- **Half-res separable Gaussian × N**, not a full mip-chain / dual-filter bloom. A legitimate, recognizable real bloom; simpler + clearly demonstrable. Documented as such (fork-friendly).
- **MINOR v0.64.0** — additive public API, pre-1.0.

## Evidence & Data

- **Verify:** `./scripts/verify.sh > /tmp/verify_ship.log 2>&1` → **`VERIFY_EXIT=0`**, "all checks passed ✓"; fmt + clippy `-D warnings` + wasm lib-build + tests **71 passed / 0 failed / 32 ignored** + rustdoc. 3 new unit tests pass (`default_config_has_bloom_off`, `make_uniforms_sets_bloom_flag`, `uniforms_are_64_bytes`).
- **Native GPU smoke:** `cargo run --example bloom` under `bloom:true` + HDR `Rgba16Float`, rendered ~7 s across two runs with **no wgpu validation error / panic** (`RUST_LOG=wgpu_core=warn`). This is the real GPU gate — CI is ubuntu-only and cannot render. (Visual window capture was blocked by macOS window-raise/accessibility limits in this env — same as prior playtests; the validation-error-free render is the gate that matters.)
- **CI #231:** 4/4 green — Build (WASM) 41s, Package dry-run 1m16s, Rustdoc 44s, Test (native) 3m34s. `mergeStateStatus=CLEAN`, `MERGEABLE`.
- **Merge:** `gh pr merge 231 --squash --delete-branch`; `git pull --ff-only` → `397649d`. 13 files, +812 / −29.

## Gotchas / discoveries

- **WGSL `vec3<u32>` has 16-byte alignment — it does NOT match Rust `[u32; 3]` (align 4).** I padded `PostProcessUniforms` with `_pad1: vec3<u32>` in `post_process.wgsl`; that inflated the shader struct to **80 B** while the Rust struct was **64 B** → wgpu validation error: *"buffer bound with size 64 where the shader expects 80"* (and it surfaced on the **post-process** draw, not the bloom pass). **Fix:** three separate `u32` pads (align 4) ⇒ WGSL struct = 64 B, matching Rust. **Rule:** in a WGSL *uniform* struct, never use `vecN<T>` purely as scalar padding — its alignment shifts the layout; pad with scalars. The `uniforms_are_64_bytes` test pins the Rust side; the smoke pins the WGSL side. (Bloom's own `BloomUniforms` uses `vec2` only — align 8, no drift.)
- **This bug is invisible to CI.** It only appears at pipeline-draw time on a real GPU; the ubuntu CI never renders. The mandatory native smoke is exactly what caught it. Do not declare a renderer change done on green CI alone.
- **Bloom requires `enabled: true`.** It composites into the scene intermediate, which only exists when post-process is on. Documented on the field.
- **`update`/`run` take `&self`** (per-frame prefilter bind group is a local, built from the externally-owned scene view); the blur/composite bind groups reference bloom-owned textures and are built once. The `&self` signatures avoid a borrow clash with `render_view` (which immutably borrows `self.render.post_renderer`).

## Where We're Going

**Nothing is queued.** The session emptied its own queue (the user's chosen offer is now shipped).

**Next session, in order:**
1. Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (the game session may have filed EW-004+).
2. If still empty → **ASK** the user for direction (standing rule; do not start backlog speculatively).
3. Remaining un-queued offers if asked for ideas: an engine-level format-renderability query (the deferred "real P4 fallback" from the HDR arc); a fuller bloom (mip-chain / dual-filter) if half-res Gaussian proves too coarse; hearing the audio-facade web demos by ear; a `bloom` web harness via `/ship-wasm-example` (bloom runs on wasm — un-gated — but ships no web page yet).

## Related Handoffs (reference only — NOT parents)

- `HANDOFF_patterns-rt-format-cache_2026-06-23.md` — the `patterns-doc` chain that documented the per-target-format pipeline *cache*; this handoff explains why bloom is NOT a 5th instance of it.
- `HANDOFF_followup-batch2-hdr-render-arc_2026-06-23.md` — the HDR/render-format arc that bloom builds on (the `Rgba16Float` intermediate bloom composites into).

---

## Session Closed
**Closed at:** 2026-06-24 (KST)
**Commit:** code landed as #231 (`397649d`); this handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off to next session
