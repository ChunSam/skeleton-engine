# ListBox scrollable selectable list widget — shipped v0.117.0 (self-picked, board empty)

**Date:** 2026-07-08
**Status:** COMPLETED
**Bead(s):** none (repo uses `plans/handoffs/*.md`, no beads)
**Epic:** none
**Chain:** `listbox-widget` seq `1`
**Parent:** none — first in chain
**Prior chain:** none — first in chain

> This session is NOT a continuation of the `dm-adoption` chain (seqs 1–4, closed
> 2026-07-03 when EW-004/005/006 were all Verified and the board went empty). It is a
> fresh **self-pick**: the engine-wishlist board was empty, the user chose "다른 엔진
> 셀프픽", and picked `ListBox` from a 3-option `AskUserQuestion`.

---

## Related Handoffs

- `plans/handoffs/HANDOFF_dm-adoption_ew-verified-coordination_2026-07-03.md` — the prior
  (closed) chain's last leg. Separate work stream; referenced only for the board/standing
  state that led into this self-pick. Not a parent.
- `plans/handoffs/PLAN_dm-adoption_ew007-floating-text-rich-predesign_2026-07-03.md` — the
  READY-but-unfiled EW-007 pre-design (FloatingText bold/rich). Still the top candidate the
  moment the game files EW-007. Not touched this session.

## Reference Documents

- `CLAUDE.md` — project conventions + the UI module-map row (now includes ListBox).
- `.claude/skills/add-ui-widget/SKILL.md` — the 10-file widget-wiring pattern this followed.
- `docs/PATTERNS.md` — UI system order / render-target-format pipeline cache / register_event note.

## The Goal

Widen the engine's breadth-first feature set per `docs/VISION.md` (a forkable general-purpose
2D skeleton, each feature proven by a small playable example). The widget suite
(Slider/CheckBox/ProgressBar/Tooltip/Dropdown/RadioGroup/TabBar) had one obvious gap: a
**scrollable, selectable many-item list** — the control games reach for constantly
(inventory, level-select, file lists, dialogue-choice lists). This session added `ListBox`
to fill that gap, wired it through the full widget signature, proved it with a playable
`ui_list_box` example, and shipped it as v0.117.0. The board was empty, so it was a
free-choice breadth pick rather than a game-driven request.

## Where We Are

- **Shipped and merged.** PR #343 squash-merged 2026-07-07T15:59Z (auto-merge, async). `main`
  @ `68953ab`, package **v0.117.0**, CLAUDE.md header **v1.6.210**, tree clean, all gates green.
- `src/ui/list_box.rs` — new `ListBox` component (~310 lines w/ tests). Fields: `items`,
  `selected`, transient `scroll_offset` (`#[serde(skip)]`), `row_height`, `font_size`,
  `bg_color`/`hover_color`/`selected_color`/`text_color`/`border_color`, `corner_radius`,
  `border`. Builders `new`/`with_selected`/`with_row_height`/`with_font_size`/`with_colors`
  (4-color)/`with_corner_radius`/`with_border`(thickness+color). Scroll-aware geometry helpers.
- `src/ui/system/list_box_pass.rs` — new pass (~230 lines w/ tests): wheel-scroll + click-select
  + render. Inserted in `UiSystem` order **after TabBar, before Dropdown**.
- `UiEvent::ListBoxChanged(Entity, usize)` — new variant, emitted only on an actual change.
- `InputSnapshot.nav_up` / `nav_down` — new keyboard-only fields (Arrow ↑/↓), consumed ONLY by
  the ListBox focus arm.
- `focus_pass.rs` — ListBox added to `collect_focusables`; a `step_list_box` helper + a
  ←/→ arm (in the existing Left/Right block) + a separate ↑/↓ block.
- `capture.rs` — `ListBox` registered pointer-opaque via `extend_kind::<ListBox>`.
- `examples/ui_list_box.rs` — playable demo (12-item inventory that scrolls + custom-styled
  level select + HUD readout + change counter + `HEADLESS_SHOT`). Flat example (no Cargo.toml
  entry needed; cargo auto-discovers `examples/*.rs`).
