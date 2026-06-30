//! Inline entity rename — edit an entity's `Tag` name directly in the editor entity list.
//!
//! Double-clicking a row in the docked Entities list starts a rename: the row's label is replaced
//! by a focused text box bound to [`EntityRename::buffer`]. Committing (Enter or clicking away)
//! writes `Tag(buffer)` to the entity; Escape cancels. The transient edit state lives in
//! `EditorState::entity_rename`; this module holds the behaviour (begin / commit / cancel) the
//! docked UI calls. Previously the only way to rename was the separate "Name:" field, which needs
//! the entity selected first — inline rename edits in place.

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::editor::state::EntityRename;
use crate::prefab::Tag;

impl App {
    /// Begin inline-renaming `entity` in the editor entity list. Seeds the edit buffer from the
    /// entity's current `Tag` (empty if it has none) and requests keyboard focus for one frame.
    /// Public so a headless capture or a game can drive a rename programmatically — the same state a
    /// double-click on a list row produces. Native-only.
    pub fn editor_begin_rename(&mut self, entity: crate::Entity) {
        let buffer = self
            .world
            .get::<Tag>(entity)
            .map(|t| t.0.clone())
            .unwrap_or_default();
        self.editor.entity_rename = Some(EntityRename {
            entity,
            buffer,
            focus_pending: true,
        });
    }

    /// Commit the in-progress inline rename: write `Tag(buffer)` to the renamed entity and clear the
    /// edit state. A blank (whitespace-only) buffer or a despawned entity cancels instead of writing,
    /// so an accidental empty name never replaces a real one. No-op when no rename is active.
    pub(in crate::app) fn editor_commit_rename(&mut self) {
        let Some(rn) = self.editor.entity_rename.take() else {
            return;
        };
        let name = rn.buffer.trim();
        if !name.is_empty() && self.world.is_alive(rn.entity) {
            self.world.add_component(rn.entity, Tag(name.to_string()));
        }
    }

    /// Cancel the in-progress inline rename without writing. No-op when no rename is active.
    pub(in crate::app) fn editor_cancel_rename(&mut self) {
        self.editor.entity_rename = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::prefab::Tag;

    #[test]
    fn begin_seeds_buffer_from_existing_tag() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Goblin".into()));

        app.editor_begin_rename(e);
        let rn = app.editor.entity_rename.as_ref().expect("rename active");
        assert_eq!(rn.entity, e);
        assert_eq!(rn.buffer, "Goblin");
        assert!(rn.focus_pending, "first frame requests focus");
    }

    #[test]
    fn begin_on_untagged_seeds_empty_buffer() {
        let mut app = App::new();
        let e = app.world.spawn();

        app.editor_begin_rename(e);
        let rn = app.editor.entity_rename.as_ref().expect("rename active");
        assert_eq!(rn.buffer, "");
    }

    #[test]
    fn commit_writes_tag_and_clears_state() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Old".into()));

        app.editor_begin_rename(e);
        app.editor.entity_rename.as_mut().unwrap().buffer = "  New Name  ".into();
        app.editor_commit_rename();

        assert!(
            app.editor.entity_rename.is_none(),
            "state cleared on commit"
        );
        assert_eq!(
            app.world.get::<Tag>(e).map(|t| t.0.as_str()),
            Some("New Name"),
            "buffer is trimmed and written to the Tag"
        );
    }

    #[test]
    fn commit_adds_tag_to_a_previously_untagged_entity() {
        let mut app = App::new();
        let e = app.world.spawn();

        app.editor_begin_rename(e);
        app.editor.entity_rename.as_mut().unwrap().buffer = "Fresh".into();
        app.editor_commit_rename();

        assert_eq!(app.world.get::<Tag>(e).map(|t| t.0.as_str()), Some("Fresh"));
    }

    #[test]
    fn commit_with_blank_buffer_keeps_old_name() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Keep".into()));

        app.editor_begin_rename(e);
        app.editor.entity_rename.as_mut().unwrap().buffer = "   ".into();
        app.editor_commit_rename();

        assert!(app.editor.entity_rename.is_none(), "state still cleared");
        assert_eq!(
            app.world.get::<Tag>(e).map(|t| t.0.as_str()),
            Some("Keep"),
            "a whitespace-only name does not overwrite the existing Tag"
        );
    }

    #[test]
    fn commit_on_despawned_entity_is_a_noop() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.editor_begin_rename(e);
        app.editor.entity_rename.as_mut().unwrap().buffer = "Ghost".into();
        app.world.despawn(e);

        app.editor_commit_rename();
        assert!(app.editor.entity_rename.is_none());
        assert!(!app.world.is_alive(e));
    }

    #[test]
    fn cancel_discards_the_edit() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Original".into()));

        app.editor_begin_rename(e);
        app.editor.entity_rename.as_mut().unwrap().buffer = "Discarded".into();
        app.editor_cancel_rename();

        assert!(app.editor.entity_rename.is_none());
        assert_eq!(
            app.world.get::<Tag>(e).map(|t| t.0.as_str()),
            Some("Original"),
            "cancel must not write the buffer"
        );
    }
}
