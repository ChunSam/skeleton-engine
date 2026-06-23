//! Real multi-pass bloom renderer.
//!
//! Unlike the cheap inline 4-tap threshold blur in `post_process.wgsl`, this produces a genuine
//! soft glow: a **bright-pass** extracts highlights into a half-resolution texture, a **separable
//! Gaussian blur** is ping-ponged across two textures `bloom_iterations` times for a wide spread,
//! and the result is **additively composited** back onto the scene intermediate — all *before* the
//! post-process pass runs (which then skips its inline bloom; see [`PostProcessConfig::bloom`]).
//!
//! Activated by [`PostProcessConfig::bloom`] (requires `enabled: true`). The pass runs on the scene
//! intermediate texture, so its pipelines are built for that texture's format (`Rgba16Float` under
//! HDR, else the surface format). Like the post-process and lighting renderers — and *unlike* the
//! sprite/material/UI/GPU-particle per-target-format pipeline *cache* — there is exactly one target
//! per frame, so a format change just rebuilds the renderer (`reconfigure`), no `HashMap` cache.
//!
//! [`PostProcessConfig::bloom`]: crate::PostProcessConfig::bloom

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::post_process::PostProcessConfig;

/// Upper bound on `bloom_iterations` (each iteration = one horizontal + one vertical blur pass).
pub(crate) const MAX_BLOOM_ITERATIONS: u32 = 8;

// Uniform shared by all three bloom stages (32 bytes — a 16B multiple). Each stage reads only the
// fields it needs (threshold/intensity/texel/direction).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    texel: [f32; 2],
    direction: [f32; 2],
    _pad: [f32; 2],
}

/// Captures scene highlights, blurs them, and additively composites the glow back onto the scene
/// intermediate. See the module docs.
pub(crate) struct BloomRenderer {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,

    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,

    prefilter_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,

    // Half-resolution ping-pong textures. prefilter: scene → A; blur: A→B→A each iteration;
    // composite: A → scene. (Kept alive; the views below borrow them.)
    _tex_a: wgpu::Texture,
    _tex_b: wgpu::Texture,
    view_a: wgpu::TextureView,
    view_b: wgpu::TextureView,

    // Bind groups whose input texture is bloom-owned (built once).
    blur_h_bg: wgpu::BindGroup,    // input A, blur horizontal → B
    blur_v_bg: wgpu::BindGroup,    // input B, blur vertical   → A
    composite_bg: wgpu::BindGroup, // input A → scene

    // Only the prefilter (threshold) and composite (intensity) uniforms vary per frame, so only
    // those are retained. The blur uniforms (texel/direction) are baked at construction and kept
    // alive by their bind groups.
    prefilter_ub: wgpu::Buffer,
    composite_ub: wgpu::Buffer,
}

