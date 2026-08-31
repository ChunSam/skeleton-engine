use super::*;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

// Native frame-pacing policy. The redraw `WaitUntil` cadence is derived from the monitor refresh
// rate; these bounds keep a bogus/adaptive reading from pacing the loop too slowly (or absurdly
// fast). `AutoVsync` still does the final presentation pacing — this only bounds the wake cadence.
// Promoted from inline literals; a future `FramePacingConfig` (low-power idle / 30 fps cap for
// editor forks) can lift these without changing behavior. Native-only (wasm uses rAF).
#[cfg(not(target_arch = "wasm32"))]
mod frame_pacing {
    /// Used when the monitor reports no usable refresh rate.
    pub(super) const FALLBACK_REFRESH_HZ: f64 = 60.0;
    /// A reported rate below this is treated as bogus and ignored (→ fallback).
    pub(super) const MIN_VALID_REFRESH_HZ: f64 = 1.0;
    /// Lower clamp: never pace the redraw cadence below 60 fps, even on a reduced-rate panel.
    pub(super) const MIN_REFRESH_HZ: f64 = 60.0;
    /// Upper clamp: cap the wake cadence so an extreme reading can't busy-spin the loop.
    pub(super) const MAX_REFRESH_HZ: f64 = 240.0;
}

impl ApplicationHandler for App {
    /// Called when the app becomes active (macOS: Resumed; other platforms: once at startup).
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (init_w, init_h, title) = self
            .world
            .resource::<WindowConfig>()
            .map(|c| (c.width, c.height, c.title.clone()))
            .unwrap_or((1280, 720, "Game".to_string()));
        // Optional window behavior (resizable / fullscreen / aspect-lock). Absent = the
        // default resizable windowed window, so games that never insert it are unaffected.
        let opts = self
            .world
            .resource::<WindowOptions>()
            .cloned()
            .unwrap_or_default();
        let attrs = Window::default_attributes()
            .with_title(&title)
            .with_inner_size(winit::dpi::LogicalSize::new(init_w, init_h))
            .with_resizable(opts.resizable);
        let attrs = match opts.mode {
            WindowMode::Windowed => attrs,
            // `Borderless(None)` = borderless fullscreen on the current monitor (no mode switch).
            WindowMode::BorderlessFullscreen => {
                attrs.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
            }
        };

