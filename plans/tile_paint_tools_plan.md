# Plan — Tile Paint tools: rectangle / flood-fill / eyedropper / brush (v8.13.0)

> Chain `editor-tile-painting` seq 3. Additive, native-gated. Extends the v8.12.0 Tile Paint
> with bulk-edit tools. Validation = pure-function unit tests (cell-set computation) + native
> GUI smoke (tool/brush UI renders, eyedropper). Merge authorized by the running /loop.

## Goal

Add paint *tools* to the docked editor's Tile Paint, so large areas are editable fast:
- **Freehand** (current behaviour) + **brush size** (1 / 3 / 5 → N×N block).
- **Rectangle** — press-drag-release fills the rectangle between the two cells with the value.
- **Bucket (flood fill)** — click flood-fills the 4-connected same-value region with the value.
- **Eyedropper** — Alt+click picks the clicked cell's value into `paint_value` (works in any tool).
Right button still erases (value 0). Every gesture commits as ONE `EditorCmd::PaintTiles` (whole-area undo).

## State (`src/app/editor/state.rs`, native-gated)
- `enum PaintTool { Freehand, Rectangle, Bucket }` (Default = Freehand, Copy/PartialEq).
- `paint_tool: PaintTool`, `paint_brush: u32` (1/3/5, default 1),
  `paint_anchor: Option<(usize, usize)>` (rectangle drag start), `paint_erase: bool` (stroke erases).
- Reset `paint_anchor`/`paint_erase` alongside the existing paint-state clear in the gizmo hook.

## Pure helpers (`gizmo.rs`, native, unit-tested)
- `rect_cells(a, b) -> Vec<(usize,usize)>` — inclusive rectangle between corners.
- `brush_cells(row, col, brush, rows, cols) -> Vec<(usize,usize)>` — N×N block (half=(brush-1)/2), clamped.
- `flood_fill(tm: &Tilemap, start) -> Vec<(usize,usize)>` — 4-connected cells equal to start's value (stack DFS, `dims()`-bounded, jagged-safe via `get_tile`).

## Handler (`App::update_tile_paint`, refactor)
1. Read input incl. Alt held (`KeyCode::AltLeft|AltRight`).
2. Eyedropper: `alt_held && (left_pressed||right_pressed)` → set `paint_value` from cell, `return`.
3. Dispatch on `paint_tool`:
   - **Freehand**: current stroke logic, but paint the `brush_cells` block at the hovered cell each frame; commit on release.
   - **Rectangle**: on press set `paint_active`+`paint_anchor`+`paint_erase`; on release fill `rect_cells(anchor, end)` and commit once; clear anchor.
   - **Bucket**: on press, `flood_fill` from the clicked cell, apply value, commit once (instant, no active state).
4. `value = if erase { 0 } else { paint_value }` (right button / `paint_erase`).
5. Number keys 1–9 still set `paint_value` (kept).

## Inspector UI (`docked.rs`, in the Tile Paint section)
- A **tool row**: `selectable_label` for Freehand / Rect / Bucket (sets `paint_tool`).
- **Brush** size selector (1/3/5) shown only for Freehand.
- Hint line updated: "Alt+click eyedropper · R-erase · Ctrl+Z undo".

## Completion criteria
1. `PaintTool` enum + 4 state fields + init + reset on paint-mode/selection exit.
2. `rect_cells` / `brush_cells` / `flood_fill` implemented + **unit tests** (rect inclusive & ordering, brush clamp at edges & sizes 1/3/5, flood fill on a small grid incl. boundary & jagged).
3. `update_tile_paint` dispatches all three tools + eyedropper; each gesture = one `PaintTiles` (undo reverts whole area). Existing 6 paint tests still pass.
4. Inspector tool/brush UI; right-erase + number keys still work.
5. Gate6 green; additive; native-only.
6. Native F2 smoke: tool buttons + brush selector render and toggle (inspector clicks); eyedropper/bucket logic unit-verified (viewport drag is cursor-freeze-limited in synthetic input, as in v8.11).
7. v8.13.0; CHANGELOG + CLAUDE; merge.

## Out of scope
- Live rectangle/selection preview overlay (no drag preview yet).
- Even-sized brushes; diagonal flood fill; global replace.
