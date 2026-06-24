//! Real multi-pass bloom renderer (mip-chain / "dual filter").
//!
//! Unlike the cheap inline 4-tap threshold blur in `post_process.wgsl`, this produces a genuine
//! soft glow. A **bright-pass** extracts highlights into a half-resolution mip; a **downsample
//! chain** (13-tap filter) then builds a mip pyramid, halving the resolution each level, and an
//! **upsample chain** (3x3 tent, additively blended) accumulates the levels back up — the
//! physically-based bloom from Jimenez's "Next Generation Post Processing in CoD: Advanced
//! Warfare". The accumulated glow (mip0) is **additively composited** back onto the scene
//! intermediate, all *before* the post-process pass runs (which then skips its inline bloom; see
//! [`PostProcessConfig::bloom`]).
//!
//! Why the pyramid over the old fixed-half-res separable Gaussian: a downsample/upsample pyramid
//! reaches a screen-wide, smooth, energy-preserving spread in a few passes, where the separable
//! blur's reach was bounded by `kernel x iterations` and looked boxy when pushed. `bloom_iterations`
//! now selects the **pyramid depth** (number of mip levels) — both control glow width, so the
//! public API is unchanged.
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

/// Upper bound on `bloom_iterations` — the mip-chain depth (number of downsample levels). The chain
/// also stops early once a level would shrink past a couple of pixels.
pub(crate) const MAX_BLOOM_ITERATIONS: u32 = 8;

/// Tent-filter radius (in source texels) for the upsample passes. 1.0 = sample the immediate
/// neighbours of the smaller mip; combined with bilinear magnification this gives a smooth spread.
const UPSAMPLE_RADIUS: f32 = 1.0;

// Uniform shared by all bloom stages (32 bytes — a 16B multiple). Each stage reads only the fields
// it needs (threshold / intensity / radius / texel). `texel` (a vec2, 8-byte aligned) sits at
// offset 16 so the struct stays std140-clean — never pad a uniform with a vecN (16-byte aligned).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    radius: f32,
    _pad0: f32,
    texel: [f32; 2],
    _pad1: [f32; 2],
}

// One level of the bloom pyramid (half the previous level's resolution). The view borrows the
// texture, so the texture is retained alongside it.
struct Mip {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

/// Captures scene highlights, blurs them through a downsample/upsample mip pyramid, and additively
/// composites the glow back onto the scene intermediate. See the module docs.
pub(crate) struct BloomRenderer {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,

    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,

    prefilter_pipeline: wgpu::RenderPipeline,
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline, // additive blend
    composite_pipeline: wgpu::RenderPipeline, // additive blend

    // The pyramid. mips[0] is half the scene resolution; each subsequent level halves again.
    mips: Vec<Mip>,

    // Static (bloom-owned-input) bind groups, indexed by their *source* step:
    //   downsample_bgs[k] reads mip[k]   → writes mip[k+1]   (k in 0..mips.len()-1)
    //   upsample_bgs[k]   reads mip[k+1] → writes mip[k]     (k in 0..mips.len()-1)
    // The uniforms they reference (per-level texel/radius) are baked at construction and kept alive
    // by the bind groups.
    downsample_bgs: Vec<wgpu::BindGroup>,
    upsample_bgs: Vec<wgpu::BindGroup>,
    composite_bg: wgpu::BindGroup, // reads mip[0] → scene

    // Only the prefilter (threshold + the constant scene texel) and composite (intensity) uniforms
    // vary per frame, so only those are retained.
    prefilter_ub: wgpu::Buffer,
    composite_ub: wgpu::Buffer,
}

impl BloomRenderer {
    /// Build the bloom renderer for a `width`×`height` scene intermediate of `format`. The pyramid
    /// starts at half that resolution and halves each level (min 1), up to [`MAX_BLOOM_ITERATIONS`].
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
        let downsample_pipeline = build("fs_downsample", wgpu::BlendState::REPLACE);
        // Additive: color += src; keep the destination alpha untouched. Shared by the upsample
        // accumulation (onto an already-downsampled mip) and the final composite (onto the scene).
        let additive = wgpu::BlendState {
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
        };
        let upsample_pipeline = build("fs_upsample", additive);
        let composite_pipeline = build("fs_composite", additive);

        // Pyramid dimensions: half the scene res, halving each level, stopping before a dimension
        // degenerates (≤2 px) or after MAX_BLOOM_ITERATIONS levels.
        let mut dims: Vec<(u32, u32)> = Vec::new();
        let mut mw = (width / 2).max(1);
        let mut mh = (height / 2).max(1);
        for _ in 0..MAX_BLOOM_ITERATIONS {
            dims.push((mw, mh));
            if mw <= 2 || mh <= 2 {
                break;
            }
            mw = (mw / 2).max(1);
            mh = (mh / 2).max(1);
        }

        let mips: Vec<Mip> = dims
            .iter()
            .map(|&(w, h)| {
                let (tex, view) = create_texture(device, "bloom mip", w, h, format);
                Mip {
                    _texture: tex,
                    view,
                }
            })
            .collect();

