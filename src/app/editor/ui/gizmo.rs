use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::{snap_to_grid, EditorCmd};

impl App {
    pub(in crate::app) fn update_editor_gizmo(&mut self, egui_ctx: &Option<egui::Context>) {
        // ── Gizmo: 선택 엔티티 강조 + 드래그 이동 ────────────────────────────────
        let egui_wants_mouse = egui_ctx
            .as_ref()
            .map(|c| c.wants_pointer_input())
            .unwrap_or(false);

        if let Some(sel) = self.inspector_selected {
            // 선택된 엔티티의 Transform을 복사 (borrow 해방)
            let tr_copy = self.world.get::<crate::components::Transform>(sel).cloned();

            if let Some(tr) = tr_copy {
                // 선택 강조: DebugDrawQueue에 테두리 사각형 추가
                if let Some(dq) = self.world.resource_mut::<DebugDrawQueue>() {
                    let half = tr.scale * 0.5;
                    // 외곽 강조 (3px 두께 효과: 약간 확장)
                    let margin = glam::Vec2::splat(3.0 / tr.scale.x.max(1.0) * tr.scale.x);
                    dq.items.push(DebugRect {
                        min: tr.position - half - margin,
                        max: tr.position + half + margin,
                        color: crate::color::Color::rgba(0.2, 0.85, 1.0, 0.65),
                        z: tr.z + 999.0,
                    });
                }

                // Gizmo 드래그 — egui가 마우스를 소비하지 않을 때만 동작
                if !egui_wants_mouse {
                    // 마우스 입력 + 카메라 좌표 변환 (짧은 borrow 블록)
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
                        if just_pressed && !self.gizmo_dragging {
                            let half = tr.scale * 0.5;
                            let hit = world_pos.x >= tr.position.x - half.x
                                && world_pos.x <= tr.position.x + half.x
                                && world_pos.y >= tr.position.y - half.y
                                && world_pos.y <= tr.position.y + half.y;
                            if hit {
                                self.gizmo_dragging = true;
                                self.gizmo_drag_offset = tr.position - world_pos;
                                #[cfg(not(target_arch = "wasm32"))]
                                {
                                    self.gizmo_drag_start_pos = Some(tr.position);
                                    // 그룹 이동 undo 기록용: 모든 선택 엔티티의 시작 위치 스냅샷.
                                    // sel 이 selected_entities 에 없을 수도 있으므로 보장한다.
                                    let mut starts: Vec<(Entity, glam::Vec2)> = Vec::new();
                                    let mut has_sel = false;
                                    for &e in &self.selected_entities {
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
                                    self.gizmo_drag_start_positions = starts;
                                }
                            }
                        }

                        if self.gizmo_dragging && held {
                            let new_pos = world_pos + self.gizmo_drag_offset;
                            #[cfg(not(target_arch = "wasm32"))]
                            let final_pos = if self.snap_enabled {
                                snap_to_grid(new_pos, self.snap_size)
                            } else {
                                new_pos
                            };
                            #[cfg(target_arch = "wasm32")]
                            let final_pos = new_pos;

                            // 드래그 엔티티의 이전 위치를 구해 delta 계산 후 그룹 이동
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let old_pos = self
                                    .world
                                    .get::<crate::components::Transform>(sel)
                                    .map(|t| t.position)
                                    .unwrap_or(final_pos);
                                let delta = final_pos - old_pos;
                                // 주 엔티티 이동
                                if let Some(t) =
                                    self.world.get_mut::<crate::components::Transform>(sel)
                                {
                                    t.position = final_pos;
                                }
                                // 나머지 선택 엔티티에 같은 delta 적용
                                let others: Vec<Entity> = self
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
                                // 그룹 이동 전체를 기록한다(기존엔 주 엔티티만 기록되어
                                // 나머지 선택 엔티티의 이동을 undo 할 수 없었다). 엔티티별로
                                // MoveEntity 를 쌓으므로 undo 가 엔티티 단위로 동작한다.
                                let starts = std::mem::take(&mut self.gizmo_drag_start_positions);
                                for (entity, start_pos) in starts {
                                    let new_pos = self
                                        .world
                                        .get::<crate::components::Transform>(entity)
                                        .map(|t| t.position)
                                        .unwrap_or(start_pos);
                                    if (new_pos - start_pos).length_squared() > 0.01 {
                                        self.cmd_history.push(EditorCmd::MoveEntity {
                                            entity,
                                            old_pos: start_pos,
                                            new_pos,
                                        });
                                    }
                                }
                                self.gizmo_drag_start_pos = None;
                            }
                            self.gizmo_dragging = false;
                        }
                    }
                } else {
                    self.gizmo_dragging = false;
                }
            }
        } else {
            self.gizmo_dragging = false;
        }
    }
}
