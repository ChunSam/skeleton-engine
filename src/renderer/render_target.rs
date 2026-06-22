use std::sync::Arc;

/// GPU texture that can be rendered into and sampled by the sprite pipeline.
pub struct RenderTarget {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) sampler: wgpu::Sampler,
    pub(crate) bind_group: Arc<wgpu::BindGroup>,
    pub width: u32,
    pub height: u32,
    /// The texture's pixel format — the offscreen pass renders into it with a sprite pipeline built
    /// for this format (a non-surface format, e.g. `Rgba16Float` for an HDR target, gets its own
    /// pipeline variant). See [`format`](RenderTarget::format).
    pub(crate) format: wgpu::TextureFormat,
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
    /// Creates a new render target of the given dimensions and pixel format.
    ///
    /// The bind-group layout is built internally using the same descriptor as the
    /// sprite pipeline (filterable texture @0, filtering sampler @1), so the
    /// resulting target is directly sampleable by sprite draw calls.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
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
        // Build our own layout — same descriptor as the sprite pipeline's texture slot
        // (filterable texture @0, filtering sampler @1) so RTs are directly sampleable
        // by sprite draw calls without borrowing the sprite renderer's layout.
        let layout = crate::renderer::texture::Texture::bind_group_layout(device);
        let bind_group = Arc::new(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("render target bind group"),
            layout: &layout,
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
            format,
            clear_color: None,
        }
    }

    /// The pixel format the target was created with (what the offscreen sprite pass must target).
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
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

    // ── Escape-hatch accessors ────────────────────────────────────────────────
    // The underlying wgpu objects are intentionally not pub fields — callers that
    // need raw GPU access for custom passes (e.g. writing normals into an RT or
    // attaching it to a custom bind group) can use these read-only accessors.
    // Pattern mirrors `PhysicsWorld::rigid_body` / the `.raw()` escape hatches.

    /// Returns a reference to the underlying GPU texture.
    ///
    /// **Escape hatch** — prefer the higher-level `App`/sprite APIs where possible.
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    /// Returns a reference to the texture view used by render passes and bind groups.
    ///
    /// **Escape hatch** — prefer the higher-level `App`/sprite APIs where possible.
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Returns a reference to the sampler configured for this render target.
    ///
    /// **Escape hatch** — prefer the higher-level `App`/sprite APIs where possible.
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }

    /// Returns a reference to the bind group (texture + sampler) for this render target.
    ///
    /// **Escape hatch** — prefer the higher-level `App`/sprite APIs where possible.
    pub fn bind_group(&self) -> &Arc<wgpu::BindGroup> {
        &self.bind_group
    }
}
