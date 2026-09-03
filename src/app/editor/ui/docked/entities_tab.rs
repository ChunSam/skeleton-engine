//! Entities-list tab body.

use super::context_menu::{entity_context_menu, EntityContextAction};
use super::entity_kind::{entity_type_icon, sorted_entity_list};
use super::*;

/// Entity list tab body.  Shows a flat, multi-selectable list of all entities.
///
/// Used in: docked left panel (Entities tab), overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn entities_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    // Action row: + New Entity, Delete, Duplicate
    ui.horizontal(|ui| {
        if ui.button(tr("＋ New Entity", "＋ 새 엔티티")).clicked() {
            let e = app.world.spawn();
            app.world
                .add_component(e, crate::components::Transform::default());
            app.world
                .add_component(e, crate::prefab::Tag("New Entity".into()));
            app.editor.inspector_selected = Some(e);
            app.editor.selected_entities = vec![e];
            app.editor
                .cmd_history
                .push(super::super::EditorCmd::CreateEntity {
                    entity: e,
                    def: None,
                });
        }
        if let Some(sel) = app.editor.inspector_selected {
            if ui
                .add_enabled(true, egui::Button::new(tr("🗑 Delete", "🗑 삭제")))
                .clicked()
            {
                // Capture the whole subtree before despawning so undo restores every
                // component *and* the parent links (not just tag/transform/sprite).
                let defs = super::super::prefab::subtree_to_defs(&app.world, sel);
                app.editor
                    .cmd_history
                    .push(super::super::EditorCmd::DeleteEntity {
                        entities: None,
                        defs,
                    });
                app.editor_despawn_subtree(sel);
                app.editor.inspector_selected = None;
                // Descendants went with it, so filter by liveness rather than by `sel`.
                app.editor
                    .selected_entities
                    .retain(|&x| app.world.is_alive(x));
            }
            if ui
                .add_enabled(true, egui::Button::new(tr("⎘ Duplicate", "⎘ 복제")))
                .clicked()
            {
                // The same op as Ctrl+D, not a second spelling of it: this one copied only the
                // `register_clone`d components and dropped the parent (v0.156.23).
                app.editor.selected_entities = vec![sel];
                app.editor.inspector_selected = Some(sel);
                app.editor_duplicate_selection();
            }
        }
    });
    ui.separator();

    // Search box: filters the list by entity label (case-insensitive substring).
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.text_edit_singleline(&mut app.editor.entity_filter);
        if !app.editor.entity_filter.is_empty() && ui.small_button("✕").clicked() {
            app.editor.entity_filter.clear();
        }
    });

    // Sort control: order the displayed list by entity index (Default) / name (A–Z) / kind (Type).
    // Display-only — the world's entity order and scene-save order are untouched.
    ui.horizontal(|ui| {
        ui.label(tr("Sort:", "정렬:"));
        let mut mode = app.editor.entity_sort;
        for (m, en, ko) in [
            (EntitySortMode::Index, "Default", "기본"),
            (EntitySortMode::Name, "A–Z", "이름"),
            (EntitySortMode::Kind, "Type", "종류"),
        ] {
            if ui.selectable_label(mode == m, tr(en, ko)).clicked() {
                mode = m;
            }
        }
        app.editor.entity_sort = mode;
    });

    // Flat entity list with multi-select, ordered by the chosen sort mode (a display-only copy).
    let filter = app.editor.entity_filter.clone();
    let display = sorted_entity_list(entity_list, app.editor.entity_sort, &app.world, tag_map);
    // A right-click menu writes its chosen (entity, action) here; applied after the list is drawn
    // (collect-then-apply, so the menu closure never has to mutate `app` mid-iteration).
    let mut ctx_action: Option<(Entity, EntityContextAction)> = None;
    egui::ScrollArea::vertical()
        .id_salt("docked_ent_list")
        .show(ui, |ui| {
            for &e in &display {
                let label = entity_label(e, tag_map);
                if !super::super::entity_matches_filter(&label, &filter) {
                    continue;
                }
                let is_sel = app.editor.selected_entities.contains(&e);
                let hidden = app.world.get::<crate::components::Hidden>(e).is_some();
                let icon = entity_type_icon(&app.world, e);
                ui.horizontal(|ui| {
                    // Per-entity visibility toggle: a filled eye = visible, slashed = hidden. Adds /
                    // removes the `Hidden` component (the sprite pass skips Hidden entities).
                    let (eye, hint) = if hidden {
                        (
                            "\u{1F648}",
                            tr("hidden — click to show", "숨김 — 클릭하여 표시"),
                        )
                    } else {
                        (
                            "\u{1F441}",
                            tr("visible — click to hide", "표시 — 클릭하여 숨김"),
                        )
                    };
                    if ui.small_button(eye).on_hover_text(hint).clicked() {
                        if hidden {
                            app.world.remove_component::<crate::components::Hidden>(e);
                        } else {
                            app.world.add_component(e, crate::components::Hidden);
                        }
                    }
                    // Type glyph: a subtle per-row hint at the entity's kind (light / sprite /
                    // tilemap / UI / …), drawn between the eye toggle and the label.
                    ui.label(egui::RichText::new(icon).weak());
                    // Inline rename: while this row is being renamed, draw a focused text box in
                    // place of the label. Enter or clicking away commits (writes the Tag); Escape
                    // cancels. Otherwise draw the (selectable) label; a double-click starts a rename.
                    let renaming = matches!(&app.editor.entity_rename, Some(r) if r.entity == e);
                    if renaming {
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
                    } else {
                        // Dim the label of a hidden entity.
                        let label_rt = if hidden {
                            egui::RichText::new(&label).weak()
                        } else {
                            egui::RichText::new(&label)
                        };
                        let resp = ui
                            .selectable_label(is_sel, label_rt)
                            .on_hover_text(tr("double-click to rename", "더블클릭하여 이름 변경"));
                        // Right-click context menu: the same rename/duplicate/focus/delete ops as the
                        // toolbar + shortcuts, per-row and discoverable. The flat list offers no
                        // "Add child" (that's a Scene-tree-only op). Records the chosen action,
                        // applied after the list is drawn.
                        resp.context_menu(|ui| entity_context_menu(ui, e, false, &mut ctx_action));
                        if resp.clicked() {
                            apply_multiselect(
                                e,
                                ui.input(|i| i.modifiers.ctrl),
                                &mut app.editor.selected_entities,
                                &mut app.editor.inspector_selected,
                            );
                        }
                        if resp.double_clicked() {
                            app.editor_begin_rename(e);
                        }
                    }
                });
            }
        });
    // Apply the right-click menu action chosen this frame (collect-then-apply keeps the menu closure
    // free of `app` mutation during iteration).
    if let Some((e, action)) = ctx_action {
        app.editor_apply_entity_context_action(e, action);
    }
}
