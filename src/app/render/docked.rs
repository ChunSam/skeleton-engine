use super::super::App;
use crate::app::egui_pass::egui_render_pass;
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
            (render.egui_renderer.take(), render.egui_output.take())
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
            egui_render_pass(&er, &mut egui_enc, &paint_jobs, &screen_desc, &final_view);
            gpu.queue.submit(std::iter::once(egui_enc.finish()));
            for id in &textures_delta.free {
                er.free_texture(id);
            }
            render.egui_renderer = Some(er);
        }
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