        // WASM: attach the <canvas id="game-canvas"> element in the HTML page to the winit window.
        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            if let Some(canvas) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(crate::DEFAULT_CANVAS_ID))
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            {
                attrs.with_canvas(Some(canvas))
            } else {
                attrs
            }
        };

        let window = match event_loop.create_window(attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                log::error!("window creation failed: {err}");
                event_loop.exit();
                return;
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            match pollster::block_on(GpuContext::new(window.clone())) {
                Ok(gpu) => self.finish_init(gpu, window),
                Err(err) => {
                    log::error!("GPU initialization failed: {err}");
                    event_loop.exit();
                }
            }
        }

        // WASM: the WebGPU/WebGL2 adapter request is Promise-based, so handle it
        // asynchronously with spawn_local. When the GPU is ready it is stored in
        // PENDING_GPU thread_local → finish_init is called from about_to_wait().
        #[cfg(target_arch = "wasm32")]
        {
            self.window = Some(window.clone());
            wasm_bindgen_futures::spawn_local(async move {
                match GpuContext::new(window.clone()).await {
                    Ok(gpu) => {
                        PENDING_GPU.with(|p| {
                            *p.borrow_mut() = Some((gpu, window));
                        });
                    }
                    Err(err) => log::error!("GPU initialization failed: {err}"),
                }
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Forward the event to egui first
        if let (Some(state), Some(window)) = (&mut self.render.egui_state, &self.window) {
            let _ = state.on_window_event(window, &event);
        }

        match event {
            // ── Close window ──────────────────────────────────────────────────────
            WindowEvent::CloseRequested => event_loop.exit(),

            // ── Window resize ─────────────────────────────────────────────────
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    // WASM: keep the drawing buffer at the logical size × devicePixelRatio (uniform
                    // scale, capped so neither axis exceeds WebGL2's 2048 max texture size) for a
                    // crisp Retina render, and keep the canvas buffer attributes in sync with the
                    // surface. The CSS display box stays at the logical size. Logical size comes from
                    // WASM_LOGICAL_SIZE (the authored canvas attributes captured in finish_init); a
                    // genuine resize only changes the DPR, not the logical game size.
                    #[cfg(target_arch = "wasm32")]
                    let size = {
                        use wasm_bindgen::JsCast;
                        let dpr = web_sys::window()
                            .map(|w| w.device_pixel_ratio())
                            .unwrap_or(1.0)
                            .max(1.0);
                        let (logical_w, logical_h) = super::WASM_LOGICAL_SIZE.with(|c| c.get());
                        if logical_w >= 1 && logical_h >= 1 {
                            let scale = dpr
                                .min(2048.0 / logical_w as f64)
                                .min(2048.0 / logical_h as f64);
                            let buf_w = ((logical_w as f64 * scale).round() as u32).clamp(1, 2048);
                            let buf_h = ((logical_h as f64 * scale).round() as u32).clamp(1, 2048);
                            if let Some(c) = web_sys::window()
                                .and_then(|w| w.document())
                                .and_then(|d| d.get_element_by_id(crate::DEFAULT_CANVAS_ID))
                                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                            {
                                c.set_width(buf_w);
                                c.set_height(buf_h);
                            }
                            winit::dpi::PhysicalSize::new(buf_w, buf_h)
                        } else {
                            size
                        }
                    };
                    gpu.resize(size);
                }
                // Aspect-ratio lock (native): when WindowOptions requests a locked ratio,
                // correct the inner size back to it (height re-derived from the new width).
                // request_inner_size triggers another Resized whose height already matches the
                // ratio, so this converges in a single step — no feedback loop.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let lock = self
                        .world
                        .resource::<WindowOptions>()
                        .and_then(|o| o.resizable.then_some(o.lock_aspect).flatten());
                    if let (Some(ratio), Some(window)) = (lock, &self.window) {
                        if ratio > 0.0 && size.width > 0 {
                            let want_h = ((size.width as f32 / ratio).round() as u32).max(1);
                            if want_h.abs_diff(size.height) > 1 {
                                let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(
                                    size.width, want_h,
                                ));
                            }
                        }
                    }
                }
                // Render one frame during a live resize drag to reduce visual stutter.
                // (On macOS, Resized fires during the modal resize loop even when RedrawRequested stalls.)
                // Guard: if RedrawRequested already called step_frame this iteration, skip here
                // to prevent double-stepping physics/tween/timer on macOS modal resize.
                #[cfg(not(target_arch = "wasm32"))]
                self.step_frame_once(event_loop);
            }

            // ── Keyboard input ──────────────────────────────────────────────────
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        logical_key,
                        state,
                        ..
                    },
                ..
            } => {
                // F1 → Overlay toggle; F2 → Docked toggle.
                // Both keys are native-only: wasm has no docked mode and keeps the
                // original DebugUi.toggle() path.
                //
                // Transition table (native):
                //   F1: Off→Overlay, Overlay→Off, Docked→Overlay
                //   F2: Off→Docked, Overlay→Docked (turns Overlay off), Docked→Off
                //
                // DebugUi.enabled is kept in sync so systems that query is_enabled()
                // continue to work in Overlay mode.
                #[cfg(not(target_arch = "wasm32"))]
                if state == ElementState::Pressed
                    && (key == winit::keyboard::KeyCode::F1 || key == winit::keyboard::KeyCode::F2)
                {
                    let old_mode = self.editor.mode;
                    let new_mode = if key == winit::keyboard::KeyCode::F1 {
                        crate::app::editor::apply_f1(self.editor.mode)
                    } else {
                        crate::app::editor::apply_f2(self.editor.mode)
                    };
                    self.editor.mode = new_mode;
                    // Persist editor preferences across docked sessions: load the saved
                    // settings the first time the docked editor opens, save them on close.
                    let was_docked = old_mode == crate::app::editor::EditorMode::Docked;
                    let is_docked = new_mode == crate::app::editor::EditorMode::Docked;
                    if is_docked && !was_docked && !self.editor.settings_loaded {
                        self.load_editor_settings();
                        self.editor.settings_loaded = true;
                    }
                    if was_docked && !is_docked {
                        self.save_editor_settings();
                    }
                    // Exiting Docked mode clears pause state so the game resumes.
                    if new_mode != crate::app::editor::EditorMode::Docked {
                        self.editor.paused = false;
                        self.editor.step_once = false;
                    }
                    // Sync DebugUi.enabled: true only in Overlay mode.
                    if let Some(debug_ui) = self.world.resource_mut::<DebugUi>() {
                        debug_ui.set_enabled(new_mode == crate::app::editor::EditorMode::Overlay);
                    }
                }
                // WASM: keep the original F1 = DebugUi.toggle() behaviour (no EditorMode).
                #[cfg(target_arch = "wasm32")]
                if key == winit::keyboard::KeyCode::F1 && state == ElementState::Pressed {
                    if let Some(debug_ui) = self.world.resource_mut::<DebugUi>() {
                        debug_ui.toggle();
                    }
                }
                // Keyboard: in Docked mode suppress game keys when egui wants keyboard input
                // (e.g. typing in a text field must not move the player character).
                // F1/F2 are handled above and are always engine-side — they are not passed
                // to InputState regardless of mode.
                #[cfg(not(target_arch = "wasm32"))]
                let egui_wants_keyboard = {
                    use crate::app::editor::EditorMode;
                    self.editor.mode == EditorMode::Docked
                        && self
                            .world
                            .resource::<crate::debug_ui::DebugUi>()
                            .map(|d| d.ctx().egui_wants_keyboard_input())
                            .unwrap_or(false)
                };
                #[cfg(target_arch = "wasm32")]
                let egui_wants_keyboard = false;

                if !egui_wants_keyboard {
                    if let Some(input) = self.world.resource_mut::<InputState>() {
                        match state {
                            ElementState::Pressed => {
                                input.press(key);
                                use winit::keyboard::{Key, NamedKey};
                                match &logical_key {
                                    Key::Character(s) => {
                                        for c in s.chars() {
                                            input.push_char(c);
                                        }
                                    }
                                    Key::Named(NamedKey::Backspace) => input.push_backspace(),
                                    Key::Named(NamedKey::Enter) => input.push_enter(),
                                    _ => {}
                                }
                            }
                            ElementState::Released => input.release(key),
                        }
                    }
                }
            }

            WindowEvent::Ime(winit::event::Ime::Preedit(text, _cursor)) => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    input.set_ime_preedit(text);
                }
            }

            WindowEvent::Ime(winit::event::Ime::Commit(text)) => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    input.push_text(&text);
                    input.clear_ime_preedit();
                }
            }

            // ── Mouse cursor move ─────────────────────────────────────────────
            // winit provides physical pixels, but UI hit-testing, `ViewportSize`, and
            // `Camera::screen_to_world` all work in logical pixels. Divide by the
            // scale factor to store logical coordinates so HiDPI (e.g. Retina 2×)
            // does not cause a mismatch.
            //
            // In Docked mode the game's InputState receives the cursor translated into
            // central-panel-local coordinates via `viewport_to_game`.  When the cursor
            // is outside the central panel the game cursor position is frozen at its
            // last valid in-panel position (no update is issued), so systems that read
            // `input.cursor()` keep the last known in-panel value.
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                let logical = Vec2::new(position.x as f32 / scale, position.y as f32 / scale);

                // Always track the untranslated window-space cursor; the docked pointer gate
                // needs the physical position even while the game cursor is frozen outside
                // the panel. Editor bookkeeping, so it stays out of `game_cursor`.
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.editor.window_cursor = Some(egui::pos2(logical.x, logical.y));
                }

                if let Some(cursor) = self.game_cursor(logical) {
                    if let Some(input) = self.world.resource_mut::<InputState>() {
                        input.set_cursor(cursor);
                    }
                }
            }

            // ── Mouse button ──────────────────────────────────────────────────
            // In Docked mode: buttons only pass through when egui does NOT want the
            // pointer AND the cursor is inside the central panel.
            WindowEvent::MouseInput { state, button, .. } => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use crate::app::editor::EditorMode;
                    if self.editor.mode == EditorMode::Docked {
                        // Layer-aware gate: inside the central rect, egui idle, and no
                        // popup/window floating above the viewport. (The previous
                        // `egui_wants_pointer_input()` check is wrong here — the game
                        // viewport IS an egui CentralPanel, so egui always "wants" it.)
                        let allowed = {
                            let ctx = self
                                .world
                                .resource::<crate::debug_ui::DebugUi>()
                                .map(|d| d.ctx().clone());
                            crate::app::editor::docked_rt::docked_game_pointer_allowed(
                                self.editor.window_cursor,
                                self.editor.central_rect,
                                ctx.as_ref(),
                            )
                        };
                        if allowed {
                            if let Some(input) = self.world.resource_mut::<InputState>() {
                                match state {
                                    ElementState::Pressed => input.press_mouse(button),
                                    ElementState::Released => input.release_mouse(button),
                                }
                            }
                        }
                        // Outside panel or egui wants pointer: game sees no click.
                        // Ensure any stale pressed state is cleared on release to prevent
                        // stuck buttons when the cursor moves out while held (press-inside,
                        // release-outside scenario).  Only fire when !allowed to avoid
                        // double-releasing when allowed already handled it above.
                        if !allowed && state == ElementState::Released {
                            if let Some(input) = self.world.resource_mut::<InputState>() {
                                input.release_mouse(button);
                            }
                        }
                    } else {
                        if let Some(input) = self.world.resource_mut::<InputState>() {
                            match state {
                                ElementState::Pressed => input.press_mouse(button),
                                ElementState::Released => input.release_mouse(button),
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match state {
                        ElementState::Pressed => input.press_mouse(button),
                        ElementState::Released => input.release_mouse(button),
                    }
                }
            }

            // ── Mouse wheel ────────────────────────────────────────────────────
            // In Docked mode scroll only passes through when the cursor is inside the
            // central panel and egui does not want the pointer.
            WindowEvent::MouseWheel { delta, .. } => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    use crate::app::editor::EditorMode;
                    if self.editor.mode == EditorMode::Docked {
                        // Same layer-aware gate as MouseInput above.
                        let allowed = {
                            let ctx = self
                                .world
                                .resource::<crate::debug_ui::DebugUi>()
                                .map(|d| d.ctx().clone());
                            crate::app::editor::docked_rt::docked_game_pointer_allowed(
                                self.editor.window_cursor,
                                self.editor.central_rect,
                                ctx.as_ref(),
                            )
                        };
                        if allowed {
                            if let Some(input) = self.world.resource_mut::<InputState>() {
                                match delta {
                                    MouseScrollDelta::LineDelta(_, y) => input.add_scroll(y),
                                    MouseScrollDelta::PixelDelta(p) => {
                                        input.add_scroll(p.y as f32 / 20.0)
                                    }
                                }
                            }
                        }
                    } else {
                        if let Some(input) = self.world.resource_mut::<InputState>() {
                            match delta {
                                MouseScrollDelta::LineDelta(_, y) => input.add_scroll(y),
                                MouseScrollDelta::PixelDelta(p) => {
                                    input.add_scroll(p.y as f32 / 20.0)
                                }
                            }
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match delta {
                        MouseScrollDelta::LineDelta(_, y) => input.add_scroll(y),
                        // Convert pixel-delta scroll (trackpad etc.) to lines: 20px ≈ 1 line (empirical)
                        MouseScrollDelta::PixelDelta(p) => input.add_scroll(p.y as f32 / 20.0),
                    }
                }
            }

            // ── Touch input ────────────────────────────────────────────────────
            // winit delivers touch locations in physical pixels, so they need the same two
            // steps the mouse path takes: divide by the window scale factor, then map into
            // the game's cursor space via `game_cursor`.
            WindowEvent::Touch(winit::event::Touch {
                phase,
                location,
                id,
                ..
            }) => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                // Convert physical → logical pixels, then through the SAME mapping the mouse
                // takes, so TouchState positions really do match InputState::cursor() and
                // Camera::screen_to_world — under a `DesignResolution` they did not, because
                // this arm stopped at the scale division. See `App::game_cursor`.
                //
                // Unlike the mouse, a touch does not freeze when `game_cursor` declines: the
                // `None` case is the docked editor with the pointer outside the central panel,
                // and swallowing a `Ended` there would leak a stuck touch id into `TouchState`.
                // Falling back to the unmapped position keeps the editor's behaviour exactly
                // as it was; the defect being fixed is in the letterboxed game path.
                let logical = Vec2::new(location.x as f32 / scale, location.y as f32 / scale);
                let pos = self.game_cursor(logical).unwrap_or(logical);
                if let Some(ts) = self.world.resource_mut::<TouchState>() {
                    match phase {
                        winit::event::TouchPhase::Started => ts.on_touch_started(id, pos),
                        winit::event::TouchPhase::Moved => ts.on_touch_moved(id, pos),
                        winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                            ts.on_touch_ended(id, pos)
                        }
                    }
                }
                // Emulate touch as left mouse button (for compatibility with existing UI systems).
                // `pos` is the mapped position from above — the same value the mouse would
                // deliver for this physical point.
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match phase {
                        winit::event::TouchPhase::Started => {
                            input.set_cursor(pos);
                            input.press_mouse(winit::event::MouseButton::Left);
                        }
                        winit::event::TouchPhase::Moved => input.set_cursor(pos),
                        winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => {
                            input.release_mouse(winit::event::MouseButton::Left);
                        }
                    }
                }
            }

            // ── Frame render ──────────────────────────────────────────────────
            WindowEvent::RedrawRequested => {
                // WASM: also check here in case the GPU is not ready by the time about_to_wait fires
                #[cfg(target_arch = "wasm32")]
                if self.gpu.is_none() {
                    if let Some((gpu, window)) = PENDING_GPU.with(|p| p.borrow_mut().take()) {
                        self.finish_init(gpu, window);
                    }
                }

                // Run update + render together inside one RedrawRequested handler.
                // (Splitting update into about_to_wait introduced a one-frame delay that
                // made input feel sluggish. Click accuracy is preserved by recording the
                // cursor position at press/release time.) The same sequence is reused in Resized.
                self.step_frame_once(event_loop);
            }

            // ── Focus loss ────────────────────────────────────────────────────
            // When the window loses focus the OS stops delivering key-up events.
            // Flush all held keys immediately so the game does not see phantom
            // held state (e.g. a character moving forever after Alt-Tab).
            WindowEvent::Focused(false) => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    input.release_all();
                }
            }

            _ => {}
        }
    }

    /// Called when the event queue is empty → poll gamepads, then schedule the next frame.
    ///
    /// Native: uses `ControlFlow::WaitUntil(next_frame)` so the loop SLEEPS between frames
    /// instead of busy-spinning under `Poll`. This gives the macOS main run loop idle time
    /// (smooth window drag, prompt click/input handling) while still guaranteeing a frame at
    /// the refresh cadence — fixing the prior `Wait`-only attempt that lagged on idle drags
    /// because it only woke on input. Input events wake the loop immediately regardless.
    #[cfg_attr(target_arch = "wasm32", allow(unused_variables))]
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Reset per-iteration step guard: each new event-loop iteration may step at most once.
        // This prevents Resized + RedrawRequested from both calling step_frame in one iteration.
        self.stepped_this_iteration = false;

        #[cfg(not(target_arch = "wasm32"))]
        self.poll_gilrs();

        // macOS: gilrs is blind to GameController-claimed pads, so poll that framework instead.
        #[cfg(target_os = "macos")]
        if let Some(state) = self.world.resource_mut::<GamepadState>() {
            crate::input::gamepad_macos::poll(state);
        }

        // WASM: detect completion of the async GPU initialization started by spawn_local.
        #[cfg(target_arch = "wasm32")]
        if self.gpu.is_none() {
            if let Some((gpu, window)) = PENDING_GPU.with(|p| p.borrow_mut().take()) {
                self.finish_init(gpu, window);
            }
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let now = Instant::now();
            let next = *self.next_frame.get_or_insert(now);
            let next = if now >= next {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
                // Advance to the next boundary; resync if we fell behind so a stall
                // (e.g. a long modal resize) doesn't trigger a burst of catch-up frames.
                let advanced = next + self.frame_interval;
                let advanced = if advanced < now {
                    now + self.frame_interval
                } else {
                    advanced
                };
                self.next_frame = Some(advanced);
                advanced
            } else {
                next
            };
            event_loop.set_control_flow(ControlFlow::WaitUntil(next));
        }

        // WASM: rAF-driven — just request a redraw each iteration.
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl App {
    /// Maps a window-logical pointer position into the space [`InputState::cursor`] is documented
    /// to be in, or `None` when the game cursor must not move at all.
    ///
    /// **Every pointer source goes through here.** The mouse and touch arms each used to do their
    /// own mapping, and only the mouse arm learned about [`Letterbox`] — so under a
    /// `DesignResolution` a finger reported window space while a mouse reported design space,
    /// through the one accessor `Camera::screen_to_world` documents as its input. One function with
    /// two callers is the point of this, not the arithmetic.
    /// The docked central-panel rect in logical points, or `None` when the window leaves no room
    /// for one.
    ///
    /// Thin wrapper over `docked_rt::docked_viewport` — the same call `App::compute_viewport` and
    /// `prepare_docked_scene_view` branch on. Those two derive the window's logical size where
    /// they stand because neither has `&self` in hand; what is shared is the **decision**, which
    /// is the half that was wrong. Before v0.156.3 this call site had a third answer of its own:
    /// it consulted only `central_rect` and passed the raw window position through when egui had
    /// not published one yet, so during the first frames of a session the scene rendered into the
    /// fallback viewport while the cursor was read in window space.
    ///
    /// `None` with no surface yet: there is no frame to map into, which is the same verdict this
    /// returns for a window too small to hold a central panel.
    #[cfg(not(target_arch = "wasm32"))]
    fn docked_central_rect(&self) -> Option<egui::Rect> {
        let scale = self
            .window
            .as_ref()
            .map(|w| w.scale_factor() as f32)
            .unwrap_or(1.0)
            .max(1.0);
        // Only the *fallback* needs the surface. With no gpu yet a published rect must still map,
        // so pass a zero window rather than declining outright: `compute_central_rect` refuses it
        // and `docked_viewport` falls back to nothing, which is the honest answer for a frame
        // that has no surface to be rendered into.
        let (win_logical_w, win_logical_h) = match self.gpu.as_ref() {
            Some(gpu) => (
                gpu.config.width as f32 / scale,
                gpu.config.height as f32 / scale,
            ),
            None => (0.0, 0.0),
        };
        crate::app::editor::docked_rt::docked_viewport(
            self.editor.central_rect,
            win_logical_w,
            win_logical_h,
            scale,
        )
        .map(|(rect, _)| rect)
    }

    fn game_cursor(&self, logical: Vec2) -> Option<Vec2> {
        // The docked editor keeps the untranslated cursor: the central panel has its own
        // `viewport_to_game` mapping, and a design resolution does not apply inside it.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::app::editor::{docked_rt::viewport_to_game, EditorMode};
            if self.editor.mode == EditorMode::Docked {
                return match self.docked_central_rect() {
                    // Outside the central rect the game cursor freezes at its last in-panel
                    // value — hence `Option`, and hence no update rather than a clamped one.
                    Some(rect) => viewport_to_game(egui::pos2(logical.x, logical.y), rect)
                        .map(|p| Vec2::new(p.x, p.y)),
                    // No viewport this frame — the same `None` the renderer and `ViewportSize`
                    // stand down on. Freeze rather than pass window coordinates through as if
                    // they were game ones.
                    None => None,
                };
            }
        }
        Some(
            self.world
                .resource::<Letterbox>()
                .map(|lb| lb.window_to_design(logical))
                .unwrap_or(logical),
        )
    }

    /// Starts the event loop. Blocks until the window is closed.
    #[allow(unused_mut)]
    pub fn run(mut self) {
        // Automated-verification hooks, read before the window exists so a game needs no code
        // change to be driven and photographed. See `crate::input_script`.
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.apply_input_script_env();
            if let Some(captures) = crate::input_script::capture_plan_from_env() {
                match self.capture_frames_headless(&captures) {
                    Ok(paths) => {
                        for p in paths {
                            println!("ENGINE_CAPTURE wrote {}", p.display());
                        }
                    }
                    Err(err) => log::error!("ENGINE_CAPTURE failed: {err}"),
                }
                // Capture is a complete run on its own — never also open a window.
                return;
            }
        }

        let event_loop = match EventLoop::new() {
            Ok(event_loop) => event_loop,
            Err(err) => {
                log::error!("event loop creation failed: {err}");
                return;
            }
        };
        // Native: `Wait` as the base policy; `about_to_wait` re-arms `ControlFlow::WaitUntil`
        // for the next frame each iteration (frame-paced redraws that still yield the macOS
        // main run loop idle time between frames). A plain `Poll` busy-spins and starves the
        // run loop (laggy window drag + input); plain `Wait` lags on idle drags (only wakes on
        // input). WaitUntil pacing gets both: continuous frames AND a responsive run loop.
        // WASM: `Poll` maps to requestAnimationFrame and is unaffected by the macOS issue.
        #[cfg(not(target_arch = "wasm32"))]
        event_loop.set_control_flow(ControlFlow::Wait);
        #[cfg(target_arch = "wasm32")]
        event_loop.set_control_flow(ControlFlow::Poll);
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(err) = event_loop.run_app(&mut self) {
            log::error!("event loop error: {err}");
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(self);
        }
    }

    /// Initializes the renderer and egui once the GPU context and window are ready.
    /// Native: called directly from resumed(). WASM: called from about_to_wait() after checking PENDING_GPU.
    /// Builds the GPU renderers that do **not** need a window — the sprite renderer (+ pending
    /// textures), the text renderer (+ fonts), pre-GPU render targets, and the `RenderCapabilities`
    /// resource. Shared by the windowed init ([`finish_init`](Self::finish_init)) and the headless
    /// screenshot path ([`save_screenshot_headless`](App::save_screenshot_headless)); egui/IME
    /// setup, which needs a window, stays in `finish_init`.
    pub(in crate::app) fn init_gpu_renderers(&mut self, gpu: &GpuContext) {
        let mut sprite_renderer = SpriteRenderer::new(&gpu.device, &gpu.queue, gpu.config.format);
        // Drain into a local first so the per-path format lookup below borrows a separate field.
        let pending_textures: Vec<String> = self.pending_textures.drain(..).collect();
        for path in pending_textures {
            let format = self
                .pending_texture_formats
                .get(&path)
                .copied()
                .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb);
            sprite_renderer.load_texture_with_format(&gpu.device, &gpu.queue, &path, format);
        }
        self.pending_texture_formats.clear();
        if let Some(assets) = self.world.resource::<AssetServer>() {
            for (path, asset) in assets.image_assets_for_gpu() {
                if !sprite_renderer.has_texture_key(path) {
                    sprite_renderer.load_texture_from_image(&gpu.device, &gpu.queue, path, asset);
                }
            }
        }
        // pending_render_targets: create RTs registered before GPU initialization. A caller-chosen
        // format that this GPU cannot render into falls back to the surface format (with a warning).
        for (name, w, h, fmt, filter) in self.pending_render_targets.drain(..) {
            let format = gpu.resolve_render_target_format(fmt, &name);
            let rt = crate::renderer::render_target::RenderTarget::new_with_filter(
                &gpu.device,
                w,
                h,
                format,
                filter,
            );
            self.render.render_targets.insert(name, rt);
        }
        // Expose GPU render-format support to game systems (e.g. to choose an HDR render target
        // only where renderable). Inserted after the surface format is known.
        self.world
            .insert_resource(crate::renderer::context::RenderCapabilities::new(
                gpu.adapter.clone(),
                gpu.config.format,
            ));
        // `FontData` + the multi-script `ExtraFonts` blobs, with the wasm default-font fallback.
        // Shared with `TextMeasurer`'s construction so a measurement is taken against exactly
        // the font stack this renderer draws with.
        let (font_bytes, extra_fonts) = crate::text_measure::font_blobs(&self.world);
        let text_renderer = Some(TextRenderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &font_bytes,
            &extra_fonts,
        ));
        self.render.sprite_renderer = Some(sprite_renderer);
        self.render.text_renderer = text_renderer;
    }

    fn finish_init(&mut self, gpu: GpuContext, window: Arc<Window>) {
        // WASM: rebind the GPU context as mutable so the surface can be resized to the DPR-scaled
        // buffer below. Native never resizes here, so it keeps the immutable parameter.
        #[cfg(target_arch = "wasm32")]
        let mut gpu = gpu;
        // WASM crisp-Retina sizing: render into a DPR-scaled drawing buffer while the canvas CSS
        // *display* box stays at the logical size. The browser then maps the larger buffer 1:1 onto
        // the logical box (sharp) instead of upscaling a logical-size buffer (soft) — and instead of
        // stretching a buffer that differs from the display box (the old left-clipped-HUD bug). The
        // LOGICAL size is the canvas's authored width/height attributes (stable across scene resets,
        // unlike WindowConfig); store it in WASM_LOGICAL_SIZE for the per-frame viewport math. The
        // scale is uniform (preserves aspect) and capped so neither axis exceeds WebGL2's 2048 max
        // texture size. Set after winit has sized the canvas; a game can still override the CSS.
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            let dpr = web_sys::window()
                .map(|w| w.device_pixel_ratio())
                .unwrap_or(1.0)
                .max(1.0);
            if let Some(canvas) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id(crate::DEFAULT_CANVAS_ID))
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            {
                let logical_w = canvas.width().max(1);
                let logical_h = canvas.height().max(1);
                super::WASM_LOGICAL_SIZE.with(|c| c.set((logical_w, logical_h)));
                let scale = dpr
                    .min(2048.0 / logical_w as f64)
                    .min(2048.0 / logical_h as f64);
                let buf_w = ((logical_w as f64 * scale).round() as u32).clamp(1, 2048);
                let buf_h = ((logical_h as f64 * scale).round() as u32).clamp(1, 2048);
                canvas.set_width(buf_w);
                canvas.set_height(buf_h);
                let style = canvas.style();
                let _ = style.set_property("width", &format!("{logical_w}px"));
                let _ = style.set_property("height", &format!("{logical_h}px"));
                gpu.resize(winit::dpi::PhysicalSize::new(buf_w, buf_h));
            }
        }
        // Build the window-independent GPU renderers (sprites, text, render targets,
        // RenderCapabilities). Shared with the headless screenshot path.
        self.init_gpu_renderers(&gpu);
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &*window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        // Match the pre-0.34 positional args (depth None, msaa 1, dithering FALSE) —
        // `RendererOptions::default()` flips dithering on, which would silently change
        // gradient rendering in the debug UI.
        let egui_renderer = egui_wgpu::Renderer::new(
            &gpu.device,
            gpu.config.format,
            egui_wgpu::RendererOptions {
                msaa_samples: 1,
                dithering: false,
                ..Default::default()
            },
        );
        self.world.insert_resource(DebugUi::new_with_ctx(egui_ctx));
        self.render.egui_renderer = Some(egui_renderer);
        self.render.egui_state = Some(egui_state);
        self.gpu = Some(gpu);
        // IME support is controlled by the `ImeConfig` resource (default: off — see `src/resources.rs`).
        // When enabled, CJK input on macOS etc. arrives via `Ime::Preedit/Commit`, but active CJK
        // input methods can swallow keyUp events and cause keys to stick. Only enable it via
        // `ImeConfig { allowed: true }` for apps that need text input.
        let ime_allowed = self
            .world
            .resource::<crate::resources::ImeConfig>()
            .map(|c| c.allowed)
            .unwrap_or(false);
        window.set_ime_allowed(ime_allowed);
        self.window = Some(window);
        self.last_frame = Some(Instant::now());
        // Pace redraw requests to the monitor refresh rate (fallback 60 Hz). `AutoVsync`
        // still does the final pacing; this only bounds the WaitUntil cadence so the loop
        // doesn't spin and starve the macOS main run loop. Clamp to [60, 240] Hz so a bogus
        // or low adaptive reading (e.g. ProMotion reporting a reduced rate) never paces the
        // redraw cadence below 60 fps; pacing slightly faster than the panel is harmless
        // because AutoVsync caps actual presentation.
        #[cfg(not(target_arch = "wasm32"))]
        {
            let refresh_hz = self
                .window
                .as_ref()
                .and_then(|w| w.current_monitor())
                .and_then(|m| m.refresh_rate_millihertz())
                .map(|mhz| mhz as f64 / 1000.0)
                .filter(|hz| *hz >= frame_pacing::MIN_VALID_REFRESH_HZ)
                .unwrap_or(frame_pacing::FALLBACK_REFRESH_HZ)
                .clamp(frame_pacing::MIN_REFRESH_HZ, frame_pacing::MAX_REFRESH_HZ);
            self.frame_interval = Duration::from_secs_f64(1.0 / refresh_hz);
            self.next_frame = Some(Instant::now());
        }
        log::info!("engine initialized");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_gilrs(&mut self) {
        let mut events = Vec::new();
        let mut gilrs_panicked = false;
        if let Some(gilrs) = &mut self.gilrs {
            // gilrs can panic internally for some controllers (e.g. it processes an axis
            // event for a gamepad id its backend never registered and unwraps a `None`).
            // Isolate that panic — mirroring the per-system catch_unwind in `schedule.rs` —
            // so a flaky controller degrades gamepad input instead of crashing the whole app.
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let mut evs = Vec::new();
                while let Some(event) = gilrs.next_event() {
                    evs.push(event);
                }
                evs
            }));
            match result {
                Ok(evs) => events = evs,
                Err(_) => gilrs_panicked = true,
            }
        }
        if gilrs_panicked {
            log::error!(
                "gamepad backend (gilrs) panicked while polling events — disabling gamepad input for this session"
            );
            self.gilrs = None;
            return;
        }
        if events.is_empty() {
            return;
        }
        if let Some(state) = self.world.resource_mut::<GamepadState>() {
            for event in events {
                state.process_event(event);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::Letterbox;

    /// A finger and a mouse must land in the same coordinate space.
    ///
    /// Regression: the `Touch` arm stopped at the physical→logical scale division while
    /// `CursorMoved` went on to apply `Letterbox::window_to_design`. Under a `DesignResolution`
    /// that made `InputState::cursor()` — the value `Camera::screen_to_world` documents as its
    /// input — mean window space from a touch and design space from a mouse.
    #[test]
    fn game_cursor_maps_window_logical_into_design_space() {
        let mut app = App::new();
        // A 1280x720 design canvas letterboxed into a square 1000x1000 window.
        app.world
            .insert_resource(Letterbox::compute(1280.0, 720.0, 1000.0, 1000.0));

        let mapped = app
            .game_cursor(Vec2::new(500.0, 500.0))
            .expect("the cursor only freezes in the docked editor");

        // The window centre is the design centre, and emphatically not (500, 500) — which is
        // what the touch arm returned. The touch arm predates `DesignResolution` (v0.62.0,
        // #217), which taught `CursorMoved` the mapping and never came back for it.
        assert!((mapped.x - 640.0).abs() < 1e-3, "x={}", mapped.x);
        assert!((mapped.y - 360.0).abs() < 1e-3, "y={}", mapped.y);
    }

    /// In Docked mode the cursor is never passed through as if window space were game space.
    ///
    /// Until v0.156.3 a `central_rect` of `None` returned `Some(logical)` — the raw window
    /// position — while the renderer and `ViewportSize` were both using the margin fallback for
    /// exactly those frames. Three consumers, three answers.
    ///
    /// ⚠️ **What this can and cannot reach.** Headlessly there is no surface, so the fallback
    /// cannot be derived and `docked_central_rect` declines for that reason rather than for a
    /// too-small window. Both are the same verdict — no viewport, no cursor — and the decision
    /// itself is unit-tested in `editor::docked_rt`. What this pins is that the pass-through is
    /// gone from this call site.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn docked_cursor_never_passes_window_coordinates_through() {
        use crate::app::editor::EditorMode;
        let mut app = App::new();
        app.world.insert_resource(Letterbox::IDENTITY);
        let p = Vec2::new(123.0, 456.0);
        // Control: outside the editor this exact position maps straight through, so a `None`
        // below is the docked branch declining and not the mapping failing generally.
        assert_eq!(app.game_cursor(p), Some(p));

        app.editor.mode = EditorMode::Docked;
        assert_eq!(
            app.game_cursor(p),
            None,
            "the docked branch handed back the raw window position"
        );
    }

    /// The published central rect still maps, unchanged: this change is about which rect the
    /// branch reads, not about the arithmetic.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn docked_cursor_maps_through_a_published_central_rect() {
        use crate::app::editor::EditorMode;
        let mut app = App::new();
        app.world.insert_resource(Letterbox::IDENTITY);
        app.editor.mode = EditorMode::Docked;
        app.editor.central_rect = Some(egui::Rect::from_min_size(
            egui::pos2(260.0, 36.0),
            egui::vec2(720.0, 534.0),
        ));
        assert_eq!(
            app.game_cursor(Vec2::new(300.0, 100.0)),
            Some(Vec2::new(40.0, 64.0)),
            "a published rect must still translate window space into panel-local space"
        );
        // Control: a position outside the panel freezes the game cursor rather than clamping.
        assert_eq!(app.game_cursor(Vec2::new(10.0, 10.0)), None);
    }

    /// No design resolution in play: the mapping must be an exact no-op, not an approximate
    /// one. Every game without a `DesignResolution` rides this path.
    #[test]
    fn game_cursor_is_identity_without_a_design_resolution() {
        let mut app = App::new();
        app.world.insert_resource(Letterbox::IDENTITY);

        let p = Vec2::new(123.0, 456.0);
        assert_eq!(app.game_cursor(p), Some(p));
    }
}
