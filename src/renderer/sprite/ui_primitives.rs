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
    pub(super) texture_key: Option<Arc<str>>,
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

/// Sorts `rects` + `images` into one z-ordered stream, reusing `primitives` as scratch.
///
/// Takes the buffer rather than returning one: this runs every frame, so the `Vec` is a field on
/// `SpriteRenderer` that is cleared and refilled (the convention in `docs/PATTERNS.md`
/// § *Per-frame scratch buffers*).
pub(super) fn sorted_ui_primitives(
    primitives: &mut Vec<UiPrimitive>,
    rects: &[DrawRect],
    images: &[DrawImage],
) {
    primitives.clear();
    primitives.reserve(rects.len() + images.len());

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

    // `sort_unstable_by`, not `sort_by`: the comparator ends in `order`, which is unique within a
    // kind while `sort_rank` already separates the kinds — so this is a strict total order and
    // stability cannot change the result. The stable sort allocates a temporary buffer whose
    // threshold depends on element size AND count, and `UiPrimitive` is large enough to cross it
    // at ~32 primitives; the unstable sort never allocates. Measured identical output.
    primitives.sort_unstable_by(|a, b| {
        a.z.partial_cmp(&b.z)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.kind.sort_rank().cmp(&b.kind.sort_rank()))
            .then_with(|| a.order.cmp(&b.order))
    });
}

/// One frame's UI primitives, sorted by z and already uploaded to the shared instance buffer —
/// ready for [`SpriteRenderer::render_ui_primitive_range`] to draw any sub-range in its own render
/// pass. The instance buffer (and the UI camera uniform) is written **once** per frame by
/// [`SpriteRenderer::prepare_ui_primitives`]; `queue.write_buffer` executes at submit time, so a
/// second upload before submission would clobber every earlier pass's instances — range draws over
/// one upload are how the z-interleaved text layering slices the surface list.
pub struct PreparedUiPrimitives {
    /// Texture-key per instance (sorted order) — drives the per-run bind group inside a range.
    keys: Vec<Option<Arc<str>>>,
    /// Z per instance (ascending) — the surface half of the text-layering interleave.
    pub(crate) zs: Vec<f32>,
}

impl PreparedUiPrimitives {
    /// Number of prepared instances.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// True when nothing was prepared.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }
}

impl SpriteRenderer {
    /// Sorts `rects` + `images` by z, uploads the instances + the screen-space camera uniform, and
    /// returns the per-instance metadata for range rendering. Call once per frame per target;
    /// follow with one or more [`render_ui_primitive_range`](Self::render_ui_primitive_range).
    #[allow(clippy::too_many_arguments)]
    pub fn prepare_ui_primitives(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rects: &[DrawRect],
        images: &[DrawImage],
        width: u32,
        height: u32,
        clip_scale: Vec2,
    ) -> PreparedUiPrimitives {
        let screen_proj = crate::camera::apply_letterbox(
            clip_scale,
            Mat4::orthographic_rh(0.0, width as f32, height as f32, 0.0, -1.0, 1.0),
        );
        let cam = CameraUniform {
            view_proj: screen_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.ui_camera_buf, 0, bytemuck::bytes_of(&cam));

        // `primitives` and `instances` never leave this function, so they are scratch fields
        // cleared and refilled each frame (`docs/PATTERNS.md` § *Per-frame scratch buffers*).
        //
        // ⚠️ `keys` and `zs` DO leave — they are the returned `PreparedUiPrimitives`, which the
        // caller holds until the frame's last range is drawn — so they are still built fresh here.
        // That is 2 allocations per frame, constant, and it does NOT scale with how much UI is on
        // screen; what scaled was the per-image `String` (now an `Arc<str>` bump) and the stable
        // sort's temporary. Making these two scratch as well needs the buffers handed back after
        // drawing, and that is a footgun API — forget the call and the win silently disappears.
        let mut primitives = std::mem::take(&mut self.ui_primitives_scratch);
        let mut instances = std::mem::take(&mut self.ui_instances_scratch);
        sorted_ui_primitives(&mut primitives, rects, images);
        let mut keys = Vec::with_capacity(primitives.len());
        let mut zs = Vec::with_capacity(primitives.len());
        instances.clear();
        for p in &primitives {
            keys.push(p.texture_key.clone());
            zs.push(p.z);
            instances.push(p.instance);
        }
        self.ui_primitives_scratch = primitives;

        if instances.len() > self.ui_instance_capacity {
            self.ui_instance_capacity = instances.len().next_power_of_two();
            self.ui_instance_buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("ui instance buffer"),
                size: (self.ui_instance_capacity * std::mem::size_of::<UiInstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !instances.is_empty() {
            queue.write_buffer(&self.ui_instance_buf, 0, bytemuck::cast_slice(&instances));
        }

        self.ui_instances_scratch = instances;
        PreparedUiPrimitives { keys, zs }
    }

    /// Draws `prepared[start..end)` in one render pass (instance-buffer byte offsets into the
    /// single per-frame upload — no re-upload). No-op for an empty range.
    pub fn render_ui_primitive_range(
        &mut self,
        ctx: &mut FrameContext,
        prepared: &PreparedUiPrimitives,
        start: usize,
        end: usize,
    ) {
        if start >= end {
            return;
        }
        let fmt = ctx.format;
        // Build/select a UI pipeline matching the target format (the surface, or e.g. the HDR
        // post-process intermediate / an offscreen RT).
        self.ensure_ui_pipeline(ctx.device, fmt);

        let instance_size = std::mem::size_of::<UiInstanceRaw>() as u64;
        let ui_pipeline = self.ui_pipeline_for(fmt);
        let mut pass = crate::renderer::common::begin_color_pass(
            ctx.encoder,
            "ui primitive pass",
            ctx.view,
            wgpu::LoadOp::Load,
        );

        pass.set_pipeline(ui_pipeline);
        pass.set_bind_group(0, &self.ui_camera_bind_group, &[]);
        pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);

        let mut i = start;
        while i < end {
            let run_key = prepared.keys[i].as_deref();
            let run_start = i;
            i += 1;
            while i < end && prepared.keys[i].as_deref() == run_key {
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

    /// Prepare + draw every UI primitive in one pass — the single-call path used when no layered
    /// text interleaves with the surfaces (and by earlier callers; behavior unchanged).
    pub fn render_ui_primitives_from_slices(
        &mut self,
        ctx: &mut FrameContext,
        rects: &[DrawRect],
        images: &[DrawImage],
        width: u32,
        height: u32,
        clip_scale: Vec2,
    ) {
        if rects.is_empty() && images.is_empty() {
            return;
        }
        let prepared = self.prepare_ui_primitives(
            ctx.device, ctx.queue, rects, images, width, height, clip_scale,
        );
        let end = prepared.len();
        self.render_ui_primitive_range(ctx, &prepared, 0, end);
    }
}
