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
        let Some((cursor, left_pressed, right_pressed, left_held, right_held, digit)) = ({
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
                        digit,
                    )
                })
        }) else {
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

        // A button counts as "active" this frame if it is held OR was just pressed.
        // The just-pressed case matters because a fast click (or a click processed
        // while the editor is paused) can deliver press *and* release within a single
        // update — `is_mouse_pressed` is already false, but `mouse_just_pressed` is
        // true — and the cell under the press must still paint.
        let left_active = left_held || left_pressed;
        let right_active = right_held || right_pressed;

        // ── Stroke start ─────────────────────────────────────────────────────
        if (left_pressed || right_pressed) && !egui_wants_mouse && !self.editor.paint_active {
            self.editor.paint_active = true;
            self.editor.paint_stroke.clear();
        }

        // ── Stroke continue: paint the hovered cell ──────────────────────────
        if self.editor.paint_active && (left_active || right_active) && !egui_wants_mouse {
            // Right button erases; otherwise apply the chosen paint value.
            let value = if right_active {
                0
            } else {
                self.editor.paint_value
            };
            let recorded = {
                let Some(tm) = self.world.get_mut::<crate::tilemap::Tilemap>(sel) else {
                    return;
                };
                match tm.cell_at_world(world_pos) {
                    Some((row, col)) => {
                        let old = tm.get_tile(row, col).unwrap_or(0);
                        if tm.set_tile(row, col, value) {
                            Some((row, col, old, value))
                        } else {
                            None
                        }
                    }
                    None => None,
                }
            };
            if let Some(change) = recorded {
                self.editor.paint_stroke.push(change);
            }
        }

        // ── Stroke end: commit the whole stroke as one undo step ─────────────
        // Commit only once no button is engaged (held or just-pressed) — a same-frame
        // press/release defers its commit to the next update, after the cell is painted.
        if self.editor.paint_active && !left_active && !right_active {
            let changes = std::mem::take(&mut self.editor.paint_stroke);
            if !changes.is_empty() {
                self.editor
                    .cmd_history
                    .push(crate::app::editor::EditorCmd::PaintTiles {
                        entity: sel,
                        changes,
                    });
            }
            self.editor.paint_active = false;
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
        if just_pressed && !self.editor.gizmo_dragging && self.editor.resize_handle_active.is_none()
        {
            // World-space AABB of the entity.
            let pos = tr.position - tr.scale * 0.5;
            // Check resize handles first (higher priority than body).
            if let Some(handle) = hit_test_handles(pos, tr.scale, world_pos) {
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
            if let Some(handle) = self.editor.resize_handle_active {
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
            if let Some(_handle) = self.editor.resize_handle_active.take() {
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
}
