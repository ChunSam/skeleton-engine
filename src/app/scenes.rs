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
        // Re-apply game-side world registrations (reflect, clone, serde components).
        // `World::new()` above discards all per-type metadata; these thunks restore
        // registrations from `register_editable_component` / `register_serde_component`
        // so they survive scene Replace. Runs after `register_core_component_metadata`
        // so the fresh `SerdeComponentRegistry` resource already exists.
        let regs = std::mem::take(&mut self.world_registrars);
        for r in &regs {
            r(&mut self.world);
        }
        self.world_registrars = regs;
        if let Some(debug_ui) = debug_ui {
            self.world.insert_resource(debug_ui);
        }
        // Re-insert preserved resources last — so they take precedence over engine defaults
        // (a preserved resource of the same type overwrites the default).
        for (tid, boxed) in preserved {
            self.world.insert_resource_erased(tid, boxed);
        }
        self.editor.inspector_selected = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.editor.selected_entities.clear();
            self.editor.copy_clipboard.clear();
        }
        self.editor.editor_save_status = None;
        // Clear the set of system indices disabled by panics.
        // After a world reset, systems get another chance; this also prevents stale
        // indices from incorrectly skipping unrelated systems when Replace swaps out
        // the entire systems list.
        // (The new world re-inserts an empty PanickedSystems resource, clearing the display too.)
        self.panicked_systems.clear();
    }

    /// Replaces the current scene (resets the world). See also [`push_scene`](App::push_scene) /
    /// [`pop_scene`](App::pop_scene) for stacking (e.g. a pause menu over the game).
    pub fn set_scene(&mut self, scene: Box<dyn Scene>) {
        self.apply_scene_cmd(SceneCmd::Replace(scene));
    }

    /// Pushes a scene onto the stack, suspending (not destroying) the current one — the App-level
    /// convenience for `SceneCmd::Push`, mirroring [`set_scene`](App::set_scene). Useful for
    /// overlays like a pause menu or inventory that resume the scene beneath on
    /// [`pop_scene`](App::pop_scene).
    pub fn push_scene(&mut self, scene: Box<dyn Scene>) {
        self.apply_scene_cmd(SceneCmd::Push(scene));
    }

    /// Pops the top scene off the stack, resuming the one beneath (no-op if the stack is empty).
    /// The App-level convenience for `SceneCmd::Pop`.
    pub fn pop_scene(&mut self) {
        self.apply_scene_cmd(SceneCmd::Pop);
    }

    fn reconcile_meta(&mut self) {
        use crate::ecs::schedule::SystemConfig;
        if self.system_meta.len() < self.systems.len() {
            self.system_meta
                .resize(self.systems.len(), SystemConfig::default());
        } else if self.system_meta.len() > self.systems.len() {
            self.system_meta.truncate(self.systems.len());
        }
        self.schedule_dirty = true;
    }

    pub(super) fn apply_scene_cmd(&mut self, cmd: SceneCmd) {
        use crate::scene::SystemRegistrar;

        // Scene systems occupy `systems[..scene_len]`; built-in tails occupy
        // `systems[scene_len..]`.  All insert/truncate operations that touch scene
        // systems must keep the tail in place.
        let tail = self.builtin_tail_count;

        match cmd {
            SceneCmd::Replace(mut new_scene) => {
                for (mut scene, _) in self.scene_stack.drain(..).rev() {
                    scene.on_exit(&mut self.world);
                }
                // Remove scene systems while preserving the permanent tail built-ins.
                // Built-in tail systems occupy `systems[systems.len()-tail..]`.
                // Scene systems occupy `systems[..systems.len()-tail]`.
                // Drain the scene portion (everything before the tail) so the tail
                // built-ins remain and their indices collapse back to the front.
                let scene_len = self.systems.len().saturating_sub(tail);
                self.systems.drain(..scene_len);
                self.system_meta.drain(..scene_len);
                self.reconcile_meta(); // sync meta (no-op when lengths already equal)
                self.reload_scene();
                let before = self.systems.len(); // = tail (only built-ins remain)
                {
                    let mut registrar = SystemRegistrar::new_with_tail(
                        &mut self.systems,
                        &mut self.system_meta,
                        tail,
                    );
                    new_scene.on_enter(&mut self.world, &mut registrar);
                }
                // `owned` = scene systems added (tail entries are not counted).
                let owned = self.systems.len() - before;
                self.scene_stack.push((new_scene, owned));
                self.reconcile_meta(); // ensure lengths stay equal (no-op when registrar kept them in sync)
            }
            SceneCmd::Push(mut new_scene) => {
                let before = self.systems.len(); // includes tail
                {
                    let mut registrar = SystemRegistrar::new_with_tail(
                        &mut self.systems,
                        &mut self.system_meta,
                        tail,
                    );
                    new_scene.on_enter(&mut self.world, &mut registrar);
                }
                // `owned` = systems inserted before the tail by this scene.
                let owned = self.systems.len() - before;
                self.scene_stack.push((new_scene, owned));
                self.reconcile_meta(); // ensure lengths stay equal (no-op when registrar kept them in sync)
            }
            SceneCmd::Pop => {
                if let Some((mut scene, owned)) = self.scene_stack.pop() {
                    scene.on_exit(&mut self.world);
                    // Remove `owned` scene systems from just before the tail.
                    // `scene_len` = total non-tail systems before the pop.
                    let scene_len = self.systems.len().saturating_sub(tail);
                    let new_scene_len = scene_len.saturating_sub(owned);
                    // Drain the popped scene's range: [new_scene_len..scene_len]
                    self.systems.drain(new_scene_len..scene_len);
                    self.system_meta.drain(new_scene_len..scene_len);
                    // Remove indices that no longer refer to valid scene systems after Pop.
                    // We keep only indices in [0, new_scene_len): those belong to the
                    // scene that remains on the stack and are still valid.
                    //
                    // Why `new_scene_len` and NOT `new_scene_len + tail`:
                    // The drain above removed [new_scene_len..scene_len].  A panicked index
                    // equal to `new_scene_len` would survive `i < new_scene_len + tail` (since
                    // new_scene_len < new_scene_len + tail), but after the drain that index
                    // now aliases the builtin tail (HierarchySystem at new_scene_len),
                    // permanently skipping GlobalTransform propagation for all later frames.
                    // Dropping tail indices here is intentional: they get a clean retry after
                    // a scene change, consistent with reload_scene which clears the full set.
                    self.panicked_systems.retain(|&i| i < new_scene_len);
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
                    self.reconcile_meta(); // sync meta after drain
                }
            }
        }
    }
}
