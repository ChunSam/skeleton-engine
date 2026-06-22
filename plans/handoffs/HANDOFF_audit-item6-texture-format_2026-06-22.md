# Audit deferred item 6 — texture upload pixel-format parameterization (v0.50.0, additive)

**Date:** 2026-06-22
**Status:** COMPLETED + merged. main @ `23285af`, package **v0.50.0**, clean tree, full gate green, CI green, squash-merged (#193).
**Bead(s):** none
**Epic:** engine-audit deferred-items follow-up arc (seq-1 audit #184)
**Chain:** `standalone-4365aa4a` seq `4`
**Parent:** `HANDOFF_audit-deferred-editor-tier5_2026-06-22.md` (seq 3)
**Prior chain:** `HANDOFF_engine-audit-fixes_2026-06-22.md` (seq 1) > `HANDOFF_audit-followup-refactors_2026-06-22.md` (seq 2) > `HANDOFF_audit-deferred-editor-tier5_2026-06-22.md` (seq 3) > this (seq 4)
**Auto:** false

---

## Stale References

Parent (seq 3) identifiers all still resolve. One **line-number shift** to note for the next session (Appendix C of the parent anchored these):

- `src/renderer/texture.rs` — the `Rgba8UnormSrgb` hardcode that was at **:130** is now **inside the new `from_rgba_with_format`** (~:198 in the new body), NOT in `from_rgba` anymore. `from_rgba` is now a thin wrapper at **:108**. Item 6 is DONE so this anchor is historical.
- `ecs/world.rs` unwrap anchors (item 7) — **unchanged**, file untouched this session (still ~1102 lines, the `~51` raw `.unwrap()` count from the parent's Appendix C still holds).

## Since Last Handoff (seq-3 plan vs reality)

Parent's "Where We're Going" listed exactly two remaining audit items: **6** (texture format, "breaking → feature+example") and **7** (world.rs 56 unwraps, "high blast radius"). The user gave a single go-ahead ("진행해") and I recommended **item 6** (more contained); the user accepted. This session:

- **Did item 6** — but found it could be made **additive (MINOR), NOT breaking** as the parent predicted, by keeping the old `from_rgba`/`from_path`/`try_from_path` as thin `Rgba8UnormSrgb` wrappers and adding `_with_format` siblings. So all existing call sites stayed byte-identical.
- **Narrowed item 6's scope on evidence:** the parent floated "HDR / linear workflows" and "render to a linear/HDR target". A true differently-formatted **render target** turned out to be a much larger feature (the sprite pipeline binds ONE color-target format at construction → needs a format-matched pipeline). Delivered the contained, genuinely-useful half: **linear data-texture uploads** (`Rgba8Unorm`), sampled verbatim without the sRGB decode.
- **Item 7 remains** — the last open audit item. Untouched (correctly — it's a careful per-unwrap hardening pass, not a sweep).
- Trajectory: still squarely on the seq-1-audit cleanup path; after item 7 the audit deferred-list is fully closed.

## Reference Documents

- `CLAUDE.md` — module map, verify rules, pre-1.0 versioning (MINOR = any release incl. breaking; this was MINOR for an additive feature).
- `docs/VISION.md` — the "a feature is not done until a small playable example exercises it" loop (the `texture_format` example is item 6's acceptance test).
- `docs/PATTERNS.md` — ECS query / borrow-workaround patterns.
- Grandparent `HANDOFF_engine-audit-fixes_2026-06-22.md` — the full 8-item audit + the `panic="abort"` weighting lens + the **WindowConfig ~70-call-site** breaking-field constraint (relevant if item 7 or future work touches WindowConfig).

## The Goal

Close the seq-1 engine-audit deferred-item list (items 1–8). Items 1–4 + 8 were done in seqs 61–66; item 5 was investigated and rejected (needs a facade feature). This session targeted **item 6: `renderer/texture.rs` hardcoded `Rgba8UnormSrgb` → parameterized format**, so a fork can load **data textures** (normal maps, masks, height/lookup tables) as linear (`Rgba8Unorm`) and have them sampled verbatim, instead of every loaded image being forced through the sRGB→linear decode. The acceptance test is a small playable example (VISION loop). Only item 7 (world.rs unwraps) remains after this.

## Where We Are

- **main @ `23285af`, package v0.50.0, CLAUDE.md header v1.6.118, clean tree, `./scripts/verify.sh` → exit 0** (fmt + clippy `-D warnings` native + wasm32 lib/bins build + `test --all-targets` + doc `-D warnings`).
- **PR #193 squash-merged** on green CI (Rustdoc / Build WASM / Package dry-run / Test native all pass), branch `feat/texture-format-param` deleted, local main fast-forwarded.
- **New public API (all additive → MINOR v0.50.0):**
  - `App::load_image_with_format(path, format: wgpu::TextureFormat) -> Handle<ImageAsset>` (`src/app/assets.rs`).
  - `SpriteRenderer::load_texture_with_format(device, queue, path, format)` (`src/renderer/sprite/textures.rs`, `pub`).
- **New internal API (`pub(crate)`):** `Texture::from_rgba_with_format`, `from_path_with_format`, `try_from_path_with_format` (`src/renderer/texture.rs`). The existing `from_rgba` / `from_path` / `try_from_path` are now thin wrappers passing `Rgba8UnormSrgb` — **call sites byte-identical**.
- **New App state:** `App.pending_texture_formats: HashMap<String, wgpu::TextureFormat>` (`src/app.rs`), a per-path format override (absent = srgb default).
- **New example:** `examples/texture_format.rs` (flat, auto-discovered, no Cargo.toml entry needed) — loads one grayscale-ramp PNG twice with byte-identical pixels but two formats, side by side.
- **Visually verified on macOS:** ran the example (caffeinate + osascript window bounds + screencapture), confirmed the `Rgba8Unorm` (linear) ramp renders visibly brighter in the midtones than the sRGB ramp — proving the format param actually changes GPU sampling.
- **Format threading routes ONLY through the `pending_textures` / `from_path` path** (drained at GPU init in `src/app/window.rs:~623`), NOT the AssetServer per-frame upload. This path inserts under both raw + canonical cache aliases, so a `Sprite` referencing the texture by its raw path resolves; the per-frame AssetServer upload skips it via `has_texture_key` (no clobber).
- **HDR `Rgba16Float` render target deliberately OUT OF SCOPE** — documented in the CHANGELOG + PR + memory.
- Memory `engine-current-state` bumped to **seq 67**; `MEMORY.md` index line refreshed; item 6 marked DONE in the seq-60 master-audit bullet + the lead paragraph (only item 7 STILL OPEN now).

## What We Tried (Chronological)

1. **Read both item-6 and item-7 anchors during onboarding** (`texture.rs`, `world.rs`, `render_target.rs`, wishlist board). Found the wishlist board EMPTY (next ID EW-002) — no game request to preempt the backlog. Recommended item 6 as more contained.
2. **Discovered `RenderTarget::new` ALREADY takes a `format` param** (`render_target.rs:28`) — so the "render to a linear/HDR target" framing was half-built. But `App::create_render_target` hardcodes `gpu.config.format` (surface = sRGB) when calling it.
3. **Probed whether a differently-formatted render target is viable** → grepped the sprite pipeline: `SpriteRenderer::new(device, queue, format)` binds the color target to a SINGLE format at construction (`sprite.rs:189`, `:228`; material pipeline `material.rs:101` uses `self.surface_format`). **Conclusion: rendering INTO a non-surface-format RT fails wgpu validation — needs a format-matched pipeline. That's a big feature → OUT OF SCOPE.** This is the key scoping decision.
4. **Settled on the contained feature: CPU texture-upload format choice** (linear data textures). Verified a `Rgba8Unorm` texture is still sampleable by the existing sprite pipeline — both `Rgba8UnormSrgb` and `Rgba8Unorm` satisfy the bind-group layout's `TextureSampleType::Float { filterable: true }` (`texture.rs:176`). No new pipeline needed.
5. **Mapped the two texture-upload paths** to decide where to thread format:
   - `pending_textures: Vec<String>` (drained at GPU init, `window.rs:623`) → `SpriteRenderer::load_texture` → `Texture::from_path` (reads file, inserts under BOTH `[raw, asset_key]` aliases via `file_texture_aliases`).
   - AssetServer image store → `upload_asset_server_images_to_gpu` (per-frame, `schedule.rs:574`) + init loop (`window.rs:627`) → `load_texture_from_image` → `from_image_asset` (inserts under the SINGLE canonical key only).
   - Confirmed `AssetServer::load_image` **synchronously decodes** on native (`image_loading.rs:32`, `decode_image_with_state`), so the image is available at init.
6. **Chose the `pending_textures`/`from_path` path** (NOT the AssetServer path) because it inserts under both aliases (so a `Sprite`-by-raw-path resolves) and it's how examples load at startup. The AssetServer per-frame upload then skips the already-loaded texture via `has_texture_key`. This avoids touching `from_image_asset` and avoids a Sprite-key mismatch.
7. **Implemented** the wrapper-pair design across 5 source files (texture.rs → sprite/textures.rs → app.rs field → app/assets.rs method → app/window.rs drain loop). Drained `pending_textures` into a local Vec first to sidestep a disjoint-borrow issue with `pending_texture_formats` in the same loop.
8. **First gate run failed on `cargo fmt --check`** — rustfmt wanted a multi-line wrapper collapsed to one line. Ran `cargo fmt`, re-ran.
9. **Second gate run failed on clippy `too_many_arguments`** (`from_rgba_with_format` = 8 args; the original `from_rgba` was already at the 7-arg boundary). Added `#[allow(clippy::too_many_arguments)]` with a justifying comment (mirrors the wgpu descriptor arg set; a struct would obscure an internal helper). Third gate run → exit 0.
10. **Wrote `examples/texture_format.rs`** mirroring `nine_slice.rs` (generate PNG to temp, `load_image`, `Sprite::with_handle`). Generated two byte-identical temp PNGs at two paths (one-format-per-path), loaded one srgb + one linear.
11. **Visual smoke test on macOS** — launched the example, positioned the window, screencaptured, Read the PNG. Confirmed the predicted brightness difference. Killed the process.
12. **`/ship` paperwork** (Cargo.toml 0.49.4→0.50.0, Cargo.lock refresh, CHANGELOG 0.50.0 entry, CLAUDE.md header v1.6.117→v1.6.118 + module-map `load_image` row). Re-ran gate → exit 0.
13. **`/land-pr` loop** — commit, push, `gh pr create` (#193), `gh pr checks --watch --fail-fast` (exit 0), confirmed `mergeStateStatus: CLEAN`, `gh pr merge --squash --delete-branch`, `git pull --ff-only`, bumped memory to seq 67.

## Key Decisions

- **Additive, not breaking.** The parent predicted item 6 was "breaking to `from_rgba`". By keeping `from_rgba`/`from_path`/`try_from_path` as thin `Rgba8UnormSrgb` wrappers and adding `_with_format` siblings, the change is purely additive → MINOR with zero call-site churn. (Pre-1.0, a new feature is MINOR either way, but additive is cleaner and safer.)
- **HDR render target rejected as out-of-scope.** The sprite pipeline's single bound color-target format means a differently-formatted RT needs its own pipeline — a separate, larger feature. Delivered the contained linear-data-texture half instead. Documented the boundary so a future session knows where the line is.
- **Thread format through the `pending_textures`/`from_path` path only**, not the AssetServer per-frame path. Rationale: that path inserts under both raw + canonical aliases (Sprite-by-raw-path resolves), it's the startup-load path examples use, and the per-frame AssetServer upload harmlessly skips via `has_texture_key`. Threading the AssetServer path too would need canonical-key format storage + a `from_image_asset` change for marginal benefit (runtime-after-init format loads, a rare case). Documented `load_image_with_format` as "call before `run()`".
- **`Texture::*_with_format` kept `pub(crate)`**, not `pub`. The public surface is `App::load_image_with_format` + `SpriteRenderer::load_texture_with_format` (the latter already-`pub` for fork GPU access). The `Texture` type's lower-level fns don't need to widen.
- **`#[allow(clippy::too_many_arguments)]` on `from_rgba_with_format`** rather than bundling args into a struct — it's an internal helper mirroring the wgpu `TextureDescriptor` arg set; a struct would obscure, not clarify.
- **Did NOT use `/add-feature-example` skill** despite the parent suggesting it. The skill scaffolds a NEW module + re-export; item 6 is additive methods spread across existing files, not a new module. Implemented manually + ran `/ship`. (Noted so the next session doesn't expect a new module.)
- **No `pub use wgpu` re-export added.** `App::load_image_with_format` exposes `wgpu::TextureFormat` in the public API, but `RenderTarget::new` / `RenderPlugin` already do this without a re-export, so followed precedent. Examples reach `wgpu::` directly as a normal dep (confirmed `examples/render_plugin.rs` does this).
- **CLAUDE.md module map: updated the `load_image` row** (added `load_image_with_format`), did NOT add a new row — consistent with the repo convention of not naming every method.
- **One-format-per-path; rejected format-in-cache-key.** Considered keying the texture cache by `{path}#{format}` so the SAME file could be loaded at two formats (cleaner example: one source file). Rejected — the `Sprite.texture` is a raw string key looked up DIRECTLY (`bind_group_for_texture_key` does `texture_cache.get(key)` with no normalization), so a format-suffixed key would force the example to know/reconstruct that suffix, and it ripples into `has_texture_key` + the alias scheme. Kept the simpler **one-format-per-path** model; the example uses two byte-identical temp PNGs at two paths instead (a clear comment notes they're identical, only the load format differs). Documented limitation: `load_image` + `load_image_with_format` on the SAME path = last-write-wins on format.

## Example structure (for reference)

`examples/texture_format.rs`: `generate_ramp_png(path)` writes a 128×128 horizontal black→white gray ramp via the `image` crate (a normal dep, usable in examples); `load_both_ramps(app)` writes two byte-identical temp PNGs (`skeleton_texfmt_ramp_srgb.png` / `_linear.png` in `std::env::temp_dir()`) and loads one via `load_image` (srgb) + one via `load_image_with_format(..., TextureFormat::Rgba8Unorm)`; `spawn_swatch` draws each as a 330px `Sprite::with_handle`; `DemoSystem` pushes explanatory `DrawText` labels each frame + Esc-to-quit. Native (temp-file gen); compiles fine — the wasm gate is lib+bins only, not examples.

## Evidence & Data

**Commit / merge:**

| Item | Value |
|---|---|
| Branch | `feat/texture-format-param` (deleted post-merge) |
| Local commit | `c23c628` |
| Merged squash commit | `23285af` (subject carries a stray `(#TBD)` — cosmetic; squash appended `(#193)`) |
| PR | #193, `mergeStateStatus: CLEAN`, squash + branch-deleted |
| CI | Rustdoc 43s · Build WASM 51s · Package dry-run 1m11s · Test native 4m5s — all pass |
| Version path | v0.49.4 → **v0.50.0** (MINOR) |

**Diffstat (merge commit):** 10 files, +302/−11. `examples/texture_format.rs` created (146 lines).

**Per-file change sizes (working diff before ship):** CLAUDE.md +4/−2, Cargo.lock 1, Cargo.toml 1, docs/CHANGELOG.md +12, src/app.rs +4, src/app/assets.rs +22, src/app/window.rs +12/−1, src/renderer/sprite/textures.rs +27, src/renderer/texture.rs +82.

**The brightness-difference reasoning (why the example is a valid acceptance test)** — concrete value, byte 188:
- sRGB-format source: sample auto-decodes `srgb_to_linear(0.737) ≈ 0.50` → shader → sRGB surface re-encodes `linear_to_srgb(0.50) ≈ 0.735` → displays ≈ **188** (round-trips to original).
- `Rgba8Unorm` (linear) source: sample reads 0.737 verbatim (no decode) → shader → surface encodes `linear_to_srgb(0.737) ≈ 0.88` → displays ≈ **224** (≈ +36/255 brighter).
- Confirmed visually: the right (linear) ramp's midtones are clearly lighter than the left (sRGB).

**Gate runs:** `/tmp/verify_texfmt.log` (3 runs: fmt-fail → clippy-fail → exit 0), `/tmp/verify_ship.log` (post-bump, exit 0). Screenshot captured to `/tmp/texfmt_shot.png` (transient).

## Code Analysis

- **`Texture::from_rgba_with_format(device, queue, layout, data, width, height, label, format)`** (`texture.rs:~178`, `pub(crate)`, `#[allow(clippy::too_many_arguments)]`) — carries the real `create_texture_with_data` body with `format` plumbed into the `TextureDescriptor`. `from_rgba` (`:108`) delegates with `Rgba8UnormSrgb`.
- **`from_path_with_format` / `try_from_path_with_format`** (`pub(crate)`) — file read + decode + `from_rgba_with_format`; `from_path` / `try_from_path` delegate with srgb.
- **`TextureCache::load_texture_with_format(device, queue, path, format)`** (`sprite/textures.rs`) — has the alias-caching + `from_path_with_format` body; `load_texture` delegates with srgb. `SpriteRenderer::load_texture_with_format` (`pub`) forwards to it.
- **`App::load_image_with_format`** (`app/assets.rs`) — pushes path to `pending_textures`, inserts `format` into `pending_texture_formats`, registers with `AssetServer` (for hot-reload watching + asset browser), returns the `Handle`.
- **Upload site** (`app/window.rs:~623`, inside `finish_init(&mut self, gpu: GpuContext, …)` — `gpu` is an OWNED param, NOT a `self` borrow, so the drain-loop borrow is clean): drains `pending_textures` into a local Vec, looks up each path's format (default `Rgba8UnormSrgb`), calls `load_texture_with_format`, then `pending_texture_formats.clear()`.
- **Sampling compatibility:** `Texture::bind_group_layout` uses `TextureSampleType::Float { filterable: true }` (`texture.rs:176`) — satisfied by both srgb and unorm 8-bit RGBA formats, so no pipeline/bind-group change.
- **`asset_key`** (`asset.rs:237`) canonicalizes paths on native; `file_texture_aliases` caches under `[raw, asset_key]`; `bind_group_for_texture_key` does a DIRECT `texture_cache.get(key)` (no asset_key normalization), which is why caching under both aliases matters for Sprite-by-raw-path lookup.

## Files Changed

### Source code
- `src/renderer/texture.rs` — `from_rgba_with_format` + `from_path_with_format` + `try_from_path_with_format` added; old fns → srgb wrappers.
- `src/renderer/sprite/textures.rs` — `TextureCache::load_texture_with_format` + `SpriteRenderer::load_texture_with_format` (pub) added; old fns → srgb wrappers.
- `src/app.rs` — `pending_texture_formats: HashMap<String, wgpu::TextureFormat>` field + init.
- `src/app/assets.rs` — `App::load_image_with_format` added.
- `src/app/window.rs` — GPU-init drain loop now format-aware (drain-to-local + per-path lookup + clear).

### Example (acceptance test)
- `examples/texture_format.rs` — NEW. Same ramp bytes, sRGB vs `Rgba8Unorm`, side by side; Esc to quit; labels explain the difference.

### Release paperwork
- `Cargo.toml` (0.50.0), `Cargo.lock` (refreshed), `docs/CHANGELOG.md` (0.50.0 entry), `CLAUDE.md` (header v1.6.118 + `load_image` module-map row).

### Memory (outside repo)
- `engine-current-state.md` → seq 67; `MEMORY.md` index line refreshed.

## User Feedback & Preferences

- **"진행해" (proceed)** — after the onboarding narration + item-6 recommendation, the user gave a single go-ahead and let me drive the design + execution end-to-end (no per-step confirmation). Consistent with the standing **merge-authority delegation** (squash-on-green-CI, no per-PR re-confirm).
- The session's resume prompt (parent's Quick-Start) asked for an **onboarding narration first, then wait for go-ahead** — honored (summarized handoff, stated item pick + reasoning, verification plan, read key + adjacent files, then waited).
- Per global + project conventions: **user-facing reports in Korean, all artifacts/prompts/code in English** — followed throughout.
- The user values **the example as the real acceptance test** (VISION loop) — so I did the macOS visual smoke, not just a compile check.

## Where We're Going

- **Item 7 is the last open audit item:** `ecs/world.rs` ~51 raw `.unwrap()`s (audit counted 56 "real" ones). `[profile.release] panic="abort"` ⇒ each live unwrap is a hard game-abort (no unwind). **Approach: reason per-unwrap, do NOT blanket-replace.** Most are structural archetype invariants (`columns.get(&tid).unwrap()` — membership guarantees the column; `downcast_ref::<T>().unwrap()` — TypeId match guarantees the downcast) and are genuinely infallible → at most want an `expect("<invariant>")` for documentation. A few (entity allocation / generation / free-list paths around `world.rs:150–208`) are where real edge-input reachability needs checking + a guard + a regression test. Small batches, gate after each batch, focused review branch.
- After item 7 lands, the **seq-1 audit deferred list (items 1–8) is fully closed** (1–4 + 6 + 8 done, 5 rejected, 7 done).
- **Possible bonus (carried since seq 2):** make `RonRegistry<V>` + `RonLoadable` `pub` (crate root) so forks can register their own RON-loaded asset types.
- **Item 5 reframed (future feature, not a refactor):** a cross-platform **audio facade** (so games write one audio path instead of native-fn + wasm-no-op-stub) — `/add-feature-example` with a game that plays audio on both native and web.
- **If item 6 ever needs the HDR/linear render-target half:** that's a real feature — add a format-matched sprite pipeline variant (or a dedicated offscreen pipeline) so a non-surface-format RT can be rendered into. Bigger; its own session.

## Risks & Blockers

- **None blocking.** main is clean + green at v0.50.0.
- Item 7 is the riskiest remaining audit item by blast radius (`panic="abort"`), but it's optional and self-paced — not a blocker.
- The dungeon-merchant wishlist board is EMPTY (next ID EW-002) — **read it first each session**; a new EW request would preempt item 7.

## Open Questions

- Does the user want **item 7 next** (closes the audit list) or to pause the audit arc? (Per the parent's cadence, the user drives item selection at each seam.)
- `load_image_with_format`'s **runtime-after-init** case (loading a format texture during gameplay, not at startup) currently falls back to srgb (format only applies via the `pending_textures` path at GPU init). Worth supporting if a real use case appears — would need threading format through the AssetServer per-frame upload + a Sprite-key story. Left as a documented limitation.

## Quick Start for Next Session

```bash
# Sync + verify clean/green
git checkout main && git pull --ff-only        # expect main @ 23285af or later (this handoff's docs PR)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo $?   # must be 0

# Check the game wishlist board FIRST (a new EW request preempts the audit backlog)
cat ../dungeon-merchant/docs/engine-wishlist.md    # ACTIVE should be empty (next ID EW-002)

# Key files for item 7 (the last open audit item)
#   src/ecs/world.rs            — the ~51 unwraps; query internals (infallible) vs alloc/generation (check reachability)
#   plans/handoffs/HANDOFF_engine-audit-fixes_2026-06-22.md  — Appendix C item 7 reasoning + panic=abort lens
# Live engine state + gotchas
#   memory engine-current-state (seq 67)

# Next action
#   Pick item 7 (ecs/world.rs unwrap hardening) — reason per-unwrap, guard+test only where an invariant
#   can actually break, expect("<invariant>") elsewhere; small batches, gate after each; focused review branch.
#   OR a wishlist EW request if one has appeared.
```

---

## Session Closed

**Closed at:** 2026-06-22
**Session status:** Handed off to next session.
**Code work:** the feature landed via PR **#193** (v0.50.0, merge commit `23285af`) — already on main before this handoff.
**Landed:** this handoff doc lands on `main` via its own `docs(handoff)` PR (matching seq-2's #187 and seq-3's #192). Memory `engine-current-state` is at seq 67; `MEMORY.md` index refreshed.
