# `bloom` shipped to the web (wasm harness + WebGL2 render smoke)

**Date:** 2026-06-24
**Status:** COMPLETED (code PR #239 merged; this handoff lands as its own `docs(handoff)` PR)
**Bead(s):** none (`bd` unavailable; engine work tracked via the dungeon-merchant wishlist board)
**Epic:** none — a user-chosen follow-up from the recommended next-work offers
**Chain:** `bloom-mip-chain` seq `2` (follow-up to seq 1, the v0.67.0 feature — same chain)
**Parent:** `HANDOFF_bloom-mip-chain_2026-06-24.md` (seq 1)
**Next:** nothing queued. Wishlist board ACTIVE is empty (next free ID **EW-004**) → next session read `../dungeon-merchant/docs/engine-wishlist.md` FIRST, then ASK for direction if still empty.

---

## The Goal

After seq 1 (the v0.67.0 mip-chain "dual filter" bloom) merged, the board was empty; the user was offered the un-queued candidates and picked **ship `bloom` to the web** via `/ship-wasm-example`. Rationale: the bloom pass is the most worthwhile example to put on the web because it renders the scene into an `Rgba16Float` **HDR intermediate** and blurs the highlights through a **pyramid of `Rgba16Float` mip render targets** — all of which are usable on WebGL2 only with the `EXT_color_buffer_float` extension. So whether the whole HDR + mip-chain pipeline actually runs is exactly the cross-platform question a browser build answers; the native Metal smoke can't exercise it.

## Where We Are

- **main @ `0e6de2a`, package v0.68.0, CLAUDE.md header v1.6.143, clean + green, no open PRs.** Code PR **#239** merged (squash, `feat(examples)`); branch deleted; main synced.
- The `bloom` example now runs in a browser with the **same engine code** as native, and a headless WebGL2 smoke confirms the HDR + mip-chain bloom pipeline renders.

## Public API surface

**None** — no `src/` (library) change. This is an example/tooling release:
- A wasm-only `#[wasm_bindgen] pub fn run_bloom()` entry point on the example.
- `examples/bloom/web/{build.sh, index.html}` (the `cargo build --example` + `wasm-bindgen` harness; `pkg/` gitignored).
- `scripts/bloom_web_smoke.sh` (optional local headless check, not a CI gate).
- `docs/WASM_SMOKES.md` documents the new smoke.

**Versioning:** v0.68.0 MINOR — matches the precedent that shipping an example to the web is a MINOR even with no library change (`hdr_render_target` web v0.60.0 #214; `render_format_query` web v0.66.0 #235).

## How it works (the shipped design)

- **Example restructure:** the example's app construction moved into a shared `fn build_app() -> App` so both the native `main()` (`build_app().run();`, cfg'd `not(wasm32)`) and the wasm entry (`console_error_panic_hook::set_once(); build_app().run();`) reuse it. On wasm `fn main() {}` is the required-but-empty stub. The native `main()` stays real — `bloom` runs both natively and on the web.
- **Self-check verdict (render-survives, not an invariant pair):** `BloomDemo` gained two `#[cfg(target_arch = "wasm32")]` fields (`frames: u32`, `reported: bool`). Each frame the run() method counts up; after **30 frames** it writes `BLOOM_WEB_CHECK: PASS (1/1)` to `document.title`. The verdict is *implicit proof of rendering*: if the HDR `Rgba16Float` intermediate or any mip-pyramid target were not renderable, the pipeline would panic at boot, `console_error_panic_hook` fires, and **no title ever appears** → the smoke FAILs on "no verdict." (Unlike `render_format_query`, there are no backend-independent boolean invariants to assert here — the meaningful signal is "the float-target HDR+bloom pipeline ran N frames without a wgpu validation error/panic.")
- **Smoke** (`scripts/bloom_web_smoke.sh`): the `render_format_query_smoke.sh` model — **SwiftShader WebGL2 flags** (`--enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader`) + **DevTools page-title read** (`--remote-debugging-port`, poll `/json` for the verdict). Orphan-safe server (`python3 -m http.server --directory`, port guard, `kill -0` liveness). Boots via `?autostart=1`.

## What We Tried (process)

1. **Onboarding** — verified baseline (main @ `04fa16f`, v0.67.0, clean+green); read the wishlist board (ACTIVE empty, next free ID EW-004); offered the un-queued candidates; user picked the bloom web demo.
2. **Read adjacent files first** — `examples/bloom.rs`, the `render_format_query` example + web harness + smoke (the structural precedent), confirmed toolchain (wasm-bindgen crate 0.2.122 == CLI 0.2.122, wasm32 target installed, `pkg/` gitignored).
3. **Implemented** — example wasm entry + `build_app()` refactor + wasm-only self-check; `web/build.sh` + `web/index.html` (modeled on `render_format_query/web`); ran `build.sh` (bundle generated); wrote + ran the smoke.
4. **Verified** — full verify gate green (twice, around the `/ship` bump); the wasm example bundle builds; headless WebGL2 render smoke **PASS (1/1)**.
5. **Landed via `/land-pr`** → `/ship` v0.68.0 → PR #239 → CI 4/4 green → squash-merge → main synced to `0e6de2a`.

No dead ends.

## Key Decisions

- **A "render-survives" self-check, not a boolean-invariant pair.** `render_format_query` could assert two backend-independent truths (surface renderable + compressed rejected). Bloom has no such cross-backend boolean; the thing worth proving is that the **float-target HDR + mip-chain pipeline boots and renders on WebGL2**, so the verdict is gated on surviving 30 frames (a boot panic ⇒ no verdict ⇒ FAIL).
- **v0.68.0 MINOR, no library change** — follows the example-to-web precedent (`hdr_render_target` v0.60.0, `render_format_query` v0.66.0).
- **Kept the native `main()` real** (dual-target example) rather than the print-only stub — `bloom` is a useful native demo too; only the wasm export + the empty wasm `main()` stub were added alongside.

## Evidence & Data

- **Verify:** `./scripts/verify.sh` → `VERIFY_EXIT=0`, "all checks passed ✓" (twice — after scaffolding and after the v0.68.0 bump; tests 71/0).
- **Bundle:** `examples/bloom/web/build.sh` emits `bloom.js` + `bloom_bg.wasm` under `pkg/`; `pkg/` gitignored (`git check-ignore` confirmed; only `build.sh` + `index.html` tracked under `examples/bloom/web/`).
- **Headless WebGL2 smoke:** `scripts/bloom_web_smoke.sh` → `BLOOM WEB SMOKE: PASS — BLOOM_WEB_CHECK: PASS (1/1)` (the HDR `Rgba16Float` intermediate + the mip-pyramid float bloom targets render under SwiftShader WebGL2 without panicking).
- **CI #239:** 4/4 green — Test (native) 5m38s, Build (WASM) 40s, Package dry-run 1m8s, Rustdoc 43s. `mergeStateStatus=CLEAN`.
- **Merge:** `gh pr merge 239 --squash --delete-branch`; main → `0e6de2a`.

## Gotchas / discoveries

- **The web build is the only thing that exercises the HDR + bloom float-target path cross-platform.** Native Metal renders `Rgba16Float` unconditionally; only WebGL2 gates it behind `EXT_color_buffer_float`. SwiftShader has the extension, so modern browsers do too — the smoke passing means the pyramid of float mip targets is fine on the web.
- **Examples get the package's `[target.wasm32…].dependencies`** — `wasm-bindgen`/`web-sys`/`console_error_panic_hook` are usable from the example's wasm entry; `web-sys`'s `Document` feature (for `document.set_title`) is already enabled. No `console` feature ⇒ `log::*` no-ops on wasm (fine; the canvas + tab-title are the output).
- **wasm-only struct fields need `#[cfg(target_arch = "wasm32")]` to avoid native `dead_code`.** `BloomDemo` carries `frames`/`reported` only under wasm; `#[derive(Default)]` + `BloomDemo::default()` makes the empty native struct and the wasm struct both construct cleanly (rust-analyzer flags the wasm blocks "inactive on native" — expected, not a warning).
- **Smoke teardown prints `Terminated: 15` for the bg python/chrome** — that's the cleanup trap (`kill`), not a failure; the `PASS` line is the real result.

## Where We're Going

**Nothing is queued.** Both the feature (seq 1) and its web harness (seq 2) are shipped.

**Next session, in order:**
1. Read `../dungeon-merchant/docs/engine-wishlist.md` FIRST (the game session may have filed EW-004+).
2. If still empty → **ASK** the user for direction (standing rule; do not start backlog speculatively).
3. Remaining un-queued offers if asked: hearing the audio-facade / positional-audio web demos by ear; an even-fuller bloom (exposed upsample radius / lens-dirt) **only if a game asks**.

## Related Handoffs (reference only)

- `HANDOFF_bloom-mip-chain_2026-06-24.md` — seq 1, the v0.67.0 mip-chain bloom this ships to the web (the **parent**).
- `HANDOFF_render-format-query-web_2026-06-24.md` — the immediately-prior web follow-up (seq 2 of its chain); the exact `/ship-wasm-example` + smoke structure followed here.

---

## Session Closed
**Closed at:** 2026-06-24 (KST)
**Commit:** code landed as #239 (`0e6de2a`); this handoff lands as its own `docs(handoff)` PR.
**Session status:** Handed off to next session
