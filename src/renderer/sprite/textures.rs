use super::*;

pub(super) fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

pub(super) fn file_texture_aliases(path: &str) -> Vec<String> {
    let mut aliases = vec![path.to_string()];
    push_unique(&mut aliases, crate::asset::asset_key(path).to_string());
    aliases
}

impl SpriteRenderer {
    pub fn texture_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_layout
    }

    pub(crate) fn has_texture_key(&self, key: &str) -> bool {
        self.texture_cache.contains_key(key)
            || self
                .texture_cache
                .contains_key(crate::asset::asset_key(key).as_ref())
    }

    pub fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) {
        let aliases = file_texture_aliases(path);
        if let Some(existing) = aliases
            .iter()
            .find_map(|key| self.texture_cache.get(key).cloned())
        {
            for alias in aliases {
                self.texture_cache
                    .entry(alias)
                    .or_insert_with(|| Arc::clone(&existing));
            }
            return;
        }

        let tex = Arc::new(Texture::from_path(
            device,
            queue,
            &self.texture_layout,
            path,
        ));
        for alias in aliases {
            self.texture_cache.insert(alias, Arc::clone(&tex));
        }
    }

    pub(crate) fn load_texture_from_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &str,
        asset: &crate::asset::ImageAsset,
    ) {
        use crate::renderer::texture::Texture;
        let tex = Texture::from_image_asset(device, queue, &self.texture_layout, asset, Some(key));
        self.texture_cache.insert(key.to_string(), Arc::new(tex));
    }

    pub fn reload_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) {
        let reload_key = crate::asset::asset_key(path).to_string();
        let mut aliases = file_texture_aliases(path);
        for key in self.texture_cache.keys() {
            if crate::asset::asset_key(key).as_ref() == reload_key.as_str() {
                push_unique(&mut aliases, key.clone());
            }
        }

        let tex = Arc::new(Texture::from_path(
            device,
            queue,
            &self.texture_layout,
            path,
        ));
        for alias in aliases {
            self.texture_cache.insert(alias, Arc::clone(&tex));
        }
        log::info!("texture hot-reload: {path}");
    }

    pub fn register_render_target(&mut self, key: &str, bg: Arc<wgpu::BindGroup>) {
        self.rt_cache.insert(key.to_string(), bg);
    }

    pub(super) fn bind_group_for_texture_key(&self, key: Option<&str>) -> &wgpu::BindGroup {
        match key.filter(|key| !key.is_empty()) {
            Some(key) => self
                .rt_cache
                .get(key)
                .map(|bg| bg.as_ref())
                .or_else(|| self.texture_cache.get(key).map(|tex| &tex.bind_group))
                .unwrap_or(&self.white_texture.bind_group),
            None => &self.white_texture.bind_group,
        }
    }
}
