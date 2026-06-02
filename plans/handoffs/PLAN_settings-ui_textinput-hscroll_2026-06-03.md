# Finish the settings-ui deferred polish: real OS fullscreen + overlay caret

**Date:** 2026-06-03
**Status:** PLANNED
**Bead(s):** none (no beads system in this repo)
**Epic:** skeleton-engine playable-examples dogfooding loop
**Chain:** `settings-ui` seq `4`
**Context:** See `HANDOFF_settings-ui_textinput-hscroll_2026-06-03.md` for session data, the render path, prior approaches, and the "Pointers for the Next Phases" section.

---

## Problem Statement

The settings-ui dogfooding cluster has two deferred items left after v1.3.0 shipped TextInput
horizontal scroll + IME honesty. (1) **Real OS fullscreen**: the Settings checkbox only stores a
`Settings.fullscreen: bool` preference — toggling it does nothing to the window; locale strings even
say "(preference)". The engine exposes no window-control API. (2) **Overlay caret**: the TextInput
caret is an inline `|`/space slot inserted into the rendered string, so it shifts trailing glyphs
sub-pixel on every blink and can't be styled. Both are well-characterized; see the handoff's "Code
Analysis", "glyphon render-path facts", and "Pointers for the Next Phases".

## Key Findings (from the handoff)

- The engine already has a **game→app request resource** pattern: `PendingResize` (a `World` resource
  the game sets, read in the loop, applied via `window.request_inner_size`, then cleared). Fullscreen
  should mirror it. → drives Phase 1.
- The example owns all fullscreen state today; `fullscreen_cb` is spawned in `SettingsScene::on_enter`
  (~line 602) and only updates `Settings.fullscreen`. → drives Phase 1.
- `caret_x(buf, caret_byte)` was added this session (`src/renderer/text.rs`) and already measures the
  caret's pen-x via `Buffer::layout_runs()`. The single-line scroll already computes the field's
  `scroll`. → drives Phase 2 (the caret screen-x is `position.x + caret_x - scroll`).
- There is **no quad pipeline inside glyphon/`text.rs`**; UI rectangles render via a separate
  `DrawRect`/`UiQueue` path. The overlay caret should be a thin `DrawRect`, not a glyphon glyph. → drives Phase 2.
- `UiSystem` cannot measure text (no `FontSystem`); only the renderer can. This is the central
  constraint for where the caret rect is produced. → drives Phase 2's design choice.

## Anti-Goals (What NOT To Do)

- **Don't let the example touch the winit `Window` directly** — it has no handle; route through an
  engine resource (the `PendingResize` pattern). (Handoff: Pointers.)
- **Don't add a new GPU pipeline for the caret** — reuse the existing `DrawRect` path; glyphon has no
  quad path. (Handoff: glyphon render-path facts.)
- **Don't change present mode / `frame_latency`** (settled in v1.2.1) and **don't re-touch wrapping**
  for labels/buttons (`single_line_caret = None` keeps them as-is).
- **Don't measure the caret in `UiSystem`** — no `FontSystem` there; measurement must stay in `text.rs`.
- **Don't bundle the `.app` packager into this plan** — it's a separate track (see Dependencies).

## Plan

### Phase 1: Real OS fullscreen

**Goal:** Toggling the Settings "Fullscreen" checkbox actually enters/exits OS fullscreen.

**Why this approach:** The `PendingResize` resource already proves the game→app→`Window` request
pattern; fullscreen is the same shape (game sets a request, the loop applies it on the real `Window`).
Minimal, fork-friendly, and the example stays decoupled from winit.

- Add a request resource in `src/resources.rs`, e.g. `pub struct PendingFullscreen(pub Option<bool>)`
  (mirror `PendingResize`): `Some(true)` → enter, `Some(false)` → exit, consumed to `None`.
- Register/insert it like `PendingResize` (default inserted in `App` setup) and re-export from `src/lib.rs`.
- In `src/app.rs`, in `step_frame` right after the `PendingResize` block, read `PendingFullscreen`; if
  `Some(on)`, call `window.set_fullscreen(on.then(|| winit::window::Fullscreen::Borderless(None)))`,
  then reset to `None`. Native only is fine; harmless on wasm (gate if winit wasm lacks it).
- Example (`settings_menu_game`): in `SettingsSystem`, on `UiEvent::Toggled(e)` (or the checkbox's
  changed path) for `fullscreen_cb`, set `Settings.fullscreen` **and** insert/set `PendingFullscreen(Some(new))`.
- Update the 3 locale strings (`opt.fullscreen` EN/KO/ES) to drop "(preference)" / "(설정값)" / "(preferencia)".
- Add a unit test for the resource (default `None`; set/consume), since fullscreen itself is manual.

**Files:** `src/resources.rs` (new resource), `src/app.rs` (apply in `step_frame`), `src/lib.rs`
(re-export), `examples/games/settings_menu/settings_menu.rs` (toggle wiring + locale RON), `docs/CHANGELOG.md`, `docs/NEXT_WORK.md`.
**Validates with:** `cargo test --lib` (+1 resource test); manual macOS — toggle checkbox enters
borderless fullscreen and exits cleanly, window content keeps rendering (reuses `step_frame`); `cargo fmt`,
native+wasm `clippy -D warnings`, wasm build, `rust-survivors` check all green.
**Rollback:** Revert the resource + the `step_frame` apply + example wiring; locale strings revert to "(preference)".

