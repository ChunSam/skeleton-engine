use super::*;

impl SpriteRenderer {
    /// Record the sorted `draw_entries` into a single render pass on `encoder`:
    /// contiguous same-texture `Sprite` runs become one instanced draw, and each
    /// `Material` entry issues its own pipeline/bind-group/draw. Increments
    /// `stats.draw_calls`.
    ///
    /// Extracted verbatim from `render()`'s draw phase; behavior is unchanged.
    pub(super) fn record_draw_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        stats: &mut crate::resources::RenderStats,
    ) {
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
                        let pipeline = &self.material.custom_pipelines[hash];
                        let tex_bg = texture_key
                            .as_deref()
                            .map(|k| self.texture_cache.bind_group_for_texture_key(Some(k)))
                            .unwrap_or(&self.texture_cache.white_texture.bind_group);
                        let (_, params_bg) = &self.material.params_buffers[entity];

                        pass.set_pipeline(pipeline);
                        pass.set_bind_group(0, &self.camera_bind_group, &[]);
                        pass.set_bind_group(1, tex_bg, &[]);
                        pass.set_bind_group(2, params_bg, &[]);
                        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
                        pass.set_vertex_buffer(
                            1,
                            self.material.mat_instance_buf.slice(byte_start..byte_end),
                        );
                        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
                        pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
                        stats.draw_calls += 1;
                        i += 1;
                    }
                }
            }
        }
    }
}
