# Plan — Editor Tile-Painting (skeleton-engine v8.11.0)

> Autonomous feature-loop arc `editor-tile-painting`. Additive (semver-minor), native-gated
> editor authoring feature. Acceptance test = a dedicated `tile_paint` example + native playtest.
> Status target: implement to completion, Gate6 green, PR opened, **await user merge** (never self-merge).

## Goal

In the F2 docked editor, when a `Tilemap` entity is selected, let the user paint tiles directly
in the viewport: left-click/drag paints the chosen tile value, right-click/drag erases (value 0),
number keys 1–9 pick the paint value (0 = erase), and each stroke is a single undoable history
command (Ctrl+Z). Reuses `Tilemap::cell_at_world` + `set_tile`; the existing reactive
`TilemapSystem` cell-diff renders the change next frame — no rebuild call needed.

## Why it's safe / additive

- All changes are editor-internal (`src/app/editor/**`) + one new example. No public engine API
  changes, no changes to `Tilemap`/`TilemapSystem`, so `rust-survivors` and every existing example
  are unaffected.
- New `EditorCmd::PaintTiles` variant is added to a `pub(super)` enum — not public API.
- Native-gated (`#[cfg(not(target_arch = "wasm32"))]`) to mirror the existing native gizmo path and
  keep the wasm lib+bins surface byte-identical.

## Reconnaissance (verified)

- **Selection:** `EditorState::inspector_selected: Option<Entity>` (`editor/state.rs:90`). Check
  tilemap with `self.world.get::<crate::tilemap::Tilemap>(sel).is_some()`. `Tilemap` is a plain
  component (not Reflect/registered) — fine, we access it directly.
- **Viewport→world:** `InputState::cursor()` is already translated to viewport-local coords by the
  docked cursor gate, so `Camera::screen_to_world(input.cursor())` yields the world position in both
  Overlay and Docked modes (`gizmo.rs:527`).
- **Input gate:** gizmo computes `egui_wants_mouse` (`gizmo.rs:209`); when true (pointer over a
  panel) viewport interaction is suppressed. Paint reuses the same gate.
- **Gizmo entry:** `App::update_editor_gizmo()` (`gizmo.rs:203`) runs once per frame after egui; it
  early-returns unless `inspector_selected` is `Some`. This is where the paint hook slots in,
  *before* the move/resize branch, returning early when paint mode is active so the gizmo is skipped.
- **Input API:** `InputState::mouse_just_pressed(btn)`, `is_mouse_pressed(btn)`,
  `mouse_just_released(btn)`, `cursor()`. `MouseButton::Left`/`Right`. Keys via
  `just_pressed(KeyCode::Digit1..=Digit9, Digit0)`.
- **History:** `EditorHistory::push(cmd)`, `undo(&mut self, world: &mut World, selected: &mut Option<Entity>)`,
  `redo(...)` (`editor.rs:67–205`). Undo/redo arms have `world` in scope → call
  `world.get_mut::<Tilemap>(entity)` + `set_tile`.
- **Tilemap API:** `tiles: Vec<Vec<u32>>` (0 = empty, ≥1 = atlas index+1), `tile_size`, `origin`;
  `cell_at_world(Vec2) -> Option<(row, col)>`, `set_tile(row, col, value) -> bool` (true only on real
  change, false out-of-range / no-op), `dims() -> (rows, cols)`, atlas `columns`/`rows`.
- **Reactivity:** `TilemapSystem` diffs cells every frame and spawns/despawns/updates tile sprites
  automatically. Painting via `set_tile` shows up next frame. No dirty flag.
- **Physics caveat:** colliders are NOT synced by `set_tile` (separate `sync_static_from_tilemap`).
  Painting is **visual-only** in the editor — documented in code + CHANGELOG; the demo has no physics.

## Implementation steps

