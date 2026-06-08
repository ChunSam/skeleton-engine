# Finalize the wasm/v4.1.0 release and harden the wasm path

**Date:** 2026-06-08
**Status:** PLANNED
**Bead(s):** none (`bd` unavailable in this project)
**Epic:** VISION feature+example loop — breadth coverage / dogfooding
**Chain:** `networking-dogfood` seq `2`
**Context:** See `HANDOFF_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md` for full session data, the three wasm bugs + root causes, verification techniques, and the exact commits.

---

## Problem Statement

coin_race now runs and plays in a real browser, and the engine is tagged `v4.0.0`/`v4.1.0` with an accurate CHANGELOG — but two loose ends remain and the wasm path is under-protected. The **`v4.1.0` tag sits at `7c6f9c0`, *before* the font (`0cd9b03`) and canvas (`ebd9081`) fixes**, so the tagged wasm example renders a broken/clipped HUD on Retina. The **rust-survivors pin commit (`e6176fa`) is local/unpushed**. And the three wasm render bugs this session fixed were all latent for months because nothing ever *rendered* wasm and looked at it — there is no regression guard. The existing `examples/wasm` lib demo (`run_demo`) was never re-verified after the fixes. See Evidence & Data in the handoff.

## Key Findings

- The 3 wasm bugs (HiDPI viewport halving, no font, canvas stretch) were independent and all invisible until the example was opened in a browser. → drives Phase 3 (smoke test).
- The `v4.1.0` tag→commit mismatch means `git checkout v4.1.0` gives a wasm example with no/clipped HUD text. Native consumers are unaffected (all 3 fixes are wasm-`cfg`). → drives Phase 1.
- All 3 fixes are wasm-only/additive, so `rust-survivors` (native) is correct at any v4.x pin; its pin commit just needs pushing. → drives Phase 1.
- `run_demo` adapts to `ViewportSize` (so HiDPI bug #1 didn't break it) but bug #2 (no font → no text) and bug #3 (canvas stretch) applied to it too; the canvas fix is now engine-wide, but `run_demo` hasn't been re-rendered/verified and its built `pkg/` is stale. → drives Phase 2.
- The headless-Chrome screenshot technique (`--force-device-scale-factor=2 --enable-unsafe-swiftshader`) is a proven, deterministic wasm render check. → drives Phase 3.
- Carryover breadth: 5 example files still have Korean comments (`doc-language-rule`). → drives Phase 4.

## Anti-Goals (What NOT To Do)

- **Do NOT re-pin rust-survivors to a commit after `7c6f9c0` to "get the wasm fixes"** — they are wasm-only; native rust-survivors needs nothing past v4.1.0. Re-pin only if Phase 1 cuts a `v4.1.1`.
- **Do NOT add the canvas-size or font fix to each example's `index.html`** — the engine handles both now (`finish_init` + `DEFAULT_FONT`). Per-html hacks were tried and reverted (see handoff "What We Tried" #13).
- **Do NOT build the reusable remote-entity helper yet** — deferred two sessions running; needs a 3rd *distinct* networked example for signal. coin_race-wasm is the same example.
- **Do NOT chase wasm Retina crispness now** — the engine renders wasm at the canvas DOM (logical) size by design (WebGL2 2048 cap); slightly soft but correct. Separate, larger change.
- **Do NOT commit/refresh a wasm `pkg/`** — `pkg/` is gitignored repo-wide; build artifacts are never tracked.

## Plan

### Phase 1: Resolve the v4.1.0 tag + push the rust-survivors pin

**Goal:** The `v4.1.0` tag marks a working wasm example, and rust-survivors' origin reflects the v4.1.0 pin.

**Why this approach:** The tagged commit `7c6f9c0` predates the two text fixes, so `v4.1.0`'s wasm example is visibly broken on Retina; the fixes are additive/wasm-only so moving the tag to `ebd9081` is safe and the tag was created today (low blast radius).

- **First, ask the user**: move `v4.1.0` → `ebd9081` (recommended) vs cut `v4.1.1` vs leave. This is a pushed-tag move — confirm before force-pushing.
- If **move**: `git tag -f -a v4.1.0 -m "v4.1.0 — coin_race on wasm (incl. default-font + canvas-size fixes)" ebd9081` then `git push -f origin v4.1.0`. Verify `git rev-list -n1 v4.1.0` = `ebd9081`.
- If **v4.1.1 instead**: bump `Cargo.toml` 4.1.0→4.1.1, add a `## 4.1.1` CHANGELOG section pointing to the font+canvas fixes, commit, `git tag -a v4.1.1 <HEAD>`, push; then re-pin rust-survivors to the v4.1.1 rev (Cargo.toml + Cargo.lock) and amend/extend `e6176fa`.
- **Push rust-survivors `e6176fa`** (after confirming): `git -C /Users/jkl/Projects/rust-survivors push origin main`. Leave their ~20 WIP doc changes uncommitted (not our work).

**Files:** git tags only (move path); +`Cargo.toml`/`docs/CHANGELOG.md`/rust-survivors pin (v4.1.1 path).
**Validates with:** `git rev-list -n1 v4.1.0` resolves to the working commit; `git -C ../rust-survivors log origin/main -1` shows `e6176fa`.
**Rollback:** tags are recreatable — `git tag -f v4.1.0 7c6f9c0 && git push -f origin v4.1.0` to restore.

### Phase 2: Verify + fix the `examples/wasm` lib demo (`run_demo`) on Retina

**Goal:** Confirm the engine's front-door wasm demo renders correctly (text + layout) on Retina now that the 3 fixes are in; fix anything that still breaks.

**Why this approach:** `run_demo` is the canonical "does wasm work" demo and was never re-rendered after the fixes; bug #2/#3 applied to it. The canvas fix is engine-wide so it *should* be correct, but verify rather than assume (every wasm assumption this session was wrong until rendered).

- Build: `examples/wasm/build.sh` (uses `wasm-pack build --target web`); confirm it builds against current `main`.
- Serve `examples/wasm` (`python3 -m http.server`), screenshot with headless Chrome `--headless=new --force-device-scale-factor=2 --enable-unsafe-swiftshader --use-gl=angle --use-angle=swiftshader --screenshot`.
- Confirm: bouncing squares centred/scaled correctly AND the demo's `DrawText` lines ("skeleton-engine — WASM demo", the subsystem list) render in the embedded DejaVu font (this is the first time `run_demo` text would render on wasm — it supplies no `FontData`).
- `run_demo`'s canvas is **1280×720** (its index.html) — confirm the `finish_init` canvas-lock handles a non-800 buffer (it reads `canvas.width()` generically, so it should).
- If text still misses or layout is off, debug with the same `web_sys::console::log_1` + `--enable-logging=stderr` technique (handoff Evidence) and fix at engine level.

**Files:** likely none (verification); possibly `src/app/window.rs` or `examples/wasm/index.html` if a non-800 edge case surfaces.
**Validates with:** headless DPR=2 screenshot of `run_demo` shows squares + readable text; `cargo +1.88.0` gate stays green.
**Rollback:** revert any engine edit; the demo isn't shipped to consumers.

### Phase 3: Add a deterministic wasm render smoke check

**Goal:** A repeatable command that renders a wasm example headless and fails on a blank/degenerate frame, so the 3-bug class is caught going forward.

**Why this approach:** CI builds wasm but never runs it — that blind spot is exactly why 3 bugs stayed latent. A headless-Chrome screenshot (proven this session) is the cheapest deterministic render check; CI lacks Chrome, so this is a documented local script, not a CI gate.

- Write `scripts/wasm_smoke.sh`: build `coin_race_game` to wasm + `wasm-bindgen`, start `coin_race_server` + `http.server`, run Chrome `--headless=new --force-device-scale-factor=2 ... --screenshot=/tmp/smoke.png`, then assert the PNG is non-trivial (e.g. `> N` bytes / not a single solid color — a blank canvas was ~4KB, a rendered one ~75KB).
- Also assert the server logged `[N] connected` (network path) — the strongest signal and Chrome-version-independent.
- Document it in `CLAUDE.md` (Verification section) as an optional local check; note it needs `wasm-bindgen-cli` matching the crate + Chrome.

**Files:** `scripts/wasm_smoke.sh` (new); `CLAUDE.md` (one-line reference).
**Validates with:** `scripts/wasm_smoke.sh` exits 0 on current `main`; manually confirm it FAILS if you revert one of the 3 fixes.
**Rollback:** delete the script + the CLAUDE.md line.

### Phase 4: (Carryover, low priority) example Korean→English comments

**Goal:** Finish the repo-wide example comment conversion (`doc-language-rule`).

**Why this approach:** Consistency for fork-friendliness; 5 files remain (the #27 batch was done in a prior session). Pure mechanical, low-risk.

- Convert Korean comments→English in: `examples/mp_server.rs`, `examples/skeletal_puppet.rs`, `examples/touch_demo.rs`, `examples/games/settings_menu/settings_menu.rs`, `examples/wasm/build.sh` + `examples/wasm/index.html` (`lang="ko"`→`"en"` + comments).
- Keep code/identifiers/paths unchanged; translate comment prose only.

**Files:** the 5 listed.
**Validates with:** `grep -rlP '[\x{AC00}-\x{D7A3}]' examples/` returns empty; `cargo +1.88.0` gate green.
**Rollback:** `git checkout examples/` for the touched files.

## Dependencies & Order

- **Phase 1 first** (release correctness; blocks nothing but should land before more work piles on the tag).
- **Phase 2 before Phase 3** — the smoke test (Phase 3) should target a known-good render, which Phase 2 confirms.
- **Phase 4 is independent** — can run any time, or in parallel via a subagent.
- Phases 2-4 do not touch the same files; safe to parallelize if desired.

## Risks & Mitigations

- **Moving a pushed tag surprises a consumer** (low — only rust-survivors consumes it, by rev not tag). Mitigation: confirm with the user first (Phase 1 step 1); tags are trivially restorable.
- **`run_demo` reveals a 4th wasm issue** (possible — every wasm assumption was wrong this session). Mitigation: Phase 2 budgets for an engine fix; use the console-logging technique that nailed bug #3.
- **Headless Chrome flakiness / SwiftShader differences** (medium). Mitigation: lean on the server `[N] connected` log as the primary signal; treat the screenshot as secondary.
- **Korean→English changes a doc-comment that affects a doctest** (low). Mitigation: the gate's `doc -D warnings` + `test --all-targets` catch it.

## Success Criteria

- **Minimum:** `v4.1.0` (or a new `v4.1.1`) resolves to a commit whose wasm coin_race renders a full HUD on Retina; rust-survivors `e6176fa` is on `origin/main`.
- `run_demo` verified rendering correctly (squares + text) on headless DPR=2.
- `scripts/wasm_smoke.sh` exists, passes on `main`, and fails when a wasm fix is reverted.
- Full `+1.88.0` gate green throughout (baseline: 311 lib + 5 coin_race_server tests).
- (Stretch) zero Korean comments left in `examples/`.

## Quick Start

```bash
cd /Users/jkl/Projects/skeleton-engine
cat plans/handoffs/HANDOFF_networking-dogfood_wasm-coin-race-v4.1_2026-06-08.md   # full data
git log --oneline -6 && git tag | grep v4   # 5031c3d..ebd9081; v4.0.0→5031c3d, v4.1.0→7c6f9c0

# Verify starting state (must be green before touching anything)
./scripts/verify.sh   # or the 5 cmds with cargo +1.88.0; 311 lib + 5 server

# Key files for Phase 1-2
#   (tags only for P1)   src/app/window.rs   src/app/schedule.rs   examples/wasm/{build.sh,index.html}

# First concrete action (Phase 1): confirm tag decision with the user, then —
git rev-list -n1 v4.1.0   # = 7c6f9c0 (the pre-fix commit; the thing to move)
# if user says move:
#   git tag -f -a v4.1.0 -m "v4.1.0 — coin_race on wasm (incl. font + canvas fixes)" ebd9081
#   git push -f origin v4.1.0
# then push the game pin:
#   git -C /Users/jkl/Projects/rust-survivors push origin main   # lands e6176fa
```
