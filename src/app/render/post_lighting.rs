use super::super::App;
use crate::app::render_state::RenderState;
#[cfg(not(target_arch = "wasm32"))]
use crate::ecs::World;
use crate::renderer::bloom::BloomRenderer;
use crate::renderer::PostProcessRenderer;

impl App {
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn ensure_intermediate_texture(
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

    /// Lazily create / resize the post-process renderer for the current surface.
    /// Extracted from `render()` (pre-frame setup; no encoder/submit involved).
    /// `out_fmt` is the final display/swapchain format; `inter_fmt` is the scene intermediate format
    /// (== `out_fmt` normally, `Rgba16Float` when `PostProcessConfig::hdr` is on).
    pub(in crate::app) fn setup_post_renderer(
        render: &mut RenderState,
        device: &wgpu::Device,
        w: u32,
        h: u32,
        out_fmt: wgpu::TextureFormat,
        inter_fmt: wgpu::TextureFormat,
    ) {
        match &mut render.post_renderer {
            None => {
                render.post_renderer =
                    Some(PostProcessRenderer::new(device, w, h, out_fmt, inter_fmt));
            }
            Some(pr) if pr.output_format() != out_fmt || pr.intermediate_format() != inter_fmt => {
                pr.reconfigure(device, w, h, out_fmt, inter_fmt);
            }
            Some(pr) if pr.width != w || pr.height != h => {
                pr.resize(device, w, h);
            }
            _ => {}
        }
    }

    /// Lazily create / resize / reconfigure the bloom renderer for the scene intermediate.
    /// Called only when post-process is enabled and `PostProcessConfig::bloom` is on. `inter_fmt`
    /// is the scene intermediate format (== the post intermediate: `Rgba16Float` under HDR, else
    /// the surface format) so the bloom pipelines match. Mirrors `setup_post_renderer` /
    /// `setup_lighting`: one target per frame, so a format change rebuilds the renderer (there is
    /// no per-target-format pipeline cache, unlike the sprite/material/UI/GPU-particle passes).
    pub(in crate::app) fn setup_bloom_renderer(
        render: &mut RenderState,
        device: &wgpu::Device,
        w: u32,
        h: u32,
        inter_fmt: wgpu::TextureFormat,
    ) {
        match &mut render.bloom_renderer {
            None => {
                render.bloom_renderer = Some(BloomRenderer::new(device, w, h, inter_fmt));
            }
            Some(br) if br.format() != inter_fmt => br.reconfigure(device, w, h, inter_fmt),
            Some(br) if br.width() != w || br.height() != h => br.resize(device, w, h),
            _ => {}
        }
    }

    /// Lazily create / resize / disable the lighting renderer + its scene-intermediate
    /// texture, returning whether lighting is active this frame. Extracted from `render()`
    /// (pre-frame setup; no encoder/submit involved). Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::app) fn setup_lighting(
        render: &mut RenderState,
        world: &World,
        device: &wgpu::Device,
        w: u32,
        h: u32,
        fmt: wgpu::TextureFormat,
        use_post: bool,
    ) -> bool {
        let has_lighting = world.resource::<crate::resources::AmbientLight>().is_some();
        if has_lighting {
            match &mut render.lighting_renderer {
                None => {
                    render.lighting_renderer = Some(
                        crate::renderer::lighting::LightingRenderer::new(device, w, h, fmt),
                    );
                }
                Some(lr) if lr.format() != fmt => {
                    lr.reconfigure(device, w, h, fmt);
                }
                Some(lr) if lr.width != w || lr.height != h => {
                    lr.resize(device, w, h);
                }
                _ => {}
            }
            // Create / resize the scene intermediate texture (only needed when post_renderer is absent)
            if !use_post {
                Self::ensure_intermediate_texture(
                    &mut render.scene_texture_for_lighting,
                    device,
                    "scene_for_lighting",
                    w,
                    h,
                    fmt,
                );
                render.post_texture_for_lighting = None;
            } else {
                render.scene_texture_for_lighting = None;
                Self::ensure_intermediate_texture(
                    &mut render.post_texture_for_lighting,
                    device,
                    "post_for_lighting",
                    w,
                    h,
                    fmt,
                );
            }
        } else {
            render.lighting_renderer = None;
            render.scene_texture_for_lighting = None;
            render.post_texture_for_lighting = None;
        }
        has_lighting
    }
}
