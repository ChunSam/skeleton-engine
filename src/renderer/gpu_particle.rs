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
    /// Staging for [`upload_new_particles`](Self::upload_new_particles): one contiguous ring run,
    /// copied out of the `(slot, particle)` pairs so it can go up in a single `write_buffer`.
    /// Cleared and refilled per run; the allocation is reused across frames.
    upload_scratch: Vec<GpuParticle>,
    /// Upper bound, in seconds, on how much longer any already-spawned particle can live.
    ///
    /// Death happens on the GPU (the compute shader decrements each particle's `life` by `dt`), so
    /// the CPU cannot ask whether the buffer still holds anything. It can bound it: no particle
    /// outlives the longest `life` ever uploaded, so counting that value down by the same `dt` the
    /// shader uses errs only in the safe direction — it can say "maybe alive" too long, never too
    /// short. See [`has_live_particles`](Self::has_live_particles).
    alive_for: f32,
}

/// Length of the run of consecutive ring slots starting at index `start`.
///
/// `collect_new_particles` advances **one shared cursor** by one per particle, so a frame's whole
/// emission lands on consecutive slots and wraps at most once. Uploading run-by-run therefore
/// costs at most two `queue.write_buffer` calls for the frame, where uploading particle-by-particle
/// cost one per particle.
///
/// A wrap ends a run: the slot after `capacity - 1` is `0`, which is not `capacity`, so the
/// comparison below splits there without needing to know the capacity.
pub(crate) fn contiguous_run_len<T>(items: &[T], start: usize, slot: impl Fn(&T) -> u32) -> usize {
    let mut end = start + 1;
    while end < items.len() && slot(&items[end]) == slot(&items[end - 1]).wrapping_add(1) {
        end += 1;
    }
    end - start
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
            upload_scratch: Vec::new(),
            alive_for: 0.0,
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
        let capacity_bytes =
            self.particle_capacity as u64 * std::mem::size_of::<GpuParticle>() as u64;
        if byte_offset + byte_data.len() as u64 > capacity_bytes {
            // Dropping the write is the right thing — `queue.write_buffer` past the end is a
            // validation error, and the engine's own ring path never gets here. But dropping it
            // *silently* is not: this is `pub`, so a game writing its own emitter sees particles
            // that simply never appear, with nothing anywhere saying why.
            //
            // Not rate-limited, deliberately. A caller in this state is wrong every frame and
            // should be told every frame, which is how the text pass already treats a failed
            // render. The alternative needs interior mutability for a `&self` method, and buying
            // that to make a bug quieter is the wrong trade.
            log::warn!(
                "gpu particle upload out of range — {} particles at slot {offset} need {} bytes \
                 but the buffer holds {} ({} particles); dropping the write",
                particles.len(),
                byte_offset + byte_data.len() as u64,
                capacity_bytes,
                self.particle_capacity,
            );
            return;
        }
        queue.write_buffer(&self.particle_buf, byte_offset, byte_data);
    }

    /// Uploads a frame's freshly-spawned particles as `(ring slot, particle)` pairs — **one
    /// `write_buffer` per contiguous run** rather than one per particle.
    ///
    /// The pairs come out of `collect_new_particles` in cursor order, which makes their slots
    /// consecutive with at most one wrap, so a frame's emission is at most two range writes no
    /// matter how many particles it spawned. Also records the longest lifetime uploaded, which is
    /// what [`has_live_particles`](Self::has_live_particles) counts down.
    pub fn upload_new_particles(&mut self, queue: &wgpu::Queue, new: &[(u32, GpuParticle)]) {
        // `take` so the scratch can be filled and read while `&self` methods run against it.
        let mut scratch = std::mem::take(&mut self.upload_scratch);
        let mut i = 0;
        while i < new.len() {
            let len = contiguous_run_len(new, i, |(slot, _)| *slot);
            scratch.clear();
            scratch.extend(new[i..i + len].iter().map(|(_, p)| *p));
            self.upload_particles(queue, &scratch, new[i].0);
            i += len;
        }
        self.upload_scratch = scratch;

        let longest = new.iter().map(|(_, p)| p.life).fold(0.0_f32, f32::max);
        if longest > self.alive_for {
            self.alive_for = longest;
        }
    }

    /// Whether a particle spawned earlier may still be alive — a conservative bound, never a
    /// false "no". Used to keep simulating and drawing after the last emitter despawned, and to
    /// stop once the buffer can only hold dead particles.
    pub fn has_live_particles(&self) -> bool {
        self.alive_for > 0.0
    }

    /// Advances the liveness bound by one frame. Call with the **same `dt` passed to
    /// [`dispatch_compute`](Self::dispatch_compute)**, immediately after it, so the CPU-side bound
    /// and the shader's own `life -= dt` stay in lockstep.
    pub fn advance_lifetimes(&mut self, dt: f32) {
        self.alive_for = (self.alive_for - dt).max(0.0);
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

    /// How many `write_buffer` calls a frame's emission costs — the run count.
    fn runs(slots: &[u32]) -> Vec<(u32, usize)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < slots.len() {
            let len = contiguous_run_len(slots, i, |s| *s);
            out.push((slots[i], len));
            i += len;
        }
        out
    }

    /// A frame's emission occupies consecutive ring slots, so it uploads as **one** range write —
    /// not one per particle, which is what the render stage used to do.
    #[test]
    fn a_whole_emission_uploads_as_one_run() {
        assert_eq!(runs(&[10, 11, 12, 13, 14]), vec![(10, 5)]);
    }

    /// Wrapping the ring splits it into exactly two — still not one per particle. Slot
    /// `capacity - 1` is followed by `0`, and `0 != capacity`, so the split needs no capacity
    /// argument to find.
    #[test]
    fn a_wrap_splits_the_emission_into_two_runs() {
        // capacity 16: … 14, 15, then wrap to 0, 1
        assert_eq!(runs(&[14, 15, 0, 1]), vec![(14, 2), (0, 2)]);
    }

    /// Degenerate inputs: nothing to upload, and a single particle.
    #[test]
    fn empty_and_single_emissions_are_handled() {
        assert_eq!(runs(&[]), vec![]);
        assert_eq!(runs(&[7]), vec![(7, 1)]);
    }

    /// Control: the run-splitting is real, not a function that always returns one run. Slots that
    /// genuinely are not consecutive must not be merged into a single write — doing so would
    /// overwrite the particles in between.
    #[test]
    fn non_consecutive_slots_are_never_merged() {
        assert_eq!(runs(&[3, 9, 10, 40]), vec![(3, 1), (9, 2), (40, 1)]);
    }
}
