use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::{snap_to_grid, EditorCmd};

// ── Pure geometry helpers (tested below) ─────────────────────────────────────

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

/// Compute the new `(offset, size)` after dragging `handle` by `delta` pixels.
///
/// Corners adjust both offset and size; edges adjust one axis.
/// Size is clamped to `MIN_UI_SIZE × MIN_UI_SIZE`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ui_resize_new_layout(
    start_offset: glam::Vec2,
    start_size: glam::Vec2,
    delta: glam::Vec2,
    handle: ResizeHandle,
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

    #[test]
    fn resize_topleft_basic() {
        let start_offset = glam::Vec2::new(50.0, 50.0);
        let start_size = glam::Vec2::new(200.0, 100.0);
        let delta = glam::Vec2::new(20.0, 10.0); // drag TopLeft down-right → shrinks
        let (new_offset, new_size) =
            ui_resize_new_layout(start_offset, start_size, delta, ResizeHandle::TopLeft);
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
        let (_, new_size) =
            ui_resize_new_layout(start_offset, start_size, delta, ResizeHandle::TopLeft);
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
        let (new_offset, new_size) =
            ui_resize_new_layout(start_offset, start_size, delta, ResizeHandle::BottomRight);
        // offset unchanged for BottomRight.
        assert!(
            (new_offset - start_offset).length() < 0.001,
            "offset changed"
        );
        assert!((new_size.x - 130.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 70.0).abs() < 0.001, "size.y");
    }
}
