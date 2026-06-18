# UI keyboard focus navigation (v0.31.0, B2)

**Date:** 2026-06-18
**Status:** **COMPLETED — implemented, 104 ui tests pass (5 new focus), windowed-playtested, CI-equivalent gate green; shipped as v0.31.0 on branch `feat/ui-focus-nav` (PR pending push/open).**
**Chain:** `engine-hardening` seq `32` · **Parent:** `HANDOFF_engine-hardening_autotile-unify_2026-06-18.md` (seq 31)
**Origin:** B2 of the seq-25 feature list. With B1 (autotile) this finishes the requested "B1+B2".

---

## What shipped (additive)
- **`UiFocus`** resource (`src/ui/focus.rs`): `{ entity: Option<Entity> }` + `is_focused()`.
  Auto-inserted in `insert_core_resources`. Re-exported at crate root.
- **`UiSystem` focus pass** (`src/ui/system/focus_pass.rs`), run **first** (so a Tab-focused
  `TextInput` gets the frame's typed chars):
  - **Tab / Shift+Tab** cycle focus across focusable widgets — entities with a `UiNode` + one of
    `Button` / `TextInput` / `Slider` / `CheckBox`, ordered by `Entity::index()`, skipping hidden
    nodes and disabled buttons; wraps around.
  - **Click-to-focus**: a click on a focusable sets focus to it (Tab resumes from there).
  - **Enter / Space** activate the focused widget: `Button` → emit `UiEvent::ButtonClicked`;
    `CheckBox` → toggle `checked` + emit `CheckBoxToggled`. Skipped when a `TextInput` is focused
    (those keys are text there).
  - **Left / Right** nudge a focused `Slider` by 5% of its range + emit `SliderChanged`.
  - **Focus ring**: four border `DrawRect`s at `node.z + 0.5`.
  - **TextInput sync**: the focused `TextInput.focused` = true, others false.
- `InputSnapshot` gained `tab` / `shift` / `activate` (Tab, Shift held, Enter|Space).
- `UiEvent` now derives `PartialEq` (handy for tests/games).
- Example `ui_focus` (column of all four widget types).

## Verification
- **104 ui tests pass** incl. 5 new (`focus_pass::tests`): Tab cycles in entity order + wraps;
  Shift+Tab backwards; Enter activates focused button (event); Space toggles focused checkbox;
  disabled button skipped.
- **Windowed playtest:** focus ring renders + moves Play→Slider on Tab (status "focused: entity N");
  Right arrow on the focused slider raised 50→65 with `last event: SliderChanged(2, 65)`.
- Full `./scripts/verify.sh` green after the version bump.

## Notes / gotchas
- `InputState::press` is `pub(crate)` — focus_pass tests simulate `just_pressed` by inserting a
  fresh `InputState` with the key pressed each step (no public frame-clear in tests).
- Components passed to a generic spawn helper need `Send + Sync + 'static` (ECS component bound) —
  the example's `spawn_widget<C>` had to add those bounds.
- Focus pass runs FIRST in `UiSystem`; the ring uses `z+0.5` so push order doesn't matter.
- One small per-frame consideration: `collect_focusables` reuses the `focus_scratch` buffer (clear
  + extend + retain), no steady-state alloc beyond the in-place sort.

## Files changed
- New: `src/ui/focus.rs`, `src/ui/system/focus_pass.rs`, `examples/ui_focus.rs`.
- Modified: `src/ui/mod.rs` + `src/lib.rs` (export `UiFocus`), `src/ui/system.rs` (wire focus pass +
  `focus_scratch`), `src/ui/system/state.rs` (InputSnapshot tab/shift/activate),
  `src/ui/system/event.rs` (`UiEvent: PartialEq`), `src/ui/system/text_input_pass.rs` (test
  fixtures), `src/app/core_resources.rs` (insert `UiFocus`), `Cargo.toml`/`Cargo.lock`,
  `docs/CHANGELOG.md`, `CLAUDE.md`.

## Risks & Blockers
- None. Additive, tests + playtest green.

## Where to go next (seq-25 list — B1+B2 now done)
- Further wasm audio (named buses / wasm crossfade — NOT kira).
- crates.io publish (irreversible, explicit go).
- (Stretch) gamepad UI focus navigation (D-pad/stick move focus, A activates); focus-ring styling
  knobs; flat-top hex / autotile across iso+hex.

## Quick start
```bash
cd /Users/jkl/Projects/skeleton-engine
grep -m1 '^version' Cargo.toml        # 0.31.0
cargo test --lib ui                    # 104 pass
cargo run --example ui_focus           # Tab to move focus, Enter/Space activate, arrows on slider
```
