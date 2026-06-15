use crate::asset::{Handle, ImageAsset};
use crate::color::Color;
use crate::renderer::uv::UvRect;

/// Uniform-grid texture atlas.
///
/// A texture with multiple sprites arranged in a fixed-size grid in a single image file.
/// Divided into `cols × rows` tiles; indices run from top-left (0) to bottom-right.
///
/// # Example
/// 4×4 atlas: 16 tiles total, index 5 → row 1 col 1
#[derive(Clone, Debug)]
pub struct TextureAtlas {
    /// Handle for the full atlas image
    pub handle: Handle<ImageAsset>,
    /// Number of columns
    pub cols: u32,
    /// Number of rows
    pub rows: u32,
}

impl TextureAtlas {
    /// Returns the UV coordinates for the given index (normalized 0.0–1.0).
    ///
    /// If the index is out of range it wraps via `% (cols * rows)`.
    pub fn uv_rect(&self, index: u32) -> UvRect {
        if self.cols == 0 || self.rows == 0 {
            return UvRect::FULL;
        }
        let total = self.cols * self.rows;
        let index = if total == 0 { 0 } else { index % total };
        let col = index % self.cols;
        let row = index / self.cols;
        UvRect::from_grid(col, row, self.cols, self.rows)
    }

    /// Image file path for this atlas (used as the renderer texture cache key).
    pub fn texture_path(&self) -> &str {
        self.handle.path()
    }

    /// Returns the atlas image path as an `Arc<str>` — O(1) refcount bump, no heap copy.
    ///
    /// Prefer this over `Arc::from(atlas.texture_path())` in hot render paths (e.g.
    /// per-AtlasSprite texture key) where a new allocation per frame would be wasteful.
    pub fn texture_path_arc(&self) -> std::sync::Arc<str> {
        self.handle.path_arc()
    }
}

/// Component that renders a specific tile from a texture atlas.
///
/// Add to an entity alongside a `Transform` component.
/// Processed in the same render pass as `Sprite`, so all rendering behavior
/// (z-order, blending, etc.) is identical.
///
/// # Example
/// ```rust,no_run
/// # use engine::{App, AtlasSprite, Transform};
/// # use glam::Vec2;
/// # let mut app = App::new();
/// let atlas = app.load_atlas("assets/characters.png", 4, 4);
/// let e = app.world.spawn();
/// app.world.add_component(e, Transform::default());
/// app.world.add_component(e, AtlasSprite::new(atlas, 3));
/// ```
#[derive(Clone, Debug)]
pub struct AtlasSprite {
    /// Atlas handle (managed by AssetServer)
    pub atlas: Handle<TextureAtlas>,
    /// Tile index within the atlas (0-based, top-left → bottom-right)
    pub index: u32,
    /// RGBA color multiplier (default white = original texture color)
    pub color: Color,
}

impl AtlasSprite {
    pub fn new(atlas: Handle<TextureAtlas>, index: u32) -> Self {
        Self {
            atlas,
            index,
            color: Color::WHITE,
        }
    }

    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle() -> Handle<ImageAsset> {
        crate::asset::AssetServer::new().load_image("__test_missing_atlas.png")
    }

    #[test]
    fn atlas_row_zero_uses_top_row_uv() {
        let atlas = TextureAtlas {
            handle: handle(),
            cols: 4,
            rows: 2,
        };

        assert_eq!(atlas.uv_rect(0), UvRect::new(0.0, 0.0, 0.25, 0.5));
        assert_eq!(atlas.uv_rect(1), UvRect::new(0.25, 0.0, 0.25, 0.5));
    }

    #[test]
    fn atlas_uv_rect_wraps_grid_index() {
        let atlas = TextureAtlas {
            handle: handle(),
            cols: 4,
            rows: 2,
        };

        assert_eq!(atlas.uv_rect(5), UvRect::from_grid(1, 1, 4, 2));
        assert_eq!(atlas.uv_rect(9), UvRect::from_grid(1, 0, 4, 2));
    }

    #[test]
    fn atlas_with_empty_grid_uses_full_uv() {
        let atlas = TextureAtlas {
            handle: handle(),
            cols: 0,
            rows: 2,
        };

        assert_eq!(atlas.uv_rect(0), UvRect::FULL);
    }

    #[test]
    fn atlas_texture_path_uses_image_handle_path() {
        let handle = handle();
        let atlas = TextureAtlas {
            handle: handle.clone(),
            cols: 4,
            rows: 2,
        };

        assert_eq!(atlas.texture_path(), handle.path());
    }

    #[test]
    fn atlas_texture_path_arc_matches_path_and_is_refcount_bump() {
        let handle = handle();
        let atlas = TextureAtlas {
            handle: handle.clone(),
            cols: 4,
            rows: 2,
        };
        let arc = atlas.texture_path_arc();
        // Arc<str> content matches the path string.
        assert_eq!(&*arc, atlas.texture_path());
        // A second call produces an equal Arc (same content); both point to the same
        // backing allocation (pointer equality confirms O(1) refcount bump).
        let arc2 = atlas.texture_path_arc();
        assert!(
            std::sync::Arc::ptr_eq(&arc, &arc2),
            "texture_path_arc must return the same Arc (refcount bump, not a copy)"
        );
    }
}
