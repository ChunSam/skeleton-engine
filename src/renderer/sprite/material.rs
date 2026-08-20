use std::collections::HashMap;

use super::*;

pub(crate) struct MaterialRenderer {
    pub(crate) sprite_shader: wgpu::ShaderModule,
    pub(crate) camera_layout: wgpu::BindGroupLayout,
    pub(crate) params_layout: wgpu::BindGroupLayout,
    pub(crate) mat_instance_buf: wgpu::Buffer,
    pub(crate) mat_instance_capacity: usize,
    /// Custom material pipelines keyed by `(frag-source hash, target color format)`. The format is
    /// part of the key so a material renders correctly into a non-surface target (an HDR / linear
    /// offscreen render target, or the HDR post-process intermediate) — each distinct target format
    /// gets its own pipeline, built lazily on first use.
    ///
    /// **Never evicted, deliberately.** The 2026-08-19 render review filed that as a leak; reading
    /// it says otherwise. The key is a *source* hash, so this grows with the number of distinct
    /// shader sources a session compiles, not with entities, time, or scene loads — a game with
    /// twelve materials holds twelve pipelines forever, and holding them is the point across a
    /// scene reset (the renderer is session state, and rebuilding one is a shader recompile).
    /// Unbounded only for a game that *generates* shader source at runtime, which nothing in this
    /// engine does. An LRU here would trade a bounded, small footprint for recompile hitches — the
    /// same trade v0.153.2 rejected when it declined to tear down the GPU-particle renderer.
    pub(crate) custom_pipelines: HashMap<(u64, wgpu::TextureFormat), wgpu::RenderPipeline>,
    pub(crate) params_buffers: HashMap<crate::ecs::Entity, (wgpu::Buffer, wgpu::BindGroup)>,
    pub(crate) material_instances_scratch: Vec<InstanceRaw>,
    /// The `(entity, source hash, params)` of every material sprite that is actually drawn this
    /// frame — the *drawn* half of the pair `split_material_entities` produces (the other half is
    /// `live_material_entities_scratch`, which also counts hidden ones).
    ///
    /// A scratch field, not a local, for the usual reason: it is refilled every frame from a
    /// `clear()`ed buffer, so a scene with material sprites stopped paying one `Vec` growth per
    /// frame for it. Free either way in a scene with no `ShaderMaterial` — an empty collect never
    /// pushes, and `Vec::new()` does not allocate.
    pub(crate) drawn_material_entities_scratch: Vec<(crate::ecs::Entity, u64, [f32; 4])>,
    pub(crate) live_material_entities_scratch: std::collections::HashSet<crate::ecs::Entity>,
    pub(crate) seen_new_hashes_scratch: std::collections::HashSet<u64>,
}

impl MaterialRenderer {
    pub(crate) fn new(
        device: &wgpu::Device,
        sprite_shader: wgpu::ShaderModule,
        camera_layout: wgpu::BindGroupLayout,
    ) -> Self {
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

        let mat_capacity = 16usize;
        let mat_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("material instance buffer"),
            size: (mat_capacity * std::mem::size_of::<InstanceRaw>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            sprite_shader,
            camera_layout,
            params_layout,
            mat_instance_buf,
            mat_instance_capacity: mat_capacity,
            custom_pipelines: HashMap::new(),
            params_buffers: HashMap::new(),
            material_instances_scratch: Vec::new(),
            drawn_material_entities_scratch: Vec::new(),
            live_material_entities_scratch: std::collections::HashSet::new(),
            seen_new_hashes_scratch: std::collections::HashSet::new(),
        }
    }

    pub(super) fn compile_pipeline(
        &mut self,
        device: &wgpu::Device,
        texture_layout: &wgpu::BindGroupLayout,
        hash: u64,
        frag_source: &str,
        target_format: wgpu::TextureFormat,
    ) {
        let frag_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("custom material frag"),
            source: wgpu::ShaderSource::Wgsl(frag_source.into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("material pipeline layout"),
            bind_group_layouts: &[
                Some(&self.camera_layout),
                Some(texture_layout),
                Some(&self.params_layout),
            ],
            immediate_size: 0,
        });
        let vertex_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("material pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &self.sprite_shader,
                entry_point: Some("vs_main"),
                buffers: &[vertex_layout, InstanceRaw::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &frag_module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });
        self.custom_pipelines
            .insert((hash, target_format), pipeline);
    }
}

impl SpriteRenderer {
    pub(super) fn compile_material_pipeline(
        &mut self,
        device: &wgpu::Device,
        hash: u64,
        frag_source: &str,
        target_format: wgpu::TextureFormat,
    ) {
        let SpriteRenderer {
            material,
            texture_cache,
            ..
        } = self;
        material.compile_pipeline(
            device,
            &texture_cache.texture_layout,
            hash,
            frag_source,
            target_format,
        );
    }
}