### 1. `EditorState` paint fields — `src/app/editor/state.rs`
Add (native-gated), after the existing fields (~L216), and init in `EditorState::new()`:
```rust
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) paint_mode: bool,          // toggle from inspector
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) paint_value: u32,          // 1.. = atlas index+1; 0 = erase
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) paint_stroke: Vec<(usize, usize, u32, u32)>, // (row,col,old,new) this stroke
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) paint_active: bool,        // a press→release stroke in progress
```
`paint_value` defaults to `1`. Reset `paint_mode=false` when the selected entity is not a Tilemap
(enforced in the inspector body, step 4).

### 2. `EditorCmd::PaintTiles` + history arms — `src/app/editor.rs`
- Add variant to `EditorCmd` (~L18):
  ```rust
  PaintTiles { entity: Entity, changes: Vec<(usize, usize, u32, u32)> }, // (row,col,old,new)
  ```
- `undo` arm: `if let Some(tm) = world.get_mut::<Tilemap>(*entity) { for (r,c,old,_) in changes.iter().rev() { tm.set_tile(*r,*c,*old); } }`
- `redo` arm: same but `tm.set_tile(*r,*c,*new)` in forward order.
- `Tilemap` import: `use crate::tilemap::Tilemap;` (or fully-qualify).

### 3. Paint handler + gizmo hook — `src/app/editor/ui/gizmo.rs`
- In `update_editor_gizmo()`, after the `let Some(sel) = inspector_selected else return;` guard and
  the `egui_wants_mouse` computation, insert (native-gated):
  ```rust
  #[cfg(not(target_arch = "wasm32"))]
  if self.editor.paint_mode && self.world.get::<crate::tilemap::Tilemap>(sel).is_some() {
      self.update_tile_paint(sel, egui_wants_mouse);
      return; // gizmo move/resize suppressed while painting
  }
  ```
- New method `App::update_tile_paint(&mut self, entity: Entity, egui_wants_mouse: bool)`:
  - Read `InputState`: `cursor`, `mouse_just_pressed/is_mouse_pressed/mouse_just_released(Left|Right)`,
    digit keys. Read `Camera` for `screen_to_world`.
  - **Number keys** (only when `!egui_wants_mouse` or always while painting): Digit1..=Digit9 →
    `paint_value = n`; Digit0 → `paint_value = 0`. Clamp to `0..=(columns*rows)`.
  - **Stroke start** (`mouse_just_pressed(Left|Right)` && `!egui_wants_mouse`): set `paint_active=true`,
    clear `paint_stroke`, record which button (Right → erase value 0, Left → `paint_value`).
  - **Stroke continue** (`is_mouse_pressed` of the active button && `paint_active`): compute
    `world = cam.screen_to_world(cursor)`, `cell_at_world(world)`; if `set_tile(row,col,val)` returns
    true, push `(row,col,old,val)` to `paint_stroke` (capture `old` via `get_tile` *before* set).
  - **Stroke end** (`mouse_just_released` of active button, or button no longer pressed): if
    `paint_stroke` non-empty, `self.editor.cmd_history.push(EditorCmd::PaintTiles { entity, changes })`;
    reset `paint_active=false`, clear stroke.
  - Borrow discipline: collect the needed input/camera values into locals first, then take
    `world.get_mut::<Tilemap>(entity)` (mirror the "collect then get_mut" pattern in PATTERNS.md).

### 4. Inspector "Tile Paint" section — `src/app/editor/ui/docked.rs`
- In `inspector_tab_body` (the right-panel body, ~L385), after the component list, add a native-gated
  block shown **only** when `world.get::<Tilemap>(sel).is_some()`:
  - `ui.collapsing("Tile Paint", ...)` with:
    - `ui.checkbox(&mut paint_mode, "Paint mode (suppresses gizmo)")`.
    - A wrapped row of buttons: `Erase (0)` + `1..=(columns*rows)`; clicking sets `paint_value`;
      highlight the selected value (`selectable_label`).
    - A short hint label: "L-click paint · R-click erase · 1–9 pick · 0 erase · Ctrl+Z undo".
  - When the selected entity is NOT a tilemap, force `paint_mode = false` (so switching selection
    cleanly exits paint mode). Also clamp `paint_value` to the atlas tile count.
