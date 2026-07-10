use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// GPU uniform (32 bytes, all `vec4` to avoid `vec3` alignment surprises):
/// `params = (coverage, style_index, aspect, softness)`, `color = rgba`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TransitionUniforms {
    params: [f32; 4],
    color: [f32; 4],
}

/// Renderer that draws a styled full-screen scene-transition overlay (fade / wipe / iris).
///
/// `App` calls this automatically when a `SceneTransition` resource has `coverage > 0.001`. It
/// alpha-blends over the finished scene, so it must run last (after sprite / UI / text / lighting /
/// post-process — in the same slot as the fade pass). The coverage geometry mirrors
/// `SceneTransition::covered_at` (with a soft edge).
pub struct TransitionRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

// Vertex stage shared with fade.rs/lighting.rs — fullscreen 6-vertex quad (provides uv).
const TRANSITION_SHADER: &str = concat!(
    include_str!("shaders/fullscreen_quad.wgsl"),
    r#"

struct Uniforms {
    params: vec4<f32>, // coverage, style, aspect, softness
    color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    let coverage = u.params.x;
    let style = u.params.y;
    let aspect = u.params.z;
    let soft = max(u.params.w, 0.0001);
    var a = 0.0;
    if (style < 0.5) {
        // Fade — uniform alpha.
        a = coverage;
    } else if (style < 1.5) {
        // WipeLeft — covered where uv.x <= coverage.
        a = 1.0 - smoothstep(coverage - soft, coverage + soft, uv.x);
    } else if (style < 2.5) {
        // WipeRight — covered where uv.x >= 1 - coverage.
        a = smoothstep(1.0 - coverage - soft, 1.0 - coverage + soft, uv.x);
    } else if (style < 3.5) {
        // WipeDown — covered where uv.y <= coverage.
        a = 1.0 - smoothstep(coverage - soft, coverage + soft, uv.y);
    } else if (style < 4.5) {
        // WipeUp — covered where uv.y >= 1 - coverage.
        a = smoothstep(1.0 - coverage - soft, 1.0 - coverage + soft, uv.y);
    } else {
        // Iris — aspect-corrected radius, normalized so the farthest corner is 1.
        let dx = (uv.x - 0.5) * aspect;
        let dy = uv.y - 0.5;
        let max_r = sqrt(0.25 * aspect * aspect + 0.25);
        let r = sqrt(dx * dx + dy * dy) / max_r;
        if (style < 5.5) {
            // IrisIn — covered where r >= 1 - coverage (closes to the centre).
            a = smoothstep(1.0 - coverage - soft, 1.0 - coverage + soft, r);
        } else {
            // IrisOut — covered where r <= coverage (grows from the centre).
            a = 1.0 - smoothstep(coverage - soft, coverage + soft, r);
        }
    }
    return vec4(u.color.rgb, clamp(a, 0.0, 1.0) * u.color.a);
}
"#
);

impl TransitionRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("transition shader"),
            source: wgpu::ShaderSource::Wgsl(TRANSITION_SHADER.into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("transition uniforms"),
            contents: bytemuck::bytes_of(&TransitionUniforms {
                params: [0.0, 0.0, 1.0, 0.01],
                color: [0.0, 0.0, 0.0, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transition bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transition bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("transition pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("transition pipeline"),
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
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    /// Updates the uniform buffer. `style` is the `TransitionStyle` shader index, `aspect` is
    /// width/height, `color` is RGBA.
    pub fn update(
        &self,
        queue: &wgpu::Queue,
        coverage: f32,
        style: f32,
        aspect: f32,
        color: [f32; 4],
    ) {
        let uniforms = TransitionUniforms {
            params: [coverage, style, aspect, 0.01],
            color,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Executes the transition pass. Must be called after all other render passes.
    pub fn run_pass(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("transition_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // overlay on top of the finished scene
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
