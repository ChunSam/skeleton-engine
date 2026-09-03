#[cfg(not(target_arch = "wasm32"))]
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::entity_to_def;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::state::ToastKind;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::tr;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::EditorCmd;
#[cfg(not(target_arch = "wasm32"))]
use crate::camera::Camera;
#[cfg(not(target_arch = "wasm32"))]
use crate::components::Transform;
#[cfg(not(target_arch = "wasm32"))]
use crate::resources::ViewportSize;
#[cfg(not(target_arch = "wasm32"))]
use glam::Vec2;

#[cfg(not(target_arch = "wasm32"))]
impl App {
    pub(in crate::app) fn handle_editor_shortcuts(&mut self, ctx: &egui::Context) {
        // A widget that wants keyboard input (a focused TextEdit, e.g. the scene-path or a data
        // cell) must keep the bare single-key shortcuts (Delete / F / ?) from firing — otherwise
        // typing a path would delete the selection or toggle the cheatsheet. Ctrl-combos are left
        // ungated, matching the existing Ctrl+Z/C/V.
        let typing = ctx.egui_wants_keyboard_input();
        let keys = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl;
            let shift = i.modifiers.shift;
            Keys {
                undo: ctrl && i.key_pressed(egui::Key::Z) && !shift,
                redo: ctrl && i.key_pressed(egui::Key::Z) && shift,
                copy: ctrl && i.key_pressed(egui::Key::C),
                paste: ctrl && i.key_pressed(egui::Key::V),
                save: ctrl && i.key_pressed(egui::Key::S),
                duplicate: ctrl && i.key_pressed(egui::Key::D),
                // Bare keys: gated on `!typing` below.
                delete: i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                focus: i.key_pressed(egui::Key::F),
                help: shift && i.key_pressed(egui::Key::Slash), // Shift+/ == ?
            }
        });

        if keys.undo {
            self.editor_undo();
        }
        if keys.redo {
            self.editor_redo();
        }
        // Ctrl+C: copy selected entities to the EntityDef clipboard.
        if keys.copy && !self.editor.selected_entities.is_empty() {
            let to_copy: Vec<Entity> = self.editor.selected_entities.clone();
            self.editor.copy_clipboard = to_copy
                .iter()
                .filter_map(|&e| entity_to_def(&self.world, e))
                .collect();
        }
        // Ctrl+V: paste entities from clipboard (20 px offset).
        if keys.paste && !self.editor.copy_clipboard.is_empty() {
            self.editor_paste_clipboard();
        }
        // Ctrl+S: save the scene to the editor's save path (same as the toolbar 💾 Save).
        if keys.save {
            super::docked::do_save_scene(self);
        }
        // Ctrl+D: duplicate the current selection.
        if keys.duplicate {
            self.editor_duplicate_selection();
        }
        // Delete / Backspace: delete the current selection (bare key — skip while typing).
        if keys.delete && !typing {
            self.editor_delete_selection();
        }
        // F: center the camera on the selected entity (bare key — skip while typing).
        if keys.focus && !typing {
            self.editor_focus_camera_on_selection();
        }
        // ? : toggle the keyboard-shortcuts cheatsheet (bare key — skip while typing).
        if keys.help && !typing {
            self.editor.show_shortcuts = !self.editor.show_shortcuts;
        }
    }

    /// Undo the last editor command, syncing the selection to the restored/removed entity.
    fn editor_undo(&mut self) {
        let mut sel = self.editor.inspector_selected;
        self.editor.cmd_history.undo(&mut self.world, &mut sel);
        self.sync_selection_after_history(sel);
    }

    /// Redo the last undone editor command, syncing the selection.
    fn editor_redo(&mut self) {
        let mut sel = self.editor.inspector_selected;
        self.editor.cmd_history.redo(&mut self.world, &mut sel);
        self.sync_selection_after_history(sel);
    }

    /// After an undo/redo, mirror the single `inspector_selected` into the multi-select list.
    fn sync_selection_after_history(&mut self, sel: Option<Entity>) {
        self.editor.inspector_selected = sel;
        match sel {
            Some(s) if !self.editor.selected_entities.contains(&s) => {
                self.editor.selected_entities = vec![s];
            }
            None => self.editor.selected_entities.clear(),
            _ => {}
        }
    }

    /// Paste the EntityDef clipboard (each entity offset +20 px, recorded for undo, then selected).
    /// `pub(in crate::app)` so a test can paste without driving egui; Ctrl+V is the only caller.
    ///
    /// ⚠️ **A copied parent and child do land under the pasted parent, and that is incidental.**
    /// `EntityDef` names its parent by tag, and `spawn_entity_def` searches the whole world and
    /// takes the first match — so with the original still there, two entities answer to the tag.
    /// Which one wins is decided by `World::query`, which walks **archetypes in creation order**:
    /// the fresh copy carries fewer components at that instant (no `Children` yet) and sits in an
    /// earlier archetype than the original, so the copy wins. Nothing states that, and no test
    /// covered it, so `pasting_a_parent_and_child_keeps_them_together` pins it (v0.156.17).
    ///
    /// ⚠️ Redo of one `CreateEntity` respawns its def alone against a world that no longer has a
    /// batch, so a redone child resolves the tag afresh. That is the tag limitation
    /// `EditorCmd::DeleteEntity` already documents: a def carries a tag, not a handle.
    pub(in crate::app) fn editor_paste_clipboard(&mut self) {
        let defs: Vec<crate::prefab::EntityDef> = self.editor.copy_clipboard.clone();
        let mut pasted: Vec<Entity> = Vec::new();
        for mut def in defs {
            if let Some(ref mut t) = def.transform {
                t.position += Vec2::new(20.0, 20.0);
            }
            let e = crate::prefab::spawn_entity_def(&mut self.world, &def);
            self.editor.cmd_history.push(EditorCmd::CreateEntity {
                entity: e,
                def: Some(def),
            });
            pasted.push(e);
        }
        if let Some(&first) = pasted.first() {
            self.editor.inspector_selected = Some(first);
            let n = pasted.len();
            self.editor.selected_entities = pasted;
            self.push_editor_toast(format!("{} {n}", tr("Pasted", "붙여넣기")), ToastKind::Info);
        }
    }

    /// The entities a selection-scoped action targets: the multi-select list, or the single
    /// `inspector_selected` if the list is empty.
    fn editor_effective_selection(&self) -> Vec<Entity> {
        if !self.editor.selected_entities.is_empty() {
            self.editor.selected_entities.clone()
        } else {
            self.editor.inspector_selected.into_iter().collect()
        }
    }

    /// Despawns `root` and its whole subtree the way the editor must: everything the subtree owns
    /// in `PhysicsWorld` — rigid bodies, colliders, tile colliders — is released first, then
    /// `hierarchy::despawn_recursive` takes the entities. Both are storage-level by design, so
    /// without the release an editor delete left the physics behind as invisible, still-colliding
    /// ghosts (v0.156.8). **Both delete paths — the Delete key and the 🗑 button — go through
    /// here.** Undo restores the entities from their `EntityDef`s, which carry no physics; that
    /// was already so, and the release does not change it.
    pub(in crate::app) fn editor_despawn_subtree(&mut self, root: Entity) {
        for e in crate::hierarchy::descendants(&self.world, root) {
            crate::physics::release_physics(&mut self.world, e);
        }
        crate::physics::release_physics(&mut self.world, root);
        crate::hierarchy::despawn_recursive(&mut self.world, root);
    }

    /// Delete every entity in the current editor selection (multi-select aware), recording each for
    /// undo, then clear the selection. Backs the Delete / Backspace shortcut.
    pub(in crate::app) fn editor_delete_selection(&mut self) {
        let mut deleted = 0u32;
        for e in self.editor_effective_selection() {
            if !self.world.is_alive(e) {
                continue;
            }
            let defs = crate::app::editor::prefab::subtree_to_defs(&self.world, e);
            deleted += defs.len() as u32;
            self.editor.cmd_history.push(EditorCmd::DeleteEntity {
                entities: None,
                defs,
            });
            // The subtree goes too, which is why the `is_alive` skip above matters: selecting a
            // parent and its child deletes the child once, not twice.
            self.editor_despawn_subtree(e);
        }
        self.editor.inspector_selected = None;
        self.editor.selected_entities.clear();
        if deleted > 0 {
            self.push_editor_toast(
                format!("{} {deleted}", tr("Deleted", "삭제됨")),
                ToastKind::Success,
            );
        }
    }

    /// Duplicate every selected entity (clone all components, offset +16 px), record each for undo,
    /// and select the clones. Backs Ctrl+D.
    pub(in crate::app) fn editor_duplicate_selection(&mut self) {
        let mut clones: Vec<Entity> = Vec::new();
        for e in self.editor_effective_selection() {
            if let Some(new_entity) = self.world.clone_entity(e) {
                if let Some(t) = self.world.get_mut::<Transform>(new_entity) {
                    t.position += Vec2::new(16.0, 16.0);
                }
                let def = entity_to_def(&self.world, new_entity);
                self.editor.cmd_history.push(EditorCmd::CreateEntity {
                    entity: new_entity,
                    def,
                });
                clones.push(new_entity);
            }
        }
        if let Some(&first) = clones.first() {
            self.editor.inspector_selected = Some(first);
            let n = clones.len();
            self.editor.selected_entities = clones;
            self.push_editor_toast(
                format!("{} {n}", tr("Duplicated", "복제됨")),
                ToastKind::Success,
            );
        }
    }

    /// Draw the keyboard-shortcuts cheatsheet window when `show_shortcuts` is on (toggled by `?`
    /// or the toolbar `? Keys` button). Renders in both overlay and docked editor modes — a
    /// discoverability aid so shortcuts aren't invisible. Closing the window (×) clears the flag.
    pub(in crate::app) fn draw_editor_shortcuts_window(&mut self, ctx: &egui::Context) {
        if !self.editor.show_shortcuts {
            return;
        }
        // (keys, English action, Korean action) — English is the source of truth (see i18n::tr).
        let rows: [(&'static str, &'static str, &'static str); 10] = [
            ("F1", "Toggle editor overlay", "에디터 오버레이 토글"),
            ("F2", "Toggle docked editor", "도킹 에디터 토글"),
            ("Ctrl+Z", "Undo", "실행 취소"),
            ("Ctrl+Shift+Z", "Redo", "다시 실행"),
            (
                "Ctrl+C / Ctrl+V",
                "Copy / paste entities",
                "엔티티 복사 / 붙여넣기",
            ),
            ("Ctrl+D", "Duplicate selection", "선택 복제"),
            ("Ctrl+S", "Save scene", "씬 저장"),
            ("Delete / Backspace", "Delete selection", "선택 삭제"),
            ("F", "Focus camera on selection", "선택으로 카메라 포커스"),
            ("?", "Toggle this cheatsheet", "이 단축키 안내 토글"),
        ];
        let mut open = true;
        egui::Window::new(tr("⌨ Keyboard Shortcuts", "⌨ 키보드 단축키"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                egui::Grid::new("editor_shortcuts_grid")
                    .striped(true)
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.strong(tr("Key", "키"));
                        ui.strong(tr("Action", "동작"));
                        ui.end_row();
                        for (k, en, ko) in rows {
                            ui.monospace(k);
                            ui.label(tr(en, ko));
                            ui.end_row();
                        }
                    });
            });
        // The window's close button drives `open`; mirror it back so × hides the cheatsheet.
        self.editor.show_shortcuts = open;
    }

    /// Center the camera on where the primary selected entity is drawn (`GlobalTransform`, else
    /// `Transform`). Backs the F (focus) shortcut. No-op when nothing is selected or the
    /// selection has neither.
    pub(in crate::app) fn editor_focus_camera_on_selection(&mut self) {
        let Some(sel) = self
            .editor
            .inspector_selected
            .or_else(|| self.editor.selected_entities.first().copied())
        else {
            return;
        };
        // Where it is drawn: `GlobalTransform` for a parented entity, whose `Transform` is only
        // the offset from its parent — focusing a child used to centre on that offset instead
        // and leave the child off-screen (v0.156.11).
        let Some(pos) = self
            .world
            .get::<crate::hierarchy::GlobalTransform>(sel)
            .map(|g| g.position)
            .or_else(|| self.world.get::<Transform>(sel).map(|t| t.position))
        else {
            return;
        };
        let viewport = self
            .world
            .resource::<ViewportSize>()
            .map(|v| Vec2::new(v.width, v.height))
            .unwrap_or(Vec2::new(800.0, 600.0));
        if let Some(cam) = self.world.resource_mut::<Camera>() {
            // Camera.position is the top-left world coord; world_to_screen = (world - pos) * zoom,
            // so to put `pos` at the viewport center: position = pos - (viewport/2) / zoom.
            let zoom = if cam.zoom.abs() < 1e-6 { 1.0 } else { cam.zoom };
            cam.position = pos - viewport * 0.5 / zoom;
        }
    }
}

/// Edge-detected editor-shortcut keys read once per frame from the egui input.
#[cfg(not(target_arch = "wasm32"))]
struct Keys {
    undo: bool,
    redo: bool,
    copy: bool,
    paste: bool,
    save: bool,
    duplicate: bool,
    delete: bool,
    focus: bool,
    help: bool,
}
