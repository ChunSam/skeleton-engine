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
        if let (Some(state), Some(window)) = (&mut self.egui_state, &self.window) {
            let _ = state.on_window_event(window, &event);
        }

        match event {
            // ── Close window ──────────────────────────────────────────────────────
            WindowEvent::CloseRequested => event_loop.exit(),

            // ── Window resize ─────────────────────────────────────────────────
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    // WASM: due to Retina DPR, winit reports CSS pixels × DPR (e.g. 2560×1440),
                    // which exceeds WebGL2's max texture size (2048), so read the canvas size
                    // directly from the DOM instead.
                    #[cfg(target_arch = "wasm32")]
                    let size = {
                        use wasm_bindgen::JsCast;
                        web_sys::window()
                            .and_then(|w| w.document())
                            .and_then(|d| d.get_element_by_id("game-canvas"))
                            .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
                            .map(|c| {
                                winit::dpi::PhysicalSize::new(c.width().max(1), c.height().max(1))
                            })
                            .unwrap_or(size)
                    };
                    gpu.resize(size);
                }
                // Render one frame during a live resize drag to reduce visual stutter.
                // (On macOS, Resized fires during the modal resize loop even when RedrawRequested stalls.)
                #[cfg(not(target_arch = "wasm32"))]
                self.step_frame(event_loop);
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
                // F1 → toggle DebugUi
                if key == winit::keyboard::KeyCode::F1 && state == ElementState::Pressed {
                    if let Some(debug_ui) = self.world.resource_mut::<DebugUi>() {
                        debug_ui.toggle();
                    }
                }
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
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    input.set_cursor(Vec2::new(
                        position.x as f32 / scale,
                        position.y as f32 / scale,
                    ));
                }
            }

            // ── Mouse button ──────────────────────────────────────────────────
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match state {
                        ElementState::Pressed => input.press_mouse(button),
                        ElementState::Released => input.release_mouse(button),
                    }
                }
            }

            // ── Mouse wheel ────────────────────────────────────────────────────
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match delta {
                        MouseScrollDelta::LineDelta(_, y) => input.add_scroll(y),
                        // Convert pixel-delta scroll (trackpad etc.) to lines: 20px ≈ 1 line (empirical)
                        MouseScrollDelta::PixelDelta(p) => input.add_scroll(p.y as f32 / 20.0),
                    }
                }
            }

            // ── Touch input ────────────────────────────────────────────────────
            WindowEvent::Touch(winit::event::Touch {
                phase,
                location,
                id,
                ..
            }) => {
                let pos = Vec2::new(location.x as f32, location.y as f32);
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
                // The `InputState` cursor used by UI hit-testing must be in logical pixels, same
                // as the mouse, so divide by the scale factor (`TouchState` keeps physical coords).
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0);
                let logical = Vec2::new(pos.x / scale, pos.y / scale);
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
                self.step_frame(event_loop);
            }

            _ => {}
        }
    }

    /// Called when the event queue is empty → poll gamepads then request a redraw every frame.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_gilrs();

        // WASM: detect completion of the async GPU initialization started by spawn_local.
        #[cfg(target_arch = "wasm32")]
        if self.gpu.is_none() {
            if let Some((gpu, window)) = PENDING_GPU.with(|p| p.borrow_mut().take()) {
                self.finish_init(gpu, window);
            }
        }

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
        // This is a game/interactive app that needs continuous per-frame updates.
        // The default `Wait` policy only wakes on input, causing noticeable drag/hover lag.
        // `Poll` combined with request_redraw in about_to_wait runs a tight loop up to the vsync limit.
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
        // WASM: winit sizes the canvas's CSS *display* box to the window's logical size, which
        // can differ from the drawing buffer and stretch the canvas — shifting fixed-position
        // HUD text off-screen (sprites stay centred, but left-edge text falls off). Lock the CSS
        // display size to the buffer so the canvas shows 1:1 with what the engine renders. Set
        // after winit has sized the canvas; a game can still override with `!important` CSS.
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            if let Some(canvas) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("game-canvas"))
                .and_then(|el| el.dyn_into::<web_sys::HtmlCanvasElement>().ok())
            {
                let style = canvas.style();
                let _ = style.set_property("width", &format!("{}px", canvas.width()));
                let _ = style.set_property("height", &format!("{}px", canvas.height()));
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
                sprite_renderer.texture_layout(),
            );
            self.render_targets.insert(name, rt);
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
        let text_renderer = Some(TextRenderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &font_bytes,
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
        let egui_renderer =
            egui_wgpu::Renderer::new(&gpu.device, gpu.config.format, None, 1, false);
        self.world.insert_resource(DebugUi::new_with_ctx(egui_ctx));
        self.egui_renderer = Some(egui_renderer);
        self.egui_state = Some(egui_state);
        self.sprite_renderer = Some(sprite_renderer);
        self.text_renderer = text_renderer;
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
        log::info!("engine initialized");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn poll_gilrs(&mut self) {
        let mut events = Vec::new();
        if let Some(gilrs) = &mut self.gilrs {
            while let Some(event) = gilrs.next_event() {
                events.push(event);
            }
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
