use std::collections::HashMap;

use super::*;

pub(crate) struct MaterialRenderer {
    pub(crate) sprite_shader: wgpu::ShaderModule,
    pub(crate) camera_layout: wgpu::BindGroupLayout,
    pub(crate) surface_format: wgpu::TextureFormat,
    pub(crate) params_layout: wgpu::BindGroupLayout,
    pub(crate) mat_instance_buf: wgpu::Buffer,
    pub(crate) mat_instance_capacity: usize,
    pub(crate) custom_pipelines: HashMap<u64, wgpu::RenderPipeline>,
    pub(crate) params_buffers: HashMap<crate::ecs::Entity, (wgpu::Buffer, wgpu::BindGroup)>,
    pub(crate) material_instances_scratch: Vec<InstanceRaw>,
    pub(crate) live_material_entities_scratch: std::collections::HashSet<crate::ecs::Entity>,
    pub(crate) seen_new_hashes_scratch: std::collections::HashSet<u64>,
}

impl MaterialRenderer {
    pub(crate) fn new(
        device: &wgpu::Device,
        sprite_shader: wgpu::ShaderModule,
        camera_layout: wgpu::BindGroupLayout,
        surface_format: wgpu::TextureFormat,
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
            surface_format,
            params_layout,
            mat_instance_buf,
            mat_instance_capacity: mat_capacity,
            custom_pipelines: HashMap::new(),
            params_buffers: HashMap::new(),
            material_instances_scratch: Vec::new(),
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
                    format: self.surface_format,
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
        self.custom_pipelines.insert(hash, pipeline);
    }
}

impl SpriteRenderer {
    pub(super) fn compile_material_pipeline(
        &mut self,
        device: &wgpu::Device,
        hash: u64,
        frag_source: &str,
    ) {
        let SpriteRenderer {
            material,
            texture_cache,
            ..
        } = self;
        material.compile_pipeline(device, &texture_cache.texture_layout, hash, frag_source);
    }
}
