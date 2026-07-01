//! Entity right-click context menu + action dispatch.

use super::*;

/// A right-click action offered on an Entities-list row or a Scene-tree node. Dispatched through
/// [`App::editor_apply_entity_context_action`] so the wiring stays testable without egui.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntityContextAction {
    /// Scene-tree only: spawn a new entity parented under the target and select the child.
    AddChild,
    Rename,
    Duplicate,
    Focus,
    Delete,
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    /// Apply a right-click [`EntityContextAction`] to `entity` (Entities list AND Scene tree). For the
    /// selection-scoped ops (Rename/Duplicate/Focus/Delete) it selects `entity` first, so they act on
    /// the right-clicked row — not whatever happened to be selected before — then runs the op.
    /// `AddChild` instead spawns a NEW entity parented under `entity` (via the cycle-safe
    /// [`crate::hierarchy::reparent`]) and selects that child, so it does NOT pre-select `entity`. A
    /// dead entity is a no-op. Drives the same public ops the toolbar/shortcuts use; native-only.
    /// Module-private (its only callers — `entities_tab_body`/`scene_tab_body` and the tests — live in
    /// this file), so the private `EntityContextAction` never leaks through a more-public signature.
    pub(super) fn editor_apply_entity_context_action(
        &mut self,
        entity: Entity,
        action: EntityContextAction,
    ) {
        if !self.world.is_alive(entity) {
            return;
        }
        if action == EntityContextAction::AddChild {
            // Same fresh-spawn shape as the "＋ New Entity" toolbar button (Transform + Tag), then
            // parented under the target. Not undoable, matching that button. Select the child so the
            // user can immediately rename/edit it.
            let child = self.world.spawn();
            self.world
                .add_component(child, crate::components::Transform::default());
            self.world
                .add_component(child, crate::prefab::Tag("New Entity".into()));
            crate::hierarchy::reparent(&mut self.world, child, Some(entity));
            self.editor.inspector_selected = Some(child);
            self.editor.selected_entities = vec![child];
            return;
        }
        self.editor.inspector_selected = Some(entity);
        self.editor.selected_entities = vec![entity];
        match action {
            EntityContextAction::Rename => self.editor_begin_rename(entity),
            EntityContextAction::Duplicate => self.editor_duplicate_selection(),
            EntityContextAction::Focus => self.editor_focus_camera_on_selection(),
            EntityContextAction::Delete => self.editor_delete_selection(),
            EntityContextAction::AddChild => {}
        }
    }
}

/// Draw the shared entity right-click context menu and record the chosen `(entity, action)` into
/// `out` (collect-then-apply — the caller applies it after the list/tree is drawn, so the closure
/// never mutates `App` mid-iteration). Shared by the Entities list and the Scene tree; `add_child`
/// includes the Scene-tree-only "＋ Add child" item (spawns a parented entity). All glyphs are
/// already verified to render in egui's bundled font (reused from the "New Entity" button + the
/// existing menu items).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn entity_context_menu(
    ui: &mut egui::Ui,
    e: Entity,
    add_child: bool,
    out: &mut Option<(Entity, EntityContextAction)>,
) {
    if add_child {
        if ui.button(tr("＋ Add child", "＋ 자식 추가")).clicked() {
            *out = Some((e, EntityContextAction::AddChild));
            ui.close();
        }
        ui.separator();
    }
    if ui.button(tr("Rename", "이름 변경")).clicked() {
        *out = Some((e, EntityContextAction::Rename));
        ui.close();
    }
    if ui.button(tr("⎘ Duplicate", "⎘ 복제")).clicked() {
        *out = Some((e, EntityContextAction::Duplicate));
        ui.close();
    }
    if ui
        .button(tr("🎯 Focus camera", "🎯 카메라 포커스"))
        .clicked()
    {
        *out = Some((e, EntityContextAction::Focus));
        ui.close();
    }
    ui.separator();
    if ui.button(tr("🗑 Delete", "🗑 삭제")).clicked() {
        *out = Some((e, EntityContextAction::Delete));
        ui.close();
    }
}

