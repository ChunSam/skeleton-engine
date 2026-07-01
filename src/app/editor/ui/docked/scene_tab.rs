//! Scene-tree tab body (drag-to-reparent + context menu).

use super::context_menu::{entity_context_menu, EntityContextAction};
use super::entity_kind::entity_type_icon;
use super::*;

/// egui drag-and-drop payload for the Scene-tree: the entity being dragged to a new parent.
/// `'static + Send + Sync` (a plain `Entity` id) as egui's `dnd_drag_source` requires.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
struct DragEntity(Entity);

/// Scene graph tab body.  Shows a parent→children indented tree.
///
/// Used in: docked left panel (Scene tab), overlay Inspector window (tab 2).
///
/// **Drag-to-reparent:** each tree node is an egui drag source AND a drop target — dragging a node
/// onto another re-parents it under that node; dragging onto the bottom "unparent" zone detaches it
/// to a root. Edits go through the cycle-safe [`App::editor_reparent`], so a drop that would create
/// a cycle (onto self or a descendant) is a no-op.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn scene_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    tag_map: &HashMap<Entity, String>,
    scene_graph_data: &[(Entity, Option<Entity>)],
    children_map: &HashMap<Entity, Vec<Entity>>,
    root_entities: &[Entity],
) {
    let _ = scene_graph_data; // used implicitly through root_entities + children_map

    let mut clicked_entity: Option<Entity> = None;
    let mut ctrl_clicked: bool = false;
    // A node drop sets `(dragged_child, Some(target_parent))`; the unparent zone sets `(_, None)`.
    let mut dropped: Option<(Entity, Option<Entity>)> = None;
    // A right-click menu choice (collect-then-apply, like `dropped` — applied after the tree is drawn).
    let mut ctx_action: Option<(Entity, EntityContextAction)> = None;

    egui::ScrollArea::vertical()
        .id_salt("docked_scene_graph")
        .show(ui, |ui| {
            let mut stack: Vec<(Entity, usize)> =
                root_entities.iter().rev().map(|&e| (e, 0)).collect();
            while let Some((entity, depth)) = stack.pop() {
                let name = entity_label(entity, tag_map);
                let is_selected = app.editor.selected_entities.contains(&entity);
                let has_children = children_map
                    .get(&entity)
                    .map(|c| !c.is_empty())
                    .unwrap_or(false);
                let prefix = if has_children { "▶ " } else { "  " };
                let icon = entity_type_icon(&app.world, entity);
                let indent = format!("{}{}{}", "  ".repeat(depth), prefix, icon);
                // Inline rename in the Scene tree: while this node is being renamed, draw a focused
                // text box (bound to the shared `entity_rename` buffer, same as the Entities list)
                // in place of the label — and NOT wrapped in a drag source, so typing/dragging in
                // the field never starts a reparent DnD. Otherwise draw the draggable node; a
                // double-click starts a rename via the shared `editor_begin_rename`.
                let renaming = matches!(&app.editor.entity_rename, Some(r) if r.entity == entity);
                if renaming {
                    ui.horizontal(|ui| {
                        ui.label(&indent);
                        let (mut commit, mut cancel) = (false, false);
                        if let Some(rn) = app.editor.entity_rename.as_mut() {
                            let resp = ui.text_edit_singleline(&mut rn.buffer);
                            if rn.focus_pending {
                                resp.request_focus();
                                rn.focus_pending = false;
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                cancel = true;
                            } else if resp.lost_focus() {
                                commit = true;
                            }
                        }
                        if commit {
                            app.editor_commit_rename();
                        } else if cancel {
                            app.editor_cancel_rename();
                        }
                    });
                } else {
                    let label_text = format!("{indent} {name}");
                    // Each node is a drag source (so it can be picked up) whose returned response
                    // also ORs in the inner selectable_label — so `.clicked()` still selects.
                    // `dnd_drag_source` sets the payload while dragged; `dnd_release_payload` fires
                    // when a drag is dropped over this node, making the node a drop target too.
                    let dnd_id = egui::Id::new(("scene_dnd", entity));
                    let response = ui
                        .dnd_drag_source(dnd_id, DragEntity(entity), |ui| {
                            // Inner response is unused — selection comes from the OR'd outer
                            // response below; this draws only the selection highlight.
                            let _ = ui.selectable_label(is_selected, &label_text);
                        })
                        .response;
                    if response.clicked() {
                        clicked_entity = Some(entity);
                        ctrl_clicked = ui.input(|i| i.modifiers.ctrl);
                    }
                    if response.double_clicked() {
                        app.editor_begin_rename(entity);
                    }
                    // Right-click a node → the shared entity menu, plus the Scene-tree-only "Add child"
                    // (spawns an entity parented under this node). Secondary-click doesn't disturb the
                    // primary-drag reparent source.
                    response
                        .context_menu(|ui| entity_context_menu(ui, entity, true, &mut ctx_action));
                    if let Some(payload) = response.dnd_release_payload::<DragEntity>() {
                        if payload.0 != entity {
                            dropped = Some((payload.0, Some(entity)));
                        }
                    }
                }
                if let Some(ch) = children_map.get(&entity) {
                    for &child in ch.iter().rev() {
                        stack.push((child, depth + 1));
                    }
                }
            }
        });

    if let Some(e) = clicked_entity {
        apply_multiselect(
            e,
            ctrl_clicked,
            &mut app.editor.selected_entities,
            &mut app.editor.inspector_selected,
        );
    }

    // Drop a node here (outside any parent) to detach it to a root.
    let unparent_frame = egui::Frame::default()
        .inner_margin(4.0)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke);
    let (_, root_payload) = ui.dnd_drop_zone::<DragEntity, _>(unparent_frame, |ui| {
        ui.label(tr("⤴ drop here to unparent", "⤴ 여기에 놓아 부모 해제"));
    });
    if let Some(payload) = root_payload {
        dropped = Some((payload.0, None));
    }

    if let Some((child, new_parent)) = dropped {
        app.editor_reparent(child, new_parent);
    }
    // Apply the right-click menu action chosen this frame (collect-then-apply keeps the menu closure
    // free of `app` mutation during iteration).
    if let Some((e, action)) = ctx_action {
        app.editor_apply_entity_context_action(e, action);
    }

    ui.separator();
    if let Some(sel) = app.editor.inspector_selected {
        tag_name_editor(ui, sel, tag_map, &mut app.world);
    } else {
        ui.label(tr("(no entity selected)", "(선택된 엔티티 없음)"));
    }
}
