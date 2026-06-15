# Plan — Editor grid overlay + coordinate readout (v8.15.0)

> Editor feature E of the A–G loop. Additive, native-gated. A self-contained egui overlay drawn on
> the docked central viewport — does NOT touch the camera or game systems (no conflicts). Validation =
> unit test for the line-math + native F2 playtest (display is awake again).

## Goal
A toggleable **grid overlay** in the docked editor viewport, world-aligned to the editor snap size,
plus a live **cursor world-coordinate readout**. Helps align tile painting / entity placement.

## Approach (overlay, not a render change)
The central panel shows the game RT as `ui.image((tex, avail))` (docked.rs:137-139). Draw the grid on
top with the egui painter (`ui.painter_at(img_rect)`), mapping world↔screen via the `Camera` resource:
- `Camera { position: Vec2 (viewport top-left in world), zoom: f32 }`, `screen_to_world` / `world_to_screen`.
- The RT is rendered at the central-panel size, shown 1:1, so image-point = `rect.min + world_to_screen(world)`.
- Visible world range: `screen_to_world((0,0))` … `screen_to_world(rect.size)`.

## State (`state.rs`, native-gated)
- `show_grid: bool` (default `false`).

## UI
- **Toolbar** (`docked_toolbar`): a `ui.checkbox(&mut show_grid, "Grid")`.
- **Central panel** (`update_docked_ui`): capture the image `Response.rect`; if `show_grid`, call
  `draw_editor_grid(ui, app, rect)`.

## `draw_editor_grid(ui, app, rect)` (native free fn)
- `cam = world.resource::<Camera>()` (default fallback); `spacing = editor.snap_size.max(1.0)`.
- `screen_spacing = spacing * cam.zoom`; skip drawing lines if `< 4.0` px (too dense when zoomed out).
- Vertical lines: for each world-x in `grid_lines_in_range(tl.x, br.x, spacing)`, screen x =
  `rect.min.x + world_to_screen(vec2(wx, 0)).x`; line top→bottom. Horizontal lines: symmetric in y.
- Faint stroke (`Color32::from_white_alpha(~28)`).
- Readout: if `ui.ctx().pointer_hover_pos()` is inside `rect`, world = `screen_to_world(p - rect.min)`;
  paint `"x {:.0}  y {:.0}"` (+ hovered cell `(row,col)` when a `Tilemap` is selected) at the rect's top-left.

## Pure helper (`gizmo.rs` or `docked.rs`, unit-tested)
- `grid_lines_in_range(start, end, spacing) -> Vec<f32>` — world coords of grid lines in `[start, end]`
  (`first = ceil(start/spacing)*spacing`, step `spacing`, guard-capped). Empty if `spacing<=0` or `end<=start`.

## Completion criteria
1. `show_grid` state + init; toolbar checkbox.
2. `draw_editor_grid` + `grid_lines_in_range`; overlay drawn only when `show_grid`.
3. Unit test for `grid_lines_in_range` (alignment to multiples, range bounds, degenerate inputs).
4. Gate6 green; additive; native-only.
5. Native F2 playtest: toggle Grid → world-aligned lines appear at snap spacing; cursor readout
   updates; lines stay aligned (eyeball). Screenshot.
6. v8.15.0; CHANGELOG + CLAUDE; merge.

## Out of scope (later E2)
- Camera pan/zoom + "Frame Selected" (F) — needs care vs. the game camera system; separate feature.
- Rulers, configurable grid spacing/colour (reuses `snap_size` for now).
