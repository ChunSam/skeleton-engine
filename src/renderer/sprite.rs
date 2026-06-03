use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

use crate::animation::player::UvRect;
use crate::asset::AssetServer;
use crate::atlas::AtlasSprite;
use crate::camera::Camera;
use crate::components::{Sprite, Transform};
use crate::ecs::World;
use crate::hierarchy::GlobalTransform;
use crate::material::ShaderMaterial;
use crate::renderer::texture::Texture;
use crate::renderer::ui::{DrawImage, DrawRect};
use crate::resources::CullConfig;

mod geometry;
mod material;
mod sort;
#[cfg(test)]
mod tests;
mod textures;
mod ui_primitives;

use geometry::{CameraUniform, InstanceRaw, Vertex, INDICES, VERTICES};
use sort::{
    assign_instance_offsets, layer_matches_mask, sort_render_entries, SpriteRenderEntry,
    SpriteRenderKind,
};
// ─── 프레임 컨텍스트 ───────────────────────────────────────────────────────────
/// 한 렌더 패스 호출에 필요한 wgpu 핸들 묶음.
///
/// `device`/`queue`/`view`/`encoder` 4개를 매 호출 인자로 나열하지 않도록 묶는다.
/// 호출 직전 참조로 즉석 구성한다 (소유하지 않음).
pub struct FrameContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

