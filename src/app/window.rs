use super::*;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

impl ApplicationHandler for App {
    /// Called when the app becomes active (macOS: Resumed; other platforms: once at startup).
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (init_w, init_h, title) = self
            .world
            .resource::<WindowConfig>()
            .map(|c| (c.width, c.height, c.title.clone()))
            .unwrap_or((1280, 720, "Game".to_string()));
        let attrs = Window::default_attributes()
            .with_title(&title)
            .with_inner_size(winit::dpi::LogicalSize::new(init_w, init_h));

        // WASM: attach the <canvas id="game-canvas"> element in the HTML page to the winit window.
        #[cfg(target_arch = "wasm32")]
        let attrs = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            if let Some(canvas) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("game-canvas"))
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
                                .and_then(|d| d.get_element_by_id("game-canvas"))
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

                #[cfg(not(target_arch = "wasm32"))]
                {
                    use crate::app::editor::{docked_rt::viewport_to_game, EditorMode};
                    // Always track the untranslated window-space cursor; the docked
                    // pointer gate needs the physical position even while the game
                    // cursor is frozen outside the panel.
                    self.editor.window_cursor = Some(egui::pos2(logical.x, logical.y));
                    if self.editor.mode == EditorMode::Docked {
                        if let Some(central_rect) = self.editor.central_rect {
                            let win_pos = egui::pos2(logical.x, logical.y);
                            if let Some(game_pos) = viewport_to_game(win_pos, central_rect) {
                                if let Some(input) = self.world.resource_mut::<InputState>() {
                                    input.set_cursor(Vec2::new(game_pos.x, game_pos.y));
                                }
                            }
                            // Outside the central rect: game cursor is frozen (no update).
                        } else {
                            // central_rect not yet computed (first frames) — pass through.
                            if let Some(input) = self.world.resource_mut::<InputState>() {
                                input.set_cursor(logical);
                            }
                        }
                    } else {
                        if let Some(input) = self.world.resource_mut::<InputState>() {
                            input.set_cursor(logical);
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    input.set_cursor(logical);
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
            // winit delivers touch locations in physical pixels. Divide by the window
            // scale factor to convert to logical (scale-adjusted) pixels, consistent
            // with the mouse cursor path (CursorMoved) and UI hit-testing.
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
                // Convert physical → logical pixels so TouchState positions match
                // InputState::cursor() and Camera::screen_to_world conventions.
                let pos = Vec2::new(location.x as f32 / scale, location.y as f32 / scale);
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
                // `pos` is already logical (divided by scale above).
                let logical = pos;
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match phase {
                        winit::event::TouchPhase::Started => {
                            input.set_cursor(logical);
                            input.press_mouse(winit::event::MouseButton::Left);
                        }
                        winit::event::TouchPhase::Moved => input.set_cursor(logical),
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
    /// Starts the event loop. Blocks until the window is closed.
    #[allow(unused_mut)]
    pub fn run(mut self) {
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
                .and_then(|d| d.get_element_by_id("game-canvas"))
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
        let mut sprite_renderer = SpriteRenderer::new(&gpu.device, &gpu.queue, gpu.config.format);
        for path in self.pending_textures.drain(..) {
            sprite_renderer.load_texture(&gpu.device, &gpu.queue, &path);
        }
        if let Some(assets) = self.world.resource::<AssetServer>() {
            for (path, asset) in assets.image_assets_for_gpu() {
                if !sprite_renderer.has_texture_key(&path) {
                    sprite_renderer.load_texture_from_image(&gpu.device, &gpu.queue, &path, &asset);
                }
            }
        }
        // pending_render_targets: create RTs registered before GPU initialization
        for (name, w, h) in self.pending_render_targets.drain(..) {
            let rt = crate::renderer::render_target::RenderTarget::new(
                &gpu.device,
                w,
                h,
                gpu.config.format,
            );
            self.render.render_targets.insert(name, rt);
        }
        let font_bytes = self
            .world
            .resource::<FontData>()
            .map(|f| f.0.clone())
            .unwrap_or_default();
        // WASM: the browser sandbox has no system fonts, so cosmic-text would panic when it
        // shapes text against an empty font db. Fall back to the engine's embedded default font
        // when the game supplies no `FontData`, so `DrawText`/HUD text renders out of the box.
        // Native loads system fonts inside `FontSystem::new()` and does not embed the default.
        #[cfg(target_arch = "wasm32")]
        let font_bytes = if font_bytes.is_empty() {
            crate::renderer::DEFAULT_FONT.to_vec()
        } else {
            font_bytes
        };
        // Additional fonts (multi-script coverage, e.g. an RTL-script font) loaded alongside FontData.
        let extra_fonts = self
            .world
            .resource::<crate::resources::ExtraFonts>()
            .map(|f| f.0.clone())
            .unwrap_or_default();
        let text_renderer = Some(TextRenderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &font_bytes,
            &extra_fonts,
        ));
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
        self.render.sprite_renderer = Some(sprite_renderer);
        self.render.text_renderer = text_renderer;
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
                .filter(|hz| *hz >= 1.0)
                .unwrap_or(60.0)
                .clamp(60.0, 240.0);
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
