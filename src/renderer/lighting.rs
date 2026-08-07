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
pub(crate) struct GpuLightData {
    pub(crate) position_ndc: [f32; 2],
    /// Light radius in the SAME space the shader measures fragment distance in:
    /// UV fraction-of-viewport-width, i.e. `radius * zoom / viewport_w`. The
    /// `_ndc` suffix is a historical misnomer — this value is UV-space, not NDC
    /// (NDC would be 2× this). The shader converts `position_ndc` to UV
    /// (`*0.5+0.5`) and compares an aspect-corrected UV distance against this, so
    /// the falloff reaches 0 at exactly the light's world-space radius. Do NOT
    /// double it (see the `light_radius_*` unit tests).
    pub(crate) radius_ndc: f32,
    pub(crate) intensity: f32,
    pub(crate) color: [f32; 3],
    pub(crate) light_height: f32, // virtual Z height for flat-normal lighting (0.05~1.0 typical)
}

/// Fixed-size header of the lighting uniform block (32 bytes), followed in the GPU
/// buffer by a runtime-sized `[GpuLightData; max_lights]` array.
///
/// The whole uniform is `32 + max_lights * 32` bytes. The light count was once baked
/// into a fixed `[GpuLightData; 16]`; splitting the header off lets the array length
/// be a runtime value (see [`LightingConfig`](crate::resources::LightingConfig)) while
/// the field order — and therefore the WGSL std140 layout — stays exactly as before
/// (header occupies offsets 0..32, lights start at 32).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct LightingHeader {
    pub(crate) ambient_color: [f32; 3],
    pub(crate) ambient_intensity: f32,
    pub(crate) light_count: u32,
    pub(crate) aspect_ratio: f32,
    pub(crate) _pad: [f32; 2],
}

// ─── WGSL shader ──────────────────────────────────────────────────────────────
// Vertex stage shared with fade.rs — see shaders/fullscreen_quad.wgsl.
// Fragment stage is lighting-specific (scene texture + point-light accumulation).

const LIGHTING_SHADER: &str = concat!(
    include_str!("shaders/fullscreen_quad.wgsl"),
    r#"
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
    lights:            array<GpuLight, MAX_LIGHTS>,
}

@group(0) @binding(0) var scene_tex:     texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> u:    LightingUniforms;
@group(0) @binding(3) var normal_tex:    texture_2d<f32>;

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
"#
);

// ─── LightingRenderer ────────────────────────────────────────────────────────

/// Full-screen pass that takes the scene texture as input and applies point lights.
///
/// Created and run automatically by `App` when an `AmbientLight` resource is present in the World.
///
/// **Platform note:** lighting is native-only. On `wasm32` targets this renderer is not
/// compiled in and the lighting pass is silently skipped (no-op on wasm32).
/// Aspect ratio the lighting shader needs, in **window** space.
///
/// The shader measures a light's distance in window UV space, and `radius_ndc` is rebased there
/// with `clip_scale`. The aspect has to live in the same space or the two disagree: under a
/// `DesignResolution` letterbox `vp_w`/`vp_h` are the DESIGN size, so a point light came out
/// elliptical and its shape changed with the window. The same uniform also feeds the Lambert
/// direction vector, so it skewed diffuse shading, not just attenuation.
///
/// An identity letterbox is `(1, 1)`, so the no-`DesignResolution` path is byte-identical.
pub(crate) fn lighting_aspect(vp_w: u32, vp_h: u32, clip_scale: glam::Vec2) -> f32 {
    if clip_scale.y.abs() > f32::EPSILON {
        (vp_h as f32 * clip_scale.x) / (vp_w as f32 * clip_scale.y)
    } else {
        vp_h as f32 / vp_w as f32
    }
}

