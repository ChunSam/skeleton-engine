# Configurable UI focus-ring styling via `FocusRingStyle` (v0.42.0, shipped + merged)

**Date:** 2026-06-19
**Status:** COMPLETED — PR #144 merged + green; `main` clean.
**Bead(s):** none (repo uses `plans/handoffs/`, not beads)
**Epic:** engine-hardening (post-roadmap feature/cleanup arc)
**Chain:** `engine-hardening` seq `43`
**Parent:** `HANDOFF_engine-hardening_gamepad-stick-ui-nav_2026-06-19.md` (seq 42)
**Prior chain:** seq 38 `session-wrap-2` > 39 `visual-audio-verify` > 40 `review-fixes` > 41 `deferred-cleanups` > 42 `gamepad-stick-ui-nav` > **43 this (focus-ring-style)**

> This session implemented one of the two **"Remaining seq-38/39 follow-ups"** seq-42 left open
> (item 3 in its "Where We're Going"): **focus-ring styling** — the `RING_COLOR`/`RING_THICKNESS`
> constants in `focus_pass.rs` were hardcoded. They now live in a `FocusRingStyle` World resource
> (color / thickness / enabled), auto-inserted with the default amber 3px ring so behavior is
> byte-identical out of the box. Shipped as v0.42.0 (PR #144), merged on green CI.

---

## Since Last Handoff (vs seq-42's "Where We're Going")

Seq-42 left four buckets: (1) per-OS input optimization + the deferred gamepad hardware test,
(2) crates.io publish, (3) remaining seq-38/39 follow-ups [**64-tile hex atlas**, **focus-ring
styling**].
- **User picked bucket (3) → focus-ring styling** (chosen over the 64-tile hex atlas after I scoped
  both: the hex atlas is a precedent-free procedural-art task — see Key Decisions; focus-ring styling
  is small, clean, fully verifiable, fits the VISION loop).
- Shipped it as **v0.42.0** (PR #144, `49d5019`), merged on 4/4 green CI.
- **crates.io still untouched** (unchanged since seq 33). The **64-tile hex atlas** (the other half of
  bucket 3) and **per-OS input + deferred gamepad hardware test** (bucket 1) remain open.
- The recurring **R1 gotcha** ("a verify wrapper that prints exit 0 while the file holds 1") bit once
  again and was caught again (fmt diff) — the rule continues to earn its keep.

## Reference Documents

- `CLAUDE.md` — conventions (now **v1.6.93**, package **v0.42.0**). R1 verify-exit rule (seq 40);
  module-map row 122 (UI focus) updated this session to mention `FocusRingStyle`.
- Parent `HANDOFF_engine-hardening_gamepad-stick-ui-nav_2026-06-19.md` (seq 42).
- `docs/VISION.md` — "a feature is not done until an example exercises it"; here the example is
  `ui_focus` (restyled to a cyan ring).
- `docs/CHANGELOG.md` — the 0.42.0 entry.

## The Goal

skeleton-engine is a fork-friendly, MIT, genre-agnostic 2D engine; the engine-hardening arc drains a
post-roadmap backlog. This segment's goal: **make the keyboard/gamepad focus ring restyleable by a
fork** (it was a hardcoded amber 3px border), shipping it as a clean additive MINOR with the example
as the acceptance test — without changing the default appearance for anyone who doesn't opt in.

## Where We Are

- `main` @ **`49d5019`** (PR #144, v0.42.0), package **v0.42.0**, CLAUDE.md header **v1.6.93**, tree
  clean, CI green (4/4: Build WASM 40s · Package dry-run 1m4s · Rustdoc 36s · Test native 5m42s).
- **Feature:** the focus pass's previously hardcoded `RING_COLOR` (amber `rgba(1.0,0.85,0.3,1.0)`) and
  `RING_THICKNESS` (3.0) move into a new **`FocusRingStyle`** World resource:
  `{ color: Color, thickness: f32, enabled: bool }` + `is_visible()` (`enabled && thickness > 0.0`).
- **`Default` is byte-identical to the old ring** (same amber, same 3px). Auto-inserted in
  `insert_core_resources` right after `UiFocus`, so games mutate it via `resource_mut`, and the focus
  pass reads it with `world.resource::<FocusRingStyle>().copied().unwrap_or_default()` (robust even
  for hand-built test `World`s that omit it).
- **Suppression:** `enabled = false` OR `thickness <= 0.0` ⇒ `push_ring` early-returns, drawing
  nothing — for a game that renders its own focus indicator.
- **No public API removed** — purely additive. `FocusRingStyle` is re-exported from `src/ui/mod.rs`
  and the crate root (`src/lib.rs`).
- **Example `ui_focus`** restyles the ring to a thicker **cyan** one (`Color::rgb(0.3,0.9,1.0)`,
  thickness 4.0) to demonstrate, and its doc/comment advertise the resource. VISION acceptance test.
- **Tests:** 5 new (3 `push_ring` unit + 2 `UiSystem` integration). **Lib tests 878 → 883.** Verify
  gate green at every stage (baseline → feature → ship). CI 4/4 green.

## What We Tried (Chronological)

1. **Onboarding.** Read the seq-42 parent handoff in full + located the latest handoff
   (`plans/handoffs/`, newest-first). Confirmed `main` @ `2a73efa` (the seq-42 *handoff* commit, #143),
   package v0.41.0, tree clean.
2. **Presented the seq-42 backlog to the user (Korean), asked which bucket** via AskUserQuestion →
   user chose **"C: hex 아틀라스 / 포커스 링"** (the seq-38/39 follow-ups).
3. **Scoped C's two halves** before committing: grepped the hex examples + the square-blob precedent.
   Found **no 47/64-tile autotile atlas exists anywhere** (all hex/iso examples use 2-tile
   interior/edge demos with `TilemapAtlas::new(..., 2, 1)`); a real hex blob atlas is a precedent-free
   procedural-art task. Focus-ring styling, by contrast, is a small clean config extraction. Asked the
   user which to start with → **"포커스 링 스타일링 (추천)"**.
4. **Ran the R1-safe baseline gate** (`/tmp/verify_ring_base.txt`, fresh path): exit **0** (878 lib
   tests per parent; the tail showed a 66/32 binary + the green "all checks passed ✓").
5. **Implemented `FocusRingStyle`** in `src/ui/focus.rs` (resource + `Default` + `is_visible` +
   doctest), removed the two `const`s from `focus_pass.rs`, threaded a `&FocusRingStyle` into
   `push_ring` (early-return when not visible), and read the resource at the call site. Re-exported in
   `ui/mod.rs` + `lib.rs`; auto-inserted in `core_resources.rs`.
6. **Wrote 5 tests** in `focus_pass.rs`'s test module (added `Color`/`UiQueue`/`FocusRingStyle`
   imports + `use super::{push_ring, UiOutput};`).
7. **Verified the disabled-ring test is robust:** checked `Button::new`'s `color_normal` =
   `rgba(0.20,0.20,0.25,1.0)` — distinct from the amber ring, so filtering `UiQueue` rects by the ring
   color cleanly isolates the ring from the button's own background rect.
8. **Updated the `ui_focus` example** (cyan ring + doc) and **CLAUDE.md module-map row 122**.
9. **First full gate FAILED (exit file = 1).** The background notification said "exit code 0" but
   `/tmp/verify_ring_feat.txt` held **1** — `cargo fmt --check` diff in `examples/ui_focus.rs:20`
   (the `use engine::{...}` block needed rustfmt's wrapping). Ran `cargo fmt`, re-ran: green, **883**.
10. **`/ship` v0.42.0** — the four-edit set (Cargo.toml + lock via `cargo update -p skeleton-engine`
    + CHANGELOG + CLAUDE.md header) + module-map row. Re-ran the gate post-bump: exit **0**, 883.
11. **User said "진행해"** → created branch `feat/focus-ring-style`, committed, pushed, opened PR #144,
    watched CI (4/4 green), squash-merged with `--delete-branch`, fast-forwarded `main` → `49d5019`.

## Key Decisions

- **Resource read via `unwrap_or_default()`, not mandatory insertion.** The focus pass falls back to
  `FocusRingStyle::default()` when the resource is absent, so behavior is identical for any `World`
  that never inserts it (including the unit-test worlds) AND the default reproduces the old hardcoded
  ring exactly. Auto-insertion in `core_resources` is a convenience so games can `resource_mut` it.
- **`enabled` AND `thickness <= 0.0` both suppress** the ring, unified behind `is_visible()`. Two
  natural "off" switches (a boolean toggle and a zero size) instead of one.
- **`Copy` resource** (`Color` is `Copy`; three small fields) → the call site `.copied()` is cheap and
  avoids borrow gymnastics against `world`.
- **Chose focus-ring styling over the 64-tile hex atlas.** The hex atlas needs 64 distinct hex tiles
  drawn per 6-neighbor mask with no existing precedent (even square `blob_47` has no real atlas in the
  repo) and is hard to verify visually. Focus-ring styling is small, complete, and fully unit-testable
  — the right single-PR deliverable. The hex atlas stays open as the meatier follow-up.
- **MINOR bump v0.42.0** (additive, 0.x cadence) — `/ship`'s four edits + module-map row 122.
- **Scope discipline:** did NOT add ring features beyond color/thickness/enabled (no corner rounding,
  no pulse animation, no per-widget override). The ring stays four border rects; extensions are listed
  as optional future work.

## Evidence & Data

### Verify gates (R1-safe, exit read from file not wrapper)
| stage | file | exit | lib tests |
|---|---|---|---|
| onboarding baseline | `/tmp/verify_ring_base.txt` | 0 | 878 |
| feature, 1st run (FAILED) | `/tmp/verify_ring_feat.txt` | **1** | — (fmt diff in `ui_focus.rs:20`, tests never ran) |
| feature, after `cargo fmt` | `/tmp/verify_ring_feat2.txt` | 0 | 883 |
| after `/ship` v0.42.0 (post-bump) | `/tmp/verify_ship42.txt` | 0 | 883 |

### CI (PR #144)
| check | result |
|---|---|
| Build (WASM) | pass 40s |
| Package dry-run | pass 1m4s |
| Rustdoc | pass 36s |
| Test (native) | pass 5m42s |

### Ship / merge
| item | value |
|---|---|
| commit | `7467483` → squashed to `49d5019` (#144) |
| PR | #144, 10 files, +238/−24 |
| merge | `gh pr merge 144 --squash --delete-branch`, main fast-forwarded |

### The 5 new tests (the styling contract)
| test | asserts |
|---|---|
| `push_ring_uses_custom_color_and_thickness` | 4 rects, all in the style color; top/bottom `h==thickness` & full width, left/right `w==thickness` & full height |
| `push_ring_disabled_or_zero_thickness_draws_nothing` | `enabled=false` → 0 rects; `thickness=0.0` → 0 rects |
| `default_focus_ring_matches_historical_appearance` | default = amber `rgba(1.0,0.85,0.3,1.0)`, 3px, `is_visible()` |
| `ui_system_consumes_focus_ring_style_resource` | end-to-end: custom cyan style → exactly 4 cyan rects reach `UiQueue` |
| `ui_system_disabled_ring_style_draws_no_ring` | end-to-end: `enabled=false` → no amber rects in `UiQueue` |

## Code Analysis

- **`FocusRingStyle`** (`src/ui/focus.rs`): `#[derive(Debug, Clone, Copy, PartialEq)]`; `Default` =
  `{ color: Color::rgba(1.0,0.85,0.3,1.0), thickness: 3.0, enabled: true }`; `is_visible(&self) ->
  bool = self.enabled && self.thickness > 0.0`. Doctest constructs a cyan/5px variant and asserts
  `is_visible()`.
- **`push_ring`** (`src/ui/system/focus_pass.rs`): now `fn push_ring(output, pos, size, z, style:
  &FocusRingStyle)`. Early-returns `if !style.is_visible()`. Otherwise pushes 4 `DrawRect`s at `z+0.5`
  using `style.thickness` (border width) and `style.color`. The two `const RING_*` are gone;
  `SLIDER_STEP_FRAC = 0.05` remains.
- **Call site** (in `focus_pass::run`, the `if visible { … }` block): reads
  `world.resource::<FocusRingStyle>().copied().unwrap_or_default()` then `push_ring(..., &style)`.
- **`submit_output`** (`src/ui/system/state.rs`) drains `UiOutput.rects` → `UiQueue` (whose
  `items: Vec<DrawRect>` is `pub`, hence the integration tests can read them). `DrawRect` has public
  `x/y/w/h/color/z`.
- **`Button::new` colors** (`src/ui/button.rs`): `color_normal = rgba(0.20,0.20,0.25,1.0)` — does NOT
  collide with the amber ring, which is what makes the disabled-ring integration test sound.
- **Auto-insert** (`src/app/core_resources.rs`): `world.insert_resource(crate::ui::FocusRingStyle::
  default());` immediately after the `UiFocus` insert.

## Files Changed (PR #144)

### Source
- `src/ui/focus.rs` — new `FocusRingStyle` resource (+ `Default`, `is_visible`, doctest); module doc
  mentions it.
- `src/ui/system/focus_pass.rs` — removed `RING_COLOR`/`RING_THICKNESS`; `push_ring` takes
  `&FocusRingStyle` + early-returns when not visible; call site reads the resource; +5 tests + their
  imports (`Color`, `UiQueue`, `FocusRingStyle`, `super::{push_ring, UiOutput}`).
- `src/app/core_resources.rs` — auto-insert `FocusRingStyle::default()` next to `UiFocus`.
- `src/ui/mod.rs` — `pub use focus::{FocusRingStyle, UiFocus};`.
- `src/lib.rs` — add `FocusRingStyle` to the `pub use ui::{…}` re-export.
### Example & docs
- `examples/ui_focus.rs` — inserts a cyan `FocusRingStyle`; doc + import updated.
- `CLAUDE.md` — module-map row 122 (UI focus) mentions `FocusRingStyle`; header v1.6.93 / package
  v0.42.0.
- `docs/CHANGELOG.md` — 0.42.0 entry.
- `Cargo.toml` / `Cargo.lock` — v0.42.0.

## User Feedback & Preferences

- **Chose "C" (hex 아틀라스 / 포커스 링)** from the seq-42 backlog, then **"포커스 링 스타일링
  (추천)"** as the first half — agreeing with the scoping recommendation.
- **"진행해"** — authorized the full commit → push → PR → squash-merge flow (in response to my
  explicit offer of exactly those steps).
- **"/handoff 하고 푸시해"** — close with a handoff and push it.
- Standing: Korean for user-facing reports/questions; agent-to-agent + file docs in English; merge is
  standing-delegated as a DIRECT instruction (never an AskUserQuestion option — Korean classifier
  misread); honest gap-naming valued; use subagents for parallel work with an explicit `model`.

## Where We're Going

1. **64-tile hex autotile atlas** — the *other* half of seq-42's bucket (3), still open. Needs 64
   distinct hex tiles authored per 6-neighbor mask (`hex_6`/`hex_6_flat`, mask 0..64) + a full-blob
   example, analogous to square `blob_47`. Precedent-free art/asset work (no 47/64-tile atlas exists
   in the repo today); likely procedural generation of the tiles. Biggest of the remaining items.
2. **Optional focus-ring extensions** (NOT done, deliberately): corner rounding, a pulse/blink
   animation, or per-widget style overrides. The ring is currently four straight border rects. Only
   pursue if a real use-case appears.
3. **Per-OS (Windows/Mac) input optimization** + the **deferred gamepad hardware test** (of the
   seq-42 analog-stick Y sign). gilrs can't read a GameController-framework-claimed Xbox pad on this
   macOS box — see `gilrs-macos-xbox-no-input` memory.
4. **crates.io publish** — the persistent untouched backlog item (irreversible, needs explicit go;
   publish `engine_reflect_derive` too). Package dry-run passes on every PR.

## Risks & Blockers

- **None for this feature** — additive, default-identical, fully tested, merged green.
- **crates.io is irreversible** — do not publish without an explicit user go.
- **CLAUDE.md is 215 lines** (over the soft 200-line guideline) — pre-existing, not introduced here
  (this PR's edits were in-place line replacements, net 0 lines). If it grows further, move a
  module-map detail into a `docs/*.md` per the growth-strategy rule.

## Open Questions

- Should the focus ring eventually support corner radius / animation, or is the flat 4-rect border
  sufficient for the skeleton? (No demand yet; left as optional future work.)
- Which of the remaining backlog items is next — the 64-tile hex atlas (big), per-OS input (env-gated),
  or crates.io (irreversible)? (User's call next session.)

## Quick Start for Next Session

```bash
cd /Users/jkl/Projects/skeleton-engine
git log --oneline -3            # 49d5019 (#144 v0.42.0) … 2a73efa (#143) … 2dc5a48 (#142)
grep -m1 '^version' Cargo.toml  # 0.42.0
rm -f /tmp/v.txt && ./scripts/verify.sh > /tmp/v.log 2>&1; echo $? > /tmp/v.txt; cat /tmp/v.txt
# R1: read the exit FILE, not the wrapper's "exit 0"; always use a FRESH path.

# Key files for this feature:
#   src/ui/focus.rs                  (FocusRingStyle resource + is_visible + doctest)
#   src/ui/system/focus_pass.rs      (push_ring takes &FocusRingStyle; call site reads the resource; 5 tests)
#   src/app/core_resources.rs        (auto-insert next to UiFocus)
#   examples/ui_focus.rs             (cyan ring demo — the VISION acceptance test)

# Next action (pick one — nothing is required; the feature is shipped & merged):
#   (A) 64-tile hex autotile atlas + full-blob example (the other seq-38/39 follow-up; big art task), OR
#   (B) per-OS input optimization + the deferred gamepad hardware test, OR
#   (C) crates.io publish (explicit go required).
```

---

## Cross-cutting gotchas (expensive-to-rediscover)

1. **R1 recurred and was caught.** The background-task notification prints "exit code 0" — that's the
   wrapper's trailing `echo`, NOT the gate's verdict. The real exit lived in the file as **1** (a
   `cargo fmt --check` diff in the example's `use engine::{…}` import block, which rustfmt re-wraps).
   Always read the exit *file* and use a fresh path. Bit me once this session (caught), as the seq-40
   rule intends.
2. **rust-analyzer flags deleted files as "unlinked".** `examples/gamepad_probe.rs` /
   `gamepad_state_probe.rs` (deleted in seq-42) showed up as unlinked-file diagnostics — stale
   rust-analyzer state, NOT real (confirmed `ls` → "No such file"). Don't chase IDE diagnostics for
   files git says don't exist.
3. **Integration-testing UI render output** goes through `UiQueue` (its `items: Vec<DrawRect>` is
   `pub`). A hand-built test `World` must `insert_resource(UiQueue::default())` or `submit_output`
   silently drops the rects. Filter `items` by a distinctive color to isolate one pass's output.
4. **A default that must stay byte-identical → read with `unwrap_or_default()` + a `Default` that
   reproduces the old constants.** This keeps every existing call site (and every test world that
   never inserts the resource) producing exactly the prior output, while making the value overridable.
5. **`#[derive] Copy` on a small config resource** lets the focus-pass call site do `.copied()` and
   sidestep holding a `&` borrow of `world` across the `push_ring` call.

## Process / versioning notes

- 0.x cadence: additive feature = MINOR → **v0.42.0**. `/ship` four-edit set (Cargo.toml + lock +
  CHANGELOG + CLAUDE.md header) + module-map row 122. No tag (none requested).
- VISION loop honored: the feature's acceptance test is the `ui_focus` example (restyled to a cyan
  ring, builds + runs). Pure unit/integration coverage for the styling logic; no hardware dependency.
- Squash-merge via plain `gh pr merge --squash --delete-branch` after CI 4/4 green — merge driven by
  the user's direct "진행해", never a Korean AskUserQuestion option (classifier-safe).

---

## Session Closed
**Closed at:** 2026-06-19 (KST)
**Commit:** feature `49d5019` (#144, v0.42.0); this handoff committed separately + pushed.
**Session status:** Handed off to next session
