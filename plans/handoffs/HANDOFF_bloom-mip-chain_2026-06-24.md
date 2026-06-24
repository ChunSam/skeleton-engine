# Fuller bloom — mip-chain "dual filter" post-process

**Date:** 2026-06-24
**Status:** COMPLETED (code PR #237 merged; this handoff lands as its own `docs(handoff)` PR)
**Bead(s):** none (`bd` unavailable; engine work tracked via the dungeon-merchant wishlist board)
**Epic:** none — a user-chosen item from the recommended next-work offers (the wishlist board ACTIVE was empty)
**Chain:** `bloom-mip-chain` seq `1`
**Related (not parent):** `HANDOFF_bloom-pass_2026-06-24.md` (the v0.64.0 bloom-pass chain this *replaces the internals of*)
**Next:** nothing queued. Wishlist board ACTIVE is empty (next free ID **EW-004**) → next session read `../dungeon-merchant/docs/engine-wishlist.md` FIRST, then ASK for direction if still empty.

---

## The Goal

The v0.64.0 bloom-pass shipped a real multi-pass bloom, but its blur was a **fixed-half-res separable Gaussian** ping-ponged `bloom_iterations` times. That blur's reach is bounded by `kernel × iterations`, so pushing the spread is expensive and the result looks boxy/banded when widened. The bloom-pass handoff explicitly listed "a fuller bloom (mip-chain / dual-filter)" as a remaining un-queued offer. With the wishlist board empty, the user picked that offer.

The goal: replace the bloom internals with the **physically-based mip-chain bloom** (Jorge Jimenez — "Next Generation Post Processing in Call of Duty: Advanced Warfare"), which reaches a screen-wide, smooth, energy-preserving glow in a few passes — **without changing the public API**.

## Where We Are

- **main @ `3f7bf6b`, package v0.67.0, CLAUDE.md header v1.6.142, clean + green, no open PRs.** Code PR **#237** merged (squash, `feat(renderer)`); branch deleted; main synced.
- `PostProcessConfig.bloom` now drives a downsample/upsample mip pyramid. The OFF path (inline 4-tap) is byte-identical to before.

## Public API surface

**Unchanged** — no signature/field/type change:
- `PostProcessConfig.bloom` / `bloom_iterations` / `bloom_threshold` / `bloom_intensity` are the same fields. `bloom_iterations` (still `u32`, `0..=8`, default `4`) is **reinterpreted** from "number of separable blur passes" to "**mip-pyramid depth** (number of mip levels)". Both meanings control glow width, so call sites are unaffected — only the doc comment changed.
- `BloomRenderer` (`pub(crate)`) keeps the identical method set: `new` / `reconfigure` / `resize` / `width` / `height` / `format` / `update` / `run` + `MAX_BLOOM_ITERATIONS`. Because the API is identical, the render-orchestration call sites — `frame.rs` step 3.5 (`br.update`; `br.run(.., cfg.bloom_iterations)`) and `post_lighting.rs::setup_bloom_renderer` — were **not touched**.

**Versioning:** v0.67.0 MINOR — a behavioral renderer improvement (pre-1.0, so MINOR covers it). Not a breaking compile change, but the visual output and the `bloom_iterations` semantics change, so it is a release, not a patch.

## How it works (the shipped design)

All in `src/renderer/bloom.rs` + `src/renderer/shaders/bloom.wgsl`:

- **Pyramid** — a `Vec<Mip>` of half-and-down render targets. `mips[0]` is half the scene-intermediate resolution; each level halves again, capped at `MAX_BLOOM_ITERATIONS` (8) levels and stopping before a dimension degenerates (`≤2 px`). Each is its own texture (separate render targets, **not** mip levels of one texture — reading mip *i+1* while writing mip *i* of the *same* texture would be a read/write hazard in wgpu).
- **Four pipelines**, all sharing one bind-group layout (texture + sampler + uniform):
  - `fs_prefilter` (REPLACE) — 13-tap downsample of the **scene** + threshold soft-knee → `mip[0]`.
  - `fs_downsample` (REPLACE) — 13-tap downsample, `mip[k] → mip[k+1]`.
  - `fs_upsample` (**additive** One,One) — 3×3 tent, `mip[k+1] → mip[k]`, blended onto the freshly-downsampled `mip[k]`.
  - `fs_composite` (**additive**) — `mip[0] × intensity → scene`.
- **Per-frame flow in `run()`** (n = `iterations.max(1).min(mips.len())`): prefilter → downsample chain `mip[0..n-1]` (REPLACE/Clear) → upsample chain `mip[n-1]..mip[0]` (additive/Load) → composite onto the scene. So `mip[i] = downsample[i] + tent(mip[i+1])`, accumulating every level into `mip[0]`.
- **Uniforms** — one 32-byte struct `{threshold, intensity, radius, _pad0, texel: vec2, _pad1: vec2}`. The downsample/upsample texels (1/source-resolution) + the upsample `radius` (1.0 texel) are **baked at construction** into per-step uniform buffers kept alive by their bind groups. Only `prefilter_ub` (threshold + the constant scene texel) and `composite_ub` (intensity) are retained and rewritten each frame in `update()`.
- **Static vs per-frame bind groups** — the downsample/upsample/composite inputs are bloom-owned textures, so those bind groups are built **once**. Only the prefilter input (the externally-owned scene view) is rebuilt per frame.
- **Format handling unchanged** — single target per frame (the scene intermediate), so a format change rebuilds the renderer (`reconfigure`), like post/lighting — NOT the per-target-format pipeline cache used by sprite/material/UI/GPU-particle.

## What We Tried (process)

1. **Onboarding** — board empty; offered the three un-queued items; user picked "풀러 블룸 (mip/dual-filter)".
2. **Read** `bloom.rs` + `bloom.wgsl` + `examples/bloom.rs` + the integration points (`frame.rs`, `post_lighting.rs`, `post_process.rs`, `render_state.rs`) → confirmed the whole pass is encapsulated behind the `BloomRenderer` `pub(crate)` API, so only the two files needed rewriting.
3. **Rewrote** the shader (downsample/upsample functions + four fragment entries) and the renderer (mip pyramid + per-step bind groups + the new `run()` chain). Updated the `bloom_iterations` doc, the module doc, and the example wording (separable Gaussian → mip-chain / "pyramid depth").
4. **Verified** — `./scripts/verify.sh` green (twice: after the rewrite, and again after the `/ship` version bump). Native GPU smoke on Metal (no validation errors) + before/after windowed screenshots.
5. **Shipped** v0.67.0 (Cargo.toml/lock, CHANGELOG, CLAUDE.md header + module-map row), PR #237, CI 4/4 green, squash-merge.

No dead ends.

## Key Decisions

- **Mip-chain (downsample/upsample pyramid), not Kawase dual-filter.** Both were on the menu; the COD pyramid is the better-documented, more standard algorithm and maps cleanly onto `bloom_iterations` as pyramid depth. (A Kawase variant would have been a similar amount of code with no clear quality win at this scale.)
- **Keep the public API; reinterpret `bloom_iterations`.** Adding a new field or renaming would break call sites (and the example/game). Since both "blur passes" and "pyramid depth" control glow width, reinterpreting is non-breaking and keeps the change surgical.
- **Separate textures per mip, not one mipmapped texture.** Avoids the same-texture read/write hazard and matches how most engine bloom implementations do it.
- **Additive upsample via blend state, not a manual read-add-write.** `One,One` blend onto the already-downsampled level is the standard accumulation and avoids extra bind groups.
- **Native GPU smoke is mandatory and visual.** CI is ubuntu-only and never renders. The screenshot pair (mip-chain wide halos vs inline 4-tap hard squares) is the real acceptance evidence, not green CI.

## Evidence & Data

- **Verify:** `./scripts/verify.sh` → `VERIFY_EXIT=0`, "all checks passed ✓" (re-run after the bump).
- **Native GPU smoke:** `cargo run --example bloom` on Metal (`RUST_LOG=wgpu_core=warn`) — ran multi-second, **no `ERROR`/panic/validation** lines. Windowed screenshots: the over-bright warm/cyan/green emitters show wide, smooth, coloured halos under the mip-chain; toggling `B` to the inline 4-tap collapses them to hard-edged clipped squares; the two dim swatches never glow (below `bloom_threshold`) in both modes.
- **CI #237:** 4/4 green — Test (native) 4m43s, Build (WASM) 39s, Package dry-run 1m1s, Rustdoc 41s. `mergeStateStatus=CLEAN`.
- **Merge:** `gh pr merge 237 --squash --delete-branch`; `git pull --ff-only` → `3f7bf6b`.
- **Diff:** 8 files, +275 / −155 (the two bloom files are the bulk; the rest are doc/version/example wording).

## Gotchas / discoveries

- **`bloom_iterations` is dynamic per frame** (the example's `←/→` change it 0..=8), so the pyramid is **allocated for the max** at construction and `run()` uses only the first `n` levels per frame. Every used level gets a REPLACE pass (prefilter/downsample) before its additive upsample, so there's no stale cross-frame data even as `n` changes.
- **No stale data across `n` changes** — downsample/prefilter use `LoadOp::Clear` + REPLACE (full overwrite); only the upsample/composite use `LoadOp::Load` + additive, and always onto a level written this frame.
- **WGSL uniform alignment** — `texel: vec2<f32>` sits at offset 16 (a vec2 is 8-byte aligned; 16 is a multiple of 8) so the 32-byte struct is std140-clean. Per the standing trap: never pad a uniform *with* a vecN.
- **Default intensity is still `0.4`** (`PostProcessConfig::default`) — the mip-chain spreads energy more than the old separable blur, so the same number can read a touch softer; the `bloom` example overrides to `1.0`. Left the default as-is (non-breaking; tune per-game).
- **Pyramid reaches ~3 px on an 880-wide window** (8 levels: 440,220,110,55,27,13,6,3) — that wide spread is exactly what makes the glow "fuller". On a 4K window the 8-level cap stops at ~15 px (good enough; the cap matches the public `0..=8` range).

## Where We're Going

**Nothing is queued.**

**Next session, in order:**
1. Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (the game session may have filed EW-004+).
2. If still empty → **ASK** the user for direction (standing rule; do not start backlog speculatively).
3. Remaining un-queued offers if asked: a `bloom` web harness via `/ship-wasm-example` (the mip-chain is the natural thing to show on WebGL2); hearing the audio-facade web demos by ear; an even-fuller bloom (exposed upsample radius / lens-dirt) only if a game asks.

## Related Handoffs (reference only)

- `HANDOFF_bloom-pass_2026-06-24.md` — the v0.64.0 bloom-pass whose internals this replaces (separable Gaussian → mip-chain).
- `HANDOFF_render-format-query-web_2026-06-24.md` — the prior chain (seq 2, v0.66.0); its "ship an example to the web" precedent is the model if the next session does a bloom web harness.

---

## Session Closed
**Closed at:** 2026-06-24 (KST)
**Commit:** code landed as #237 (`3f7bf6b`); this handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off to next session
