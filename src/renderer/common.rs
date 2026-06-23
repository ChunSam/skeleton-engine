//! Shared wgpu boilerplate for the renderer passes.
//!
//! Several render modules (`sprite`, `lighting`, `post_process`, `texture`, the UI-primitive
//! pass, and the egui overlay pass) built byte-identical bind-group-layout entries, clamp
//! samplers, and single-color render passes inline. The 2026-06-23 code-quality scan (P3)
//! flagged the duplication: it isn't a bug, but it makes wgpu upgrades costlier and lets the
//! copies drift. These `pub(crate)` helpers centralize the shared shapes; the values produced
//! are identical to the previous inline literals, so behavior is unchanged.

/// A `Texture` bind-group-layout entry for a filterable float 2D texture, fragment-visible.
///
/// The shared first entry of the texture / post-process / lighting layouts.
pub(crate) fn filterable_texture_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

/// A `Sampler(Filtering)` bind-group-layout entry, fragment-visible.
pub(crate) fn filtering_sampler_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

/// A uniform-buffer bind-group-layout entry, fragment-visible (no dynamic offset).
pub(crate) fn uniform_buffer_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// Create a clamp-to-edge sampler using `filter` for both mag and min (and default mipmap/lod).
///
/// The post-process and lighting passes use `Linear` (smooth upscaling of the full-screen
/// intermediate); sprite/atlas textures use `Nearest` (crisp pixel art). All three otherwise
/// shared the same descriptor.
pub(crate) fn create_clamp_sampler(
    device: &wgpu::Device,
    label: Option<&str>,
    filter: wgpu::FilterMode,
) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        mag_filter: filter,
        min_filter: filter,
        ..Default::default()
    })
}

/// Begin a render pass with a single color attachment, `store`, and no depth attachment.
///
/// `load` selects the clear/load behavior (`LoadOp::Load` to keep prior contents, or
/// `LoadOp::Clear(color)` to clear). Covers every single-attachment pass in the renderer; the
/// returned pass borrows `encoder` (call `.forget_lifetime()` where a `'static` pass is needed,
/// e.g. the egui overlay).
pub(crate) fn begin_color_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    label: &str,
    view: &'a wgpu::TextureView,
    load: wgpu::LoadOp<wgpu::Color>,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
        multiview_mask: None,
    })
}
