use super::*;

impl App {
    pub fn create_render_target(&mut self, name: impl Into<String>, width: u32, height: u32) {
        let name = name.into();
        if let (Some(gpu), Some(sr)) = (&self.gpu, &self.sprite_renderer) {
            // GPU already initialized — create immediately
            let rt = crate::renderer::render_target::RenderTarget::new(
                &gpu.device,
                width,
                height,
                gpu.config.format,
                sr.texture_layout(),
            );
            self.render_targets.insert(name, rt);
        } else {
            // GPU not yet initialized — defer to pending
            self.pending_render_targets.push((name, width, height));
        }
    }

    #[deprecated(
        since = "4.6.0",
        note = "use load_image — returns a Handle<ImageAsset> and participates in hot reload"
    )]
    pub fn load_texture(&mut self, path: impl Into<String>) {
        self.pending_textures.push(path.into());
    }

    pub fn load_image(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
        let path = path.into();
        self.pending_textures.push(path.clone());
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
    ) -> Handle<crate::asset::ScriptAsset> {
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer missing")
            .load_script(path)
    }

    pub(super) fn upload_asset_server_images_to_gpu(&mut self) {
        let (Some(sr), Some(gpu)) = (&mut self.sprite_renderer, &self.gpu) else {
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
