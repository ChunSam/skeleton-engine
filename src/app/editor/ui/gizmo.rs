use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::{snap_to_grid, EditorCmd};

impl App {
    pub(in crate::app) fn update_editor_gizmo(&mut self, egui_ctx: &Option<egui::Context>) {
        // ── Gizmo: highlight selected entity + drag to move ──────────────────
        let egui_wants_mouse = egui_ctx
            .as_ref()
            .map(|c| c.wants_pointer_input())
            .unwrap_or(false);

        if let Some(sel) = self.editor.inspector_selected {
            // Copy the selected entity's Transform (releases the borrow).
            let tr_copy = self.world.get::<crate::components::Transform>(sel).cloned();

            if let Some(tr) = tr_copy {
                // Selection highlight: add an outline rectangle to the DebugDrawQueue.
                if let Some(dq) = self.world.resource_mut::<DebugDrawQueue>() {
                    let half = tr.scale * 0.5;
                    // Outline highlight (3 px thickness effect: expand slightly).
                    let margin = glam::Vec2::splat(3.0 / tr.scale.x.max(1.0) * tr.scale.x);
                    dq.items.push(DebugRect {
                        min: tr.position - half - margin,
                        max: tr.position + half + margin,
                        color: crate::color::Color::rgba(0.2, 0.85, 1.0, 0.65),
                        z: tr.z + 999.0,
                    });
                }

                // Gizmo drag — only when egui is not consuming mouse input.
                if !egui_wants_mouse {
                    // Mouse input + camera coordinate transform (short borrow block).
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
                                let pressed =
                                    inp.mouse_just_pressed(winit::event::MouseButton::Left);
                                let held = inp.is_mouse_pressed(winit::event::MouseButton::Left);
                                let released =
                                    inp.mouse_just_released(winit::event::MouseButton::Left);
                                (world_pos, pressed, held, released)
                            })
                    };

                    if let Some((world_pos, just_pressed, held, just_released)) = gizmo_input {
                        if just_pressed && !self.editor.gizmo_dragging {
                            let half = tr.scale * 0.5;
                            let hit = world_pos.x >= tr.position.x - half.x
                                && world_pos.x <= tr.position.x + half.x
                                && world_pos.y >= tr.position.y - half.y
                                && world_pos.y <= tr.position.y + half.y;
                            if hit {
                                self.editor.gizmo_dragging = true;
                                self.editor.gizmo_drag_offset = tr.position - world_pos;
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    self.editor.gizmo_drag_start_pos = Some(tr.position);
                                    // Snapshot start positions of all selected entities for group-move undo.
                                    // Ensure sel is included even if absent from selected_entities.
                                    let mut starts: Vec<(Entity, glam::Vec2)> = Vec::new();
                                    let mut has_sel = false;
                                    for &e in &self.editor.selected_entities {
                                        if let Some(t) =
                                            self.world.get::<crate::components::Transform>(e)
                                        {
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

                        if self.editor.gizmo_dragging && held {
                            let new_pos = world_pos + self.editor.gizmo_drag_offset;
                            #[cfg(not(target_arch = "wasm32"))]
                            let final_pos = if self.editor.snap_enabled {
                                snap_to_grid(new_pos, self.editor.snap_size)
                            } else {
                                new_pos
                            };
                            #[cfg(target_arch = "wasm32")]
                            let final_pos = new_pos;

                            // Get the drag entity's previous position, compute delta, then move the group.
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let old_pos = self
                                    .world
                                    .get::<crate::components::Transform>(sel)
                                    .map(|t| t.position)
                                    .unwrap_or(final_pos);
                                let delta = final_pos - old_pos;
                                // Move the primary entity.
                                if let Some(t) =
                                    self.world.get_mut::<crate::components::Transform>(sel)
                                {
                                    t.position = final_pos;
                                }
                                // Apply the same delta to the remaining selected entities.
                                let others: Vec<Entity> = self
                                    .editor
                                    .selected_entities
                                    .iter()
                                    .copied()
                                    .filter(|&e| e != sel)
                                    .collect();
                                for other in others {
                                    if let Some(t) =
                                        self.world.get_mut::<crate::components::Transform>(other)
                                    {
                                        t.position += delta;
                                    }
                                }
                            }
                            #[cfg(target_arch = "wasm32")]
                            if let Some(t) = self.world.get_mut::<crate::components::Transform>(sel)
                            {
                                t.position = final_pos;
                            }
                        }

                        if just_released {
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                // Record the entire group move (previously only the primary
                                // entity was recorded, making the other selections un-undoable).
                                // Push one MoveEntity per entity so undo operates entity-by-entity.
                                let starts =
                                    std::mem::take(&mut self.editor.gizmo_drag_start_positions);
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
                            }
                            self.editor.gizmo_dragging = false;
                        }
                    }
                } else {
                    self.editor.gizmo_dragging = false;
                }
            }
        } else {
            self.editor.gizmo_dragging = false;
        }
    }
}
