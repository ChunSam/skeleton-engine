# Optional "breathing" pulse for the keyboard-focus ring

**Date:** 2026-06-20 (KST)
**Status:** COMPLETED — PR #169 merged + green; `main` @ `53f471d`, package **v0.44.0** (MINOR bump), tree clean.
**Bead(s):** none (repo uses `plans/handoffs/`)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `51`
**Parent:** `HANDOFF_engine-hardening_smoke-orphan-port-guard_2026-06-20.md` (seq 50)

> Wishlist board was **empty** (next free ID EW-002). The user chose the seq-43 **focus-ring polish**
> backlog item over waiting, then — when shown the cost split — picked **pulse only**. Shipped as a
> normal additive feature (MINOR bump 0.43.6 → 0.44.0), with the `ui_focus` example as the VISION
> acceptance test.

---

## The Goal

Add a gentle "breathing" alpha pulse to the keyboard-focus ring so it draws the eye to the focused
widget — the long-deferred seq-43 polish item. **Corner-radius (the other half of that backlog item)
was deliberately dropped:** the ring is four axis-aligned `DrawRect` border bars and `DrawRect` is a
plain quad with no radius, so real rounded corners need a rounded-rect SDF in the UI render pipeline —
a renderer feature, not polish. The user confirmed pulse-only after seeing this cost asymmetry.

## Where We Are

- `main` @ **`53f471d`** (PR #169), tree clean, CI 4/4 green. Package **v0.44.0** (additive feature = MINOR, pre-1.0).
- **PR #169** (`feat/focus-ring-pulse` → squash-merged, branch deleted), +163 / −13 across 8 files.
- **Wishlist board** (`../dungeon-merchant/docs/engine-wishlist.md`): unchanged — still empty, next free ID EW-002.

## What shipped

1. **`FocusRingStyle` (`src/ui/focus.rs`)** — two additive fields + a helper:
   - `pulse_hz: f32` — pulse frequency, cycles/sec. **`0.0` (default) = no pulse**, byte-identical to before.
   - `pulse_min_alpha: f32` — ring alpha at the trough, as a fraction of `color`'s alpha (`1.0` = no dimming). Clamped `[0,1]`.
   - `pulse_alpha(t) -> f32` — pure helper, a raised sine in `[pulse_min_alpha, 1.0]`; returns a flat
     `1.0` when no pulse is configured (`pulse_hz <= 0` **or** `pulse_min_alpha >= 1`), so a default
     style is never modulated.
2. **`UiSystem` (`src/ui/system.rs`)** — new `ring_elapsed: f32` field accumulates `dt`, wrapped via
   `rem_euclid(RING_PULSE_WRAP=3600.0)` so `+= dt` never reaches the f32 magnitude where it loses
   sub-frame precision (and integer-Hz pulses stay phase-continuous across the wrap). Threaded into the focus pass.
3. **`push_ring` (`src/ui/system/focus_pass.rs`)** — takes `elapsed`, multiplies the ring color's alpha
   by `style.pulse_alpha(elapsed)` before emitting the four border rects. Non-pulsing style untouched.
4. **`examples/ui_focus.rs`** — the demo's cyan ring now pulses (`pulse_hz: 1.2`, `pulse_min_alpha: 0.35`);
   the VISION acceptance example. Module doc + the `FocusRingStyle` doctest updated.
5. **Release paperwork** (`/ship`): `Cargo.toml`/`Cargo.lock` → 0.44.0, `docs/CHANGELOG.md` 0.44.0 entry,
   `CLAUDE.md` header (v1.6.101) + the UI module-map row.

## Key Decisions

- **Pulse only; corner-radius dropped** (renderer feature, not polish — see The Goal). User-confirmed via AskUserQuestion.
- **Off by default** — `pulse_hz = 0.0` makes `pulse_alpha` a flat `1.0`, so the historical amber 3px ring
  and every existing call site are byte-identical. The new fields slot in behind `..Default::default()`.
- **Accumulate the clock in `UiSystem`, not a global Time resource** — there is no engine-wide elapsed-time
  resource, and `UiSystem` already persists per-frame state (scratch buffers, `StickNav`); the cursor-blink
  in `text_input_pass` is the precedent for dt-driven UI animation. Wrapped to avoid f32 precision stall.
- **MINOR bump 0.44.0** — additive public API (`pulse_hz`/`pulse_min_alpha`/`pulse_alpha`), pre-1.0 = MINOR.
- **Shipped via branch + PR + squash-merge on green** (merge authority standing-delegated).

## Evidence & Data

### Verify gate (local, before push) + CI (PR #169)
```
./scripts/verify.sh → exit 0   (fmt / clippy / wasm lib+bins / test --all-targets / rustdoc), at v0.44.0 + lock refreshed
lib tests 885 → 889 (the 4 new tests below)
CI 4/4 green: Build(WASM) 36s · Package dry-run 1m1s · Rustdoc 35s · Test(native) 3m45s
```

### New tests (all green)
- `ui::focus::tests::pulse_alpha_unity_when_disabled` — default (and `pulse_min_alpha >= 1`) → flat `1.0`.
- `ui::focus::tests::pulse_alpha_oscillates_in_range` — midpoint at t=0, peak at ¼ period, trough at ¾, always in `[min,1]`.
- `ui::focus::tests::pulse_alpha_clamps_negative_min` — negative `pulse_min_alpha` clamps to `0.0`, alpha never negative.
- `ui::system::focus_pass::tests::push_ring_applies_pulse_alpha` — ring rects carry min alpha at the trough, full alpha at the peak.

## Files Changed
- **PR #169 (engine repo):** `src/ui/focus.rs` (fields + `pulse_alpha` + tests), `src/ui/system.rs`
  (`ring_elapsed` + wrap const), `src/ui/system/focus_pass.rs` (`run`/`push_ring` take `elapsed`, apply
  pulse, + test), `examples/ui_focus.rs` (pulse + doc), `Cargo.toml`/`Cargo.lock` (0.44.0),
  `docs/CHANGELOG.md` (0.44.0), `CLAUDE.md` (header v1.6.101 + UI module-map row).
- **Memory:** `engine-current-state.md` + `MEMORY.md` index refreshed to seq 51 / `53f471d` / v0.44.0.

## User Feedback & Preferences
- Board is the front door: when it's empty, ask before backlog. (This session: asked, user picked option 3
  (focus-ring polish), then chose pulse-only when shown the corner-radius cost.)
