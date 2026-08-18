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

    /// Sets the global [`TimeScale`](crate::TimeScale) — the multiplier applied to the `dt` that
    /// gameplay (scene) systems receive. `1.0` = normal, `0.0` = frozen (hit-stop), `0.5` =
    /// slow-mo, `2.0` = fast-forward. Built-in/editor systems are unaffected.
    pub fn set_time_scale(&mut self, scale: f32) {
        if let Some(ts) = self.world.resource_mut::<crate::resources::TimeScale>() {
            ts.set(scale);
        }
    }

    /// Returns the current global time scale (`1.0` if the resource is missing).
    pub fn time_scale(&self) -> f32 {
        self.world
            .resource::<crate::resources::TimeScale>()
            .map(|t| t.get())
            .unwrap_or(1.0)
    }

    /// Advances the app by **one full frame of everything except rendering** — with no window, no
    /// GPU and no event loop.
    ///
    /// This is the frame step [`run`](Self::run) drives, minus the draw: scripted input is applied,
    /// the schedule runs, then the end-of-frame work happens — event/input flush, **scene-transition
    /// commands**, fade/transition ticks and hot-reload polling. Every GPU touch inside it is
    /// already guarded on a context being present, so a bare [`App::new`] advances fine without one.
    ///
    /// It exists because a `Scene` change is not a system: `SceneCmd::Replace` is consumed *after*
    /// the schedule, so a test that ticks systems by hand can never cross a scene boundary and never
    /// sees the `World` reset. Anything expressed as systems + resources can be driven directly;
    /// this is how the rest — the reset, and which resources survive it — gets driven too. Pair it
    /// with an [`InputScript`](crate::InputScript) to synthesize input, which is registered
    /// persistent so a run in progress survives the very transition under test.
    ///
    /// Do **not** call this from a game that also calls [`run`](Self::run) — `run` already steps the
    /// app, and a second call per frame would double-advance it. This is for headless drivers:
    /// acceptance tests, tools, and offline simulation.
    ///
    /// ```
    /// # use engine::{App, InputAction, InputScript, KeyCode};
    /// let mut app = App::new();
    /// app.set_input_script(InputScript::new([(1, InputAction::KeyPress(KeyCode::Enter))]));
    /// for _ in 0..10 {
    ///     app.step_headless(1.0 / 60.0);
    /// }
    /// ```
    pub fn step_headless(&mut self, dt: f32) {
        self.update(dt);
    }

    pub(super) fn update(&mut self, dt: f32) {
        self.world.clear_change_tracking();

        // 0. Scripted input playback — injected BEFORE the systems run and after last frame's
        // `input.flush()`, so a scripted press reads as `just_pressed` within its own frame,
        // exactly like a real one. No-op unless a script is installed.
        self.apply_input_script_frame();

        // 1. Viewport + scale-factor computation
        if self.gpu.is_some() {
            self.compute_viewport();
        }

        // 2. Egui frame begin
        let egui_ctx: Option<egui::Context> = {
            use super::egui_pass::begin_egui_frame;
            begin_egui_frame(
                self.window.as_deref(),
                self.render.egui_state.as_mut(),
                self.world.resource::<DebugUi>(),
            )
        };

        // 3–6. Schedule recompute, editor pause, system loop, camera update
        self.run_systems(dt, &egui_ctx);

        // 7–13. Editor UI, egui end, flush, scene cmd, fade, hot-reload, asset upload
        self.post_systems(dt, egui_ctx);
    }

    /// Concern 1 — Viewport + scale-factor computation.
    ///
    /// Reads `gpu.config` (physical buffer size) and the window DPR (native) or the authored
    /// canvas size (`WASM_LOGICAL_SIZE`, wasm) to derive the logical `ViewportSize` and
    /// `DisplayScaleFactor` resources that the rest of the frame reads.
    ///
    /// In Docked editor mode (native), the viewport is narrowed to the central panel rect so the
    /// game camera and screen-space UI do not see the panel chrome.
    ///
    /// Caller must ensure `self.gpu.is_some()` before calling.
    fn compute_viewport(&mut self) {
        let gpu = self
            .gpu
            .as_ref()
            .expect("compute_viewport called without gpu");
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
        let win_logical_w = gpu.config.width as f32 / scale_factor;
        let win_logical_h = gpu.config.height as f32 / scale_factor;

        // Optional fixed design (virtual) resolution: when set, the game authors at this size and
        // the engine reports it as ViewportSize + letterboxes the content into the real window.
        // `.copied()` drops the immutable World borrow before the inserts below.
        let design = self
            .world
            .resource::<DesignResolution>()
            .copied()
            .filter(|d| d.width > 0.0 && d.height > 0.0);

        // Maps the raw window-logical size to (ViewportSize, Letterbox), applying the design
        // resolution when present. Used by the non-editor render paths (native + wasm).
        let apply_design = |win_w: f32, win_h: f32| match design {
            Some(d) => (
                ViewportSize {
                    width: d.width,
                    height: d.height,
                },
                Letterbox::compute(d.width, d.height, win_w, win_h),
            ),
            None => (
                ViewportSize {
                    width: win_w,
                    height: win_h,
                },
                Letterbox::IDENTITY,
            ),
        };

        #[cfg(not(target_arch = "wasm32"))]
        let (viewport_size, letterbox) = {
            use crate::app::editor::docked_rt::compute_central_rect;
            use crate::app::editor::EditorMode;
            if self.editor.mode == EditorMode::Docked {
                // The docked editor owns the viewport (the central panel rect); the design
                // resolution does not apply there.
                let rect = self
                    .editor
                    .central_rect
                    .or_else(|| compute_central_rect(win_logical_w, win_logical_h));
                let vp = match rect {
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
                };
                (vp, Letterbox::IDENTITY)
            } else {
                apply_design(win_logical_w, win_logical_h)
            }
        };
        #[cfg(target_arch = "wasm32")]
        let (viewport_size, letterbox) = apply_design(win_logical_w, win_logical_h);

        self.world.insert_resource(viewport_size);
        self.world.insert_resource(letterbox);
        self.world.insert_resource(DisplayScaleFactor(scale_factor));
    }

    /// Concerns 3–6 — Schedule recompute, editor pause logic, system execution loop,
    /// camera update + bounds clamp.
    ///
    /// `egui_ctx` is passed through so systems that want to draw egui widgets have access
    /// to the live `egui::Context`.
    fn run_systems(&mut self, dt: f32, egui_ctx: &Option<egui::Context>) {
        let _ = egui_ctx; // held for future use; systems access DebugUi via the World resource

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
                                "system order circular dependency detected — falling back to insertion order. Blocked \
                                 systems (the cycle PLUS everything downstream of it, not the cycle \
                                 alone): {remaining:?} — look for the mutual before/after pair among them"
                            );
                            self.exec_order = (0..self.systems.len()).collect();
                        }
                        ScheduleErrorPolicy::DisableRunOnCycle => {
                            log::error!(
                                "system order circular dependency detected — skipping user system execution. Blocked \
                                 systems (the cycle PLUS everything downstream of it, not the cycle \
                                 alone): {remaining:?} — look for the mutual before/after pair among them"
                            );
                            self.exec_order.clear();
                        }
                        ScheduleErrorPolicy::PanicOnCycle => {
                            panic!("system order circular dependency detected. Blocked systems (the cycle PLUS everything downstream of it, not the cycle alone): {remaining:?} — look for the mutual before/after pair among them");
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

        // Global time-scale: scene systems (indices < tail_start) receive a scaled dt so games
        // can hit-stop / slow-mo / fast-forward by setting the TimeScale resource. Built-in tail
        // systems (hierarchy/gizmo) keep real dt so the editor stays responsive at time_scale == 0.
        let time_scale = self
            .world
            .resource::<crate::resources::TimeScale>()
            .map(|t| t.get())
            .unwrap_or(1.0);
        let scaled_dt = dt * time_scale;
        // Expose the real (unscaled) dt so a system can opt out of time-scaling (e.g. a hit-stop
        // controller that sets TimeScale(0) still needs real time to end its own freeze).
        if let Some(rd) = self.world.resource_mut::<crate::resources::RealDt>() {
            rd.0 = dt;
        }

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
                // AssertUnwindSafe: World consistency is not guaranteed after a panic.
                //
                // On panic this frame:
                //   DisableSystemAndContinue — records the system as permanently disabled for
                //   future frames, then **aborts the remaining systems for the current frame**
                //   (breaks out of the loop). Subsequent systems are skipped because the World
                //   may be half-mutated; they run normally next frame.
                //   AbortAfterLog — re-panics immediately (no frame-abort needed).
                //
                // Note: does not work with panic = "abort" builds or FFI panics.
                // Scene systems get time-scaled dt; built-in tail systems get real dt.
                let sys_dt = if i < tail_start { scaled_dt } else { dt };
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.systems[i].run(&mut self.world, sys_dt);
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
                                // Abort remaining systems this frame: the World may be
                                // half-mutated after the panic, so running subsequent
                                // systems risks cascading corruption. They resume normally
                                // next frame (the panicked system is now permanently skipped).
                                break;
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
    }

    /// Concerns 7–13 — editor UI, egui frame end, event/input flush, scene transition,
    /// fade tick, hot-reload, async asset upload.
    ///
    /// Takes ownership of `egui_ctx` so it can be consumed by [`end_egui_frame`].
    fn post_systems(&mut self, dt: f32, egui_ctx: Option<egui::Context>) {
        // 7. Keep the Tile Paint swatch atlas registered with egui before building the UI.
        #[cfg(not(target_arch = "wasm32"))]
        self.register_paint_atlas_texture();

        self.update_editor_ui(&egui_ctx, dt);

        // 8. End egui frame + tessellate → hand off to render()
        if let Some(ctx) = egui_ctx {
            use super::egui_pass::end_egui_frame;
            end_egui_frame(ctx, self.window.as_deref(), &mut self.render.egui_output);
        }

        // 9. Flush the event queue after all systems have run.
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

        // 10. Process scene-transition command (after event/input flush)
        let cmd = self
            .world
            .resource_mut::<SceneChange>()
            .and_then(|sc| sc.0.take());
        if let Some(cmd) = cmd {
            self.apply_scene_cmd(cmd);
        }

        // 11. Advance FadeTransition alpha
        if let Some(fade) = self
            .world
            .resource_mut::<crate::resources::FadeTransition>()
        {
            // On wasm the fade renderer field is absent and the fade effect is not rendered.
            // Log a one-time warning so developers know fades are visual no-ops on wasm.
            #[cfg(target_arch = "wasm32")]
            {
                use std::sync::OnceLock;
                static WARNED: OnceLock<()> = OnceLock::new();
                WARNED.get_or_init(|| {
                    log::warn!(
                        "FadeTransition: fade rendering is not supported on wasm (native-only). \
                         The FadeTransition resource is updated but no fade effect is rendered."
                    );
                });
            }
            fade.update(dt);
        }

        // 11b. Advance the styled SceneTransition: swap the pending scene at full cover (while
        // hidden), then drop the resource once the reveal finishes.
        let (just_covered, is_done) = if let Some(t) =
            self.world
                .resource_mut::<crate::scene_transition::SceneTransition>()
        {
            t.update(dt);
            (t.just_covered(), t.is_done())
        } else {
            (false, false)
        };
        if just_covered {
            let cmd = self
                .world
                .resource_mut::<crate::scene_transition::PendingSceneTransition>()
                .and_then(|p| p.0.take());
            if let Some(cmd) = cmd {
                self.apply_scene_cmd(cmd);
            }
        }
        if is_done {
            self.world
                .remove_resource::<crate::scene_transition::SceneTransition>();
            self.world
                .remove_resource::<crate::scene_transition::PendingSceneTransition>();
        }

        // 12. Hot reload: receive list of changed files and re-upload their GPU textures.
        let reloaded: Vec<String> = self
            .world
            .resource_mut::<AssetServer>()
            .map(|as_| as_.poll_reloads())
            .unwrap_or_default();
        if !reloaded.is_empty() {
            if let (Some(sr), Some(gpu)) = (&mut self.render.sprite_renderer, &self.gpu) {
                for path in &reloaded {
                    sr.reload_texture(&gpu.device, &gpu.queue, path);
                }
            }
        }

        // Hot-reload RON registries: forward changed paths to each registered registry.
        // Registries are registered via `App::register_hot_reloadable`; the three built-ins
        // (DataTableRegistry, AnimationClipRegistry, ParticleConfigRegistry) are auto-registered
        // in `App::new`. Forkers can add their own registries without editing engine internals.
        #[cfg(not(target_arch = "wasm32"))]
        if !reloaded.is_empty() {
            // Clone the fn-pointer vec to avoid a simultaneous borrow of `self.world`.
            let forwarders = self.hot_reload_forwarders.clone();
            for f in &forwarders {
                f(&mut self.world, &reloaded);
            }
        }

        // 13. Async load completion: upload finished assets to the GPU and update LoadProgress.
        let async_completed: Vec<(String, ImageAsset)> = self
            .world
            .resource_mut::<AssetServer>()
            .map(|as_| as_.poll_async_completions())
            .unwrap_or_default();
        if !async_completed.is_empty() {
            if let (Some(sr), Some(gpu)) = (&mut self.render.sprite_renderer, &self.gpu) {
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

    #[derive(Default)]
    struct LastDt(f32);

    struct RecordDtSystem;
    impl System for RecordDtSystem {
        fn run(&mut self, world: &mut World, dt: f32) {
            world.resource_mut::<LastDt>().unwrap().0 = dt;
        }
        fn name(&self) -> &'static str {
            "record_dt"
        }
    }

    /// A scene system receives `dt * time_scale`; `0.0` freezes it (hit-stop).
    #[test]
    fn time_scale_scales_scene_system_dt() {
        let mut app = App::new();
        app.world.insert_resource(LastDt::default());
        app.add_system(RecordDtSystem);

        // Default time scale is 1.0 — dt passes through unchanged.
        app.update(1.0);
        assert!((app.world.resource::<LastDt>().unwrap().0 - 1.0).abs() < 1e-6);
        assert!((app.time_scale() - 1.0).abs() < 1e-6);

        // 0.5 → slow motion.
        app.set_time_scale(0.5);
        app.update(1.0);
        let got = app.world.resource::<LastDt>().unwrap().0;
        assert!(
            (got - 0.5).abs() < 1e-6,
            "expected scaled dt 0.5, got {got}"
        );

        // 0.0 → frozen.
        app.set_time_scale(0.0);
        app.update(1.0);
        let got0 = app.world.resource::<LastDt>().unwrap().0;
        assert!(
            got0.abs() < 1e-6,
            "time_scale 0 should give dt 0, got {got0}"
        );

        // Negative is clamped to 0 on read.
        app.set_time_scale(-3.0);
        assert!((app.time_scale() - 0.0).abs() < 1e-6);
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
