use std::marker::PhantomData;
use std::path::Path;
use std::sync::Arc;

use super::{alloc_id, asset_key, AssetServer, Handle};

impl AssetServer {
    /// 텍스처 아틀라스를 로드하고 핸들을 반환한다. 같은 경로를 다시 호출하면 캐시된 핸들을 반환한다.
    ///
    /// 내부적으로 이미지(`Handle<ImageAsset>`)도 함께 로드한다.
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

    /// 아틀라스 핸들로 `TextureAtlas` 데이터를 조회한다.
    pub fn get_atlas(
        &self,
        handle: &Handle<crate::atlas::TextureAtlas>,
    ) -> Option<&crate::atlas::TextureAtlas> {
        self.atlases.get(&handle.id)
    }
}