### Phase 2: Overlay caret (stable, non-shifting)

**Goal:** Draw the TextInput caret as a thin quad at the measured glyph x so it never shifts the text
on blink; remove the inline `|`/space caret from the rendered string.

**Why this approach:** `caret_x` + the single-line `scroll` already give the exact caret screen-x in
`text.rs`; a thin `DrawRect` there avoids the inline-caret glyph shimmer. UiSystem can't measure, so
the rect must originate where the measurement is (the renderer), or via a one-frame feedback channel.

- **Design checkpoint first (small spike):** choose between
  - **(A) Renderer emits the caret rect (recommended):** `text.rs`, when single-line + caret visible,
    pushes a thin caret `DrawRect` (≈2px wide, field height) at `position.x + caret_x - scroll` into
    the rect queue/pass it already renders alongside. No frame lag.
  - **(B) Feedback channel (fallback):** renderer writes caret screen-x into a resource keyed by entity;
    `UiSystem` draws the `DrawRect` next frame. Simpler ownership, but 1-frame lag on fast caret moves.
  Pick (A) unless the rect pass ordering makes it awkward; record the choice in the handoff/PR.
- Add a `caret_visible: bool` (blink state) to the single-line render info (extend `single_line_caret`
  to carry blink, or add a sibling field) so the renderer knows when to draw the bar.
- Change `display_with_caret` (`src/ui/text_input.rs`) to **not** insert the `|`/space — the rendered
  string becomes `head + preedit + tail`; `caret_display_offset()` (already `cursor + preedit.len()`)
  stays the measurement anchor. Keep blink timing in `UiSystem` (drives `caret_visible`).
- Caret rect: clip to the field `TextBounds` so it disappears at the edges like the text; color from
  `TextInput.text_color` or a fixed caret color.
- Update the `display_with_caret` unit tests (they assert the inline `|`/space — rewrite to assert the
  caretless display string + that `caret_display_offset` is unchanged).

**Files:** `src/renderer/text.rs` (caret rect emission + blink field), `src/ui/text_input.rs`
(`display_with_caret` drops inline caret; tests), `src/ui/system.rs` (pass blink state), `docs/CHANGELOG.md`.
**Validates with:** `cargo test --lib` (rewritten caret tests pass); manual macOS — caret is a stable
bar that blinks without shifting text, follows scroll, clips at edges, sits correctly with IME preedit;
fmt/clippy(native+wasm)/wasm build/rust-survivors green.
**Rollback:** Restore the inline `|`/space `display_with_caret` (revert `text.rs` caret rect + the
blink field); the v1.3.0 behavior returns.

## Dependencies & Order

- Phase 1 and Phase 2 are **independent** (different subsystems: window control vs text render) and
  could be done in either order or in parallel worktrees. Recommend Phase 1 first (smaller, builds the
  window-control resource that may be reused later).
- The **macOS `.app` bundler** is a separate track (not a phase here): `cargo-bundle` or
  `cargo-packager` + `[package.metadata.bundle]` + `Info.plist`. Schedule as its own plan if desired.

## Risks & Mitigations

- **Phase 2 design (A vs B):** rect-pass ordering in `text.rs` may not cleanly allow emitting a rect.
  *Likely: low–medium.* Mitigation: the spike resolves it before coding; (B) is a working fallback.
- **Caret + IME preedit interaction:** the caret anchor (`caret_display_offset`) sits after the preedit;
  verify the bar lands after the composing text. *Mitigation:* covered by the manual IME test (reuse the
  v1.3.0 long-text field).
- **Fullscreen on wasm:** `set_fullscreen` semantics differ; *Mitigation:* gate the apply to native, leave
  the preference-only behavior on wasm.
- **CI process (from last session):** always `cargo fmt` after edits and run `cargo test --all-targets`
  before pushing — see the handoff's "Pre-push checklist". *Mitigation:* follow it to avoid CI round-trips.

## Success Criteria

- **Minimum viable:** Phase 1 done — checkbox toggles real fullscreen, content keeps rendering, tests +
  CI green, released (likely v1.3.1 or v1.4.0 depending on API surface added).
- **Full success:** Phase 2 also done — caret is a stable non-shifting bar; the inline caret slot is gone;
  `display_with_caret` tests rewritten and green; settings_menu demonstrates both; deferred ledger in
  `docs/NEXT_WORK.md` shows only the `.app` bundler + fresh dogfooding candidates remaining.
- Both phases additive/non-breaking to existing examples and `rust-survivors`.

## Quick Start

```bash
# Restore full context
cat plans/handoffs/HANDOFF_settings-ui_textinput-hscroll_2026-06-03.md

# Key source files for Phase 1
#   src/app.rs            (step_frame, PendingResize apply block — mirror it)
#   src/resources.rs      (PendingResize def — add PendingFullscreen alongside)
#   src/lib.rs            (re-export list)
#   examples/games/settings_menu/settings_menu.rs  (fullscreen_cb ~602, SettingsSystem, locale RON)

# Verify starting state
git switch main && git pull        # expect a2eb352 or later
cargo test --lib                   # 262 passed
cargo run --example settings_menu_game   # confirm current (preference-only) checkbox

# First concrete action (Phase 1)
#   Add `pub struct PendingFullscreen(pub Option<bool>);` to src/resources.rs (mirror PendingResize),
#   insert it as a default resource, and apply it in src/app.rs::step_frame via window.set_fullscreen.
```