pub struct LightingRenderer {
    /// Normal buffer texture (same size as the viewport, Rgba8Unorm).
    normal_texture: wgpu::Texture,
    /// Normal buffer texture view (lighting shader binding 3).
    pub(crate) normal_view: wgpu::TextureView,
    /// Current output texture width.
    pub(crate) width: u32,
    /// Current output texture height.
    pub(crate) height: u32,
    /// Maximum point lights this renderer's shader + uniform buffer were built for.
    /// A change (game inserts/edits [`LightingConfig`](crate::resources::LightingConfig))
    /// triggers a rebuild via `reconfigure` — the shader array length is baked at build.
    max_lights: usize,
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    /// Cached bind group for the lighting pass.
    ///
    /// Rebuilt lazily in `run_pass` when the scene texture pointer changes (i.e. on
    /// resize / texture recreation) or when the normal buffer is recreated. Avoids
    /// a `device.create_bind_group` call every frame on the submit path.
    cached_bind_group: Option<wgpu::BindGroup>,
    /// Raw pointer address of the `TextureView` that `cached_bind_group` was built
    /// from.  Used as a cheap identity check — `wgpu::TextureView` has no `PartialEq`.
    cached_scene_view_ptr: usize,
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

/// Selects indices of up to `max_lights` lights, nearest to the camera first.
///
/// Replaces the old behavior where distant lights were silently dropped (first N in query order)
/// when the light count exceeded the cap. The return order is not sorted
/// (additive lighting makes the order within the selection irrelevant).
fn select_nearest_lights(
    positions: &[glam::Vec2],
    camera_pos: glam::Vec2,
    max_lights: usize,
) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..positions.len()).collect();
    if positions.len() > max_lights && max_lights > 0 {
        idx.select_nth_unstable_by(max_lights - 1, |&a, &b| {
            positions[a]
                .distance_squared(camera_pos)
                .total_cmp(&positions[b].distance_squared(camera_pos))
        });
        idx.truncate(max_lights);
    } else if max_lights == 0 {
        idx.clear();
    }
    idx
}

