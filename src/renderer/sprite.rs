use std::cmp::Ordering;
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
use crate::nine_slice::{nine_slice_subquads, NineSlice};
use crate::renderer::texture::Texture;
use crate::renderer::ui::{DrawImage, DrawRect};
use crate::renderer::uv::{BlendUv, UvRect};
use crate::resources::CullConfig;

mod batch;
mod collect;
mod draw;
mod geometry;
mod material;
mod sort;
#[cfg(test)]
mod tests;
mod textures;
mod ui_primitives;

use geometry::{CameraUniform, InstanceRaw, UiInstanceRaw, Vertex, INDICES, VERTICES};
use material::MaterialRenderer;
use sort::{
    assign_instance_offsets, layer_matches_mask, sort_render_entries, SpriteRenderEntry,
    SpriteRenderKind,
};
use textures::TextureCache;

// ─── Frame context ───────────────────────────────────────────────────────────
/// Bundle of wgpu handles required for a single render-pass call.
///
/// Groups the five recurring arguments `device`/`queue`/`view`/`format`/`encoder` so they
/// don't have to be listed individually on every call.
/// Constructed by reference immediately before the call (no ownership taken).
/// `format` is the [`wgpu::TextureFormat`] of `view`; [`RenderPlugin`](crate::RenderPlugin)
/// implementations use it when creating their own render pipelines.
pub struct FrameContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub view: &'a wgpu::TextureView,
    /// Texture format of `view`. Use this when building a [`wgpu::RenderPipeline`]
    /// that targets the same render pass.
    pub format: wgpu::TextureFormat,
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
    // For UI screen-space rendering
    ui_pipeline: wgpu::RenderPipeline,
    ui_camera_buf: wgpu::Buffer,
    ui_camera_bind_group: wgpu::BindGroup,
    ui_instance_buf: wgpu::Buffer,
    ui_instance_capacity: usize,
    // Per-frame scratch Vecs promoted to fields to avoid per-frame heap allocs.
    // Each collection is cleared at the top of `render()` and refilled.
    draw_entries: Vec<SpriteRenderEntry>,
    sprite_instances_scratch: Vec<InstanceRaw>,
    /// Scratch: AtlasSprite query results (entity, index, color, atlas handle).
    /// Promoted from a per-frame local `Vec` — cleared and refilled each frame.
    atlas_entries_scratch: Vec<(
        crate::ecs::Entity,
        u32,
        crate::color::Color,
        crate::asset::Handle<crate::atlas::TextureAtlas>,
    )>,
    // Sub-structs owning texture caching and material pipeline concerns.
    texture_cache: TextureCache,
    material: MaterialRenderer,
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

        // ── Texture cache (owns texture_layout + white_texture) ────────────────
        let texture_cache = TextureCache::new(device, queue);

        // ── Render pipeline ─────────────────────────────────────────────────
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&texture_cache.texture_layout)],
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
                buffers: &[vertex_layout.clone(), InstanceRaw::layout()],
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

        // ── UI pipeline (own shader: rounded-rect SDF; reuses the sprite bind-group
        //    layouts so it can share the white-texture / image bind groups) ──────────
        let ui_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ui shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/ui.wgsl").into()),
        });
        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ui pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout, UiInstanceRaw::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
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
            size: (ui_capacity * std::mem::size_of::<UiInstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── MaterialRenderer (takes ownership of camera_layout + shader) ───
        let material = MaterialRenderer::new(device, shader, camera_layout, format);

        Self {
            pipeline,
            vertex_buf,
            index_buf,
            instance_buf,
            instance_capacity: capacity,
            camera_buf,
            camera_bind_group,
            ui_pipeline,
            ui_camera_buf,
            ui_camera_bind_group,
            ui_instance_buf,
            ui_instance_capacity: ui_capacity,
            draw_entries: Vec::new(),
            sprite_instances_scratch: Vec::new(),
            atlas_entries_scratch: Vec::new(),
            texture_cache,
            material,
        }
    }

    /// Borrow the GPU texture view for a cached image/atlas by its asset path.
    ///
    /// Returns `None` if the path has not been uploaded yet. Used by the editor to
    /// hand an atlas texture to egui (`register_native_texture`) for the tile-paint
    /// swatch palette.
    pub fn texture_view(&self, path: &str) -> Option<&wgpu::TextureView> {
        self.texture_cache.texture_cache.get(path).map(|t| &t.view)
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

        // ── Collect → batch → draw (each phase extracted to its own submodule) ──
        // The phases share `draw_entries` / `stats` and the cull closures; they are
        // sequenced here and behave exactly as the prior inline body.
        self.collect_draw_entries(world, layer_mask, &is_visible, &is_above_lod, &mut stats);
        self.batch_and_upload(device, queue, &mut stats);
        self.record_draw_pass(encoder, view, &mut stats);

        // Prevent GPU params buffer leaks for ShaderMaterial entities: remove
        // buffers/bind-groups for entities that are no longer alive (despawned or
        // had their ShaderMaterial removed). params_buffers previously only grew
        // because nothing ever removed from it.
        self.material
            .params_buffers
            .retain(|e, _| self.material.live_material_entities_scratch.contains(e));

        stats
    }
}
