use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScheduleErrorPolicy {
    /// Logs the error and runs systems in insertion order.
    #[default]
    LogAndFallback,
    /// Logs the error and skips user system execution for that frame.
    DisableRunOnCycle,
    /// Panics on a circular dependency. Use in tests/dev when you want fast failure.
    PanicOnCycle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SystemPanicPolicy {
    /// Logs the panic, disables the system for future frames, and continues running.
    #[default]
    DisableSystemAndContinue,
    /// Logs the panic, then re-panics so the caller sees the failure immediately.
    AbortAfterLog,
}

pub(super) fn format_panic_payload(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// Merge a pending (unconsumed) egui textures delta with the current frame's one.
///
/// If render() skipped a frame (surface Lost/Outdated/Timeout/Occluded), the previous
/// textures_delta was never consumed. egui 0.34's skrifa font atlas sends
/// incremental per-glyph updates and never re-sends the full image, so a single
/// dropped delta poisons every later partial update (egui-wgpu panics with
/// "Tried to update a texture that has not been allocated yet"). Deltas must be
/// appended old → new, never overwritten; stale paint jobs may be replaced.
pub(super) fn merge_textures_delta(
    pending: Option<egui::TexturesDelta>,
    newer: egui::TexturesDelta,
) -> egui::TexturesDelta {
    match pending {
        Some(mut p) => {
            p.append(newer);
            p
        }
        None => newer,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn write_crash_log(system_name: &str, message: &str) {
    #[cfg(test)]
    {
        let _ = (system_name, message);
    }

    #[cfg(not(test))]
    {
        use std::io::Write;
        let path = "crash.log";
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let content = format!("[{timestamp}] system panic: {system_name}\nerror: {message}\n\n");
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = file.write_all(content.as_bytes());
        }
    }
}

impl App {
    pub fn register_event<E: 'static>(&mut self) {
        self.world.insert_resource(Events::<E>::default());
        self.event_flushers.push(Box::new(|world: &mut World| {
            if let Some(events) = world.resource_mut::<Events<E>>() {
                events.flush();
            }
        }));
        self.event_initializers.push(Box::new(|world: &mut World| {
            world.insert_resource(Events::<E>::default());
        }));
    }

    pub fn add_system<S: System + 'static>(&mut self, system: S) {
        // Insert before the permanent tail built-ins so their indices stay highest.
        let insert_at = self.systems.len().saturating_sub(self.builtin_tail_count);
        self.systems.insert(insert_at, Box::new(system));
        self.system_meta
            .insert(insert_at, crate::ecs::schedule::SystemConfig::default());
        self.schedule_dirty = true;
    }

    pub fn add_system_labeled<S: System + 'static>(
        &mut self,
        system: S,
        config: crate::ecs::schedule::SystemConfig,
    ) {
        // Insert before the permanent tail built-ins so their indices stay highest.
        let insert_at = self.systems.len().saturating_sub(self.builtin_tail_count);
        self.systems.insert(insert_at, Box::new(system));
        self.system_meta.insert(insert_at, config);
        self.schedule_dirty = true;
    }

    pub fn set_enabled(&mut self, set: crate::ecs::schedule::SystemLabel, enabled: bool) {
        if enabled {
            self.disabled_sets.remove(set);
        } else {
            self.disabled_sets.insert(set);
        }
    }

    pub fn set_schedule_error_policy(&mut self, policy: ScheduleErrorPolicy) {
        self.schedule_error_policy = policy;
        self.schedule_dirty = true;
    }

    pub fn set_system_panic_policy(&mut self, policy: SystemPanicPolicy) {
        self.system_panic_policy = policy;
    }

    pub(super) fn update(&mut self, dt: f32) {
        self.world.clear_change_tracking();
        // The world coordinate system is logical pixels; the GPU surface (gpu.config) is the
        // physical drawing buffer. Divide the buffer by the render scale to recover the logical
        // viewport (otherwise sprites/UI look half-size on Retina/HiDPI) and expose the scale as
        // DisplayScaleFactor so text renders at device resolution.
        // Native: render scale = the window DPR.
        // Wasm: the buffer is uniformly DPR-scaled (logical × devicePixelRatio, capped so neither
        // axis exceeds WebGL2's 2048 limit) by window.rs while the canvas CSS box stays logical, so
        // the render scale = buffer / logical (= DPR when uncapped, less when capped). Logical size
        // = the authored canvas attributes (WASM_LOGICAL_SIZE, captured in finish_init). (Pre-fix,
        // wasm forced scale = 1 on a logical-size buffer, which rendered correctly but soft on
        // Retina.)
        if let Some(gpu) = &self.gpu {
            #[cfg(not(target_arch = "wasm32"))]
            let scale_factor = self
                .window
                .as_ref()
                .map(|w| w.scale_factor() as f32)
                .unwrap_or(1.0)
                .max(1.0);
            #[cfg(target_arch = "wasm32")]
            let scale_factor = {
                // Logical size = the authored canvas attributes captured in finish_init (stable
                // across scene resets, unlike WindowConfig). render scale = buffer / logical.
                let logical_w = super::WASM_LOGICAL_SIZE.with(|c| c.get()).0;
                if logical_w >= 1 {
                    (gpu.config.width as f32 / logical_w as f32).max(1.0)
                } else {
                    1.0
                }
            };

            // In Docked mode the game camera and screen-space UI must see the central
            // viewport rect, not the full window. Compute the placeholder rect each
            // frame from the window's logical size; package 2 will overwrite
            // `editor.central_rect` with the real egui panel rect instead.
            //
            // The scale factor always tracks the real window DPR — only the logical
            // size reported to the game changes.
            #[cfg(not(target_arch = "wasm32"))]
            let viewport_size = {
                use crate::app::editor::docked_rt::compute_central_rect;
                use crate::app::editor::EditorMode;
                if self.editor.mode == EditorMode::Docked {
                    let win_logical_w = gpu.config.width as f32 / scale_factor;
                    let win_logical_h = gpu.config.height as f32 / scale_factor;
                    // Use the cached central_rect when package 2 writes it; otherwise
                    // recompute from the placeholder margins every frame.
                    let rect = self
                        .editor
                        .central_rect
                        .or_else(|| compute_central_rect(win_logical_w, win_logical_h));
                    match rect {
                        Some(r) => ViewportSize {
                            width: r.width(),
                            height: r.height(),
                        },
                        None => ViewportSize {
                            width: (win_logical_w
                                - crate::app::editor::docked_rt::MARGIN_LEFT
                                - crate::app::editor::docked_rt::MARGIN_RIGHT)
                                .max(1.0),
                            height: (win_logical_h
                                - crate::app::editor::docked_rt::MARGIN_TOP
                                - crate::app::editor::docked_rt::MARGIN_BOTTOM)
                                .max(1.0),
                        },
                    }
                } else {
                    ViewportSize {
                        width: gpu.config.width as f32 / scale_factor,
                        height: gpu.config.height as f32 / scale_factor,
                    }
                }
            };
            #[cfg(target_arch = "wasm32")]
            let viewport_size = ViewportSize {
                width: gpu.config.width as f32 / scale_factor,
                height: gpu.config.height as f32 / scale_factor,
            };

            self.world.insert_resource(viewport_size);
            self.world.insert_resource(DisplayScaleFactor(scale_factor));
        }

        // Begin egui frame
        let egui_ctx: Option<egui::Context> = {
            let window = self.window.as_ref();
            let state = self.egui_state.as_mut();
            if let (Some(window), Some(state)) = (window, state) {
                if let Some(debug_ui) = self.world.resource::<DebugUi>() {
                    let ctx = debug_ui.ctx().clone();
                    let raw_input = state.take_egui_input(window);
                    ctx.begin_pass(raw_input);
                    Some(ctx)
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Recompute the schedule (when labels/ordering changed)
        if self.schedule_dirty {
            // Safety: absorb systems pushed directly onto the scene
            if self.system_meta.len() != self.systems.len() {
                self.system_meta.resize(
                    self.systems.len(),
                    crate::ecs::schedule::SystemConfig::default(),
                );
            }
            match crate::ecs::schedule::compute_order(&self.system_meta) {
                Ok(order) => self.exec_order = order,
                Err(crate::ecs::schedule::ScheduleError::Cycle(remaining)) => {
                    match self.schedule_error_policy {
                        ScheduleErrorPolicy::LogAndFallback => {
                            log::error!(
                                "system order circular dependency detected — falling back to insertion order (affected indices: {remaining:?})"
                            );
                            self.exec_order = (0..self.systems.len()).collect();
                        }
                        ScheduleErrorPolicy::DisableRunOnCycle => {
                            log::error!(
                                "system order circular dependency detected — skipping user system execution (affected indices: {remaining:?})"
                            );
                            self.exec_order.clear();
                        }
                        ScheduleErrorPolicy::PanicOnCycle => {
                            panic!("system order circular dependency detected (affected indices: {remaining:?})");
                        }
                    }
                }
            }
            self.schedule_dirty = false;
        }

        // Editor pause: when Docked && paused && !step_once, restrict the execution set to
        // tail built-in systems only (e.g. HierarchySystem).  Built-ins occupy
        // `systems[systems.len() - builtin_tail_count..]`; their indices in exec_order are
        // therefore >= (systems.len() - builtin_tail_count).  Scene systems (smaller indices)
        // are skipped so the game simulation freezes, while hierarchy/gizmo updates keep running
        // so entities continue to follow gizmo drags even while paused.
        //
        // If `step_once` is set, run the full pipeline once and then clear `step_once`.
        #[cfg(not(target_arch = "wasm32"))]
        let pause_skip_scene = {
            use crate::app::editor::EditorMode;
            let is_docked_paused = self.editor.mode == EditorMode::Docked
                && self.editor.paused
                && !self.editor.step_once;
            if self.editor.mode == EditorMode::Docked && self.editor.paused && self.editor.step_once
            {
                // Step one full frame then return to paused.
                self.editor.step_once = false;
            }
            is_docked_paused
        };
        #[cfg(target_arch = "wasm32")]
        let pause_skip_scene = false;
        // First scene-system index = 0; tail starts at systems.len() - builtin_tail_count.
        let tail_start = self.systems.len().saturating_sub(self.builtin_tail_count);

        // Execute systems + profiler instrumentation (iterate exec_order, skip disabled sets)
        {
            let system_count = self.systems.len();
            let mut timings: Vec<(usize, &'static str, u64)> = Vec::with_capacity(system_count);
            // Take exec_order out of self so the loop body can mutably borrow self (via
            // catch_unwind / systems[i].run) without conflicting with a shared borrow of
            // self.exec_order.  The Vec is put back unchanged after the loop.
            // A recompute can only happen when schedule_dirty is set by add_system/
            // add_system_labeled — those are not callable from inside a running system — so
            // the Vec we restore is always still valid for the current frame.
            let order = std::mem::take(&mut self.exec_order);
            for i in order.iter().copied() {
                if i >= self.systems.len() {
                    continue;
                }
                // Skip systems disabled due to a prior panic
                if self.panicked_systems.contains(&i) {
                    continue;
                }
                // Editor pause: skip scene systems (indices < tail_start) while paused.
                // Tail built-ins (indices >= tail_start) always run so hierarchy stays live.
                if pause_skip_scene && i < tail_start {
                    continue;
                }
                if let Some(set) = self.system_meta.get(i).and_then(|m| m.set) {
                    if self.disabled_sets.contains(set) {
                        continue;
                    }
                }
                let name = self.systems[i].name();
                let t0 = Instant::now();
                // Panic isolation: wrap in catch_unwind so the engine keeps running if a system panics.
                // AssertUnwindSafe: World consistency is not guaranteed after a panic, but
                // disabling the offending system prevents further damage.
                // Note: does not work with panic = "abort" builds or FFI panics.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.systems[i].run(&mut self.world, dt);
                }));
                match result {
                    Ok(()) => {
                        timings.push((i, name, t0.elapsed().as_micros() as u64));
                    }
                    Err(panic) => {
                        let msg = format_panic_payload(&panic);
                        log::error!("system panic [{name}]: {msg}");
                        #[cfg(not(target_arch = "wasm32"))]
                        write_crash_log(name, &msg);
                        match self.system_panic_policy {
                            SystemPanicPolicy::DisableSystemAndContinue => {
                                self.panicked_systems.insert(i);
                                if let Some(ps) = self
                                    .world
                                    .resource_mut::<crate::resources::PanickedSystems>()
                                {
                                    ps.disabled.push(name.to_string());
                                }
                            }
                            SystemPanicPolicy::AbortAfterLog => std::panic::resume_unwind(panic),
                        }
                    }
                }
            }
            // Restore the order Vec (no allocation, just moves the pointer back).
            self.exec_order = order;
            if let Some(prof) = self.world.resource_mut::<crate::resources::ProfilerData>() {
                if prof.systems.len() != system_count {
                    prof.systems.clear();
                    prof.systems
                        .resize(system_count, crate::resources::SystemProfile::default());
                }
                for (i, name, us) in timings {
                    prof.record_system(i, name, us);
                }
                prof.frame_ms = dt * 1000.0;
            }
        }
        // Update camera effects (shake decay, zoom tween, smooth follow), then apply
        // world-bounds clamping so position never scrolls outside Camera::bounds.
        {
            let follow_pos = self
                .world
                .resource::<Camera>()
                .and_then(|cam| cam.follow_entity)
                .and_then(|e| self.world.get::<crate::components::Transform>(e))
                .map(|t| t.position);
            let viewport = self
                .world
                .resource::<ViewportSize>()
                .map(|v| (v.width, v.height));
            if let Some(cam) = self.world.resource_mut::<Camera>() {
                cam.update(dt, follow_pos);
                if let Some((vw, vh)) = viewport {
                    cam.clamp_to_bounds(vw, vh);
                }
            }
        }

        self.update_editor_ui(&egui_ctx, dt);

        // End egui frame + tessellate → hand off to render()
        if let Some(ctx) = egui_ctx {
            let ppp = self
                .window
                .as_ref()
                .map(|w| w.scale_factor() as f32)
                .unwrap_or(1.0);
            let full_output = ctx.end_pass();
            let paint_jobs = ctx.tessellate(full_output.shapes, ppp);
            let textures_delta = merge_textures_delta(
                self.egui_output.take().map(|(_, pending, _)| pending),
                full_output.textures_delta,
            );
            self.egui_output = Some((paint_jobs, textures_delta, ppp));
        }
        // Flush the event queue after all systems have run.
        // Must use std::mem::take to avoid conflicting borrows of &mut self.world.
        let flushers = std::mem::take(&mut self.event_flushers);
        for flush in &flushers {
            flush(&mut self.world);
        }
        self.event_flushers = flushers;
        if let Some(input) = self.world.resource_mut::<InputState>() {
            input.flush();
        }
        if let Some(gamepad) = self.world.resource_mut::<GamepadState>() {
            gamepad.flush();
        }
        if let Some(ts) = self.world.resource_mut::<TouchState>() {
            ts.flush();
        }
        // Process scene-transition command (after event/input flush)
        let cmd = self
            .world
            .resource_mut::<SceneChange>()
            .and_then(|sc| sc.0.take());
        if let Some(cmd) = cmd {
            self.apply_scene_cmd(cmd);
        }

        // Advance FadeTransition alpha
        if let Some(fade) = self
            .world
            .resource_mut::<crate::resources::FadeTransition>()
        {
            fade.update(dt);
        }

        // Hot reload: receive list of changed files and re-upload their GPU textures.
        let reloaded: Vec<String> = self
            .world
            .resource_mut::<AssetServer>()
            .map(|as_| as_.poll_reloads())
            .unwrap_or_default();
        if !reloaded.is_empty() {
            if let (Some(sr), Some(gpu)) = (&mut self.sprite_renderer, &self.gpu) {
                for path in &reloaded {
                    sr.reload_texture(&gpu.device, &gpu.queue, path);
                }
            }
        }

        // Hot-reload data tables: forward changed paths to the registry.
        #[cfg(not(target_arch = "wasm32"))]
        if !reloaded.is_empty() {
            if let Some(reg) = self
                .world
                .resource_mut::<crate::data_table::DataTableRegistry>()
            {
                for path in &reloaded {
                    reg.reload_path(path);
                }
            }
        }

        // Hot-reload animation clip sets: forward changed paths to the registry.
        #[cfg(not(target_arch = "wasm32"))]
        if !reloaded.is_empty() {
            if let Some(reg) = self
                .world
                .resource_mut::<crate::animation::clip_set::AnimationClipRegistry>()
            {
                for path in &reloaded {
                    reg.reload_path(path);
                }
            }
        }

        // Hot-reload particle configs: forward changed paths to the registry.
        #[cfg(not(target_arch = "wasm32"))]
        if !reloaded.is_empty() {
            if let Some(reg) = self
                .world
                .resource_mut::<crate::particle::ParticleConfigRegistry>()
            {
                for path in &reloaded {
                    reg.reload_path(path);
                }
            }
        }

        // Async load completion: upload finished assets to the GPU and update LoadProgress.
        let async_completed: Vec<(String, ImageAsset)> = self
            .world
            .resource_mut::<AssetServer>()
            .map(|as_| as_.poll_async_completions())
            .unwrap_or_default();
        if !async_completed.is_empty() {
            if let (Some(sr), Some(gpu)) = (&mut self.sprite_renderer, &self.gpu) {
                for (path, asset) in &async_completed {
                    sr.load_texture_from_image(&gpu.device, &gpu.queue, path, asset);
                }
            }
            // Update LoadProgress.loaded
            if let Some(prog) = self.world.resource_mut::<LoadProgress>() {
                prog.loaded += async_completed.len();
            }
        }
        self.upload_asset_server_images_to_gpu();
    }
}

// ── A6 editor-pause tests (native only) ─────────────────────────────────────
#[cfg(all(test, not(target_arch = "wasm32")))]
mod pause_tests {
    use super::super::*;
    use crate::app::editor::EditorMode;
    use crate::ecs::{System, World};

    #[derive(Default)]
    struct Counter(u32);

    struct CountSystem;
    impl System for CountSystem {
        fn run(&mut self, world: &mut World, _dt: f32) {
            world.resource_mut::<Counter>().unwrap().0 += 1;
        }
        fn name(&self) -> &'static str {
            "count_system"
        }
    }

    fn app_with_count() -> App {
        let mut app = App::new();
        app.world.insert_resource(Counter::default());
        app.add_system(CountSystem);
        app
    }

    /// In Docked+paused mode, scene systems (index < tail_start) are skipped.
    /// The builtin tail (HierarchySystem) still runs — but that just means the
    /// counter system (a scene system) must NOT increment.
    #[test]
    fn pause_skips_scene_systems_but_runs_tail() {
        let mut app = app_with_count();
        app.editor.mode = EditorMode::Docked;
        app.editor.paused = true;
        app.editor.step_once = false;

        // Run two frames while paused — counter must stay 0.
        app.update(1.0 / 60.0);
        app.update(1.0 / 60.0);

        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            0,
            "paused: scene system must not run"
        );
    }

    /// step_once=true causes the full pipeline (including scene systems) to run
    /// exactly once, then clears step_once (stays paused).
    #[test]
    fn step_once_runs_exactly_one_full_frame() {
        let mut app = app_with_count();
        app.editor.mode = EditorMode::Docked;
        app.editor.paused = true;
        app.editor.step_once = true;

        // First update: step fires (scene systems run), step_once cleared.
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            1,
            "step_once: scene system must run once"
        );
        assert!(
            !app.editor.step_once,
            "step_once must be cleared after the frame"
        );
        assert!(app.editor.paused, "still paused after step");

        // Second update: paused again, counter must not advance.
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            1,
            "second frame must not advance while paused"
        );
    }

    /// Transitioning away from Docked (F2 → Off) clears paused and step_once.
    #[test]
    fn f2_exit_clears_pause() {
        let mut app = app_with_count();
        app.editor.mode = EditorMode::Docked;
        app.editor.paused = true;
        app.editor.step_once = true;

        // Simulate the mode transition that F2 (or the Exit button) triggers.
        let new_mode = crate::app::editor::apply_f2(app.editor.mode);
        app.editor.mode = new_mode;
        // The window.rs code clears pause when leaving Docked; replicate that here.
        if app.editor.mode != EditorMode::Docked {
            app.editor.paused = false;
            app.editor.step_once = false;
        }

        assert_eq!(app.editor.mode, EditorMode::Off);
        assert!(!app.editor.paused, "pause must be cleared on exit");
        assert!(!app.editor.step_once, "step_once must be cleared on exit");

        // After exit, updates run normally.
        app.update(1.0 / 60.0);
        assert_eq!(
            app.world.resource::<Counter>().unwrap().0,
            1,
            "game must resume after docked exit"
        );
    }
}

