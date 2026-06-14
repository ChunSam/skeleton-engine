use super::egui_pass::{egui_render_pass, paint_jobs_contain_callbacks};
use super::*;
use winit::event_loop::ActiveEventLoop;

impl App {
    fn debug_shape_to_draw_rects(shape: crate::resources::DebugShape, q: &mut UiQueue) {
        use crate::resources::DebugShape;
        const Z: f32 = 999.0;

        // Line-segment approximation helper: fills the segment between two points with thickness×thickness dots.
        let mut push_line =
            |start: glam::Vec2, end: glam::Vec2, color: crate::color::Color, thickness: f32| {
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
                // top
                q.push(DrawRect::new(min.x, min.y, w, t, color).with_z(Z));
                // bottom
                q.push(DrawRect::new(min.x, max.y - t, w, t, color).with_z(Z));
                // left
                q.push(DrawRect::new(min.x, min.y, t, h, color).with_z(Z));
                // right
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

    fn render(&mut self) -> Result<(), wgpu::CurrentSurfaceTexture> {
        let gpu = match self.gpu.as_mut() {
            Some(g) => g,
            None => return Ok(()),
        };

        // ── Docked editor: manage the game-scene offscreen texture ────────────
        // The RT is recreated whenever the debounce fires (stable-3-frames rule).
        // While docked, the scene renders into this texture instead of the surface.
        #[cfg(not(target_arch = "wasm32"))]
        let docked_render_view: Option<wgpu::TextureView> = {
            use crate::app::editor::docked_rt::{compute_central_rect, rect_to_physical};
            use crate::app::editor::EditorMode;

            if self.editor.mode == EditorMode::Docked {
                let scale = self
                    .window
                    .as_ref()
                    .map(|w| w.scale_factor() as f32)
                    .unwrap_or(1.0)
                    .max(1.0);
                let win_logical_w = gpu.config.width as f32 / scale;
                let win_logical_h = gpu.config.height as f32 / scale;

                // Compute the central viewport rect (logical points).
                // Package 2 writes central_rect from real panel bounds; until then use
                // the placeholder-margin fallback.
                let rect = self
                    .editor
                    .central_rect
                    .or_else(|| compute_central_rect(win_logical_w, win_logical_h));

                if let Some(rect) = rect {
                    if let Some((target_pw, target_ph)) = rect_to_physical(rect, scale) {
                        // Tick the debounce — only recreate when stable for 3 frames.
                        let current_size = self
                            .docked_scene_texture
                            .as_ref()
                            .map(|(w, h, _, _, _)| (*w, *h));
                        if let Some((new_w, new_h)) = self
                            .editor
                            .rt_debounce
                            .tick((target_pw, target_ph), current_size)
                        {
                            // Free the old egui texture registration before recreating.
                            if let (Some(er), Some(old_id)) = (
                                &mut self.egui_renderer,
                                self.editor.docked_texture_id.take(),
                            ) {
                                er.free_texture(&old_id);
                            }
                            // Create new offscreen texture.
                            let tex = gpu.device.create_texture(&wgpu::TextureDescriptor {
                                label: Some("docked_scene"),
                                size: wgpu::Extent3d {
                                    width: new_w,
                                    height: new_h,
                                    depth_or_array_layers: 1,
                                },
                                mip_level_count: 1,
                                sample_count: 1,
                                dimension: wgpu::TextureDimension::D2,
                                format: gpu.config.format,
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                    | wgpu::TextureUsages::TEXTURE_BINDING,
                                view_formats: &[],
                            });
                            let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
                            self.docked_scene_texture =
                                Some((new_w, new_h, gpu.config.format, tex, view));

                            // Register with egui so CentralPanel can display the texture.
                            if let (Some(er), Some((_, _, _, _, view))) =
                                (&mut self.egui_renderer, &self.docked_scene_texture)
                            {
                                let id = er.register_native_texture(
                                    &gpu.device,
                                    view,
                                    wgpu::FilterMode::Linear,
                                );
                                self.editor.docked_texture_id = Some(id);
                            }
                        }

                        // Also refresh the egui registration when format changed (e.g. surface re-created).
                        if let Some((_, _, fmt, _, _)) = &self.docked_scene_texture {
                            if *fmt != gpu.config.format {
                                if let (Some(er), Some(old_id)) = (
                                    &mut self.egui_renderer,
                                    self.editor.docked_texture_id.take(),
                                ) {
                                    er.free_texture(&old_id);
                                }
                                // Force a debounce flush next frame — set stable_count to 0.
                                self.editor.rt_debounce.reset();
                                self.docked_scene_texture = None;
                            }
                        }

                        // Build a fresh TextureView from the current texture for this frame.
                        // The view stored in docked_scene_texture is authoritative; we borrow
                        // it as the render target for the scene pass below.
                        // We cannot return a &wgpu::TextureView here because it would
                        // borrow self for the rest of the function. Instead, create a
                        // second view from the texture (zero-cost, same GPU object).
                        self.docked_scene_texture.as_ref().map(|(_, _, _, tex, _)| {
                            tex.create_view(&wgpu::TextureViewDescriptor::default())
                        })
                    } else {
                        // Degenerate central rect (zero physical size) — skip scene render this frame.
                        None
                    }
                } else {
                    None
                }
            } else {
                // Not docked: tear down the RT so it's freed when mode exits.
                if self.docked_scene_texture.is_some() {
                    if let (Some(er), Some(old_id)) = (
                        &mut self.egui_renderer,
                        self.editor.docked_texture_id.take(),
                    ) {
                        er.free_texture(&old_id);
                    }
                    self.docked_scene_texture = None;
                    self.editor.rt_debounce.reset();
                }
                None
            }
        };
        // On WASM there is no docked mode; the scene always targets the surface.
        #[cfg(target_arch = "wasm32")]
        let docked_render_view: Option<wgpu::TextureView> = None;

        // Check PostProcessConfig resource (intermediate texture is used only when enabled=true)
        let pp_config: Option<PostProcessConfig> =
            self.world.resource::<PostProcessConfig>().copied();
        let use_post = pp_config.map(|c| c.enabled).unwrap_or(false);

        // Initialize / resize the post-process renderer
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

        // Initialize / resize / disable the lighting renderer
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
                // Create / resize the scene intermediate texture (only needed when post_renderer is absent)
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

        // In docked mode, skip the scene pass entirely when the RT is not yet ready.
        // This prevents drawing to a stale/wrong texture during the debounce warm-up.
        // `docked_render_view` is already computed above; re-check it here by testing
        // whether the RT is still None while mode == Docked.
        #[cfg(not(target_arch = "wasm32"))]
        {
            use crate::app::editor::EditorMode;
            if self.editor.mode == EditorMode::Docked && docked_render_view.is_none() {
                // RT not ready yet — still need to acquire + present the frame so the
                // window stays responsive, but skip all scene rendering.
                let (frame, suboptimal) = match gpu.surface.get_current_texture() {
                    wgpu::CurrentSurfaceTexture::Success(t) => (t, false),
                    wgpu::CurrentSurfaceTexture::Suboptimal(t) => (t, true),
                    e => return Err(e),
                };
                let final_view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                // Submit a clear-only pass so egui has a surface to draw on.
                let mut enc = gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("docked-wait encoder"),
                    });
                {
                    let _pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("docked-wait clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &final_view,
                            resolve_target: None,
                            depth_slice: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color {
                                    r: 0.08,
                                    g: 0.08,
                                    b: 0.12,
                                    a: 1.0,
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
                gpu.queue.submit(std::iter::once(enc.finish()));
                // Egui pass (shows "no game frame yet" placeholder).
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
                    let mut egui_enc =
                        gpu.device
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
                    egui_render_pass(&er, &mut egui_enc, &paint_jobs, &screen_desc, &final_view);
                    gpu.queue.submit(std::iter::once(egui_enc.finish()));
                    for id in &textures_delta.free {
                        er.free_texture(id);
                    }
                    self.egui_renderer = Some(er);
                }
                if let Some(window) = &self.window {
                    window.pre_present_notify();
                }
                frame.present();
                // If the surface became suboptimal (e.g. DPI/monitor change), reconfigure
                // so subsequent frames are optimal. Present first, reconfigure after.
                if suboptimal {
                    gpu.reconfigure();
                }
                return Ok(());
            }
        }

        let (frame, suboptimal) = match gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) => (t, false),
            wgpu::CurrentSurfaceTexture::Suboptimal(t) => (t, true),
            e => return Err(e),
        };
        let final_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
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
        {
            // Step 1: collect (target_name, camera, rt_width, rt_height, view_ptr, bind_group_clone)
            // render_targets and sprite_renderer cannot be borrowed simultaneously,
            // so we store the view reference as a raw pointer.
            // Safety: the render_targets HashMap is not modified inside this loop.
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
                // ① Swap camera — if no prior camera existed remove it after render, otherwise restore
                let saved_cam = self.world.resource::<crate::camera::Camera>().copied();
                self.world.insert_resource(cam);

                // ② Safety: render_targets is not modified during this loop.
                let rt_view = unsafe { &*view_ptr };

                // Each offscreen target is rendered with its **own command submission**.
                // The SpriteRenderer's camera uniform is a single shared buffer (`camera_buf`)
                // updated via `queue.write_buffer`. Within a single submit only the **last**
                // write to that buffer takes effect, so recording the offscreen draw and the main
                // pass in the same submit would cause the offscreen target to render with the
                // (later-written) **main camera**.
                // Submitting here ties this target's camera write and its draw together as a pair.
                let mut oenc = gpu
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("offscreen encoder"),
                    });

