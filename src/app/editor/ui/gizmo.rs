use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::{snap_to_grid, EditorCmd};
#[cfg(not(target_arch = "wasm32"))]
use crate::ui::Anchor;

// ── Pure geometry helpers (tested below) ─────────────────────────────────────

/// Returns the anchor base position — the point in screen-space from which
/// `UiNode::offset` is added.  This is the same formula used by
/// `UiNode::screen_pos` and must be kept in sync with it.
///
/// Factored out so both `UiNode::screen_pos` callers and the resize gizmo
/// share a single authoritative definition (no drift).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn anchor_base(anchor: Anchor, size: glam::Vec2, vw: f32, vh: f32) -> glam::Vec2 {
    match anchor {
        Anchor::TopLeft => glam::Vec2::ZERO,
        Anchor::TopCenter => glam::Vec2::new((vw - size.x) / 2.0, 0.0),
        Anchor::TopRight => glam::Vec2::new(vw - size.x, 0.0),
        Anchor::Center => glam::Vec2::new((vw - size.x) / 2.0, (vh - size.y) / 2.0),
        Anchor::BottomLeft => glam::Vec2::new(0.0, vh - size.y),
        Anchor::BottomCenter => glam::Vec2::new((vw - size.x) / 2.0, vh - size.y),
        Anchor::BottomRight => glam::Vec2::new(vw - size.x, vh - size.y),
    }
}

/// Returns the corrected offset after a screen-space drag.
///
/// `start_offset` is the UiNode offset at drag start, `start_cursor` is the
/// cursor position (logical pixels) at drag start, and `cursor` is the current
/// cursor position.  The size and anchor are unchanged, so cursor delta maps
/// 1:1 to an offset delta.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ui_drag_new_offset(
    start_offset: glam::Vec2,
    start_cursor: glam::Vec2,
    cursor: glam::Vec2,
) -> glam::Vec2 {
    start_offset + (cursor - start_cursor)
}

/// Minimum size clamped for both UI nodes and world sprites.
#[cfg(not(target_arch = "wasm32"))]
const MIN_UI_SIZE: f32 = 8.0;

/// Minimum scale for world sprites.
#[cfg(not(target_arch = "wasm32"))]
const MIN_SPRITE_SCALE: f32 = 2.0;

/// Resize-handle size (square, logical pixels).
#[cfg(not(target_arch = "wasm32"))]
const HANDLE_SIZE: f32 = 6.0;

/// Hit-test radius around a handle centre (logical pixels).
#[cfg(not(target_arch = "wasm32"))]
const HANDLE_HIT_RADIUS: f32 = 8.0;

/// Gap (world units) between the entity's top edge and the rotation handle.
#[cfg(not(target_arch = "wasm32"))]
const ROT_HANDLE_GAP: f32 = 16.0;

/// Hit-test radius (world units) around the rotation handle.
#[cfg(not(target_arch = "wasm32"))]
const ROT_HIT_RADIUS: f32 = 8.0;

/// Rotation snap step when the editor Snap toggle is on (15°).
#[cfg(not(target_arch = "wasm32"))]
const ROT_SNAP: f32 = std::f32::consts::PI / 12.0;

/// World position of the rotation handle: centred above the entity's top edge
/// (the engine's Y axis points down, so "above" is a smaller `y`).
#[cfg(not(target_arch = "wasm32"))]
fn rotation_handle_pos(position: glam::Vec2, scale: glam::Vec2, gap: f32) -> glam::Vec2 {
    glam::Vec2::new(position.x, position.y - scale.y.abs() * 0.5 - gap)
}

/// Angle (radians) of `cursor` around `center` — `atan2(dy, dx)`.
#[cfg(not(target_arch = "wasm32"))]
fn cursor_angle(center: glam::Vec2, cursor: glam::Vec2) -> f32 {
    let d = cursor - center;
    d.y.atan2(d.x)
}

/// Round `angle` to the nearest multiple of `step` (identity if `step <= 0`).
#[cfg(not(target_arch = "wasm32"))]
fn snap_angle(angle: f32, step: f32) -> f32 {
    if step <= 0.0 {
        return angle;
    }
    (angle / step).round() * step
}

/// New rotation while dragging the rotation handle: the start rotation plus the cursor-angle
/// delta, optionally snapped to `snap` radians.
#[cfg(not(target_arch = "wasm32"))]
fn applied_rotation(start_rot: f32, start_angle: f32, cur_angle: f32, snap: Option<f32>) -> f32 {
    let r = start_rot + (cur_angle - start_angle);
    match snap {
        Some(step) => snap_angle(r, step),
        None => r,
    }
}

/// Returns the 8 handle centre positions (in the same space as `pos`) in
/// `ResizeHandle` declaration order: TL, T, TR, L, R, BL, B, BR.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handle_centers(pos: glam::Vec2, size: glam::Vec2) -> [glam::Vec2; 8] {
    let cx = pos.x + size.x * 0.5;
    let cy = pos.y + size.y * 0.5;
    let r = pos.x + size.x;
    let b = pos.y + size.y;
    [
        glam::Vec2::new(pos.x, pos.y), // TopLeft
        glam::Vec2::new(cx, pos.y),    // Top
        glam::Vec2::new(r, pos.y),     // TopRight
        glam::Vec2::new(pos.x, cy),    // Left
        glam::Vec2::new(r, cy),        // Right
        glam::Vec2::new(pos.x, b),     // BottomLeft
        glam::Vec2::new(cx, b),        // Bottom
        glam::Vec2::new(r, b),         // BottomRight
    ]
}

