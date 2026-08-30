//! Tile-painting operations for the editor — native only.
//!
//! Pure cell-set helpers (`rect_cells`, `brush_cells`, `flood_fill`) and the
//! `App` impl block for paint tool dispatch, extracted from `gizmo.rs`.

use super::*;

// ── Tile-paint cell-set helpers (pure, tested) ───────────────────────────────

/// Inclusive rectangle of cells spanned by two corner cells (any order).
#[cfg(not(target_arch = "wasm32"))]
fn rect_cells(a: (usize, usize), b: (usize, usize)) -> Vec<(usize, usize)> {
    let (r0, r1) = (a.0.min(b.0), a.0.max(b.0));
    let (c0, c1) = (a.1.min(b.1), a.1.max(b.1));
    let mut out = Vec::with_capacity((r1 - r0 + 1) * (c1 - c0 + 1));
    for r in r0..=r1 {
        for c in c0..=c1 {
            out.push((r, c));
        }
    }
    out
}

/// N×N brush block centred on `(row, col)`, clamped to a `rows × cols` grid.
/// `brush` is the side length (1, 3, 5 …); `half = (brush - 1) / 2`.
#[cfg(not(target_arch = "wasm32"))]
fn brush_cells(
    row: usize,
    col: usize,
    brush: u32,
    rows: usize,
    cols: usize,
) -> Vec<(usize, usize)> {
    if rows == 0 || cols == 0 {
        return Vec::new();
    }
    let half = (brush.saturating_sub(1) / 2) as usize;
    let r0 = row.saturating_sub(half);
    let r1 = (row + half).min(rows - 1);
    let c0 = col.saturating_sub(half);
    let c1 = (col + half).min(cols - 1);
    let mut out = Vec::new();
    for r in r0..=r1 {
        for c in c0..=c1 {
            out.push((r, c));
        }
    }
    out
}

/// 4-connected flood fill: every cell reachable from `start` whose value equals
/// `start`'s current value. Bounded by [`Tilemap::dims`]; jagged rows are handled by
/// `get_tile` returning `None` for short rows.
#[cfg(not(target_arch = "wasm32"))]
fn flood_fill(tm: &crate::tilemap::Tilemap, start: (usize, usize)) -> Vec<(usize, usize)> {
    let (rows, cols) = tm.dims();
    let Some(target) = tm.get_tile(start.0, start.1) else {
        return Vec::new();
    };
    let mut seen = vec![vec![false; cols]; rows];
    let mut stack = vec![start];
    let mut out = Vec::new();
    while let Some((r, c)) = stack.pop() {
        if r >= rows || c >= cols || seen[r][c] {
            continue;
        }
        if tm.get_tile(r, c) != Some(target) {
            continue;
        }
        seen[r][c] = true;
        out.push((r, c));
        if r > 0 {
            stack.push((r - 1, c));
        }
        stack.push((r + 1, c));
        if c > 0 {
            stack.push((r, c - 1));
        }
        stack.push((r, c + 1));
    }
    out
}

impl App {
    // ── Tile painting (native only) ───────────────────────────────────────────