                // ③ Clear the RT
                {
                    let _pass = oenc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("offscreen clear"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: rt_view,
                            resolve_target: None,
                            depth_slice: None,
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
                        multiview_mask: None,
                    });
                }

                // ④ Render sprites → RT (layer_mask prevents self-capture)
                if let Some(sr) = &mut self.sprite_renderer {
                    sr.render(
                        &mut FrameContext {
                            device: &gpu.device,
                            queue: &gpu.queue,
                            view: rt_view,
                            encoder: &mut oenc,
                        },
                        &self.world,
                        rt_w,
                        rt_h,
                        layer_mask,
                    );
                }

                // Immediately flush this target's camera write + draw. (Applied before the main
                // pass's camera write; the RT texture is filled before the main pass samples it.)
                gpu.queue.submit(std::iter::once(oenc.finish()));

                // ⑤ Restore camera — remove it if it was absent before to avoid polluting the World
                match saved_cam {
                    Some(c) => self.world.insert_resource(c),
                    None => {
                        self.world.remove_resource::<crate::camera::Camera>();
                    }
                }

                // ⑥ Register the RT bind_group with the sprite renderer (sampleable via Sprite.texture key)
                if let Some(sr) = &mut self.sprite_renderer {
                    sr.register_render_target(&target_name, bg);
                }
            }
        }

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

        // Step 1: Clear background
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

        // Step 2: Draw sprites (main pass — no layer filter)
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
                0, // layer_mask = 0: render all layers
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

        // Step 2.8: GPU particle rendering (native only)
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

        // Step 4: Post-process pass (intermediate texture → scene_target or lighting intermediate texture)
        if use_post {
            #[cfg(not(target_arch = "wasm32"))]
            let post_output: &wgpu::TextureView = if use_lighting {
                self.post_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
                    .unwrap_or(scene_target)
            } else {
                scene_target
            };
            #[cfg(target_arch = "wasm32")]
            let post_output: &wgpu::TextureView = scene_target;

            if let (Some(pr), Some(cfg)) = (&self.post_renderer, pp_config.as_ref()) {
                pr.update_uniforms(&gpu.queue, cfg);
                pr.run_pass(&mut enc, post_output);
            }
        }

        // Step 4.5: Lighting pass
        #[cfg(not(target_arch = "wasm32"))]
        if use_lighting {
            // scene input: post output if post is enabled, otherwise the scene intermediate texture
            let scene_input: Option<&wgpu::TextureView> = if use_post {
                self.post_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
            } else {
                self.scene_texture_for_lighting
                    .as_ref()
                    .map(|(_, view, _, _, _)| view)
            };
            if let Some(lr) = &mut self.lighting_renderer {
                // Light positions must use the same logical viewport the sprite pass
                // uses (render.rs), not the physical surface size — otherwise on a
                // HiDPI display (scale > 1) lights drift from their sprites and shrink.
                lr.update(&gpu.queue, &self.world, logical_w, logical_h);

                // Initialize the normal buffer to a flat normal (0.5, 0.5, 1.0).
                lr.clear_normal_buffer(&mut enc);

                if let Some(scene_input) = scene_input {
                    lr.run_pass(&gpu.device, &mut enc, scene_input, scene_target);
                } else {
                    log::warn!("lighting pass skipped because scene input texture is missing");
                }
            }
        }

        // Step 4.7: HUD/text pass — drawn onto scene_target after post and lighting. In
        // normal mode scene_target == final_view; in docked mode it's the offscreen texture
        // so text renders into the game viewport (not over the editor chrome).
        {
            #[cfg(not(target_arch = "wasm32"))]
            let (w, h) = if is_docked_with_rt {
                // In docked mode, text should lay out to the offscreen texture size (which
                // matches the viewport logical size × scale).
                (
                    self.docked_scene_texture
                        .as_ref()
                        .map(|(w, _, _, _, _)| *w)
                        .unwrap_or(gpu.config.width),
                    self.docked_scene_texture
                        .as_ref()
                        .map(|(_, h, _, _, _)| *h)
                        .unwrap_or(gpu.config.height),
                )
            } else {
                (gpu.config.width, gpu.config.height)
            };
            #[cfg(target_arch = "wasm32")]
            let (w, h) = (gpu.config.width, gpu.config.height);
            if let Some(tr) = &mut self.text_renderer {
                tr.render(
                    &gpu.device,
                    &gpu.queue,
                    &mut enc,
                    scene_target,
                    &mut self.world,
                    w,
                    h,
                );
            }
        }

        // Step 5 (pre): Fade overlay pass (topmost game-scene pass, before egui)
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Lazy init if needed
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
                    fr.update(&gpu.queue, fade.color.to_rgb(), fade.alpha);
                    // Fade covers the game scene; in docked mode that's the offscreen texture.
                    fr.run_pass(&mut enc, scene_target);
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.08,
                            b: 0.12,
                            a: 1.0,
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

        // Submit after scene + post-process + lighting + fade are complete
        gpu.queue.submit(std::iter::once(enc.finish()));

        // Step 5: egui overlay pass
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
                // Due to the Renderer::render<'rp>(&'rp self, &mut RenderPass<'rp>) lifetime constraint,
                // we tie &er and &mut egui_enc to the same lifetime 'a in a standalone function.
                egui_render_pass(&er, &mut egui_enc, &paint_jobs, &screen_desc, &final_view);
            }
            gpu.queue.submit(std::iter::once(egui_enc.finish()));
            for id in &textures_delta.free {
                er.free_texture(id);
            }
            self.egui_renderer = Some(er);
        }

        // winit recommendation: notify the compositor just before present to reduce display latency.
        if let Some(window) = &self.window {
            window.pre_present_notify();
        }
        frame.present();
        // If the surface became suboptimal (e.g. DPI/monitor change), reconfigure
        // so subsequent frames are optimal. Present first, reconfigure after.
        if suboptimal {
            gpu.reconfigure();
        }
        Ok(())
    }

    pub(super) fn step_frame(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = self
            .last_frame
            .map(|t| (now - t).as_secs_f32().min(0.1))
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
