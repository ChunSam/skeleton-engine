use super::*;
use crate::app::editor::theme;
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
            // Every flag, not just `gizmo_dragging`: the selection can vanish mid-drag (Ctrl+Z
            // over a `CreateEntity` sets it to `None`) and this return is then the only code
            // that runs — the release handler is never reached again. See
            // `EditorState::clear_drag_state`. Since v0.156.6 the gesture is *recorded* on its
            // way out, not just cleared: what it applied is on the entity either way.
            self.abandon_gesture();
            // A tile-paint stroke is abandoned by the same event, and clearing flags is not
            // enough: its cells are already on the map, so it is committed against the tilemap
            // it was painted on. Left standing, it used to be carried into the *next* stroke on
            // a different tilemap and undone against that one (v0.155.8).
            #[cfg(not(target_arch = "wasm32"))]
            self.finish_paint_stroke();
            return;
        };

        // A gesture belongs to the entity it started on. If the selection moved while the button
        // was held — undo/redo set it from the keyboard, which no `egui_wants_mouse` guard
        // covers — end it there: every branch below reads `sel` fresh, so a held frame would
        // apply the first entity's start state to the second, and the release would record the
        // first entity's old value under the second's id.
        #[cfg(not(target_arch = "wasm32"))]
        if self.editor.gesture_active() && self.editor.gesture_entity != Some(sel) {
            self.abandon_gesture();
        }

        // A tile-paint stroke belongs to the entity it was painted on, whether or not paint mode
        // is still on. In Docked mode the inspector clears `paint_mode` for a non-Tilemap
        // selection *before* this runs, so the arm below that ends a stroke when "the selection
        // ceased to be a Tilemap" was unreachable there: the stroke stayed buffered, neither
        // committed nor cleared, until some later stroke committed it (wiping the redo stack) or
        // nothing ever did.
        #[cfg(not(target_arch = "wasm32"))]
        if self.editor.paint_active && self.editor.paint_entity != Some(sel) {
            self.finish_paint_stroke();
        }

        // ── Tile paint: when paint mode is on for a Tilemap entity, viewport
        //    clicks paint tiles and the move/resize gizmo is suppressed. ──────
        #[cfg(not(target_arch = "wasm32"))]
        if self.editor.paint_mode {
            if self.world.get::<crate::tilemap::Tilemap>(sel).is_some() {
                self.update_tile_paint(sel, egui_wants_mouse);
                self.editor.gizmo_dragging = false;
                return;
            }
            // Selection is no longer a Tilemap — leave paint mode cleanly. The stroke is
            // committed rather than dropped: those cells are on the map either way, and the
            // batch knows which tilemap they belong to.
            self.finish_paint_stroke();
            self.editor.paint_mode = false;
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
            // The node went away under an in-progress gesture: record and clear it here, since
            // the release handler below is never reached again for this entity.
            self.abandon_gesture();
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
                    theme::GIZMO_SELECT_COLOR,
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
                            theme::GIZMO_HANDLE_COLOR,
                        )
                        .with_z(node.z + 0.02),
                    );
                }
            }
        }

        if egui_wants_mouse {
            self.abandon_gesture();
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
                self.editor.gesture_entity = Some(sel);
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
                    self.editor.gesture_entity = Some(sel);
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
            self.commit_gesture(sel);
        }
    }

    // ── World-space Transform gizmo (move + resize) ───────────────────────────

    fn update_transform_gizmo(&mut self, sel: crate::ecs::Entity, egui_wants_mouse: bool) {
        // Copy the selected entity's Transform (releases the borrow).
        let tr_copy = self.world.get::<crate::components::Transform>(sel).cloned();

        let Some(tr) = tr_copy else {
            // Same as the `UiNode` arm: the component went away under a gesture. This site used
            // to clear `gizmo_dragging` alone, leaving a rotation or resize flag set forever.
            self.abandon_gesture();
            return;
        };

        // Selection highlight: add a translucent filled rectangle via DebugDraw.
        if let Some(dbg) = self.world.resource_mut::<crate::resources::DebugDraw>() {
            let half = tr.scale * 0.5;
            let margin = glam::Vec2::splat(3.0 / tr.scale.x.max(1.0) * tr.scale.x);
            dbg.rect_filled_z(
                tr.position - half - margin,
                tr.position + half + margin,
                theme::GIZMO_SELECT_COLOR,
                tr.z + theme::GIZMO_SELECT_Z_BIAS,
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
                        theme::GIZMO_HANDLE_COLOR,
                        tr.z + theme::GIZMO_HANDLE_Z_BIAS,
                    );
                }

                // ── Rotation handle (green, above the top edge) ───────────────
                let rh = rotation_handle_pos(tr.position, tr.scale, ROT_HANDLE_GAP);
                let hs = 5.0;
                dbg.rect_filled_z(
                    glam::Vec2::new(rh.x - hs, rh.y - hs),
                    glam::Vec2::new(rh.x + hs, rh.y + hs),
                    theme::GIZMO_ROTATE_COLOR,
                    tr.z + theme::GIZMO_HANDLE_Z_BIAS,
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
            // egui took the pointer mid-gesture — in Docked mode, the cursor leaving the central
            // panel with the button held. What the drag applied so far stays on the entity, so
            // it is recorded rather than dropped; the release lands outside the panel and finds
            // nothing active.
            self.abandon_gesture();
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
                self.editor.gesture_entity = Some(sel);
                self.editor.rotate_start_rotation = tr.rotation;
                self.editor.rotate_start_angle = cursor_angle(tr.position, world_pos);
            } else if let Some(handle) = hit_test_handles(pos, tr.scale, world_pos) {
                self.editor.resize_handle_active = Some(handle);
                self.editor.gesture_entity = Some(sel);
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
                    self.editor.gesture_entity = Some(sel);
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
            self.commit_gesture(sel);
        }
    }

    /// Records whatever the active gesture has already applied to `e` as one undo command and
    /// clears the gesture. **The release handler and every abandon site go through here**, so a
    /// gesture ended by anything other than a release — the selection moving, egui taking the
    /// pointer, the component going away — is recorded exactly as a release would have recorded
    /// it. The change is already on the entity either way; the only choice is whether it is
    /// undoable (v0.155.8 made the same call for a tile-paint stroke).
    ///
    /// The kind of gesture is read off the flags and the kind of entity off its components: a
    /// `UiNode` gesture records `MoveUiNode` / `ResizeUiNode`, anything else `MoveEntity` /
    /// `ResizeEntity` / `RotateEntity`. A gone entity records nothing — there is nothing left to
    /// undo on it. No-op when no gesture is active.
    #[cfg(not(target_arch = "wasm32"))]
    fn commit_gesture(&mut self, e: crate::ecs::Entity) {
        use crate::components::Transform;
        use crate::ui::UiNode;
        let is_ui = self.world.get::<UiNode>(e).is_some();
        if self.editor.rotate_active {
            let old_rotation = self.editor.rotate_start_rotation;
            if let Some(new_rotation) = self.world.get::<Transform>(e).map(|t| t.rotation) {
                if (new_rotation - old_rotation).abs() > 1e-4 {
                    self.editor.cmd_history.push(EditorCmd::RotateEntity {
                        entity: e,
                        old_rotation,
                        new_rotation,
                    });
                }
            }
        } else if self.editor.resize_handle_active.is_some() {
            if is_ui {
                let old_offset = self.editor.resize_drag_start_offset;
                let old_size = self.editor.resize_drag_start_size;
                if let Some((new_offset, new_size)) =
                    self.world.get::<UiNode>(e).map(|n| (n.offset, n.size))
                {
                    if (new_offset - old_offset).length_squared() > 0.01
                        || (new_size - old_size).length_squared() > 0.01
                    {
                        self.editor.cmd_history.push(EditorCmd::ResizeUiNode {
                            entity: e,
                            old_offset,
                            old_size,
                            new_offset,
                            new_size,
                        });
                    }
                }
            } else {
                let old_scale = self.editor.resize_drag_start_scale;
                if let Some(new_scale) = self.world.get::<Transform>(e).map(|t| t.scale) {
                    if (new_scale - old_scale).length_squared() > 0.01 {
                        self.editor.cmd_history.push(EditorCmd::ResizeEntity {
                            entity: e,
                            old_scale,
                            new_scale,
                        });
                    }
                }
            }
        } else if self.editor.gizmo_dragging {
            if is_ui {
                let old_offset = self.editor.resize_drag_start_offset;
                if let Some(new_offset) = self.world.get::<UiNode>(e).map(|n| n.offset) {
                    if (new_offset - old_offset).length_squared() > 0.01 {
                        self.editor.cmd_history.push(EditorCmd::MoveUiNode {
                            entity: e,
                            old_offset,
                            new_offset,
                        });
                    }
                }
            } else {
                // The whole group: every selected entity moved by the same delta.
                let starts = std::mem::take(&mut self.editor.gizmo_drag_start_positions);
                for (entity, old_pos) in starts {
                    if let Some(new_pos) = self.world.get::<Transform>(entity).map(|t| t.position) {
                        if (new_pos - old_pos).length_squared() > 0.01 {
                            self.editor.cmd_history.push(EditorCmd::MoveEntity {
                                entity,
                                old_pos,
                                new_pos,
                            });
                        }
                    }
                }
            }
        }
        self.editor.clear_drag_state();
    }

    /// Ends a gesture that will never see its release: records it against the entity it started
    /// on ([`crate::app::editor::EditorState::gesture_entity`]) and clears the flags. Safe with no
    /// gesture active.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn abandon_gesture(&mut self) {
        match self.editor.gesture_entity {
            Some(e) => self.commit_gesture(e),
            None => self.editor.clear_drag_state(),
        }
    }

    /// wasm has no undo history, so abandoning a gesture is clearing it.
    #[cfg(target_arch = "wasm32")]
    pub(in crate::app) fn abandon_gesture(&mut self) {
        self.editor.clear_drag_state();
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

    /// A drag interrupted by the selection going away must not leave the gizmo deaf.
    ///
    /// `EditorCmd::CreateEntity` undo sets the selection to `None`, and Ctrl+Z is reachable with
    /// the mouse still held. `update_editor_gizmo` then returns at its `let Some(sel) … else`
    /// arm on every subsequent frame, so the release handler never runs again. Before
    /// `EditorState::clear_drag_state` that arm cleared `gizmo_dragging` and nothing else, so
    /// `rotate_active` survived — and the press guard in `update_transform_gizmo_native`
    /// requires all three flags clear, so every later gesture was silently refused.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn selection_lost_mid_drag_does_not_deafen_the_gizmo() {
        fn select_fresh(
            app: &mut crate::app::App,
        ) -> (crate::ecs::Entity, crate::components::Transform) {
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
            (e, tr)
        }

        let mut app = crate::app::App::new();
        let (e, tr) = select_fresh(&mut app);

        // Press the rotation handle at (0, -26) — a drag is now in progress.
        app.update_transform_gizmo_native(e, tr, glam::Vec2::new(0.0, -26.0), true, false, false);
        assert!(
            app.editor.rotate_active,
            "precondition: the rotation drag must actually have started, or the rest of this \
             test proves nothing"
        );

        // The selection vanishes mid-drag, and frames keep running.
        app.editor.inspector_selected = None;
        for _ in 0..5 {
            app.update_editor_gizmo(&None);
        }
        assert!(
            !app.editor.rotate_active,
            "the abandoned drag left `rotate_active` set — the release handler is unreachable \
             once the selection is gone, so nothing else will ever clear it"
        );

        // The observable consequence: a plain move press on a newly selected entity.
        let (e2, tr2) = select_fresh(&mut app);
        app.update_transform_gizmo_native(e2, tr2, glam::Vec2::ZERO, true, false, false);
        assert!(
            app.editor.gizmo_dragging,
            "a press well inside the AABB was refused — the stale flag is still gating the \
             press guard"
        );

        // Control: the identical press on a never-dragged App is accepted, so "refused" above
        // could not have meant "the press coordinates were wrong".
        let mut clean = crate::app::App::new();
        let (c, trc) = select_fresh(&mut clean);
        clean.update_transform_gizmo_native(c, trc, glam::Vec2::ZERO, true, false, false);
        assert!(
            clean.editor.gizmo_dragging,
            "control: this press must be accepted on a clean App, or the assertion above is \
             not measuring what it claims"
        );
    }

    // ── Gestures end on the entity they started on ────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_at(
        app: &mut crate::app::App,
        pos: glam::Vec2,
    ) -> (crate::ecs::Entity, crate::components::Transform) {
        let e = app.world.spawn();
        let tr = crate::components::Transform::new(pos, glam::Vec2::splat(20.0), 0.0);
        app.world.add_component(e, tr.clone());
        (e, tr)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn select(app: &mut crate::app::App, e: crate::ecs::Entity) {
        app.editor.inspector_selected = Some(e);
        app.editor.selected_entities = vec![e];
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn position(app: &crate::app::App, e: crate::ecs::Entity) -> glam::Vec2 {
        app.world
            .get::<crate::components::Transform>(e)
            .expect("entity has a Transform")
            .position
    }

    /// A gesture belongs to the entity it started on, not to whatever is selected when a later
    /// frame runs. Ctrl+Z is reachable mid-drag and every undo arm re-selects the entity it
    /// touched, so the selection can move from A to B with the button still held. The held
    /// branches read `sel` fresh: they applied A's captured start state to B, and the release
    /// recorded A's old value under B's id.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_gesture_ends_on_its_own_entity_when_the_selection_moves() {
        let mut app = crate::app::App::new();
        let (a, tra) = spawn_at(&mut app, glam::Vec2::ZERO);
        let (b, _) = spawn_at(&mut app, glam::Vec2::new(100.0, 0.0));
        select(&mut app, a);

        // Press A's body and drag it 10 units right.
        app.update_transform_gizmo_native(a, tra.clone(), glam::Vec2::ZERO, true, false, false);
        app.update_transform_gizmo_native(a, tra, glam::Vec2::new(10.0, 0.0), false, true, false);
        assert_eq!(
            position(&app, a),
            glam::Vec2::new(10.0, 0.0),
            "precondition: the drag moved A"
        );

        // The selection moves to B with the button still held — what an undo arm does.
        select(&mut app, b);
        app.update_editor_gizmo(&None);
        assert!(
            !app.editor.gesture_active(),
            "the gesture must end when its entity stops being the selection"
        );
        assert_eq!(
            position(&app, b),
            glam::Vec2::new(100.0, 0.0),
            "B was never part of the gesture"
        );

        // A's partial move was recorded — under A.
        assert_eq!(app.editor.cmd_history.undo_len(), 1);
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(
            position(&app, a),
            glam::Vec2::ZERO,
            "undo restores A, so the abandoned move was recorded against A"
        );
        assert_eq!(
            position(&app, b),
            glam::Vec2::new(100.0, 0.0),
            "and that undo does not touch B"
        );

        // A held frame on B now moves nothing: no gesture is active.
        let trb = app
            .world
            .get::<crate::components::Transform>(b)
            .cloned()
            .unwrap();
        app.update_transform_gizmo_native(
            b,
            trb.clone(),
            glam::Vec2::new(150.0, 0.0),
            false,
            true,
            false,
        );
        assert_eq!(
            position(&app, b),
            glam::Vec2::new(100.0, 0.0),
            "a held frame with no press behind it must not move B"
        );
        // Control: the same held frame after a press on B moves it, so the line above is not
        // passing because held frames never move anything.
        app.update_transform_gizmo_native(
            b,
            trb.clone(),
            glam::Vec2::new(100.0, 0.0),
            true,
            false,
            false,
        );
        app.update_transform_gizmo_native(b, trb, glam::Vec2::new(110.0, 0.0), false, true, false);
        assert_eq!(position(&app, b), glam::Vec2::new(110.0, 0.0), "control");
    }

    /// In Docked mode the game viewport is an egui panel, so a drag whose cursor leaves it hands
    /// the pointer to egui mid-gesture. The frames already applied stay on the entity, and the
    /// release lands outside the panel where nothing is active, so the move used to end with no
    /// undo entry — Ctrl+Z then reverted the *previous* command instead.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_docked_drag_that_leaves_the_panel_is_recorded_not_dropped() {
        let mut app = crate::app::App::new();
        let (a, tra) = spawn_at(&mut app, glam::Vec2::ZERO);
        select(&mut app, a);
        app.update_transform_gizmo_native(a, tra.clone(), glam::Vec2::ZERO, true, false, false);
        app.update_transform_gizmo_native(a, tra, glam::Vec2::new(10.0, 0.0), false, true, false);
        assert_eq!(
            position(&app, a),
            glam::Vec2::new(10.0, 0.0),
            "precondition"
        );

        app.editor.mode = crate::app::editor::EditorMode::Docked;
        app.editor.central_rect = Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(100.0, 100.0),
        ));

        // Control first: with the cursor still inside the panel nothing ends the gesture.
        app.editor.window_cursor = Some(egui::pos2(50.0, 50.0));
        app.update_editor_gizmo(&None);
        assert!(
            app.editor.gizmo_dragging,
            "control: inside the panel the gesture survives"
        );
        assert_eq!(
            app.editor.cmd_history.undo_len(),
            0,
            "control: nothing recorded yet"
        );

        // The cursor overshoots into the inspector.
        app.editor.window_cursor = Some(egui::pos2(500.0, 50.0));
        app.update_editor_gizmo(&None);
        assert!(
            !app.editor.gizmo_dragging,
            "egui took the pointer, so the gesture is over"
        );
        assert_eq!(
            app.editor.cmd_history.undo_len(),
            1,
            "the 10 units already applied are one undoable move"
        );
        let mut sel = app.editor.inspector_selected;
        app.editor.cmd_history.undo(&mut app.world, &mut sel);
        assert_eq!(position(&app, a), glam::Vec2::ZERO);
    }

    /// The `Transform` going away under a gesture is the third abandon site. It cleared
    /// `gizmo_dragging` alone, so a rotation or resize left its flag set and the press guard
    /// refused every later gesture — the v0.155.4 defect, on the one site that fix did not reach.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_gesture_whose_entity_lost_its_transform_is_cleared() {
        let mut app = crate::app::App::new();
        let (a, tra) = spawn_at(&mut app, glam::Vec2::ZERO);
        select(&mut app, a);
        app.update_transform_gizmo_native(a, tra, glam::Vec2::new(0.0, -26.0), true, false, false);
        assert!(
            app.editor.rotate_active,
            "precondition: the rotation drag started"
        );

        app.world
            .remove_component::<crate::components::Transform>(a);
        app.update_editor_gizmo(&None);
        assert!(
            !app.editor.rotate_active,
            "the Transform-gone arm must clear the rotation flag"
        );

        // Consequence: a fresh entity accepts a press.
        let (b, trb) = spawn_at(&mut app, glam::Vec2::ZERO);
        select(&mut app, b);
        app.update_transform_gizmo_native(b, trb, glam::Vec2::ZERO, true, false, false);
        assert!(
            app.editor.gizmo_dragging,
            "the press guard is no longer gated by the stale flag"
        );
    }
}
