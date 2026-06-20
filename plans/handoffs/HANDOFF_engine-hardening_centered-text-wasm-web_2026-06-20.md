# Ship `centered_text` to the web (wasm) — EW-001 browser demo + render smoke

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PR #165 merged + green; `main` @ `08ea5de`, package **v0.43.6** (unchanged), tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `49`
**Parent:** `HANDOFF_engine-hardening_centered-text-visual-example_2026-06-20.md` (seq 48)

> Picked up the **deferred** item #2 from seq-48's "Where We're Going": ship the EW-001 visual
> example (`centered_text`) to the browser so the fix is eyeball-checkable on the web, plus a
> `wasm_smoke`-style headless render check. The wishlist board was **empty** (next free ID EW-002),
> the user chose this option, then said proceed. Shipped as an **example-only** PR (no version bump).

---

## The Goal

seq-48 shipped `examples/centered_text.rs` — a native window that renders `DrawText::centered` on
off-center guide lines to prove EW-001 visually. This session puts that same example **in a browser
(wasm)** via the `ship-wasm-example` template, and adds a headless render smoke so CI-independent
regression protection exists (CI builds wasm but never *runs* a frame).

## Where We Are

- `main` @ **`08ea5de`** (PR #165), tree clean, CI 4/4 green. Package **v0.43.6** (UNCHANGED — example-only).
- **PR #165** (`example/centered-text-wasm` → squash-merged, branch deleted), +313 / −20 across 7 files.
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): unchanged — still empty, next free ID EW-002.

## What shipped

1. **Subdir move + registration.** `examples/centered_text.rs` → `examples/centered_text/centered_text.rs`
   (matching the `web_audio`/`wasm_save` layout so the web harness co-locates) + a `[[example]]` entry in
   `Cargo.toml` (subdir examples are NOT auto-discovered — only flat `examples/*.rs` are). Native
   `cargo run --example centered_text` is unchanged (cargo uses the registered path).
2. **Wasm entry point** (coin_race **dual-target** pattern). Refactored the example's `main()` body into a
   shared `fn run()`; native `#[cfg(not(wasm32))] fn main() { run(); }` stays real; added
   `#[cfg(wasm32)] #[wasm_bindgen] pub fn run_centered_text() { run(); }` + an empty wasm `main()`.
3. **Web harness** `examples/centered_text/web/{build.sh,index.html}`. `build.sh` = `cargo build --release
   --example centered_text --target wasm32-unknown-unknown` then `wasm-bindgen --target web --out-dir pkg`.
   `index.html` = canvas `id="game-canvas"` 960×540 + a Start button → `run_centered_text()`, plus a
   `?autostart=1` path so the headless smoke renders without the (human-only) click. `pkg/` is gitignored.
4. **Render smoke** `scripts/centered_text_smoke.sh` — a network-free variant of `wasm_smoke.sh` (no
   coin_race server/socket): build → serve → headless Chrome DPR=2 `?autostart=1` → assert a non-blank
   PNG → save the screenshot to eyeball. PASS (non-blank 147 KB).
5. **Docs.** Moved CLAUDE.md's 4 "Optional wasm X check" bullets into new `docs/WASM_SMOKES.md` + a one-line
   reference (the documented growth strategy), bringing CLAUDE.md **215 → 202 lines**; added the
   centered-text smoke as the 4th. Text-renderer module-map row notes the web demo + smoke.

## Key Decisions

- **Example-only, NO version bump.** No engine `src/`/API/behavior change → v0.43.6 stays (seq-45/46 docs-PR
  precedent). The wasm entry point is in the *example*, not the lib.
- **Subdir layout, not flat + sibling dir.** Matched the two existing wasm examples (`web_audio`,
  `wasm_save`) for consistency and clean co-location; cost is one `[[example]]` line.
