//! `render_plugin` — custom GPU pass example
//!
//! Demonstrates how to inject a custom wgpu render pass into the engine using
//! [`RenderPlugin`] without forking the engine's `render()` function.
//!
//! The plugin draws an animated vignette (dark border fade) over the scene each
//! frame. It reads a [`Pulse`] resource from the ECS world to drive the animation
//! intensity, showing that `RenderPlugin::record` has full read-only World access.
//!
//! Run with: `cargo run --example render_plugin`

use engine::{
    App, FrameContext, RenderPlugin, Sprite, System, Transform, Vec2, WindowConfig, World,
};

// ─── Pulse resource ───────────────────────────────────────────────────────────

/// A simple time-accumulator resource the vignette plugin reads each frame.
///
/// A game system increments `t` by `dt`. The plugin converts this to an
/// oscillating `[0, 1]` intensity that pulses the vignette strength.
#[derive(Clone, Copy, Default)]
struct Pulse {
    t: f32,
}

// ─── Pulse system ────────────────────────────────────────────────────────────

struct PulseSystem;

impl System for PulseSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        if let Some(pulse) = world.resource_mut::<Pulse>() {
            pulse.t += dt;
        }
    }
    fn name(&self) -> &'static str {
        "pulse"
    }
}

// ─── Vignette render plugin ───────────────────────────────────────────────────

/// A [`RenderPlugin`] that renders a full-screen animated vignette overlay.
///
/// On first `record`, the plugin lazily creates its own [`wgpu::RenderPipeline`]
/// using the `ctx.device` and `ctx.format` provided by the engine. Each frame it:
/// 1. Reads [`Pulse`] from `world` to get the current oscillation value.
/// 2. Writes a uniform containing the vignette intensity to the GPU.
/// 3. Draws a fullscreen triangle that darkens the screen edges.
///
/// The render pass uses `wgpu::LoadOp::Load` so it *composites over* the existing
/// scene (sprites, UI, etc.) rather than clearing it.
#[derive(Default)]
struct VignettePlugin {
    /// Lazily initialized on the first call to `record`.
    pipeline: Option<wgpu::RenderPipeline>,
    /// Uniform buffer holding `[intensity: f32, _pad: f32, _pad: f32, _pad: f32]`.
    uniform_buf: Option<wgpu::Buffer>,
    /// Bind group referencing the uniform buffer.
    bind_group: Option<wgpu::BindGroup>,
}

impl VignettePlugin {
    /// Create (or re-use) the pipeline and uniform resources.
    ///
    /// Called on the first frame. `format` comes from `ctx.format` — the engine
    /// guarantees it matches the render target's texture format.
    fn ensure_pipeline(&mut self, device: &wgpu::Device, format: wgpu::TextureFormat) {
        if self.pipeline.is_some() {
            return;
        }

        // Combined WGSL source for vertex and fragment stages.
        // The vertex shader emits a fullscreen triangle (no vertex buffer needed).
        // The fragment shader reads `intensity` and applies vignette darkening at edges.
        let src = r#"
struct Uniform {
    intensity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) var<uniform> u: Uniform;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Fullscreen triangle: three vertices whose clip positions cover the entire NDC square.
@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>( 3.0,  1.0),
        vec2<f32>(-1.0,  1.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0),
    );
    var out: VOut;
    out.pos = vec4<f32>(positions[vi], 0.0, 1.0);
    out.uv  = uvs[vi];
    return out;
}

// Vignette factor based on distance from center [0.5, 0.5].
@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let d = distance(in.uv, vec2<f32>(0.5, 0.5));
    // Smooth vignette: transparent at radius 0.3, fully dark at 0.7.
    let vignette = smoothstep(0.3, 0.7, d);
    // Scale by the animated intensity received from the uniform.
    let alpha = vignette * u.intensity * 0.6;
    return vec4<f32>(0.0, 0.0, 0.0, alpha);
}
"#;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("vignette shader"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });

        // Bind group layout: one uniform buffer at binding 0 (fragment-visible).
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("vignette bgl"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // 4 × f32 = 16 bytes — meets the minimum uniform buffer alignment requirement.
        let uniform_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("vignette uniform"),
            size: std::mem::size_of::<[f32; 4]>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("vignette bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("vignette layout"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });

        // Alpha blending: vignette darkens edges by compositing a semi-transparent black quad.
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("vignette pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[], // fullscreen triangle — no vertex buffer
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    // ctx.format ensures we match the engine's render target exactly.
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
        });

        self.uniform_buf = Some(uniform_buf);
        self.bind_group = Some(bind_group);
        self.pipeline = Some(pipeline);
    }
}

impl RenderPlugin for VignettePlugin {
    fn record(&mut self, ctx: &mut FrameContext<'_>, world: &World, _viewport: (u32, u32)) {
        // Lazy init — we have no GPU handles at construction time.
        // ctx.format is the engine's surface format; must match the pipeline's color target.
        self.ensure_pipeline(ctx.device, ctx.format);

        // Read the animated pulse value from the ECS World.
        // This is the key RenderPlugin capability: live read-only World access.
        let intensity = world
            .resource::<Pulse>()
            .map(|p| (p.t * 2.0).sin() * 0.5 + 0.5) // oscillate smoothly between 0 and 1
            .unwrap_or(0.5);

        // Upload the updated intensity to the GPU uniform buffer each frame.
        // bytemuck::bytes_of is available because bytemuck is a direct dep of this package.
        let uniform_data: [f32; 4] = [intensity, 0.0, 0.0, 0.0];
        if let Some(buf) = &self.uniform_buf {
            ctx.queue
                .write_buffer(buf, 0, bytemuck::cast_slice(&uniform_data));
        }

        // Begin the render pass. LoadOp::Load preserves the existing scene pixels
        // so the vignette composites over the sprites/UI/particles drawn so far.
        let mut pass = ctx.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("vignette pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: ctx.view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // composite — do NOT clear the scene
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
            multiview_mask: None,
        });

        if let (Some(pipeline), Some(bg)) = (&self.pipeline, &self.bind_group) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, bg, &[]);
            pass.draw(0..3, 0..1); // fullscreen triangle: 3 vertices, 1 instance
        }
    }
}

// ─── Main ────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "RenderPlugin demo — animated vignette".to_string(),
        width: 800,
        height: 600,
        clear_color: [0.08, 0.08, 0.12, 1.0],
    });

    // Insert the Pulse resource that the plugin reads each frame.
    app.world.insert_resource(Pulse::default());

    // Spawn a simple colored sprite so the vignette effect is visible over it.
    let entity = app.world.spawn();
    app.world.add_component(
        entity,
        Transform {
            position: Vec2::new(400.0, 300.0),
            scale: Vec2::splat(80.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(entity, Sprite::colored(0.2, 0.6, 1.0));

    // Game system that advances the Pulse resource each frame.
    app.add_system(PulseSystem);

    // Register the vignette render plugin. It will be called after sprites/UI/particles
    // each frame, compositing a pulsing dark vignette border over the scene.
    //
    // Multiple plugins can be registered; they run in registration order.
    app.add_render_plugin(VignettePlugin::default());

    app.run();
}
