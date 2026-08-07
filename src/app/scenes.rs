use super::*;

/// How many of the systems a `Replace` is about to drain belong to no `Scene`.
///
/// Draining a scene's own systems is the *point* of `Replace` — they were registered in
/// `Scene::on_enter` and die with it, which `scene_stack` records as `owned`. Whatever is left
/// over in the scene portion got there through `App::add_system` / `add_system_labeled`
/// instead, and `Replace` drops it with **no symptom**: the system just stops running.
///
/// `beat_crawler` shipped several releases that way — its `AudioFacadeSystem` was registered on
/// the `App` before `set_scene`, so `Audio::update` never ticked, the turn clock ran on its
/// watchdog fallback, and the only sign was a HUD string reading "schedule" instead of
/// "listening" (fixed in v0.142.0).
fn app_registered_system_count(scene_len: usize, owned_by_scenes: usize) -> usize {
    scene_len.saturating_sub(owned_by_scenes)
}

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
            // Undo/redo holds raw `Entity` handles, and the ECS reuses ids — so a command
            // surviving a world reset does not merely fail, it silently retargets onto whatever
            // new entity now occupies that slot. (Native-only: `cmd_history` is not a field of
            // the reduced wasm `EditorState`.)
            self.editor.cmd_history.clear();
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

    /// Transitions to `scene` with a styled cover → swap → reveal animation
    /// ([`SceneTransition`](crate::SceneTransition)). `App` covers the screen over `half_duration`
    /// seconds in the given [`style`](crate::TransitionStyle), swaps to `scene` (like
    /// [`set_scene`](App::set_scene)) *while fully hidden*, then reveals it over another
    /// `half_duration`. The transition resource is auto-registered as persistent so it survives the
    /// mid-transition world reset, and is dropped when the reveal finishes.
    ///
    /// Native-only visuals (the overlay is not drawn on wasm), but the scene swap still happens on
    /// every platform. For a bare cover/reveal with no swap, insert a `SceneTransition` resource
    /// directly. Customize the colour via
    /// [`SceneTransition::with_color`](crate::SceneTransition::with_color) on a manual insert, or set
    /// it on the resource after this call.
    pub fn transition_to_scene(
        &mut self,
        scene: Box<dyn Scene>,
        style: crate::scene_transition::TransitionStyle,
        half_duration: f32,
    ) {
        crate::scene_transition::start_scene_transition(
            &mut self.world,
            scene,
            style,
            half_duration,
        );
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
                // Measured before the stack is drained — `owned` lives on the stack entries.
                let orphaned = app_registered_system_count(
                    self.systems.len().saturating_sub(tail),
                    self.scene_stack.iter().map(|(_, owned)| *owned).sum(),
                );
                if orphaned > 0 {
                    log::warn!(
                        "SceneCmd::Replace is discarding {orphaned} system(s) registered directly \
                         on the App; they will never run again, and nothing else will report it. \
                         Register them inside `Scene::on_enter` via the `SystemRegistrar` — a \
                         system added with `App::add_system` before `set_scene` is swept away by \
                         the world reset."
                    );
                }
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

#[cfg(test)]
mod tests {
    use super::app_registered_system_count;

    /// The whole point of the count: a scene's own systems are *supposed* to go, so they must
    /// never be reported. Only the excess — what `App::add_system` put there — is a problem.
    #[test]
    fn scene_owned_systems_are_never_reported() {
        // A scene registered all four; Replace dropping them is correct behaviour.
        assert_eq!(app_registered_system_count(4, 4), 0);
        // A pushed stack of two scenes owning 3 + 2 — Replace tears down the whole stack.
        assert_eq!(app_registered_system_count(5, 3 + 2), 0);
        // No scene has ever entered, so all four came from the App. This is beat_crawler's case.
        assert_eq!(app_registered_system_count(4, 0), 4);
        // Mixed: a scene owns 3, one was added on the App afterwards.
        assert_eq!(app_registered_system_count(4, 3), 1);
    }

    /// `owned` is bookkeeping, and bookkeeping can drift. Saturating means a drift shows up as
    /// silence rather than as a panic in the middle of a scene change.
    #[test]
    fn a_larger_owned_count_saturates_instead_of_underflowing() {
        assert_eq!(app_registered_system_count(2, 5), 0);
    }
}