- **`?autostart=1`, not auto-run-always.** Humans get a Start button (clean UX, the template's intent); the
  headless smoke passes the param. No audio/gesture gate here, so this is purely about who triggers render.
- **Network-free smoke.** `centered_text` has no server, so the smoke is `wasm_smoke.sh` minus the
  server/socket stand-up — simpler, but it can only assert *non-blank* (the centering itself is the
  subtle class `wasm_smoke.sh` documents it can't auto-check → eyeball the screenshot).
- **Shipped via branch + PR + squash-merge on green** (merge authority standing-delegated).

## Evidence & Data

### Browser render (the acceptance) — `/tmp/centered_text_smoke.png`, headless DPR=2
All three cyan `centered @ x = 192/480/768` labels center **on** their white guide lines (incl. far-right
x=768, which pre-fix drifted ~half the viewport off-screen); the green `two lines, both / centered on the
guide` block centers BOTH lines on the middle guide (proof `align = Center` is kept); the amber
`DrawText::new` label's LEFT edge sits on the left guide (the contrast). EW-001 confirmed on the web.

### Two smoke bugs caught + fixed this session
1. **Stale-server false-pass.** A first smoke run rendered the **wrong page** (the `wasm_save` AEAD
   self-check) yet **PASSED** the non-blank check — an orphaned `http.server` from a prior session held the
   port, so my own `python3` failed to bind (silently) and the stale server answered Chrome. Fix: the smoke
   now **refuses a port already in use** (`lsof … && exit 2`) and verifies its own server process came up.
2. **The orphan source.** `( cd "$WEB_DIR" && python3 -m http.server )` runs python as a *grandchild* of the
   `&` subshell, so `kill "$HTTPD_PID"` kills the subshell and **orphans python onto the port** (→ bug 1 on
   the next run). Fix: `python3 -m http.server "$PORT" --directory "$WEB_DIR"` (no subshell) so `$!` IS
   python and `kill` reaps it. Re-run leaves **0** surviving `http.server` processes.
   - NOTE: the existing `scripts/wasm_smoke.sh` uses the same `( cd && python3 )` pattern and likely shares
     the orphan leak, but self-masks (same port + same coin_race page across runs → a stale server still
     serves the right page). Left untouched — out of scope for this example-only PR; worth a follow-up.

### Verify gate (local, before push) + CI (PR #165)
```
cargo build --example centered_text --target wasm32-unknown-unknown → clean (no native-only deps)
./scripts/verify.sh → exit 0   (fmt / clippy / wasm lib+bins / test --all-targets [885 lib tests] / rustdoc)
examples/centered_text/web/build.sh → emits pkg/centered_text.js + _bg.wasm
scripts/centered_text_smoke.sh → PASS (non-blank 147 KB)
CI 4/4 green: Build(WASM) 37s · Package dry-run 52s · Rustdoc 35s · Test(native) 3m38s
```

## Files Changed
- **PR #165 (engine repo):** `examples/centered_text.rs` → `examples/centered_text/centered_text.rs`
  (moved + wasm entry), `Cargo.toml` (`[[example]]`), `examples/centered_text/web/{build.sh,index.html}`
  (new), `scripts/centered_text_smoke.sh` (new), `CLAUDE.md` (smoke bullets → reference; module-map row),
  `docs/WASM_SMOKES.md` (new).
- **Memory:** `engine-current-state.md` refreshed to seq 49 / `08ea5de`.
- **NOT touched:** no engine `src/`, no `Cargo.lock` (Cargo.toml example entry doesn't move the lock),
  no CHANGELOG, no version, no REFERENCE.html. `pkg/` stays gitignored (never committed).

## User Feedback & Preferences
- Board is the front door: when it's empty, ask before backlog. (This session: asked, user picked the web
  demo, then said proceed.)
- Standing prefs (unchanged): Korean user-facing reports; English code/handoffs/sub-agent prompts;
  source-verify before writing; merge standing-delegated (squash on green CI); honest gap-naming.

## Where We're Going
1. **Watch the board for EW-002+.** `../dungeon-merchant/docs/engine-wishlist.md` is empty; next free ID
   is EW-002. Read it first every engine session.
2. **Optional: fix the `wasm_smoke.sh` orphan leak** (the `( cd && python3 )` → `--directory` fix from this
   session) + add the busy-port guard. Low priority (it self-masks today). Would make all the smokes
   orphan-safe and consistent.
3. **Engine-hardening backlog (unchanged, needs a user go):** crates.io publish (irreversible; publish
   `engine_reflect_derive` too), per-OS gamepad input + the deferred analog-stick Y-sign hardware test.
4. **Optional polish:** seq-43 focus-ring corner-radius/pulse. Low value.

## Risks & Blockers
- **None outstanding** — PR merged green, tree clean.
- Smoke is non-CI (needs Chrome/GPU) — optional local check, by design.

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 08ea5de (#165 centered_text wasm) … 1ba78be (#164 seq-48 close)
grep -m1 '^version' Cargo.toml  # 0.43.6
git status -s                   # clean

# FIRST: check the wishlist board for new/updated requests
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md  # empty; next free ID EW-002

# See the EW-001 fix in a browser:
examples/centered_text/web/build.sh
python3 -m http.server 8080 --directory examples/centered_text/web   # open localhost:8080, click Start
scripts/centered_text_smoke.sh   # headless render smoke (PASS + screenshot to eyeball)
```

## Cross-cutting Gotchas (expensive-to-rediscover)
1. **Stale `http.server` → false-pass.** A render smoke that only checks "non-blank PNG" will PASS off the
   WRONG page if an orphaned server holds the port. Always guard the port (refuse if in use) AND verify your
   own server came up. (Bit this session: rendered the `wasm_save` page, passed anyway.)
2. **`( cd && python3 ) &` orphans python.** `$!` is the subshell, not python; `kill` leaves python on the
   port. Use `python3 -m http.server --directory <dir>` so `$!` IS python. Same trap as backgrounded Chrome
   under SwiftShader (reap it yourself).
3. **Subdir examples need a `[[example]]` entry.** Only flat `examples/*.rs` auto-discover. Moving a flat
   example into `examples/<name>/<name>.rs` (to co-locate a `web/` harness) requires registering it, or
   `cargo build --example <name>` fails "no example target named".
4. **Dual-target example = keep native `main()` real.** `centered_text` runs natively AND on wasm, so follow
   the coin_race pattern (`run()` + `#[cfg]` main + `#[wasm_bindgen]` export), NOT the `web_audio`/`wasm_save`
   wasm-only print-stub.
5. **Engine binds to `<canvas id="game-canvas">`** (`src/app/window.rs`) and reads the authored
   width/height attributes as the logical size — set the canvas to 960×540 to match the WindowConfig.
6. **Example-only change → no version bump** (seq-45/46 precedent). A `src/` change is what triggers `/ship`.

---

## Session Status
**Goal met** — `centered_text` ships to the web (PR #165 merged green, no version bump), EW-001
browser-render eyeball-confirmed, render smoke added (+ 2 smoke bugs fixed). `main` @ `08ea5de`, tree clean.
Board still empty (next ID EW-002). Handed off to next session (seq 50).

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #165 (`08ea5de`, centered_text wasm web harness + render smoke) → this handoff.
**Session status:** Goal met — EW-001 visual example shipped to the browser + headless render smoke, no
version bump (example-only). Board empty (next free ID EW-002). Handed off to next session (seq 50).
