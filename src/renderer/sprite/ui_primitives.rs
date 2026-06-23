use super::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum UiPrimitiveKind {
    Image,
    Rect,
}

impl UiPrimitiveKind {
    pub(super) fn sort_rank(self) -> u8 {
        match self {
            UiPrimitiveKind::Image => 0,
            UiPrimitiveKind::Rect => 1,
        }
    }
}

pub(super) struct UiPrimitive {
    pub(super) z: f32,
    pub(super) kind: UiPrimitiveKind,
    pub(super) order: usize,
    pub(super) texture_key: Option<String>,
    pub(super) instance: UiInstanceRaw,
}

/// Builds a UI quad instance. `corner = [radius, border]` (screen px) drives the
/// rounded-rect SDF in `ui.wgsl`; `[0.0, 0.0]` (every image, an unrounded rect) is the
/// fast path that renders identically to the original plain quad.
pub(super) fn ui_quad_instance(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: [f32; 4],
    uv: UvRect,
    corner: [f32; 2],
) -> UiInstanceRaw {
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let model = Mat4::from_scale_rotation_translation(
        Vec3::new(w, h, 1.0),
        Quat::IDENTITY,
        Vec3::new(cx, cy, 0.0),
    );
    UiInstanceRaw {
        model: model.to_cols_array_2d(),
        color,
        uv_offset: [uv.u_offset, uv.v_offset],
        uv_size: [uv.u_size, uv.v_size],
        px_size: [w, h],
        corner,
    }
}

pub(super) fn sorted_ui_primitives(rects: &[DrawRect], images: &[DrawImage]) -> Vec<UiPrimitive> {
    let mut primitives = Vec::with_capacity(rects.len() + images.len());

    primitives.extend(images.iter().enumerate().map(|(order, image)| UiPrimitive {
        z: image.z,
        kind: UiPrimitiveKind::Image,
        order,
        texture_key: image.texture_key(),
        instance: ui_quad_instance(
            image.x,
            image.y,
            image.w,
            image.h,
            image.color.to_array(),
            image.uv,
            [0.0, 0.0],
        ),
    }));

    primitives.extend(rects.iter().enumerate().map(|(order, rect)| UiPrimitive {
        z: rect.z,
        kind: UiPrimitiveKind::Rect,
        order,
        texture_key: None,
        instance: ui_quad_instance(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            rect.color.to_array(),
            UvRect::FULL,
            [rect.corner_radius, rect.border],
        ),
    }));

    primitives.sort_by(|a, b| {
        a.z.partial_cmp(&b.z)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.kind.sort_rank().cmp(&b.kind.sort_rank()))
            .then_with(|| a.order.cmp(&b.order))
    });

    primitives
}

impl SpriteRenderer {
    pub fn render_ui_primitives_from_slices(
        &mut self,
        ctx: &mut FrameContext,
        rects: &[DrawRect],
        images: &[DrawImage],
        width: u32,
        height: u32,
    ) {
        if rects.is_empty() && images.is_empty() {
            return;
        }

        let fmt = ctx.format;
        // Build/select a UI pipeline matching the target format (the surface, or e.g. the HDR
        // post-process intermediate / an offscreen RT).
        self.ensure_ui_pipeline(ctx.device, fmt);

        let device = ctx.device;
        let queue = ctx.queue;
        let view = ctx.view;
        let encoder = &mut *ctx.encoder;

        let screen_proj = Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0);
        let cam = CameraUniform {
            view_proj: screen_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.ui_camera_buf, 0, bytemuck::bytes_of(&cam));

        let entries: Vec<(Option<String>, UiInstanceRaw)> = sorted_ui_primitives(rects, images)
            .into_iter()
            .map(|primitive| (primitive.texture_key, primitive.instance))
            .collect();
        let instances: Vec<UiInstanceRaw> = entries.iter().map(|(_, instance)| *instance).collect();

        if instances.len() > self.ui_instance_capacity {
            self.ui_instance_capacity = instances.len().next_power_of_two();
            self.ui_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui instance buffer"),
                size: (self.ui_instance_capacity * std::mem::size_of::<UiInstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        queue.write_buffer(&self.ui_instance_buf, 0, bytemuck::cast_slice(&instances));

        let instance_size = std::mem::size_of::<UiInstanceRaw>() as u64;
        let ui_pipeline = self.ui_pipeline_for(fmt);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ui primitive pass"),
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

        pass.set_pipeline(ui_pipeline);
        pass.set_bind_group(0, &self.ui_camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);

        let mut i = 0usize;
        while i < entries.len() {
            let run_key = entries[i].0.as_deref();
            let run_start = i;
            i += 1;
            while i < entries.len() && entries[i].0.as_deref() == run_key {
                i += 1;
            }
            let run_len = i - run_start;
            let byte_start = run_start as u64 * instance_size;
            let byte_end = byte_start + run_len as u64 * instance_size;
            let bind_group = self.bind_group_for_texture_key(run_key);
            pass.set_bind_group(1, bind_group, &[]);
            pass.set_vertex_buffer(1, self.ui_instance_buf.slice(byte_start..byte_end));
            pass.draw_indexed(0..INDICES.len() as u32, 0, 0..run_len as u32);
        }
    }
}
