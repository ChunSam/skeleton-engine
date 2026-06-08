use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::components::{PointLight, Transform};
use crate::ecs::World;
use crate::resources::AmbientLight;

// ─── GPU structs ──────────────────────────────────────────────────────────────

/// Single point light data sent to the GPU (32 bytes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuLightData {
    pub position_ndc: [f32; 2],
    /// Light radius in the SAME space the shader measures fragment distance in:
    /// UV fraction-of-viewport-width, i.e. `radius * zoom / viewport_w`. The
    /// `_ndc` suffix is a historical misnomer — this value is UV-space, not NDC
    /// (NDC would be 2× this). The shader converts `position_ndc` to UV
    /// (`*0.5+0.5`) and compares an aspect-corrected UV distance against this, so
    /// the falloff reaches 0 at exactly the light's world-space radius. Do NOT
    /// double it (see the `light_radius_*` unit tests).
    pub radius_ndc: f32,
    pub intensity: f32,
    pub color: [f32; 3],
    pub light_height: f32, // virtual Z height for flat-normal lighting (0.05~1.0 typical)
}

/// Full GPU uniform block (544 bytes).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct LightingUniforms {
    pub ambient_color: [f32; 3],
    pub ambient_intensity: f32,
    pub light_count: u32,
    pub aspect_ratio: f32,
    pub _pad: [f32; 2],
    pub lights: [GpuLightData; 16],
}

// ─── WGSL shader ──────────────────────────────────────────────────────────────

const LIGHTING_SHADER: &str = r#"
struct GpuLight {
    position_ndc: vec2<f32>,
    radius_ndc:   f32,
    intensity:    f32,
    color:        vec3<f32>,
    light_height: f32,
}

struct LightingUniforms {
    ambient_color:     vec3<f32>,
    ambient_intensity: f32,
    light_count:       u32,
    aspect_ratio:      f32,
    _pad:              vec2<f32>,
    lights:            array<GpuLight, 16>,
}

@group(0) @binding(0) var scene_tex:     texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> u:    LightingUniforms;
@group(0) @binding(3) var normal_tex:    texture_2d<f32>;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VOut {
    var pos = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0,  1.0), vec2(1.0, -1.0), vec2( 1.0, 1.0),
    );
    var uv = array<vec2<f32>, 6>(
        vec2(0.0, 1.0), vec2(1.0, 1.0), vec2(0.0, 0.0),
        vec2(0.0, 0.0), vec2(1.0, 1.0), vec2(1.0, 0.0),
    );
    var out: VOut;
    out.pos = vec4(pos[idx], 0.0, 1.0);
    out.uv  = uv[idx];
    return out;
}

@fragment
fn fs_main(in: VOut) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, scene_sampler, in.uv);

    // Normal from the flat-normal buffer: [0,1] -> [-1,1], then normalize.
    let n_sample = textureSample(normal_tex, scene_sampler, in.uv);
    let N = normalize(n_sample.xyz * 2.0 - vec3(1.0, 1.0, 1.0));

    var total = u.ambient_color * u.ambient_intensity;

    for (var i = 0u; i < u.light_count; i = i + 1u) {
        let l        = u.lights[i];
        let uv_light = l.position_ndc * 0.5 + vec2(0.5, 0.5);
        let diff_uv  = uv_light - in.uv;

        // Distance attenuation (screen space, aspect-corrected)
        let d     = length(vec2(diff_uv.x, diff_uv.y * u.aspect_ratio));
        let atten = max(0.0, 1.0 - d / l.radius_ndc);

        // Lambert diffuse using the flat-normal buffer.
        // Light direction in UV space -> normalize to get L vector
        // diff_uv.y is negated because UV Y is flipped relative to NDC Y
        let L       = normalize(vec3(diff_uv.x, -diff_uv.y * u.aspect_ratio, l.light_height));
        let diffuse = max(0.0, dot(N, L));

        total = total + l.color * l.intensity * diffuse * atten * atten;
    }

    return vec4(scene.rgb * min(total, vec3(1.0)), scene.a);
}
"#;

// ─── LightingRenderer ────────────────────────────────────────────────────────

/// Full-screen pass that takes the scene texture as input and applies point lights.
///
/// Created and run automatically by `App` when an `AmbientLight` resource is present in the World.
pub struct LightingRenderer {
    /// Normal buffer texture (same size as the viewport, Rgba8Unorm).
    normal_texture: wgpu::Texture,
    /// Normal buffer texture view (lighting shader binding 3).
    pub normal_view: wgpu::TextureView,
    /// Current output texture width.
    pub width: u32,
    /// Current output texture height.
    pub height: u32,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
}

