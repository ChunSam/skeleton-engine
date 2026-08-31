use super::tr;
use crate::app::App;
use crate::ecs::{Entity, World};

/// Capture `root` and its whole subtree as defs, **parents before children**, each carrying the
/// index *into the returned list* of its parent (`None` for `root` itself).
///
/// The index is what makes undo-of-delete restore the hierarchy exactly. `EntityDef.parent` is a
/// **tag**, so it cannot express a parent that has no `Tag` and misroutes one whose tag is shared
/// — the "parent link dropped" case the scene saver warns about. Within a single delete the real
/// structure is known, so it is recorded directly. `root`'s own parent lies outside the subtree
/// and stays tag-based in its `EntityDef.parent`.
pub(in crate::app) fn subtree_to_defs(
    world: &World,
    root: Entity,
) -> Vec<(crate::prefab::EntityDef, Option<usize>)> {
    let order: Vec<Entity> = std::iter::once(root)
        .chain(crate::hierarchy::descendants(world, root))
        .collect();
    let index_of = |e: Entity| order.iter().position(|&o| o == e);
    order
        .iter()
        .map(|&e| {
            let parent_ix = world
                .get::<crate::hierarchy::Parent>(e)
                .and_then(|p| index_of(p.0));
            (entity_to_def(world, e).unwrap_or_default(), parent_ix)
        })
        .collect()
}

pub(in crate::app) fn entity_to_def(
    world: &World,
    entity: Entity,
) -> Option<crate::prefab::EntityDef> {
    let components = world
        .resource::<crate::prefab::SerdeComponentRegistry>()
        .map(|r| r.serialize_entity(world, entity))
        .unwrap_or_default();
    Some(crate::prefab::EntityDef {
        tag: world.get::<crate::prefab::Tag>(entity).map(|t| t.0.clone()),
        transform: world.get::<crate::components::Transform>(entity).cloned(),
        sprite: world.get::<crate::components::Sprite>(entity).cloned(),
        // Resolve the parent link to the parent's Tag (EntityDef.parent is tag-based) so
        // Undo-of-Delete / Duplicate / Paste restore the hierarchy, not a root entity.
        // Mirrors do_save_scene_with_list's parent resolution.
        parent: world
            .get::<crate::hierarchy::Parent>(entity)
            .and_then(|p| world.get::<crate::prefab::Tag>(p.0).map(|t| t.0.clone())),
        components,
    })
}

impl App {
    /// Copy the named serde-registered component off `sel` into the component clipboard.
    /// No-op if the component isn't present or isn't serde-registered.
    pub(in crate::app) fn copy_component(&mut self, sel: Entity, type_name: &str) {
        let value = self
            .world
            .resource::<crate::prefab::SerdeComponentRegistry>()
            .and_then(|r| r.serialize_entity(&self.world, sel).remove(type_name));
        if let Some(value) = value {
            self.editor.component_clipboard = Some((type_name.to_string(), value));
        }
    }

    /// Paste the clipboard component onto `sel`, inserting or overwriting that one component.
    /// Uses the `remove_resource → deserialize_into → insert_resource` dance (the registry
    /// cannot be borrowed while `&mut World` is needed). Not pushed to undo history — matching
    /// the editor's existing Add/Remove-component actions.
    pub(in crate::app) fn paste_component(&mut self, sel: Entity) {
        let Some((name, value)) = self.editor.component_clipboard.clone() else {
            return;
        };
        if let Some(registry) = self
            .world
            .remove_resource::<crate::prefab::SerdeComponentRegistry>()
        {
            let mut one = std::collections::HashMap::new();
            one.insert(name, value);
            registry.deserialize_into(&mut self.world, sel, &one);
            self.world.insert_resource(registry);
        }
    }

    /// Save the selected entity as a prefab RON file at `path` (captures tag/transform/sprite/
    /// parent + serde-registered components via `entity_to_def`). Sets `prefab_status` AND surfaces
    /// the outcome as an action toast (success/error), matching the scene-save feedback.
    pub(in crate::app) fn save_selected_as_prefab(&mut self, sel: Entity, path: &str) {
        use crate::app::editor::state::ToastKind;
        let (status, kind) = match entity_to_def(&self.world, sel) {
            Some(def) => {
                let prefab = crate::prefab::Prefab { def };
                match prefab.save(std::path::Path::new(path)) {
                    Ok(()) => (
                        format!("{} → {path}", tr("Saved prefab", "프리팹 저장됨")),
                        ToastKind::Success,
                    ),
                    Err(e) => (
                        format!("{}: {e}", tr("Save failed", "저장 실패")),
                        ToastKind::Error,
                    ),
                }
            }
            None => (
                tr("No entity to save", "저장할 엔티티 없음").to_string(),
                ToastKind::Error,
            ),
        };
        self.push_editor_toast(status.clone(), kind);
        self.editor.prefab_status = Some(status);
    }

    /// Load a prefab from `path` and spawn it (with a `PrefabInstance` marker so Break Prefab
    /// works), then select the new entity. Sets `prefab_status` AND surfaces the outcome as an
    /// action toast (success/error), matching the scene-save feedback.
    pub(in crate::app) fn spawn_prefab(&mut self, path: &str) {
        use crate::app::editor::state::ToastKind;
        let (status, kind) = match crate::prefab::Prefab::load(std::path::Path::new(path)) {
            Ok(prefab) => {
                let e = prefab.spawn_with_tracking(&mut self.world, path.to_string());
                self.editor.inspector_selected = Some(e);
                self.editor.selected_entities = vec![e];
                (
                    format!("{} {path}", tr("Spawned prefab from", "프리팹 생성:")),
                    ToastKind::Success,
                )
            }
            Err(e) => (
                format!("{}: {e}", tr("Load failed", "불러오기 실패")),
                ToastKind::Error,
            ),
        };
        self.push_editor_toast(status.clone(), kind);
        self.editor.prefab_status = Some(status);
    }
}
