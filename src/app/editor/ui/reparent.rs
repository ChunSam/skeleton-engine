//! Drag-to-reparent — restructure the entity hierarchy from the editor's Scene tree.
//!
//! The docked editor's **Scene** tab shows a read-only parent→children tree. This adds the write
//! side: dragging a node onto another node re-parents it under that node; dragging it onto the
//! bottom "unparent" zone detaches it to a root. The actual graph edit goes through the cycle-safe
//! [`crate::hierarchy::reparent`] (no-op + `false` for a self/descendant-cycle/no-change move);
//! this layer adds the editor toast and is what `scene_tab_body`'s egui drag-and-drop calls.

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::prefab::Tag;

impl App {
    /// Re-parent `child` under `new_parent` (or to a root with `None`) from the editor, via the
    /// cycle-safe [`crate::hierarchy::reparent`], and surface a success toast when the hierarchy
    /// actually changed. Returns that change flag — a self / descendant-cycle / no-op move returns
    /// `false` and shows no toast. Public so a headless capture or a game tool can drive a reparent
    /// the same way a Scene-tree drag does. Native-only.
    pub fn editor_reparent(
        &mut self,
        child: crate::Entity,
        new_parent: Option<crate::Entity>,
    ) -> bool {
        if !crate::hierarchy::reparent(&mut self.world, child, new_parent) {
            return false;
        }
        let label = |w: &crate::ecs::World, e: crate::Entity| {
            w.get::<Tag>(e)
                .map(|t| t.0.clone())
                .unwrap_or_else(|| format!("Entity {}:{}", e.index(), e.generation()))
        };
        let msg = match new_parent {
            Some(p) => format!("{} → {}", label(&self.world, child), label(&self.world, p)),
            None => format!("{} → ({})", label(&self.world, child), tr("root", "루트")),
        };
        self.editor_toast_success(msg);
        true
    }
}

#[cfg(test)]
mod tests {
    use crate::app::App;
    use crate::hierarchy::{attach, Parent};

    fn spawn(app: &mut App) -> crate::Entity {
        let e = app.world.spawn();
        app.world
            .add_component(e, crate::components::Transform::default());
        e
    }

    #[test]
    fn editor_reparent_moves_and_toasts() {
        let mut app = App::new();
        let (p, c) = (spawn(&mut app), spawn(&mut app));
        let before = app.editor.toasts.len();

        assert!(app.editor_reparent(c, Some(p)), "valid move returns true");
        assert_eq!(app.world.get::<Parent>(c).map(|x| x.0), Some(p));
        assert_eq!(
            app.editor.toasts.len(),
            before + 1,
            "a successful reparent pushes one toast"
        );
    }

    #[test]
    fn editor_reparent_rejects_cycle_without_toast() {
        let mut app = App::new();
        let (p, c) = (spawn(&mut app), spawn(&mut app));
        attach(&mut app.world, c, p);
        let before = app.editor.toasts.len();

        // Reparenting p under its own child c would create a cycle → refused, no toast.
        assert!(!app.editor_reparent(p, Some(c)));
        assert!(app.world.get::<Parent>(p).is_none(), "p stays a root");
        assert_eq!(
            app.editor.toasts.len(),
            before,
            "no toast on a rejected move"
        );
    }

    #[test]
    fn editor_reparent_to_root_detaches() {
        let mut app = App::new();
        let (p, c) = (spawn(&mut app), spawn(&mut app));
        attach(&mut app.world, c, p);

        assert!(app.editor_reparent(c, None), "detach returns true");
        assert!(app.world.get::<Parent>(c).is_none(), "c becomes a root");
    }
}
