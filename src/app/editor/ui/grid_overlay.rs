//! World-aligned grid overlay for the docked editor viewport — native only.
//!
//! Pure egui painting on top of the game image: a grid at the editor snap spacing plus a
//! cursor world-coordinate readout (and the hovered cell when a `Tilemap` is selected). It
//! does not touch the camera or game systems. Extracted verbatim from `docked.rs`;
//! `draw_editor_grid` is called from `update_docked_ui` in `docked.rs`.

#![cfg(not(target_arch = "wasm32"))]

use crate::app::editor::theme;
use crate::app::editor::tr;
use crate::app::App;

/// World coordinates of grid lines within `[start, end]` at `spacing` intervals
/// (aligned to multiples of `spacing`). Empty for degenerate inputs.
fn grid_lines_in_range(start: f32, end: f32, spacing: f32) -> Vec<f32> {
    if spacing <= 0.0 || end <= start {
        return Vec::new();
    }
    let first = (start / spacing).ceil() * spacing;
    let mut out = Vec::new();
    let mut x = first;
    let mut guard = 0;
    // Guard caps the count so a tiny spacing / huge range can't spin forever.
    while x <= end && guard < 100_000 {
        out.push(x);
        x += spacing;
        guard += 1;
    }
    out
}

/// Draw the world-aligned grid overlay + cursor coordinate readout on the docked viewport.
/// Pure egui painting on top of the game image — it does not touch the camera or game systems.
pub(in crate::app) fn draw_editor_grid(ui: &egui::Ui, app: &App, rect: egui::Rect) {
    if rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }
    let cam_default = crate::camera::Camera::default();
    let cam = app
        .world
        .resource::<crate::camera::Camera>()
        .unwrap_or(&cam_default);
    let spacing = app.editor.snap_size.max(1.0);
    let painter = ui.painter_at(rect);
    let stroke = egui::Stroke::new(
        theme::GRID_LINE_WIDTH,
        egui::Color32::from_white_alpha(theme::GRID_LINE_ALPHA),
    );

    // Visible world range (top-left → bottom-right of the image rect).
    let tl = cam.screen_to_world(glam::Vec2::ZERO);
    let br = cam.screen_to_world(glam::Vec2::new(rect.width(), rect.height()));

    // Only draw when grid cells are at least a few pixels apart on screen.
    if spacing * cam.zoom >= 4.0 {
        for wx in grid_lines_in_range(tl.x, br.x, spacing) {
            let sx = rect.min.x + cam.world_to_screen(glam::Vec2::new(wx, tl.y)).x;
            painter.line_segment(
                [egui::pos2(sx, rect.top()), egui::pos2(sx, rect.bottom())],
                stroke,
            );
        }
        for wy in grid_lines_in_range(tl.y, br.y, spacing) {
            let sy = rect.min.y + cam.world_to_screen(glam::Vec2::new(tl.x, wy)).y;
            painter.line_segment(
                [egui::pos2(rect.left(), sy), egui::pos2(rect.right(), sy)],
                stroke,
            );
        }
    }

    // Cursor world-coordinate readout (and hovered cell if a Tilemap is selected).
    if let Some(p) = ui.ctx().pointer_hover_pos() {
        if rect.contains(p) {
            let world = cam.screen_to_world(glam::Vec2::new(p.x - rect.min.x, p.y - rect.min.y));
            let mut text = format!("x {:.0}  y {:.0}", world.x, world.y);
            if let Some(sel) = app.editor.inspector_selected {
                if let Some(tm) = app.world.get::<crate::tilemap::Tilemap>(sel) {
                    if let Some((row, col)) = tm.cell_at_world(world) {
                        text.push_str(&format!("  {} ({row}, {col})", tr("cell", "셀")));
                    }
                }
            }
            painter.text(
                rect.left_top() + egui::vec2(6.0, 6.0),
                egui::Align2::LEFT_TOP,
                text,
                egui::FontId::monospace(theme::CURSOR_READOUT_FONT_SIZE),
                egui::Color32::from_white_alpha(theme::CURSOR_READOUT_ALPHA),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::grid_lines_in_range;

    #[test]
    fn grid_lines_align_to_multiples_within_range() {
        assert_eq!(
            grid_lines_in_range(0.0, 50.0, 16.0),
            vec![0.0, 16.0, 32.0, 48.0]
        );
        // Start not on a multiple → first line snaps up to the next multiple.
        assert_eq!(grid_lines_in_range(5.0, 50.0, 16.0), vec![16.0, 32.0, 48.0]);
        // Negative range works (camera can show negative world coords).
        assert_eq!(
            grid_lines_in_range(-10.0, 10.0, 5.0),
            vec![-10.0, -5.0, 0.0, 5.0, 10.0]
        );
        // Degenerate inputs → empty.
        assert!(grid_lines_in_range(0.0, 50.0, 0.0).is_empty());
        assert!(grid_lines_in_range(50.0, 0.0, 16.0).is_empty());
    }
}
