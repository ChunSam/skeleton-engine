#[cfg(not(target_arch = "wasm32"))]
use super::*;
#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::entity_to_def;

#[cfg(not(target_arch = "wasm32"))]
impl App {
    pub(in crate::app) fn handle_editor_shortcuts(&mut self, ctx: &egui::Context) {
        let (want_undo, want_redo, want_copy, want_paste) = ctx.input(|i| {
            let ctrl = i.modifiers.ctrl;
            let z = i.key_pressed(egui::Key::Z);
            let shift = i.modifiers.shift;
            let c = i.key_pressed(egui::Key::C);
            let v = i.key_pressed(egui::Key::V);
            (
                ctrl && z && !shift,
                ctrl && z && shift,
                ctrl && c,
                ctrl && v,
            )
        });
        if want_undo {
            let mut sel = self.editor.inspector_selected;
            self.editor.cmd_history.undo(&mut self.world, &mut sel);
            self.editor.inspector_selected = sel;
            if let Some(s) = self.editor.inspector_selected {
                if !self.editor.selected_entities.contains(&s) {
                    self.editor.selected_entities = vec![s];
                }
            } else {
                self.editor.selected_entities.clear();
            }
        }
        if want_redo {
            let mut sel = self.editor.inspector_selected;
            self.editor.cmd_history.redo(&mut self.world, &mut sel);
            self.editor.inspector_selected = sel;
            if let Some(s) = self.editor.inspector_selected {
                if !self.editor.selected_entities.contains(&s) {
                    self.editor.selected_entities = vec![s];
                }
            } else {
                self.editor.selected_entities.clear();
            }
        }
        // Ctrl+C: copy selected entities to the EntityDef clipboard
        if want_copy && !self.editor.selected_entities.is_empty() {
            let to_copy: Vec<Entity> = self.editor.selected_entities.clone();
            self.editor.copy_clipboard = to_copy
                .iter()
                .filter_map(|&e| entity_to_def(&self.world, e))
                .collect();
        }
        // Ctrl+V: paste entities from clipboard (20 px offset)
        if want_paste && !self.editor.copy_clipboard.is_empty() {
            let defs: Vec<crate::prefab::EntityDef> = self.editor.copy_clipboard.clone();
            let mut pasted: Vec<Entity> = Vec::new();
            for mut def in defs {
                if let Some(ref mut t) = def.transform {
                    t.position += glam::Vec2::new(20.0, 20.0);
                }
                let e = crate::prefab::spawn_entity_def(&mut self.world, &def);
                pasted.push(e);
            }
            // set the first pasted entity as the primary selection
            if let Some(&first) = pasted.first() {
                self.editor.inspector_selected = Some(first);
                self.editor.selected_entities = pasted;
            }
        }
    }
}
