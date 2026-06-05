use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::components::{PointLight, Transform};
use crate::ecs::World;
use crate::resources::AmbientLight;

// ─── GPU 구조체 ───────────────────────────────────────────────────────────────

/// GPU로 전송되는 단일 포인트 라이트 데이터 (32바이트).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct GpuLightData {
    pub position_ndc: [f32; 2],
    pub radius_ndc: f32,
    pub intensity: f32,
    pub color: [f32; 3],
    pub light_height: f32, // virtual Z height for flat-normal lighting (0.05~1.0 typical)
}

/// GPU 유니폼 전체 (544바이트).
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

// ─── WGSL 셰이더 ──────────────────────────────────────────────────────────────

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

/// 씬 텍스처를 입력받아 포인트 라이트를 적용하는 풀스크린 패스.
///
/// `AmbientLight` 리소스가 World에 있을 때 `App`이 자동으로 생성·실행한다.
pub struct LightingRenderer {
    /// 노멀 버퍼 텍스처 (뷰포트와 같은 크기, Rgba8Unorm).
    normal_texture: wgpu::Texture,
    /// 노멀 버퍼 텍스처 뷰 (라이팅 셰이더 binding 3).
    pub normal_view: wgpu::TextureView,
    /// 현재 출력 텍스처 너비.
    pub width: u32,
    /// 현재 출력 텍스처 높이.
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
    let radius_ndc = radius * camera.zoom / viewport_w;
    ([ndc_x, ndc_y], radius_ndc)
}

/// 라이팅 패스가 한 번에 처리할 수 있는 최대 광원 수 (GPU 유니폼 배열 크기와 일치).
const MAX_LIGHTS: usize = 16;

/// 카메라에서 가까운 순으로 최대 `MAX_LIGHTS`개 광원의 인덱스를 고른다.
///
/// 광원이 캡을 넘을 때 먼 광원이 조용히 누락되던 기존 동작(쿼리 순서로 앞 16개)을
/// 대체한다. 반환 순서는 정렬되어 있지 않다 (가법 라이팅이라 16개 내부 순서는 무관).
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
    /// 새 `LightingRenderer`를 생성한다.
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
                // binding 0: 씬 텍스처
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
                // binding 1: 샘플러
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                // binding 2: 유니폼 버퍼
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

    /// 창 크기 변경 시 노멀 버퍼를 재생성한다.
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

    /// 매 프레임 노멀 버퍼를 평면 노멀 색상(0.5, 0.5, 1.0, 1.0)으로 초기화한다.
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

    /// ECS World에서 라이트 데이터를 수집하고 유니폼 버퍼를 갱신한다.
    ///
    /// 광원이 `MAX_LIGHTS`(16)를 넘으면 카메라에서 가까운 16개만 전송한다
    /// (먼 광원을 임의로 누락시키지 않는다).
    pub fn update(&self, queue: &wgpu::Queue, world: &World, vp_w: u32, vp_h: u32) {
        let ambient = world
            .resource::<AmbientLight>()
            .copied()
            .unwrap_or_default();

        let camera = world.resource::<Camera>().copied().unwrap_or_default();

        // 모든 포인트 라이트를 (월드 위치, 라이트 데이터)로 수집한다.
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

    /// 씬 텍스처에 라이팅을 적용해 `output_view`에 출력한다.
    pub fn run_pass(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
    ) {
        // 씬 텍스처와 노멀 버퍼는 매 프레임 바뀔 수 있으므로 bind group을 즉석 생성한다.
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

    // ── 내부 헬퍼 ─────────────────────────────────────────────────────────────

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

// ─── 단위 테스트 ───────────────────────────────────────────────────────────────

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

    #[test]
    fn select_nearest_lights_returns_all_under_cap() {
        let positions: Vec<glam::Vec2> = (0..5).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO);
        assert_eq!(selected.len(), 5);
    }

    #[test]
    fn select_nearest_lights_keeps_closest_to_camera() {
        // 18개 광원을 x = 0,1,..,17 에 배치 → 카메라(원점)에서 인덱스가 작을수록 가깝다.
        let positions: Vec<glam::Vec2> = (0..18).map(|i| glam::Vec2::new(i as f32, 0.0)).collect();
        let selected = select_nearest_lights(&positions, glam::Vec2::ZERO);

        assert_eq!(selected.len(), MAX_LIGHTS);
        // 가장 가까운 16개(인덱스 0..=15)만 선택되고, 가장 먼 16·17은 빠져야 한다.
        let mut sorted = selected.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, (0..MAX_LIGHTS).collect::<Vec<usize>>());
        assert!(!selected.contains(&16));
        assert!(!selected.contains(&17));
    }
}
