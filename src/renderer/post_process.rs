use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// Tone-mapping operator applied by the post-process pass: maps an (optionally over-bright,
/// HDR) scene into the displayable `[0, 1]` range. Only meaningful when [`PostProcessConfig::hdr`]
/// is on (otherwise the scene was already clamped at 8-bit store time). `#[non_exhaustive]` so
/// more operators can be added without a breaking change.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Tonemap {
    /// No tone-mapping (the raw colour is clamped). Default — byte-identical to the pre-tonemap pass.
    #[default]
    None,
    /// Reinhard `c / (1 + c)` — cheap, desaturates highlights.
    Reinhard,
    /// ACES filmic approximation (Narkowicz) — filmic highlight roll-off, the usual default for HDR.
    AcesFilmic,
}

impl Tonemap {
    /// Stable `u32` encoding sent to the shader uniform.
    fn as_u32(self) -> u32 {
        match self {
            Tonemap::None => 0,
            Tonemap::Reinhard => 1,
            Tonemap::AcesFilmic => 2,
        }
    }
}

/// Post-processing configuration. Insert as a World resource to activate.
///
/// ```rust,no_run
/// # use engine::{App, PostProcessConfig};
/// # let mut app = App::new();
/// app.world.insert_resource(PostProcessConfig {
///     enabled: true,
///     vignette_strength: 0.5,
///     ..Default::default()
/// });
/// ```
///
/// **HDR tone-mapping:** set `hdr: true` to render the scene into an `Rgba16Float` intermediate
/// (preserving colour values `> 1.0` instead of clamping them at store time), then pick a
/// [`tonemap`](Self::tonemap) operator and an [`exposure`](Self::exposure) to map it back to the
/// display. The HDR intermediate is currently fed by the **sprite, UI-primitive, and render-plugin**
/// passes (GPU particles and shader-materials in an HDR post scene are not yet supported). When
/// `hdr` is false the pass behaves exactly as before.
#[derive(Clone, Copy, Debug)]
pub struct PostProcessConfig {
    /// When false the post-process pass is skipped entirely.
    pub enabled: bool,
    /// Vignette strength (0.0 = none, 1.0 = edges fully darkened).
    pub vignette_strength: f32,
    /// Radius at which vignetting begins (0.0~1.0, relative to screen center).
    pub vignette_radius: f32,
    /// Chromatic aberration strength (0.0 = none; ~0.005 is a subtle amount).
    pub chroma_offset: f32,
    /// Luminance threshold above which bloom is applied (0.0~1.0; 0.7~0.9 recommended).
    pub bloom_threshold: f32,
    /// Bloom brightness multiplier.
    pub bloom_intensity: f32,
    /// Brightness offset (-1.0~1.0; 0.0 = original).
    pub brightness: f32,
    /// Contrast multiplier (0.0~3.0; 1.0 = original).
    pub contrast: f32,
    /// Saturation multiplier (0.0~3.0; 1.0 = original, 0.0 = grayscale).
    pub saturation: f32,
    /// Render the scene into an `Rgba16Float` HDR intermediate so `> 1.0` colours survive to be
    /// tone-mapped. `false` (default) keeps the surface-format intermediate = original behaviour.
    pub hdr: bool,
    /// Exposure multiplier applied to the scene colour before tone-mapping (`1.0` = none).
    pub exposure: f32,
    /// Tone-mapping operator (see [`Tonemap`]). Only meaningful with `hdr: true`.
    pub tonemap: Tonemap,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            vignette_strength: 0.45,
            vignette_radius: 0.65,
            chroma_offset: 0.003,
            bloom_threshold: 0.75,
            bloom_intensity: 0.4,
            brightness: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            hdr: false,
            exposure: 1.0,
            tonemap: Tonemap::None,
        }
    }
}

// Uniform struct sent to the GPU.
// Layout (std140-compatible, 12 × 4 bytes = 48 bytes, already a 16B multiple):
//   [0]  vignette_strength
//   [4]  vignette_radius
//   [8]  chroma_offset
//   [12] bloom_threshold
//   [16] bloom_intensity
//   [20] brightness
//   [24] contrast
//   [28] saturation
//   [32] texel_size.x  (1.0 / target_width)
//   [36] texel_size.y  (1.0 / target_height)
//   [40] exposure      (scene colour multiply before tone-mapping; was _pad0.x)
//   [44] tonemap       (u32 operator id; was _pad0.y)
//
// The trailing 8 bytes were padding (`pad0: vec2`); they now carry `exposure` + `tonemap`, so the
// struct size is unchanged (48 B) — no buffer/layout change.
//
// `texel_size` is pre-computed here so the shader avoids per-fragment
// `textureDimensions` calls (which emit a driver query on some backends).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PostProcessUniforms {
    vignette_strength: f32,
    vignette_radius: f32,
    chroma_offset: f32,
    bloom_threshold: f32,
    bloom_intensity: f32,
    brightness: f32,
    contrast: f32,
    saturation: f32,
    /// Pre-computed texel size: (1/width, 1/height). Populated by `update_uniforms`.
    texel_size: [f32; 2],
    /// Exposure multiplier applied before tone-mapping (1.0 = none).
    exposure: f32,
    /// Tone-map operator id (`Tonemap::as_u32`).
    tonemap: u32,
}

