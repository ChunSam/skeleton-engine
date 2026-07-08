# Stepper numeric +/- spinner widget shipped (v0.118.0) + overdue memory-hygiene trim

**Date:** 2026-07-08
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `listbox-widget` seq `2`
**Parent:** `HANDOFF_listbox-widget_scrollable-list_2026-07-08.md`
**Prior chain:** `HANDOFF_listbox-widget_scrollable-list_2026-07-08.md` (seq 1) > this (seq 2)

> This session is a **direct continuation** of the `listbox-widget` chain. Seq 1's
> "Where We're Going" laid out exactly this session's work: §3 (board empty → ASK for
> direction; named candidate *"more widgets (Stepper/SpinBox was the runner-up this
> session)"*) and §4 (the overdue `engine-current-state.md` tip-line trim). This session
> executed both: the hygiene trim first, then `Stepper` (the named runner-up) as a
> user-chosen self-pick. The board was still empty, so it was a free-choice breadth pick,
> not a game-driven request.

---

## Related Handoffs

- `plans/handoffs/PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the
  READY-but-unfiled EW-007 pre-design (FloatingText bold/rich). Still the top candidate the
  moment the game files EW-007. Not touched this session. Reference only.
- `plans/handoffs/HANDOFF_dm-adoption_ew-verified-coordination_2026-07-03.md` — the prior
  (closed) `dm-adoption` chain's last leg. Separate, closed work stream; referenced only for
  board/standing state. Not a parent.

## Since Last Handoff

Parent (seq 1) planned four next-actions; here is what happened to each:

- **§1 "Land THIS handoff"** → done before this session (it became memory seq 164, merged as
  PR #344 `0268de8`). This session opened on a clean `main` at that tip.
- **§2 "Read the board FIRST; serve EW-007 if filed"** → board read first thing; **still
  EMPTY** (EW-007 unfiled — the game shipped its own content PRs #14/#15 but filed no engine
  request). Nothing to serve.
- **§3 "If board empty → ASK for direction"** → asked via `AskUserQuestion`; user answered
  **"추천 진행해"** (proceed with recommendation), then picked **"Stepper 위젯 (추천)"** from a
  second question. Stepper was the *exact* runner-up seq 1 named. Trajectory unchanged:
  widget-suite breadth self-picks while the board is quiet.
- **§4 "Overdue hygiene trim of `engine-current-state.md`"** → **done** (the risky ~4000-char
  single-line edit seq 1 deferred). Cut at the `dm-adoption` chain boundary, not seq 1's
  suggested "seq ≤141" — a cleaner cut (see Key Decisions).

No open questions from seq 1 remained; no risks materialized. Board still the gating signal.

## Reference Documents

- `CLAUDE.md` — project conventions + the UI module-map row (now includes Stepper).
- `.claude/skills/add-ui-widget/SKILL.md` — the 10-file widget-wiring pattern this followed.
- `docs/PATTERNS.md` — UI system order / render-target-format pipeline cache / register_event note.

## The Goal

Widen the engine's breadth-first feature set per `docs/VISION.md` (a forkable, genre-agnostic
2D skeleton, each feature proven by a small playable example). The widget suite
(Slider/CheckBox/ProgressBar/Tooltip/Dropdown/RadioGroup/TabBar/ListBox) had a clear gap: a
**numeric value-adjustment control** — the `-`/`+` spinner games reach for constantly (quantity
fields, discrete settings values). This session added `Stepper` to fill that gap, wired it
through the full widget signature, proved it with a playable `ui_stepper` example, and shipped
it as v0.118.0. It also cleared an overdue memory-hygiene item (the `engine-current-state.md`
tip line had grown to 28 seqs / 36 KB on one line, risking the 25k-token Edit cap).

## Where We Are

- **Shipped and merged.** PR #345 squash-merged (async auto-merge). `main` @ **`0be4b2f`**,
  package **v0.118.0**, CLAUDE.md header **v1.6.211**, tree clean, all gates green.
- `src/ui/stepper.rs` — new `Stepper` component (~230 code lines + 9 tests). Fields: `value`,
  `min`, `max`, `step`, `decimals: u32`, `font_size`, `bg_color`/`button_color`/
  `button_hover_color`/`text_color`/`border_color`, `corner_radius`, `border`. Builders
  `new(min,max,value)`/`with_step`/`with_decimals`/`with_font_size`/`with_colors`(4-color)/
  `with_corner_radius`/`with_border`. Helpers `clamped_value`/`stepped`/`at_min`/`at_max`/
  `label`/`button_width`/`zone_at`.
- `src/ui/stepper.rs` — new `StepButton { Dec, Inc }` enum, the return of `zone_at` (single
  geometry source shared by render + click resolution). Re-exported at `engine::StepButton`.
- `src/ui/system/stepper_pass.rs` — new pass (~170 code + 12 tests): click-select (CheckBox
  press+release ownership) + render. Inserted in the `UiSystem` order **after ListBox, before
  Dropdown** (pass step 13; the order doc is now 16 steps).
- `UiEvent::StepperChanged(Entity, f32)` — new variant, emitted only on an actual change. **An
  `f32` payload like `SliderChanged`, NOT a `usize` index** (Stepper is value-based).
- `focus_pass.rs` — Stepper added to `collect_focusables`; a value-based `else if` arm in the
  existing `nav_left || nav_right` block (steps `value` by `step`, clamped, emits on change).
- `capture.rs` — `Stepper` registered pointer-opaque via `extend_kind::<Stepper>` (whole node).
- `system.rs` — `mod stepper_pass;` + `stepper_scratch` field + `run()` call + pass-order doc.
- `mod.rs` + `lib.rs` — `pub mod stepper;` + `pub use stepper::{StepButton, Stepper};` and the
  `engine::{… StepButton, Stepper …}` crate re-export.
- `core_resources.rs` + `component_registry.rs` — reflect/clone/serde + editor add/remove,
  mirroring the other widgets.
- `examples/ui_stepper.rs` — playable demo (volume 0..100/5 · difficulty 1..5 · custom-green
  zoom 0.5..3.0 step 0.25 decimals 2 + live HUD readout + change counter + `HEADLESS_SHOT`).
  Flat example (cargo auto-discovers `examples/*.rs`, no `Cargo.toml` entry).
- **21 new unit tests pass** (9 in `stepper.rs` + 12 in `stepper_pass.rs`) + 1 doctest. Full
  suite green; `./scripts/verify.sh` all 7 gates green **twice** (pre- and post-`/ship`).
- Headless capture (`/tmp/ui_stepper.png`) eyeballed: three steppers with `-`/`+` buttons,
  centered values (70 / 5 / 2.25), framed borders, custom-green zoom styling. Clean.
- **Hygiene trim done.** `engine-current-state.md` tip line: **36,444 → 13,336 chars** (28 → 12
  seqs). seq 137–152 moved verbatim to `[[engine-history-archive]]` new `## Trim 2026-07-08`
  section; no content lost (Python surgical edit, verified). Pointer updated.
- **Memory bumped to seq 165** (the Stepper code PR). After THIS handoff lands → seq 166.
- **Additive** — no existing widget or event changed (only a new `UiEvent` variant, technically
  breaking for exhaustive matches but pre-1.0 licensed, and new re-exports).

## What We Tried (Chronological)

1. **Onboarding (as the paste-prompt asked).** Read the seq-1 parent handoff, read the board
   (`../dungeon-merchant/docs/engine-wishlist.md` — EMPTY), verified state (`git log` → `0268de8`
   #344; `cargo test --lib list_box` → 20 green; tree clean). Read the key files + adjacent ones
   (add-ui-widget skill, focus_pass, the memory tip line). Reported understanding in Korean, waited.
2. **User: "추천 진행해."** My recommendation was trim-first, then optionally a widget. Started
   the hygiene trim.
3. **Hygiene trim.** Read the archive's structure (grep for section headers → newest-trim-first
   convention, sections are giant single lines). Found the `## Trim 2026-07-02` format
   (`## header` / blank / `(parenthetical cut note)` / blank / `<verbatim chunk>`). Located the
   cut boundaries in the tip line via Python (`prior = seq 152` at idx 11370, the old
   `older chain →` pointer at 34734, `Board` at 34910). **Dry-ran** the surgery (extract
   `archived_chunk = tip[11370:34734]`, sanity-check seq 153 stays live / seq 152 gone / seq 137
   + #313 in the chunk). Applied via a single Python script (backup to `/tmp/{ecs,eha}.bak`
   first): edit line 9 (new hash/version/header + updated pointer), insert a new archive section
   after line 12. Verified: seqs 153–164 live, 152/137 archived, Board para + Live gotchas intact,
   archive 89→95 lines.
4. **Asked for the next work** (`AskUserQuestion`, board empty). Offered Stepper (recommended) /
   other widget / pause. User picked **Stepper**.
5. **Loaded `/add-ui-widget`.** Decided the interaction model FIRST: **value-based click-select**
   (not index-based like the recent widgets; not press-open like Dropdown). Payload = `f32`.
6. **Front-loaded reads** (why the wiring compiled first try): `slider.rs` (the value-based
   template), `radio_group_pass.rs` (the click-select pass template), `system.rs` (pass order +
   struct), `event.rs`, `capture.rs`, `state.rs` (`InputSnapshot`/`node_layout`/`UiOutput`),
   `button_pass.rs` (how labels center: `.with_bounds(...).with_align(TextAlign::Center)`), the
   registration blocks (`core_resources`/`component_registry`), `mod.rs`, `lib.rs`, and
   `ui_list_box.rs` (the example headless pattern).
7. **Wrote component + pass (21 tests).** `cargo test --lib stepper` → **21/21 green on the
   first run.** Wired files 3–8 in order; the only compile errors were the expected
   "not yet wired" ones (E0432/E0599) which cleared as each wiring edit landed.
8. **Wrote the example, generated the headless PNG, eyeballed it** → correct three steppers,
   buttons, centered values, custom styling.
9. **`cargo fmt`** (avoid the reflow trap — it reformatted the fresh `with_colors`/`DrawText`
   wrapping) then **`cargo test --doc stepper`** → the doctest passes.
10. **CLAUDE.md UI row** appended (Stepper clause + `src/ui/stepper.rs` in the file list), then
    **`./scripts/verify.sh`** (background, exit read non-piped) → **all 7 gates green.**
11. **`/ship`** 0.117.0 → 0.118.0 (Cargo.toml + `cargo update -p skeleton-engine` lock refresh +
    CHANGELOG 0.118.0 entry + CLAUDE.md header v1.6.210 → v1.6.211). Re-ran verify.sh → green
    again (version bump forces a recompile). Confirmed `Cargo.lock` shows `version = "0.118.0"`.
12. **`/land-pr` Async:** branch `feat/ui-stepper` → commit → push → PR **#345** →
    `gh pr merge 345 --auto --squash` (armed; `mergeStateStatus: BLOCKED` = checks running).
    Background poll loop (`until [ state = MERGED ]; sleep 30`) → **MERGED `0be4b2f`**. Deferred
    wrap-up: `git checkout main && git pull --ff-only`, `git branch -D feat/ui-stepper`, memory
    **seq 165** bump (Python-verified coherent, tip now 16,043 chars).

## Key Decisions

- **Value-based (`f32`), not index-based.** `UiEvent::StepperChanged(Entity, f32)` mirrors
  `SliderChanged`, not the `…Changed(Entity, usize)` shape the `/add-ui-widget` skill template
  and every recent widget (Dropdown/RadioGroup/TabBar/ListBox) use. A stepper's meaningful
  payload is a *value*, not a *selection*. Deliberate deviation from the template, like Slider.
- **Persist `value` directly** — NOT Slider's `initial_value` (serialized) + `value`
  (`#[serde(skip)]` runtime) split. Slider splits because a drag mutates `value` continuously
  every frame; a Stepper's discrete steps make direct persistence correct and simpler (matches
  `RadioGroup.selected`/`ListBox.selected`, which also persist their runtime-mutated state).
- **No transient state.** Button hover is recomputed each frame from `PointerCapture`
  (`zone_at(input.cursor, …)`), so there are no `#[serde(skip)]` fields (unlike Dropdown's
  `open`/`press_opened`). Cleaner and one fewer footgun.
- **`clamped_value` uses `.max(min).min(max)`, NOT `f32::clamp`.** `f32::clamp` *panics* when
  `min > max`; a Reflect/Inspector edit could momentarily leave `min > max`. `.max(min).min(max)`
  degrades gracefully (returns `max`). Pinned by test
  `clamped_value_survives_inverted_bounds_without_panicking`.
- **Button width = `node.y.min(node.x/3).max(0)`** — square buttons (width = node height), capped
  at a third of the node width so the two buttons + the value label always fit even on a
  short-wide or tall-narrow node. `0` for a degenerate node (guards the render/click path).
- **ASCII `-`/`+` glyphs** (not U+2212 minus / other), centered via
  `DrawText::with_bounds(...).with_align(TextAlign::Center)` at layered z — maximum font safety
  (no tofu risk) and the exact centering pattern `button_pass`/`tab_bar_pass` already use.
- **Inserted after ListBox / before Dropdown** in the pass order. Order among click-select
  widgets is cosmetic (PointerCapture handles occlusion), but Dropdown draws an overlay list
  above everything so it stays last-but-tooltip.
- **Rejected: dimming the `-`/`+` button at a bound** (the "disabled" spinner look). Kept v1 lean
  — clamping handles bounds silently, and `at_min()`/`at_max()` are exposed for a game that wants
  the visual. Revisit only if a game reports it looks off.
- **Trim cut at the `dm-adoption` chain boundary (seq ≤152), not seq 1's suggested "seq ≤141".**
  The file's own rule is "keep the current chain + one prior session"; keeping `listbox-widget`
  (162–164) + `dm-adoption` (153–161) and archiving ≤152 lands on a clean chain boundary (seq 152
  DOCS-PATTERNS ends the archived region, seq 153 HANDOFF #333 starts `dm-adoption`).

## Evidence & Data

### Version / PR

| Item | Value |
|---|---|
| Package version | 0.117.0 → **0.118.0** (MINOR — new public API) |
| CLAUDE.md header | v1.6.210 → **v1.6.211** |
| PR | **#345**, squash-merged (async auto-merge) |
| main tip | `0be4b2f` |
| Memory global seq | **165** (code PR); this handoff PR will be seq 166 |
| Diff | 15 files, 1087 insertions, 10 deletions |
| Async landing | 13th unattended auto-merge (armed `--auto`, bg poll loop confirmed MERGED) |

### Tests (21 new unit + 1 doctest, all green)

| File | Count | Coverage |
|---|---|---|
| `src/ui/stepper.rs` | 9 | new-clamps-value, stepped-clamps-at-bounds, at_min/at_max, inverted-bounds-no-panic, label-decimals, zone_at-buttons-only, button_width-⅓-cap, serde-roundtrip, reflect-roundtrip |
| `src/ui/system/stepper_pass.rs` | 12 | +click increments+emits, −click decrements+emits, center-area no-op, +at-max silent, −at-min silent, covered-by-panel no-op, drag-off cancels, press-+/release-− uses release button, focused ←/→ step+emit, focused-arrow-at-bound silent, renders bg/2-buttons/border/value+glyphs, degenerate-node no-panic |

Full suite: `test result: ok. 98 passed` (lib unit slice) + doctests incl. the Stepper doctest.
`[verify] all checks passed ✓` twice (pre- and post-`/ship`).

### The 10-file widget wiring signature (all present)

| # | File | What |
|---|---|---|
| 1 | `src/ui/stepper.rs` (new) | `Stepper` + `StepButton` + Reflect + serde + geometry helpers + 9 tests |
| 2 | `src/ui/system/stepper_pass.rs` (new) | `pub(super) fn run(...)` click-select + render + 12 tests |
| 3 | `src/ui/system.rs` | `mod stepper_pass;` + `stepper_scratch` field + `run()` call (after ListBox) + pass-order doc (now 16 steps) |
| 4 | `src/ui/system/event.rs` | `UiEvent::StepperChanged(Entity, f32)` |
| 5 | `src/ui/system/focus_pass.rs` | collect + value-based ←/→ arm in the `nav_left \|\| nav_right` block |
| 6 | `src/ui/system/capture.rs` | `extend_kind::<Stepper>(world, viewport, 0.0)` |
| 7 | `src/ui/mod.rs` + `src/lib.rs` | `pub mod stepper;` + `pub use …{StepButton, Stepper}` re-exports |
| 8 | `src/app/core_resources.rs` + `src/app/editor/component_registry.rs` | reflect/clone/serde + add/remove |
| 9 | `examples/ui_stepper.rs` (new) | playable demo + `HEADLESS_SHOT` self-check |
| 10 | `CLAUDE.md` UI row + `/ship` paperwork | module-map clause + version/CHANGELOG |

### Hygiene trim (before / after)

| Item | Before | After |
|---|---|---|
| `engine-current-state.md` tip line | 36,444 chars (seq 137–164, 28 seqs) | 13,336 → then 16,043 after the seq-165 bump (seq 153–165, 13 seqs) |
| `engine-history-archive.md` | 89 lines | 95 lines (new `## Trim 2026-07-08` section) |
| Cut boundary | — | `dm-adoption` chain boundary (archived seq 137–152) |
| Backups | — | `/tmp/ecs.bak`, `/tmp/eha.bak` (this-session safety net) |

## Code Analysis

- **`Stepper::zone_at(cursor, pos, node_size) -> Option<StepButton>`** — the single geometry
  source. `bw = button_width(node_size)`; inside-y test; `[pos.x, pos.x+bw)` → `Dec`,
  `[pos.x+w-bw, pos.x+w)` → `Inc`, else `None` (the central value area / outside). Guards `bw<=0`.
- **`Stepper::stepped(forward) -> f32`** — `(clamped_value() ± step).max(min).min(max)`. At a
  bound it returns the bound == current, so the pass's `(nv - clamped_value()).abs() > EPSILON`
  guard stays silent (no event). Both the click path and the focus ←/→ arm use this.
- **`Stepper::clamped_value() -> f32`** — `value.max(min).min(max)` (panic-safe vs `f32::clamp`).
  `at_min()`/`at_max()` compare it to the bounds; `label()` = `format!("{:.*}", decimals, clamped)`.
- **`stepper_pass::run`** click block — `clicked = just_released && pressed_owner == Some(e) &&
  released_owner == Some(e)`; then `zone_at(release_cursor, …)` → `stepped(btn == Inc)` →
  emit-on-change. Render: bg rect (rounded, z) → 2 button rects (z+step, hover-tinted via
  `button_color(st, hovered)`) → glyphs + value label (z+step*2, centered) → border (z+step*3).
- **`focus_pass` arm** (value-based, after the ListBox arm in `nav_left || nav_right`):
  `let nv = st.stepped(input.nav_right); if (nv - st.clamped_value()).abs() > EPSILON { st.value =
  nv; push StepperChanged }`. Gamepad steps via ←/→ (its D-pad/stick Up/Down cycle focus).
- **`push_centered(output, text, x, width, node_y, node_h, st, z)`** — shared helper (`#[allow
  (clippy::too_many_arguments)]`), `text_y = node_y + (node_h - font_size)/2`,
  `with_bounds((width, node_h)).with_align(TextAlign::Center).with_z(z)`.
- **Default style:** `bg` rgba(.10,.11,.15,1), `button` rgba(.20,.22,.28,1), `button_hover`
  rgba(.30,.34,.44,1), `text` rgba_u8(212,214,222,255), `border` rgba(.34,.36,.44,1), `min` 0 /
  `max` 100 / `step` 1 / `decimals` 0 / `font_size` 16 / `corner_radius` 6 / `border` 1.

## Files Changed

### Source code (new)
- `src/ui/stepper.rs` — the `Stepper` component + `StepButton` enum + Reflect + geometry helpers.
- `src/ui/system/stepper_pass.rs` — click-select + render pass.
- `examples/ui_stepper.rs` — playable demo + headless self-check.

### Source code (modified)
- `src/ui/system.rs` — pass registration (mod / scratch field / call after ListBox / doc-comment).
- `src/ui/system/event.rs` — `UiEvent::StepperChanged`.
- `src/ui/system/focus_pass.rs` — collect + value-based ←/→ arm.
- `src/ui/system/capture.rs` — `Stepper` pointer-opaque.
- `src/ui/mod.rs`, `src/lib.rs` — re-exports.
- `src/app/core_resources.rs`, `src/app/editor/component_registry.rs` — registrations.

### Docs / release
- `CLAUDE.md` — UI module-map row clause + header v1.6.211 / v0.118.0.
- `docs/CHANGELOG.md` — 0.118.0 entry.
- `Cargo.toml`, `Cargo.lock` — 0.118.0.

### Memory (not in git — `~/.claude/.../memory/`)
- `engine-current-state.md` — hygiene trim (seq 137–152 out) + seq-165 bump.
- `engine-history-archive.md` — new `## Trim 2026-07-08` section (verbatim seq 137–152).

## User Feedback & Preferences

- Opened via a paste prompt (seq-1 handoff continuation); wanted onboarding narrated, then a
  wait for go-ahead before executing.
- **"추천 진행해"** (proceed with your recommendation) — greenlit the trim-first plan without
  dictating specifics. When the definite part (trim) was framed as certain and the widget part as
  "원하시면" (optional), the user's "추천" covered the trim; I then asked before the widget.
- From the next-work question, picked **"Stepper 위젯 (추천)"** — the recommended option.
- Standing preferences (from memory, still in force): user-facing reports in **Korean**,
  agent-to-agent/code/docs in English; **merge authority delegated** (squash on green CI, no
  per-session re-confirm); **async auto-merge is the default landing** for CI-verifiable changes;
  always pass an explicit `model` to subagents.
- Then invoked **`/handoff`** (this doc).

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `listbox-widget` seq 2), async
   auto-merge. On merge, bump memory to **seq 166** (handoff PR) pointing at the handoff merge hash.
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). If a
   request is filed (EW-007+), serve it priority-order. EW-007 (FloatingText bold/rich) has a
   READY pre-design note — serve same-day per that plan.
3. **If the board is still empty → ASK the user for direction** (self-pick queue is now quite
   thin). The obvious value-adjustment gap is filled; remaining widget candidates are softer:
   a Switch/Toggle (styled boolean), a color swatch, a menu/menu-bar, or a multi-line/chrome-less
   Button variant (the game flagged that in EW-005's thread but did not file it). Or pivot to
   non-widget breadth. **Do NOT pre-implement an unfiled board request** (work-around-first pipe).
4. **Hygiene: the tip line is healthy again** (seq 153–165, ~16 KB). Next trim due ~seq 172
   (keep current chain + one prior). The trim procedure is now proven (Python surgical edit;
   dry-run boundaries, back up to `/tmp`, verify no loss) — repeat it, don't hand-edit the giant line.

## Risks & Blockers

- None for the shipped feature (merged, green, verified).
- Minor, unhit this session: 2 audio tests can fail locally on a no-audio-device box (they
  passed here; verify.sh was green). Not a code risk.

## Open Questions

- None blocking. The one deliberate lean choice (no button dimming at a bound) is settled and
  documented; `at_min()`/`at_max()` are exposed if a game wants the disabled look. Revisit only
  if a game reports the always-lit buttons read oddly at a bound.

## Quick Start for Next Session

```bash
# No beads in this repo — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 165/166), listbox-widget chain seq 2.

# Reference docs
#   CLAUDE.md (UI module-map row now has Stepper + ListBox)
#   .claude/skills/add-ui-widget/SKILL.md (the 10-file pattern)

# Key files to read first (if extending Stepper or adding another widget)
#   src/ui/stepper.rs                   — the component + geometry helpers (zone_at/stepped)
#   src/ui/system/stepper_pass.rs       — click-select + render + hover
#   src/ui/system/focus_pass.rs         — the value-based ←/→ arm (mirrors Slider)
#   src/ui/slider.rs                    — the value-based template Stepper followed

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 0be4b2f (#345) or later
cargo test --lib stepper                                      # 21 tests green

# See it run
HEADLESS_SHOT=/tmp/ui_stepper.png cargo run --example ui_stepper   # or: cargo run --example ui_stepper

# Next action
#   Read ../dungeon-merchant/docs/engine-wishlist.md. If a request is filed, serve it
#   priority-order (EW-007 has a READY pre-design note). If STILL empty, ASK the user for
#   direction (self-pick queue thin — see Where We're Going §3).
```

## Session Closed

**Closed at:** 2026-07-08
**Session status:** Handed off to next session — this handoff lands as its own `docs(handoff)`
PR (chain `listbox-widget` seq 2), async auto-merge. The memory **seq-166 bump** (updating
`main @` to the handoff merge hash) is the next session's opening wrap-up, per the recorded
cadence ("deferred wrap-up done at seq-N start"). Code state at close: `main @ 0be4b2f`,
v0.118.0, tree clean, all gates green.