fn light_position_ndc(
    position: glam::Vec2,
    radius: f32,
    camera: Camera,
    vp_w: u32,
    vp_h: u32,
) -> ([f32; 2], f32) {
    let viewport_w = vp_w.max(1) as f32;
    let viewport_h = vp_h.max(1) as f32;
    let camera_origin = camera.position + camera.shake_offset();
    let screen = (position - camera_origin) * camera.zoom;
    let half_w = viewport_w / 2.0;
    let half_h = viewport_h / 2.0;
    let ndc_x = screen.x / half_w - 1.0;
    let ndc_y = screen.y / half_h - 1.0;
    // Radius in UV-fraction-of-width (NOT NDC): a world radius `r` is `r*zoom`
    // pixels, and the shader measures fragment distance as a fraction of viewport
    // width (uv = pixel / viewport_w). These share one space, so the light's edge
    // lands at exactly the world radius. (Despite the `_ndc` name — see the field
    // doc on `GpuLightData::radius_ndc`. Doubling this would render lights 2× too
    // large.)
    let radius_ndc = radius * camera.zoom / viewport_w;
    ([ndc_x, ndc_y], radius_ndc)
}

/// Maximum number of lights the lighting pass can process in one call (matches the GPU uniform array size).
const MAX_LIGHTS: usize = 16;

/// Selects indices of up to `MAX_LIGHTS` lights, nearest to the camera first.
///
/// Replaces the old behavior where distant lights were silently dropped (first 16 in query order)
/// when the light count exceeded the cap. The return order is not sorted
/// (additive lighting makes the order within the 16 irrelevant).
fn select_nearest_lights(positions: &[glam::Vec2], camera_pos: glam::Vec2) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..positions.len()).collect();
    if positions.len() > MAX_LIGHTS {
        idx.select_nth_unstable_by(MAX_LIGHTS - 1, |&a, &b| {
            positions[a]
                .distance_squared(camera_pos)
                .total_cmp(&positions[b].distance_squared(camera_pos))
        });
        idx.truncate(MAX_LIGHTS);
    }
    idx
}

impl LightingRenderer {
    /// Creates a new `LightingRenderer`.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lighting shader"),
            source: wgpu::ShaderSource::Wgsl(LIGHTING_SHADER.into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("lighting sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lighting uniforms"),
            contents: bytemuck::bytes_of(&LightingUniforms::zeroed()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting bgl"),
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
                // binding 2: uniform buffer
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
                // binding 3: flat-normal buffer texture
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lighting pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lighting pipeline"),
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
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
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

        let (normal_texture, normal_view) = Self::create_normal_buffer(device, width, height);

        Self {
            normal_texture,
            normal_view,
            width,
            height,
            format: surface_format,
            pipeline,
            sampler,
            bind_group_layout,
            uniform_buffer,
        }
    }

    pub(crate) fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    pub(crate) fn reconfigure(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
    ) {
        *self = Self::new(device, width, height, surface_format);
    }

    /// Recreates the normal buffer when the window is resized.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

        let (ntex, nview) = Self::create_normal_buffer(device, width, height);
        self.normal_texture = ntex;
        self.normal_view = nview;

        self.width = width;
        self.height = height;
    }

