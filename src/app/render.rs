use super::egui_pass::{egui_render_pass, paint_jobs_contain_callbacks};
use super::*;
use winit::event_loop::ActiveEventLoop;

impl App {
    fn debug_shape_to_draw_rects(shape: crate::resources::DebugShape, q: &mut UiQueue) {
        use crate::resources::DebugShape;
        const Z: f32 = 999.0;

        // 선분 근사 헬퍼: 두 점 사이를 thickness×thickness 점들로 채운다.
        let mut push_line =
            |start: glam::Vec2, end: glam::Vec2, color: [f32; 4], thickness: f32| {
                let delta = end - start;
                let len = delta.length();
                if len < 0.001 {
                    return;
                }
                let steps = (len / thickness.max(0.5)).ceil() as usize;
                let half = thickness / 2.0;
                for i in 0..=steps {
                    let t = i as f32 / steps.max(1) as f32;
                    let pos = start + delta * t;
                    q.push(
                        DrawRect::new(pos.x - half, pos.y - half, thickness, thickness, color)
                            .with_z(Z),
                    );
                }
            };

        match shape {
            DebugShape::Rect { min, max, color } => {
                let t = 1.5_f32;
                let w = max.x - min.x;
                let h = max.y - min.y;
                // 위
                q.push(DrawRect::new(min.x, min.y, w, t, color).with_z(Z));
                // 아래
                q.push(DrawRect::new(min.x, max.y - t, w, t, color).with_z(Z));
                // 왼쪽
                q.push(DrawRect::new(min.x, min.y, t, h, color).with_z(Z));
                // 오른쪽
                q.push(DrawRect::new(max.x - t, min.y, t, h, color).with_z(Z));
            }
            DebugShape::Line {
                start,
                end,
                color,
                thickness,
            } => {
                push_line(start, end, color, thickness);
            }
            DebugShape::Circle {
                center,
                radius,
                color,
            } => {
                let n = 24u32;
                for i in 0..n {
                    let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
                    let p0 = center + glam::Vec2::new(a0.cos(), a0.sin()) * radius;
                    let p1 = center + glam::Vec2::new(a1.cos(), a1.sin()) * radius;
                    push_line(p0, p1, color, 1.5);
                }
            }
            DebugShape::Cross { pos, size, color } => {
                let half = size / 2.0;
                push_line(
                    pos - glam::Vec2::X * half,
                    pos + glam::Vec2::X * half,
                    color,
                    1.5,
                );
                push_line(
                    pos - glam::Vec2::Y * half,
                    pos + glam::Vec2::Y * half,
                    color,
                    1.5,
                );
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn ensure_intermediate_texture(
        slot: &mut Option<(
            wgpu::Texture,
            wgpu::TextureView,
            u32,
            u32,
            wgpu::TextureFormat,
        )>,
        device: &wgpu::Device,
        label: &'static str,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> bool {
        let needs_new = match slot {
            Some((_, _, w, h, fmt)) => *w != width || *h != height || *fmt != format,
            None => true,
        };
        if needs_new {
            let tex = device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
            *slot = Some((tex, view, width, height, format));
        }
        needs_new
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let gpu = match self.gpu.as_mut() {
            Some(g) => g,
            None => return Ok(()),
        };

        // PostProcessConfig 리소스 확인 (enabled=true일 때만 중간 텍스처 사용)
        let pp_config: Option<PostProcessConfig> =
            self.world.resource::<PostProcessConfig>().copied();
        let use_post = pp_config.map(|c| c.enabled).unwrap_or(false);

        // 포스트프로세스 렌더러 초기화 / 리사이즈
        if use_post {
            let (w, h, fmt) = (gpu.config.width, gpu.config.height, gpu.config.format);
            match &mut self.post_renderer {
                None => {
                    self.post_renderer = Some(PostProcessRenderer::new(&gpu.device, w, h, fmt));
                }
                Some(pr) if pr.format() != fmt => {
                    pr.reconfigure(&gpu.device, w, h, fmt);
                }
                Some(pr) if pr.width != w || pr.height != h => {
                    pr.resize(&gpu.device, w, h);
                }
                _ => {}
            }
        }

        // 라이팅 렌더러 초기화 / 리사이즈 / 비활성화
        #[cfg(not(target_arch = "wasm32"))]
        let use_lighting = {
            let has_lighting = self
                .world
                .resource::<crate::resources::AmbientLight>()
                .is_some();
            let (w, h, fmt) = (gpu.config.width, gpu.config.height, gpu.config.format);
            if has_lighting {
                match &mut self.lighting_renderer {
                    None => {
                        self.lighting_renderer =
                            Some(crate::renderer::lighting::LightingRenderer::new(
                                &gpu.device,
                                w,
                                h,
                                fmt,
                            ));
                    }
                    Some(lr) if lr.format() != fmt => {
                        lr.reconfigure(&gpu.device, w, h, fmt);
                    }
                    Some(lr) if lr.width != w || lr.height != h => {
                        lr.resize(&gpu.device, w, h);
                    }
                    _ => {}
                }
                // 씬 중간 텍스처 생성 / 리사이즈 (post_renderer가 없을 때만 필요)
                if !use_post {
                    Self::ensure_intermediate_texture(
                        &mut self.scene_texture_for_lighting,
                        &gpu.device,
                        "scene_for_lighting",
                        w,
                        h,
                        fmt,
                    );
                    self.post_texture_for_lighting = None;
                } else {
                    self.scene_texture_for_lighting = None;
                    Self::ensure_intermediate_texture(
                        &mut self.post_texture_for_lighting,
                        &gpu.device,
                        "post_for_lighting",
                        w,
                        h,
                        fmt,
                    );
                }
            } else {
                self.lighting_renderer = None;
                self.scene_texture_for_lighting = None;
                self.post_texture_for_lighting = None;
            }
            has_lighting
        };
        #[cfg(target_arch = "wasm32")]
        let use_lighting = false;

        let frame = gpu.surface.get_current_texture()?;
        let final_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        // ── 오프스크린 패스: OffscreenCamera 엔티티마다 RT에 렌더 ─────────────
        {
            // 1단계: (target_name, camera, rt_width, rt_height, view_ptr, bind_group_clone) 수집
            // render_targets와 sprite_renderer를 동시에 borrow할 수 없으므로
            // raw pointer로 view 참조를 보관한다.
            // Safety: render_targets HashMap은 이 루프 안에서 수정되지 않는다.
            let offscreen_cams: Vec<(String, crate::camera::Camera, u32)> = self
                .world
                .query::<crate::components::OffscreenCamera>()
                .map(|(_, oc)| (oc.target.clone(), oc.camera, oc.layer_mask))
                .collect();

            let rt_info: Vec<OffscreenRenderInfo> = offscreen_cams
                .into_iter()
                .filter_map(|(name, cam, layer_mask)| {
                    self.render_targets.get(&name).map(|rt| {
                        (
                            name,
                            cam,
                            rt.width,
                            rt.height,
                            &rt.view as *const wgpu::TextureView,
                            std::sync::Arc::clone(&rt.bind_group),
                            layer_mask,
                        )
                    })
                })
                .collect();

            for (target_name, cam, rt_w, rt_h, view_ptr, bg, layer_mask) in rt_info {
                // ① 카메라 교체 — 기존 카메라가 없으면 렌더 후 제거, 있으면 원복
                let saved_cam = self.world.resource::<crate::camera::Camera>().copied();
                self.world.insert_resource(cam);

                // ② Safety: render_targets는 이 루프에서 수정되지 않는다.
                let rt_view = unsafe { &*view_ptr };

                // ③ RT clear
                {
                    let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("offscreen clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: rt_view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.0,
                                    g: 0.0,
                                    b: 0.0,
                                    a: 1.0,
                                }),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                }

                // ④ 스프라이트 렌더 → RT (layer_mask로 자기캡처 방지)
                if let Some(sr) = &mut self.sprite_renderer {
                    sr.render(
                        &mut FrameContext {
                            device: &gpu.device,
                            queue: &gpu.queue,
                            view: rt_view,
                            encoder: &mut enc,
                        },
                        &self.world,
                        rt_w,
                        rt_h,
                        layer_mask,
                    );
                }

                // ⑤ 카메라 복원 — 원래 없던 경우 제거해 World 오염 방지
                match saved_cam {
                    Some(c) => self.world.insert_resource(c),
                    None => {
                        self.world.remove_resource::<crate::camera::Camera>();
                    }
                }

                // ⑥ RT bind_group을 스프라이트 렌더러에 등록 (Sprite.texture 키로 샘플 가능)
                if let Some(sr) = &mut self.sprite_renderer {
                    sr.register_render_target(&target_name, bg);
                }
            }
        }

        // 렌더 타겟 선택:
        //   라이팅 있고 포스트 없음 → 중간 씬 텍스처
        //   포스트 있음 (라이팅 여부 무관) → post_renderer.target_view
        //   둘 다 없음 → 스왑체인 직접
        let render_view: &wgpu::TextureView = if use_lighting && !use_post {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some((_, view, _, _, _)) = self.scene_texture_for_lighting.as_ref() {
                    view
                } else {
                    log::warn!(
                        "lighting requested but scene texture is missing; rendering to final view"
                    );
                    &final_view
                }
            }
            #[cfg(target_arch = "wasm32")]
            {
                &final_view
            }
        } else if use_post {
            if let Some(pr) = self.post_renderer.as_ref() {
                &pr.target_view
            } else {
                log::warn!(
                    "post process requested but renderer is missing; rendering to final view"
                );
                &final_view
            }
        } else {
            &final_view
        };

        // 1단계: 배경 Clear
        let [cr, cg, cb, ca] = self
            .world
            .resource::<WindowConfig>()
            .map(|c| c.clear_color)
            .unwrap_or([0.08, 0.08, 0.12, 1.0]);
        {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: cr,
                            g: cg,
                            b: cb,
                            a: ca,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
        }

        let viewport = self
            .world
            .resource::<ViewportSize>()
            .copied()
            .unwrap_or_else(|| ViewportSize::new(gpu.config.width, gpu.config.height));
        let logical_w = viewport.width.round().max(1.0) as u32;
        let logical_h = viewport.height.round().max(1.0) as u32;

        // 2단계: 스프라이트 그리기 (메인 패스 — 레이어 필터 없음)
        if let Some(sr) = &mut self.sprite_renderer {
            let render_stats = sr.render(
                &mut FrameContext {
                    device: &gpu.device,
                    queue: &gpu.queue,
                    view: render_view,
                    encoder: &mut enc,
                },
                &self.world,
                logical_w,
                logical_h,
                0, // layer_mask = 0: 전체 레이어 렌더
            );
            if let Some(prof) = self.world.resource_mut::<crate::resources::ProfilerData>() {
                prof.render = render_stats;
            }
        }

        // 2.5단계: UI 사각형 그리기 (DebugDrawQueue → UiQueue 변환)
        let debug_rects: Vec<DrawRect> = self
            .world
            .resource_mut::<DebugDrawQueue>()
            .map(|q| {
                std::mem::take(&mut q.items)
                    .into_iter()
                    .map(|r| {
                        DrawRect::new(
                            r.min.x,
                            r.min.y,
                            r.max.x - r.min.x,
                            r.max.y - r.min.y,
                            r.color,
                        )
                        .with_z(r.z)
                    })
                    .collect()
            })
            .unwrap_or_default();
        if let Some(q) = self.world.resource_mut::<UiQueue>() {
            q.items.extend(debug_rects);
        }

        // 2.6단계: DebugDraw 도형 → UiQueue 변환 (Rect/Line/Circle/Cross)
        let debug_shapes: Vec<crate::resources::DebugShape> = self
            .world
            .resource_mut::<DebugDraw>()
            .map(|d| std::mem::take(&mut d.shapes))
            .unwrap_or_default();
        if !debug_shapes.is_empty() {
            if let Some(q) = self.world.resource_mut::<UiQueue>() {
                for shape in debug_shapes {
                    Self::debug_shape_to_draw_rects(shape, q);
                }
            }
        }

        let ui_images = self
            .world
            .resource_mut::<UiImageQueue>()
            .map(|q| std::mem::take(&mut q.items))
            .unwrap_or_default();
        let ui_rects: Vec<DrawRect> = self
            .world
            .resource_mut::<UiQueue>()
            .map(|q| std::mem::take(&mut q.items))
            .unwrap_or_default();
        if !ui_rects.is_empty() || !ui_images.is_empty() {
            if let Some(sr) = &mut self.sprite_renderer {
                sr.render_ui_primitives_from_slices(
                    &mut FrameContext {
                        device: &gpu.device,
                        queue: &gpu.queue,
                        view: render_view,
                        encoder: &mut enc,
                    },
                    &ui_rects,
                    &ui_images,
                    logical_w,
                    logical_h,
                );
            }
        }

        // 2.8단계: GPU 파티클 렌더링 (네이티브 전용)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let has_emitters = self
                .world
                .query::<crate::gpu_particle::GpuParticleEmitter>()
                .next()
                .is_some();
            if has_emitters && self.gpu_particle_renderer.is_none() {
                self.gpu_particle_renderer =
                    Some(crate::renderer::gpu_particle::GpuParticleRenderer::new(
                        &gpu.device,
                        gpu.config.format,
                        4096,
                    ));
            }
            if let Some(gpr) = &self.gpu_particle_renderer {
                let new_particles = crate::gpu_particle::collect_new_particles(
                    &mut self.world,
                    gpr.capacity(),
                    self.last_dt,
                );
                for (slot, p) in &new_particles {
                    gpr.upload_particles(&gpu.queue, std::slice::from_ref(p), *slot);
                }
                gpr.dispatch_compute(&mut enc, &gpu.queue, self.last_dt);
                gpr.render(
                    &gpu.queue,
                    render_view,
                    &mut enc,
                    &self.world,
                    logical_w,
                    logical_h,
                );
            }
        }

        // 4단계: 포스트프로세스 패스 (중간 텍스처 → 스왑체인 또는 라이팅 중간 텍스처)
        if use_post {
            #[cfg(not(target_arch = "wasm32"))]
            let post_output: &wgpu::TextureView = if use_lighting {
                self.post_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
                    .unwrap_or(&final_view)
            } else {
                &final_view
            };
            #[cfg(target_arch = "wasm32")]
            let post_output: &wgpu::TextureView = &final_view;

            if let (Some(pr), Some(cfg)) = (&self.post_renderer, pp_config.as_ref()) {
                pr.update_uniforms(&gpu.queue, cfg);
                pr.run_pass(&mut enc, post_output);
            }
        }

        // 4.5단계: 라이팅 패스
        #[cfg(not(target_arch = "wasm32"))]
        if use_lighting {
            if let Some(lr) = &self.lighting_renderer {
                // Light positions must use the same logical viewport the sprite pass
                // uses (render.rs:392), not the physical surface size — otherwise on a
                // HiDPI display (scale > 1) lights drift from their sprites and shrink.
                lr.update(&gpu.queue, &self.world, logical_w, logical_h);

                // 노멀 버퍼를 평면 노멀(0.5, 0.5, 1.0)으로 초기화한다.
                lr.clear_normal_buffer(&mut enc);

                // scene input: post가 있으면 post output, 없으면 씬 중간 텍스처
                let scene_input: Option<&wgpu::TextureView> = if use_post {
                    self.post_texture_for_lighting
                        .as_ref()
                        .map(|(_, view, _, _, _)| view)
                } else {
                    self.scene_texture_for_lighting
                        .as_ref()
                        .map(|(_, view, _, _, _)| view)
                };
                if let Some(scene_input) = scene_input {
                    lr.run_pass(&gpu.device, &mut enc, scene_input, &final_view);
                } else {
                    log::warn!("lighting pass skipped because scene input texture is missing");
                }
            }
        }

        // 4.7단계: HUD/텍스트 패스 — post·lighting 이후 final_view 에 그린다. 화면공간
        // HUD/텍스트가 월드 라이팅·포스트프로세스에 어두워지지 않게 하기 위함이다
        // (페이드 오버레이보다는 아래, egui 보다도 아래).
        {
            let (w, h) = (gpu.config.width, gpu.config.height);
            if let Some(tr) = &mut self.text_renderer {
                tr.render(
                    &gpu.device,
                    &gpu.queue,
                    &mut enc,
                    &final_view,
                    &mut self.world,
                    w,
                    h,
                );
            }
        }

        // 5단계 (pre): 페이드 오버레이 패스 (다른 모든 패스 이후 최상위)
        #[cfg(not(target_arch = "wasm32"))]
        {
            // 필요 시 lazy init
            if self.fade_renderer.is_none() {
                self.fade_renderer = Some(crate::renderer::fade::FadeRenderer::new(
                    &gpu.device,
                    gpu.config.format,
                ));
            }
            if let (Some(fr), Some(fade)) = (
                &self.fade_renderer,
                self.world.resource::<crate::resources::FadeTransition>(),
            ) {
                if fade.alpha > 0.001 {
                    fr.update(&gpu.queue, fade.color, fade.alpha);
                    fr.run_pass(&mut enc, &final_view);
                }
            }
        }

        // 씬+포스트프로세스+라이팅+페이드 완료 후 제출
        gpu.queue.submit(std::iter::once(enc.finish()));

        // 5단계: egui 오버레이 패스
        if let (Some(mut er), Some((paint_jobs, textures_delta, ppp))) =
            (self.egui_renderer.take(), self.egui_output.take())
        {
            let screen_desc = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [gpu.config.width, gpu.config.height],
                pixels_per_point: ppp,
            };
            for (id, delta) in &textures_delta.set {
                er.update_texture(&gpu.device, &gpu.queue, *id, delta);
            }
            let mut egui_enc = gpu
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("egui encoder"),
                });
            er.update_buffers(
                &gpu.device,
                &gpu.queue,
                &mut egui_enc,
                &paint_jobs,
                &screen_desc,
            );
            if paint_jobs_contain_callbacks(&paint_jobs) {
                log::warn!(
                    "egui paint callbacks are unsupported and were skipped to preserve render-pass lifetime safety"
                );
            } else {
                // Renderer::render<'rp>(&'rp self, &mut RenderPass<'rp>) 의 lifetime 제약 때문에
                // 독립 함수에서 &er 와 &mut egui_enc 를 동일 lifetime 'a 로 묶는다.
                egui_render_pass(&er, &mut egui_enc, &paint_jobs, &screen_desc, &final_view);
            }
            gpu.queue.submit(std::iter::once(egui_enc.finish()));
            for id in &textures_delta.free {
                er.free_texture(id);
            }
            self.egui_renderer = Some(er);
        }

        // winit 권장: present 직전 컴포지터에 통지해 표시 지연을 줄인다.
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        frame.present();
        Ok(())
    }

    pub(super) fn step_frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = self
            .last_frame
            .map(|t| (now - t).as_secs_f32().min(0.1))
            .unwrap_or(1.0 / 60.0);
        // 계측: 프레임 간격이 ~30fps(33ms)보다 벌어지면 기록한다. 라이브 드래그
        // 중 멈춤을 정량화하기 위한 디버그 로그 (RUST_LOG=debug 로 게이트됨).
        if dt > 0.033 {
            log::debug!("frame gap {:.1}ms (drag/stall?)", dt * 1000.0);
        }
        self.last_frame = Some(now);
        self.last_dt = dt;

        self.update(dt);

        // 시스템이 ShouldQuit(true) 를 설정했으면 종료
        if self
            .world
            .resource::<ShouldQuit>()
            .map(|q| q.0)
            .unwrap_or(false)
        {
            event_loop.exit();
            return;
        }

        // PendingResize: 게임이 요청한 해상도로 창 크기 변경
        let pending = self.world.resource::<PendingResize>().and_then(|r| r.0);
        if let Some((w, h)) = pending {
            if let Some(window) = &self.window {
                let _ = window.request_inner_size(winit::dpi::LogicalSize::new(w, h));
            }
            if let Some(r) = self.world.resource_mut::<PendingResize>() {
                *r = PendingResize(None);
            }
        }

        match self.render() {
            Ok(()) => {}
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                if let Some(gpu) = &self.gpu {
                    gpu.reconfigure();
                }
            }
            Err(e) => log::error!("렌더링 오류: {e:?}"),
        }
    }
}