- **20 new tests pass** (8 in `list_box.rs` + 12 in `list_box_pass.rs`). Full suite green.
- Headless capture eyeballed: inventory scrolled 3 rows (Torch/row 6 highlighted blue), rounded
  border frames both lists, partial rows clip cleanly, custom green level-select styling.
- Registrations mirror the other widgets: `register_reflect_named`/`register_clone`/
  `SerdeComponentRegistry::register` in `core_resources.rs`; add/remove closures in
  `component_registry.rs`; re-exports in `mod.rs` + `lib.rs`.
- **Additive** — no existing widget or event changed (only a new `UiEvent` variant, which is
  technically breaking for exhaustive matches but pre-1.0 licensed, and new `InputSnapshot`
  fields that are `pub(super)` internal).

## What We Tried (Chronological)

1. **Read the closest models before writing anything.** RadioGroup (`radio_group.rs` +
   `radio_group_pass.rs`) = the click-select / one-focus-stop / `selected: usize` template;
   ScrollView (`scroll_view.rs` + `scroll_view_pass.rs`) = the wheel-scroll + `scroll_offset` +
   `clamp_scroll` template. Also read focus_pass, capture, event, state (InputSnapshot),
   system.rs pass order, node.rs, the registration files, and `examples/ui_radio.rs` as the
   example template. This front-loading is why the wiring went in cleanly on the first compile.
2. **Decided the interaction model = click-select + wheel scroll.** ListBox = RadioGroup's
   click-select model (press+release ownership, drag-off cancels) PLUS ScrollView's wheel-scroll
   dimension. Not press-open (that's Dropdown). Confirmed against the `/add-ui-widget` skill's
   "decide the interaction model FIRST" step.
3. **Investigated the partial-row clipping problem.** A scrolling list always has partial rows
   at the top/bottom edges. Read `src/renderer/text/renderer.rs:256-265` (the `TextBounds`
   construction) and found: **the text scissor `top` == the text `position.y`** — they are
   coupled, so a row scrolled partly ABOVE the box (text position above `pos.y`) would spill
   upward; you cannot independently clip the scissor top vs the glyph baseline with this
   `DrawText` API. Resolution: clamp highlight rects to the visible band (no spill), and render
   a row's LABEL only when `row_top >= pos.y - 0.5` (drop the top partial row's label, keep its
   highlight); clip the bottom partial row's label via the `bounds` height. Documented in the
   pass comment + CLAUDE.md + CHANGELOG.
4. **Keyboard nav: added ↑/↓ AND kept ←/→.** The engine convention is ←/→ steps selection
   (RadioGroup/TabBar/Dropdown/Slider all do; D-pad/stick Up/Down are already consumed as
   focus-cycling in `InputSnapshot::from_world`). But a vertical list *wants* ↑/↓. Keyboard
   ArrowUp/ArrowDown were verified UNUSED by any existing pass, so I added keyboard-only
   `nav_up`/`nav_down` to `InputSnapshot` (NOT gamepad — pad Up/Down still cycle focus). Kept
   the ↑/↓ handling in a SEPARATE block from the ←/→ block so the existing Slider/Dropdown/
   Radio/Tab arms stay byte-untouched (avoided the trap where widening the shared `nav_left ||
   nav_right` gate would make a lone ArrowUp nudge a focused Slider DOWN).
5. **Factored `step_list_box` helper** in focus_pass so the ←/→ arm and the ↑/↓ block step
   identically (clamp + `scroll_to_selected` + return `Some(next)` on change). Avoided a
   `clippy::option_map_unit_fn` from using `.map(|next| push())` for side effects.
6. **Wrote component + pass + 20 tests, compiled + ran tests** (`cargo test --lib list_box`) →
   20/20 green on the first run.
7. **Wrote the example, generated the headless PNG, eyeballed it** → correct scroll + selection
   + clipping + custom styling.
8. **Ran `./scripts/verify.sh`** (background, exit read non-piped) → all 7 gates green.
9. **`/ship`** 0.116.0 → 0.117.0 (Cargo.toml + lock + CHANGELOG + CLAUDE.md header). Re-ran
   verify.sh → green again (version bump forces a crate recompile).