- Standing prefs (unchanged): Korean user-facing reports; English code/handoffs/sub-agent prompts;
  source-verify before writing; merge standing-delegated (squash on green CI); honest gap-naming.

## Where We're Going
1. **Watch the board for EW-002+.** `../dungeon-merchant/docs/engine-wishlist.md` is empty; next free ID
   is EW-002. Read it first every engine session.
2. **Deferred from this item: focus-ring corner-radius.** Would need rounded-rect rendering in the UI
   pipeline — a `radius` on `DrawRect` + an SDF rounded-box branch in the UI fragment shader (affects ALL
   UI rects → regression risk). A real renderer feature if ever wanted; not polish.
3. **Engine-hardening backlog (unchanged, needs a user go):** crates.io publish (irreversible; publish
   `engine_reflect_derive` too), per-OS gamepad input + the deferred analog-stick Y-sign hardware test.

## Risks & Blockers
- **None outstanding** — PR merged green, tree clean.

## Quick Start for Next Session
```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 53f471d (#169 focus-ring pulse) … 49ba91f (#168 seq-50 handoff)
grep -m1 '^version' Cargo.toml  # 0.44.0
git status -s                   # clean

# FIRST: check the wishlist board for new/updated requests
sed -n '53,70p' ../dungeon-merchant/docs/engine-wishlist.md  # empty; next free ID EW-002

# See the pulsing ring: cargo run --example ui_focus  (Tab to move focus; the cyan ring breathes)
```

## Cross-cutting Gotchas (expensive-to-rediscover)
1. **No engine-wide elapsed-time resource.** For dt-driven UI animation, accumulate in the system struct
   (`UiSystem` already persists state) — the cursor-blink in `text_input_pass` is the precedent. **Wrap the
   accumulator** (`rem_euclid`) or `+= dt` stalls once the value is large enough that dt < 1 ulp (~37h at 60fps).
2. **The focus ring is four axis-aligned `DrawRect` bars; `DrawRect` has no corner radius.** Anything
   "rounded" on a UI rect is a renderer change (SDF rounded-box in the UI frag shader), not a styling tweak.
3. **`FocusRingStyle` literals in tests must use `..Default::default()`** — adding fields broke two test
   initializers that listed all fields explicitly (E0063). Internal `push_ring` is called directly by two
   tests, so its signature change rippled there too. (rust-analyzer's inline diagnostics lagged a few edits
   behind during this — trust `cargo`, per the standing IDE-staleness gotcha.)
4. **Additive `src/` feature → MINOR bump + `/ship`** (here 0.44.0). Distinct from seq-49/50's example/script-only
   PRs that did NOT bump.

---

## Session Status
**Goal met** — the keyboard-focus ring can now pulse (`FocusRingStyle::pulse_hz`/`pulse_min_alpha` +
`pulse_alpha`), off by default and exercised by `ui_focus` (PR #169 merged green, v0.44.0). Corner-radius
deliberately deferred as a renderer feature. `main` @ `53f471d`, tree clean. Board still empty (next ID
EW-002). Handed off to next session (seq 52).

## Session Closed
**Closed at:** 2026-06-20 (KST)
**Commits:** #169 (`53f471d`, focus-ring pulse, v0.44.0) → this handoff.
**Session status:** Goal met — focus-ring breathing pulse shipped (additive, default-off, MINOR bump),
`ui_focus` exercises it; corner-radius deferred (renderer feature). Board empty (next free ID EW-002).
Handed off to next session (seq 52).
