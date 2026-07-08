# Switch styled boolean-toggle widget shipped (v0.119.0)

**Date:** 2026-07-09
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `listbox-widget` seq `3`
**Parent:** `HANDOFF_listbox-widget_stepper-widget_2026-07-08.md`
**Prior chain:** `HANDOFF_listbox-widget_scrollable-list_2026-07-08.md` (seq 1) > `HANDOFF_listbox-widget_stepper-widget_2026-07-08.md` (seq 2) > this (seq 3)

> This session is a **direct continuation** of the `listbox-widget` chain. Seq 2's "Where We're
> Going" laid out exactly this session's shape: §2 (board FIRST — serve EW-007 if filed) and §3
> (board empty → ASK; self-pick queue now thin, named candidates a Switch/Toggle, color swatch,
> menu-bar, or a non-widget pivot). This session executed the seq-166 deferred wrap-up first, then —
> board still empty — asked for direction; the user picked **Switch**, the first candidate seq 2
> named. A free-choice breadth self-pick, not a game-driven request.

---

## Related Handoffs

- `plans/handoffs/PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the
  READY-but-unfiled EW-007 pre-design (FloatingText bold/rich). Still the top candidate the moment
  the game files EW-007. Untouched this session. Reference only.

## Since Last Handoff

Parent (seq 2) planned four next-actions; here is what happened to each:

- **§1 "Land THIS handoff (seq 2) → bump memory to seq 166"** → done at THIS session's open (the
  deferred wrap-up): PR #346 confirmed merged (`3c43a3c`), `main` pulled, memory bumped to **seq
  166** pointing at the handoff merge. The session opened on a **stale** local `main` (see Key
  Decisions / Gotchas) and had to `git fetch` + `git pull --ff-only` first.
- **§2 "Read the board FIRST; serve EW-007 if filed"** → board read first thing; **still EMPTY**
  (EW-007 unfiled). Nothing to serve.
- **§3 "If board empty → ASK for direction (self-pick queue thin)"** → asked via `AskUserQuestion`
  with an honest framing (the widget suite is mature; pausing or a non-widget pivot are equally
  valid). User picked **"Switch/Toggle 위젯"** — the exact first candidate seq 2 named. Trajectory
  unchanged: widget-suite breadth self-picks while the board is quiet.
- **§4 "Hygiene: next trim ~seq 172"** → not due yet; tip line is healthy (18.4 KB after this bump).

No open questions from seq 2 remained; no risks materialized.

## Reference Documents

- `CLAUDE.md` — project conventions + the UI module-map row (now includes Switch).
- `.claude/skills/add-ui-widget/SKILL.md` — the 10-file widget-wiring pattern this followed.
- `docs/PATTERNS.md` — UI system order / render-target-format pipeline cache / register_event note.

## The Goal

Widen the engine's breadth-first feature set per `docs/VISION.md` (a forkable, genre-agnostic 2D
skeleton, each feature proven by a small playable example). The widget suite
(Slider/CheckBox/ProgressBar/Tooltip/Dropdown/RadioGroup/TabBar/ListBox/Stepper) had one soft gap
left: a **switch-look boolean toggle** — the settings-row control (Sound/Music/Fullscreen on-off)
that reads more naturally as a sliding switch than a tick box. This session added `Switch` to fill
that gap, wired it through the full 10-file widget signature, proved it with a playable `ui_switch`
example, and shipped it as v0.119.0.

## Where We Are

- **Shipped and merged.** PR #347 squash-merged (async auto-merge). `main` @ **`b2d8719`**, package
  **v0.119.0**, CLAUDE.md header **v1.6.212**, tree clean, all gates green.
- `src/ui/switch.rs` — new `Switch` component (~230 lines + 8 tests). Fields: `on: bool`, `label`,
  `track_width`, `track_height`, `on_color`/`off_color`/`knob_color`/`text_color`, `font_size`.
  Builders `new(label)`/`with_on`/`with_size`/`with_colors`(3-color)/`with_text_color`/
  `with_font_size`. Helpers `track_color`/`track_rect`/`knob_rect`/`track_radius`/`knob_radius` +
  private `track_size`. Module-level `const KNOB_PAD: f32 = 3.0`.
- `src/ui/system/switch_pass.rs` — new pass (~105 code + 10 tests): whole-node click-toggle
  (CheckBox press+release ownership) + render (pill track, round knob, optional label). Inserted in
  the `UiSystem` order **after Stepper, before Dropdown** (pass step 14; the order doc is now 17
  steps).
- `UiEvent::SwitchToggled(Entity, bool)` — new variant, the new on/off state. **A `bool` payload
  like `CheckBoxToggled`, NOT a `usize` index** (Switch is a boolean, not a selection).
- `focus_pass.rs` — Switch added to `collect_focusables`; a toggle arm in the `activate` block
  (mirrors CheckBox); a **value-absolute** arm in the `nav_left || nav_right` block (← sets off, →
  sets on, emit only on change).
- `capture.rs` — `Switch` registered pointer-opaque via `extend_kind::<Switch>` (whole node).
- `system.rs` — `mod switch_pass;` + `switch_scratch` field + `run()` call + pass-order doc.
- `mod.rs` + `lib.rs` — `pub mod switch;` + `pub use switch::Switch;` and the `engine::{… Switch …}`
  crate re-export.
- `core_resources.rs` + `component_registry.rs` — reflect/clone/serde + editor add/remove, mirroring
  the other widgets.
- `examples/ui_switch.rs` — playable demo (Sound on / Music / Fullscreen default-style + a larger
  custom-green Vsync + live on/off HUD readout + change counter + `HEADLESS_SHOT`). Flat example
  (cargo auto-discovers `examples/*.rs`, no `Cargo.toml` entry).
- **18 new unit tests pass** (8 in `switch.rs` + 10 in `switch_pass.rs`) + 1 doctest. Full suite
  green; `./scripts/verify.sh` all 7 gates green **twice** (pre- and post-`/ship`).
- Headless capture (`/tmp/ui_switch.png`) eyeballed: four switches — Sound (blue, on, knob right),
  Music (blue, on), Fullscreen (gray, off, knob left), Vsync (custom green, larger, on) — rounded
  pills, circular knobs, correct left/right knob positions and per-state colors, live readout. Clean.
- **Memory bumped to seq 167** (the Switch code PR). After THIS handoff lands → seq 168.
- **Additive** — no existing widget or event changed (only a new `UiEvent` variant, technically
  breaking for exhaustive matches but pre-1.0 licensed, and new re-exports).

## What We Tried (Chronological)

1. **Onboarding (as the paste-prompt asked).** Read the seq-2 parent handoff — but it **did not
   exist locally** at the path the paste named. `gh pr view 346` showed it MERGED (2026-07-08T10:38Z)
   yet local `main` tip was `0be4b2f` (#345). `git fetch` revealed `0be4b2f..3c43a3c` — local `main`
   was one commit stale. `git pull --ff-only` brought the seq-2 handoff file down; then read it.
2. **Deferred wrap-up (seq-166 bump).** Confirmed #346 merged, `main` synced to `3c43a3c`, bumped
   memory `engine-current-state.md` to **seq 166** (main@ → 3c43a3c, tip = HANDOFF #346, seq 165
   demoted to prior) via an asserted single-occurrence Python replace.
3. **Read the board** (`../dungeon-merchant/docs/engine-wishlist.md`) — **EMPTY** (EW-007 unfiled;
   EW-004/005/006 all Verified + archived). Verified state: `cargo test --lib stepper` → 21 green;
   git log shows `0be4b2f`/#345 + `3c43a3c`/#346.
4. **Onboarding reads.** Key files (`stepper.rs`, `stepper_pass.rs`, `focus_pass.rs`) + adjacent
   ones NOT in the paste's list (the `/add-ui-widget` skill, `system.rs` pass order) to know the
   exact insertion point and guardrails for a new widget. Reported understanding in Korean, waited.
5. **Asked for direction** (`AskUserQuestion`, board empty) with an honest 3-option framing: **대기
   (board-signal, my honest recommendation)** / **Switch/Toggle 위젯** / **비위젯 브레드스**. User
   picked **Switch**.
6. **Loaded `/add-ui-widget`.** Decided the interaction model FIRST: **click-select** (whole-node
   toggle, like CheckBox), payload = `bool`. Read the closest template — `checkbox.rs`,
   `checkbox_pass.rs` — plus `event.rs`, `capture.rs`, the registration blocks, and `ui_stepper.rs`
   (the example headless pattern).
7. **Wrote component + pass (18 tests).** `cargo test --lib switch` → **18/18 green + 6 unrelated
   "switch"-named tests** on the first run after wiring. The only compile errors were the expected
   "not yet wired" ones (E0432/E0599) which cleared as each wiring edit landed.
8. **Wired files 3–8 in order** (system.rs pass-order + doc, event.rs variant, capture, focus_pass
   activate + ←/→ arms + collect, mod/lib re-exports, core_resources + component_registry).
9. **`cargo fmt`** (avoid the reflow trap — it wrapped the fresh `lib.rs` re-export list and
   registration blocks) then wrote the example, generated the headless PNG, **eyeballed it** → four
   correct switches, rounded pills, circular knobs, left/right positions, custom green.
10. **CLAUDE.md UI row** appended (Switch clause + `src/ui/switch.rs` in the file list), then
    **`./scripts/verify.sh`** (background, exit read non-piped) → **all 7 gates green**.
11. **`/ship`** 0.118.0 → 0.119.0 (Cargo.toml + `cargo update -p skeleton-engine` lock refresh +
    CHANGELOG 0.119.0 entry + CLAUDE.md header v1.6.211 → v1.6.212). Re-ran verify.sh → green again.
    Confirmed `Cargo.lock` shows `version = "0.119.0"`.
12. **`/land-pr` Async:** branch `feat/ui-switch` → commit `c6bd10a` → push → PR **#347** →
    `gh pr merge 347 --auto --squash` (armed; `mergeStateStatus: BLOCKED` = checks running).
    Background poll loop (`until [ state = MERGED ]; sleep 30`) → **MERGED `b2d8719`**
    (2026-07-08T15:48Z). Deferred wrap-up: `git checkout main && git pull --ff-only`,
    `git branch -D feat/ui-switch`, memory **seq 167** bump (Python-verified, tip now 18,428 chars).

## Key Decisions

- **Click-select (whole-node), like CheckBox — NOT press-open.** Clicking anywhere on the node flips
  `on` (`toggled = just_released && pressed_owner == Some(e) && released_owner == Some(e)`; drag-off
  cancels). The capture surface is the whole node (`extend_kind::<Switch>`), so the label is
  clickable too. Copied the block from `checkbox_pass.rs`, as the skill directs.
- **←/→ set the state ABSOLUTELY (← off, → on), not toggle.** This is the deliberate differentiator
  from CheckBox: a switch is spatially a 2-position control, so ← = off / → = on reads naturally and
  makes it fully pad/keyboard-operable. Emits only on an actual change (→ on an already-on switch is
  silent). **Enter/Space toggle** (mirrors CheckBox's activate arm). CheckBox itself has NO ←/→ arm,
  so this is strictly more capable.
- **Payload `bool`, not `usize` index.** `UiEvent::SwitchToggled(Entity, bool)` mirrors
  `CheckBoxToggled`, not the `…Changed(Entity, usize)` shape the recent selection widgets
  (Dropdown/RadioGroup/TabBar/ListBox) use. A switch's meaningful payload is a *state*, not a
  *selection* — deliberate deviation from the skill template, like Stepper's `f32`.
- **Persist `on` directly** — no `#[serde(skip)]` transient state. A switch has no continuous
  drag/open state to hide (unlike Slider's `initial_value`/`value` split or Dropdown's `open`);
  button hover is recomputed each frame from the geometry helpers. Cleaner, one fewer footgun —
  matches Stepper's no-transient-state choice.
- **Single geometry source: `track_rect` / `knob_rect`.** One place computes the pill track (left-
  aligned, vertically centered, height clamped to the node) and the knob (a square of diameter
  `track_h − 2·KNOB_PAD`, flush left when off / right when on). Render consults them; the click path
  is whole-node so it needs no zone math — but the helpers still centralize the visual geometry and
  are unit-tested (`knob_sits_left_when_off_and_right_when_on`). Pill radius = `track_h/2`, knob
  radius = `(track_h − 2·pad)/2` (a circle), both via the UI SDF pipeline (`with_corner_radius`).
- **`track_size` clamps defensively** — `track_width.max(0.0)` and `track_height.min(node.y).max(0.0)`
  so a zero/short node can't produce a negative knob diameter (test
  `degenerate_node_has_a_non_negative_knob`). No `f32::clamp`-style panic surface exists here (no
  min/max bounds), unlike Stepper.
- **Inserted after Stepper / before Dropdown** in the pass order (step 14). Order among click-select
  widgets is cosmetic (PointerCapture handles occlusion), but Dropdown draws an overlay list above
  everything so it stays last-but-tooltip; this follows the established "new click-select widget goes
  just before Dropdown" convention (ListBox, Stepper did the same), minimizing renumber churn.
- **Default style = the CheckBox accent.** `on_color` = blue rgba(0.28,0.56,0.90,1) (== CheckBox's
  `checked_color`), `off_color` = dark gray rgba(0.24,0.25,0.30,1), `knob_color` = near-white
  rgba_u8(235,237,242). Track 46×24. Keeps the suite visually coherent.
- **Rejected: an animated knob slide.** The knob SNAPS between the off/on positions (no tween/
  transient animation clock), consistent with the suite's immediate-mode widgets and their
  no-transient-state discipline. A game wanting a slide can tween the render itself; revisit only if
  requested.
- **Stale-main gotcha handled up front.** Because the "deferred wrap-up at seq-N start" cadence lands
  the *prior* session's handoff PR after that session closed, this session's local `main` was one
  commit behind origin at open. Fetch + ff-pull before touching memory — else the seq-166 bump would
  have pointed `main @` at the wrong hash.

## Evidence & Data

### Version / PR

| Item | Value |
|---|---|
| Package version | 0.118.0 → **0.119.0** (MINOR — new public API) |
| CLAUDE.md header | v1.6.211 → **v1.6.212** |
| PR | **#347**, squash-merged (async auto-merge) |
| main tip | `b2d8719` (was `3c43a3c` after the seq-166 wrap-up) |
| Commit (pre-squash) | `c6bd10a` on `feat/ui-switch` |
| Memory global seq | **167** (code PR); this handoff PR will be seq 168 |
| Async landing | 15th unattended auto-merge (armed `--auto`, bg poll loop confirmed MERGED 2026-07-08T15:48Z) |

### Tests (18 new unit + 1 doctest, all green)

| File | Count | Coverage |
|---|---|---|
| `src/ui/switch.rs` | 8 | new-is-off, track_color-follows-state, knob-left-off/right-on, track-centered+height-clamped, pill+knob-round, degenerate-node-non-negative-knob, serde-roundtrip, reflect-roundtrip |
| `src/ui/system/switch_pass.rs` | 10 | click toggles-on+emits, click again toggles-off, label-area click also toggles, covered-by-panel no-op, drag-off cancels, focused-Space toggles+emits, focused →on/←off, focused → when already-on silent, renders track+knob+label, degenerate-node no-panic |

Full suite: lib unit slice green + the Switch doctest (`Switch::new("Sound").with_on(true)`).
`[verify] all checks passed ✓` twice (pre- and post-`/ship`). CI: 5/5 required checks green.

### The 10-file widget wiring signature (all present)

| # | File | What |
|---|---|---|
| 1 | `src/ui/switch.rs` (new) | `Switch` + Reflect + serde + geometry helpers (`track_rect`/`knob_rect`) + 8 tests |
| 2 | `src/ui/system/switch_pass.rs` (new) | `pub(super) fn run(...)` whole-node click-toggle + render + 10 tests |
| 3 | `src/ui/system.rs` | `mod switch_pass;` + `switch_scratch` field + `run()` call (after Stepper) + pass-order doc (now 17 steps) |
| 4 | `src/ui/system/event.rs` | `UiEvent::SwitchToggled(Entity, bool)` |
| 5 | `src/ui/system/focus_pass.rs` | collect + activate-toggle arm + value-absolute ←/→ arm |
| 6 | `src/ui/system/capture.rs` | `extend_kind::<Switch>(world, viewport, 0.0)` (whole node) |
| 7 | `src/ui/mod.rs` + `src/lib.rs` | `pub mod switch;` + `pub use …Switch` re-exports |
| 8 | `src/app/core_resources.rs` + `src/app/editor/component_registry.rs` | reflect/clone/serde + add/remove |
| 9 | `examples/ui_switch.rs` (new) | playable demo + `HEADLESS_SHOT` self-check |
| 10 | `CLAUDE.md` UI row + `/ship` paperwork | module-map clause + version/CHANGELOG |

### Default style constants

| Field | Value |
|---|---|
| `track_width` / `track_height` | 46.0 / 24.0 |
| `on_color` | rgba(0.28, 0.56, 0.90, 1.0) (== CheckBox `checked_color`) |
| `off_color` | rgba(0.24, 0.25, 0.30, 1.0) |
| `knob_color` | rgba_u8(235, 237, 242, 255) |
| `text_color` | rgba_u8(210, 210, 220, 255) (== CheckBox) |
| `font_size` | 16.0 |
| `KNOB_PAD` (module const) | 3.0 |

## Code Analysis

- **`Switch::track_rect(pos, node_size) -> (Vec2, Vec2)`** — left-aligned, vertically centered; size
  = `track_size` = `(track_width.max(0), track_height.min(node.y).max(0))`. The single geometry
  source. `y = pos.y + (node.y − track_h)/2`.
- **`Switch::knob_rect(pos, node_size) -> (Vec2, Vec2)`** — square of side `d = (track_h −
  2·KNOB_PAD).max(0)`; `y = track_y + pad`; `x = track_x + pad` (off) or `track_x + track_w − pad −
  d` (on). Uses `self.on`.
- **`switch_pass::run`** click block — `toggled = just_released && pressed_owner == Some(e) &&
  released_owner == Some(e)`; then `sw.on = !sw.on; push SwitchToggled(e, sw.on)`. Render: track pill
  (`track_color()`, rounded `track_radius`, z) → knob (`knob_color`, rounded `knob_radius`, z+step) →
  label (right of the track, bounds-clamped, z+step, occludable).
- **`focus_pass` activate arm** — inserted between the CheckBox and Dropdown arms: `else if let
  Some(sw) = world.get_mut::<Switch>(e) { sw.on = !sw.on; push SwitchToggled(e, sw.on) }`.
- **`focus_pass` ←/→ arm** (value-absolute, after the Stepper arm in `nav_left || nav_right`): `let
  target = input.nav_right; if sw.on != target { sw.on = target; push SwitchToggled(e, target) }`.
  Gamepad reaches it via ←/→ (D-pad/stick Up/Down cycle focus).
- **Dual-modality note** — a boolean has no cross-widget invariant to enforce (unlike Dropdown's "at
  most one open"), so no keyboard-path invariant duplication was needed. Click sets `just_released`;
  keyboard never does — so click and Enter/Space/←→ can never double-fire in one frame.

## Files Changed

### Source code (new)
- `src/ui/switch.rs` — the `Switch` component + Reflect + geometry helpers + 8 tests.
- `src/ui/system/switch_pass.rs` — whole-node click-toggle + render pass + 10 tests.
- `examples/ui_switch.rs` — playable demo + headless self-check.

### Source code (modified)
- `src/ui/system.rs` — pass registration (mod / scratch field / call after Stepper / doc-comment 17 steps).
- `src/ui/system/event.rs` — `UiEvent::SwitchToggled(Entity, bool)`.
- `src/ui/system/focus_pass.rs` — collect + activate-toggle arm + value-absolute ←/→ arm + `Switch` import.
- `src/ui/system/capture.rs` — `Switch` pointer-opaque + import.
- `src/ui/mod.rs`, `src/lib.rs` — re-exports.
- `src/app/core_resources.rs`, `src/app/editor/component_registry.rs` — registrations.

### Docs / release
- `CLAUDE.md` — UI module-map row clause + header v1.6.212 / v0.119.0.
- `docs/CHANGELOG.md` — 0.119.0 entry.
- `Cargo.toml`, `Cargo.lock` — 0.119.0.

### Memory (not in git — `~/.claude/.../memory/`)
- `engine-current-state.md` — seq-166 bump (open, deferred wrap-up) + seq-167 bump (close).

## User Feedback & Preferences

- Opened via a paste prompt (seq-2 handoff continuation); wanted onboarding narrated, then a wait
  for go-ahead before executing. Narrated the 5-step onboarding in Korean, then waited.
- When the board was empty and I framed the decision honestly (widget suite mature; pausing or a
  non-widget pivot equally valid; **대기 marked as my recommendation**), the user overrode toward
  action and picked **"Switch/Toggle 위젯"**. Read: the user values continued visible breadth
  progress over strict work-around-first idleness, as long as the pick is clean and low-risk.
- **"핸드오프 하고 푸시하고 마무리"** — after the merge + wrap-up, greenlit writing the handoff and
  landing it to close the chain.
- Standing preferences (from memory, still in force): user-facing reports in **Korean**, agent-to-
  agent/code/docs in English; **merge authority delegated** (squash on green CI, no per-session
  re-confirm); **async auto-merge is the default landing** for CI-verifiable changes; always pass an
  explicit `model` to subagents.

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `listbox-widget` seq 3), async
   auto-merge. On merge, bump memory to **seq 168** (handoff PR) pointing at the handoff merge hash.
   This is the recorded "deferred wrap-up done at seq-N start" cadence — next session's opening step.
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). If a
   request is filed (EW-007+), serve it priority-order. EW-007 (FloatingText bold/rich) has a READY
   pre-design note — serve same-day per that plan.
3. **If the board is still empty → ASK the user for direction.** The self-pick widget queue is now
   **essentially exhausted** — Switch was the last clean common gap. Remaining candidates are all
   soft or blocked: a color swatch / picker, a menu / menu-bar, or the multi-line/chrome-less Button
   variant the game flagged in EW-005's thread **but did not file** (do NOT pre-implement — work-
   around-first pipe). The honest next move is likely a **non-widget breadth pivot** or **pausing
   for the board**. Recommend pausing/pivoting over another marginal widget.
4. **Hygiene: tip line is healthy** (seq 153–167, ~18.4 KB). Next trim due ~**seq 172** (keep current
   chain + one prior). Repeat the proven Python surgical edit (dry-run boundaries, back up to `/tmp`,
   verify no loss) — don't hand-edit the giant line.

## Risks & Blockers

- None for the shipped feature (merged, green, verified).
- Minor, unhit this session: 2 audio tests can fail locally on a no-audio-device box (they passed
  here; verify.sh was green). Not a code risk.

## Open Questions

- None blocking. The one deliberate lean choice (no animated knob slide) is settled and documented;
  a game can tween the render itself. Revisit only if a game reports the snap reads oddly.

## Quick Start for Next Session

```bash
# No beads in this repo — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 167/168), listbox-widget chain seq 3.

# Reference docs
#   CLAUDE.md (UI module-map row now has Switch + Stepper + ListBox)
#   .claude/skills/add-ui-widget/SKILL.md (the 10-file pattern)

# Key files to read first (if extending Switch or adding another widget)
#   src/ui/switch.rs                    — the component + geometry helpers (track_rect/knob_rect)
#   src/ui/system/switch_pass.rs        — whole-node click-toggle + render
#   src/ui/system/focus_pass.rs         — the activate-toggle + value-absolute ←/→ arms
#   src/ui/checkbox.rs                  — the boolean template Switch followed

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect b2d8719 (#347) or later
cargo test --lib switch                                       # 18 tests green (+ doctest)

# See it run
HEADLESS_SHOT=/tmp/ui_switch.png cargo run --example ui_switch   # or: cargo run --example ui_switch

# Next action
#   Read ../dungeon-merchant/docs/engine-wishlist.md. If a request is filed, serve it priority-order
#   (EW-007 has a READY pre-design note). If STILL empty, ASK the user — the self-pick widget queue
#   is now exhausted; recommend a non-widget pivot or pausing over another marginal widget.
```

## Session Closed

**Closed at:** 2026-07-09
**Session status:** Handed off to next session — this handoff lands as its own `docs(handoff)` PR
(chain `listbox-widget` seq 3), async auto-merge. The memory **seq-168 bump** (updating `main @` to
the handoff merge hash) is the next session's opening wrap-up, per the recorded cadence. Code state
at close: `main @ b2d8719`, v0.119.0, tree clean, all gates green.