/// Hit-test the 8 resize handles; returns the first one whose centre is within
/// `HANDLE_HIT_RADIUS` of `cursor`, or `None`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn hit_test_handles(
    pos: glam::Vec2,
    size: glam::Vec2,
    cursor: glam::Vec2,
) -> Option<ResizeHandle> {
    use ResizeHandle::*;
    let centres = handle_centers(pos, size);
    let variants = [
        TopLeft,
        Top,
        TopRight,
        Left,
        Right,
        BottomLeft,
        Bottom,
        BottomRight,
    ];
    for (c, v) in centres.iter().zip(variants.iter()) {
        if (cursor - *c).length() <= HANDLE_HIT_RADIUS {
            return Some(*v);
        }
    }
    None
}

/// Compute the new `(offset, size)` after dragging `handle` by `delta` pixels,
/// preserving the screen position of the corner that should stay fixed for ANY
/// `anchor` type.
///
/// For `Anchor::TopLeft` the anchor base is constant (zero), so the compensation
/// term is zero and behaviour is identical to the previous implementation.
/// For non-TopLeft anchors `base(size)` depends on the current size, so resizing
/// would shift the widget without the correction below.
///
/// The fix: after computing `(offset, new_size)` with TopLeft-style math,
/// add the difference `base(start_size) − base(new_size)` to offset so the
/// fixed corner's screen position (`base + offset`) remains unchanged.
///
/// Corners adjust both offset and size; edges adjust one axis.
/// Size is clamped to `MIN_UI_SIZE × MIN_UI_SIZE`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ui_resize_new_layout(
    start_offset: glam::Vec2,
    start_size: glam::Vec2,
    delta: glam::Vec2,
    handle: ResizeHandle,
    anchor: Anchor,
    viewport_wh: (f32, f32),
) -> (glam::Vec2, glam::Vec2) {
    use ResizeHandle::*;
    let mut offset = start_offset;
    let mut size = start_size;
    match handle {
        TopLeft => {
            offset += delta;
            size -= delta;
        }
        Top => {
            offset.y += delta.y;
            size.y -= delta.y;
        }
        TopRight => {
            offset.y += delta.y;
            size.x += delta.x;
            size.y -= delta.y;
        }
        Left => {
            offset.x += delta.x;
            size.x -= delta.x;
        }
        Right => {
            size.x += delta.x;
        }
        BottomLeft => {
            offset.x += delta.x;
            size.x -= delta.x;
            size.y += delta.y;
        }
        Bottom => {
            size.y += delta.y;
        }
        BottomRight => {
            size += delta;
        }
    }
    // Clamp size.  When a top/left edge would push size below min, lock it.
    if size.x < MIN_UI_SIZE {
        let excess = MIN_UI_SIZE - size.x;
        size.x = MIN_UI_SIZE;
        match handle {
            TopLeft | Left | BottomLeft => {
                offset.x -= excess;
            }
            _ => {}
        }
    }
    if size.y < MIN_UI_SIZE {
        let excess = MIN_UI_SIZE - size.y;
        size.y = MIN_UI_SIZE;
        match handle {
            TopLeft | Top | TopRight => {
                offset.y -= excess;
            }
            _ => {}
        }
    }
    // Anchor-base compensation: for non-TopLeft anchors the base position
    // depends on `size`, so a size change shifts `screen_pos = base + offset`
    // unless we compensate by adjusting `offset` by the change in base.
    // For TopLeft: base == Vec2::ZERO always → compensation is zero (no change).
    let (vw, vh) = viewport_wh;
    let base_delta = anchor_base(anchor, start_size, vw, vh) - anchor_base(anchor, size, vw, vh);
    offset += base_delta;
    (offset, size)
}

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

// ─────────────────────────────────────────────────────────────────────────────

impl App {
    pub(in crate::app) fn update_editor_gizmo(&mut self, egui_ctx: &Option<egui::Context>) {
        // ── Gizmo: highlight selected entity + drag to move ──────────────────
        // Docked mode cannot use `egui_wants_pointer_input()` — the game viewport
        // is itself an egui CentralPanel, so egui always claims the pointer there.
        // Use the layer-aware gate instead (same rule as the window input routing).
        #[cfg(not(target_arch = "wasm32"))]
        let egui_wants_mouse = {
            use crate::app::editor::{docked_rt::docked_game_pointer_allowed, EditorMode};
            if self.editor.mode == EditorMode::Docked {
                !docked_game_pointer_allowed(
                    self.editor.window_cursor,
                    self.editor.central_rect,
                    egui_ctx.as_ref(),
                )
            } else {
                egui_ctx
                    .as_ref()
                    .map(|c| c.egui_wants_pointer_input())
                    .unwrap_or(false)
            }
        };
        #[cfg(target_arch = "wasm32")]
        let egui_wants_mouse = egui_ctx
            .as_ref()
            .map(|c| c.egui_wants_pointer_input())
            .unwrap_or(false);

        let Some(sel) = self.editor.inspector_selected else {
            self.editor.gizmo_dragging = false;
            return;
        };

        // ── Tile paint: when paint mode is on for a Tilemap entity, viewport
        //    clicks paint tiles and the move/resize gizmo is suppressed. ──────
        #[cfg(not(target_arch = "wasm32"))]
        if self.editor.paint_mode {
            if self.world.get::<crate::tilemap::Tilemap>(sel).is_some() {
                self.update_tile_paint(sel, egui_wants_mouse);
                self.editor.gizmo_dragging = false;
                return;
            }
            // Selection is no longer a Tilemap — leave paint mode cleanly.
            self.editor.paint_mode = false;
            self.editor.paint_active = false;
            self.editor.paint_stroke.clear();
            self.editor.paint_anchor = None;
        }

        // ── Branch: UiNode (screen-space) vs Transform (world-space) ─────────
        // Check for UiNode first; a UiNode entity may also carry a Transform but
        // we always treat it in screen-space.
        let has_ui_node = self.world.get::<crate::ui::UiNode>(sel).is_some();

        if has_ui_node {
            self.update_ui_node_gizmo(sel, egui_wants_mouse);
        } else {
            // Existing world-space path, now extended with resize handles.
            self.update_transform_gizmo(sel, egui_wants_mouse);
        }
    }

