use super::*;

impl App {
    pub fn create_render_target(&mut self, name: impl Into<String>, width: u32, height: u32) {
        let name = name.into();
        if let (Some(gpu), Some(sr)) = (&self.gpu, &self.sprite_renderer) {
            // GPU가 이미 초기화된 경우 즉시 생성
            let rt = crate::renderer::render_target::RenderTarget::new(
                &gpu.device,
                width,
                height,
                gpu.config.format,
                sr.texture_layout(),
            );
            self.render_targets.insert(name, rt);
        } else {
            // GPU 초기화 전이면 pending에 보관
            self.pending_render_targets.push((name, width, height));
        }
    }

    pub fn load_texture(&mut self, path: impl Into<String>) {
        self.pending_textures.push(path.into());
    }

    pub fn load_image(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
        let path = path.into();
        self.pending_textures.push(path.clone());
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer 리소스 누락")
            .load_image(&path)
    }

    pub fn load_image_async(&mut self, path: impl Into<String>) -> Handle<ImageAsset> {
        let path = path.into();
        let handle = self
            .world
            .resource_mut::<AssetServer>()
            .expect("AssetServer 없음")
            .load_image_async(&path);
        // LoadProgress.total 증가
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
            .expect("AssetServer 없음")
            .load_atlas(&path, cols, rows)
    }

    pub fn load_script(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Handle<crate::asset::ScriptAsset> {
        self.world
            .resource_mut::<AssetServer>()
            .expect("AssetServer 없음")
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