10. **`/land-pr` Async:** commit → push → PR #343 → `gh pr merge 343 --auto --squash` → armed.
    Background watcher polled the PR state → MERGED. Deferred wrap-up: `git pull --ff-only`,
    branch delete, memory seq bump (163).

## Key Decisions

- **click-select + wheel scroll (not press-open).** ListBox is a persistent selectable list, not
  a popup; press+release ownership matches RadioGroup/TabBar/CheckBox, drag-off cancels.
- **Keyboard ↑/↓ added as keyboard-only `nav_up`/`nav_down`, ←/→ kept for convention/pad.**
  Satisfies the natural vertical-list expectation without breaking the pad focus-cycle or the
  other widgets' ←/→ arms. New fields consumed ONLY by ListBox.
- **Partial-row rendering: clamp highlights, drop top-partial label, bottom-clip via bounds.**
  Forced by the glyphon scissor-top == text-top coupling. The selected/keyboard-stepped row is
  always fully scrolled into view (`scroll_to_selected`), so it never hits the dropped-label case.
- **`scroll_offset` is `#[serde(skip)]` transient** (mirrors ScrollView) — a loaded scene starts
  the list at the top; selection persists, scroll position does not.
- **Default look = framed** (corner_radius 6.0, border 1.0). ListBox is a NEW widget with no
  prior behavior to preserve, so unlike DrawRect's "0,0 = byte-identical fast path" ethos, a
  proper framed default is the right call for a list box.
- **Selected fill wins over hover tint** when a row is both.
- **Inserted after TabBar / before Dropdown** in the pass order. Order among click-select widgets
  is largely cosmetic (PointerCapture handles occlusion), but Dropdown draws an overlay list
  above everything so it stays last-but-tooltip.
- **Rejected: a whole-run weight/second widget.** Just filled the one list-box gap; other widget
  ideas (Button multi-line, Stepper/SpinBox) were offered to the user as alternatives but ListBox
  was chosen.

## Evidence & Data

### Version / PR

| Item | Value |
|---|---|
| Package version | 0.116.0 → **0.117.0** (MINOR — new public API) |
| CLAUDE.md header | v1.6.209 → **v1.6.210** |
| PR | **#343**, squash-merged 2026-07-07T15:59Z (async auto-merge) |
| main tip | `68953ab` |
| Memory global seq | **163** (code PR); handoff PR will be seq 164 |
| Files changed | 17 (11 modified + 3 new src + ... see Files Changed) |
| Diff | 1156 insertions, 11 deletions |

### Tests (20 new, all green)

| File | Count | Coverage |
|---|---|---|
| `src/ui/list_box.rs` | 8 | selection clamp-on-read, `row_at` through scroll offset, dead-space rejection, `clamp_scroll` bounds, `scroll_to_selected` keep-in-view, builders, serde (scroll not persisted), reflect |
| `src/ui/system/list_box_pass.rs` | 12 | click selects+emits, reselect silent, click a scrolled row → right index, covered-by-panel does not select, drag-off cancels, wheel scrolls+clamps, wheel on non-hovered no-op, ←/→ step+scroll-into-view, ↑/↓ step, bounds-clamp silent, render bg/border/one-selected-highlight, empty list no panic |

Full suite: `test result: ok. 97 passed` (lib unit slice shown) + doctests incl. the ListBox
doctest. `[verify] all checks passed ✓` twice (pre- and post-`/ship`).

### The 10-file widget wiring signature (all present)

| # | File | What |
|---|---|---|
| 1 | `src/ui/list_box.rs` (new) | component + Reflect + serde + geometry helpers + 8 tests |
| 2 | `src/ui/system/list_box_pass.rs` (new) | `pub(super) fn run(...)` + 12 tests |
| 3 | `src/ui/system.rs` | `mod list_box_pass;` + `list_box_scratch` field + `run()` call + pass-order doc |
| 4 | `src/ui/system/event.rs` | `UiEvent::ListBoxChanged(Entity, usize)` |
| 5 | `src/ui/system/focus_pass.rs` | collect + `step_list_box` + ←/→ arm + ↑/↓ block |
| 6 | `src/ui/system/capture.rs` | `extend_kind::<ListBox>(world, viewport, 0.0)` |
| 7 | `src/ui/mod.rs` + `src/lib.rs` | `pub mod list_box;` + `pub use ...ListBox` re-exports |
| 8 | `src/app/core_resources.rs` + `src/app/editor/component_registry.rs` | reflect/clone/serde + add/remove |
| 9 | `examples/ui_list_box.rs` (new) | playable demo + `HEADLESS_SHOT` self-check |
| 10 | `CLAUDE.md` UI row + `/ship` paperwork | module-map clause + version/CHANGELOG |

