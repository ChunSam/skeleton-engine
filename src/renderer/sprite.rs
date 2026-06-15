use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use wgpu::util::DeviceExt;

use crate::asset::AssetServer;
use crate::atlas::AtlasSprite;
use crate::camera::Camera;
use crate::components::{Sprite, Transform};
use crate::ecs::World;
use crate::hierarchy::GlobalTransform;
use crate::material::ShaderMaterial;
use crate::renderer::texture::Texture;
use crate::renderer::ui::{DrawImage, DrawRect};
use crate::renderer::uv::{BlendUv, UvRect};
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
// ─── Frame context ───────────────────────────────────────────────────────────
/// Bundle of wgpu handles required for a single render-pass call.
///
/// Groups the four recurring arguments `device`/`queue`/`view`/`encoder` so they
/// don't have to be listed individually on every call.
/// Constructed by reference immediately before the call (no ownership taken).
pub struct FrameContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    pub encoder: &'a mut wgpu::CommandEncoder,
}

// ─── Sprite renderer ─────────────────────────────────────────────────────────
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
    // For UI screen-space rendering
    ui_camera_buf: wgpu::Buffer,
    ui_camera_bind_group: wgpu::BindGroup,
    ui_instance_buf: wgpu::Buffer,
    ui_instance_capacity: usize,
    // ── For ShaderMaterial custom rendering ────────────────────────────────────────
    sprite_shader: wgpu::ShaderModule,
    camera_layout: wgpu::BindGroupLayout,
    surface_format: wgpu::TextureFormat,
    params_layout: wgpu::BindGroupLayout,
    mat_instance_buf: wgpu::Buffer,
    mat_instance_capacity: usize,
    custom_pipelines: HashMap<u64, wgpu::RenderPipeline>,
    params_buffers: HashMap<crate::ecs::Entity, (wgpu::Buffer, wgpu::BindGroup)>,
    /// RenderTarget bind_group cache (key = RenderTarget name)
    rt_cache: HashMap<String, Arc<wgpu::BindGroup>>,
    // ── Per-frame scratch Vecs promoted to fields to avoid per-frame heap allocs ──
    // Each vec is cleared at the top of `render()` and refilled. The backing memory
    // is reused across frames so no heap allocation occurs on steady-state frames
    // (same pattern as instance_capacity above).
    draw_entries: Vec<SpriteRenderEntry>,
    sprite_instances_scratch: Vec<InstanceRaw>,
    material_instances_scratch: Vec<InstanceRaw>,
}

/// Culls a sprite against frustum + LOD thresholds and — if visible — appends a
/// [`SpriteRenderEntry`] to `draw_entries`.
///
/// Returns `true` if the entry was pushed (visible), `false` if culled.
/// Increments `stats.sprites_culled` on cull and `next_order` on push.
#[allow(clippy::too_many_arguments)]
fn push_sprite_if_visible(
    pos: glam::Vec2,
    scale: glam::Vec2,
    rotation: f32,
    z: f32,
    layer: i32,
    tex_key: std::sync::Arc<str>,
    instance: InstanceRaw,
    is_visible: &dyn Fn(glam::Vec2, glam::Vec2, f32) -> bool,
    is_above_lod: &dyn Fn(glam::Vec2) -> bool,
    stats: &mut crate::resources::RenderStats,
    next_order: &mut usize,
    draw_entries: &mut Vec<SpriteRenderEntry>,
) -> bool {
    if !is_visible(pos, scale, rotation) {
        stats.sprites_culled += 1;
        return false;
    }
    if !is_above_lod(scale) {
        stats.sprites_culled += 1;
        return false;
    }
    let order = *next_order;
    *next_order += 1;
    draw_entries.push(SpriteRenderEntry::sprite(
        layer, z, order, tex_key, instance,
    ));
    true
}

