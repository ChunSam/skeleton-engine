# `render_format_query` shipped to the web (wasm harness + WebGL2 self-check smoke)

**Date:** 2026-06-24
**Status:** COMPLETED (code PR #235 merged; this handoff lands as its own `docs(handoff)` PR)
**Bead(s):** none (`bd` unavailable; engine work tracked via the dungeon-merchant wishlist board)
**Epic:** none — a user-chosen follow-up from the recommended next-work offers
**Chain:** `render-format-query` seq `2` (follow-up to seq 1, the v0.65.0 feature — same chain)
**Parent:** `HANDOFF_render-format-query_2026-06-24.md` (seq 1)
**Next:** nothing queued. Wishlist board ACTIVE is empty (next free ID **EW-004**) → next session read `../dungeon-merchant/docs/engine-wishlist.md` FIRST, then ASK for direction if still empty.

---

## The Goal

After seq 1 (the v0.65.0 render-format renderability query feature) merged, the user asked for the recommended next task and picked the **top recommendation: ship `render_format_query` to the web** via `/ship-wasm-example`. Rationale: the query's whole reason for existing is the **WebGL2** case — a float render target like `Rgba16Float` is renderable there only with `EXT_color_buffer_float`. On desktop (Metal/Vulkan/DX12) nearly everything is renderable, so the native smoke can't exercise the cross-platform / WebGL2 path that motivated the feature. A web harness + headless smoke proves the query actually works on the backend it was built for.

## Where We Are

- **main @ `7979d72`, package v0.66.0, CLAUDE.md header v1.6.141, clean + green, no open PRs.** Code PR **#235** merged (squash, `chore(examples)`); branch deleted; main synced.
- The `render_format_query` example now runs in a browser with the **same engine code** as native, and a headless WebGL2 smoke asserts the query is correct.

## Public API surface

**None** — no `src/` (library) change. This is an example/tooling release:
- A wasm-only `#[wasm_bindgen] pub fn run_render_format_query()` entry point on the example.
- `examples/render_format_query/web/{build.sh, index.html}` (the `cargo build --example` + `wasm-bindgen` harness; `pkg/` gitignored).
- `scripts/render_format_query_smoke.sh` (optional local headless check, not a CI gate).
- `docs/WASM_SMOKES.md` documents the new smoke.

**Versioning:** v0.66.0 MINOR — matches the precedent that shipping an example to the web is a MINOR even with no library change (`hdr_render_target` web, v0.60.0, #214).

## How it works (the shipped design)

- **Example restructure:** the example's app construction moved into a shared `fn build_app() -> App` so both the native `main()` (`env_logger::init(); build_app().run();`, cfg'd `not(wasm32)`) and the wasm entry (`console_error_panic_hook::set_once(); build_app().run();`) reuse it. On wasm `fn main() {}` is the required-but-empty stub.
- **Self-check verdict:** the `QuerySystem` one-shot block (already logging the capability table via `log::info!`) gained a `#[cfg(target_arch = "wasm32")]` branch that asserts two **backend-independent** invariants and writes the verdict to `document.title`:
  1. `caps.supports_render_target(caps.surface_format())` — the surface format **must** be a renderable color render target (it is the on-screen target).
  2. `!caps.supports_render_target(Bc1RgbaUnorm)` — a block-compressed format is **never** a color render attachment.
  → `RENDER_FORMAT_QUERY_CHECK: PASS (2/2)` (or `FAIL (n/2)`). These hold on any backend, so the smoke is robust regardless of whether `Rgba16Float` happens to be renderable in the test browser.
- **Smoke** (`scripts/render_format_query_smoke.sh`): combines the two existing smoke models — the **WebGL2/SwiftShader render flags** from `hdr_web_smoke.sh` (`--enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader`) + the **DevTools page-title read** from `wasm_save_smoke.sh` (`--remote-debugging-port`, poll `/json` for the verdict). Orphan-safe server (`python3 -m http.server --directory`, port guard, `kill -0` liveness). Boots via `?autostart=1`.

## What We Tried (process)

1. **Onboarding/recommendation** — board empty; user asked for the recommended next task; I recommended the web harness (top of the un-queued offers) and the user picked it.
2. **Prereqs** — wasm-bindgen crate 0.2.122 == CLI 0.2.122; wasm32 target installed; confirmed `wasm-bindgen`/`web-sys`/`console_error_panic_hook` are wasm-target deps usable from examples; `web-sys` has the `Window`+`Document` features for `document.set_title`.
3. **Implemented** — example wasm entry + `build_app()` refactor + wasm-only self-check; `web/build.sh` + `web/index.html` (modeled on `positional_audio/web`); ran `build.sh` (bundle generated); wrote + ran the smoke.
4. **Verified** — native + wasm example builds; full verify gate green (twice, around the `/ship` bump); headless WebGL2 smoke **PASS (2/2)**.
5. **Landed via `/land-pr`** → `/ship` v0.66.0 → PR #235 → CI 4/4 green → squash-merge.

No dead ends.

## Key Decisions

- **Self-check on backend-independent invariants, not on `Rgba16Float`.** Modern browsers usually have `EXT_color_buffer_float` (so `Rgba16Float` reads `yes` even on web), and SwiftShader supports it too — so asserting "Rgba16Float is non-renderable on web" would be wrong/flaky. Asserting "surface renderable + compressed rejected" is true on every backend and still proves the query discriminates and runs cross-platform.
- **v0.66.0 MINOR, no library change** — follows the `hdr_render_target`-to-web precedent (v0.60.0).
- **Combined smoke model** (SwiftShader render flags + DevTools title read) because this example is a hybrid: it renders via WebGL2 *and* reports a verdict, unlike the pure-compute `wasm_save` (title only) or render-only `hdr_web` (screenshot only).

## Evidence & Data

- **Verify:** `./scripts/verify.sh` → `VERIFY_EXIT=0`, "all checks passed ✓" (re-run after the bump).
- **Bundle:** `examples/render_format_query/web/build.sh` emits `render_format_query.js` (149 KB) + `render_format_query_bg.wasm` (12.7 MB); `pkg/` gitignored (`git check-ignore` confirmed).
- **Headless WebGL2 smoke:** `scripts/render_format_query_smoke.sh` → `RENDER_FORMAT_QUERY SMOKE: PASS — RENDER_FORMAT_QUERY_CHECK: PASS (2/2)` (example boots + renders on SwiftShader WebGL2; surface renderable + compressed rejected).
- **CI #235:** 4/4 green — Test (native) 4m23s, Build (WASM) 1m9s, Package dry-run 1m5s, Rustdoc 41s. `mergeStateStatus=CLEAN`.
- **Merge:** `gh pr merge 235 --squash --delete-branch`; `git pull --ff-only` → `7979d72`.

## Gotchas / discoveries

- **Examples can use the package's normal `[dependencies]`, not just `[dev-dependencies]`.** `wasm-bindgen`/`web-sys`/`js-sys`/`console_error_panic_hook` live under `[target.'cfg(target_arch = "wasm32")'.dependencies]` (regular deps, wasm-gated) yet are usable from the example's wasm entry — examples/tests/benches get both dependency tables. (The existing `positional_audio`/`audio_facade` wasm examples already rely on this.)
- **`web-sys` is feature-gated;** `document.set_title` needs the `Document` feature — already enabled in `Cargo.toml`. No `console` feature, so `log::info!` on wasm no-ops (no logger installed) — that's fine; the canvas table + the tab-title verdict are the visible output.
- **Self-check invariants must be backend-independent.** (See Key Decisions — a `Rgba16Float`-based assertion would be browser-dependent.)
- **Smoke teardown prints `Terminated: 15` for the bg python/chrome** — that's the cleanup trap (`kill`), not a failure; the verdict line is the real result.

## Where We're Going

**Nothing is queued.** Both the feature (seq 1) and its web harness (seq 2) are shipped.

**Next session, in order:**
1. Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (the game session may have filed EW-004+).
2. If still empty → **ASK** the user for direction (standing rule; do not start backlog speculatively).
3. Remaining un-queued offers if asked: a fuller bloom (mip-chain / dual-filter) if the half-res Gaussian proves too coarse; a `bloom` web harness via `/ship-wasm-example`; hearing the audio-facade web demos by ear.

## Related Handoffs (reference only)

- `HANDOFF_render-format-query_2026-06-24.md` — seq 1, the v0.65.0 feature this ships to the web (the **parent**).
- `HANDOFF_followup-batch2-hdr-render-arc_2026-06-23.md` — the HDR arc whose `hdr_render_target` web shipping (v0.60.0, #214) is the precedent followed here.

---

## Session Closed
**Closed at:** 2026-06-24 (KST)
**Commit:** code landed as #235 (`7979d72`); this handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off to next session