Plus one extra touch: `src/ui/system/state.rs` (the new `nav_up`/`nav_down` snapshot fields) and
`src/ui/system/text_input_pass.rs` (its two literal `InputSnapshot { .. }` test constructors
needed the two new fields added — E0063 caught this).

## Code Analysis

- **`ListBox::row_at(cursor, pos, node_size) -> Option<usize>`** — the single geometry source
  shared by render + click resolution. Scroll-aware: `idx = ((cursor.y - pos.y + scroll_offset)
  / row_height) as usize`, guarded by an inside-node-rect test and `idx < items.len()` (rejects
  dead space below the last row). Rows are hittable only where they lie inside the node rect (=
  the PointerCapture surface).
- **`ListBox::scroll_to_selected(view_height)`** — scrolls the minimum: up if the selected row's
  top < `scroll_offset`, down if its bottom > `scroll_offset + view_height`, else untouched; then
  `clamp_scroll`. Called on click-select AND on keyboard step so the selected row is always fully
  in view (which is why it never hits the dropped-top-partial-label case).
- **`clamp_scroll(view_height)`** — `scroll_offset.clamp(0.0, (content_height - view_height).max(0))`
  (identical shape to ScrollView). `content_height() = items.len() * row_height`.
- **Render z-layering (per row):** bg fill at `z`; row highlight (selected/hover) at
  `z + UI_SUBLAYER_Z_STEP`; label at `z + UI_SUBLAYER_Z_STEP*2`; border outline at
  `z + UI_SUBLAYER_Z_STEP*3` (frames on top). `UI_SUBLAYER_Z_STEP = 0.001` (shared const).
- **Labels carry `.with_z(z + step*2)`** (not the always-on-top text pass) — per the text
  z-ordering guardrail, so an occluding surface hides them.
- **`InputSnapshot.nav_up`/`nav_down`** set from `input.just_pressed(KeyCode::ArrowUp/Down)` in
  `from_world`; NOT folded from gamepad (D-pad/stick Up/Down remain focus-cycle in the same fn).
- **`step_list_box(lb, forward, view_height) -> Option<usize>`** (focus_pass): clamp-step +
  `scroll_to_selected`, returns `Some(next)` only on change. `forward = nav_right` for the ←/→ arm,
  `nav_down` for the ↑/↓ block.
- **`LABEL_PAD = 8.0`** — horizontal inset of a row label from the list-box edges.
- **Default style:** `bg` rgba(.10,.11,.15,1), `hover` rgba(1,1,1,.06), `selected`
  rgba(.24,.46,.78,.85), `text` rgba_u8(212,214,222,255), `border` rgba(.34,.36,.44,1),
  `row_height` 28, `font_size` 16, `corner_radius` 6, `border` 1.

## Files Changed

### Source code (new)
- `src/ui/list_box.rs` — the `ListBox` component + Reflect + geometry helpers.
- `src/ui/system/list_box_pass.rs` — wheel-scroll + click-select + render pass.
- `examples/ui_list_box.rs` — playable demo + headless self-check.

### Source code (modified)
- `src/ui/system.rs` — pass registration (mod / scratch field / call / doc-comment).
- `src/ui/system/event.rs` — `UiEvent::ListBoxChanged`.
- `src/ui/system/focus_pass.rs` — collect + `step_list_box` + ←/→ arm + ↑/↓ block.
- `src/ui/system/capture.rs` — `ListBox` pointer-opaque.
- `src/ui/system/state.rs` — `InputSnapshot.nav_up`/`nav_down`.
- `src/ui/system/text_input_pass.rs` — two test `InputSnapshot` literals gain the two new fields.
- `src/ui/mod.rs`, `src/lib.rs` — re-exports.
- `src/app/core_resources.rs`, `src/app/editor/component_registry.rs` — registrations.