impl SpriteRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        // ── Load shader (compile-time embed) ───────────────────────────────────
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/sprite.wgsl").into()),
        });

        // ── Camera uniform buffer ──────────────────────────────────────────────
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

        // ── Texture layout + default white texture ─────────────────────────────
        let texture_layout = Texture::bind_group_layout(device);
        let white_texture = Texture::white(device, queue, &texture_layout);

        // ── Render pipeline ─────────────────────────────────────────────────
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_layout)],
            immediate_size: 0,
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
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout, InstanceRaw::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
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
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            // Pipeline cache field added in wgpu 22 — None disables caching
            cache: None,
        });

        // ── Static vertex and index buffers ────────────────────────────────────────
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

        // ── Initial instance buffer (pre-allocated for 128 instances) ───────────────────────────
        let capacity = 128;
        let instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            size: (capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── UI screen-space camera buffer + bind group ──────────────────────
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

        // ── UI instance buffer (pre-allocated for 64 instances) ─────────────────────────────
        let ui_capacity = 64;
        let ui_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ui instance buffer"),
            size: (ui_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── ShaderMaterial: params uniform layout (@group(2)) ──────────────
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

        // ── ShaderMaterial: instance buffer (dynamically reallocated as material entity count grows) ──
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
            draw_entries: Vec::new(),
            sprite_instances_scratch: Vec::new(),
            material_instances_scratch: Vec::new(),
        }
    }

    /// Borrow the GPU texture view for a cached image/atlas by its asset path.
    ///
    /// Returns `None` if the path has not been uploaded yet. Used by the editor to
    /// hand an atlas texture to egui (`register_native_texture`) for the tile-paint
    /// swatch palette.
    pub fn texture_view(&self, path: &str) -> Option<&wgpu::TextureView> {
        self.texture_cache.get(path).map(|t| &t.view)
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
        // ── Camera: read from the ECS resource and compute view_proj ───
        let fallback = Camera::default();
        let camera = world.resource::<Camera>().unwrap_or(&fallback);
        let view_proj = camera.view_proj(width as f32, height as f32);
        let cam = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.camera_buf, 0, bytemuck::bytes_of(&cam));

        // ── Culling config + visible region ──────────────────────────────────────────
        let cull = world.resource::<CullConfig>().copied().unwrap_or_default();
        let (vmin, vmax) = camera.visible_rect(width as f32, height as f32);

        // Conservative AABB intersection helper that accounts for rotation.
        // Computes the half-width of the rotated AABB using |cos θ|·w/2 + |sin θ|·h/2.
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

        // ── Collect all sprites into (layer, z) globally-sorted entries ──────
        // If GlobalTransform is present, use the hierarchy-composed result; otherwise fall back to Transform.
        // Entities without RenderLayer(i32) are treated as layer 0.
        //
        // draw_entries is a field on SpriteRenderer so its backing Vec<> allocation is
        // reused across frames (no per-frame heap alloc on steady state).
        self.draw_entries.clear();
        let mut next_order = 0usize;
        for (entity, sprite) in world.query::<Sprite>() {
            if world.get::<ShaderMaterial>(entity).is_some() {
                continue;
            }

            let uv = world.get::<UvRect>(entity).copied().unwrap_or(UvRect::FULL);
            // During a crossfade, pass the "to" frame + progress for the shader lerp.
            let blend = world.get::<BlendUv>(entity).copied();
            let make_instance = |model: [[f32; 4]; 4]| match blend {
                Some(b) if b.weight > 0.0 => {
                    InstanceRaw::blended(model, sprite.color.to_array(), uv, b.to, b.weight)
                }
                _ => InstanceRaw::single(model, sprite.color.to_array(), uv),
            };
            let layer = world
                .get::<crate::components::RenderLayer>(entity)
                .map(|l| l.0)
                .unwrap_or(0);
            if !layer_matches_mask(layer, layer_mask) {
                continue;
            }
            // Prefer image_handle path if present; fall back to the texture path.
            // path_arc() is an O(1) refcount bump; Arc::from(path()) would copy the string.
            let tex_key: Arc<str> = sprite
                .image_handle
                .as_ref()
                .map(|h| h.path_arc())
                .or_else(|| sprite.texture.clone())
                .unwrap_or_else(|| Arc::from(""));
            if let Some(gt) = world.get::<GlobalTransform>(entity) {
                push_sprite_if_visible(
                    gt.position,
                    gt.scale,
                    gt.rotation,
                    gt.z,
                    layer,
                    tex_key,
                    make_instance(gt.to_matrix().to_cols_array_2d()),
                    &is_visible,
                    &is_above_lod,
                    &mut stats,
                    &mut next_order,
                    &mut self.draw_entries,
                );
            } else if let Some(transform) = world.get::<Transform>(entity) {
                push_sprite_if_visible(
                    transform.position,
                    transform.scale,
                    transform.rotation,
                    transform.z,
                    layer,
                    tex_key,
                    make_instance(transform.to_matrix().to_cols_array_2d()),
                    &is_visible,
                    &is_above_lod,
                    &mut stats,
                    &mut next_order,
                    &mut self.draw_entries,
                );
            }
        }
        // ── Collect AtlasSprites: first collect (index, color, atlas handle) ──
        // Break the query iterator borrow before reading AssetServer and GlobalTransform.
        let atlas_entries: Vec<(
            crate::ecs::Entity,
            u32,
            crate::color::Color,
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
                        // O(1) refcount bump; Arc::from(path()) would copy the string bytes.
                        let tex_key: Arc<str> = atlas.texture_path_arc();
                        let layer = world
                            .get::<crate::components::RenderLayer>(*entity)
                            .map(|l| l.0)
                            .unwrap_or(0);
                        if !layer_matches_mask(layer, layer_mask) {
                            continue;
                        }
                        if let Some(gt) = world.get::<GlobalTransform>(*entity) {
                            push_sprite_if_visible(
                                gt.position,
                                gt.scale,
                                gt.rotation,
                                gt.z,
                                layer,
                                tex_key,
                                InstanceRaw::single(
                                    gt.to_matrix().to_cols_array_2d(),
                                    color.to_array(),
                                    uv,
                                ),
                                &is_visible,
                                &is_above_lod,
                                &mut stats,
                                &mut next_order,
                                &mut self.draw_entries,
                            );
                        } else if let Some(tr) = world.get::<Transform>(*entity) {
                            push_sprite_if_visible(
                                tr.position,
                                tr.scale,
                                tr.rotation,
                                tr.z,
                                layer,
                                tex_key,
                                InstanceRaw::single(
                                    tr.to_matrix().to_cols_array_2d(),
                                    color.to_array(),
                                    uv,
                                ),
                                &is_visible,
                                &is_above_lod,
                                &mut stats,
                                &mut next_order,
                                &mut self.draw_entries,
                            );
                        }
                    }
                }
            }
        }

        // ── Collect ShaderMaterials: merge into the same (layer, z) stream as regular sprites.
        // No source clone here — the clone decision happens below, after the layer/cull
        // filters, so the *first surviving* entity of a new hash carries the source
        // (cloning per-entity at query time would clone once per entity sharing a new
        // material; deduping at query time could hand the source to an entity that is
        // then culled, leaving the pipeline uncompiled).
        let mat_ids: Vec<(crate::ecs::Entity, u64, [f32; 4])> = world
            .query::<ShaderMaterial>()
            .map(|(e, mat)| (e, mat.source_hash(), mat.params))
            .collect();

        // Set of live entities (= currently holding a ShaderMaterial). Populated
        // from the full world-query result regardless of culling, so retaining
        // params_buffers to this set at frame end removes GPU buffers only for
        // despawned or material-removed entities.
        let live_material_entities: std::collections::HashSet<crate::ecs::Entity> =
            mat_ids.iter().map(|(e, ..)| *e).collect();

        // Hashes whose source has already been claimed by a surviving entity this
        // call — keeps frag_source clones to at most one per new pipeline.
        let mut seen_new_hashes: std::collections::HashSet<u64> = std::collections::HashSet::new();
        for (entity, hash, params) in mat_ids {
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
            let tex_key: Option<Arc<str>> = sprite
                .image_handle
                .as_ref()
                .map(|h| h.path_arc())
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

            // Clone the WGSL source only for the first surviving entity of a
            // not-yet-compiled hash; every other entry carries an empty string.
            let frag_source =
                if !self.custom_pipelines.contains_key(&hash) && seen_new_hashes.insert(hash) {
                    world
                        .get::<ShaderMaterial>(entity)
                        .map(|m| m.frag_source().to_owned())
                        .unwrap_or_default()
                } else {
                    String::new()
                };

            self.draw_entries.push(SpriteRenderEntry::material(
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

        sort_render_entries(&mut self.draw_entries);
        stats.sprites_rendered = self.draw_entries.len() as u32;

        // assign_instance_offsets fills the pre-allocated scratch vecs on self,
        // avoiding per-frame heap allocation in steady state.
        assign_instance_offsets(
            &mut self.draw_entries,
            &mut self.sprite_instances_scratch,
            &mut self.material_instances_scratch,
        );

        if !self.sprite_instances_scratch.is_empty() {
            if self.sprite_instances_scratch.len() > self.instance_capacity {
                self.instance_capacity = self.sprite_instances_scratch.len().next_power_of_two();
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
                bytemuck::cast_slice(&self.sprite_instances_scratch),
            );
        }

        if !self.material_instances_scratch.is_empty() {
            if self.material_instances_scratch.len() > self.mat_instance_capacity {
                self.mat_instance_capacity =
                    self.material_instances_scratch.len().next_power_of_two();
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
                bytemuck::cast_slice(&self.material_instances_scratch),
            );

            // Compile pipelines for new material hashes.
            // Collect (hash, frag_source) first so no live borrow from self.draw_entries
            // conflicts with the &mut self call to compile_material_pipeline.
            let to_compile: Vec<(u64, String)> = self
                .draw_entries
                .iter()
                .filter_map(|e| {
                    if let SpriteRenderKind::Material {
                        hash, frag_source, ..
                    } = &e.kind
                    {
                        if !frag_source.is_empty() && !self.custom_pipelines.contains_key(hash) {
                            return Some((*hash, frag_source.clone()));
                        }
                    }
                    None
                })
                .collect();
            for (hash, frag_source) in to_compile {
                self.compile_material_pipeline(device, hash, &frag_source);
            }

            // Write material params uniforms.
            // Collect (entity, params) first so self.draw_entries borrow ends
            // before we mutably access self.params_buffers.
            let mat_params: Vec<(crate::ecs::Entity, [f32; 4])> = self
                .draw_entries
                .iter()
                .filter_map(|e| {
                    if let SpriteRenderKind::Material { entity, params, .. } = &e.kind {
                        Some((*entity, *params))
                    } else {
                        None
                    }
                })
                .collect();
            for (eid, params) in mat_params {
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
                queue.write_buffer(buf, 0, bytemuck::cast_slice(&params));
            }
        }

        let instance_size = std::mem::size_of::<InstanceRaw>() as u64;

        // Open ONE render pass for the entire pre-sorted entry stream. Each
        // texture-run and each material then issues its own
        // set_pipeline/set_bind_group/draw_indexed into this single pass.
        // Previously every texture-run AND every material opened its own
        // `begin_render_pass`, forcing a full attachment load+store per run.
        // (Skip opening a pass entirely when there is nothing to draw.)
        if !self.draw_entries.is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sprite pass"),
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

            let mut i = 0usize;
            while i < self.draw_entries.len() {
                match &self.draw_entries[i].kind {
                    SpriteRenderKind::Sprite {
                        texture_key,
                        instance_offset,
                        ..
                    } => {
                        let run_key: &str = texture_key;
                        let run_start_offset = *instance_offset;
                        let mut run_len = 1usize;
                        i += 1;
                        while i < self.draw_entries.len() {
                            match &self.draw_entries[i].kind {
                                SpriteRenderKind::Sprite {
                                    texture_key,
                                    instance_offset,
                                    ..
                                } if texture_key.as_ref() == run_key
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
                            .as_deref()
                            .map(|k| self.bind_group_for_texture_key(Some(k)))
                            .unwrap_or(&self.white_texture.bind_group);
                        let (_, params_bg) = &self.params_buffers[entity];

                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &self.camera_bind_group, &[]);
                        pass.set_bind_group(1, tex_bg, &[]);
                        pass.set_bind_group(2, params_bg, &[]);
                        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                        pass.set_vertex_buffer(
                            1,
                            self.mat_instance_buf.slice(byte_start..byte_end),
                        );
                        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
                        stats.draw_calls += 1;
                        i += 1;
                    }
                }
            }
        }

        // Prevent GPU params buffer leaks for ShaderMaterial entities: remove
        // buffers/bind-groups for entities that are no longer alive (despawned or
        // had their ShaderMaterial removed). params_buffers previously only grew
        // because nothing ever removed from it.
        self.params_buffers
            .retain(|e, _| live_material_entities.contains(e));

        stats
    }
}
