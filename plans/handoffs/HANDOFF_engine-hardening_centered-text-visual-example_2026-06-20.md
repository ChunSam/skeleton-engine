# EW-001 visual confirmation example — `examples/centered_text.rs`

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PR #162 merged + green; `main` @ `c25494a`, package **v0.43.6** (unchanged), tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `48`
**Parent:** `HANDOFF_engine-hardening_ew001-centered-text-drift_2026-06-20.md` (seq 47)

> Picked up the **deferred** item #2 from seq-47's "Where We're Going": a small example that renders
> `DrawText::centered` at an off-center x so the EW-001 fix can be eyeballed (the math regression tests
> were already the acceptance). The wishlist board was **empty** (EW-001 closed last session, next free
> ID EW-002), so — per the rule that the board is the intended driver and backlog needs a user go — I
> asked the user for direction; they chose the visual example. Shipped as an **example-only** PR (no
> version bump).

---

## The Goal

seq-47 shipped the EW-001 fix (v0.43.6) proven by deterministic headless-shaping tests, but left no
example rendering `DrawText::centered` at an off-center x to *see*. This session adds that example so a
human (or a wasm/windowed playtest) can visually confirm the centered text lands on `position.x`.

## Where We Are

- `main` @ **`c25494a`** (PR #162), tree clean, CI 4/4 green. Package **v0.43.6** (UNCHANGED — example-only).
- **PR #162** (`example/centered-text-ew001-visual` → squash-merged, branch deleted), +133 / −1 across 2 files:
  `examples/centered_text.rs` (new, 132 lines) + `CLAUDE.md` (text-renderer module-map row mentions the example).
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): unchanged — still empty, next free ID EW-002.

## What the example shows (`examples/centered_text.rs`)

A single `System` (`CenteredTextDemo`) over a 960×540 window:

- **Three guide columns** at `x = vw·0.2 / 0.5 / 0.8` (two off-center + the center as a control). Each
  is a full-height faint-white **vertical guide line** drawn via `DebugDraw::line`.
- **Single-line centered labels** (`DrawText::centered("centered @ x = {x:.0}", …)`) on each guide → the
  shaped center lands ON the guide for any x. Far-right `x=768` is the money shot: pre-fix it would have
  drifted ~half the viewport further right (off-screen); now it's centered on its guide.
- **Multi-line centered block** (`"two lines, both\ncentered on the guide"`) on the middle guide → BOTH
  lines center on the same x. This is the visible proof the fix **keeps `align = Center`** (dropping it,
  the rejected EW-001 suggestion, would have left-aligned the two lines).
- **Contrast label** — a default `DrawText::new` (top-left anchor) at the left guide → its LEFT edge sits
  on the guide, visibly NOT centered. The difference `centered` exists to remove.
- Bundled `assets/fonts/DejaVuSans.ttf` via `FontData` for deterministic metrics (same font as `ew001_*`).

## Key Decisions

