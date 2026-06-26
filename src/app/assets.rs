use super::*;

impl App {
    /// Creates a render target sampleable by the sprite pipeline, using the **surface** pixel
    /// format. For an HDR (`Rgba16Float`) or other caller-chosen format, use
    /// [`create_render_target_with_format`](Self::create_render_target_with_format).
    pub fn create_render_target(&mut self, name: impl Into<String>, width: u32, height: u32) {
        self.create_render_target_impl(name.into(), width, height, None, wgpu::FilterMode::Nearest);
    }

    /// Creates a render target (surface format) sampled with a caller-chosen `filter` when the RT is
    /// shown for display. `wgpu::FilterMode::Linear` smooths a render target that is displayed scaled
    /// up or used as a blur source (the default `Nearest` keeps it pixelated). Pair with an
    /// `OffscreenCamera` like [`create_render_target`](Self::create_render_target).
    pub fn create_render_target_with_filter(
        &mut self,
        name: impl Into<String>,
        width: u32,
        height: u32,
        filter: wgpu::FilterMode,
    ) {
        self.create_render_target_impl(name.into(), width, height, None, filter);
    }

    /// Creates a render target with a caller-chosen pixel `format` (e.g. `wgpu::TextureFormat::Rgba16Float`
    /// for an HDR offscreen buffer, or a linear `Rgba8Unorm`). The offscreen pass renders into it with a
    /// sprite pipeline built to match `format` (built lazily, cached per format), so an `OffscreenCamera`
    /// targeting this RT renders correctly even when `format` differs from the surface format. The RT is
    /// still sampleable by ordinary sprite draws (e.g. to display the result). Tone-mapping an HDR target
    /// for final display is the game's responsibility (e.g. a `ShaderMaterial` on the display sprite).
    pub fn create_render_target_with_format(
        &mut self,
        name: impl Into<String>,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) {
        self.create_render_target_impl(
            name.into(),
            width,
            height,
            Some(format),
            wgpu::FilterMode::Nearest,
        );
    }

    fn create_render_target_impl(
        &mut self,
        name: String,
        width: u32,
        height: u32,
        format: Option<wgpu::TextureFormat>,
        filter: wgpu::FilterMode,
    ) {
        if let Some(gpu) = &self.gpu {
            // GPU already initialized — create immediately. `None` inherits the surface format; a
            // caller-chosen format that this GPU cannot render into falls back (with a warning).
            let resolved = gpu.resolve_render_target_format(format, &name);
            let rt = crate::renderer::render_target::RenderTarget::new_with_filter(
                &gpu.device,
                width,
                height,
                resolved,
                filter,
            );
            self.render.render_targets.insert(name, rt);
        } else {
            // GPU not yet initialized — defer to pending (the format resolves at creation).
            self.pending_render_targets
                .push((name, width, height, format, filter));
        }
    }

    pub fn load_image(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
        let path = path.into();
        self.pending_textures.push(path.clone());
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer resource missing")
            .load_image(&path)
    }

    /// Loads an image as a GPU texture with a caller-chosen pixel `format`, instead of the
    /// default `Rgba8UnormSrgb`.
    ///
    /// Pass `wgpu::TextureFormat::Rgba8Unorm` (linear) for a **data texture** — a normal map,
    /// mask, height or lookup table — whose bytes are not sRGB-encoded color and must be
    /// sampled verbatim (no sRGB→linear decode). The texture stays sampleable by ordinary
    /// sprites. Like [`App::load_image`], call this before the texture is first uploaded
    /// (i.e. before `run()`); the format applies when the pending texture is loaded at GPU init.
    pub fn load_image_with_format(
        &mut self,
        path: impl Into<String>,
        format: wgpu::TextureFormat,
    ) -> Handle<ImageAsset> {
        let path = path.into();
        self.pending_textures.push(path.clone());
        self.pending_texture_formats.insert(path.clone(), format);
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer resource missing")
            .load_image(&path)
    }

    pub fn load_image_async(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
        let path = path.into();
        let handle = self
            .world
            .resource_mut::<AssetServer>()
            .expect("AssetServer missing")
            .load_image_async(&path);
        // Increment LoadProgress.total
        if let Some(prog) = self.world.resource_mut::<LoadProgress>() {
            prog.total += 1;
        }
        handle
    }

    pub fn load_atlas(
        &mut self,
        path: impl Into<String>,
        cols: u32,
        rows: u32,
    ) -> Handle<crate::atlas::TextureAtlas> {
        let path = path.into();
        self.pending_textures.push(path.clone());
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer missing")
            .load_atlas(&path, cols, rows)
    }

    pub fn load_script(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Handle<crate::scripting::ScriptAsset> {
        let path_ref = path.as_ref();
        // Register the path with the AssetServer file watcher so changes are
        // detected and forwarded to ScriptRegistry::reload_path via the
        // HotReloadable forwarder registered in App::new.
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(assets) = self.world.resource_mut::<AssetServer>() {
            assets.watch_path(path_ref.to_string_lossy().as_ref());
        }
        self.world
            .resource_mut::<crate::scripting::ScriptRegistry>()
            .expect("ScriptRegistry missing")
            .load_script(path_ref)
    }

    pub(super) fn upload_asset_server_images_to_gpu(&mut self) {
        let (Some(sr), Some(gpu)) = (&mut self.render.sprite_renderer, &self.gpu) else {
            return;
        };
        let images = self
            .world
            .resource::<AssetServer>()
            .map(|assets| assets.image_assets_for_gpu())
            .unwrap_or_default();
        for (path, asset) in images {
            if !sr.has_texture_key(&path) {
                sr.load_texture_from_image(&gpu.device, &gpu.queue, &path, &asset);
            }
        }
    }
}