// ─── 스프라이트 렌더러 ─────────────────────────────────────────────────────────
pub struct SpriteRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buf: wgpu::Buffer,
    index_buf: wgpu::Buffer,
    instance_buf: wgpu::Buffer,
    instance_capacity: usize,
    camera_buf: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    texture_layout: wgpu::BindGroupLayout,
    white_texture: Texture,
    texture_cache: HashMap<String, Arc<Texture>>,
    // UI screen-space 렌더링용
    ui_camera_buf: wgpu::Buffer,
    ui_camera_bind_group: wgpu::BindGroup,
    ui_instance_buf: wgpu::Buffer,
    ui_instance_capacity: usize,
    // ── ShaderMaterial 커스텀 렌더링용 ────────────────────────────────────────
    sprite_shader: wgpu::ShaderModule,
    camera_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    params_layout: wgpu::BindGroupLayout,
    mat_instance_buf: wgpu::Buffer,
    mat_instance_capacity: usize,
    custom_pipelines: HashMap<u64, wgpu::RenderPipeline>,
    params_buffers: HashMap<crate::ecs::Entity, (wgpu::Buffer, wgpu::BindGroup)>,
    /// RenderTarget bind_group 캐시 (키 = RenderTarget 이름)
    rt_cache: HashMap<String, Arc<wgpu::BindGroup>>,
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        // ── 셰이더 로드 (컴파일 타임 임베딩) ───────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
        });

        // ── 카메라 유니폼 버퍼 ──────────────────────────────────────────────
        let camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("camera layout"),
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
            label: Some("camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buf.as_entire_binding(),
            }],
        });

        // ── 텍스처 레이아웃 + 기본 흰색 텍스처 ─────────────────────────────
        let texture_layout = Texture::bind_group_layout(device);
        let white_texture = Texture::white(device, queue, &texture_layout);

        // ── 렌더 파이프라인 ─────────────────────────────────────────────────
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[&camera_layout, &texture_layout],
            push_constant_ranges: &[],
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprite pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[vertex_layout, InstanceRaw::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            // wgpu 22 에서 추가된 파이프라인 캐시 필드 — None 이면 캐시 비활성화
            cache: None,
        });

        // ── 정적 버텍스·인덱스 버퍼 ────────────────────────────────────────
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad vertex"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad index"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });

        // ── 초기 인스턴스 버퍼 (128개 분량 예약) ───────────────────────────
        let capacity = 128;
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            size: (capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── UI screen-space 카메라 버퍼 + 바인드 그룹 ──────────────────────
        let ui_camera_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui camera uniform"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ui_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ui camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: ui_camera_buf.as_entire_binding(),
            }],
        });

        // ── UI 인스턴스 버퍼 (64개 분량 예약) ─────────────────────────────
        let ui_capacity = 64;
        let ui_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui instance buffer"),
            size: (ui_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── ShaderMaterial: params 유니폼 레이아웃 (@group(2)) ──────────────
        let params_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("material params layout"),
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

        // ── ShaderMaterial: 인스턴스 버퍼 (머티리얼 엔티티 수만큼 동적 재할당) ──
        let mat_capacity = 16usize;
        let mat_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material instance buffer"),
            size: (mat_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buf,
            index_buf,
            instance_buf,
            instance_capacity: capacity,
            camera_buf,
            camera_bind_group,
            texture_layout,
            white_texture,
            texture_cache: HashMap::new(),
            ui_camera_buf,
            ui_camera_bind_group,
            ui_instance_buf,
            ui_instance_capacity: ui_capacity,
            sprite_shader: shader,
            camera_layout,
            surface_format: format,
            params_layout,
            mat_instance_buf,
            mat_instance_capacity: mat_capacity,
            custom_pipelines: HashMap::new(),
            params_buffers: HashMap::new(),
            rt_cache: HashMap::new(),
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut FrameContext,
        world: &World,
        width: u32,
        height: u32,
        layer_mask: u32,
    ) -> crate::resources::RenderStats {
        let device = ctx.device;
        let queue = ctx.queue;
        let view = ctx.view;
        let encoder = &mut *ctx.encoder;
        let mut stats = crate::resources::RenderStats::default();
        // ── 카메라: ECS 리소스에서 Camera 를 읽어 view_proj 를 계산한다 ───
        let fallback = Camera::default();
        let camera = world.resource::<Camera>().unwrap_or(&fallback);
        let view_proj = camera.view_proj(width as f32, height as f32);
        let cam = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&cam));

        // ── 컬링 설정 + 가시 영역 ──────────────────────────────────────────
        let cull = world.resource::<CullConfig>().copied().unwrap_or_default();
        let (vmin, vmax) = camera.visible_rect(width as f32, height as f32);

        // 회전을 고려한 보수적 AABB 교차 판정 헬퍼.
        // |cos θ|·w/2 + |sin θ|·h/2 공식으로 회전 후 AABB 반폭을 계산한다.
        let is_visible = |pos: glam::Vec2, scale: glam::Vec2, rotation: f32| -> bool {
            if !cull.frustum_culling {
                return true;
            }
            let sin_r = rotation.sin().abs();
            let cos_r = rotation.cos().abs();
            let hw = cos_r * scale.x * 0.5 + sin_r * scale.y * 0.5;
            let hh = sin_r * scale.x * 0.5 + cos_r * scale.y * 0.5;
            pos.x + hw >= vmin.x
                && pos.x - hw <= vmax.x
                && pos.y + hh >= vmin.y
                && pos.y - hh <= vmax.y
        };

        let is_above_lod = |scale: glam::Vec2| -> bool {
            if cull.min_pixel_size <= 0.0 {
                return true;
            }
            let px_w = scale.x * camera.zoom;
            let px_h = scale.y * camera.zoom;
            px_w.min(px_h) >= cull.min_pixel_size
        };

        // ── 전체 스프라이트 수집: (layer, z) 전역 정렬용 엔트리 ──────
        // GlobalTransform이 있으면 계층 합성 결과를 사용하고, 없으면 Transform으로 fallback.
        // RenderLayer(i32)가 없으면 0 으로 취급한다.
        let mut draw_entries: Vec<SpriteRenderEntry> = Vec::new();
        let mut next_order = 0usize;
        for (entity, sprite) in world.query::<Sprite>() {
            if world.get::<ShaderMaterial>(entity).is_some() {
                continue;
            }

            let uv = world.get::<UvRect>(entity).copied().unwrap_or(UvRect::FULL);
            let layer = world
                .get::<crate::components::RenderLayer>(entity)
                .map(|l| l.0)
                .unwrap_or(0);
            if !layer_matches_mask(layer, layer_mask) {
                continue;
            }
            // image_handle이 있으면 그 경로를 우선 사용, 없으면 texture 경로 사용
            let tex_key = sprite
                .image_handle
                .as_ref()
                .map(|h| h.path().to_string())
                .or_else(|| sprite.texture.clone())
                .unwrap_or_default();
            if let Some(gt) = world.get::<GlobalTransform>(entity) {
                if !is_visible(gt.position, gt.scale, gt.rotation) {
                    stats.sprites_culled += 1;
                    continue;
                }
                if !is_above_lod(gt.scale) {
                    stats.sprites_culled += 1;
                    continue;
                }
                let order = next_order;
                next_order += 1;
                draw_entries.push(SpriteRenderEntry::sprite(
                    layer,
                    gt.z,
                    order,
                    tex_key,
                    InstanceRaw::from_global(gt, sprite, uv),
                ));
            } else if let Some(transform) = world.get::<Transform>(entity) {
                if !is_visible(transform.position, transform.scale, transform.rotation) {
                    stats.sprites_culled += 1;
                    continue;
                }
                if !is_above_lod(transform.scale) {
                    stats.sprites_culled += 1;
                    continue;
                }
                let order = next_order;
                next_order += 1;
                draw_entries.push(SpriteRenderEntry::sprite(
                    layer,
                    transform.z,
                    order,
                    tex_key,
                    InstanceRaw::from(transform, sprite, uv),
                ));
            }
        }
        // ── AtlasSprite 수집: (index, color, atlas handle) 을 먼저 collect ──
        // query 이터레이터 borrow를 끊은 뒤 AssetServer 와 GlobalTransform 을 읽는다.
        let atlas_entries: Vec<(
            crate::ecs::Entity,
            u32,
            [f32; 4],
            crate::asset::Handle<crate::atlas::TextureAtlas>,
        )> = world
            .query::<AtlasSprite>()
            .map(|(e, s)| (e, s.index, s.color, s.atlas.clone()))
            .collect();

        if !atlas_entries.is_empty() {
            if let Some(server) = world.resource::<AssetServer>() {
                for (entity, index, color, atlas_handle) in &atlas_entries {
                    if let Some(atlas) = server.get_atlas(atlas_handle) {
                        let uv = world
                            .get::<UvRect>(*entity)
                            .copied()
                            .unwrap_or_else(|| atlas.uv_rect(*index));
                        let tex_key = atlas.texture_path().to_string();
                        let layer = world
                            .get::<crate::components::RenderLayer>(*entity)
                            .map(|l| l.0)
                            .unwrap_or(0);
                        if !layer_matches_mask(layer, layer_mask) {
                            continue;
                        }
                        if let Some(gt) = world.get::<GlobalTransform>(*entity) {
                            if !is_visible(gt.position, gt.scale, gt.rotation) {
                                stats.sprites_culled += 1;
                                continue;
                            }
                            if !is_above_lod(gt.scale) {
                                stats.sprites_culled += 1;
                                continue;
                            }
                            let order = next_order;
                            next_order += 1;
                            draw_entries.push(SpriteRenderEntry::sprite(
                                layer,
                                gt.z,
                                order,
                                tex_key,
                                InstanceRaw {
                                    model: gt.to_matrix().to_cols_array_2d(),
                                    color: *color,
                                    uv_offset: [uv.u_offset, uv.v_offset],
                                    uv_size: [uv.u_size, uv.v_size],
                                },
                            ));
                        } else if let Some(tr) = world.get::<Transform>(*entity) {
                            if !is_visible(tr.position, tr.scale, tr.rotation) {
                                stats.sprites_culled += 1;
                                continue;
                            }
                            if !is_above_lod(tr.scale) {
                                stats.sprites_culled += 1;
                                continue;
                            }
                            let order = next_order;
                            next_order += 1;
                            draw_entries.push(SpriteRenderEntry::sprite(
                                layer,
                                tr.z,
                                order,
                                tex_key,
                                InstanceRaw {
                                    model: tr.to_matrix().to_cols_array_2d(),
                                    color: *color,
                                    uv_offset: [uv.u_offset, uv.v_offset],
                                    uv_size: [uv.u_size, uv.v_size],
                                },
                            ));
                        }
                    }
                }
            }
        }

        let mut entries = draw_entries;

        // ── ShaderMaterial 수집: 일반 스프라이트와 같은 (layer, z) 스트림에 합친다.
        let mat_ids: Vec<(crate::ecs::Entity, u64, String, [f32; 4])> = world
            .query::<ShaderMaterial>()
            .map(|(e, mat)| {
                let mut h = std::collections::hash_map::DefaultHasher::new();
                mat.frag_source.hash(&mut h);
                (e, h.finish(), mat.frag_source.clone(), mat.params)
            })
            .collect();

        for (entity, hash, frag_source, params) in mat_ids {
            let uv = world.get::<UvRect>(entity).copied().unwrap_or(UvRect::FULL);
            let sprite = match world.get::<Sprite>(entity) {
                Some(sprite) => sprite,
                None => continue,
            };
            let layer = world
                .get::<crate::components::RenderLayer>(entity)
                .map(|l| l.0)
                .unwrap_or(0);
            if !layer_matches_mask(layer, layer_mask) {
                continue;
            }
            let tex_key = sprite
                .image_handle
                .as_ref()
                .map(|h| h.path().to_string())
                .or_else(|| sprite.texture.clone());

            let (z, instance) = if let Some(gt) = world.get::<GlobalTransform>(entity) {
                if !is_visible(gt.position, gt.scale, gt.rotation) || !is_above_lod(gt.scale) {
                    stats.sprites_culled += 1;
                    continue;
                }
                (gt.z, InstanceRaw::from_global(gt, sprite, uv))
            } else if let Some(transform) = world.get::<Transform>(entity) {
                if !is_visible(transform.position, transform.scale, transform.rotation)
                    || !is_above_lod(transform.scale)
                {
                    stats.sprites_culled += 1;
                    continue;
                }
                (transform.z, InstanceRaw::from(transform, sprite, uv))
            } else {
                continue;
            };

            entries.push(SpriteRenderEntry::material(
                layer,
                z,
                next_order,
                entity,
                hash,
                frag_source,
                params,
                tex_key,
                instance,
            ));
            next_order += 1;
        }

        sort_render_entries(&mut entries);
        stats.sprites_rendered = entries.len() as u32;

        let (sprite_instances, material_instances) = assign_instance_offsets(&mut entries);

        if !sprite_instances.is_empty() {
            if sprite_instances.len() > self.instance_capacity {
                self.instance_capacity = sprite_instances.len().next_power_of_two();
                self.instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("instance buffer"),
                    size: (self.instance_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            queue.write_buffer(
                &self.instance_buf,
                0,
                bytemuck::cast_slice(&sprite_instances),
            );
        }

        if !material_instances.is_empty() {
            if material_instances.len() > self.mat_instance_capacity {
                self.mat_instance_capacity = material_instances.len().next_power_of_two();
                self.mat_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("material instance buffer"),
                    size: (self.mat_instance_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            queue.write_buffer(
                &self.mat_instance_buf,
                0,
                bytemuck::cast_slice(&material_instances),
            );

            let material_sources: Vec<(u64, String)> = entries
                .iter()
                .filter_map(|entry| match &entry.kind {
                    SpriteRenderKind::Material {
                        hash, frag_source, ..
                    } => Some((*hash, frag_source.clone())),
                    SpriteRenderKind::Sprite { .. } => None,
                })
                .collect();
            for (hash, frag_source) in material_sources {
                if !self.custom_pipelines.contains_key(&hash) {
                    self.compile_material_pipeline(device, hash, &frag_source);
                }
            }

            for entry in &entries {
                if let SpriteRenderKind::Material { entity, params, .. } = &entry.kind {
                    let eid = *entity;
                    if !self.params_buffers.contains_key(&eid) {
                        let buf = device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some("material params buf"),
                            size: 16,
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        });
                        let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("material params bind group"),
                            layout: &self.params_layout,
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: buf.as_entire_binding(),
                            }],
                        });
                        self.params_buffers.insert(eid, (buf, bg));
                    }
                    let (buf, _) = &self.params_buffers[&eid];
                    queue.write_buffer(buf, 0, bytemuck::cast_slice(params));
                }
            }
        }

        let instance_size = std::mem::size_of::<InstanceRaw>() as u64;
        let mut i = 0usize;
        while i < entries.len() {
            match &entries[i].kind {
                SpriteRenderKind::Sprite {
                    texture_key,
                    instance_offset,
                    ..
                } => {
                    let run_key = texture_key.as_str();
                    let run_start_offset = *instance_offset;
                    let mut run_len = 1usize;
                    i += 1;
                    while i < entries.len() {
                        match &entries[i].kind {
                            SpriteRenderKind::Sprite {
                                texture_key,
                                instance_offset,
                                ..
                            } if texture_key == run_key
                                && *instance_offset == run_start_offset + run_len =>
                            {
                                run_len += 1;
                                i += 1;
                            }
                            _ => break,
                        }
                    }

                    let byte_start = run_start_offset as u64 * instance_size;
                    let byte_end = byte_start + run_len as u64 * instance_size;
                    let bind_group = self.bind_group_for_texture_key(Some(run_key));

                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("sprite pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_bind_group(1, bind_group, &[]);
                    pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                    pass.set_vertex_buffer(1, self.instance_buf.slice(byte_start..byte_end));
                    pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..INDICES.len() as u32, 0, 0..run_len as u32);
                    stats.draw_calls += 1;
                }
                SpriteRenderKind::Material {
                    entity,
                    hash,
                    texture_key,
                    instance_offset,
                    ..
                } => {
                    let byte_start = *instance_offset as u64 * instance_size;
                    let byte_end = byte_start + instance_size;
                    let pipeline = &self.custom_pipelines[hash];
                    let tex_bg = texture_key
                        .as_ref()
                        .map(|k| self.bind_group_for_texture_key(Some(k)))
                        .unwrap_or(&self.white_texture.bind_group);
                    let (_, params_bg) = &self.params_buffers[entity];

                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("material pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        occlusion_query_set: None,
                        timestamp_writes: None,
                    });
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    pass.set_bind_group(1, tex_bg, &[]);
                    pass.set_bind_group(2, params_bg, &[]);
                    pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                    pass.set_vertex_buffer(1, self.mat_instance_buf.slice(byte_start..byte_end));
                    pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                    pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
                    stats.draw_calls += 1;
                    i += 1;
                }
            }
        }

        stats
    }
}