impl LightingRenderer {
    /// Creates a new `LightingRenderer` sized for up to `max_lights` point lights.
    ///
    /// `max_lights` sets the WGSL `array<GpuLight, N>` length (baked at shader build) and
    /// the uniform buffer size (`32 + max_lights * 32` bytes). A value of 0 is clamped to
    /// 1 so the shader array stays non-empty (no lights are then ever selected).
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
        max_lights: usize,
    ) -> Self {
        // WGSL forbids a zero-length array; clamp the *shader* array to >=1. The cull still
        // selects 0 lights when the configured cap is 0, so this only guards the type.
        let array_len = max_lights.max(1);
        // `LIGHTING_SHADER` carries the `MAX_LIGHTS` token (invalid WGSL on its own) so the
        // GPU array length stays bound to the configured cap — the single source of truth
        // shared with the uniform buffer size and the nearest-light cull.
        let shader_src = LIGHTING_SHADER.replace("MAX_LIGHTS", &array_len.to_string());
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("lighting shader"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });

        let sampler = super::common::create_clamp_sampler(
            device,
            Some("lighting sampler"),
            wgpu::FilterMode::Linear,
        );

        // Header (32 B) + array_len * GpuLightData (32 B each), zero-initialized so a pass
        // that somehow runs before the first `update` reads a valid (all-dark) block.
        let uniform_size =
            std::mem::size_of::<LightingHeader>() + array_len * std::mem::size_of::<GpuLightData>();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lighting uniforms"),
            contents: &vec![0u8; uniform_size],
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting bgl"),
            entries: &[
                super::common::filterable_texture_entry(0), // binding 0: scene texture
                super::common::filtering_sampler_entry(1),  // binding 1: sampler
                super::common::uniform_buffer_entry(2),     // binding 2: uniform buffer
                super::common::filterable_texture_entry(3), // binding 3: flat-normal buffer texture
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lighting pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("lighting pipeline"),
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
                    format: surface_format,
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

        let (normal_texture, normal_view) = Self::create_normal_buffer(device, width, height);

        Self {
            normal_texture,
            normal_view,
            width,
            height,
            max_lights,
            format: surface_format,
            pipeline,
            sampler,
            bind_group_layout,
            uniform_buffer,
            cached_bind_group: None,
            cached_scene_view_ptr: 0,
        }
    }

    pub(crate) fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// The point-light cap this renderer was built for (drives the rebuild-on-change
    /// check in `setup_lighting` when a game edits [`LightingConfig`]).
    pub(crate) fn max_lights(&self) -> usize {
        self.max_lights
    }

    pub(crate) fn reconfigure(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        surface_format: wgpu::TextureFormat,
    ) {
        *self = Self::new(device, width, height, surface_format, self.max_lights);
    }

    /// Rebuilds the renderer for a new point-light cap (shader array length + uniform
    /// buffer size are baked at build time, so a cap change needs a full rebuild).
    pub(crate) fn set_max_lights(&mut self, device: &wgpu::Device, max_lights: usize) {
        *self = Self::new(device, self.width, self.height, self.format, max_lights);
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

        // normal_view is new; the cached bind group references the old one.
        // Drop it so run_pass rebuilds it with the new normal and scene views.
        self.cached_bind_group = None;
        self.cached_scene_view_ptr = 0;
    }

    /// Clears the normal buffer to the flat-normal color (0.5, 0.5, 1.0, 1.0) each frame.
    pub fn clear_normal_buffer(&self, encoder: &mut wgpu::CommandEncoder) {
        // LoadOp::Clear fills the attachment with the flat-normal color; no draw call needed.
        let _pass = super::common::begin_color_pass(
            encoder,
            "clear_normal",
            &self.normal_view,
            wgpu::LoadOp::Clear(wgpu::Color {
                r: 0.5,
                g: 0.5,
                b: 1.0,
                a: 1.0,
            }),
        );
        // pass drops here — clear is committed
    }

    /// Collects light data from the ECS World and updates the uniform buffer.
    ///
    /// When more than `max_lights` point lights are present, only the `max_lights` closest
    /// to the camera are sent (distant lights are not dropped arbitrarily).
    pub fn update(
        &self,
        queue: &wgpu::Queue,
        world: &World,
        vp_w: u32,
        vp_h: u32,
        clip_scale: glam::Vec2,
    ) {
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

        let max_lights = self.max_lights;
        if collected.len() > max_lights {
            static CAP_WARN: std::sync::Once = std::sync::Once::new();
            let n = collected.len();
            CAP_WARN.call_once(|| {
                log::warn!(
                    "lighting: {n} point lights exceed the {max_lights}-light cap; \
                     rendering the nearest {max_lights} to the camera \
                     (raise it with LightingConfig {{ max_lights }})"
                );
            });
        }

        let positions: Vec<glam::Vec2> = collected.iter().map(|(p, _)| *p).collect();
        // Cull anchor = viewport center in world space (camera.position is the top-left corner).
        let cull_center = camera.position
            + glam::Vec2::new(vp_w as f32, vp_h as f32) / (2.0 * camera.zoom.max(f32::EPSILON));
        let selected = select_nearest_lights(&positions, cull_center, max_lights);

        // Lights region of the uniform: exactly `max_lights` entries, zeroed past the count.
        let mut lights_gpu = vec![GpuLightData::zeroed(); max_lights];
        let mut light_count = 0u32;
        for &i in &selected {
            let (pos, light) = collected[i];
            let (mut position_ndc, mut radius_ndc) =
                light_position_ndc(pos, light.radius, camera, vp_w, vp_h);
            // Letterbox: scale the design-space NDC into the centered window sub-rect, and the
            // width-fraction radius by the horizontal scale (identity = no-op).
            position_ndc[0] *= clip_scale.x;
            position_ndc[1] *= clip_scale.y;
            radius_ndc *= clip_scale.x;
            lights_gpu[light_count as usize] = GpuLightData {
                position_ndc,
                radius_ndc,
                intensity: light.intensity,
                color: light.color.to_rgb(),
                light_height: light.light_height,
            };
            light_count += 1;
        }

        let header = LightingHeader {
            ambient_color: ambient.color.to_rgb(),
            ambient_intensity: ambient.intensity,
            light_count,
            // The shader measures light distance in WINDOW UV space, and `radius_ndc` above was
            // already rebased there via `clip_scale`. The aspect must live in the same space or
            // the two disagree: under a `DesignResolution` letterbox `vp_w`/`vp_h` are the
            // DESIGN size, so a point light came out elliptical and its shape changed with the
            // window. The same uniform also feeds the Lambert direction vector, so it skewed
            // diffuse shading, not just attenuation. Guard `clip_scale.y` — an identity
            // letterbox is (1,1), so the OFF path is byte-identical.
            aspect_ratio: lighting_aspect(vp_w, vp_h, clip_scale),
            _pad: [0.0; 2],
        };

        // GPU layout = [header (32 B)] ++ [GpuLightData; max_lights]. Assemble as one
        // contiguous byte block matching the WGSL `LightingUniforms` std140 layout.
        let mut bytes = Vec::with_capacity(
            std::mem::size_of::<LightingHeader>()
                + lights_gpu.len() * std::mem::size_of::<GpuLightData>(),
        );
        bytes.extend_from_slice(bytemuck::bytes_of(&header));
        bytes.extend_from_slice(bytemuck::cast_slice(&lights_gpu));
        queue.write_buffer(&self.uniform_buffer, 0, &bytes);
    }

    /// Applies lighting to the scene texture and writes the result to `output_view`.
    ///
    /// The bind group (scene texture + normal buffer + sampler + uniform buffer) is cached
    /// and only rebuilt when `scene_view` points to a different texture than last call (i.e.
    /// after a resize / texture recreation). The normal buffer view lives on `self` and is
    /// invalidated via `cached_bind_group = None` inside `resize`.
    pub fn run_pass(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        let scene_ptr = scene_view as *const wgpu::TextureView as usize;
        if self.cached_bind_group.is_none() || self.cached_scene_view_ptr != scene_ptr {
            self.cached_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            }));
            self.cached_scene_view_ptr = scene_ptr;
        }

        // The branch above always sets cached_bind_group when it was None.
        let bind_group = self
            .cached_bind_group
            .as_ref()
            .expect("cached_bind_group is set in the branch above");

        let mut pass = super::common::begin_color_pass(
            encoder,
            "lighting pass",
            output_view,
            wgpu::LoadOp::Clear(wgpu::Color::BLACK),
        );

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, bind_group, &[]);
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

    fn uniform_size(max_lights: usize) -> usize {
        std::mem::size_of::<LightingHeader>() + max_lights * std::mem::size_of::<GpuLightData>()
    }

    #[test]
    fn gpu_struct_sizes() {
        assert_eq!(std::mem::size_of::<GpuLightData>(), 32);
        assert_eq!(std::mem::size_of::<LightingHeader>(), 32);
        // The full uniform = header + max_lights light slots. The default cap (16)
        // reproduces the historical 544-byte block exactly (32 + 16*32); the size
        // scales linearly with the configured cap.
        assert_eq!(uniform_size(crate::resources::DEFAULT_MAX_LIGHTS), 544);
        assert_eq!(uniform_size(64), 2080);
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

    const CAP: usize = crate::resources::DEFAULT_MAX_LIGHTS;

    #[test]
    fn select_nearest_lights_returns_all_under_cap() {
        let positions: Vec<glam::Vec2> = (0..5).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO, CAP);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn select_nearest_lights_keeps_closest_to_camera() {
        // Place 18 lights at x = 0,1,..,17 → smaller index = closer to the camera at the origin.
        let positions: Vec<glam::Vec2> = (0..18).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO, CAP);

        assert_eq!(selected.len(), CAP);
        // Only the 16 closest (indices 0..=15) should be selected; the farthest 16 and 17 must be excluded.
        let mut sorted = selected.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..CAP).collect::<Vec<usize>>());
        assert!(!selected.contains(&16));
        assert!(!selected.contains(&17));
    }

    #[test]
    fn select_nearest_lights_honors_a_custom_cap() {
        // 30 lights, cap raised to 24 → exactly the 24 nearest (indices 0..=23) selected.
        let positions: Vec<glam::Vec2> = (0..30).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO, 24);
        assert_eq!(selected.len(), 24);
        let mut sorted = selected.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..24).collect::<Vec<usize>>());

        // A cap of 0 selects nothing (a darkened scene, no panic on the empty-array path).
        assert!(select_nearest_lights(&positions, glam::Vec2::ZERO, 0).is_empty());
    }

    #[test]
    fn select_nearest_lights_uses_viewport_center_not_top_left() {
        // Camera top-left at (0,0), zoom=1, viewport 800×600 → center = (400, 300).
        // Place one light at (400,300) (center) and one at (10,10) (near top-left).
        // The center light should be closer to the cull anchor (400,300).
        let camera = Camera::default();
        let (vp_w, vp_h) = (800u32, 600u32);
        let cull_center = camera.position
            + glam::Vec2::new(vp_w as f32, vp_h as f32) / (2.0 * camera.zoom.max(f32::EPSILON));
        assert!(
            (cull_center - glam::Vec2::new(400.0, 300.0)).length() < 1e-3,
            "cull center should be viewport center: {cull_center:?}"
        );

        // With 18 lights: 16 near (400,300) and 2 far near top-left corner.
        // The 16 near-center lights should be selected; the 2 top-left lights excluded.
        let mut positions: Vec<glam::Vec2> = (0..16)
            .map(|i| glam::Vec2::new(400.0 + i as f32, 300.0))
            .collect();
        let far_idx_0 = positions.len();
        positions.push(glam::Vec2::new(1.0, 1.0)); // near top-left
        let far_idx_1 = positions.len();
        positions.push(glam::Vec2::new(2.0, 1.0)); // near top-left

        let selected = select_nearest_lights(&positions, cull_center, CAP);
        assert_eq!(selected.len(), CAP);
        assert!(
            !selected.contains(&far_idx_0),
            "top-left light should be excluded"
        );
        assert!(
            !selected.contains(&far_idx_1),
            "top-left light should be excluded"
        );
    }

    /// The lighting aspect must be measured in WINDOW space, not design space.
    ///
    /// `radius_ndc` is rebased into window space with `clip_scale`, and the shader compares the
    /// two — so leaving the aspect in design space made a point light elliptical under any
    /// `DesignResolution` letterbox, with the distortion changing as the window resized. The
    /// same uniform feeds the Lambert direction vector, so diffuse shading skewed too.
    #[test]
    fn lighting_aspect_is_measured_in_window_space() {
        // No letterbox: identity clip scale must be byte-identical to the raw ratio.
        let plain = lighting_aspect(1600, 900, glam::Vec2::ONE);
        assert!((plain - 900.0 / 1600.0).abs() < 1e-6);

        // A 1280x720 design canvas letterboxed into a 1000x1000 window: the horizontal axis is
        // squeezed relative to the vertical, so the aspect must NOT stay 720/1280.
        let clip = glam::Vec2::new(1.0, 0.5625);
        let boxed = lighting_aspect(1280, 720, clip);
        let design_only = 720.0f32 / 1280.0;
        assert!(
            (boxed - design_only).abs() > 1e-3,
            "aspect ignored the letterbox: {boxed} vs design-space {design_only}"
        );
        assert!((boxed - (720.0 * 1.0) / (1280.0 * 0.5625)).abs() < 1e-6);

        // Degenerate clip scale falls back rather than dividing by zero.
        let degenerate = lighting_aspect(800, 600, glam::Vec2::new(1.0, 0.0));
        assert!(degenerate.is_finite());
    }
}
