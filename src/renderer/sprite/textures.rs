use std::collections::HashMap;
use std::sync::Arc;

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

/// The pixel format a hot-reload must re-upload with: whatever the live texture already carries,
/// falling back to `Rgba8UnormSrgb` when nothing is cached under any alias yet.
///
/// Split from [`TextureCache::reload_texture`] and handed a lookup rather than the map so the
/// *policy* is unit-testable — building a real cache entry needs a GPU device, which CI's `test`
/// job does not have, and this is the half that was wrong. `format_of` is
/// `|k| cache.get(k).map(|t| t.texture.format())` at the call site.
pub(super) fn reload_format(
    aliases: &[String],
    format_of: impl Fn(&str) -> Option<wgpu::TextureFormat>,
) -> wgpu::TextureFormat {
    aliases
        .iter()
        .find_map(|key| format_of(key))
        .unwrap_or(wgpu::TextureFormat::Rgba8UnormSrgb)
}

/// What [`TextureCache::register_render_target`] should do with an incoming bind group.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum RtRegistration {
    /// Already cached, and the same object — do nothing. This is the steady-state answer, and
    /// the reason the enum exists.
    Unchanged,
    /// Cached under this name, but a different bind group: the render target was rebuilt
    /// (resized, reformatted). Overwrite in place, reusing the key already in the map.
    Replace,
    /// Not cached yet — the only path that needs to own a copy of the name.
    Insert,
}

/// Decides that, given what the cache holds for a name and what is being registered.
///
/// Split out and made generic purely so it is testable: building a real `Arc<wgpu::BindGroup>`
/// needs a GPU device, which CI's `test` job does not have — the same reason `reload_format`
/// above takes a lookup instead of the map.
///
/// ⚠️ The point is `Unchanged`. `register_render_target` is called **once per offscreen render
/// target per frame** from `render_offscreen_targets`, always with an `Arc::clone` of the same
/// bind group the `RenderTarget` already holds, and it used to `insert(key.to_string(), bg)`
/// unconditionally — one `String` allocation per target per frame, to overwrite an entry with
/// an identical one. Pointer identity is what distinguishes that from a genuine rebuild;
/// comparing the bind groups by value would not, and there is nothing to compare anyway.
pub(super) fn rt_registration<T>(existing: Option<&Arc<T>>, incoming: &Arc<T>) -> RtRegistration {
    match existing {
        Some(cached) if Arc::ptr_eq(cached, incoming) => RtRegistration::Unchanged,
        Some(_) => RtRegistration::Replace,
        None => RtRegistration::Insert,
    }
}

pub(crate) struct TextureCache {
    pub(crate) white_texture: Texture,
    pub(crate) texture_cache: HashMap<String, Arc<Texture>>,
    /// Bind groups for offscreen render targets, by target name — what makes an RT sampleable
    /// through `Sprite.texture`.
    ///
    /// **Also never evicted, and also not the leak it was filed as.** It mirrors
    /// `RenderState::render_targets` one-for-one, and *that* map has no removal path at all: a
    /// render target created by `create_render_target` lives for the session. So this cannot
    /// outgrow its source, and the thing worth fixing — if a game ever needs to destroy a render
    /// target — is upstream, in `render_targets`, not here. Evicting here alone would only drop a
    /// bind group for a target that is still live.
    pub(crate) rt_cache: HashMap<String, Arc<wgpu::BindGroup>>,
    pub(crate) texture_layout: wgpu::BindGroupLayout,
}

impl TextureCache {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture_layout = Texture::bind_group_layout(device);
        let white_texture = Texture::white(device, queue, &texture_layout);
        Self {
            white_texture,
            texture_cache: HashMap::new(),
            rt_cache: HashMap::new(),
            texture_layout,
        }
    }

    pub(crate) fn has_texture_key(&self, key: &str) -> bool {
        self.texture_cache.contains_key(key)
            || self
                .texture_cache
                .contains_key(crate::asset::asset_key(key).as_ref())
    }

    pub(crate) fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) {
        self.load_texture_with_format(device, queue, path, wgpu::TextureFormat::Rgba8UnormSrgb);
    }

    pub(crate) fn load_texture_with_format(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        format: wgpu::TextureFormat,
    ) {
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

        let tex = Arc::new(Texture::from_path_with_format(
            device,
            queue,
            &self.texture_layout,
            path,
            format,
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

    pub(crate) fn reload_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
    ) {
        let reload_key = crate::asset::asset_key(path).to_string();
        let mut aliases = file_texture_aliases(path);
        for key in self.texture_cache.keys() {
            if crate::asset::asset_key(key).as_ref() == reload_key.as_str() {
                push_unique(&mut aliases, key.clone());
            }
        }

        // Reload in the format the live texture was uploaded with, not the `from_path` default.
        // `Texture::from_path` hardcodes `Rgba8UnormSrgb`, so a **data** texture — a normal map,
        // mask, height or lookup table, uploaded linear via `load_texture_with_format` — came back
        // from a hot-reload with an sRGB decode attached to it. Silently, and only after the first
        // edit, so the run that verified the asset was not the run that broke it. No new state is
        // needed to fix it: the GPU texture already knows its own format.
        let format = reload_format(&aliases, |key| {
            self.texture_cache.get(key).map(|t| t.texture.format())
        });

        let tex = Arc::new(Texture::from_path_with_format(
            device,
            queue,
            &self.texture_layout,
            path,
            format,
        ));
        for alias in aliases {
            self.texture_cache.insert(alias, Arc::clone(&tex));
        }
        log::info!("texture hot-reload: {path} ({format:?})");
    }

    pub(crate) fn register_render_target(&mut self, key: &str, bg: Arc<wgpu::BindGroup>) {
        match rt_registration(self.rt_cache.get(key), &bg) {
            RtRegistration::Unchanged => {}
            RtRegistration::Replace => {
                if let Some(slot) = self.rt_cache.get_mut(key) {
                    *slot = bg;
                }
            }
            RtRegistration::Insert => {
                self.rt_cache.insert(key.to_string(), bg);
            }
        }
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

impl SpriteRenderer {
    pub(crate) fn has_texture_key(&self, key: &str) -> bool {
        self.texture_cache.has_texture_key(key)
    }

    pub fn load_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) {
        self.texture_cache.load_texture(device, queue, path);
    }

    /// Loads a file texture with a caller-chosen pixel format (e.g. `Rgba8Unorm` for a
    /// linear data texture that must be sampled without the sRGB decode). The default
    /// [`SpriteRenderer::load_texture`] uses `Rgba8UnormSrgb` (correct for color art).
    pub fn load_texture_with_format(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &str,
        format: wgpu::TextureFormat,
    ) {
        self.texture_cache
            .load_texture_with_format(device, queue, path, format);
    }

    pub(crate) fn load_texture_from_image(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &str,
        asset: &crate::asset::ImageAsset,
    ) {
        self.texture_cache
            .load_texture_from_image(device, queue, key, asset);
    }

    pub fn reload_texture(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, path: &str) {
        self.texture_cache.reload_texture(device, queue, path);
    }

    pub fn register_render_target(&mut self, key: &str, bg: Arc<wgpu::BindGroup>) {
        self.texture_cache.register_render_target(key, bg);
    }

    pub(super) fn bind_group_for_texture_key(&self, key: Option<&str>) -> &wgpu::BindGroup {
        self.texture_cache.bind_group_for_texture_key(key)
    }
}
