use std::sync::Arc;

/// GPU texture that can be rendered into and sampled by the sprite pipeline.
pub struct RenderTarget {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: Arc<wgpu::BindGroup>,
    pub width: u32,
    pub height: u32,
    /// Optional per-target clear color `[r, g, b, a]` (sRGB, `f64`).
    ///
    /// When `Some`, the offscreen pass clears with this color instead of
    /// inheriting `WindowConfig::clear_color`.  Useful for render-to-texture
    /// targets that require a transparent or differently-colored background
    /// (e.g. a security-camera feed over a different background).
    ///
    /// `None` (the default) means "inherit the global `WindowConfig::clear_color`."
    pub clear_color: Option<[f64; 4]>,
}

impl RenderTarget {
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("render target texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let bind_group = Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render target bind group"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        }));

        Self {
            texture,
            view,
            sampler,
            bind_group,
            width,
            height,
            clear_color: None,
        }
    }

    /// Sets a per-target clear color, overriding `WindowConfig::clear_color` for this RT.
    ///
    /// # Example
    /// ```rust,ignore
    /// // Transparent black background for compositing.
    /// let rt = app.create_render_target("overlay", 320, 240)
    ///     .with_clear_color([0.0, 0.0, 0.0, 0.0]);
    /// ```
    pub fn with_clear_color(mut self, color: [f64; 4]) -> Self {
        self.clear_color = Some(color);
        self
    }
}
