use super::super::App;
use super::EDITOR_SURFACE_CLEAR;
use crate::app::egui_pass::submit_egui;
use crate::app::render_state::RenderState;
use crate::renderer::{
    DrawRect, FrameContext, GpuContext, PostProcessConfig, UiImageQueue, UiQueue,
};
use crate::resources::{
    DebugDraw, Letterbox, PendingResize, ShouldQuit, ViewportSize, WindowConfig,
    DEFAULT_CLEAR_COLOR,
};
use winit::event_loop::ActiveEventLoop;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

impl App {
    /// Final egui overlay pass: record + submit the editor/debug UI onto the surface view.
    /// Paint callbacks are unsupported (skipped) to preserve render-pass lifetime safety.
    /// Extracted from `render()`.
    fn present_egui(render: &mut RenderState, gpu: &GpuContext, final_view: &wgpu::TextureView) {
        // `guard_callbacks = true`: this is the final surface overlay, where paint callbacks are
        // unsupported (render-pass lifetime safety) and skipped with a warn.
        submit_egui(render, gpu, final_view, true);
    }

    pub(in crate::app) fn render(&mut self) -> Result<(), wgpu::CurrentSurfaceTexture> {
        let gpu = match self.gpu.as_mut() {
            Some(g) => g,
            None => return Ok(()),
        };

        // ── Docked editor: manage the game-scene offscreen texture ────────────
        // While docked, the scene renders into a debounced offscreen RT (recreated on the
        // stable-3-frames rule) instead of the surface; `prepare_docked_scene_view` owns that
        // lifecycle and returns this frame's scene target (`None` = warming up / not docked).
        #[cfg(not(target_arch = "wasm32"))]
        let docked_render_view: Option<wgpu::TextureView> = Self::prepare_docked_scene_view(
            &mut self.render,
            &mut self.editor,
            self.window.as_deref(),
            gpu,
        );
        // On WASM there is no docked mode; the scene always targets the surface.
        #[cfg(target_arch = "wasm32")]
        let docked_render_view: Option<wgpu::TextureView> = None;

        // Check PostProcessConfig resource (intermediate texture is used only when enabled=true)
        let pp_config: Option<PostProcessConfig> =
            self.world.resource::<PostProcessConfig>().copied();
        let use_post = pp_config.map(|c| c.enabled).unwrap_or(false);

        // Initialize / resize the post-process renderer. When HDR is requested the scene
        // intermediate is Rgba16Float (so > 1.0 colours survive to be tone-mapped); the pass still
        // outputs the surface format.
        if use_post {
            let inter_fmt = if pp_config.map(|c| c.hdr).unwrap_or(false) {
                wgpu::TextureFormat::Rgba16Float
            } else {
                gpu.config.format
            };
            Self::setup_post_renderer(
                &mut self.render,
                &gpu.device,
                gpu.config.width,
                gpu.config.height,
                gpu.config.format,
                inter_fmt,
            );
            // The real multi-pass bloom pass runs on the scene intermediate (same format), before
            // the post-process composite. Set it up only when requested.
            if pp_config.map(|c| c.bloom).unwrap_or(false) {
                Self::setup_bloom_renderer(
                    &mut self.render,
                    &gpu.device,
                    gpu.config.width,
                    gpu.config.height,
                    inter_fmt,
                );
            }
        }

        // Initialize / resize / disable the lighting renderer
        #[cfg(not(target_arch = "wasm32"))]
        let use_lighting = Self::setup_lighting(
            &mut self.render,
            &self.world,
            &gpu.device,
            gpu.config.width,
            gpu.config.height,
            gpu.config.format,
            use_post,
        );
        #[cfg(target_arch = "wasm32")]
        let use_lighting = false;

        // In docked mode, skip the scene pass entirely when the RT is not yet ready.
        // This prevents drawing to a stale/wrong texture during the debounce warm-up.
        // `docked_render_view` is already computed above; re-check it here by testing
        // whether the RT is still None while mode == Docked.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::app::editor::EditorMode;
            if self.editor.mode == EditorMode::Docked && docked_render_view.is_none() {
                return Self::present_docked_placeholder(
                    &mut self.render,
                    self.window.as_deref(),
                    gpu,
                );
            }
        }