    /// Paint tiles on the selected `Tilemap` while paint mode is active.
    ///
    /// Left-click/drag paints [`EditorState::paint_value`]; right-click/drag erases
    /// (value `0`). Number keys `1..=9` set the paint value (`0` = erase), clamped to
    /// the atlas tile count. Each press→release stroke records every cell it actually
    /// changed and is committed to the editor history as a single
    /// [`EditorCmd::PaintTiles`](crate::app::editor::EditorCmd) so one Ctrl+Z reverts
    /// the whole stroke. A committed stroke also resyncs static tile colliders via
    /// [`App::sync_tilemap_colliders`], which is a no-op unless the tilemap opted in with a
    /// `TilemapColliders` component — so undo, redo and the paint itself all leave physics
    /// agreeing with the tiles. (This doc used to say the opposite, that painting was
    /// visual-only and syncing was the app's job; `commit_paint_stroke` has done it since.)
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn update_tile_paint(
        &mut self,
        sel: crate::ecs::Entity,
        egui_wants_mouse: bool,
    ) {
        use winit::event::MouseButton;
        use winit::keyboard::KeyCode;

        // ── Read input into owned locals (release the InputState borrow) ──────
        let Some((cursor, left_pressed, right_pressed, left_held, right_held, alt_held, digit)) =
            ({
                self.world
                    .resource::<crate::input::InputState>()
                    .map(|input| {
                        const DIGITS: [(KeyCode, u32); 10] = [
                            (KeyCode::Digit0, 0),
                            (KeyCode::Digit1, 1),
                            (KeyCode::Digit2, 2),
                            (KeyCode::Digit3, 3),
                            (KeyCode::Digit4, 4),
                            (KeyCode::Digit5, 5),
                            (KeyCode::Digit6, 6),
                            (KeyCode::Digit7, 7),
                            (KeyCode::Digit8, 8),
                            (KeyCode::Digit9, 9),
                        ];
                        let digit = DIGITS
                            .iter()
                            .find(|(k, _)| input.just_pressed(*k))
                            .map(|(_, v)| *v);
                        (
                            input.cursor(),
                            input.mouse_just_pressed(MouseButton::Left),
                            input.mouse_just_pressed(MouseButton::Right),
                            input.is_mouse_pressed(MouseButton::Left),
                            input.is_mouse_pressed(MouseButton::Right),
                            input.is_pressed(KeyCode::AltLeft)
                                || input.is_pressed(KeyCode::AltRight),
                            digit,
                        )
                    })
            })
        else {
            return;
        };

        // Atlas tile count bounds the selectable paint value.
        let max_value = {
            let Some(tm) = self.world.get::<crate::tilemap::Tilemap>(sel) else {
                return;
            };
            tm.atlas.columns.saturating_mul(tm.atlas.rows)
        };
        if let Some(v) = digit {
            self.editor.paint_value = v.min(max_value);
        }

        // World position under the cursor (cursor is already viewport-local).
        let world_pos = {
            let cam_default = crate::camera::Camera::default();
            let cam = self
                .world
                .resource::<crate::camera::Camera>()
                .unwrap_or(&cam_default);
            cam.screen_to_world(cursor)
        };

        // ── Eyedropper: Alt+click picks the hovered cell's value (any tool) ───
        if alt_held && (left_pressed || right_pressed) && !egui_wants_mouse {
            if let Some(tm) = self.world.get::<crate::tilemap::Tilemap>(sel) {
                if let Some((row, col)) = tm.cell_at_world(world_pos) {
                    if let Some(v) = tm.get_tile(row, col) {
                        self.editor.paint_value = v;
                    }
                }
            }
            return;
        }

        // A button counts as "active" this frame if it is held OR was just pressed
        // (a fast click can deliver press *and* release within a single update).
        let left_active = left_held || left_pressed;
        let right_active = right_held || right_pressed;

        match self.editor.paint_tool {
            crate::app::editor::PaintTool::Freehand => self.tile_paint_freehand(
                sel,
                world_pos,
                egui_wants_mouse,
                left_active,
                right_active,
            ),
            crate::app::editor::PaintTool::Rectangle => self.tile_paint_rectangle(
                sel,
                world_pos,
                egui_wants_mouse,
                left_pressed,
                right_pressed,
                left_active,
                right_active,
            ),
            crate::app::editor::PaintTool::Bucket => self.tile_paint_bucket(
                sel,
                world_pos,
                egui_wants_mouse,
                left_pressed,
                right_pressed,
            ),
        }
    }

    /// Freehand brush: paint the N×N block under the cursor each frame; commit on release.
    #[cfg(not(target_arch = "wasm32"))]
    fn tile_paint_freehand(
        &mut self,
        sel: crate::ecs::Entity,
        world_pos: glam::Vec2,
        egui_wants_mouse: bool,
        left_active: bool,
        right_active: bool,
    ) {
        if (left_active || right_active) && !egui_wants_mouse && !self.editor.paint_active {
            self.editor.paint_active = true;
            self.editor.paint_stroke.clear();
        }
        if self.editor.paint_active && (left_active || right_active) && !egui_wants_mouse {
            let value = if right_active {
                0
            } else {
                self.editor.paint_value
            };
            let brush = self.editor.paint_brush;
            let cells = {
                let Some(tm) = self.world.get::<crate::tilemap::Tilemap>(sel) else {
                    return;
                };
                let (rows, cols) = tm.dims();
                match tm.cell_at_world(world_pos) {
                    Some((r, c)) => brush_cells(r, c, brush, rows, cols),
                    None => Vec::new(),
                }
            };
            self.apply_paint_cells(sel, &cells, value);
        }
        if self.editor.paint_active && !left_active && !right_active {
            self.commit_paint_stroke(sel);
            self.editor.paint_active = false;
        }
    }