/// Tests for the Entities-list right-click context menu's action dispatch. The egui menu buttons only
/// record `(entity, action)`; `editor_apply_entity_context_action` does the real work — so testing it
/// directly covers the behaviour without driving egui.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod context_action_tests {
    use super::EntityContextAction;
    use crate::prefab::Tag;
    use crate::{App, Transform};

    #[test]
    fn rename_selects_the_target_and_starts_a_rename() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Goblin".into()));

        app.editor_apply_entity_context_action(e, EntityContextAction::Rename);

        assert_eq!(app.editor.inspector_selected, Some(e), "target is selected");
        let rn = app.editor.entity_rename.as_ref().expect("rename active");
        assert_eq!(rn.entity, e);
        assert_eq!(rn.buffer, "Goblin", "rename buffer seeded from the Tag");
    }

    #[test]
    fn delete_acts_on_the_right_clicked_row_even_if_it_was_not_selected() {
        // The dispatch selects the target first, so Delete removes the right-clicked entity, not the
        // previously-selected one — the whole point of select-then-op.
        let mut app = App::new();
        let selected = app.world.spawn();
        let clicked = app.world.spawn();
        app.editor.inspector_selected = Some(selected);
        app.editor.selected_entities = vec![selected];

        app.editor_apply_entity_context_action(clicked, EntityContextAction::Delete);

        assert!(!app.world.is_alive(clicked), "right-clicked entity deleted");
        assert!(
            app.world.is_alive(selected),
            "the old selection is untouched"
        );
    }

    #[test]
    fn duplicate_clones_the_target() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Src".into()));
        app.world.add_component(e, Transform::default());
        let before = app.world.entities().len();

        app.editor_apply_entity_context_action(e, EntityContextAction::Duplicate);

        assert_eq!(app.world.entities().len(), before + 1, "one clone spawned");
        // Selection moved to the clone (not the original).
        assert_ne!(app.editor.inspector_selected, Some(e));
        assert!(app.editor.inspector_selected.is_some());
    }

    #[test]
    fn action_on_a_dead_entity_is_a_noop() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.despawn(e);
        // Must not panic or start a rename on a despawned entity.
        app.editor_apply_entity_context_action(e, EntityContextAction::Rename);
        assert!(app.editor.entity_rename.is_none());
    }

    #[test]
    fn add_child_spawns_a_parented_entity_and_selects_the_child() {
        use crate::hierarchy::{Children, Parent};
        let mut app = App::new();
        let parent = app.world.spawn();
        app.world.add_component(parent, Tag("Parent".into()));
        let before = app.world.entities().len();

        app.editor_apply_entity_context_action(parent, EntityContextAction::AddChild);

        assert_eq!(app.world.entities().len(), before + 1, "one child spawned");
        // Selection moved to the NEW child, not the right-clicked parent.
        let child = app.editor.inspector_selected.expect("child selected");
        assert_ne!(child, parent, "the child is selected, not the parent");
        assert_eq!(app.editor.selected_entities, vec![child]);
        // The child is parented under the target, and the target lists it as a child.
        assert_eq!(
            app.world.get::<Parent>(child).map(|p| p.0),
            Some(parent),
            "child points at the parent"
        );
        assert!(
            app.world
                .get::<Children>(parent)
                .is_some_and(|c| c.0.contains(&child)),
            "parent lists the child"
        );
        // Fresh-spawn shape: Transform + a Tag (like the New Entity button).
        assert!(app.world.get::<Transform>(child).is_some());
    }

    #[test]
    fn add_child_on_a_dead_entity_is_a_noop() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.despawn(e);
        let before = app.world.entities().len();
        app.editor_apply_entity_context_action(e, EntityContextAction::AddChild);
        assert_eq!(
            app.world.entities().len(),
            before,
            "no child spawned under a dead entity"
        );
    }
}
