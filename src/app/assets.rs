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

    /// Pins asset resolution to one explicit root directory.
    ///
    /// By default the engine finds relative asset paths on its own — next to the executable, in a
    /// macOS bundle's `Contents/Resources`, or in the working directory — so a packaged build works
    /// however it is launched. Call this only to override that; see
    /// [`asset_path`](crate::asset_path) for the search order.
    pub fn set_asset_root(&mut self, root: impl Into<std::path::PathBuf>) {
        crate::asset_path::set_asset_root(root);
    }

    /// Makes a failed asset load panic instead of falling back to a magenta placeholder.
    ///
    /// Off by default (a released game should degrade, not die). Turn it on in a dev build so a
    /// missing texture stops at the load rather than painting the window magenta.
    pub fn set_strict_assets(&mut self, strict: bool) {
        crate::asset_path::set_strict_assets(strict);
    }

    /// Every asset the engine failed to load, in load order.
    ///
    /// The engine substitutes a placeholder and keeps running, which is easy to miss — poll this
    /// after loading to log a diagnostic, show a warning, or refuse to start.
    ///
    /// ```rust,no_run
    /// # use engine::App;
    /// # let mut app = App::new();
    /// app.load_image("assets/hero.png");
    /// for failure in app.asset_failures() {
    ///     eprintln!("missing asset: {} — {}", failure.path, failure.error);
    /// }
    /// ```
    pub fn asset_failures(&self) -> Vec<crate::AssetFailure> {
        crate::asset_path::asset_failures()
    }

    pub fn load_image(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
        let path = path.into();
        self.pending_textures.push(path.clone());
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer resource missing")
            .load_image(&path)
    }

    /// Registers an image already in memory (e.g. `include_bytes!("hero.png")`) under `key` and
    /// returns a handle — **no file is read**.
    ///
    /// For a single-file / jam build that embeds its art at compile time, or a runtime-generated
    /// image. The `key` is used **verbatim** as both the cache key and the handle path, so a
    /// [`Sprite::with_handle(handle)`](crate::Sprite::with_handle) — or a
    /// [`Sprite::textured(key)`](crate::Sprite::textured) naming the same string — renders it
    /// exactly like a path-loaded image. The `key` is a *logical identifier*, never resolved
    /// against the [asset root](crate::asset_path); it needs no `assets/` on disk and cannot
    /// collide with a real file.
    ///
    /// Cross-platform, including wasm — where `include_bytes!` sidesteps the async fetch that
    /// [`load_image`](Self::load_image) needs. Unlike `load_image`, this queues **no** filesystem
    /// read; the decoded image is uploaded to the GPU by the same per-frame path as async loads.
    /// A corrupt-bytes decode failure is reported via
    /// [`asset_failures`](Self::asset_failures), exactly like a missing file.
    ///
    /// ```rust,no_run
    /// # use engine::App;
    /// # let mut app = App::new();
    /// static HERO_PNG: &[u8] = &[]; // include_bytes!("assets/hero.png")
    /// let hero = app.load_image_bytes("hero", HERO_PNG);
    /// // spawn a sprite with `engine::Sprite::with_handle(hero)`
    /// # let _ = hero;
    /// ```
    pub fn load_image_bytes(&mut self, key: impl Into<String>, bytes: &[u8]) -> Handle<ImageAsset> {
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer resource missing")
            .load_image_bytes(key, bytes)
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

    /// Registers a texture atlas whose sprite sheet is already in memory (e.g.
    /// `include_bytes!("tiles.png")`) under `key` and returns a handle — **no file is read**.
    ///
    /// The byte-source twin of [`load_atlas`](Self::load_atlas); it completes for atlases what
    /// [`load_image_bytes`](Self::load_image_bytes) does for single images, so a single-file / jam
    /// build can bake its whole sprite sheet into the binary. The grid is the same uniform
    /// `cols × rows` layout.
    ///
    /// The `key` is used **verbatim** as the atlas cache key, the handle path and the renderer's
    /// texture key, so an [`AtlasSprite`](crate::AtlasSprite) on the returned handle renders exactly
    /// like a path-loaded atlas. It is a *logical identifier*, never resolved against the
    /// [asset root](crate::asset_path); it needs no file on disk and cannot collide with one.
    ///
    /// Cross-platform, including wasm — where `include_bytes!` sidesteps the async fetch a path load
    /// needs. Unlike `load_atlas`, this queues **no** filesystem read; the decoded sheet is uploaded
    /// to the GPU by the same per-frame path as async loads. Corrupt bytes are reported via
    /// [`asset_failures`](Self::asset_failures), exactly like a missing file.
    ///
    /// ```rust,no_run
    /// # use engine::{App, AtlasSprite, Transform};
    /// # let mut app = App::new();
    /// static TILES_PNG: &[u8] = &[]; // include_bytes!("assets/tiles.png")
    /// let tiles = app.load_atlas_bytes("tiles", TILES_PNG, 4, 4);
    /// let e = app.world.spawn();
    /// app.world.add_component(e, Transform::default());
    /// app.world.add_component(e, AtlasSprite::new(tiles, 5));
    /// ```
    pub fn load_atlas_bytes(
        &mut self,
        key: impl Into<String>,
        bytes: &[u8],
        cols: u32,
        rows: u32,
    ) -> Handle<crate::atlas::TextureAtlas> {
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer missing")
            .load_atlas_bytes(key, bytes, cols, rows)
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
        // Borrow straight from the server rather than collecting: this runs every frame and
        // the `has_texture_key` guard rejects nearly every item. `self.world` is a different
        // field from `self.render` / `self.gpu`, so the disjoint borrows coexist.
        let Some(assets) = self.world.resource::<AssetServer>() else {
            return;
        };
        for (path, asset) in assets.image_assets_for_gpu() {
            if !sr.has_texture_key(path) {
                sr.load_texture_from_image(&gpu.device, &gpu.queue, path, asset);
            }
        }
    }
}
