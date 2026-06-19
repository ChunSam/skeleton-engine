use super::*;

impl SpriteRenderer {
    /// Sort the collected `draw_entries`, pack their instances into the GPU
    /// instance buffers (growing them as needed), compile any not-yet-seen
    /// material pipelines, and upload per-entity material params.
    ///
    /// Records `stats.sprites_rendered`. Extracted verbatim from `render()`'s
    /// batch/upload phase; behavior is unchanged.
    pub(super) fn batch_and_upload(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        stats: &mut crate::resources::RenderStats,
    ) {
        sort_render_entries(&mut self.draw_entries);
        stats.sprites_rendered = self.draw_entries.len() as u32;

        // assign_instance_offsets fills the pre-allocated scratch vecs on self,
        // avoiding per-frame heap allocation in steady state.
        assign_instance_offsets(
            &mut self.draw_entries,
            &mut self.sprite_instances_scratch,
            &mut self.material.material_instances_scratch,
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

        if !self.material.material_instances_scratch.is_empty() {
            if self.material.material_instances_scratch.len() > self.material.mat_instance_capacity
            {
                self.material.mat_instance_capacity = self
                    .material
                    .material_instances_scratch
                    .len()
                    .next_power_of_two();
                self.material.mat_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("material instance buffer"),
                    size: (self.material.mat_instance_capacity * std::mem::size_of::<InstanceRaw>())
                        as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
            }
            queue.write_buffer(
                &self.material.mat_instance_buf,
                0,
                bytemuck::cast_slice(&self.material.material_instances_scratch),
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
                        if !frag_source.is_empty()
                            && !self.material.custom_pipelines.contains_key(hash)
                        {
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
            // before we mutably access self.material.params_buffers.
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
                if !self.material.params_buffers.contains_key(&eid) {
                    let buf = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("material params buf"),
                        size: 16,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("material params bind group"),
                        layout: &self.material.params_layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: buf.as_entire_binding(),
                        }],
                    });
                    self.material.params_buffers.insert(eid, (buf, bg));
                }
                let (buf, _) = &self.material.params_buffers[&eid];
                queue.write_buffer(buf, 0, bytemuck::cast_slice(&params));
            }
        }
    }
}