#[cfg(test)]
mod egui_delta_tests {
    use super::merge_textures_delta;
    use egui::epaint::image::{ColorImage, ImageDelta};
    use egui::TextureId;

    #[test]
    fn pending_texture_deltas_are_merged_not_dropped() {
        let img = || ColorImage::filled([2, 2], egui::Color32::RED);
        let full = ImageDelta::full(img(), Default::default());
        let partial = ImageDelta::partial([1, 1], img(), Default::default());

        let mut older = egui::TexturesDelta::default();
        older.set.push((TextureId::Managed(0), full));
        let mut newer = egui::TexturesDelta::default();
        newer.set.push((TextureId::Managed(0), partial));
        newer.free.push(TextureId::Managed(7));

        let merged = merge_textures_delta(Some(older), newer);
        assert_eq!(merged.set.len(), 2, "both deltas must survive the merge");
        assert!(
            merged.set[0].1.pos.is_none(),
            "full allocation must be applied before the partial update"
        );
        assert!(merged.set[1].1.pos.is_some());
        assert_eq!(merged.free, vec![TextureId::Managed(7)]);
    }

    #[test]
    fn no_pending_delta_passes_through() {
        let mut newer = egui::TexturesDelta::default();
        newer.free.push(TextureId::Managed(3));
        let merged = merge_textures_delta(None, newer);
        assert!(merged.set.is_empty());
        assert_eq!(merged.free, vec![TextureId::Managed(3)]);
    }
}
