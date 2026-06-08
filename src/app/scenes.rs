use super::*;

impl App {
    pub fn register_persistent<T: 'static>(&mut self) {
        let tid = std::any::TypeId::of::<T>();
        if !self.persistent_resources.contains(&tid) {
            self.persistent_resources.push(tid);
        }
    }

    pub fn reload_scene(&mut self) {
        // Extract preserved resources into type-erased boxes before the reset.
        let preserved: Vec<(std::any::TypeId, Box<dyn std::any::Any>)> = self
            .persistent_resources
            .iter()
            .filter_map(|&tid| self.world.take_resource_erased(tid).map(|b| (tid, b)))
            .collect();
        let debug_ui = self.world.remove_resource::<DebugUi>();

        self.world = World::new();
        Self::insert_core_resources(&mut self.world);
        // Re-insert registered event resources
        let inits = std::mem::take(&mut self.event_initializers);
        for init in &inits {
            init(&mut self.world);
        }
        self.event_initializers = inits;
        Self::register_core_component_metadata(&mut self.world);
        if let Some(debug_ui) = debug_ui {
            self.world.insert_resource(debug_ui);
        }
        // Re-insert preserved resources last — so they take precedence over engine defaults
        // (a preserved resource of the same type overwrites the default).
        for (tid, boxed) in preserved {
            self.world.insert_resource_erased(tid, boxed);
        }
        self.inspector_selected = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.selected_entities.clear();
            self.copy_clipboard.clear();
        }
        self.editor_save_status = None;
        // Clear the set of system indices disabled by panics.
        // After a world reset, systems get another chance; this also prevents stale
        // indices from incorrectly skipping unrelated systems when Replace swaps out
        // the entire systems list.
        // (The new world re-inserts an empty PanickedSystems resource, clearing the display too.)
        self.panicked_systems.clear();
    }

    pub fn set_scene(&mut self, scene: Box<dyn Scene>) {
        self.apply_scene_cmd(SceneCmd::Replace(scene));
    }

    fn reconcile_meta(&mut self) {
        use crate::ecs::schedule::SystemMeta;
        if self.system_meta.len() < self.systems.len() {
            self.system_meta
                .resize(self.systems.len(), SystemMeta::default());
        } else if self.system_meta.len() > self.systems.len() {
            self.system_meta.truncate(self.systems.len());
        }
        self.schedule_dirty = true;
    }

    pub(super) fn apply_scene_cmd(&mut self, cmd: SceneCmd) {
        match cmd {
            SceneCmd::Replace(mut new_scene) => {
                for (mut scene, _) in self.scene_stack.drain(..).rev() {
                    scene.on_exit(&mut self.world);
                }
                self.systems.clear();
                self.reconcile_meta(); // sync meta after systems.clear()
                self.reload_scene();
                let before = self.systems.len();
                new_scene.on_enter(&mut self.world, &mut self.systems);
                let owned = self.systems.len() - before;
                self.scene_stack.push((new_scene, owned));
                self.reconcile_meta(); // absorb systems pushed directly by the scene in on_enter
            }
            SceneCmd::Push(mut new_scene) => {
                let before = self.systems.len();
                new_scene.on_enter(&mut self.world, &mut self.systems);
                let owned = self.systems.len() - before;
                self.scene_stack.push((new_scene, owned));
                self.reconcile_meta(); // absorb systems pushed directly by the scene in on_enter
            }
            SceneCmd::Pop => {
                if let Some((mut scene, owned)) = self.scene_stack.pop() {
                    scene.on_exit(&mut self.world);
                    let new_len = self.systems.len().saturating_sub(owned);
                    self.systems.truncate(new_len);
                    // Remove indices of systems removed by Pop from the panic set
                    // (remaining indices stay valid). Without this, a truncated index
                    // persists and would incorrectly skip a new system at that same index.
                    self.panicked_systems.retain(|&i| i < new_len);
                    // Rebuild the display PanickedSystems resource based on the remaining indices.
                    let names: Vec<String> = self
                        .panicked_systems
                        .iter()
                        .filter_map(|&i| self.systems.get(i).map(|s| s.name().to_string()))
                        .collect();
                    if let Some(ps) = self
                        .world
                        .resource_mut::<crate::resources::PanickedSystems>()
                    {
                        ps.disabled = names;
                    }
                    self.reconcile_meta(); // sync meta after truncate
                }
            }
        }
    }
}