        let make_ub = |label: &str, uni: BloomUniforms| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::bytes_of(&uni),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            })
        };
        let make_bg = |label: &str, view: &wgpu::TextureView, ub: &wgpu::Buffer| {
            make_bind_group(device, &bind_group_layout, view, &sampler, ub, label)
        };
        let texel_of = |(w, h): (u32, u32)| [1.0 / w as f32, 1.0 / h as f32];

        // Downsample step k reads mip[k] (sampled at mip[k]'s texel) → writes mip[k+1].
        // Upsample step k reads mip[k+1] (at mip[k+1]'s texel, scaled by radius) → writes mip[k].
        let mut downsample_bgs = Vec::with_capacity(mips.len().saturating_sub(1));
        let mut upsample_bgs = Vec::with_capacity(mips.len().saturating_sub(1));
        for k in 0..mips.len().saturating_sub(1) {
            let ds_ub = make_ub(
                "bloom downsample ub",
                BloomUniforms {
                    texel: texel_of(dims[k]),
                    ..BloomUniforms::zeroed()
                },
            );
            downsample_bgs.push(make_bg("bloom downsample bg", &mips[k].view, &ds_ub));

            let us_ub = make_ub(
                "bloom upsample ub",
                BloomUniforms {
                    radius: UPSAMPLE_RADIUS,
                    texel: texel_of(dims[k + 1]),
                    ..BloomUniforms::zeroed()
                },
            );
            upsample_bgs.push(make_bg("bloom upsample bg", &mips[k + 1].view, &us_ub));
        }

        // Prefilter samples the full-res scene (texel = 1/scene-res) into mip0; threshold is
        // per-frame. Composite reads the accumulated mip0; intensity is per-frame.
        let prefilter_ub = make_ub(
            "bloom prefilter ub",
            BloomUniforms {
                texel: [1.0 / width as f32, 1.0 / height as f32],
                ..BloomUniforms::zeroed()
            },
        );
        let composite_ub = make_ub("bloom composite ub", BloomUniforms::zeroed());
        let composite_bg = make_bg("bloom composite bg", &mips[0].view, &composite_ub);

        Self {
            width,
            height,
            format,
            sampler,
            bind_group_layout,
            prefilter_pipeline,
            downsample_pipeline,
            upsample_pipeline,
            composite_pipeline,
            mips,
            downsample_bgs,
            upsample_bgs,
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

    /// Recreate the pyramid at a new window size.
    pub(crate) fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        *self = Self::new(device, width, height, self.format);
    }

    /// Refresh the per-frame uniforms (threshold/intensity) from the config.
    pub(crate) fn update(&self, queue: &wgpu::Queue, config: &PostProcessConfig) {
        // Prefilter carries threshold + the constant scene texel; rewrite the whole struct so the
        // texel survives. Composite carries only intensity.
        let prefilter = BloomUniforms {
            threshold: config.bloom_threshold,
            texel: [1.0 / self.width as f32, 1.0 / self.height as f32],
            ..BloomUniforms::zeroed()
        };
        queue.write_buffer(&self.prefilter_ub, 0, bytemuck::bytes_of(&prefilter));

        let composite = BloomUniforms {
            intensity: config.bloom_intensity,
            ..BloomUniforms::zeroed()
        };
        queue.write_buffer(&self.composite_ub, 0, bytemuck::bytes_of(&composite));
    }

    /// Run the bloom passes, additively compositing the glow onto `scene_view` (the scene
    /// intermediate). `scene_view` must match this renderer's `format`. `iterations` selects the
    /// pyramid depth (clamped to the available levels, ≤ [`MAX_BLOOM_ITERATIONS`]).
    pub(crate) fn run(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        iterations: u32,
    ) {
        // At least mip0; never more levels than the pyramid actually has.
        let n = (iterations.max(1) as usize).min(self.mips.len());

        // Bright-pass: scene → mip0. The input is the externally-owned scene view, so this bind
        // group is built per frame (cheap; the rest reference bloom-owned textures and are cached).
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
            &self.mips[0].view,
            &self.prefilter_pipeline,
            &prefilter_bg,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        );

        // Downsample chain: mip[k] → mip[k+1], REPLACE (each level fully overwritten this frame).
        for k in 0..n.saturating_sub(1) {
            draw_pass(
                encoder,
                "bloom downsample",
                &self.mips[k + 1].view,
                &self.downsample_pipeline,
                &self.downsample_bgs[k],
                wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            );
        }

        // Upsample chain: mip[k+1] → mip[k], additive onto the freshly-downsampled mip[k] (Load).
        for k in (0..n.saturating_sub(1)).rev() {
            draw_pass(
                encoder,
                "bloom upsample",
                &self.mips[k].view,
                &self.upsample_pipeline,
                &self.upsample_bgs[k],
                wgpu::LoadOp::Load,
            );
        }

        // Additive composite: mip0 → scene_view (Load preserves the scene; blend adds the glow).
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

// A fullscreen-triangle draw into `target` with `pipeline` + `bind_group`. `load` selects whether
// the target is cleared first (downsample/prefilter, REPLACE) or preserved (upsample, additive).
fn draw_pass(
    encoder: &mut wgpu::CommandEncoder,
    label: &str,
    target: &wgpu::TextureView,
    pipeline: &wgpu::RenderPipeline,
    bind_group: &wgpu::BindGroup,
    load: wgpu::LoadOp<wgpu::Color>,
) {
    let mut pass = super::common::begin_color_pass(encoder, label, target, load);
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.draw(0..3, 0..1);
}