- **Example-only, NO version bump.** No engine `src/`/API/behavior change → v0.43.6 stays. Treated like
  the seq-45/46 docs PRs (which also didn't bump). Did NOT touch the already-merged v0.43.6 CHANGELOG entry.
- **`DebugDraw` guide lines, not a camera rig.** Everything is screen-space, so no `world_to_screen`.
- **Kept it focused on the one claim** (center-on-position.x for any x) + the two corollaries (multi-line
  per-line centering kept; top-left contrast). No interactivity beyond Esc-to-quit.
- **Shipped via branch + PR + squash-merge on green** (user picked this; merge authority standing-delegated).

## Evidence & Data

### KEY coordinate-space fact (verified this session — was NOT in prior handoffs)
`DrawText`, `DebugDraw`, and `ViewportSize` all live in the **same logical-pixel screen space**, so a
guide line and a centered label at the same logical x align at ANY DPR / window scale, with no camera math:

| Surface | Where | Space |
|---|---|---|
| `DrawText` position | `src/renderer/text/renderer.rs:180` — `position = d.position * scale_factor` | input is **logical** px (scaled to physical internally; viewport is physical) |
| `DebugDraw` → `DrawRect` | `src/app/render/debug_draw.rs` (no transform) + UI projection `Mat4::orthographic_rh(0,width,height,0,…)` at `src/renderer/sprite/ui_primitives.rs:105`, fed `logical_w/h` from `src/app/render/frame.rs:443` | **logical** px |
| `ViewportSize` | `src/app/schedule.rs:255` — `gpu.config.width / scale_factor` | **logical** px |

This is why the example reads `ViewportSize` for robust full-height guides + fractional x and needs no
`Camera`. (If you ever add a world-anchored centered label, that's the `world_to_screen` path — see `minimap`.)

### Verify gate (local, before push) + CI (PR #162)
```
cargo clippy --example centered_text -- -D warnings → clean
./scripts/verify.sh → all checks passed ✓   (fmt / clippy / wasm lib+bins / test --all-targets [885 lib tests] / rustdoc)
CI 4/4 green: Build(WASM) 37s · Package dry-run 55s · Rustdoc 31s · Test(native) 3m20s
```

### Windowed playtest (macOS, per the `playtest-windowed-examples` memory)
Built `target/debug/examples/centered_text`, launched under `caffeinate`, positioned the window via
`osascript`, `screencapture`'d it: **all three columns center on their guides** (incl. far-right x=768),
the green multi-line block centers both lines on the middle guide, the amber top-left label starts at the
left guide. `osascript … key code 53` (Esc) → **exited cleanly** (quit path exercised). Screenshot shared
with the user.

## Files Changed
- **PR #162 (engine repo):** `examples/centered_text.rs` (new), `CLAUDE.md` (module-map row).
- **Memory:** `engine-current-state.md` refreshed to seq 48 / `c25494a`.
- **NOT touched:** no `src/`, no `Cargo.toml`/`Cargo.lock`, no CHANGELOG, no version.

## User Feedback & Preferences
- Board is the front door: when it's **empty**, don't auto-start backlog — ask. (This session: asked,
  user picked the visual example over backlog/crates.io/idle.)
- Standing prefs (unchanged): Korean user-facing reports; English code/handoffs/sub-agent prompts;
  source-verify before writing; merge standing-delegated (squash on green CI); honest gap-naming.

## Where We're Going
1. **Watch the board for EW-002+.** `../dungeon-merchant/docs/engine-wishlist.md` is empty; next free ID
   is EW-002. Read it first every engine session.
2. **Optional: ship `centered_text` to the web** (`ship-wasm-example` skill) for a browser-eyeballable
   EW-001 demo + a `wasm_smoke`-style headless render check. Low priority (native playtest already done).
3. **Engine-hardening backlog (unchanged, needs a user go):** crates.io publish (irreversible; publish
   `engine_reflect_derive` too), per-OS gamepad input + the deferred analog-stick Y-sign hardware test.
4. **Optional polish:** seq-43 focus-ring corner-radius/pulse. Low value.

## Risks & Blockers
- **None outstanding** — PR merged green, tree clean.
- HTML/REFERENCE not touched this session (no QA-scan needed).

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # c25494a (#162 centered_text) … 7ea01a8 (#161 seq-47 close)
grep -m1 '^version' Cargo.toml  # 0.43.6
git status -s                   # clean

# FIRST: check the wishlist board for new/updated requests
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md  # empty; next free ID EW-002

# See the EW-001 fix in action:
cargo run --example centered_text   # Esc quits; centered labels sit on their guide lines
cargo test --lib renderer::text::tests::ew001   # the headless acceptance tests (both green)
```

## Cross-cutting Gotchas (expensive-to-rediscover)
1. **`DrawText` / `DebugDraw` / `ViewportSize` are all logical-pixel screen space** (table above). A
   centered label and a `DebugDraw` line at the same logical x align at any DPR — no camera math. Don't
   reach for `world_to_screen` unless the anchor is a *world* position (then it's the `minimap` pattern).
2. **Flat single-file examples auto-register.** `examples/foo.rs` needs NO `[[example]]` in `Cargo.toml`
   (only non-flat/subdir or special-deps examples do). `grep -c foo Cargo.toml` → 0 is expected.
3. **Example-only change → no version bump.** Adding/updating an example is not an engine library change;
   treat like a docs PR (seq-45/46 precedent). A `src/` change is what triggers `/ship` paperwork.
4. **Poll `gh pr checks <n>` before `--watch`.** `--watch` exits "no checks reported" if run <~30s after
   `pr create`. Loop until a check registers, THEN `--watch`. (Held again: registered ~3s here.)
5. **Windowed playtest recipe** (`playtest-windowed-examples` memory): `cargo build --example` first (so
   the window opens fast), launch under `caffeinate -dimsu` backgrounded, `osascript` to front+position,
   `screencapture -x -R<x,y,w,h>`, then `osascript … key code 53` for Esc. The double-background means the
   launcher shell returns exit 0 immediately while the app keeps running — verify with `pgrep -f`.

---

## Session Status
**Goal met** — EW-001 visual-confirmation example shipped (`examples/centered_text.rs`, PR #162 merged
green, no version bump), windowed-playtested. `main` @ `c25494a`, tree clean. Board still empty (next ID
EW-002). Handed off to next session (seq 49).

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #162 (`c25494a`, centered_text example) → handoff #163 (`6c1d51a`) → this close marker.
**Session status:** Goal met — EW-001 visual example shipped + playtested, no version bump (example-only).
Board empty (next free ID EW-002). Handed off to next session (seq 49).
