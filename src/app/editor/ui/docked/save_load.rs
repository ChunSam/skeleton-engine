//! Scene save / load controls.

use super::*;

/// Scene save controls (path text field + Save/Load buttons + status messages).
///
/// Used in: docked right panel footer, overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn save_load_controls(
    ui: &mut egui::Ui,
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(tr("Path:", "경로:"));
        ui.add(egui::TextEdit::singleline(&mut app.editor.editor_save_path).desired_width(160.0));
        if ui.button(tr("📂 Load", "📂 불러오기")).clicked() {
            do_load_scene(app);
        }
        if ui.button(tr("💾 Save", "💾 저장")).clicked() {
            do_save_scene_with_list(app, entity_list, tag_map);
        }
    });
    if let Some(msg) = &app.editor.editor_save_status {
        ui.small(msg.as_str());
    }
    if let Some(msg) = &app.editor.editor_load_status {
        ui.small(msg.as_str());
    }
}

// ── Save / Load helpers ──────────────────────────────────────────────────────

/// Execute a scene save using the current `entity_list` and `tag_map`.
///
/// Uses a topological sort so parents appear before children in the RON output.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_save_scene_with_list(
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    let mut scene_def = crate::prefab::SceneDef::default();
    let sorted = crate::hierarchy::topological_sort_entities(entity_list, &app.world);
    let mut dropped_parent_links: u32 = 0;
    for &e in &sorted {
        let tag = app.world.get::<crate::prefab::Tag>(e).map(|t| t.0.clone());
        let transform = app.world.get::<crate::components::Transform>(e).cloned();
        let sprite = app.world.get::<crate::components::Sprite>(e).cloned();
        let parent_entity = app.world.get::<crate::hierarchy::Parent>(e).map(|p| p.0);
        let parent = parent_entity.and_then(|p| tag_map.get(&p)).cloned();
        // Warn when a parent exists but has no tag — the hierarchy link cannot
        // be represented in the RON format and will be silently lost on reload.
        if parent_entity.is_some() && parent.is_none() {
            let child_name = tag.as_deref().unwrap_or("(untagged)");
            log::warn!(
                "scene save: parent of '{}' has no Tag — parent link dropped (will not restore on load)",
                child_name
            );
            dropped_parent_links += 1;
        }
        let components = app
            .world
            .resource::<crate::prefab::SerdeComponentRegistry>()
            .map(|r| r.serialize_entity(&app.world, e))
            .unwrap_or_default();
        if tag.is_some() || transform.is_some() || sprite.is_some() || !components.is_empty() {
            scene_def.entities.push(crate::prefab::EntityDef {
                tag,
                transform,
                sprite,
                parent,
                components,
            });
        }
    }
    let count = scene_def.entities.len();
    let path = app.editor.editor_save_path.clone();
    let save_result = scene_def.save(std::path::Path::new(&path));
    // Surface the outcome as a toast (covers both the toolbar button and Ctrl+S).
    match &save_result {
        Ok(()) => app.push_editor_toast(
            format!("{} ({count})", tr("Scene saved", "씬 저장됨")),
            crate::app::editor::state::ToastKind::Success,
        ),
        Err(e) => app.push_editor_toast(
            format!("{}: {e}", tr("Save failed", "저장 실패")),
            crate::app::editor::state::ToastKind::Error,
        ),
    }
    app.editor.editor_save_status = match save_result {
        Ok(()) => {
            if dropped_parent_links > 0 {
                Some(format!(
                    "✓ {count} {} → {path} ({dropped_parent_links} {})",
                    tr("entities", "엔티티"),
                    tr(
                        "parent link(s) dropped: untagged parent",
                        "부모 링크 누락: 태그 없는 부모"
                    ),
                ))
            } else {
                Some(format!("✓ {count} {} → {path}", tr("entities", "엔티티")))
            }
        }
        Err(e) => Some(format!("✗ {e}")),
    };
    app.editor.editor_load_status = None;
}

/// Execute a scene save without an explicit entity list (queries the world).
///
/// Used by the toolbar "💾 Save" which runs before the entity list is built.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_save_scene(app: &mut App) {
    let entity_list: Vec<Entity> = app.world.entities().to_vec();
    let tag_map: HashMap<Entity, String> = app
        .world
        .query::<Tag>()
        .map(|(e, t)| (e, t.0.clone()))
        .collect();
    do_save_scene_with_list(app, &entity_list, &tag_map);
}

/// Execute a scene load from `editor.editor_save_path`.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_load_scene(app: &mut App) {
    let path_str = app.editor.editor_save_path.clone();
    let path = std::path::Path::new(&path_str);
    match crate::prefab::SceneDef::load(path) {
        Ok(scene_def) => {
            // Despawn ALL current entities before loading so that UiNode-only
            // entities (menus/HUD without a Transform) don't accumulate on reload.
            let to_remove: Vec<Entity> = app.world.entities().to_vec();
            for e in to_remove {
                app.world.despawn(e);
            }
            app.editor.inspector_selected = None;
            app.editor.selected_entities.clear();
            let count = scene_def.entities.len();
            crate::prefab::spawn_scene_def(&mut app.world, &scene_def);
            app.editor.editor_load_status = Some(format!(
                "✓ {count} {} ← {path_str}",
                tr("entities", "엔티티")
            ));
            app.editor.editor_save_status = None;
        }
        Err(e) => {
            app.editor.editor_load_status = Some(format!("✗ {e}"));
        }
    }
}

// reflect_value_editor is defined in super (ui/mod.rs) without a cfg gate,
// so both native and wasm can call it.  docked.rs calls it via `super::reflect_value_editor`
// through `use super::*` at the top of this file.
