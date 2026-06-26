use super::super::App;
use super::EDITOR_SURFACE_CLEAR;
use crate::app::editor::EditorState;
use crate::app::egui_pass::submit_egui;
use crate::app::render_state::RenderState;
use crate::renderer::GpuContext;

impl App {
    /// Docked-editor warm-up frame: the game-scene offscreen RT is not ready yet (debounce not
    /// fired), so skip all scene rendering but still acquire + clear + present the surface with
    /// the egui overlay, keeping the window responsive. Returns the present result. Extracted from
    /// `render()`; native-only (no docked mode on wasm).
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn present_docked_placeholder(
        render: &mut RenderState,
        window: Option<&winit::window::Window>,
        gpu: &mut GpuContext,
    ) -> Result<(), wgpu::CurrentSurfaceTexture> {
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
        gpu.queue.submit(std::iter::once(enc.finish()));
        // Egui pass (shows "no game frame yet" placeholder). `guard_callbacks = false`: this
        // placeholder UI never produces paint callbacks, so the pass is recorded directly.
        submit_egui(render, gpu, &final_view, false);
        if let Some(window) = window {
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

    /// Docked-editor scene-target management: when in docked mode, (re)create the offscreen
    /// game-scene texture on the debounce schedule, keep its egui registration in sync, and
    /// return a fresh `TextureView` of it to use as this frame's scene render target.
    ///
    /// Returns `None` when not docked (after tearing down any existing RT) or while the RT is
    /// still warming up — in which case `render()` targets the surface as usual, so the common
    /// (non-docked) render path is byte-identical. Extracted from `render()`; native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn prepare_docked_scene_view(
        render: &mut RenderState,
        editor: &mut EditorState,
        window: Option<&winit::window::Window>,
        gpu: &GpuContext,
    ) -> Option<wgpu::TextureView> {
        use crate::app::editor::docked_rt::{compute_central_rect, rect_to_physical};
        use crate::app::editor::EditorMode;

        if editor.mode == EditorMode::Docked {
            let scale = window
                .map(|w| w.scale_factor() as f32)
                .unwrap_or(1.0)
                .max(1.0);
            let win_logical_w = gpu.config.width as f32 / scale;
            let win_logical_h = gpu.config.height as f32 / scale;

            // Compute the central viewport rect (logical points).
            // Package 2 writes central_rect from real panel bounds; until then use
            // the placeholder-margin fallback.
            let rect = editor
                .central_rect
                .or_else(|| compute_central_rect(win_logical_w, win_logical_h));

            if let Some(rect) = rect {
                if let Some((target_pw, target_ph)) = rect_to_physical(rect, scale) {
                    // Tick the debounce — only recreate when stable for 3 frames.
                    let current_size = render
                        .docked_scene_texture
                        .as_ref()
                        .map(|(w, h, _, _, _)| (*w, *h));
                    if let Some((new_w, new_h)) = editor
                        .rt_debounce
                        .tick((target_pw, target_ph), current_size)
                    {
                        // Free the old egui texture registration before recreating.
                        {
                            let RenderState {
                                egui_renderer,
                                docked_scene_texture,
                                ..
                            } = &mut *render;
                            if let (Some(er), Some(old_id)) =
                                (egui_renderer.as_mut(), editor.docked_texture_id.take())
                            {
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
                            *docked_scene_texture =
                                Some((new_w, new_h, gpu.config.format, tex, view));

                            // Register with egui so CentralPanel can display the texture.
                            if let (Some(er), Some((_, _, _, _, view))) =
                                (egui_renderer.as_mut(), docked_scene_texture.as_ref())
                            {
                                let id = er.register_native_texture(
                                    &gpu.device,
                                    view,
                                    wgpu::FilterMode::Linear,
                                );
                                editor.docked_texture_id = Some(id);
                            }
                        }
                    }

                    // Also refresh the egui registration when format changed (e.g. surface re-created).
                    {
                        let RenderState {
                            egui_renderer,
                            docked_scene_texture,
                            ..
                        } = &mut *render;
                        if let Some((_, _, fmt, _, _)) = docked_scene_texture.as_ref() {
                            if *fmt != gpu.config.format {
                                if let (Some(er), Some(old_id)) =
                                    (egui_renderer.as_mut(), editor.docked_texture_id.take())
                                {
                                    er.free_texture(&old_id);
                                }
                                // Force a debounce flush next frame — set stable_count to 0.
                                editor.rt_debounce.reset();
                                *docked_scene_texture = None;
                            }
                        }
                    }

                    // Build a fresh TextureView from the current texture for this frame.
                    // The view stored in docked_scene_texture is authoritative; we borrow
                    // it as the render target for the scene pass below.
                    // We cannot return a borrowed &wgpu::TextureView here because it would
                    // borrow `render` for the rest of the caller. Instead, create a
                    // second view from the texture (zero-cost, same GPU object).
                    render
                        .docked_scene_texture
                        .as_ref()
                        .map(|(_, _, _, tex, _)| {
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
            {
                let RenderState {
                    egui_renderer,
                    docked_scene_texture,
                    ..
                } = &mut *render;
                if docked_scene_texture.is_some() {
                    if let (Some(er), Some(old_id)) =
                        (egui_renderer.as_mut(), editor.docked_texture_id.take())
                    {
                        er.free_texture(&old_id);
                    }
                    *docked_scene_texture = None;
                    editor.rt_debounce.reset();
                }
            }
            None
        }
    }

    /// Keep the egui-side registration of the Tile Paint swatch atlas in sync with the
    /// current selection. Called once per frame just before the editor UI is built so the
    /// inspector can draw real tile thumbnails via the stored [`egui::TextureId`].
    ///
    /// Registers the selected `Tilemap`'s atlas texture while the editor is open (so the
    /// swatch palette shows real thumbnails as soon as the Tile Paint section appears, even
    /// before paint mode is enabled), re-registers when the atlas path changes, and frees the
    /// registration (no egui texture leak) when the selection changes to a non-tilemap or the
    /// editor closes.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn register_paint_atlas_texture(&mut self) {
        use crate::app::editor::EditorMode;

        // The atlas path we want registered this frame (None = nothing should be registered).
        // Tied to "a tilemap is selected in an open editor" — the same condition that shows the
        // Tile Paint section — not to paint mode, so the palette is populated up front.
        let desired: Option<String> = if self.editor.mode != EditorMode::Off {
            self.editor
                .inspector_selected
                .and_then(|e| self.world.get::<crate::tilemap::Tilemap>(e))
                .map(|tm| tm.atlas.texture.clone())
        } else {
            None
        };

        let current = self
            .editor
            .paint_atlas_tex
            .as_ref()
            .map(|(p, _)| p.as_str());
        if current == desired.as_deref() {
            return; // already in the right state (including both None)
        }

        // Free the stale registration.
        if let (Some(er), Some((_, old_id))) = (
            self.render.egui_renderer.as_mut(),
            self.editor.paint_atlas_tex.take(),
        ) {
            er.free_texture(&old_id);
        }

        // Register the new atlas texture, if it has been uploaded to the GPU yet.
        if let Some(path) = desired {
            let RenderState {
                egui_renderer,
                sprite_renderer,
                ..
            } = &mut self.render;
            if let (Some(er), Some(gpu), Some(sr)) = (
                egui_renderer.as_mut(),
                self.gpu.as_ref(),
                sprite_renderer.as_ref(),
            ) {
                if let Some(view) = sr.texture_view(&path) {
                    let id =
                        er.register_native_texture(&gpu.device, view, wgpu::FilterMode::Nearest);
                    self.editor.paint_atlas_tex = Some((path, id));
                }
            }
        }
    }
}