/// Populates `PostProcessUniforms` from a config + the current render-target dimensions.
///
/// `width`/`height` are the intermediate texture size (set by `PostProcessRenderer::new`
/// or `PostProcessRenderer::resize`). They are available on `PostProcessRenderer` as
/// `self.width` / `self.height`, so `update_uniforms` can read them directly.
fn make_uniforms(c: &PostProcessConfig, width: u32, height: u32) -> PostProcessUniforms {
    let (tw, th) = if width > 0 && height > 0 {
        (1.0 / width as f32, 1.0 / height as f32)
    } else {
        (0.0, 0.0)
    };
    PostProcessUniforms {
        vignette_strength: c.vignette_strength,
        vignette_radius: c.vignette_radius,
        chroma_offset: c.chroma_offset,
        bloom_threshold: c.bloom_threshold,
        bloom_intensity: c.bloom_intensity,
        brightness: c.brightness,
        contrast: c.contrast,
        saturation: c.saturation,
        texel_size: [tw, th],
        exposure: c.exposure,
        tonemap: c.tonemap.as_u32(),
    }
}

/// Renderer that captures the scene into an intermediate texture, applies post-processing,
/// and outputs the result to the final swapchain.
pub struct PostProcessRenderer {
    /// Texture view of the intermediate render target where the scene is drawn first.
    pub(crate) target_view: wgpu::TextureView,
    target_texture: wgpu::Texture,
    /// Current resolution of the intermediate texture.
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Pixel format of the **final output** (the swapchain / display target the pass writes to).
    output_format: wgpu::TextureFormat,
    /// Pixel format of the **intermediate** scene texture the scene is drawn into (== `output_format`
    /// unless HDR is on, in which case `Rgba16Float`). The scene passes must render with a pipeline
    /// matching this.
    intermediate_format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl PostProcessRenderer {
    /// Creates the post-process renderer. `output_format` is the final display/swapchain format the
    /// pass writes to; `intermediate_format` is the scene texture the scene is drawn into (pass
    /// `Rgba16Float` for an HDR intermediate, else the same as `output_format`).
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
        intermediate_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("post_process shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/post_process.wgsl").into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("post_process sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("post_process uniforms"),
            contents: bytemuck::bytes_of(&PostProcessUniforms::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("post_process bgl"),
            entries: &[
                // binding 0: scene texture
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // binding 1: sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 2: uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("post_process pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("post_process pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: output_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let (target_texture, target_view) =
            Self::create_target(device, width, height, intermediate_format);

        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &target_view,
            &sampler,
            &uniform_buffer,
        );

        Self {
            target_texture,
            target_view,
            width,
            height,
            output_format,
            intermediate_format,
            pipeline,
            sampler,
            bind_group_layout,
            bind_group,
            uniform_buffer,
        }
    }

    /// The final output (display/swapchain) format the pass writes to.
    pub(crate) fn output_format(&self) -> wgpu::TextureFormat {
        self.output_format
    }

    /// The intermediate scene-texture format (the format the scene passes must render with). Equals
    /// `output_format` unless HDR is enabled (`Rgba16Float`).
    pub(crate) fn intermediate_format(&self) -> wgpu::TextureFormat {
        self.intermediate_format
    }

    pub(crate) fn reconfigure(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        output_format: wgpu::TextureFormat,
        intermediate_format: wgpu::TextureFormat,
    ) {
        *self = Self::new(device, width, height, output_format, intermediate_format);
    }

    /// Recreates the intermediate texture when the window is resized.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let (tex, view) = Self::create_target(device, width, height, self.intermediate_format);
        self.target_texture = tex;
        self.target_view = view;
        self.width = width;
        self.height = height;
        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            &self.target_view,
            &self.sampler,
            &self.uniform_buffer,
        );
    }

    /// Updates the uniform buffer with the current configuration and render-target dimensions.
    ///
    /// Populates `texel_size` from `self.width` / `self.height` so the shader can use
    /// `u.texel_size` instead of a per-fragment `textureDimensions(scene_tex)` call.
    pub fn update_uniforms(&self, queue: &wgpu::Queue, config: &PostProcessConfig) {
        let uni = make_uniforms(config, self.width, self.height);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uni));
    }

    /// Runs the post-process pass from the intermediate texture into the final swapchain view.
    pub fn run_pass(&self, enc: &mut wgpu::CommandEncoder, final_view: &wgpu::TextureView) {
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("post_process pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: final_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        // Fullscreen triangle: 3 vertices, no vertex buffer
        pass.draw(0..3, 0..1);
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn create_target(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("post_process target"),
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

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        uniform_buffer: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post_process bind group"),
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
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        })
    }
}