    // ── Tile painting (native only) ───────────────────────────────────────────

    /// Paint tiles on the selected `Tilemap` while paint mode is active.
    ///
    /// Left-click/drag paints [`EditorState::paint_value`]; right-click/drag erases
    /// (value `0`). Number keys `1..=9` set the paint value (`0` = erase), clamped to
    /// the atlas tile count. Each press→release stroke records every cell it actually
    /// changed and is committed to the editor history as a single
    /// [`EditorCmd::PaintTiles`](crate::app::editor::EditorCmd) so one Ctrl+Z reverts
    /// the whole stroke. Painting is **visual-only**: tile colliders are not synced
    /// (that is the app's responsibility via `sync_static_from_tilemap`).
    #[cfg(not(target_arch = "wasm32"))]
    fn update_tile_paint(&mut self, sel: crate::ecs::Entity, egui_wants_mouse: bool) {
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
        }
    }

    // ── Screen-space UiNode gizmo ─────────────────────────────────────────────

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    fn update_ui_node_gizmo(&mut self, sel: crate::ecs::Entity, egui_wants_mouse: bool) {
        // Copy data out (short borrow).
        let node_copy = self.world.get::<crate::ui::UiNode>(sel).cloned();
        let Some(node) = node_copy else {
            return;
        };

        let vp_default = crate::resources::ViewportSize::default();
        let vp = self
            .world
            .resource::<crate::resources::ViewportSize>()
            .copied()
            .unwrap_or(vp_default);

        let screen_pos = node.screen_pos(&vp);

        // ── Selection highlight via UiQueue (screen-space) ────────────────────
        const MARGIN: f32 = 2.0;
        if let Some(q) = self.world.resource_mut::<crate::renderer::ui::UiQueue>() {
            q.push(
                crate::renderer::ui::DrawRect::new(
                    screen_pos.x - MARGIN,
                    screen_pos.y - MARGIN,
                    node.size.x + MARGIN * 2.0,
                    node.size.y + MARGIN * 2.0,
                    crate::color::Color::rgba(0.2, 0.85, 1.0, 0.65),
                )
                .with_z(node.z + 0.01),
            );

            // ── 8 resize handles ──────────────────────────────────────────────
            #[cfg(not(target_arch = "wasm32"))]
            {
                let centres = handle_centers(screen_pos, node.size);
                for c in &centres {
                    q.push(
                        crate::renderer::ui::DrawRect::new(
                            c.x - HANDLE_SIZE * 0.5,
                            c.y - HANDLE_SIZE * 0.5,
                            HANDLE_SIZE,
                            HANDLE_SIZE,
                            crate::color::Color::rgba(1.0, 1.0, 1.0, 0.9),
                        )
                        .with_z(node.z + 0.02),
                    );
                }
            }
        }

        if egui_wants_mouse {
            self.editor.gizmo_dragging = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.editor.resize_handle_active = None;
            }
            return;
        }

        // ── Input ─────────────────────────────────────────────────────────────
        let input_copy = self
            .world
            .resource::<crate::input::InputState>()
            .map(|inp| {
                let cursor = inp.cursor();
                let pressed = inp.mouse_just_pressed(winit::event::MouseButton::Left);
                let held = inp.is_mouse_pressed(winit::event::MouseButton::Left);
                let released = inp.mouse_just_released(winit::event::MouseButton::Left);
                (cursor, pressed, held, released)
            });
        let Some((cursor, just_pressed, held, just_released)) = input_copy else {
            return;
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.update_ui_node_gizmo_native(
                sel,
                node,
                screen_pos,
                vp,
                cursor,
                just_pressed,
                held,
                just_released,
            );
        }

        // WASM: move-only (no resize handles, no undo).
        #[cfg(target_arch = "wasm32")]
        {
            let inside = cursor.x >= screen_pos.x
                && cursor.x <= screen_pos.x + node.size.x
                && cursor.y >= screen_pos.y
                && cursor.y <= screen_pos.y + node.size.y;

            if just_pressed && !self.editor.gizmo_dragging && inside {
                self.editor.gizmo_dragging = true;
                self.editor.gizmo_drag_offset = node.offset - cursor;
            }
            if self.editor.gizmo_dragging && held {
                if let Some(n) = self.world.get_mut::<crate::ui::UiNode>(sel) {
                    n.offset = cursor + self.editor.gizmo_drag_offset;
                }
            }
            if just_released {
                self.editor.gizmo_dragging = false;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    fn update_ui_node_gizmo_native(
        &mut self,
        sel: crate::ecs::Entity,
        node: crate::ui::UiNode,
        screen_pos: glam::Vec2,
        vp: crate::resources::ViewportSize,
        cursor: glam::Vec2,
        just_pressed: bool,
        held: bool,
        just_released: bool,
    ) {
        // ── Press: start a drag ───────────────────────────────────────────────
        if just_pressed && !self.editor.gizmo_dragging && self.editor.resize_handle_active.is_none()
        {
            // Try resize handles first (higher priority).
            if let Some(handle) = hit_test_handles(screen_pos, node.size, cursor) {
                self.editor.resize_handle_active = Some(handle);
                self.editor.resize_drag_start_cursor = cursor;
                self.editor.resize_drag_start_offset = node.offset;
                self.editor.resize_drag_start_size = node.size;
            } else {
                // Fall back to move-drag (body hit-test).
                let inside = cursor.x >= screen_pos.x
                    && cursor.x <= screen_pos.x + node.size.x
                    && cursor.y >= screen_pos.y
                    && cursor.y <= screen_pos.y + node.size.y;
                if inside {
                    self.editor.gizmo_dragging = true;
                    self.editor.gizmo_drag_offset = node.offset - cursor;
                    self.editor.resize_drag_start_offset = node.offset; // record start for undo
                }
            }
        }

        // ── Hold: apply move or resize ────────────────────────────────────────
        if held {
            if let Some(handle) = self.editor.resize_handle_active {
                let delta = cursor - self.editor.resize_drag_start_cursor;
                let (new_offset, new_size) = ui_resize_new_layout(
                    self.editor.resize_drag_start_offset,
                    self.editor.resize_drag_start_size,
                    delta,
                    handle,
                    node.anchor,
                    (vp.width, vp.height),
                );
                let snapped_size = if self.editor.snap_enabled {
                    glam::Vec2::new(
                        (new_size.x / self.editor.snap_size).round() * self.editor.snap_size,
                        (new_size.y / self.editor.snap_size).round() * self.editor.snap_size,
                    )
                    .max(glam::Vec2::splat(MIN_UI_SIZE))
                } else {
                    new_size
                };
                let snapped_offset = if self.editor.snap_enabled {
                    snap_to_grid(new_offset, self.editor.snap_size)
                } else {
                    new_offset
                };
                if let Some(n) = self.world.get_mut::<crate::ui::UiNode>(sel) {
                    n.offset = snapped_offset;
                    n.size = snapped_size;
                }
            } else if self.editor.gizmo_dragging {
                let new_offset = ui_drag_new_offset(
                    self.editor.resize_drag_start_offset,
                    // start_cursor = drag_start_offset - gizmo_drag_offset
                    self.editor.resize_drag_start_offset - self.editor.gizmo_drag_offset,
                    cursor,
                );
                let final_offset = if self.editor.snap_enabled {
                    snap_to_grid(new_offset, self.editor.snap_size)
                } else {
                    new_offset
                };
                if let Some(n) = self.world.get_mut::<crate::ui::UiNode>(sel) {
                    n.offset = final_offset;
                }
            }
        }

        // ── Release: record undo command ──────────────────────────────────────
        if just_released {
            if let Some(handle) = self.editor.resize_handle_active.take() {
                let _ = handle;
                let old_offset = self.editor.resize_drag_start_offset;
                let old_size = self.editor.resize_drag_start_size;
                let (new_offset, new_size) = self
                    .world
                    .get::<crate::ui::UiNode>(sel)
                    .map(|n| (n.offset, n.size))
                    .unwrap_or((old_offset, old_size));
                if (new_offset - old_offset).length_squared() > 0.01
                    || (new_size - old_size).length_squared() > 0.01
                {
                    self.editor.cmd_history.push(EditorCmd::ResizeUiNode {
                        entity: sel,
                        old_offset,
                        old_size,
                        new_offset,
                        new_size,
                    });
                }
            } else if self.editor.gizmo_dragging {
                let old_offset = self.editor.resize_drag_start_offset;
                let new_offset = self
                    .world
                    .get::<crate::ui::UiNode>(sel)
                    .map(|n| n.offset)
                    .unwrap_or(old_offset);
                if (new_offset - old_offset).length_squared() > 0.01 {
                    self.editor.cmd_history.push(EditorCmd::MoveUiNode {
                        entity: sel,
                        old_offset,
                        new_offset,
                    });
                }
                self.editor.gizmo_dragging = false;
            }
        }
    }

    // ── World-space Transform gizmo (move + resize) ───────────────────────────

    fn update_transform_gizmo(&mut self, sel: crate::ecs::Entity, egui_wants_mouse: bool) {
        // Copy the selected entity's Transform (releases the borrow).
        let tr_copy = self.world.get::<crate::components::Transform>(sel).cloned();

        let Some(tr) = tr_copy else {
            self.editor.gizmo_dragging = false;
            return;
        };

        // Selection highlight: add a translucent filled rectangle via DebugDraw.
        if let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() {
            let half = tr.scale * 0.5;
            let margin = glam::Vec2::splat(3.0 / tr.scale.x.max(1.0) * tr.scale.x);
            dbg.rect_filled_z(
                tr.position - half - margin,
                tr.position + half + margin,
                crate::color::Color::rgba(0.2, 0.85, 1.0, 0.65),
                tr.z + 999.0,
            );

            // ── 8 world-space resize handles ──────────────────────────────────
            #[cfg(not(target_arch = "wasm32"))]
            {
                let pos = tr.position - tr.scale * 0.5;
                let centres = handle_centers(pos, tr.scale);
                for c in &centres {
                    let hs = 4.0; // half handle size in world units
                    dbg.rect_filled_z(
                        glam::Vec2::new(c.x - hs, c.y - hs),
                        glam::Vec2::new(c.x + hs, c.y + hs),
                        crate::color::Color::rgba(1.0, 1.0, 1.0, 0.9),
                        tr.z + 1000.0,
                    );
                }

                // ── Rotation handle (green, above the top edge) ───────────────
                let rh = rotation_handle_pos(tr.position, tr.scale, ROT_HANDLE_GAP);
                let hs = 5.0;
                dbg.rect_filled_z(
                    glam::Vec2::new(rh.x - hs, rh.y - hs),
                    glam::Vec2::new(rh.x + hs, rh.y + hs),
                    crate::color::Color::rgba(0.3, 1.0, 0.4, 0.95),
                    tr.z + 1000.0,
                );
            }
        }

        // Gizmo drag — only when egui is not consuming mouse input.
        if !egui_wants_mouse {
            let cam_default = crate::camera::Camera::default();
            let gizmo_input = {
                let cam = self
                    .world
                    .resource::<crate::camera::Camera>()
                    .unwrap_or(&cam_default);
                self.world
                    .resource::<crate::input::InputState>()
                    .map(|inp| {
                        let world_pos = cam.screen_to_world(inp.cursor());
                        let pressed = inp.mouse_just_pressed(winit::event::MouseButton::Left);
                        let held = inp.is_mouse_pressed(winit::event::MouseButton::Left);
                        let released = inp.mouse_just_released(winit::event::MouseButton::Left);
                        (world_pos, pressed, held, released)
                    })
            };

            if let Some((world_pos, just_pressed, held, just_released)) = gizmo_input {
                // Native path: supports group-move, snap, undo, and resize.
                #[cfg(not(target_arch = "wasm32"))]
                self.update_transform_gizmo_native(
                    sel,
                    tr,
                    world_pos,
                    just_pressed,
                    held,
                    just_released,
                );

                // WASM: simple move only.
                #[cfg(target_arch = "wasm32")]
                {
                    if just_pressed && !self.editor.gizmo_dragging {
                        let half = tr.scale * 0.5;
                        let hit = world_pos.x >= tr.position.x - half.x
                            && world_pos.x <= tr.position.x + half.x
                            && world_pos.y >= tr.position.y - half.y
                            && world_pos.y <= tr.position.y + half.y;
                        if hit {
                            self.editor.gizmo_dragging = true;
                            self.editor.gizmo_drag_offset = tr.position - world_pos;
                        }
                    }
                    if self.editor.gizmo_dragging && held {
                        let new_pos = world_pos + self.editor.gizmo_drag_offset;
                        if let Some(t) = self.world.get_mut::<crate::components::Transform>(sel) {
                            t.position = new_pos;
                        }
                    }
                    if just_released {
                        self.editor.gizmo_dragging = false;
                    }
                }
            }
        } else {
            self.editor.gizmo_dragging = false;
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.editor.resize_handle_active = None;
                self.editor.rotate_active = false;
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn update_transform_gizmo_native(
        &mut self,
        sel: crate::ecs::Entity,
        tr: crate::components::Transform,
        world_pos: glam::Vec2,
        just_pressed: bool,
        held: bool,
        just_released: bool,
    ) {
        // ── Press ─────────────────────────────────────────────────────────────
        if just_pressed
            && !self.editor.gizmo_dragging
            && self.editor.resize_handle_active.is_none()
            && !self.editor.rotate_active
        {
            // World-space AABB of the entity.
            let pos = tr.position - tr.scale * 0.5;
            // Rotation handle has top priority (it sits outside the AABB).
            let rot_handle = rotation_handle_pos(tr.position, tr.scale, ROT_HANDLE_GAP);
            if (world_pos - rot_handle).length() <= ROT_HIT_RADIUS {
                self.editor.rotate_active = true;
                self.editor.rotate_start_rotation = tr.rotation;
                self.editor.rotate_start_angle = cursor_angle(tr.position, world_pos);
            } else if let Some(handle) = hit_test_handles(pos, tr.scale, world_pos) {
                self.editor.resize_handle_active = Some(handle);
                self.editor.resize_drag_start_cursor = world_pos;
                self.editor.resize_drag_start_scale = tr.scale;
            } else {
                let half = tr.scale * 0.5;
                let hit = world_pos.x >= tr.position.x - half.x
                    && world_pos.x <= tr.position.x + half.x
                    && world_pos.y >= tr.position.y - half.y
                    && world_pos.y <= tr.position.y + half.y;
                if hit {
                    self.editor.gizmo_dragging = true;
                    self.editor.gizmo_drag_offset = tr.position - world_pos;
                    self.editor.gizmo_drag_start_pos = Some(tr.position);
                    // Snapshot start positions of all selected entities for group-move undo.
                    let mut starts: Vec<(crate::ecs::Entity, glam::Vec2)> = Vec::new();
                    let mut has_sel = false;
                    for &e in &self.editor.selected_entities {
                        if let Some(t) = self.world.get::<crate::components::Transform>(e) {
                            if e == sel {
                                has_sel = true;
                            }
                            starts.push((e, t.position));
                        }
                    }
                    if !has_sel {
                        starts.push((sel, tr.position));
                    }
                    self.editor.gizmo_drag_start_positions = starts;
                }
            }
        }

        // ── Hold ─────────────────────────────────────────────────────────────
        if held {
            if self.editor.rotate_active {
                let cur = cursor_angle(tr.position, world_pos);
                let snap = self.editor.snap_enabled.then_some(ROT_SNAP);
                let new_rot = applied_rotation(
                    self.editor.rotate_start_rotation,
                    self.editor.rotate_start_angle,
                    cur,
                    snap,
                );
                if let Some(t) = self.world.get_mut::<crate::components::Transform>(sel) {
                    t.rotation = new_rot;
                }
            } else if let Some(handle) = self.editor.resize_handle_active {
                // Scale the entity from its centre.  The corner/edge delta in
                // world-space maps directly to a scale change; position stays fixed.
                let delta = world_pos - self.editor.resize_drag_start_cursor;
                let start_scale = self.editor.resize_drag_start_scale;

                // Compute new scale from drag direction and handle type.
                use ResizeHandle::*;
                let mut new_scale = start_scale;
                match handle {
                    TopLeft => {
                        new_scale.x = (start_scale.x - delta.x * 2.0).max(MIN_SPRITE_SCALE);
                        new_scale.y = (start_scale.y - delta.y * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    Top => {
                        new_scale.y = (start_scale.y - delta.y * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    TopRight => {
                        new_scale.x = (start_scale.x + delta.x * 2.0).max(MIN_SPRITE_SCALE);
                        new_scale.y = (start_scale.y - delta.y * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    Left => {
                        new_scale.x = (start_scale.x - delta.x * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    Right => {
                        new_scale.x = (start_scale.x + delta.x * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    BottomLeft => {
                        new_scale.x = (start_scale.x - delta.x * 2.0).max(MIN_SPRITE_SCALE);
                        new_scale.y = (start_scale.y + delta.y * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    Bottom => {
                        new_scale.y = (start_scale.y + delta.y * 2.0).max(MIN_SPRITE_SCALE);
                    }
                    BottomRight => {
                        new_scale.x = (start_scale.x + delta.x * 2.0).max(MIN_SPRITE_SCALE);
                        new_scale.y = (start_scale.y + delta.y * 2.0).max(MIN_SPRITE_SCALE);
                    }
                }
                let final_scale = if self.editor.snap_enabled {
                    glam::Vec2::new(
                        (new_scale.x / self.editor.snap_size).round() * self.editor.snap_size,
                        (new_scale.y / self.editor.snap_size).round() * self.editor.snap_size,
                    )
                    .max(glam::Vec2::splat(MIN_SPRITE_SCALE))
                } else {
                    new_scale
                };
                if let Some(t) = self.world.get_mut::<crate::components::Transform>(sel) {
                    t.scale = final_scale;
                }
            } else if self.editor.gizmo_dragging {
                let new_pos = world_pos + self.editor.gizmo_drag_offset;
                let final_pos = if self.editor.snap_enabled {
                    snap_to_grid(new_pos, self.editor.snap_size)
                } else {
                    new_pos
                };
                let old_pos = self
                    .world
                    .get::<crate::components::Transform>(sel)
                    .map(|t| t.position)
                    .unwrap_or(final_pos);
                let delta = final_pos - old_pos;
                if let Some(t) = self.world.get_mut::<crate::components::Transform>(sel) {
                    t.position = final_pos;
                }
                let others: Vec<crate::ecs::Entity> = self
                    .editor
                    .selected_entities
                    .iter()
                    .copied()
                    .filter(|&e| e != sel)
                    .collect();
                for other in others {
                    if let Some(t) = self.world.get_mut::<crate::components::Transform>(other) {
                        t.position += delta;
                    }
                }
            }
        }

        // ── Release ───────────────────────────────────────────────────────────
        if just_released {
            if self.editor.rotate_active {
                self.editor.rotate_active = false;
                let old_rotation = self.editor.rotate_start_rotation;
                let new_rotation = self
                    .world
                    .get::<crate::components::Transform>(sel)
                    .map(|t| t.rotation)
                    .unwrap_or(old_rotation);
                if (new_rotation - old_rotation).abs() > 1e-4 {
                    self.editor.cmd_history.push(EditorCmd::RotateEntity {
                        entity: sel,
                        old_rotation,
                        new_rotation,
                    });
                }
            } else if let Some(_handle) = self.editor.resize_handle_active.take() {
                let old_scale = self.editor.resize_drag_start_scale;
                let new_scale = self
                    .world
                    .get::<crate::components::Transform>(sel)
                    .map(|t| t.scale)
                    .unwrap_or(old_scale);
                if (new_scale - old_scale).length_squared() > 0.01 {
                    self.editor.cmd_history.push(EditorCmd::ResizeEntity {
                        entity: sel,
                        old_scale,
                        new_scale,
                    });
                }
            } else {
                // Record the entire group move.
                let starts = std::mem::take(&mut self.editor.gizmo_drag_start_positions);
                for (entity, start_pos) in starts {
                    let new_pos = self
                        .world
                        .get::<crate::components::Transform>(entity)
                        .map(|t| t.position)
                        .unwrap_or(start_pos);
                    if (new_pos - start_pos).length_squared() > 0.01 {
                        self.editor.cmd_history.push(EditorCmd::MoveEntity {
                            entity,
                            old_pos: start_pos,
                            new_pos,
                        });
                    }
                }
                self.editor.gizmo_drag_start_pos = None;
                self.editor.gizmo_dragging = false;
            }
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── handle_centers / hit_test_handles ───────────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_handle_hit_corners() {
        // Box at (50, 50) with size (200, 100).
        let pos = glam::Vec2::new(50.0, 50.0);
        let size = glam::Vec2::new(200.0, 100.0);

        // Top-left corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(50.0, 50.0)),
            Some(ResizeHandle::TopLeft)
        );
        // Top-right corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(250.0, 50.0)),
            Some(ResizeHandle::TopRight)
        );
        // Bottom-left corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(50.0, 150.0)),
            Some(ResizeHandle::BottomLeft)
        );
        // Bottom-right corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(250.0, 150.0)),
            Some(ResizeHandle::BottomRight)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_handle_hit_middle_returns_none() {
        let pos = glam::Vec2::new(50.0, 50.0);
        let size = glam::Vec2::new(200.0, 100.0);

        // Centre of the box — far from any handle.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(150.0, 100.0)),
            None
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_handle_hit_edge_midpoints() {
        let pos = glam::Vec2::new(50.0, 50.0);
        let size = glam::Vec2::new(200.0, 100.0);

        // Top edge midpoint: (150, 50).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(150.0, 50.0)),
            Some(ResizeHandle::Top)
        );
        // Right edge midpoint: (250, 100).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(250.0, 100.0)),
            Some(ResizeHandle::Right)
        );
        // Bottom edge midpoint: (150, 150).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(150.0, 150.0)),
            Some(ResizeHandle::Bottom)
        );
        // Left edge midpoint: (50, 100).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(50.0, 100.0)),
            Some(ResizeHandle::Left)
        );
    }

    // ─── ui_drag_new_offset ───────────────────────────────────────────────────

    #[test]
    fn move_offset_invariant_simple() {
        let start_offset = glam::Vec2::new(100.0, 200.0);
        let start_cursor = glam::Vec2::new(150.0, 220.0);
        let cursor = glam::Vec2::new(170.0, 230.0); // moved by (20, 10)
        let result = ui_drag_new_offset(start_offset, start_cursor, cursor);
        let expected = glam::Vec2::new(120.0, 210.0);
        assert!(
            (result - expected).length() < 0.001,
            "got {result:?} expected {expected:?}"
        );
    }

    #[test]
    fn move_offset_invariant_zero_delta() {
        let start_offset = glam::Vec2::new(50.0, 80.0);
        let start_cursor = glam::Vec2::new(100.0, 100.0);
        let result = ui_drag_new_offset(start_offset, start_cursor, start_cursor);
        assert!((result - start_offset).length() < 0.001);
    }

    // ─── ui_resize_new_layout ────────────────────────────────────────────────
    // TopLeft anchor: base is always zero → compensation is zero → same assertions as before.

    #[test]
    fn resize_topleft_basic() {
        let start_offset = glam::Vec2::new(50.0, 50.0);
        let start_size = glam::Vec2::new(200.0, 100.0);
        let delta = glam::Vec2::new(20.0, 10.0); // drag TopLeft down-right → shrinks
        let (new_offset, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::TopLeft,
            Anchor::TopLeft,
            (800.0, 600.0),
        );
        assert!((new_offset.x - 70.0).abs() < 0.001, "offset.x");
        assert!((new_offset.y - 60.0).abs() < 0.001, "offset.y");
        assert!((new_size.x - 180.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 90.0).abs() < 0.001, "size.y");
    }

    #[test]
    fn resize_topleft_min_clamp() {
        let start_offset = glam::Vec2::new(50.0, 50.0);
        let start_size = glam::Vec2::new(20.0, 20.0);
        // Delta so large it would go negative.
        let delta = glam::Vec2::new(100.0, 100.0);
        let (_, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::TopLeft,
            Anchor::TopLeft,
            (800.0, 600.0),
        );
        assert!(
            new_size.x >= MIN_UI_SIZE,
            "size.x below min: {}",
            new_size.x
        );
        assert!(
            new_size.y >= MIN_UI_SIZE,
            "size.y below min: {}",
            new_size.y
        );
    }

    #[test]
    fn resize_bottomright_grows() {
        let start_offset = glam::Vec2::new(10.0, 10.0);
        let start_size = glam::Vec2::new(100.0, 50.0);
        let delta = glam::Vec2::new(30.0, 20.0);
        let (new_offset, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::BottomRight,
            Anchor::TopLeft,
            (800.0, 600.0),
        );
        // offset unchanged for BottomRight with TopLeft anchor.
        assert!(
            (new_offset - start_offset).length() < 0.001,
            "offset changed"
        );
        assert!((new_size.x - 130.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 70.0).abs() < 0.001, "size.y");
    }

    /// For a Center-anchored node, dragging the BottomRight handle must grow
    /// the size WITHOUT shifting the widget's top-left screen position.
    ///
    /// screen_pos = base(anchor, size, vw, vh) + offset
    ///
    /// Before: size=(100,50), offset=(0,0), vp=(800,600)
    ///   base = ((800-100)/2, (600-50)/2) = (350, 275)  → screen_pos = (350, 275)
    ///
    /// After drag BottomRight by (30, 20): size=(130,70)
    ///   For screen_pos to stay at (350, 275) we need:
    ///   base_new = ((800-130)/2, (600-70)/2) = (335, 265)
    ///   new_offset = screen_pos - base_new = (15, 10)
    ///
    /// The fix applies: offset += base(start_size) - base(new_size)
    ///   = (350,275) - (335,265) = (15,10)  ✓
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_center_anchor_fixed_corner_preserved() {
        let vp = (800.0_f32, 600.0_f32);
        let start_size = glam::Vec2::new(100.0, 50.0);
        let start_offset = glam::Vec2::ZERO;
        let anchor = Anchor::Center;

        // Compute screen_pos before resize.
        let base_before = anchor_base(anchor, start_size, vp.0, vp.1);
        let screen_pos_before = base_before + start_offset;

        let delta = glam::Vec2::new(30.0, 20.0);
        let (new_offset, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::BottomRight,
            anchor,
            vp,
        );

        // Compute screen_pos after resize.
        let base_after = anchor_base(anchor, new_size, vp.0, vp.1);
        let screen_pos_after = base_after + new_offset;

        assert!(
            (screen_pos_after - screen_pos_before).length() < 0.001,
            "top-left screen position shifted: before={screen_pos_before:?} after={screen_pos_after:?}"
        );
        // Size did grow as expected.
        assert!((new_size.x - 130.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 70.0).abs() < 0.001, "size.y");
    }

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

    // ─── rotation gizmo ──────────────────────────────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rotation_helpers_math() {
        use std::f32::consts::PI;
        // cursor_angle cardinal directions (Y down: +y is "down").
        assert!(cursor_angle(glam::Vec2::ZERO, glam::Vec2::new(10.0, 0.0)).abs() < 1e-5);
        assert!(
            (cursor_angle(glam::Vec2::ZERO, glam::Vec2::new(0.0, 10.0)) - PI / 2.0).abs() < 1e-5
        );
        // Rotation handle sits above the top edge (smaller y).
        assert_eq!(
            rotation_handle_pos(
                glam::Vec2::new(50.0, 50.0),
                glam::Vec2::new(20.0, 20.0),
                16.0
            ),
            glam::Vec2::new(50.0, 24.0)
        );
        // Snap to nearest multiple; step<=0 is identity.
        assert!(snap_angle(0.4, 1.0).abs() < 1e-6);
        assert!((snap_angle(0.6, 1.0) - 1.0).abs() < 1e-6);
        assert_eq!(snap_angle(0.7, 0.0), 0.7);
        // applied_rotation = start + (cur - start_angle).
        assert!((applied_rotation(0.0, -PI / 2.0, 0.0, None) - PI / 2.0).abs() < 1e-5);
        assert!((applied_rotation(1.0, 0.5, 0.5, None) - 1.0).abs() < 1e-6); // no delta
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rotation_gizmo_drag_rotates_and_undoes() {
        use std::f32::consts::PI;
        let mut app = crate::app::App::new();
        let e = app.world.spawn();
        app.world.add_component(
            e,
            crate::components::Transform::new(glam::Vec2::ZERO, glam::Vec2::splat(20.0), 0.0),
        );
        app.editor.inspector_selected = Some(e);
        app.editor.selected_entities = vec![e];
        let tr = app
            .world
            .get::<crate::components::Transform>(e)
            .cloned()
            .unwrap();

        // Press on the rotation handle at (0, -26) = (0, -(scale.y/2 + gap)).
        app.update_transform_gizmo_native(
            e,
            tr.clone(),
            glam::Vec2::new(0.0, -26.0),
            true,
            false,
            false,
        );
        assert!(app.editor.rotate_active, "rotation drag started");

        // Drag to (26, 0): a quarter turn around the centre → rotation ≈ +PI/2.
        app.update_transform_gizmo_native(
            e,
            tr.clone(),
            glam::Vec2::new(26.0, 0.0),
            false,
            true,
            false,
        );
        let rot = app
            .world
            .get::<crate::components::Transform>(e)
            .unwrap()
            .rotation;
        assert!((rot - PI / 2.0).abs() < 1e-3, "rotated ~90°, got {rot}");

        // Release commits the RotateEntity command.
        app.update_transform_gizmo_native(e, tr, glam::Vec2::new(26.0, 0.0), false, false, true);
        assert!(!app.editor.rotate_active);

        // Undo reverts to the original rotation.
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert!(
            app.world
                .get::<crate::components::Transform>(e)
                .unwrap()
                .rotation
                .abs()
                < 1e-4,
            "undo reverted rotation to 0"
        );
    }
}