    /// Rectangle tool: anchor on press, fill `anchor..release` on release.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    fn tile_paint_rectangle(
        &mut self,
        sel: crate::ecs::Entity,
        world_pos: glam::Vec2,
        egui_wants_mouse: bool,
        left_pressed: bool,
        right_pressed: bool,
        left_active: bool,
        right_active: bool,
    ) {
        if (left_pressed || right_pressed) && !egui_wants_mouse && !self.editor.paint_active {
            self.editor.paint_active = true;
            self.editor.paint_erase = right_pressed && !left_pressed;
            self.editor.paint_stroke.clear();
            self.editor.paint_anchor = self
                .world
                .get::<crate::tilemap::Tilemap>(sel)
                .and_then(|tm| tm.cell_at_world(world_pos));
        }
        if self.editor.paint_active && !left_active && !right_active {
            let value = if self.editor.paint_erase {
                0
            } else {
                self.editor.paint_value
            };
            let end = self
                .world
                .get::<crate::tilemap::Tilemap>(sel)
                .and_then(|tm| tm.cell_at_world(world_pos));
            if let (Some(a), Some(b)) = (self.editor.paint_anchor, end) {
                let cells = rect_cells(a, b);
                self.apply_paint_cells(sel, &cells, value);
            }
            self.commit_paint_stroke(sel);
            self.editor.paint_active = false;
            self.editor.paint_anchor = None;
        }
    }

    /// Bucket tool: a click flood-fills the 4-connected same-value region.
    #[cfg(not(target_arch = "wasm32"))]
    fn tile_paint_bucket(
        &mut self,
        sel: crate::ecs::Entity,
        world_pos: glam::Vec2,
        egui_wants_mouse: bool,
        left_pressed: bool,
        right_pressed: bool,
    ) {
        if !(left_pressed || right_pressed) || egui_wants_mouse {
            return;
        }
        let value = if right_pressed && !left_pressed {
            0
        } else {
            self.editor.paint_value
        };
        let cells = {
            let Some(tm) = self.world.get::<crate::tilemap::Tilemap>(sel) else {
                return;
            };
            match tm.cell_at_world(world_pos) {
                Some(start) => flood_fill(tm, start),
                None => Vec::new(),
            }
        };
        self.editor.paint_stroke.clear();
        self.apply_paint_cells(sel, &cells, value);
        self.commit_paint_stroke(sel);
    }

    /// Apply `value` to each `(row, col)` cell, recording changed cells into the stroke.
    #[cfg(not(target_arch = "wasm32"))]
    fn apply_paint_cells(&mut self, sel: crate::ecs::Entity, cells: &[(usize, usize)], value: u32) {
        let Some(tm) = self.world.get_mut::<crate::tilemap::Tilemap>(sel) else {
            return;
        };
        for &(row, col) in cells {
            let old = tm.get_tile(row, col).unwrap_or(0);
            if tm.set_tile(row, col, value) {
                self.editor.paint_stroke.push((row, col, old, value));
            }
        }
    }

    /// Commit the accumulated stroke as one undoable `PaintTiles` command (if non-empty).
    #[cfg(not(target_arch = "wasm32"))]
    fn commit_paint_stroke(&mut self, sel: crate::ecs::Entity) {
        let changes = std::mem::take(&mut self.editor.paint_stroke);
        if !changes.is_empty() {
            self.editor
                .cmd_history
                .push(crate::app::editor::EditorCmd::PaintTiles {
                    entity: sel,
                    changes,
                });
            // Keep static tile colliders in sync if this tilemap opted in via TilemapColliders.
            self.sync_tilemap_colliders(sel);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── tile painting ───────────────────────────────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    fn setup_paint_app() -> (crate::app::App, crate::ecs::Entity) {
        use crate::tilemap::{Tilemap, TilemapAtlas};
        let mut app = crate::app::App::new();
        // Identity screen→world: position (0,0), zoom 1.
        app.world
            .insert_resource(crate::camera::Camera::new(glam::Vec2::ZERO, 1.0));
        // 4×4 grid, tile_size 10, origin (0,0); atlas 2×2 ⇒ max paint value 4.
        let tiles = vec![vec![0u32; 4]; 4];
        let tm = Tilemap::new(
            TilemapAtlas::new("test_atlas", 2, 2),
            tiles,
            10.0,
            glam::Vec2::ZERO,
        );
        let e = app.world.spawn();
        app.world.add_component(e, tm);
        app.editor.inspector_selected = Some(e);
        app.editor.paint_mode = true;
        (app, e)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn tile(app: &crate::app::App, e: crate::ecs::Entity, row: usize, col: usize) -> u32 {
        app.world
            .get::<crate::tilemap::Tilemap>(e)
            .unwrap()
            .get_tile(row, col)
            .unwrap()
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn cursor(app: &mut crate::app::App, x: f32, y: f32) {
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .set_cursor(glam::Vec2::new(x, y));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn press(app: &mut crate::app::App, btn: winit::event::MouseButton) {
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .press_mouse(btn);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn release(app: &mut crate::app::App, btn: winit::event::MouseButton) {
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .release_mouse(btn);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn flush(app: &mut crate::app::App) {
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .flush();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_paint_left_click_paints_then_undo_redo() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_value = 2;

        // Cell (row 1, col 2) center = (2*10+5, 1*10+5) = (25, 15).
        cursor(&mut app, 25.0, 15.0);
        press(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false); // press frame → paints
        flush(&mut app);
        release(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false); // release frame → commits stroke

        assert_eq!(tile(&app, e, 1, 2), 2, "cell painted");

        // One undo reverts the whole stroke.
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 1, 2), 0, "undo cleared cell");
        // Redo re-applies it.
        app.editor.cmd_history.redo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 1, 2), 2, "redo restored cell");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_paint_drag_is_one_undo_step() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_value = 1;

        // Press over cell (0,0) center (5,5), drag to cell (0,1) center (15,5), release.
        cursor(&mut app, 5.0, 5.0);
        press(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false);
        flush(&mut app);
        cursor(&mut app, 15.0, 5.0);
        app.update_tile_paint(e, false); // still held → paints second cell
        flush(&mut app);
        release(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false); // commit

        assert_eq!(tile(&app, e, 0, 0), 1, "first cell painted");
        assert_eq!(tile(&app, e, 0, 1), 1, "second cell painted");

        // A single undo reverts BOTH cells (one stroke = one command).
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 0, 0), 0, "undo cleared first cell");
        assert_eq!(tile(&app, e, 0, 1), 0, "undo cleared second cell");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_paint_right_click_erases() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        // Pre-fill cell (0,0) with tile value 3.
        app.world
            .get_mut::<crate::tilemap::Tilemap>(e)
            .unwrap()
            .set_tile(0, 0, 3);
        app.editor.paint_value = 2; // ignored by right-click (erase = 0)

        cursor(&mut app, 5.0, 5.0);
        press(&mut app, MouseButton::Right);
        app.update_tile_paint(e, false);
        flush(&mut app);
        release(&mut app, MouseButton::Right);
        app.update_tile_paint(e, false);

        assert_eq!(tile(&app, e, 0, 0), 0, "right-click erased the cell");

        // Undo restores the erased value.
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 0, 0), 3, "undo restored erased value");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_paint_digit_keys_select_value_clamped() {
        use winit::keyboard::KeyCode;
        let (mut app, e) = setup_paint_app();

        // Atlas has 4 tiles → Digit5 clamps to 4.
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .press(KeyCode::Digit5);
        app.update_tile_paint(e, false);
        assert_eq!(
            app.editor.paint_value, 4,
            "value clamped to atlas tile count"
        );

        flush(&mut app);
        // Digit0 selects erase.
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .press(KeyCode::Digit0);
        app.update_tile_paint(e, false);
        assert_eq!(app.editor.paint_value, 0, "Digit0 selects erase");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_paint_blocked_when_egui_wants_mouse() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_value = 2;

        cursor(&mut app, 25.0, 15.0);
        press(&mut app, MouseButton::Left);
        // egui_wants_mouse = true → click is over a panel, must NOT paint.
        app.update_tile_paint(e, true);
        assert_eq!(
            tile(&app, e, 1, 2),
            0,
            "no paint while pointer is over egui"
        );
        assert!(!app.editor.paint_active, "stroke not started over egui");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_paint_same_frame_press_release_still_paints() {
        // A fast click can deliver press AND release before a single update runs
        // (common while the docked editor is paused): `is_mouse_pressed` is already
        // false but `mouse_just_pressed` is true. The cell must still paint.
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_value = 3;

        cursor(&mut app, 25.0, 15.0); // cell (1, 2)
        press(&mut app, MouseButton::Left);
        release(&mut app, MouseButton::Left); // same frame, before any update/flush
        app.update_tile_paint(e, false);
        assert_eq!(tile(&app, e, 1, 2), 3, "same-frame click painted the cell");

        // Next frame (buttons clear) commits the stroke so it is undoable.
        flush(&mut app);
        app.update_tile_paint(e, false);
        assert!(!app.editor.paint_active, "stroke committed next frame");
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 1, 2), 0, "committed stroke is undoable");
    }

    // ─── paint tools: pure cell-set helpers ──────────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rect_cells_inclusive_any_corner_order() {
        let mut cells = rect_cells((3, 1), (1, 2)); // rows 1..=3, cols 1..=2
        cells.sort_unstable();
        assert_eq!(cells, vec![(1, 1), (1, 2), (2, 1), (2, 2), (3, 1), (3, 2)]);
        assert_eq!(rect_cells((0, 0), (0, 0)), vec![(0, 0)]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn brush_cells_sizes_and_clamp() {
        assert_eq!(brush_cells(2, 2, 1, 5, 5), vec![(2, 2)]); // 1×1
        assert_eq!(brush_cells(2, 2, 3, 5, 5).len(), 9); // full 3×3
                                                         // 3×3 at the top-left corner clamps to a 2×2.
        let mut corner = brush_cells(0, 0, 3, 5, 5);
        corner.sort_unstable();
        assert_eq!(corner, vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
        assert_eq!(brush_cells(2, 2, 5, 5, 5).len(), 25); // full 5×5
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn flood_fill_4_connected_stops_at_other_values() {
        use crate::tilemap::{Tilemap, TilemapAtlas};
        let tiles = vec![
            vec![1, 1, 0], //
            vec![1, 0, 0],
            vec![0, 0, 2],
        ];
        let tm = Tilemap::new(TilemapAtlas::new("x", 2, 2), tiles, 10.0, glam::Vec2::ZERO);
        let mut region = flood_fill(&tm, (0, 0)); // the 1-region
        region.sort_unstable();
        assert_eq!(region, vec![(0, 0), (0, 1), (1, 0)]);
        // A single isolated cell.
        assert_eq!(flood_fill(&tm, (2, 2)), vec![(2, 2)]);
    }

    // ─── paint tools: behaviour through update_tile_paint ────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_bucket_fills_region_one_undo() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_tool = crate::app::editor::PaintTool::Bucket;
        app.editor.paint_value = 2;

        cursor(&mut app, 15.0, 15.0); // cell (1,1); whole 4×4 grid is value 0
        press(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false); // bucket fills on press

        assert_eq!(tile(&app, e, 0, 0), 2);
        assert_eq!(tile(&app, e, 3, 3), 2);
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 0, 0), 0, "one undo reverts the whole fill");
        assert_eq!(tile(&app, e, 3, 3), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_rectangle_fills_area_one_undo() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_tool = crate::app::editor::PaintTool::Rectangle;
        app.editor.paint_value = 3;

        cursor(&mut app, 5.0, 5.0); // cell (0,0)
        press(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false); // anchor set
        flush(&mut app);
        cursor(&mut app, 25.0, 25.0); // drag to cell (2,2)
        release(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false); // fill rect (0,0)..(2,2)

        assert_eq!(tile(&app, e, 0, 0), 3);
        assert_eq!(tile(&app, e, 2, 2), 3);
        assert_eq!(tile(&app, e, 3, 3), 0, "outside the rectangle is untouched");
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(tile(&app, e, 0, 0), 0, "one undo reverts the rectangle");
        assert_eq!(tile(&app, e, 2, 2), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_brush_paints_block() {
        use winit::event::MouseButton;
        let (mut app, e) = setup_paint_app();
        app.editor.paint_tool = crate::app::editor::PaintTool::Freehand;
        app.editor.paint_brush = 3;
        app.editor.paint_value = 2;

        cursor(&mut app, 15.0, 15.0); // cell (1,1) → 3×3 block (0,0)..(2,2)
        press(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false);
        assert_eq!(tile(&app, e, 0, 0), 2);
        assert_eq!(tile(&app, e, 2, 2), 2);
        assert_eq!(tile(&app, e, 3, 3), 0, "outside the brush is untouched");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn tile_eyedropper_alt_click_picks_value() {
        use winit::event::MouseButton;
        use winit::keyboard::KeyCode;
        let (mut app, e) = setup_paint_app();
        app.world
            .get_mut::<crate::tilemap::Tilemap>(e)
            .unwrap()
            .set_tile(1, 1, 3);
        app.editor.paint_value = 1;

        cursor(&mut app, 15.0, 15.0); // cell (1,1) == value 3
        app.world
            .resource_mut::<crate::input::InputState>()
            .unwrap()
            .press(KeyCode::AltLeft);
        press(&mut app, MouseButton::Left);
        app.update_tile_paint(e, false);

        assert_eq!(
            app.editor.paint_value, 3,
            "eyedropper picked the cell value"
        );
        assert_eq!(tile(&app, e, 1, 1), 3, "eyedropper did not paint");
    }
}
