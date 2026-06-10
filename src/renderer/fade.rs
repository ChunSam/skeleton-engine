use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

/// GPU uniform (16 bytes: RGB + alpha)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FadeUniforms {
    color: [f32; 3],
    alpha: f32,
}

/// Renderer that draws a full-screen color overlay.
///
/// The App calls this automatically when the `FadeTransition` resource has alpha > 0.001.
/// Because it uses alpha blending, it must run last — after the sprite, UI, text,
/// lighting, and post-process passes.
pub struct FadeRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

// Vertex stage shared with lighting.rs — fullscreen 6-vertex quad.
// Fragment stage is fade-specific (uniform color overlay, alpha blending).
const FADE_SHADER: &str = concat!(
    include_str!("shaders/fullscreen_quad.wgsl"),
    r#"

struct Uniforms {
    color: vec3<f32>,
    alpha: f32,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4(u.color, u.alpha);
}
"#
);

impl FadeRenderer {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fade shader"),
            source: wgpu::ShaderSource::Wgsl(FADE_SHADER.into()),
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fade uniforms"),
            contents: bytemuck::bytes_of(&FadeUniforms {
                color: [0.0, 0.0, 0.0],
                alpha: 0.0,
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fade bgl"),
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
            label: Some("fade bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fade pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("fade pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
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
            multiview: None,
            cache: None,
        });

        Self {
            pipeline,
            uniform_buffer,
            bind_group,
        }
    }

    /// Updates the uniform buffer with the current fade values.
    pub fn update(&self, queue: &wgpu::Queue, color: [f32; 3], alpha: f32) {
        let uniforms = FadeUniforms { color, alpha };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Executes the fade pass. Must be called after all other render passes.
    pub fn run_pass(&self, encoder: &mut wgpu::CommandEncoder, output_view: &wgpu::TextureView) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fade_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // overlay on top of existing render
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..1);
    }
}
