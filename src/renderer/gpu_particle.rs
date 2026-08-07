use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::ecs::World;
use crate::renderer::CameraUniform;

/// Compute-shader workgroup size. Single source of truth: it drives the dispatch
/// `div_ceil` below AND is substituted into the WGSL `@workgroup_size(...)` at shader-load
/// time, so the two can never silently drift (an over-dispatch wastes threads, an
/// under-dispatch skips particles).
const COMPUTE_WORKGROUP_SIZE: u32 = 64;

// ─── GPU Particle Data (80 bytes, 16 B aligned) ───────────────────────────────
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug)]
pub struct GpuParticle {
    pub pos: [f32; 2],
    pub vel: [f32; 2],
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub _pad: f32,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    /// Per-particle constant acceleration (pixels/s²); integrated each step by the
    /// compute shader. `[0.0, 0.0]` = constant-velocity (byte-identical to before).
    pub gravity: [f32; 2],
    /// Padding to keep the struct 16-byte aligned (array stride must be a multiple of 16).
    pub _pad2: [f32; 2],
}

// ─── Compute Uniforms ────────────────────────────────────────────────────────
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ComputeUniforms {
    dt: f32,
    _pad: [f32; 3],
}

// CameraUniform is defined in `crate::renderer` (shared with sprite/geometry).

/// GPU compute-shader based particle renderer (native only).
///
/// Managed internally by `App`. Users only need to attach a `GpuParticleEmitter` component.
pub struct GpuParticleRenderer {
    // ── Compute pipeline ───────────────────────────────────────────────────
    compute_pipeline: wgpu::ComputePipeline,
    compute_bind_group: wgpu::BindGroup,
    compute_uniform_buf: wgpu::Buffer,
    // ── Particle buffer (STORAGE | VERTEX dual-use) ───────────────────────
    particle_buf: wgpu::Buffer,
    particle_capacity: u32,
    // ── Render pipeline ────────────────────────────────────────────────────
    render_pipeline: wgpu::RenderPipeline,
    /// The surface format the base `render_pipeline` targets. A scene target with a different
    /// format (e.g. the `Rgba16Float` HDR post-process intermediate) gets a matching pipeline
    /// from `extra_render_pipelines`.
    base_format: wgpu::TextureFormat,
    /// Reused to lazily build a render pipeline per non-surface target format.
    render_pipeline_layout: wgpu::PipelineLayout,
    /// Lazily-built render pipelines keyed by target format (the HDR / offscreen case); the surface
    /// format always uses `render_pipeline` directly.
    extra_render_pipelines: std::collections::HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>,
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    particle_bind_group: wgpu::BindGroup,
    /// Write cursor into the ring `particle_buf`, **persisted across frames**.
    ///
    /// It lived as a frame-local `let mut frame_cursor = 0u32` in the render stage, so every
    /// frame restarted the ring at slot 0 and overwrote the particles the previous frame had
    /// just spawned — a full-capacity buffer only ever held one frame's worth of emission, and
    /// anything with a lifetime longer than a frame was destroyed before it could be drawn.
    /// Owning it here keeps it advancing while still giving every emitter within a frame a
    /// disjoint slot (`collect_new_particles` shares one cursor across emitters).
    frame_cursor: u32,
}

/// Builds the GPU-particle render pipeline for a given color-target `format`. Shared by
/// [`GpuParticleRenderer::new`] (the surface format) and
/// [`GpuParticleRenderer::ensure_render_pipeline`] (a non-surface render-target format, e.g. the
/// `Rgba16Float` HDR post-process intermediate), so the pipeline descriptor (alpha blend, triangle
/// list, no depth) lives in exactly one place.
fn build_particle_render_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("gpu particle render pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    })
}

