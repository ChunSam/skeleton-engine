use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::{snap_to_grid, EditorCmd};

// Pure geometry helpers (anchor/resize/rotation math + gizmo constants) live in
// `gizmo_math.rs`; this file holds the input-handling + rendering interaction logic.
#[cfg(not(target_arch = "wasm32"))]
use super::gizmo_math::{
    applied_rotation, cursor_angle, handle_centers, hit_test_handles, rotation_handle_pos,
    ui_drag_new_offset, ui_resize_new_layout, HANDLE_SIZE, MIN_SPRITE_SCALE, MIN_UI_SIZE,
    ROT_HANDLE_GAP, ROT_HIT_RADIUS, ROT_SNAP,
};

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
