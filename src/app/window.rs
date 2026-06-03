use super::*;

use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

impl ApplicationHandler for App {
    /// 앱이 활성화될 때 호출 (macOS: Resumed, 기타: 시작 시 1회)
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let (init_w, init_h, title) = self
            .world
            .resource::<WindowConfig>()
            .map(|c| (c.width, c.height, c.title.clone()))
            .unwrap_or((1280, 720, "Game".to_string()));
        let attrs = Window::default_attributes()
            .with_title(&title)
            .with_inner_size(winit::dpi::LogicalSize::new(init_w, init_h));

        // WASM: HTML 내 <canvas id="game-canvas"> 를 winit 창에 연결한다.
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
                log::error!("창 생성 실패: {err}");
                event_loop.exit();
                return;
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            match pollster::block_on(GpuContext::new(window.clone())) {
                Ok(gpu) => self.finish_init(gpu, window),
                Err(err) => {
                    log::error!("GPU 초기화 실패: {err}");
                    event_loop.exit();
                }
            }
        }

        // WASM: WebGPU/WebGL2 adapter 요청이 Promise 기반이므로 spawn_local로 비동기 처리한다.
        // GPU 준비 완료 시 PENDING_GPU thread_local에 저장 → about_to_wait()에서 finish_init 호출.
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
                    Err(err) => log::error!("GPU 초기화 실패: {err}"),
                }
            });
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // egui에 이벤트 선전달
        if let (Some(state), Some(window)) = (&mut self.egui_state, &self.window) {
            let _ = state.on_window_event(window, &event);
        }

        match event {
            // ── 창 닫기 ──────────────────────────────────────────────────────
            WindowEvent::CloseRequested => event_loop.exit(),

            // ── 창 크기 변경 ─────────────────────────────────────────────────
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    // WASM: Retina DPR 때문에 winit이 CSS 픽셀 × DPR(= 2560×1440)을 보고한다.
                    // WebGL2의 최대 텍스처 크기(2048)를 초과하므로, DOM에서 canvas 크기를 직접 읽는다.
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
                // 라이브 리사이즈 드래그 중에도 한 프레임 그려 멈춤을 완화한다.
                // (macOS 모달 루프 동안 RedrawRequested 가 멈춰도 Resized 는 들어온다.)
                #[cfg(not(target_arch = "wasm32"))]
                self.step_frame(event_loop);
            }

            // ── 키보드 입력 ──────────────────────────────────────────────────
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
                // F1 → DebugUi 토글
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

            // ── 마우스 커서 이동 ─────────────────────────────────────────────
            // winit이 주는 좌표는 물리 픽셀이지만, UI 히트테스트·`ViewportSize`·
            // `Camera::screen_to_world`는 모두 논리 픽셀 기준이다. HiDPI(예: Retina 2×)
            // 에서 어긋나지 않도록 scale factor로 나눠 논리 좌표로 저장한다.
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

            // ── 마우스 버튼 ──────────────────────────────────────────────────
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match state {
                        ElementState::Pressed => input.press_mouse(button),
                        ElementState::Released => input.release_mouse(button),
                    }
                }
            }

            // ── 마우스 휠 ────────────────────────────────────────────────────
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(input) = self.world.resource_mut::<InputState>() {
                    match delta {
                        MouseScrollDelta::LineDelta(_, y) => input.add_scroll(y),
                        // 픽셀 단위 휠(트랙패드 등)을 line 단위로 환산: 20px ≈ 1 line (경험적 근사값)
                        MouseScrollDelta::PixelDelta(p) => input.add_scroll(p.y as f32 / 20.0),
                    }
                }
            }

            // ── 터치 입력 ────────────────────────────────────────────────────
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
                // 터치를 마우스 왼쪽 버튼으로 에뮬레이션 (기존 UI 시스템 호환).
                // UI 히트테스트가 쓰는 `InputState` 커서는 마우스와 동일하게 논리
                // 좌표여야 하므로 scale factor로 나눈다 (`TouchState`는 물리 좌표 유지).
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

            // ── 프레임 렌더 ──────────────────────────────────────────────────
            WindowEvent::RedrawRequested => {
                // WASM: about_to_wait 타이밍에 GPU가 준비되지 않은 경우를 대비해 여기서도 체크
                #[cfg(target_arch = "wasm32")]
                if self.gpu.is_none() {
                    if let Some((gpu, window)) = PENDING_GPU.with(|p| p.borrow_mut().take()) {
                        self.finish_init(gpu, window);
                    }
                }

                // update + render 를 한 RedrawRequested 안에서 연속 실행한다.
                // (한때 update 를 about_to_wait 로 분리했으나 update→render 사이에
                // 한 프레임 지연이 생겨 입력 반응이 늦었다. 클릭 정확성은 press/release
                // 시점 커서 기록으로 보장된다.) 동일 시퀀스를 Resized 에서도 재사용한다.
                self.step_frame(event_loop);
            }

            _ => {}
        }
    }

    /// 이벤트 큐가 비었을 때 → 게임패드 폴링 후 매 프레임 redraw 요청.
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        self.poll_gilrs();

        // WASM: spawn_local로 시작한 GPU 비동기 초기화 완료를 여기서 감지한다.
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
    /// 이벤트 루프를 시작한다. 창이 닫힐 때까지 블로킹된다.
    #[allow(unused_mut)]
    pub fn run(mut self) {
        let event_loop = match EventLoop::new() {
            Ok(event_loop) => event_loop,
            Err(err) => {
                log::error!("이벤트 루프 생성 실패: {err}");
                return;
            }
        };
        // 게임/인터랙티브 앱이므로 매 프레임 연속 갱신한다. 기본값 `Wait` 는 입력이
        // 있을 때만 깨어나 드래그·호버 반응이 한 박자 늦게 느껴진다. `Poll` 은
        // about_to_wait 의 request_redraw 와 함께 vsync 한계까지 연속 루프를 돈다.
        event_loop.set_control_flow(ControlFlow::Poll);
        #[cfg(not(target_arch = "wasm32"))]
        if let Err(err) = event_loop.run_app(&mut self) {
            log::error!("이벤트 루프 오류: {err}");
        }
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::EventLoopExtWebSys;
            event_loop.spawn_app(self);
        }
    }

    /// GPU 컨텍스트와 창이 준비된 후 렌더러·egui를 초기화한다.
    /// 네이티브: resumed()에서 직접 호출. WASM: about_to_wait()에서 PENDING_GPU 확인 후 호출.
    fn finish_init(&mut self, gpu: GpuContext, window: Arc<Window>) {
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
        // pending_render_targets: GPU 초기화 전에 등록된 RT를 여기서 실제 생성
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
        // WASM: 시스템 폰트가 없으므로 font_bytes가 비어있으면 텍스트 렌더러를 생성하지 않는다.
        // cosmic-text는 폰트 없이 shape를 시도할 때 패닉한다.
        #[cfg(not(target_arch = "wasm32"))]
        let text_renderer = Some(TextRenderer::new(
            &gpu.device,
            &gpu.queue,
            gpu.config.format,
            &font_bytes,
        ));
        #[cfg(target_arch = "wasm32")]
        let text_renderer = if !font_bytes.is_empty() {
            Some(TextRenderer::new(
                &gpu.device,
                &gpu.queue,
                gpu.config.format,
                &font_bytes,
            ))
        } else {
            None
        };
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
        // IME 활성화: 켜지 않으면 macOS 등에서 한글/일어/중국어가 조합되지 않고
        // 자모/캐나 단위 `Character` 이벤트로 들어온다. `Ime::Preedit/Commit` 핸들러는
        // 이미 있으므로, 허용만 하면 조합된 글자가 `Commit` 으로 전달된다.
        window.set_ime_allowed(true);
        self.window = Some(window);
        self.last_frame = Some(Instant::now());
        log::info!("엔진 초기화 완료");
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