### Docs / release
- `CLAUDE.md` — UI module-map row clause + header v1.6.210 / v0.117.0.
- `docs/CHANGELOG.md` — 0.117.0 entry.
- `Cargo.toml`, `Cargo.lock` — 0.117.0.

## User Feedback & Preferences

- Session opened via `/remote-control` + "마지막 핸드오프 확인하고 작업 알려줘" — the user wanted the
  state read and a task recommended, not a specific task dictated.
- When told the board was empty, the user chose **"다른 엔진 셀프픽"** (other engine self-pick) over
  waiting for the game to file EW-007 or proactively implementing it — respect the work-around-first
  pipe (don't pre-implement a board request the game hasn't filed).
- From the self-pick shortlist, the user picked **"ListBox 위젯 (추천)"** — the recommended option.
- Standing preferences (from memory, still in force): user-facing reports in **Korean**,
  agent-to-agent/code/docs in English; merge authority delegated (squash on green CI, no
  re-confirm); async auto-merge is the default landing for CI-verifiable changes; always pass an
  explicit `model` to subagents.
- The user then asked to **`/handoff` and merge** (this doc).

## Where We're Going

1. **Land THIS handoff** as its own `docs(handoff)` PR (chain `listbox-widget` seq 1), async
   auto-merge. On merge, bump memory to **seq 164** (handoff PR) pointing at the handoff merge hash.
2. **Next session: read the board FIRST** (`../dungeon-merchant/docs/engine-wishlist.md`). If a
   request is filed (EW-007+), serve it priority-order. EW-007 (FloatingText bold/rich) has a READY
   pre-design note — serve same-day per that plan.
3. **If the board is still empty → ASK the user for direction** (self-pick queue is thin). Named
   unfiled candidates: FloatingText weight/rich-markup (pre-design READY), Button multi-line, or
   more widgets (Stepper/SpinBox was the runner-up this session).
4. **Overdue hygiene: trim `engine-current-state.md`.** Its tip line now spans seq 137–163 (27
   seqs) — a full exact-match trim of the oldest tail (seq ~141 and below) into
   `[[engine-history-archive]]` is due. Deferred this session to avoid a risky ~4000-char
   single-line edit; do it as a dedicated, careful operation.

## Risks & Blockers

- None for the shipped feature (merged, green, verified).
- The memory-file trim (above) is a hygiene risk if left indefinitely — the tip line grows
  unbounded and single-line edits get riskier each seq.

## Open Questions

- None blocking. The one design judgment call (drop the top-partial row's label vs. some other
  clip strategy) is settled and documented; revisit only if a game reports it looks odd in play.

## Quick Start for Next Session

```bash
# No beads in this repo — context lives in memory + handoffs.
# Recalled memory: engine-current-state (seq 163/164), listbox-widget chain seq 1.

# Reference docs
#   CLAUDE.md (UI module-map row now has ListBox)
#   .claude/skills/add-ui-widget/SKILL.md (the 10-file pattern)

# Key files to read first (if extending ListBox or adding another widget)
#   src/ui/list_box.rs                  — the component + geometry helpers
#   src/ui/system/list_box_pass.rs      — wheel-scroll + click-select + render
#   src/ui/system/focus_pass.rs         — step_list_box + ←/→ arm + ↑/↓ block
#   src/ui/radio_group.rs               — the click-select template it followed

# Verify current state
git -C /Users/jkl/Projects/skeleton-engine log --oneline -1   # expect 68953ab (#343) or later
cargo test --lib list_box                                     # 20 tests green

# See it run
HEADLESS_SHOT=/tmp/ui_list_box.png cargo run --example ui_list_box   # or: cargo run --example ui_list_box

# Next action
#   Read ../dungeon-merchant/docs/engine-wishlist.md. If a request is filed, serve it
#   priority-order (EW-007 has a READY pre-design note). If STILL empty, ASK the user for
#   direction. Separately: the engine-current-state.md tip-line trim is overdue.
```
