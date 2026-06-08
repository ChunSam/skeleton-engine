use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ScheduleErrorPolicy {
    /// 에러를 로그에 남기고 시스템 삽입 순서로 실행한다.
    #[default]
    LogAndFallback,
    /// 에러를 로그에 남기고 해당 프레임의 사용자 시스템 실행을 건너뛴다.
    DisableRunOnCycle,
    /// 순환 의존성을 panic으로 처리한다. 테스트/개발 환경에서 빠른 실패가 필요할 때 쓴다.
    PanicOnCycle,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SystemPanicPolicy {
    /// panic을 기록하고 해당 시스템을 이후 프레임에서 비활성화한 뒤 계속 실행한다.
    #[default]
    DisableSystemAndContinue,
    /// panic을 기록한 뒤 다시 panic시켜 호출자가 실패를 즉시 볼 수 있게 한다.
    AbortAfterLog,
}

pub(super) fn format_panic_payload(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "알 수 없는 패닉".to_string()
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
        let content = format!("[{timestamp}] 시스템 패닉: {system_name}\n오류: {message}\n\n");
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
        self.systems.push(Box::new(system));
        self.system_meta
            .push(crate::ecs::schedule::SystemMeta::default());
        self.schedule_dirty = true;
    }

    pub fn add_system_labeled<S: System + 'static>(
        &mut self,
        system: S,
        config: crate::ecs::schedule::SystemConfig,
    ) {
        self.systems.push(Box::new(system));
        self.system_meta.push(config.into());
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
        // The world coordinate system is logical pixels; the GPU surface is physical.
        // Native: the surface is CSS-size × DPR, so divide by the DPR to recover the logical
        // viewport (otherwise sprites/UI look half-size on Retina/HiDPI).
        // Wasm: the surface is sized to the canvas DOM (CSS-logical) size — the Resized handler
        // caps it there to stay under the WebGL2 texture limit — so it is ALREADY logical.
        // Dividing again would halve the viewport and push fixed-coordinate scenes off-screen on
        // a Retina display (the coin_race wasm example surfaced this: it rendered only at DPR=1).
        if let Some(gpu) = &self.gpu {
            #[cfg(not(target_arch = "wasm32"))]
            let scale_factor = self
                .window
                .as_ref()
                .map(|w| w.scale_factor() as f32)
                .unwrap_or(1.0)
                .max(1.0);
            #[cfg(target_arch = "wasm32")]
            let scale_factor = 1.0_f32;
            self.world.insert_resource(ViewportSize {
                width: gpu.config.width as f32 / scale_factor,
                height: gpu.config.height as f32 / scale_factor,
            });
            self.world.insert_resource(DisplayScaleFactor(scale_factor));
        }

        // egui 프레임 시작
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

        // 스케줄 재계산 (라벨/순서 변경 시)
        if self.schedule_dirty {
            // 안전: 씬 직접 push 흡수
            if self.system_meta.len() != self.systems.len() {
                self.system_meta.resize(
                    self.systems.len(),
                    crate::ecs::schedule::SystemMeta::default(),
                );
            }
            match crate::ecs::schedule::compute_order(&self.system_meta) {
                Ok(order) => self.exec_order = order,
                Err(crate::ecs::schedule::ScheduleError::Cycle(remaining)) => {
                    match self.schedule_error_policy {
                        ScheduleErrorPolicy::LogAndFallback => {
                            log::error!(
                                "시스템 순서 순환 의존성 감지 — 삽입 순서로 폴백 (영향 인덱스: {remaining:?})"
                            );
                            self.exec_order = (0..self.systems.len()).collect();
                        }
                        ScheduleErrorPolicy::DisableRunOnCycle => {
                            log::error!(
                                "시스템 순서 순환 의존성 감지 — 사용자 시스템 실행 건너뜀 (영향 인덱스: {remaining:?})"
                            );
                            self.exec_order.clear();
                        }
                        ScheduleErrorPolicy::PanicOnCycle => {
                            panic!("시스템 순서 순환 의존성 감지 (영향 인덱스: {remaining:?})");
                        }
                    }
                }
            }
            self.schedule_dirty = false;
        }

        // 시스템 실행 + 프로파일러 계측 (exec_order 순회, 비활성 set 스킵)
        {
            let system_count = self.systems.len();
            let mut timings: Vec<(usize, &'static str, u64)> = Vec::with_capacity(system_count);
            let order = self.exec_order.clone();
            for i in order {
                if i >= self.systems.len() {
                    continue;
                }
                // 패닉으로 비활성화된 시스템 건너뜀
                if self.panicked_systems.contains(&i) {
                    continue;
                }
                if let Some(set) = self.system_meta.get(i).and_then(|m| m.set) {
                    if self.disabled_sets.contains(set) {
                        continue;
                    }
                }
                let name = self.systems[i].name();
                let t0 = Instant::now();
                // 패닉 격리: 시스템 패닉 시 엔진이 계속 실행되도록 catch_unwind로 감싼다.
                // AssertUnwindSafe: World는 패닉 후 일관성을 보장하지 않을 수 있으나,
                // 해당 시스템을 비활성화하면 추가 피해를 막는다.
                // 주의: panic = "abort" 빌드 또는 FFI 패닉에서는 동작하지 않음.
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    self.systems[i].run(&mut self.world, dt);
                }));
                match result {
                    Ok(()) => {
                        timings.push((i, name, t0.elapsed().as_micros() as u64));
                    }
                    Err(panic) => {
                        let msg = format_panic_payload(&panic);
                        log::error!("시스템 패닉 [{name}]: {msg}");
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
        // 계층 변환 전파 — 유저 시스템(물리 포함) 이후, 렌더 직전에 실행
        HierarchySystem.run(&mut self.world, dt);

        // 카메라 이펙트 업데이트 (shake decay, zoom tween, smooth follow)
        {
            let follow_pos = self
                .world
                .resource::<Camera>()
                .and_then(|cam| cam.follow_entity)
                .and_then(|e| self.world.get::<crate::components::Transform>(e))
                .map(|t| t.position);
            if let Some(cam) = self.world.resource_mut::<Camera>() {
                cam.update(dt, follow_pos);
            }
        }

        self.update_editor_ui(&egui_ctx, dt);

        // egui 프레임 종료 + tessellate → render() 로 전달
        if let Some(ctx) = egui_ctx {
            let ppp = self
                .window
                .as_ref()
                .map(|w| w.scale_factor() as f32)
                .unwrap_or(1.0);
            let full_output = ctx.end_pass();
            let paint_jobs = ctx.tessellate(full_output.shapes, ppp);
            self.egui_output = Some((paint_jobs, full_output.textures_delta, ppp));
        }
        // 모든 시스템 실행 후 이벤트 큐를 비운다.
        // std::mem::take 으로 꺼내야 &mut self.world 와 충돌하지 않는다.
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
        // 씬 전환 명령 처리 (이벤트/입력 flush 이후)
        let cmd = self
            .world
            .resource_mut::<SceneChange>()
            .and_then(|sc| sc.0.take());
        if let Some(cmd) = cmd {
            self.apply_scene_cmd(cmd);
        }

        // FadeTransition 알파 진행
        if let Some(fade) = self
            .world
            .resource_mut::<crate::resources::FadeTransition>()
        {
            fade.update(dt);
        }

        // 핫 리로딩: 변경된 파일 목록을 받아 GPU 텍스처를 재업로드한다.
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

        // 비동기 로드 완료 처리: 완료된 에셋을 GPU에 업로드하고 LoadProgress를 갱신한다.
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
            // LoadProgress.loaded 갱신
            if let Some(prog) = self.world.resource_mut::<LoadProgress>() {
                prog.loaded += async_completed.len();
            }
        }
        self.upload_asset_server_images_to_gpu();
    }
}