    /// Clears the normal buffer to the flat-normal color (0.5, 0.5, 1.0, 1.0) each frame.
    pub fn clear_normal_buffer(&self, encoder: &mut wgpu::CommandEncoder) {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("clear_normal"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.normal_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    // LoadOp::Clear fills the attachment with the flat normal color.
                    // No draw call needed.
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.5,
                        g: 0.5,
                        b: 1.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        // pass drops here — clear is committed
    }

    /// Collects light data from the ECS World and updates the uniform buffer.
    ///
    /// When more than `MAX_LIGHTS` (16) lights are present, only the 16 closest to the camera
    /// are sent (distant lights are not dropped arbitrarily).
    pub fn update(&self, queue: &wgpu::Queue, world: &World, vp_w: u32, vp_h: u32) {
        let ambient = world
            .resource::<AmbientLight>()
            .copied()
            .unwrap_or_default();

        let camera = world.resource::<Camera>().copied().unwrap_or_default();

        // Collect all point lights as (world position, light data).
        let collected: Vec<(glam::Vec2, PointLight)> = world
            .query2::<PointLight, Transform>()
            .map(|(_, light, transform)| (transform.position, *light))
            .collect();

        if collected.len() > MAX_LIGHTS {
            static CAP_WARN: std::sync::Once = std::sync::Once::new();
            let n = collected.len();
            CAP_WARN.call_once(|| {
                log::warn!(
                    "lighting: {n} point lights exceed the {MAX_LIGHTS}-light cap; \
                     rendering the nearest {MAX_LIGHTS} to the camera"
                );
            });
        }

        let positions: Vec<glam::Vec2> = collected.iter().map(|(p, _)| *p).collect();
        let selected = select_nearest_lights(&positions, camera.position);

        let mut lights_gpu = [GpuLightData::zeroed(); MAX_LIGHTS];
        let mut light_count = 0u32;
        for &i in &selected {
            let (pos, light) = collected[i];
            let (position_ndc, radius_ndc) =
                light_position_ndc(pos, light.radius, camera, vp_w, vp_h);
            lights_gpu[light_count as usize] = GpuLightData {
                position_ndc,
                radius_ndc,
                intensity: light.intensity,
                color: light.color.to_rgb(),
                light_height: light.light_height,
            };
            light_count += 1;
        }

        let uniforms = LightingUniforms {
            ambient_color: ambient.color.to_rgb(),
            ambient_intensity: ambient.intensity,
            light_count,
            aspect_ratio: vp_h as f32 / vp_w as f32,
            _pad: [0.0; 2],
            lights: lights_gpu,
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    /// Applies lighting to the scene texture and writes the result to `output_view`.
    pub fn run_pass(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        // The scene texture and normal buffer may change each frame, so create the bind group on the fly.
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.normal_view),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("lighting pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..6, 0..1);
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn create_normal_buffer(
        device: &wgpu::Device,
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("normal_buf"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        (tex, view)
    }
}

// ─── Unit tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_struct_sizes() {
        assert_eq!(std::mem::size_of::<GpuLightData>(), 32);
        assert_eq!(std::mem::size_of::<LightingUniforms>(), 544);
    }

    #[test]
    fn light_position_uses_camera_transform() {
        let camera = Camera::default();
        let (ndc, radius) =
            light_position_ndc(glam::Vec2::new(400.0, 300.0), 100.0, camera, 800, 600);
        assert!((ndc[0] - 0.0).abs() < 1e-5);
        assert!((ndc[1] - 0.0).abs() < 1e-5);
        assert!((radius - 0.125).abs() < 1e-5);

        let (ndc, _) = light_position_ndc(glam::Vec2::new(400.0, 0.0), 100.0, camera, 800, 600);
        assert!((ndc[0] - 0.0).abs() < 1e-5);
        assert!((ndc[1] + 1.0).abs() < 1e-5);

        let camera = Camera::new(glam::Vec2::new(100.0, 50.0), 2.0);
        let (ndc, radius) =
            light_position_ndc(glam::Vec2::new(300.0, 200.0), 100.0, camera, 800, 600);
        assert!((ndc[0] - 0.0).abs() < 1e-5);
        assert!((ndc[1] - 0.0).abs() < 1e-5);
        assert!((radius - 0.25).abs() < 1e-5);
    }

    /// #15 contract: the CPU `radius_ndc` and the shader's fragment distance live
    /// in the SAME space (UV fraction-of-width), so a point light's falloff
    /// reaches 0 at exactly its world-space radius — not half, not double.
    ///
    /// This guards against the (tempting but wrong) "fix" of doubling the radius
    /// to `2*radius/viewport_w`: the shader measures distance in UV ([0,1]),
    /// where the matching radius is `radius*zoom/viewport_w`. Doubling it would
    /// render lights 2× too large.
    #[test]
    fn light_radius_falloff_reaches_zero_at_world_radius() {
        let camera = Camera::default(); // pos (0,0), zoom 1
        let (vp_w, vp_h) = (800u32, 600u32);
        let world_radius = 100.0_f32;
        let light_world = glam::Vec2::new(400.0, 300.0); // screen center
        let (_ndc, radius_uv) = light_position_ndc(light_world, world_radius, camera, vp_w, vp_h);

        // The shader computes diff_uv = uv_light - in.uv, both in [0,1]. A fragment
        // exactly `world_radius` world-units to the right of the light is this far
        // in UV-fraction-of-width:
        let edge_d_uv = world_radius * camera.zoom / vp_w as f32;

        // atten = 1 - d/radius_uv must hit 0 right at the world radius → the CPU
        // radius equals the edge UV distance (same space).
        assert!(
            (radius_uv - edge_d_uv).abs() < 1e-6,
            "radius_uv {radius_uv} must equal the world-radius UV distance {edge_d_uv} \
             (same space); a 2× value would mean half/double-size lights"
        );

        // Half-radius fragment → atten 0.5 (linear falloff sanity check).
        let atten_half = 1.0 - (edge_d_uv * 0.5) / radius_uv;
        assert!((atten_half - 0.5).abs() < 1e-6);

        // Aspect-corrected vertical edge also lands at the world radius: a fragment
        // `world_radius` px above maps to diff_uv.y = r/vp_h, scaled by
        // aspect_ratio = vp_h/vp_w → r/vp_w, matching radius_uv.
        let aspect_ratio = vp_h as f32 / vp_w as f32;
        let vertical_d_uv = (world_radius / vp_h as f32) * aspect_ratio;
        assert!(
            (vertical_d_uv - radius_uv).abs() < 1e-6,
            "falloff must be circular"
        );
    }

    #[test]
    fn select_nearest_lights_returns_all_under_cap() {
        let positions: Vec<glam::Vec2> = (0..5).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn select_nearest_lights_keeps_closest_to_camera() {
        // Place 18 lights at x = 0,1,..,17 → smaller index = closer to the camera at the origin.
        let positions: Vec<glam::Vec2> = (0..18).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO);

        assert_eq!(selected.len(), MAX_LIGHTS);
        // Only the 16 closest (indices 0..=15) should be selected; the farthest 16 and 17 must be excluded.
        let mut sorted = selected.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..MAX_LIGHTS).collect::<Vec<usize>>());
        assert!(!selected.contains(&16));
        assert!(!selected.contains(&17));
    }
}
