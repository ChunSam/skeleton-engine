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
    ui_drag_new_offset, ui_resize_new_layout, HANDLE_HIT_RADIUS, HANDLE_SIZE, MIN_SPRITE_SCALE,
    MIN_UI_SIZE, ROT_HANDLE_GAP, ROT_HIT_RADIUS, ROT_SNAP,
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
            // Logical pixels throughout in the UI gizmo, so the constant is used as it stands.
            if let Some(handle) = hit_test_handles(screen_pos, node.size, cursor, HANDLE_HIT_RADIUS)
            {
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
                    snap_to_grid(new_size, self.editor.snap_size)
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
        // The transform the entity is DRAWN at (releases the borrow). For a parented entity
        // that is its `GlobalTransform`; its own `Transform` is only the offset from the
        // parent, and hit-testing there put the box near the world origin while the sprite
        // drew on the parent — a press on what you can see started nothing (v0.156.18).
        let tr_copy = self.editor_world_transform(sel);

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
                let zoom = cam.safe_zoom();
                self.world
                    .resource::<crate::input::InputState>()
                    .map(|inp| {
                        let world_pos = cam.screen_to_world(inp.cursor());
                        let pressed = inp.mouse_just_pressed(winit::event::MouseButton::Left);
                        let held = inp.is_mouse_pressed(winit::event::MouseButton::Left);
                        let released = inp.mouse_just_released(winit::event::MouseButton::Left);
                        (world_pos, pressed, held, released, zoom)
                    })
            };

            // `zoom` sizes the handle hit radii, which only the native gizmo has.
            #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
            if let Some((world_pos, just_pressed, held, just_released, zoom)) = gizmo_input {
                // Native path: supports group-move, snap, undo, and resize.
                #[cfg(not(target_arch = "wasm32"))]
                self.update_transform_gizmo_native(
                    sel,
                    tr,
                    world_pos,
                    just_pressed,
                    held,
                    just_released,
                    zoom,
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
                        // Through the parent, like the native path: `new_pos` is world-space.
                        self.editor_set_world_position(sel, new_pos);
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

    /// `zoom` is the camera's, so the handle hit radii can be a constant number of screen
    /// pixels rather than of world units — see [`HANDLE_HIT_RADIUS`].
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub(in crate::app) fn update_transform_gizmo_native(
        &mut self,
        sel: crate::ecs::Entity,
        tr: crate::components::Transform,
        world_pos: glam::Vec2,
        just_pressed: bool,
        held: bool,
        just_released: bool,
        zoom: f32,
    ) {
        // The radii are logical pixels; at this zoom they are this many world units.
        let handle_radius = HANDLE_HIT_RADIUS / zoom;
        let rot_radius = ROT_HIT_RADIUS / zoom;
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
            if (world_pos - rot_handle).length() <= rot_radius {
                self.editor.rotate_active = true;
                self.editor.gesture_entity = Some(sel);
                self.editor.rotate_start_rotation = tr.rotation;
                self.editor.rotate_start_angle = cursor_angle(tr.position, world_pos);
            } else if let Some(handle) = hit_test_handles(pos, tr.scale, world_pos, handle_radius) {
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
                    for &e in self.editor.selected_entities.clone().iter() {
                        if let Some(t) = self.editor_world_transform(e) {
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
                self.editor_set_world_rotation(sel, new_rot);
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
                // ⚠️ A negative `Transform.scale` is not supported here, and the editor does not
                // offer one: `MIN_SPRITE_SCALE` floors every arm, so the first frame of a resize
                // on a flipped sprite un-flips it to 2 px. `Transform`'s own doc steers mirroring
                // to `SpriteFlip` and warns that a negative scale breaks rotation, so this floor
                // is the policy rather than an oversight — only the highlight above anticipates
                // one (`scale.abs()` on the rotation handle). Recorded rather than fixed
                // (2026-09-03); the resize is undoable, so the cost is one Ctrl+Z.
                let final_scale = if self.editor.snap_enabled {
                    snap_to_grid(new_scale, self.editor.snap_size)
                        .max(glam::Vec2::splat(MIN_SPRITE_SCALE))
                } else {
                    new_scale
                };
                self.editor_set_world_scale(sel, final_scale);
            } else if self.editor.gizmo_dragging {
                let new_pos = world_pos + self.editor.gizmo_drag_offset;
                let final_pos = if self.editor.snap_enabled {
                    snap_to_grid(new_pos, self.editor.snap_size)
                } else {
                    new_pos
                };
                // The delta is world-space, and so is every entity's position below: a group
                // member under a rotated or scaled parent cannot take a world delta on its
                // local offset.
                let old_pos = self
                    .editor_world_transform(sel)
                    .map(|t| t.position)
                    .unwrap_or(final_pos);
                let delta = final_pos - old_pos;
                self.editor_set_world_position(sel, final_pos);
                let others: Vec<crate::ecs::Entity> = self
                    .editor
                    .selected_entities
                    .iter()
                    .copied()
                    .filter(|&e| e != sel)
                    .collect();
                for other in others {
                    if let Some(t) = self.editor_world_transform(other) {
                        self.editor_set_world_position(other, t.position + delta);
                    }
                }
            }
        }

        // ── Release ───────────────────────────────────────────────────────────
        if just_released {
            self.commit_gesture(sel);
        }
    }

    /// The transform `e` is **drawn** at: its `GlobalTransform` when the hierarchy has composed
    /// one, else its own `Transform`. The renderer's policy (`renderer/sprite/collect.rs`) and
    /// the collision grid's, so the gizmo grabs what the eye sees. Compiled on wasm too — the
    /// move-only gizmo there drags the same parented entities.
    pub(in crate::app) fn editor_world_transform(
        &self,
        e: crate::ecs::Entity,
    ) -> Option<crate::components::Transform> {
        let local = self.world.get::<crate::components::Transform>(e)?;
        Some(
            match self.world.get::<crate::hierarchy::GlobalTransform>(e) {
                Some(g) => crate::components::Transform {
                    position: g.position,
                    scale: g.scale,
                    rotation: g.rotation,
                    z: g.z,
                },
                None => local.clone(),
            },
        )
    }

    /// The parent's world transform, when `e` has a live parent the hierarchy has composed.
    fn editor_parent_world(
        &self,
        e: crate::ecs::Entity,
    ) -> Option<crate::hierarchy::GlobalTransform> {
        let parent = self.world.get::<crate::hierarchy::Parent>(e)?.0;
        self.world
            .get::<crate::hierarchy::GlobalTransform>(parent)
            .copied()
    }

    /// Writes a **world** position onto `e`'s local `Transform`, inverting the parent's transform
    /// when it has one. Exact: the same matrix `hierarchy::compose` multiplies by, inverted.
    fn editor_set_world_position(&mut self, e: crate::ecs::Entity, world_pos: glam::Vec2) {
        let local = match self.editor_parent_world(e) {
            Some(p) => {
                let v =
                    p.to_matrix().inverse() * glam::Vec4::new(world_pos.x, world_pos.y, 0.0, 1.0);
                glam::Vec2::new(v.x, v.y)
            }
            None => world_pos,
        };
        if let Some(t) = self.world.get_mut::<crate::components::Transform>(e) {
            t.position = local;
        }
    }

    /// Writes a **world** scale onto `e`'s local `Transform`, dividing out the parent's.
    ///
    /// ⚠️ Component-wise, which is exact only while the parent is unrotated or scaled uniformly.
    /// A rotated parent with non-uniform scale shears, and `GlobalTransform` cannot represent
    /// shear either — `hierarchy::compose` decomposes the product the same lossy way. A zero
    /// parent scale leaves the local scale alone rather than producing an infinity.
    #[cfg(not(target_arch = "wasm32"))]
    fn editor_set_world_scale(&mut self, e: crate::ecs::Entity, world_scale: glam::Vec2) {
        let local = match self.editor_parent_world(e) {
            Some(p) if p.scale.x.abs() > 1e-6 && p.scale.y.abs() > 1e-6 => world_scale / p.scale,
            Some(_) => return,
            None => world_scale,
        };
        if let Some(t) = self.world.get_mut::<crate::components::Transform>(e) {
            t.scale = local;
        }
    }

    /// Writes a **world** rotation onto `e`'s local `Transform`, subtracting the parent's.
    #[cfg(not(target_arch = "wasm32"))]
    fn editor_set_world_rotation(&mut self, e: crate::ecs::Entity, world_rotation: f32) {
        let local = match self.editor_parent_world(e) {
            Some(p) => world_rotation - p.rotation,
            None => world_rotation,
        };
        if let Some(t) = self.world.get_mut::<crate::components::Transform>(e) {
            t.rotation = local;
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
        use crate::ui::UiNode;
        let is_ui = self.world.get::<UiNode>(e).is_some();
        if self.editor.rotate_active {
            let old_rotation = self.editor.rotate_start_rotation;
            // World values throughout: the gesture was captured from the drawn transform.
            if let Some(new_rotation) = self.editor_world_transform(e).map(|t| t.rotation) {
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
                if let Some(new_scale) = self.editor_world_transform(e).map(|t| t.scale) {
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
                    if let Some(new_pos) = self.editor_world_transform(entity).map(|t| t.position) {
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
            1.0,
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
            1.0,
        );
        let rot = app
            .world
            .get::<crate::components::Transform>(e)
            .unwrap()
            .rotation;
        assert!((rot - PI / 2.0).abs() < 1e-3, "rotated ~90°, got {rot}");

        // Release commits the RotateEntity command.
        app.update_transform_gizmo_native(
            e,
            tr,
            glam::Vec2::new(26.0, 0.0),
            false,
            false,
            true,
            1.0,
        );
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
        app.update_transform_gizmo_native(
            e,
            tr,
            glam::Vec2::new(0.0, -26.0),
            true,
            false,
            false,
            1.0,
        );
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
        app.update_transform_gizmo_native(e2, tr2, glam::Vec2::ZERO, true, false, false, 1.0);
        assert!(
            app.editor.gizmo_dragging,
            "a press well inside the AABB was refused — the stale flag is still gating the \
             press guard"
        );

        // Control: the identical press on a never-dragged App is accepted, so "refused" above
        // could not have meant "the press coordinates were wrong".
        let mut clean = crate::app::App::new();
        let (c, trc) = select_fresh(&mut clean);
        clean.update_transform_gizmo_native(c, trc, glam::Vec2::ZERO, true, false, false, 1.0);
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
        app.update_transform_gizmo_native(
            a,
            tra.clone(),
            glam::Vec2::ZERO,
            true,
            false,
            false,
            1.0,
        );
        app.update_transform_gizmo_native(
            a,
            tra,
            glam::Vec2::new(10.0, 0.0),
            false,
            true,
            false,
            1.0,
        );
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
            1.0,
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
            1.0,
        );
        app.update_transform_gizmo_native(
            b,
            trb,
            glam::Vec2::new(110.0, 0.0),
            false,
            true,
            false,
            1.0,
        );
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
        app.update_transform_gizmo_native(
            a,
            tra.clone(),
            glam::Vec2::ZERO,
            true,
            false,
            false,
            1.0,
        );
        app.update_transform_gizmo_native(
            a,
            tra,
            glam::Vec2::new(10.0, 0.0),
            false,
            true,
            false,
            1.0,
        );
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
        app.update_transform_gizmo_native(
            a,
            tra,
            glam::Vec2::new(0.0, -26.0),
            true,
            false,
            false,
            1.0,
        );
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
        app.update_transform_gizmo_native(b, trb, glam::Vec2::ZERO, true, false, false, 1.0);
        assert!(
            app.editor.gizmo_dragging,
            "the press guard is no longer gated by the stale flag"
        );
    }

    // ── A parented entity is grabbed where it is drawn ────────────────────────

    /// P at (500, 300), C local (16, 0) under it, so C draws at (516, 300). Everything the
    /// gizmo does with `sel` starts from that composed transform.
    #[cfg(not(target_arch = "wasm32"))]
    fn parented_pair(
        app: &mut crate::app::App,
        parent_rotation: f32,
        parent_scale: glam::Vec2,
    ) -> (crate::ecs::Entity, crate::ecs::Entity) {
        let p = app.world.spawn();
        app.world.add_component(
            p,
            crate::components::Transform::new(
                glam::Vec2::new(500.0, 300.0),
                parent_scale,
                parent_rotation,
            ),
        );
        let c = app.world.spawn();
        app.world.add_component(
            c,
            crate::components::Transform::new(
                glam::Vec2::new(16.0, 0.0),
                glam::Vec2::splat(20.0),
                0.0,
            ),
        );
        assert!(crate::hierarchy::reparent(&mut app.world, c, Some(p)));
        hierarchy_pass(app);
        (p, c)
    }

    /// One `HierarchySystem` pass, which is what recomputes `GlobalTransform`. A drag writes the
    /// LOCAL transform; the world one it is drawn at only catches up on the next frame's pass, so
    /// a test that asserts a world position after a drag has to run this the way a frame does.
    #[cfg(not(target_arch = "wasm32"))]
    fn hierarchy_pass(app: &mut crate::app::App) {
        use crate::ecs::System;
        crate::hierarchy::HierarchySystem::default().run(&mut app.world, 0.0);
    }

    /// The gizmo hit-tested and highlighted a parented entity at its LOCAL `Transform.position`,
    /// which is only its offset from the parent — so the box sat near the world origin while the
    /// sprite drew on the parent, and a press on what you can see started nothing.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_parented_entity_is_grabbed_where_it_is_drawn() {
        let mut app = crate::app::App::new();
        let (_, c) = parented_pair(&mut app, 0.0, glam::Vec2::ONE);
        select(&mut app, c);
        let drawn = app
            .editor_world_transform(c)
            .expect("the child has a transform");
        assert_eq!(
            drawn.position,
            glam::Vec2::new(516.0, 300.0),
            "precondition: the child is drawn on the parent, not at its offset"
        );

        // A press on the sprite the eye sees.
        app.update_transform_gizmo_native(
            c,
            drawn.clone(),
            drawn.position,
            true,
            false,
            false,
            1.0,
        );
        assert!(
            app.editor.gizmo_dragging,
            "a press on the drawn sprite must start the drag"
        );

        // Dragging 10 to the right puts it there in WORLD space, and its local offset is what
        // the parent-relative value has to be.
        app.update_transform_gizmo_native(
            c,
            drawn,
            glam::Vec2::new(526.0, 300.0),
            false,
            true,
            false,
            1.0,
        );
        hierarchy_pass(&mut app);
        assert_eq!(
            app.editor_world_transform(c).unwrap().position,
            glam::Vec2::new(526.0, 300.0),
            "the drag lands under the cursor once the frame's hierarchy pass runs"
        );
        assert_eq!(
            app.world
                .get::<crate::components::Transform>(c)
                .unwrap()
                .position,
            glam::Vec2::new(26.0, 0.0),
            "and what is written is the local offset, not the world position"
        );
    }

    /// The parent's rotation and scale go through the same inverse. A quarter turn and a doubling
    /// make the local offset something a subtraction could not produce.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_drag_under_a_rotated_scaled_parent_writes_the_right_local_offset() {
        use std::f32::consts::FRAC_PI_2;
        let mut app = crate::app::App::new();
        let (_, c) = parented_pair(&mut app, FRAC_PI_2, glam::Vec2::splat(2.0));
        select(&mut app, c);
        let drawn = app.editor_world_transform(c).unwrap();

        // Put it at a world position 40 above the parent (engine Y is down, so -40).
        let target = glam::Vec2::new(500.0, 260.0);
        app.update_transform_gizmo_native(
            c,
            drawn.clone(),
            drawn.position,
            true,
            false,
            false,
            1.0,
        );
        app.update_transform_gizmo_native(c, drawn, target, false, true, false, 1.0);

        hierarchy_pass(&mut app);
        let got = app.editor_world_transform(c).unwrap().position;
        assert!(
            (got - target).length() < 1e-3,
            "the drag lands where the cursor is: {got:?} vs {target:?}"
        );
        // parent = rotate(+90°) ∘ scale(2). Inverting: local = R(-90°) * (world - p) / 2
        // = R(-90°) * (0, -40) / 2 = (-20, 0).
        let local = app
            .world
            .get::<crate::components::Transform>(c)
            .unwrap()
            .position;
        assert!(
            (local - glam::Vec2::new(-20.0, 0.0)).length() < 1e-3,
            "the local offset inverts the parent's rotation AND scale: {local:?}"
        );
    }

    /// The half the two tests above cannot reach: they hand `update_transform_gizmo_native` a
    /// transform, so the **read** is theirs, not the gizmo's. This one presses a real mouse
    /// through `update_editor_gizmo`, which is where the gizmo decides what transform to grab.
    ///
    /// The control is the point: a press at the child's LOCAL offset — where the box used to be
    /// drawn, near the world origin — must now start nothing.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn the_gizmo_grabs_at_the_drawn_position_not_the_local_one() {
        let press_at = |app: &mut crate::app::App, x: f32, y: f32| {
            app.editor.clear_drag_state();
            let inp = app
                .world
                .resource_mut::<crate::input::InputState>()
                .expect("App::new inserts InputState");
            inp.flush();
            inp.set_cursor(glam::Vec2::new(x, y));
            inp.press_mouse(winit::event::MouseButton::Left);
            app.update_editor_gizmo(&None);
            app.editor.gizmo_dragging
        };

        let mut app = crate::app::App::new();
        // Identity screen→world.
        app.world
            .insert_resource(crate::camera::Camera::new(glam::Vec2::ZERO, 1.0));
        app.editor.mode = crate::app::editor::EditorMode::Overlay;
        let (_, c) = parented_pair(&mut app, 0.0, glam::Vec2::ONE);
        select(&mut app, c);

        assert!(
            press_at(&mut app, 516.0, 300.0),
            "a press on the sprite the eye sees must start the drag"
        );
        assert!(
            !press_at(&mut app, 16.0, 0.0),
            "and a press at the local offset — where the box used to be — must not"
        );

        // Control: for an unparented entity the two are the same place, and it still grabs.
        let mut app = crate::app::App::new();
        app.world
            .insert_resource(crate::camera::Camera::new(glam::Vec2::ZERO, 1.0));
        app.editor.mode = crate::app::editor::EditorMode::Overlay;
        let (a, _) = spawn_at(&mut app, glam::Vec2::new(16.0, 0.0));
        select(&mut app, a);
        assert!(
            press_at(&mut app, 16.0, 0.0),
            "control: an unparented entity is grabbed at its own position"
        );
    }

    /// The pure `hit_test_handles` test cannot see this: the gizmo is what divides the radius by
    /// the camera zoom, and a press has to travel through `update_editor_gizmo` to reach that.
    /// At zoom 1 a 16×16 entity is all handle; at zoom 4 its middle is a move region.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_small_entity_can_be_dragged_once_zoomed_in() {
        let press_centre = |zoom: f32| -> bool {
            let mut app = crate::app::App::new();
            app.world
                .insert_resource(crate::camera::Camera::new(glam::Vec2::ZERO, zoom));
            app.editor.mode = crate::app::editor::EditorMode::Overlay;
            let e = app.world.spawn();
            app.world.add_component(
                e,
                crate::components::Transform::new(glam::Vec2::ZERO, glam::Vec2::splat(16.0), 0.0),
            );
            select(&mut app, e);
            let inp = app
                .world
                .resource_mut::<crate::input::InputState>()
                .expect("App::new inserts InputState");
            inp.flush();
            // The entity's centre is the world origin, which is screen (0,0) at any zoom here.
            inp.set_cursor(glam::Vec2::ZERO);
            inp.press_mouse(winit::event::MouseButton::Left);
            app.update_editor_gizmo(&None);
            app.editor.gizmo_dragging
        };

        assert!(
            !press_centre(1.0),
            "precondition: at zoom 1 the handles swallow a 16-unit entity, which is the defect"
        );
        assert!(
            press_centre(4.0),
            "zoomed in, the radius is worth fewer world units and the middle is grabbable"
        );
    }

    /// Control: an unparented entity is unchanged by all of this — its world transform is its
    /// own, and the drag writes the world position straight through.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn an_unparented_entity_still_drags_in_world_space() {
        let mut app = crate::app::App::new();
        let (a, tra) = spawn_at(&mut app, glam::Vec2::new(100.0, 100.0));
        select(&mut app, a);
        app.update_transform_gizmo_native(
            a,
            tra.clone(),
            glam::Vec2::new(100.0, 100.0),
            true,
            false,
            false,
            1.0,
        );
        app.update_transform_gizmo_native(
            a,
            tra,
            glam::Vec2::new(140.0, 100.0),
            false,
            true,
            false,
            1.0,
        );
        assert_eq!(position(&app, a), glam::Vec2::new(140.0, 100.0));
    }
}
