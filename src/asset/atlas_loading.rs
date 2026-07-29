use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use super::{alloc_id, asset_key, AssetServer, Handle};

impl AssetServer {
    /// Loads a texture atlas and returns a handle. Returns the cached handle if the same path is loaded again.
    ///
    /// Also loads the underlying image (`Handle<ImageAsset>`) internally.
    pub fn load_atlas(
        &mut self,
        path: impl AsRef<Path>,
        cols: u32,
        rows: u32,
    ) -> Handle<crate::atlas::TextureAtlas> {
        let key = asset_key(path.as_ref());
        if let Some(&id) = self.atlas_path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        let img_handle = self.load_image(path.as_ref());
        let id = alloc_id();
        let atlas = crate::atlas::TextureAtlas {
            handle: img_handle,
            cols,
            rows,
        };
        self.atlases.insert(id, atlas);
        self.atlas_path_to_id.insert(Arc::clone(&key), id);
        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }

    /// Registers a texture atlas whose sprite sheet is already in memory under `key`, returning a
    /// handle — **no file is read**.
    ///
    /// The byte-source twin of [`load_atlas`](Self::load_atlas), for a sheet embedded at compile
    /// time with `include_bytes!` (a single-file / jam build) or produced at runtime. The grid is
    /// the same uniform `cols × rows` layout; only where the pixels come from differs.
    ///
    /// The `key` is used **verbatim** as the atlas cache key, as the handle path, and — because the
    /// underlying image is registered under the same string — as the renderer's texture-cache key,
    /// so an [`AtlasSprite`](crate::AtlasSprite) built on the returned handle renders exactly like a
    /// path-loaded atlas. It is a *logical identifier*, not a path: never resolved against the asset
    /// root or canonicalized, so it cannot collide with a real file and needs no file on disk.
    /// Loading the same `key` again returns the cached handle (the bytes are ignored on a hit).
    ///
    /// Cross-platform, including wasm, for the same reason as
    /// [`load_image_bytes`](Self::load_image_bytes): the bytes are already present, so nothing is
    /// fetched. On a decode failure the atlas still registers (tiling the magenta fallback) and the
    /// failure is recorded via [`asset_failures`](crate::asset_path::asset_failures).
    pub fn load_atlas_bytes(
        &mut self,
        key: impl Into<String>,
        bytes: &[u8],
        cols: u32,
        rows: u32,
    ) -> Handle<crate::atlas::TextureAtlas> {
        let key: Arc<str> = Arc::from(key.into());
        if let Some(&id) = self.atlas_path_to_id.get(&key) {
            return Handle {
                id,
                path: key,
                _marker: PhantomData,
            };
        }
        // The image goes in under the SAME verbatim key, so `atlas.texture_path()` and the
        // renderer's texture-cache key are one string. Diverging them is the 2026-05-29
        // white-sprite bug.
        let img_handle = self.load_image_bytes(&*key, bytes);
        let id = alloc_id();
        let atlas = crate::atlas::TextureAtlas {
            handle: img_handle,
            cols,
            rows,
        };
        self.atlases.insert(id, atlas);
        self.atlas_path_to_id.insert(Arc::clone(&key), id);
        Handle {
            id,
            path: key,
            _marker: PhantomData,
        }
    }

    /// Looks up `TextureAtlas` data by its handle.
    pub fn get_atlas(
        &self,
        handle: &Handle<crate::atlas::TextureAtlas>,
    ) -> Option<&crate::atlas::TextureAtlas> {
        self.atlases.get(&handle.id)
    }
}