        // Acquire the final render target: the swapchain texture (windowed) or the offscreen
        // headless color texture (no window/surface — for `save_screenshot_headless`).
        let (surface_frame, suboptimal, final_view) = match &gpu.surface {
            Some(surface) => {
                let (frame, suboptimal) = match surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) => (t, false),
                    wgpu::CurrentSurfaceTexture::Suboptimal(t) => (t, true),
                    e => return Err(e),
                };
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                (Some(frame), suboptimal, view)
            }
            None => (None, false, gpu.headless_view()),
        };
        // In docked mode the scene pipeline terminates at the offscreen texture;
        // the surface only receives egui.  All passes that used to write to
        // `final_view` now write to `scene_target` instead.
        // In normal mode, scene_target == final_view.
        let scene_target: &wgpu::TextureView = match &docked_render_view {
            Some(drv) => drv,
            None => &final_view,
        };
        let mut enc = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });

        // ── Offscreen pass: render each OffscreenCamera entity into its RT ─────────────
        Self::render_offscreen_targets(&mut self.render, &mut self.world, gpu);

        // In docked mode the scene renders into a separate offscreen texture; the
        // surface pass then shows only the egui UI (which contains the game image).
        // When the docked RT is not yet ready (debounce not fired), skip the scene
        // pass for this frame.
        let is_docked_with_rt = docked_render_view.is_some();

        // Select render target:
        //   docked mode + RT ready → docked offscreen texture
        //   lighting without post → intermediate scene texture
        //   post enabled (regardless of lighting) → post_renderer.target_view
        //   neither → swapchain directly
        //
        // Note: `docked_render_view` is an owned `wgpu::TextureView` built from
        // the docked texture each frame, so we can borrow it here.
        let render_view: &wgpu::TextureView = if let Some(ref drv) = docked_render_view {
            drv
        } else if use_lighting && !use_post {
            #[cfg(not(target_arch = "wasm32"))]
            {
                if let Some((_, view, _, _, _)) = self.render.scene_texture_for_lighting.as_ref() {
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
            if let Some(pr) = self.render.post_renderer.as_ref() {
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

        // Format of `render_view` — the scene passes must build pipelines matching it. It differs
        // from the surface format only in the HDR post-process case, where `render_view` is the
        // post-process renderer's `Rgba16Float` intermediate. (Docked / lighting-without-post route
        // through surface-format textures, so they stay `gpu.config.format`.)
        let scene_format: wgpu::TextureFormat = if docked_render_view.is_none() && use_post {
            self.render
                .post_renderer
                .as_ref()
                .map(|pr| pr.intermediate_format())
                .unwrap_or(gpu.config.format)
        } else {
            gpu.config.format
        };

        // Step 1: Clear background
        let [cr, cg, cb, ca] = self
            .world
            .resource::<WindowConfig>()
            .map(|c| c.clear_color)
            .unwrap_or(DEFAULT_CLEAR_COLOR);
        {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_view,
                    resolve_target: None,
                    depth_slice: None,
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
                multiview_mask: None,
            });
        }

        let viewport = self
            .world
            .resource::<ViewportSize>()
            .copied()
            .unwrap_or_else(|| ViewportSize::new(gpu.config.width, gpu.config.height));
        let logical_w = viewport.width.round().max(1.0) as u32;
        let logical_h = viewport.height.round().max(1.0) as u32;
        // The design-resolution letterbox clip-scale (identity when no DesignResolution is set, so
        // the projections are byte-identical to the non-letterboxed path). Applies to the main
        // surface pass only — offscreen render targets always pass identity.
        let clip_scale = self
            .world
            .resource::<Letterbox>()
            .map(|lb| lb.clip_scale)
            .unwrap_or(glam::Vec2::ONE);

        // Step 2: Draw sprites (main pass — no layer filter)
        if let Some(sr) = &mut self.render.sprite_renderer {
            let render_stats = sr.render(
                &mut FrameContext {
                    device: &gpu.device,
                    queue: &gpu.queue,
                    view: render_view,
                    format: scene_format,
                    encoder: &mut enc,
                },
                &self.world,
                logical_w,
                logical_h,
                0, // layer_mask = 0: render all layers
                clip_scale,
            );
            if let Some(prof) = self.world.resource_mut::<crate::resources::ProfilerData>() {
                prof.render = render_stats;
            }
        }

        // Step 2.6: Convert DebugDraw shapes + filled rects → UiQueue
        let (debug_shapes, debug_filled) = self
            .world
            .resource_mut::<DebugDraw>()
            .map(|d| {
                (
                    std::mem::take(&mut d.shapes),
                    std::mem::take(&mut d.filled_rects),
                )
            })
            .unwrap_or_default();
        if !debug_shapes.is_empty() || !debug_filled.is_empty() {
            if let Some(q) = self.world.resource_mut::<UiQueue>() {
                for shape in debug_shapes {
                    Self::debug_shape_to_draw_rects(shape, q);
                }
                for r in debug_filled {
                    q.items.push(
                        DrawRect::new(
                            r.min.x,
                            r.min.y,
                            r.max.x - r.min.x,
                            r.max.y - r.min.y,
                            r.color,
                        )
                        .with_z(r.z),
                    );
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
        // Texts with an explicit UI-layer z (`DrawText::with_z`) leave the queue here and
        // composite among the UI rects/images below; z-None texts stay queued for the on-top
        // text pass (step 4.7) — the historical always-on-top behavior.
        let mut layered_texts: Vec<crate::renderer::DrawText> = self
            .world
            .resource_mut::<crate::renderer::TextQueue>()
            .map(|q| q.take_layered())
            .unwrap_or_default();
        // Physical pixel size of the text viewport — the docked offscreen texture in docked
        // mode, else the surface size. Shared by the layered batches here and step 4.7.
        #[cfg(not(target_arch = "wasm32"))]
        let (text_w, text_h) = if is_docked_with_rt {
            self.render
                .docked_scene_texture
                .as_ref()
                .map(|(w, h, _, _, _)| (*w, *h))
                .unwrap_or((gpu.config.width, gpu.config.height))
        } else {
            (gpu.config.width, gpu.config.height)
        };
        #[cfg(target_arch = "wasm32")]
        let (text_w, text_h) = (gpu.config.width, gpu.config.height);
        if !ui_rects.is_empty() || !ui_images.is_empty() || !layered_texts.is_empty() {
            if let Some(sr) = &mut self.render.sprite_renderer {
                // The UI-primitive pass picks a pipeline matching `scene_format`, so it renders
                // correctly into the HDR post-process intermediate as well as the surface.
                if layered_texts.is_empty() {
                    // Fast path — no layered text: one pass over every primitive, as before.
                    sr.render_ui_primitives_from_slices(
                        &mut FrameContext {
                            device: &gpu.device,
                            queue: &gpu.queue,
                            view: render_view,
                            format: scene_format,
                            encoder: &mut enc,
                        },
                        &ui_rects,
                        &ui_images,
                        logical_w,
                        logical_h,
                        clip_scale,
                    );
                } else {
                    // Z-interleaved path: upload the sorted primitives ONCE, then alternate
                    // primitive sub-ranges with pooled glyphon text batches so a surface drawn
                    // above a text's z actually covers it (an open dropdown list / tooltip /
                    // panel over a widget label). Stable sort keeps push order on z ties.
                    layered_texts
                        .sort_by(|a, b| a.z.partial_cmp(&b.z).unwrap_or(std::cmp::Ordering::Equal));
                    let prepared = sr.prepare_ui_primitives(
                        &gpu.device,
                        &gpu.queue,
                        &ui_rects,
                        &ui_images,
                        logical_w,
                        logical_h,
                        clip_scale,
                    );
                    let text_zs: Vec<f32> = layered_texts.iter().filter_map(|t| t.z).collect();
                    let runs = crate::renderer::text::interleave_runs(&prepared.zs, &text_zs);
                    let text_scale = self
                        .world
                        .resource::<crate::resources::DisplayScaleFactor>()
                        .map(|s| s.0)
                        .unwrap_or(1.0)
                        .max(1.0);
                    let text_lb = self
                        .world
                        .resource::<crate::resources::Letterbox>()
                        .copied()
                        .unwrap_or_default();
                    let mut next_surface = 0usize;
                    let mut texts_iter = layered_texts.into_iter();
                    for run in runs {
                        if run.surfaces > 0 {
                            sr.render_ui_primitive_range(
                                &mut FrameContext {
                                    device: &gpu.device,
                                    queue: &gpu.queue,
                                    view: render_view,
                                    format: scene_format,
                                    encoder: &mut enc,
                                },
                                &prepared,
                                next_surface,
                                next_surface + run.surfaces,
                            );
                            next_surface += run.surfaces;
                        }
                        if run.texts > 0 {
                            let batch: Vec<_> = texts_iter.by_ref().take(run.texts).collect();
                            if let Some(tr) = &mut self.render.text_renderer {
                                tr.render_batch(
                                    &gpu.device,
                                    &gpu.queue,
                                    &mut enc,
                                    render_view,
                                    scene_format,
                                    batch,
                                    text_w,
                                    text_h,
                                    text_scale,
                                    text_lb,
                                );
                            }
                        }
                    }
                }
            }
        }

        // Step 2.8: GPU particle rendering (native only)
        #[cfg(not(target_arch = "wasm32"))]
        {
            let has_emitters = self
                .world
                .query::<crate::gpu_particle::GpuParticleEmitter>()
                .next()
                .is_some();
            if has_emitters && self.render.gpu_particle_renderer.is_none() {
                // Capacity is tunable via an optional GpuParticleConfig resource (default 4096).
                let capacity = self
                    .world
                    .resource::<crate::gpu_particle::GpuParticleConfig>()
                    .copied()
                    .unwrap_or_default()
                    .capacity;
                self.render.gpu_particle_renderer =
                    Some(crate::renderer::gpu_particle::GpuParticleRenderer::new(
                        &gpu.device,
                        gpu.config.format,
                        capacity,
                    ));
            }
            // GPU particles are format-matched: lazily build (and cache) a render pipeline for the
            // scene's target format, so they render correctly into the `Rgba16Float` HDR
            // post-process intermediate too (a no-op for the surface format = the common case).
            if has_emitters {
                if let Some(gpr) = self.render.gpu_particle_renderer.as_mut() {
                    gpr.ensure_render_pipeline(&gpu.device, scene_format);
                }
            }
            if let Some(gpr) = &self.render.gpu_particle_renderer {
                let mut frame_cursor = 0u32;
                let new_particles = crate::gpu_particle::collect_new_particles(
                    &mut self.world,
                    gpr.capacity(),
                    self.last_dt,
                    &mut frame_cursor,
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
                    scene_format,
                    clip_scale,
                );
            }
        }

        // Step 3: User render plugins — custom scene passes (outlines, overlays, effects).
        // Records into render_view before post-process/lighting so downstream effects apply.
        // No-op (and byte-identical to no-plugin output) when none are registered.
        if !self.render.render_plugins.is_empty() {
            let mut plugin_ctx = FrameContext {
                device: &gpu.device,
                queue: &gpu.queue,
                view: render_view,
                format: scene_format,
                encoder: &mut enc,
            };
            for plugin in &mut self.render.render_plugins {
                plugin.record(&mut plugin_ctx, &self.world, (logical_w, logical_h));
            }
        }

        // Step 3.5: Bloom pass — extract bright highlights, blur, and additively composite the glow
        // back onto the scene intermediate, before the post-process composite. Only when post is on
        // and bloom is requested; the post shader then skips its cheap inline bloom (bloom_enabled).
        if use_post {
            if let Some(cfg) = pp_config.as_ref() {
                if cfg.bloom {
                    if let Some(br) = self.render.bloom_renderer.as_ref() {
                        br.update(&gpu.queue, cfg);
                        br.run(&gpu.device, &mut enc, render_view, cfg.bloom_iterations);
                    }
                }
            }
        }

        // Step 4: Post-process pass (intermediate texture → scene_target or lighting intermediate texture)
        if use_post {
            #[cfg(not(target_arch = "wasm32"))]
            let post_output: &wgpu::TextureView = if use_lighting {
                self.render
                    .post_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
                    .unwrap_or(scene_target)
            } else {
                scene_target
            };
            #[cfg(target_arch = "wasm32")]
            let post_output: &wgpu::TextureView = scene_target;

            if let (Some(pr), Some(cfg)) = (&self.render.post_renderer, pp_config.as_ref()) {
                pr.update_uniforms(&gpu.queue, cfg);
                pr.run_pass(&mut enc, post_output);
            }
        }

        // Step 4.5: Lighting pass
        #[cfg(not(target_arch = "wasm32"))]
        if use_lighting {
            // scene input: post output if post is enabled, otherwise the scene intermediate texture
            let scene_input: Option<&wgpu::TextureView> = if use_post {
                self.render
                    .post_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
            } else {
                self.render
                    .scene_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
            };
            if let Some(lr) = &mut self.render.lighting_renderer {
                // Light positions must use the same logical viewport the sprite pass
                // uses (render.rs), not the physical surface size — otherwise on a
                // HiDPI display (scale > 1) lights drift from their sprites and shrink.
                lr.update(&gpu.queue, &self.world, logical_w, logical_h, clip_scale);

                // Initialize the normal buffer to a flat normal (0.5, 0.5, 1.0).
                lr.clear_normal_buffer(&mut enc);

                if let Some(scene_input) = scene_input {
                    lr.run_pass(&gpu.device, &mut enc, scene_input, scene_target);
                } else {
                    log::warn!("lighting pass skipped because scene input texture is missing");
                }
            }
        }

        // Step 4.7: HUD/text pass — the remaining (z-None, always-on-top) texts, drawn onto
        // scene_target after post and lighting. In normal mode scene_target == final_view; in
        // docked mode it's the offscreen texture so text renders into the game viewport (not
        // over the editor chrome). `text_w`/`text_h` were computed with the layered batches at
        // step 2.7 (docked offscreen size in docked mode, else the surface size).
        if let Some(tr) = &mut self.render.text_renderer {
            tr.render(
                &gpu.device,
                &gpu.queue,
                &mut enc,
                scene_target,
                &mut self.world,
                text_w,
                text_h,
            );
            // One per-frame text cleanup after the LAST batch: shaped-cache eviction, atlas
            // trims, pooled-renderer reset, generation bump.
            tr.end_frame();
        }

        // Step 5 (pre): Fade overlay pass (topmost game-scene pass, before egui)
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Lazy init if needed
            if self.render.fade_renderer.is_none() {
                self.render.fade_renderer = Some(crate::renderer::fade::FadeRenderer::new(
                    &gpu.device,
                    gpu.config.format,
                ));
            }
            if let (Some(fr), Some(fade)) = (
                &self.render.fade_renderer,
                self.world.resource::<crate::resources::FadeTransition>(),
            ) {
                if fade.alpha > 0.001 {
                    fr.update(&gpu.queue, fade.color.to_rgb(), fade.alpha);
                    // Fade covers the game scene; in docked mode that's the offscreen texture.
                    fr.run_pass(&mut enc, scene_target);
                }
            }
        }

        // Step 5 (pre): Styled scene-transition overlay (fade/wipe/iris) — same slot as the fade.
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.render.transition_renderer.is_none() {
                self.render.transition_renderer =
                    Some(crate::renderer::transition::TransitionRenderer::new(
                        &gpu.device,
                        gpu.config.format,
                    ));
            }
            if let (Some(tr), Some(t)) = (
                &self.render.transition_renderer,
                self.world
                    .resource::<crate::scene_transition::SceneTransition>(),
            ) {
                if t.coverage > 0.001 {
                    let aspect = gpu.config.width as f32 / gpu.config.height.max(1) as f32;
                    let c = t.color;
                    tr.update(
                        &gpu.queue,
                        t.coverage,
                        t.style.shader_index(),
                        aspect,
                        [c.r, c.g, c.b, c.a],
                    );
                    tr.run_pass(&mut enc, scene_target);
                }
            }
        }

        // In docked mode the surface (final_view) has not been cleared yet — the scene
        // pass wrote to the offscreen texture.  Clear the surface to black so egui has
        // a clean background.  The egui pass uses LoadOp::Load, so this clear persists.
        if is_docked_with_rt {
            let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("docked surface clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &final_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(EDITOR_SURFACE_CLEAR),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        // Submit after scene + post-process + lighting + fade are complete
        gpu.queue.submit(std::iter::once(enc.finish()));

        // Step 5: egui overlay pass
        Self::present_egui(&mut self.render, gpu, &final_view);

        // winit recommendation: notify the compositor just before present to reduce display latency.
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        // Headless mode has no swapchain to present; the frame stays in the offscreen texture
        // for read-back. Windowed mode presents and reconfigures on a suboptimal surface.
        if let Some(frame) = surface_frame {
            frame.present();
            if suboptimal {
                gpu.reconfigure();
            }
        }
        Ok(())
    }

    /// Advance the game loop at most once per event-loop iteration. Both `Resized`
    /// (macOS modal resize) and `RedrawRequested` can fire in the same iteration; the
    /// `stepped_this_iteration` guard prevents a double step (reset in `about_to_wait`).
    pub(in crate::app) fn step_frame_once(&mut self, event_loop: &ActiveEventLoop) {
        if !self.stepped_this_iteration {
            self.stepped_this_iteration = true;
            self.step_frame(event_loop);
        }
    }

    pub(in crate::app) fn step_frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        // Cap the frame delta so a single stall (window drag, breakpoint, backgrounded tab)
        // can't hand systems a huge dt that tunnels physics / leaps animations. Configurable
        // via the `FrameConfig` resource; absent → the historical 0.1 s cap.
        let frame_cfg = self
            .world
            .resource::<crate::resources::FrameConfig>()
            .copied()
            .unwrap_or_default();
        let dt = self
            .last_frame
            .map(|t| frame_cfg.cap((now - t).as_secs_f32()))
            .unwrap_or(1.0 / 60.0);
        // Telemetry: log when the frame gap exceeds ~30fps (33ms). Debug log to quantify
        // stalls during live drag (gated by RUST_LOG=debug).
        if dt > 0.033 {
            log::debug!("frame gap {:.1}ms (drag/stall?)", dt * 1000.0);
        }
        self.last_frame = Some(now);
        self.last_dt = dt;

        self.update(dt);

        // Exit if a system set ShouldQuit(true)
        if self
            .world
            .resource::<ShouldQuit>()
            .map(|q| q.0)
            .unwrap_or(false)
        {
            event_loop.exit();
            return;
        }

        // PendingResize: resize the window to the resolution requested by the game
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
            Err(wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated) => {
                if let Some(gpu) = &self.gpu {
                    gpu.reconfigure();
                }
            }
            // Transient/benign conditions: skip this frame silently.
            // Occluded = window minimized or behind another window.
            // Timeout  = compositor took too long; try again next frame.
            Err(wgpu::CurrentSurfaceTexture::Occluded | wgpu::CurrentSurfaceTexture::Timeout) => {}
            // Genuine errors (Validation, etc.) still surface as errors.
            Err(e) => log::error!("render error: {e:?}"),
        }
    }
}