- Mirror into the overlay inspector if it shares the same body fn (recon: bodies are shared free
  functions re-exported via `ui/mod.rs`); otherwise overlay simply won't show it (acceptable — docked
  is the painting surface).

### 5. Example `tile_paint` — `examples/games/tile_paint/tile_paint.rs` (+ `Cargo.toml` entry)
- Generate a tiny 4-tile atlas PNG at runtime (2×2 grid, e.g. grass/dirt/stone/water solid colors)
  to a temp path, `load_tilemap_atlas`, mirror the dig_quest/multi_terrain runtime-atlas pattern.
- Spawn one entity with a `Tilemap` (e.g. 20×15 cells, `tile_size` 32, `origin` at a positive offset
  like (40, 40) — engine origin is TOP-LEFT), all cells 0 (empty). Add `TilemapSystem`.
- Camera framed on the grid. `DrawText` HUD: "Press F2 → click the Tilemap in the entities list →
  Tile Paint → Paint mode. L-click paint · R-click erase · 1–4 tile · Ctrl+Z undo".
- `Cargo.toml`: add `[[example]] name = "tile_paint_game" path = "examples/games/tile_paint/tile_paint.rs"`.

### 6. Docs / version
- `Cargo.toml` version → `8.11.0`; `Cargo.lock` regenerate.
- `docs/CHANGELOG.md`: new `## 8.11.0` block (Added: editor tile-painting; note visual-only/physics
  caveat).
- `CLAUDE.md`: header → v8.11.0 / bump doc version; module-map row for tile-painting under the
  editor entry; stay ≤200 lines.
- `docs/HANDOFF.md`: short entry (English).

## Completion criteria (DONE = all checked)

1. `EditorState` has `paint_mode`/`paint_value`/`paint_stroke`/`paint_active` (native-gated), init in `new()`.
2. `EditorCmd::PaintTiles` variant + working `undo`/`redo` arms (undo restores old values in reverse,
   redo re-applies new) — a multi-cell stroke is one undo step.
3. `App::update_tile_paint` paints on L-click/drag, erases on R-click/drag, only records actually-changed
   cells, and **suppresses gizmo move/resize** while paint mode is on for a tilemap.
4. Number keys 1–9 set `paint_value`, 0 sets erase; clamped to atlas tile count.
5. Inspector shows the "Tile Paint" section **only** for tilemap entities; non-tilemap selection
   force-clears `paint_mode`. Value palette selects `paint_value` with visible highlight.
6. Painting reflects immediately in the rendered tilemap (relies on `TilemapSystem` diff) — verified by
   playtest screenshot (painted cells appear; erase removes; Ctrl+Z reverts the whole stroke).
7. Physics-stale caveat documented (code doc-comment + CHANGELOG).
8. New `tile_paint` example builds and runs; `Cargo.toml` entry added.
9. **Gate6 fully green:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo build --target wasm32-unknown-unknown`, `cargo test --all-targets`,
   `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`, `cargo package --locked --allow-dirty`.
10. Native playtest performed (F2 docked → select tilemap → paint/erase/undo) with a screenshot
    confirming the painted result.
11. Version bumped to 8.11.0; CHANGELOG + CLAUDE.md + HANDOFF updated.
12. Feature branch pushed, PR opened — **left for the user to merge** (no self-merge).

## Out of scope (future)

- Texture-swatch palette (register the atlas texture with egui for real tile thumbnails) — MVP uses
  numbered buttons.
- Live collider sync while painting (would call `sync_static_from_tilemap`); editor paint is
  visual-only for now.
- Rectangle/flood-fill/bucket tools, multi-tile brushes, layer support.
- wasm editor painting (native-gated for MVP).
