# Phase 3 shipped: images from in-memory bytes (`App::load_image_bytes`) for `include_bytes!` single-file builds (v0.128.0)

**Date:** 2026-07-15
**Status:** COMPLETED
**Bead(s):** none (no `bd` in this repo)
**Epic:** none
**Chain:** `asset-root-windows` seq `3`
**Parent:** `plans/handoffs/HANDOFF_asset-root-windows_loud-loader-failures_2026-07-14.md` (seq 2)
**Prior chain:** `HANDOFF_asset-root-windows_downstream-bug-report_2026-07-14.md` (seq 1) > `HANDOFF_asset-root-windows_loud-loader-failures_2026-07-14.md` (seq 2) > this

---

## Stale References

Everything the parent (and grandparent) named still exists and was used this session: `asset_path::resolve`, `record_failure`, `asset_failures`, `set_strict_assets`, `App::load_image`, `AssetServer::load_image`, `pending_textures`, `image_assets_for_gpu`, `upload_asset_server_images_to_gpu`, `asset_key`, `alloc_id`, `magenta_fallback`, `decode_image_with_state`, `Handle::path`/`path_arc`, `Sprite::with_handle`/`textured`/`textured_with_handle`. No stale identifiers.

## Since Last Handoff

- **Parent's plan was: land seq-2's handoff (done, #362), then read both request channels; if empty, ASK — the shelf was down to standing candidates.** This session followed it exactly. Both channels were empty of open *engine* work: dungeon-merchant's board has EW-007/008 at `Shipped` (ball with the game to verify + `[x]`), rust-survivors is `_None._`. So the direction question fired, and the **user chose "A" = Phase 3 (`App::load_image_bytes`)** from the three standing candidates.
- **Phase 3 was the parent's #1 standing candidate, and it is now DONE.** The parent (and grandparent seq-1) both listed `App::load_image_bytes` as the natural next feature — "completes Request A's third bullet, enables `include_bytes!` single-file builds; audio side already done (`AudioManager::play_bytes`)." Shipped as v0.128.0 / PR #363.
- **The parent's explicit warning was honored before any code was written.** Grandparent seq-1 Open Questions: *"Byte-sourced images need a key that the renderer's texture cache resolves like a path but never reads from disk … that touches the exact identity/cache-key machinery this session was careful not to disturb. Worth designing before building."* This session read that first, mapped the two upload paths, and designed the verbatim-key seam before touching code.
- **The chain's remaining candidates are now just two, both non-urgent:** the vorbis/`.ogg` decode test gap, and hot-reload-under-an-asset-root (watchers still register the caller's path — the one EW-008 acceptance clause the engine does not meet, disclosed to the game in seq 2).
- **Trajectory:** the `asset-root-windows` chain has now shipped its whole arc — Phase 1 (rodio/Windows, seq-1 #358), Phase 2 (asset roots + loud image failures, seq-1 #359), loud failures for *every* loader (seq-2 #361), and Phase 3 (byte-sourced images, this seq #363). The downstream bug-report that started it is fully served; what remains is self-picked polish.

## Reference Documents

- `CLAUDE.md` — conventions, module map, verify-gate rules. **Updated this session** (header → v1.6.221 / package v0.128.0; the `src/asset.rs` module-map row extended to document `load_image_bytes`).
- `docs/VISION.md` — the feature+example loop ("the example is the acceptance test"). Leaned on it: `examples/embedded_image` *is* the Phase 3 acceptance test.
- `docs/CHANGELOG.md` — 0.128.0 entry written this session.
- Grandparent seq-1 handoff — the cache-key constraint + the dual-subsystem `App::load_image` analysis (Code Analysis section). **Read this before ever touching image identity again.**

---

## The Goal

Ship **Phase 3** of the `asset-root-windows` chain: `App::load_image_bytes(key, bytes)`, so an image already held in memory — the canonical case being one embedded at compile time via `include_bytes!` — can be rendered with **no file on disk**. Every other image API in the engine is path-based (`App::load_image("assets/hero.png")` reads a file, resolved against the asset root), which is right for a game that ships an `assets/` folder next to its executable. But a small game, a jam entry, or a wasm demo often wants to ship as **one self-contained file** with its art baked in. The audio half of this already existed (`AudioManager::play_bytes`); this closes the image half.

The load-bearing constraint, inherited from the whole chain: **do not disturb the identity/cache-key machinery.** The 2026-05-29 "Unify Image Texture Cache Keys" bug (a handle keyed by the canonical path while the GPU texture cache is keyed by the relative one → every sprite renders white) is the thing this chain has been careful, three times over, not to reintroduce. Phase 3 must register a decoded image under a key that the renderer, the handle, and a `Sprite::textured(key)` lookup all agree on — without ever sending that key down a filesystem path.

End state: `App::load_image_bytes` exists, is cross-platform (works on wasm, where `include_bytes!` sidesteps async fetch), reports a corrupt embed through the same `asset_failures()` channel as a missing file, and a runnable example proves an embedded PNG renders with no external asset file.

---

## Where We Are

- **`main @ 5d5bf61`, package v0.128.0, CLAUDE.md header v1.6.221, clean tree, all gates green.**
- **Engine PR #363 — MERGED** `5d5bf61` (2026-07-15T04:13:50Z, async auto-merge, CI **6/6** including `Build (Windows / DX12)`).
- **New public API: `App::load_image_bytes(key, bytes) -> Handle<ImageAsset>`** (`src/app/assets.rs`), a thin forwarder to **`AssetServer::load_image_bytes(key, bytes)`** (`src/asset/image_loading.rs`).
- **`key` is stored VERBATIM** — the caller's string, NOT run through `asset_key` (which canonicalizes) and NOT resolved against the asset root. This is the crux (see Key Decisions): it makes the texture-cache key, `Handle::path()`, and a `Sprite::textured(key)` / `Sprite::with_handle` render lookup provably the same string.
- **New helper `decode_bytes_with_state(key, bytes) -> (ImageAsset, AssetLoadState)`** (`src/asset/image_loading.rs`, `pub(super)`, cross-platform) — mirrors the existing native-only `decode_image_with_state` minus the filesystem read: it decodes straight from memory via `image::load_from_memory`, and on a decode error records the failure and returns the magenta 1×1 fallback.
- **Does NOT push to `pending_textures`.** That vec drives the renderer's disk-read upload path (`renderer/texture.rs`); a byte image has no disk path. Instead the decoded CPU `ImageAsset` is registered in `AssetServer` (`path_to_id`/`images`/`image_load_states`), and the existing per-frame `upload_asset_server_images_to_gpu` (`src/app/schedule.rs:626`) plus the GPU-init path (`src/app/window.rs::init_gpu_renderers`, via `image_assets_for_gpu`) upload it to the renderer's texture cache under `key`, exactly like an async load.
- **Cross-platform, un-gated.** The `image` crate is in `Cargo.toml` `[dependencies]` (not target-gated, `features=["png"]`), so `image::load_from_memory` works on wasm too. `load_image_bytes` therefore compiles and runs on native AND wasm — a genuine win over `load_image`, which on wasm needs async fetch.
- **A corrupt embed reports via `asset_failures()`** (`record_failure` → `error!`, magenta fallback, `AssetLoadState::Failed`) — loud-failure parity with a missing file. Consistent with seq-2's "every loader reports" work.
- **Fully additive: no existing signature changed, no `src/lib.rs` re-export needed** (`App`, `AssetServer`, `Handle`, `ImageAsset` are already public via `load_image`).
- **Example `examples/embedded_image.rs` (new, flat/auto-discovered)** — embeds `examples/assets/player.png` (32×32, 186 bytes) via `include_bytes!` and renders it with a `Sprite::with_handle`; also feeds a deliberately-corrupt byte slice. Under `HEADLESS_SHOT` it is a 3-part self-checking acceptance test.
- **Acceptance test passes from `/`** (a working dir with no `assets/`): `OK: embedded 32x32 image decoded from include_bytes! with no external file; corrupt embed reported. failures = ["embedded/corrupt"]`, exit 0.
- **Screenshot pixel-check confirms the render**: the sprite draws in its real art color (cyan `(64,196,232)`, ~10k px), **not white** (the key-agreement invariant holds) and **not magenta** (the fallback).
- **2 new unit tests** in `src/asset/tests.rs`: `load_image_bytes_registers_a_decoded_image_under_the_verbatim_key` and `load_image_bytes_reports_a_corrupt_embed_as_a_failure`. Lib test count: **1245** (was 1243 at seq-2 close; +2).
- **One gate-relevant discipline honored:** ran `cargo fmt` before verify (the fmt-reflow trap), and read every verify exit code from a non-piped call.
- Memory bumped: `engine-current-state` → seq **183**.
- **Both downstream request channels remain empty of open engine items** (dm board EW-007/008 `Shipped`, awaiting the game's verify; rust-survivors `_None._`). Next free board ID **EW-009**.

---

## What We Tried (Chronological)

### Chunk 1 — Onboarding and the direction ask (early)

1. **User pasted a seq-2 continuation prompt** asking me to read `HANDOFF_asset-root-windows_loud-loader-failures_2026-07-14.md` and continue from "Where We're Going", with a structured onboarding narration (summarize handoff → state what to verify → read key files + explore adjacent → explain planned first action → **wait for go-ahead**).
2. **Read the seq-2 handoff, ran the verify gate, read both request channels in parallel.** Discovered the seq-2 handoff (#362) was **already merged** (tip `7b11f92`) — so "Where We're Going" step 1 (land the handoff) was already done. Verify gate: green. Both channels: empty of open engine work.
3. **Read the key files** (`src/asset_path.rs`, `src/app/editor/loading.rs`, `examples/packaged_assets.rs`) plus adjacent (`src/scripting/loading.rs`, `src/app/assets.rs`).
4. **Reported the situation in Korean and laid out the standing directive:** both channels empty → ASK. Presented the three standing candidates (A: Phase 3 `load_image_bytes`, B: vorbis/`.ogg` test gap, C: hot-reload-under-asset-root, D: propose a new area), recommended A, and waited.
5. **User replied "a진행"** — proceed with A (Phase 3).

### Chunk 2 — Mandated design research before code (mid)

6. **Read the grandparent seq-1 handoff in full** — specifically the Open Questions (Phase 3 warning) and the Code Analysis section documenting that **`App::load_image` feeds TWO independent subsystems**: `pending_textures` (renderer GPU-upload, reads from disk in `renderer/texture.rs`) AND `AssetServer::load_image` (CPU decode NOW, returns the Handle). Both key by the same string; the two keys must agree or the sprite renders white.
7. **Confirmed no byte-source image API already existed** (`grep load_image_bytes|from_bytes|load_from_memory` in `src/` → nothing).
8. **Read `src/asset/image_loading.rs`** — found `decode_image_with_state(path)` already does `std::fs::read(resolve(path))` → `image::load_from_memory(&bytes)`. So the decode-from-memory step I needed was already the second half of the existing function. And `image_assets_for_gpu()` returns `(path.to_string(), img.clone())` for every `path_to_id` entry.
9. **Traced the GPU upload seam** (`grep upload_asset_server_images_to_gpu|pending_textures|load_texture_from_image`): the GPU-init path (`window.rs:661-667`) uploads every `image_assets_for_gpu()` entry not already cached; the per-frame path (`schedule.rs:626`) does the same each frame. So an image registered in `AssetServer` under `key` gets uploaded to the renderer cache under `key` — the clean seam, no `pending_textures` needed.
10. **Read `asset_key`** (`src/asset.rs:237`) — it calls `path.canonicalize()`, which **requires the file to exist**; for a nonexistent logical key it falls back to `path.to_string_lossy()`. Decisive: for a byte key that names no file, `asset_key` is effectively identity — but it could misfire if the key coincidentally matches a real file. → **Store the byte key verbatim, skip `asset_key` entirely.**
11. **Read `src/renderer/sprite/collect.rs:67-74`** — the render key is `sprite.image_handle.map(h.path_arc()).or(sprite.texture.clone())`. So with a verbatim key, `Sprite::with_handle`, `Sprite::textured(key)`, AND `textured_with_handle(key, Some(h))` all resolve to the same string → all three patterns render correctly.
12. **Checked `Cargo.toml`** — `image` is in `[dependencies]` (line 161, un-gated, `features=["png"]`), available on **every** target incl. wasm. → `load_image_bytes` can be cross-platform, un-gated.
13. **Confirmed the example fixture:** `examples/assets/player.png` is 186 bytes, 32×32, a valid PNG (read the IHDR with Python). Small enough to embed.
14. **Presented the locked design** (verbatim key, seed AssetServer only, rely on existing per-frame upload, cross-platform, corrupt reports) and proceeded (user had already approved A).

### Chunk 3 — Implementation (mid)

15. **`AssetServer::load_image_bytes(key, bytes)`** — `key: Arc<str> = Arc::from(key.into())`; cache-hit returns the existing handle (bytes ignored on a hit); else `alloc_id()`, insert into `path_to_id`, `decode_bytes_with_state(&key, bytes)`, insert the asset + state, return `Handle { id, path: key }`.
16. **`decode_bytes_with_state(key, bytes)`** — un-gated, `image::load_from_memory(bytes)` → RGBA `ImageAsset` on success; on error `record_failure(key, "embedded image decode failed: …")` + `magenta_fallback()` + `AssetLoadState::Failed`.
17. **`App::load_image_bytes(key, bytes)`** — thin forwarder; does NOT push to `pending_textures`; rustdoc with a `no_run` example.
18. **Example `examples/embedded_image.rs`** — `include_bytes!("assets/player.png")` under key `"embedded/player"`, plus a corrupt `b"\x89PNG\r\n\x1a\n this is not really a png"` under `"embedded/corrupt"`; renders the real one with `Sprite::with_handle`; HUD shows cwd + the failures panel. Headless: 3-part assertion (embed decodes to 32×32 · valid embed not a failure · corrupt embed IS reported), written as a `problems: Vec<String>` so a bad run reports every violated clause.
19. **2 unit tests** in `src/asset/tests.rs` — the happy path encodes a real 2×2 PNG in memory (via `image::DynamicImage::write_to(Png)`) so it reads nothing from disk, then asserts `handle.path() == key` verbatim, dims `(2,2)`, `Loaded`, and a cache hit on re-load; the failure path feeds garbage and asserts `Failed` + a recorded failure keyed on a **unique** path (never on the list's length — the process-global caveat).
20. **CLAUDE.md module-map row** for `src/asset.rs` extended with the `load_image_bytes` contract + the example.

### Chunk 4 — Verification (mid/late)

21. **Fast targeted check first:** `cargo build --example embedded_image` (5.6s, clean) → `cargo test --lib load_image_bytes` (**2 passed**) → ran the example **headless from `/`**: exit 0, `failures = ["embedded/corrupt"]` (only the corrupt one — the valid embed decoded and was not reported).
22. **Screenshot pixel inspection** (`PIL` over `/tmp/embedded.png`): background `(80,80,97)`, sprite region `(64,196,232)` cyan ~10050 px, HUD text near-white. The sprite renders in its real art color — **not white, not magenta** — proving the key-agreement invariant end to end.
23. **`cargo fmt`** (the reflow trap — it re-wrapped a few `format!`/`match` lines), then **full `./scripts/verify.sh` → exit 0** (all 7 gates: fmt, clippy, wasm build, wasm clippy, test --all-targets, doctests, rustdoc).

### Chunk 5 — Ship + land (late)

24. **`/land-pr` → branched `feat/load-image-bytes`** (carried the uncommitted changes).
25. **`/ship`:** `Cargo.toml` 0.127.0 → **0.128.0** (MINOR, new public API); `cargo update -p skeleton-engine` (`Locking 0 packages`, lock version bumped in place → 0.128.0); CHANGELOG 0.128.0 entry (Added: `load_image_bytes` + example); CLAUDE.md header v1.6.220 → **v1.6.221** + package version. **Re-ran verify after the bump → exit 0.**
26. **Committed** `feat(assets): load images from in-memory bytes for include_bytes! single-file builds (v0.128.0)`; pushed `feat/load-image-bytes`.
27. **PR #363 opened; async auto-merge armed** (`gh pr merge 363 --auto --squash`). No judgment gate applied — pure Rust logic + unit tests run on CI's native job; the GPU render half I verified locally (screenshot). No OS-gated/audio/hot-reload code.
28. **Watched CI in the background** (`gh pr checks 363 --watch`): **6/6 green** — Test (native) 5m55s, Build (Windows / DX12) 2m37s, Build (WASM) 55s, Rustdoc 56s, Package dry-run 1m13s, Render tests (lavapipe) 1m10s. Auto-merge landed `5d5bf61` at 2026-07-15T04:13:50Z.
29. **Synced `main`** (`git pull --ff-only`, tip `5d5bf61`), pruned the branch, **bumped memory to seq 183**.

---

## Key Decisions

- **The byte `key` is stored VERBATIM — not through `asset_key`/canonicalize, not resolved against the asset root.** This is the whole design. `asset_key` canonicalizes, which requires the file to exist; a byte image names no file, so canonicalize would fail and fall back to the raw string in the normal case — but it could *misfire* if the key coincidentally matched a real file on disk (canonicalizing to an absolute path the caller never expects, which a raw `Sprite::textured(key)` lookup would then miss → white sprite). Storing verbatim makes the cache key, `Handle::path()`, and every `Sprite` render-key path provably equal the caller's string. It is also the most intuitive contract: *the key you pass is the key you render with.*
- **Seed only the `AssetServer`, never `pending_textures`.** `pending_textures` is the renderer's disk-read upload queue; pushing `key` there would send it to `renderer/texture.rs` → `std::fs::read(resolve(key))` → a spurious failure (and a magenta texture). Instead the decoded CPU image rides the already-existing `upload_asset_server_images_to_gpu` seam (per-frame + at GPU init), the same path async loads use. This is *why* the change is so small — the GPU-upload machinery already handles "an AssetServer image not yet in the renderer cache."
- **Cross-platform, un-gated — not native-only.** The obvious mirror of `decode_image_with_state` (which is `#[cfg(not(wasm32))]`) would have made `load_image_bytes` native-only. But that native-gating exists because the *filesystem read* is native-only, not the decode. `image::load_from_memory` and the whole GPU-upload seam work on wasm, and the `image` crate is un-gated in Cargo.toml. So `load_image_bytes` is un-gated — and that's the *point* for wasm: `include_bytes!` art works in a single-file wasm build where a path load needs async fetch.
- **A corrupt embed is a loud failure, not a silent blank.** Consistent with seq-2's philosophy (a swallowed load is indistinguishable from success). A bad `include_bytes!` (wrong bytes, truncated) records through `asset_failures()` with a magenta fallback, exactly like a missing file. This also makes the example's headless test *bidirectional* (valid decodes + not-a-failure; corrupt IS a failure) → non-vacuous by construction.
- **A new flat example, not an extension of `packaged_assets`.** `packaged_assets` tells the *asset-root + loud-failures* story (path-based loads from a foreign cwd). Embedded images are a *different* story (no path at all). Splitting them keeps each example a single lesson. `embedded_image` is flat (`examples/embedded_image.rs`), so Cargo auto-discovers it — no `Cargo.toml` `[[example]]` entry needed.
- **Embed the smallest real PNG (`player.png`, 186 bytes).** `include_bytes!` bakes the bytes into the example binary; a small asset keeps that lean. 32×32 is a real sprite (renders as a recognizable colored character), so the screenshot pixel-check is meaningful.
- **No `Result` / no `#[must_use]` surprise.** `load_image_bytes` returns a `Handle<ImageAsset>` just like `load_image` — same ergonomics, no new error-handling burden at call sites. Failures surface through `asset_failures()`, the channel the whole chain standardized on.
- **Async auto-merge, no watch-and-confirm gate.** The change is pure Rust logic covered by CI's native tests + the wasm/render/Windows jobs; the only CI-unverifiable part (the actual pixels) I checked locally before arming. Matches the standing merge delegation.

---

## Evidence & Data

### The two GPU-upload paths (why the byte seam needs no `pending_textures`)

| Path | Where | What it does | Covers the byte image? |
|---|---|---|---|
| `pending_textures` drain | `window.rs::init_gpu_renderers:651-660` | reads each queued path from **disk** via `load_texture_with_format` | ❌ — byte images are never queued here |
| AssetServer images @ init | `window.rs:661-667` | uploads every `image_assets_for_gpu()` entry not already cached, keyed by its `path_to_id` key | ✅ — the byte image is registered here |
| AssetServer images per-frame | `schedule.rs:626` (`upload_asset_server_images_to_gpu`) | same `if !has_texture_key` upload, every frame | ✅ — covers images registered after GPU init |

### The render-key resolution (why a verbatim key renders under all three Sprite ctors)

`src/renderer/sprite/collect.rs:69-74`:
```rust
let tex_key: Arc<str> = sprite
    .image_handle.as_ref().map(|h| h.path_arc())   // handle path (= path_to_id key)
    .or_else(|| sprite.texture.clone())            // else the raw texture string
    .unwrap_or_else(|| Arc::from(""));
```

| Sprite ctor | render key | matches cache key (`key`)? |
|---|---|---|
| `Sprite::with_handle(h)` | `h.path_arc()` = `key` | ✅ |
| `Sprite::textured(key)` | raw `key` | ✅ (because `key` stored verbatim) |
| `Sprite::textured_with_handle(key, Some(h))` | `h.path_arc()` = `key` | ✅ |

Because the byte key is stored verbatim, all three equal the renderer cache key → no white-sprite bug.

### `asset_key` behavior (why the byte path skips it)

`src/asset.rs:237` — `asset_key(path)`:
```rust
if let Ok(canonical) = path.canonicalize() {   // REQUIRES the file to exist
    return canonical.to_string_lossy().as_ref().into();
}
path.to_string_lossy().as_ref().into()          // fallback for a nonexistent path
```
A logical byte key (`"embedded/player"`) names no file → `canonicalize` fails → raw string. Harmless in the normal case, but a key that *coincidentally* matches a real file would canonicalize to an absolute path a raw `Sprite::textured(key)` would miss. `load_image_bytes` therefore never calls `asset_key`.

### Screenshot pixel histogram (`/tmp/embedded.png`, run from `/`)

| Color (RGBA) | Count | What it is |
|---|---|---|
| `(80, 80, 97, 255)` | 278590 | default clear color (background) |
| `(64, 196, 232, 255)` | 10050 | **the sprite** — real art color (cyan), scaled 160px |
| `(24, 40, 64, 255)` | 4450 | darker sprite pixels |
| `(250, 250, 255, 255)` | 1200 | HUD text |
| `(235, 230, 199, 255)` | 454 | HUD "ink" text |

Sprite center sample = `(64, 196, 232)`. **Not white (invariant holds), not magenta (fallback).**

### The headless acceptance test, both directions

| Run | Command | Result |
|---|---|---|
| From `/` (no `assets/`) | `cd / && HEADLESS_SHOT=… embedded_image` | `OK: embedded 32x32 image decoded from include_bytes! with no external file; corrupt embed reported. failures = ["embedded/corrupt"]` → exit 0 |
| Failure clause (structural) | corrupt embed clause | fails the run if a bad `include_bytes!` is NOT in `asset_failures()` |

### Gate history

| Run | Result | Notes |
|---|---|---|
| targeted: build example | 0 | `cargo build --example embedded_image`, 5.6s |
| targeted: 2 unit tests | 0 | both new tests pass |
| targeted: headless from `/` | 0 | exit 0, only corrupt reported |
| full verify (pre-ship) | 0 | 7 gates, after `cargo fmt` |
| full verify (post-`/ship` bump) | 0 | lock + doc re-checked |
| CI #363 | 6/6 | incl. `Build (Windows / DX12)`, `Render tests (lavapipe)` |

### CI #363 — the 6 required checks

| Check | Result |
|---|---|
| Test (native) | ✅ 5m55s (last, as always) |
| Build (Windows / DX12) | ✅ 2m37s |
| Build (WASM) | ✅ 55s |
| Rustdoc | ✅ 56s |
| Package dry-run | ✅ 1m13s |
| Render tests (lavapipe) | ✅ 1m10s |

### Test count trajectory

| Point | Lib tests |
|---|---|
| seq-2 close (v0.127.0) | 1243 |
| **seq-3 close (v0.128.0)** | **1245** (+2: verbatim-key registration, corrupt-embed failure) |

### Merge log

| Repo | PR | Commit | What |
|---|---|---|---|
| skeleton-engine | **#363** | `5d5bf61` | v0.128.0 — `App::load_image_bytes` + example `embedded_image` |

### The example fixture (real, embedded)

`examples/assets/player.png` — 32×32, 186 bytes, valid PNG (IHDR verified). Chosen for size (baked into the example binary via `include_bytes!`) and because it renders as a recognizable colored sprite (makes the pixel-check meaningful).

### The two new functions, verbatim (primary artifact)

`AssetServer::load_image_bytes` (`src/asset/image_loading.rs`):
```rust
pub fn load_image_bytes(&mut self, key: impl Into<String>, bytes: &[u8]) -> Handle<ImageAsset> {
    let key: Arc<str> = Arc::from(key.into());
    if let Some(&id) = self.path_to_id.get(&key) {
        return Handle { id, path: key, _marker: PhantomData };   // cache hit — bytes ignored
    }
    let id = alloc_id();
    self.path_to_id.insert(Arc::clone(&key), id);
    let (asset, state) = decode_bytes_with_state(&key, bytes);
    self.images.insert(id, asset);
    self.image_load_states.insert(id, state);
    Handle { id, path: key, _marker: PhantomData }
}
```

`decode_bytes_with_state` (`src/asset/image_loading.rs`, `pub(super)`, un-gated):
```rust
pub(super) fn decode_bytes_with_state(key: &str, bytes: &[u8]) -> (ImageAsset, AssetLoadState) {
    match image::load_from_memory(bytes) {
        Ok(img) => {
            let rgba = img.to_rgba8();
            let (w, h) = rgba.dimensions();
            (ImageAsset { data: Arc::new(rgba.into_raw()), width: w, height: h }, AssetLoadState::Loaded)
        }
        Err(e) => {
            let msg = format!("embedded image decode failed '{key}': {e}");
            crate::asset_path::record_failure(key, format!("embedded image decode failed: {e}"));
            (magenta_fallback(), AssetLoadState::Failed(msg))
        }
    }
}
```

The `App` forwarder (`src/app/assets.rs`) is a one-liner over `AssetServer::load_image_bytes` — the only subtlety is what it *doesn't* do: it never pushes to `self.pending_textures` (contrast `App::load_image`, which does).

### Rejected: the native-only mirror

The obvious implementation mirrors `decode_image_with_state`, which is `#[cfg(not(target_arch = "wasm32"))]` — so a copy-paste would have made `load_image_bytes` native-only. **Rejected.** That gate exists for the *filesystem read*, not the decode. `image::load_from_memory` compiles and runs on wasm (the `image` crate is un-gated in Cargo.toml), and the GPU-upload seam is un-gated too. Making `load_image_bytes` cross-platform is not just possible — it is the *reason* the feature matters on wasm, where a path load needs async fetch and an embedded one does not.

---

## Code Analysis

- **`AssetServer::load_image_bytes(key: impl Into<String>, bytes: &[u8]) -> Handle<ImageAsset>`** (`src/asset/image_loading.rs`, public) — `key` → `Arc<str>` verbatim; cache hit on `path_to_id` returns the existing `Handle` (bytes ignored); miss allocates an id, inserts `path_to_id[key]=id`, decodes via `decode_bytes_with_state`, inserts `images[id]` + `image_load_states[id]`, returns `Handle { id, path: key, _marker }`. No `#[cfg]` — cross-platform.
- **`decode_bytes_with_state(key: &str, bytes: &[u8]) -> (ImageAsset, AssetLoadState)`** (`src/asset/image_loading.rs`, `pub(super)`) — `image::load_from_memory(bytes)` → `to_rgba8()` → `ImageAsset { data: Arc<Vec<u8>>, width, height }` + `Loaded`; on `Err`, `record_failure(key, …)` + `magenta_fallback()` + `Failed(msg)`. Un-gated (mirrors `decode_image_with_state` minus the `std::fs::read`).
- **`App::load_image_bytes(key: impl Into<String>, bytes: &[u8]) -> Handle<ImageAsset>`** (`src/app/assets.rs`) — forwards to `AssetServer::load_image_bytes`. Deliberately does **not** touch `pending_textures` (contrast `load_image`, which pushes there).
- **`AssetServer::image_assets_for_gpu(&self) -> Vec<(String, ImageAsset)>`** (`src/asset/image_loading.rs:113`) — iterates `path_to_id`, returns `(key.to_string(), img.clone())` for each image; the seam that carries the byte image to the renderer cache under `key`.
- **`App::upload_asset_server_images_to_gpu(&mut self)`** (`src/app/assets.rs:186`) — for each `image_assets_for_gpu()` entry, `if !sr.has_texture_key(&path) { sr.load_texture_from_image(…, &path, &asset) }`. Called per-frame at `schedule.rs:626`, and mirrored at GPU init in `window.rs:661-667`.
- **`asset_key(path) -> Arc<str>`** (`src/asset.rs:237`) — canonicalizes if the file exists, else returns the raw string. Deliberately NOT used for byte keys.
- **`Handle { id, path: Arc<str>, _marker }`** — `path()` → `&str`, `path_arc()` → `Arc<str>` (O(1) refcount bump, used as the render key in `collect.rs`).
- **The verify gate** (`./scripts/verify.sh`, in order): fmt → clippy --all-targets → wasm build (lib+bins) → wasm clippy --lib → test --all-targets → test --doc → rustdoc `-D warnings`. Read its exit from a non-piped call.

---

## Files Changed

### Source code
- `src/asset/image_loading.rs` — **NEW `AssetServer::load_image_bytes`** (public) + **NEW `decode_bytes_with_state`** (`pub(super)`, cross-platform). No existing fn changed.
- `src/app/assets.rs` — **NEW `App::load_image_bytes`** forwarder (with a `no_run` rustdoc example). Does not push to `pending_textures`.

### Tests
- `src/asset/tests.rs` — **+2 tests**: `load_image_bytes_registers_a_decoded_image_under_the_verbatim_key` (encodes a real 2×2 PNG in memory; asserts verbatim key + dims + `Loaded` + cache hit) and `load_image_bytes_reports_a_corrupt_embed_as_a_failure` (keyed on a unique path, never on list length).

### Examples
- `examples/embedded_image.rs` — **NEW, flat/auto-discovered.** Renders an `include_bytes!` 32×32 PNG with no external file; feeds a corrupt embed; `HEADLESS_SHOT` = a 3-part self-checking acceptance test.

### Release paperwork
- `Cargo.toml` / `Cargo.lock` — 0.127.0 → **0.128.0**.
- `docs/CHANGELOG.md` — the 0.128.0 entry (Added: `load_image_bytes` + example).
- `CLAUDE.md` — header v1.6.220 → **v1.6.221**; the `src/asset.rs` module-map row extended (documents `load_image_bytes`, the verbatim-key invariant, the no-`pending_textures` seam, cross-platform, `decode_bytes_with_state`, and the example).

### Memory
- `engine-current-state.md` — seq **183** entry; `main @ 5d5bf61`, v0.128.0, header v1.6.221.

---

## User Feedback & Preferences (REQUIRED)

- **Structured onboarding request** (the pasted seq-2 continuation): "narrate your onboarding … Then wait for my go-ahead before executing." The expectation is: onboard visibly, present the planned first action, and **stop** — do not start coding until told. Honored (waited after presenting the direction options).
- **"a진행"** — terse approval to proceed with candidate **A** (Phase 3 `load_image_bytes`) from the direction ask. Terse approval is normal; it does not invite a re-plan.
- **"handoffplan"** (the current invocation) — write the session handoff, then a plan for the next session, then commit and close. (This file is the handoff half.)
- **Standing: the board takes priority; if both channels are empty, ASK for direction.** Honored — this session asked, the user picked A, then it shipped.
- **Standing: merge authority is delegated** — squash on green CI, async auto-merge default, no per-PR re-confirm. Honored (armed `--auto` on #363).
- **Standing: user-facing reports in Korean; code, docs, commit messages, PR bodies, and handoffs in English.** Followed throughout.

---

## Where We're Going

*(This handoff is paired with `PLAN_asset-root-windows_embedded-images_2026-07-15.md`, which defines the next session's work in phases. Summary below; the plan is the authority.)*

1. **The board takes priority — check both channels FIRST.** `../dungeon-merchant/docs/engine-wishlist.md` (next free ID **EW-009**; EW-007/008 still `Shipped`, ball with the game) and `../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md` (`_None._`). A newly-filed request preempts anything below.
2. **If both empty, the chain's two remaining candidates (self-picked, both non-urgent):**
   - **Hot-reload under an asset root** — the notify watchers still register the *caller's* path, not `resolve(path)`, so F2 hot-reload is only guaranteed dev-from-repo-root. This is the one EW-008 acceptance clause the engine does not meet (disclosed to the game in seq 2). A concrete, bounded engine change; the *recommended* plan target (see the plan).
   - **The vorbis/`.ogg` decode test gap** — nothing in the engine exercises ogg decoding after the rodio 0.22 symphonia swap. Blocked on a license-clean fixture (`afconvert` can't encode ogg; the game's music is licensed and can't be committed to an MIT repo).
3. **Or propose a NEW breadth area** if the user wants to leave the `asset-root-windows` chain (more procgen modes / audio-driven hooks / a 2nd capstone game / tilemap streaming).

---

## Risks & Blockers

- **`load_image_bytes`'s wasm path is compiled but not runtime-exercised.** The wasm gate builds lib+bins only (not examples), and CI has no headless-wasm image test. The decode (`image::load_from_memory`) and the upload seam are the same on both targets, so the risk is low — but if a wasm consumer reports a blank embedded sprite, the wasm GPU-upload path is where to look. A `wasm_smoke.sh`-style browser check would close it.
- **The hot-reload watcher gap is still open** (carried from seq 2). `resolve()` runs at the fs read only; the watchers register the caller's path. A packaged build that hot-reloads watches the wrong file. Nobody has hit it (hot-reload is a dev activity). It is candidate #1 for the next session.
- **Vorbis/`.ogg` is still untested anywhere** (carried from seq 1). The rodio 0.22 codec swap was verified for mp3 + wav only.
- **`dungeon-merchant` has no CI and no branch protection** (private repo without Pro) — its board PR convention is discipline, not enforcement. Don't read "CLEAN" there as "verified".
- **`rust-survivors` has auto-merge DISABLED** — merge its PRs by hand after watching checks (unlike the engine repo).

## Open Questions

- **Should there be an atlas / audio parity for byte sources?** `load_atlas` is still path-only; `load_image_bytes` + a `TextureAtlas::from_handle`-style helper would let a jam game embed a spritesheet. Not requested; note it if a jam build needs it.
- **Should `load_image_bytes` grow a `_with_format` variant** (like `load_image_with_format`, for a linear/data texture from bytes)? Trivial to add if a use case appears; skipped as speculative.
- **Should the two remaining chain candidates be folded into one "asset polish" pass, or does the chain close here?** The downstream bug-report that started `asset-root-windows` is fully served; the remainder is self-picked. The user may prefer a fresh direction.

---

## Quick Start for Next Session

```bash
# Nothing is dangling — engine #363 is merged, main is clean.

# 1. Engine state
cd ~/Projects/skeleton-engine
git log --oneline -3     # expect 5d5bf61 (v0.128.0) at the tip
git status -s            # expect clean

# 2. READ THE BOARD FIRST — both channels
#   ../dungeon-merchant/docs/engine-wishlist.md          (next free ID EW-009; EW-007/008 awaiting the game's verify)
#   ../rust-survivors/docs/ENGINE_CHANGE_REQUESTS.md     (currently _None._)
#   A newly-filed request preempts the plan below.

# 3. The plan for this session
cat plans/handoffs/PLAN_asset-root-windows_embedded-images_2026-07-15.md

# 4. Read first (if taking the hot-reload candidate)
#   src/asset_path.rs                  — resolve() + the process-global caveats
#   src/asset.rs                       — AssetServer, watch_path / watched_paths, the notify watcher
#   src/asset/image_loading.rs         — load_image's watcher registration (the pattern to fix)
#   src/asset/hot_reload.rs            — poll_reloads / the reload dispatch

# 5. Verify current state (read the exit code — do NOT pipe or `;`-chain it)
./scripts/verify.sh > /tmp/verify.log 2>&1; echo "VERIFY_EXIT=$?"

# 6. Re-prove Phase 3 still works
cargo build --example embedded_image
(cd / && HEADLESS_SHOT=/tmp/embedded.png "$OLDPWD/target/debug/examples/embedded_image")
# expect: OK: embedded 32x32 image decoded from include_bytes! …; exit 0

# 7. Next action
#   Follow the plan: board-check gate, then (if empty) the hot-reload-under-asset-root feature.
```
