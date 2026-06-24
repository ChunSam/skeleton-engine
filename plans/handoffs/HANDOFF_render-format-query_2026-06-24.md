# Engine-level render-format renderability query + automatic render-target fallback

**Date:** 2026-06-24
**Status:** COMPLETED (code PR #233 merged; this handoff lands as its own `docs(handoff)` PR)
**Bead(s):** none (`bd` unavailable; engine work tracked via the dungeon-merchant wishlist board)
**Epic:** none — a user-chosen feature from the empty-queue "Where We're Going" offers
**Chain:** `render-format-query` seq `1` (new chain — NOT a continuation of `bloom-pass`, which is COMPLETE)
**Parent:** none
**Next:** nothing queued. Wishlist board ACTIVE is empty (next free ID **EW-004**) → next session read `../dungeon-merchant/docs/engine-wishlist.md` FIRST, then ASK for direction if still empty.

---

## The Goal

The session started on a clean+green `main` with the queue genuinely empty (wishlist ACTIVE empty, `bloom-pass` chain done). Per the standing rule I read the board first, confirmed it empty, and ASKED the user for direction, offering the un-queued candidates the prior handoff left. The user picked the **format-renderability query** — the deferred "real P4 fallback" from the HDR/render-format arc (seq 76).

Deliverable: an **engine-level way to ask whether a `wgpu::TextureFormat` is a usable color render target** on the current GPU/backend, plus **automatic graceful fallback** when a requested render-target format is not renderable — shipped with a playable example (the VISION acceptance test).

## Where We Are

- **main @ `502896f`, package v0.65.0, CLAUDE.md header v1.6.140, clean + green, no open PRs.** Code PR **#233** merged (squash, `feat(renderer)`); branch deleted; main synced.
- **New feature, fully wired + verified.** A query API + an automatic fallback, both backed by the now-retained GPU adapter.

## Public API surface

Additive (→ MINOR). One new public type + two new `GpuContext` methods + a behavior-softening of one existing `App` method:

- **`RenderCapabilities`** — re-exported at the crate root (`engine::RenderCapabilities`), inserted into the `World` as a resource at GPU init. `supports_render_target(format: wgpu::TextureFormat) -> bool` and `surface_format() -> wgpu::TextureFormat`. `Clone` (holds a cloned `wgpu::Adapter`).
- **`GpuContext`** — now retains `pub adapter: wgpu::Adapter` (was dropped after init); gains `supports_render_target(format)` and `resolve_render_target_format(requested, name)`.
- **`App::create_render_target_with_format`** — unchanged signature, but now **falls back** to the surface format (with a `log::warn!`) when the requested format is not renderable, instead of attempting an invalid texture. `None` and renderable formats behave exactly as before.

## How it works (the shipped design)

**The gap (why P4 deferred this):** only `wgpu::Adapter::get_texture_format_features(format).allowed_usages` reports per-format renderability (the `Device` does not expose it), and `GpuContext` *dropped* the adapter after init. So the engine had no way to answer "is `Rgba16Float` renderable here?" — on a WebGL2 context that lacks `EXT_color_buffer_float` it is not, and `create_render_target_with_format(Rgba16Float)` would build an invalid texture.

**Three parts** (`src/renderer/context.rs` unless noted):
1. **Retain the adapter** — added `adapter` to `GpuContext` (wgpu's `Adapter` is `#[derive(Clone)]`, internally ref-counted — cheap). A private free fn `format_supports_render_target(adapter, format)` is the single source of truth (`allowed_usages.contains(RENDER_ATTACHMENT)`).
2. **Query API** — `RenderCapabilities` resource holds a cloned adapter + the surface format; `GpuContext::supports_render_target` delegates to the same free fn. The resource is inserted in `src/app/window.rs` right after the pending-RT drain (`RenderCapabilities::new(gpu.adapter.clone(), gpu.config.format)`).
3. **Automatic fallback** — `GpuContext::resolve_render_target_format(requested, name)` resolves `None`→surface, renderable→as-requested, unrenderable→surface (+ warn). Both RT-creation sites call it: the immediate path (`src/app/assets.rs`) and the deferred GPU-init drain (`src/app/window.rs`). A pure helper `pick_render_target_format(requested, is_renderable, surface)` holds the decision so it is unit-testable without a GPU.

**Example** `examples/render_format_query.rs`: a system reads `RenderCapabilities` each frame and renders a color-coded yes/no table over a spread of formats + the HDR-vs-fallback decision; it also requests an `Rgba16Float` RT up front to exercise the fallback path, and logs the table once (so a headless smoke can assert).

## What We Tried (process)

1. **Onboarding** — read the bloom handoff, the wishlist board (empty), the engine-state memory; confirmed clean+green `main @ fd66f3b` v0.64.0. ASKED for direction → user chose the format-renderability query.
2. **Read the code path** — `render_target.rs` (RT texture uses `RENDER_ATTACHMENT|TEXTURE_BINDING`), `assets.rs` + `window.rs` (the two RT-creation sites), `context.rs` (the adapter is dropped after init — the crux), `world.rs` (resources are `Box<dyn Any>`, **not** `Send+Sync` → a `wgpu::Adapter` resource compiles on wasm too).
3. **Verified the wgpu API** in the vendored source: `Adapter: Clone`, `get_texture_format_features(&self, format) -> TextureFormatFeatures { allowed_usages: TextureUsages, .. }`.
4. **Implemented** the adapter retention + query + fallback + resource + re-exports + example + 3 unit tests + CLAUDE.md row.
5. **Verify gate green** twice (`VERIFY_EXIT=0`; lib 931 tests incl. the 3 new ones) — once before the `/ship` bump, once after.
6. **Native GPU smoke** caught a wrong example assumption (see Gotchas), fixed it, re-ran clean.
7. **Landed via `/land-pr`** → `/ship` v0.65.0 → PR #233 → CI 4/4 green → squash-merge.

No design dead ends; the one correction was the example's format-discriminator choice.

## Key Decisions

- **Query + automatic fallback (both), not query-only.** The handoff framed it as the "real *fallback*"; a query alone would still let an unrenderable request build an invalid texture. The fallback makes `create_render_target_with_format` robust; the query lets a game branch deliberately.
- **Retain the adapter rather than precompute a format set.** General (answers any format), and cheap (Adapter is ref-counted). Storable as a World resource because resources are `Box<dyn Any>` (no Send+Sync bound) → works on wasm despite wasm GL handles being `!Send`.
- **`RenderCapabilities` lives in `renderer::context`**, localizing the `wgpu::Adapter` dependency; re-exported at the crate root for game ergonomics.
- **Pure decision helper (`pick_render_target_format`)** extracted so the fallback policy is CI-testable; the live `get_texture_format_features` query is GPU-only (native smoke).
- **MINOR v0.65.0** — additive public API, pre-1.0.

## Evidence & Data

- **Verify:** `./scripts/verify.sh` → `VERIFY_EXIT=0`, "all checks passed ✓"; fmt + clippy `-D warnings` + wasm lib/bins build + tests (lib **931 passed / 0 failed**, incl. the 3 new `pick_format_*`) + rustdoc. Re-run green after the version bump.
- **Native GPU smoke** (CI is ubuntu-only, cannot render): `cargo run --example render_format_query` on macOS/Metal → surface `Bgra8UnormSrgb`; `Rgba16Float`/`Rgba32Float`/`Rgba8Unorm`/`Rgba8UnormSrgb` → renderable (`yes`), `Bc1RgbaUnorm` → `no`. The `Rgba16Float` RT was created with **no wgpu validation error / panic / fallback warning** (Metal renders it).
- **CI #233:** 4/4 green — Build (WASM) 44s, Package dry-run 1m10s, Rustdoc 1m7s, Test (native) 4m52s. `mergeStateStatus=CLEAN`, `MERGEABLE`.
- **Merge:** `gh pr merge 233 --squash --delete-branch`; `git pull --ff-only` → `502896f`. 10 files, +354 / −17.

## Gotchas / discoveries

- **`get_texture_format_features` reports the *backend-native* capability, not the portable WebGPU subset.** My first example used `Rgba8Snorm` as a guaranteed "not renderable" discriminator (true per the WebGPU spec) — but on **Metal it reported `yes`** (Apple GPUs *can* render to snorm). So the example showed an all-`yes` table on desktop, hiding the discrimination. **Fix:** probe a **block-compressed** format (`Bc1RgbaUnorm`) — compressed formats are *never* a color render attachment on any backend, so it reliably shows `no`. **Lesson:** the query is accurate to the real GPU; don't reason about renderability from the portable spec. On desktop nearly all uncompressed color formats are renderable; the float formats are the ones that flip to `no` on WebGL2 without `EXT_color_buffer_float`.
- **This is invisible to CI.** The live query + RT creation only run on a real GPU; ubuntu CI never renders. The mandatory native smoke is what surfaced the example issue. Do not declare a renderer change done on green CI alone.
- **Resources are not `Send+Sync` here** (`resources: HashMap<TypeId, Box<dyn Any>>`), which is *why* a `wgpu::Adapter` (which is `!Send` on the wasm GL backend) can be stored as a `RenderCapabilities` resource on both targets.
- **`RenderCapabilities` is inserted only at GPU init**, so it does not exist before `App::run()` (where games configure RTs). A game that wants to branch on it must do so from a system / `Scene::on_enter`; the automatic fallback covers the before-`run()` RT requests.

## Where We're Going

**Nothing is queued.** The session emptied its own queue (the user's chosen offer is now shipped).

**Next session, in order:**
1. Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (the game session may have filed EW-004+).
2. If still empty → **ASK** the user for direction (standing rule; do not start backlog speculatively).
3. Remaining un-queued offers if asked for ideas: a fuller bloom (mip-chain / dual-filter) if the half-res Gaussian proves too coarse; hearing the audio-facade web demos by ear; a `bloom` and/or `render_format_query` web harness via `/ship-wasm-example` (both run on wasm — the format query is *most* meaningful on WebGL2, where `Rgba16Float` actually flips to non-renderable — but neither ships a web page yet).

## Related Handoffs (reference only — NOT parents)

- `HANDOFF_bloom-pass_2026-06-24.md` — the immediately-prior chain (real multi-pass bloom); same "user picks from empty-queue offers" cadence.
- `HANDOFF_followup-batch2-hdr-render-arc_2026-06-23.md` — the HDR/render-format arc (seq 76) whose P4 explicitly deferred this engine-level format-feature query as out of scope; this handoff closes it.

---

## Session Closed
**Closed at:** 2026-06-24 (KST)
**Commit:** code landed as #233 (`502896f`); this handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off to next session