impl GpuParticleRenderer {
    /// Creates a renderer capable of processing `capacity` particles simultaneously.
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, capacity: u32) -> Self {
        // ── Compute shader ───────────────────────────────────────────────
        let compute_src = include_str!("shaders/gpu_particle_compute.wgsl").replace(
            "@workgroup_size(64)",
            &format!("@workgroup_size({COMPUTE_WORKGROUP_SIZE})"),
        );
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu particle compute"),
            source: wgpu::ShaderSource::Wgsl(compute_src.into()),
        });

        // ── Particle buffer ───────────────────────────────────────────────
        let particle_size = (capacity as usize) * std::mem::size_of::<GpuParticle>();
        let particle_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu particle buf"),
            size: particle_size as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // ── Compute uniforms ─────────────────────────────────────────────
        let compute_uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gpu particle compute uniforms"),
            contents: bytemuck::bytes_of(&ComputeUniforms {
                dt: 0.0,
                _pad: [0.0; 3],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // ── Compute bind group layout ─────────────────────────────────────
        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu particle compute bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu particle compute bg"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: compute_uniform_buf.as_entire_binding(),
                },
            ],
        });

        // ── Compute pipeline ──────────────────────────────────────────────
        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gpu particle compute layout"),
                bind_group_layouts: &[Some(&compute_bgl)],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("gpu particle compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // ── Render shader ─────────────────────────────────────────────────
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu particle render"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/gpu_particle_render.wgsl").into(),
            ),
        });

        // ── Camera uniform (group 0) ──────────────────────────────────────
        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu particle camera buf"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu particle camera bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu particle camera bg"),
            layout: &camera_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        // ── Particle buffer bind group (group 1) ──────────────────────────
        let particle_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpu particle render particle bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let particle_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpu particle render particle bg"),
            layout: &particle_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: particle_buf.as_entire_binding(),
            }],
        });

        // ── Render pipeline ───────────────────────────────────────────────
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("gpu particle render layout"),
                bind_group_layouts: &[Some(&camera_bgl), Some(&particle_layout)],
                immediate_size: 0,
            });

        let render_pipeline = build_particle_render_pipeline(
            device,
            &render_shader,
            &render_pipeline_layout,
            surface_format,
        );

        Self {
            compute_pipeline,
            compute_bind_group,
            compute_uniform_buf,
            particle_buf,
            particle_capacity: capacity,
            render_pipeline,
            base_format: surface_format,
            render_pipeline_layout,
            extra_render_pipelines: std::collections::HashMap::new(),
            camera_buf,
            camera_bind_group,
            particle_bind_group,
            frame_cursor: 0,
        }
    }

    /// Ensures a render pipeline matching `format` exists — builds + caches one for a non-surface
    /// render-target format (e.g. the `Rgba16Float` HDR post-process intermediate) on first use; a
    /// no-op for the base (surface) format and on cache hits. Recompiles the particle render shader
    /// for the new pipeline, paid once per distinct format (never per frame). Mirrors
    /// `SpriteRenderer::ensure_sprite_pipeline`.
    pub fn ensure_render_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if format == self.base_format || self.extra_render_pipelines.contains_key(&format) {
            return;
        }
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpu particle render (rt format)"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shaders/gpu_particle_render.wgsl").into(),
            ),
        });
        let pipeline =
            build_particle_render_pipeline(device, &shader, &self.render_pipeline_layout, format);
        self.extra_render_pipelines.insert(format, pipeline);
    }

    /// The render pipeline matching `format`: the base pipeline for the surface format, else the
    /// cached extra ([`ensure_render_pipeline`](Self::ensure_render_pipeline) must have built it;
    /// falls back to the base pipeline if somehow missing, to never panic mid-frame).
    fn render_pipeline_for(&self, format: wgpu::TextureFormat) -> &wgpu::RenderPipeline {
        if format == self.base_format {
            &self.render_pipeline
        } else {
            self.extra_render_pipelines
                .get(&format)
                .unwrap_or(&self.render_pipeline)
        }
    }

    /// Uploads new particle data to the GPU buffer (overwrites the emission slot).
    pub fn upload_particles(&self, queue: &wgpu::Queue, particles: &[GpuParticle], offset: u32) {
        if particles.is_empty() {
            return;
        }
        let byte_offset = offset as u64 * std::mem::size_of::<GpuParticle>() as u64;
        let byte_data = bytemuck::cast_slice(particles);
        if byte_offset + byte_data.len() as u64
            <= self.particle_capacity as u64 * std::mem::size_of::<GpuParticle>() as u64
        {
            queue.write_buffer(&self.particle_buf, byte_offset, byte_data);
        }
    }

    /// Updates particle positions and lifetimes via the compute shader.
    pub fn dispatch_compute(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queue: &wgpu::Queue,
        dt: f32,
    ) {
        queue.write_buffer(
            &self.compute_uniform_buf,
            0,
            bytemuck::bytes_of(&ComputeUniforms { dt, _pad: [0.0; 3] }),
        );
        let workgroups = self.particle_capacity.div_ceil(COMPUTE_WORKGROUP_SIZE);
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("gpu particle compute pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.compute_pipeline);
        pass.set_bind_group(0, &self.compute_bind_group, &[]);
        pass.dispatch_workgroups(workgroups, 1, 1);
    }

    /// Renders particles to the screen.
    // Args mirror the wgpu pass inputs (queue/view/encoder/world/viewport) plus the letterbox
    // clip scale; bundling them into a struct would obscure an internal renderer call.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        world: &World,
        width: u32,
        height: u32,
        target_format: wgpu::TextureFormat,
        clip_scale: glam::Vec2,
    ) {
        let fallback = Camera::default();
        let camera = world.resource::<Camera>().unwrap_or(&fallback);
        let view_proj = crate::camera::apply_letterbox(
            clip_scale,
            camera.view_proj(width as f32, height as f32),
        );
        queue.write_buffer(
            &self.camera_buf,
            0,
            bytemuck::bytes_of(&CameraUniform {
                view_proj: view_proj.to_cols_array_2d(),
            }),
        );

        let vertex_count = self.particle_capacity * 6;
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gpu particle render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });
        pass.set_pipeline(self.render_pipeline_for(target_format));
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_bind_group(1, &self.particle_bind_group, &[]);
        pass.draw(0..vertex_count, 0..1);
    }

    pub fn capacity(&self) -> u32 {
        self.particle_capacity
    }

    /// The persistent ring write cursor. See [`GpuParticleRenderer::frame_cursor`] on the field
    /// for why this is not per-frame state.
    pub fn frame_cursor(&self) -> u32 {
        self.frame_cursor
    }

    /// Stores the cursor back after a frame's emission has consumed some slots.
    pub fn set_frame_cursor(&mut self, cursor: u32) {
        self.frame_cursor = cursor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The compute + render shaders index a tightly-packed `GpuParticle` buffer by a stride
    // baked into the WGSL. If a field change moves the size off 80 bytes, that stride drifts
    // and rendering corrupts silently — this assert makes it a build failure instead.
    #[test]
    fn gpu_particle_size_is_stable() {
        assert_eq!(std::mem::size_of::<GpuParticle>(), 80);
    }
}