impl BloomRenderer {
    /// Build the bloom renderer for a `width`×`height` scene intermediate of `format`. The bloom
    /// working textures are half that resolution (min 1).
    pub(crate) fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/bloom.wgsl").into()),
        });

        let sampler = super::common::create_clamp_sampler(
            device,
            Some("bloom sampler"),
            wgpu::FilterMode::Linear,
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom bgl"),
            entries: &[
                super::common::filterable_texture_entry(0),
                super::common::filtering_sampler_entry(1),
                super::common::uniform_buffer_entry(2),
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bloom pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let build = |entry: &str, blend: wgpu::BlendState| {
            build_pipeline(device, &shader, &pipeline_layout, entry, format, blend)
        };
        let prefilter_pipeline = build("fs_prefilter", wgpu::BlendState::REPLACE);
        let blur_pipeline = build("fs_blur", wgpu::BlendState::REPLACE);
        // Additive: color += src; keep the destination alpha untouched.
        let composite_pipeline = build(
            "fs_composite",
            wgpu::BlendState {
                color: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::One,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
                alpha: wgpu::BlendComponent {
                    src_factor: wgpu::BlendFactor::Zero,
                    dst_factor: wgpu::BlendFactor::One,
                    operation: wgpu::BlendOperation::Add,
                },
            },
        );

        let bloom_w = (width / 2).max(1);
        let bloom_h = (height / 2).max(1);
        let (tex_a, view_a) = create_texture(device, "bloom tex a", bloom_w, bloom_h, format);
        let (tex_b, view_b) = create_texture(device, "bloom tex b", bloom_w, bloom_h, format);

        let make_ub = |label: &str, uni: BloomUniforms| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::bytes_of(&uni),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        };
        let texel = [1.0 / bloom_w as f32, 1.0 / bloom_h as f32];
        let prefilter_ub = make_ub("bloom prefilter ub", BloomUniforms::zeroed());
        let blur_h_ub = make_ub(
            "bloom blur h ub",
            BloomUniforms {
                texel,
                direction: [1.0, 0.0],
                ..BloomUniforms::zeroed()
            },
        );
        let blur_v_ub = make_ub(
            "bloom blur v ub",
            BloomUniforms {
                texel,
                direction: [0.0, 1.0],
                ..BloomUniforms::zeroed()
            },
        );
        let composite_ub = make_ub("bloom composite ub", BloomUniforms::zeroed());

        let make_bg = |label: &str, view: &wgpu::TextureView, ub: &wgpu::Buffer| {
            make_bind_group(device, &bind_group_layout, view, &sampler, ub, label)
        };
        let blur_h_bg = make_bg("bloom blur h bg", &view_a, &blur_h_ub);
        let blur_v_bg = make_bg("bloom blur v bg", &view_b, &blur_v_ub);
        let composite_bg = make_bg("bloom composite bg", &view_a, &composite_ub);

        Self {
            width,
            height,
            format,
            sampler,
            bind_group_layout,
            prefilter_pipeline,
            blur_pipeline,
            composite_pipeline,
            _tex_a: tex_a,
            _tex_b: tex_b,
            view_a,
            view_b,
            blur_h_bg,
            blur_v_bg,
            composite_bg,
            prefilter_ub,
            composite_ub,
        }
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }
    pub(crate) fn height(&self) -> u32 {
        self.height
    }
    pub(crate) fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Rebuild for a new format (e.g. HDR toggled on/off → `Rgba16Float` ↔ surface).
    pub(crate) fn reconfigure(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) {
        *self = Self::new(device, width, height, format);
    }

    /// Recreate the working textures at a new window size.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, width, height, self.format);
    }

    /// Refresh the per-frame uniforms (threshold/intensity) from the config.
    pub(crate) fn update(&self, queue: &wgpu::Queue, config: &PostProcessConfig) {
        // Only threshold (prefilter) and intensity (composite) vary per frame; texel/direction are
        // baked at construction.
        let mut prefilter = BloomUniforms::zeroed();
        prefilter.threshold = config.bloom_threshold;
        queue.write_buffer(&self.prefilter_ub, 0, bytemuck::bytes_of(&prefilter));

        let mut composite = BloomUniforms::zeroed();
        composite.intensity = config.bloom_intensity;
        queue.write_buffer(&self.composite_ub, 0, bytemuck::bytes_of(&composite));
    }

    /// Run the bloom passes, additively compositing the glow onto `scene_view` (the scene
    /// intermediate). `scene_view` must match this renderer's `format`. `iterations` is the number
    /// of horizontal+vertical blur passes (clamped to [`MAX_BLOOM_ITERATIONS`]).
    pub(crate) fn run(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        iterations: u32,
    ) {
        let iters = iterations.min(MAX_BLOOM_ITERATIONS);

        // Bright-pass: scene → A. The input is the externally-owned scene view, so this bind group
        // is built per frame (cheap; the others reference bloom-owned textures and are cached).
        let prefilter_bg = make_bind_group(
            device,
            &self.bind_group_layout,
            scene_view,
            &self.sampler,
            &self.prefilter_ub,
            "bloom prefilter bg",
        );
        draw_pass(
            encoder,
            "bloom prefilter",
            &self.view_a,
            &self.prefilter_pipeline,
            &prefilter_bg,
        );

        // Separable blur, ping-ponged: H (A→B) then V (B→A). After every iteration the result is
        // back in A, so the composite bind group (input A) is stable.
        for _ in 0..iters {
            draw_pass(
                encoder,
                "bloom blur h",
                &self.view_b,
                &self.blur_pipeline,
                &self.blur_h_bg,
            );
            draw_pass(
                encoder,
                "bloom blur v",
                &self.view_a,
                &self.blur_pipeline,
                &self.blur_v_bg,
            );
        }

        // Additive composite: A → scene_view (LoadOp::Load preserves the scene; blend adds the glow).
        let mut pass = super::common::begin_color_pass(
            encoder,
            "bloom composite",
            scene_view,
            wgpu::LoadOp::Load,
        );
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &self.composite_bg, &[]);
        pass.draw(0..3, 0..1);
    }
}

// ── Internal helpers ────────────────────────────────────────────────────────

fn build_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    fs_entry: &str,
    format: wgpu::TextureFormat,
    blend: wgpu::BlendState,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("bloom pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(blend),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

fn create_texture(
    device: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
) -> (wgpu::Texture, wgpu::TextureView) {
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
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}

fn make_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
    label: &str,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

// A clear-then-draw fullscreen pass into `target` with `pipeline` + `bind_group`.
fn draw_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
) {
    let mut pass = super::common::begin_color_pass(
        encoder,
        label,
        target,
        wgpu::LoadOp::Clear(wgpu::Color::BLACK),
    );
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
