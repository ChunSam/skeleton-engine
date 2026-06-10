use glam::{Mat4, Quat, Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::asset::{Handle, ImageAsset};
use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

// ─── Render components ────────────────────────────────────────────────────────────

/// Component holding position, scale, and rotation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec2,
    pub scale: Vec2,
    /// Rotation angle in radians, around the Z axis
    pub rotation: f32,
    // Higher z is drawn on top (sprites are rendered in ascending z order).
    pub z: f32,
}

impl Transform {
    pub fn new(position: Vec2, scale: Vec2, rotation: f32) -> Self {
        Self {
            position,
            scale,
            rotation,
            z: 0.0,
        }
    }

    /// Builds the 4×4 model matrix to send from ECS to the GPU
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            Vec3::new(self.scale.x, self.scale.y, 1.0),
            Quat::from_rotation_z(self.rotation),
            Vec3::new(self.position.x, self.position.y, 0.0),
        )
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE * 64.0,
            rotation: 0.0,
            z: 0.0,
        }
    }
}

/// Component holding sprite appearance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    /// Texture file path (None renders a solid-color rectangle). RON serialization supported.
    pub texture: Option<String>,
    /// RGBA color multiplier (white = original texture color)
    pub color: Color,
    /// Image handle loaded through AssetServer. Not serialized — runtime only.
    /// Takes priority over `texture`.
    #[serde(skip)]
    pub image_handle: Option<Handle<ImageAsset>>,
}

impl Sprite {
    pub fn colored(r: f32, g: f32, b: f32) -> Self {
        Self {
            texture: None,
            color: Color::rgb(r, g, b),
            image_handle: None,
        }
    }

    pub fn textured(path: impl Into<String>) -> Self {
        Self {
            texture: Some(path.into()),
            color: Color::WHITE,
            image_handle: None,
        }
    }

    /// Specifies a texture via an AssetServer handle. Takes priority over the `texture` path.
    pub fn with_handle(handle: Handle<ImageAsset>) -> Self {
        Self {
            texture: None,
            color: Color::WHITE,
            image_handle: Some(handle),
        }
    }

    /// Preserves the path fallback while preferring the image handle when present.
    pub fn textured_with_handle(
        path: impl Into<String>,
        handle: Option<Handle<ImageAsset>>,
    ) -> Self {
        Self {
            texture: Some(path.into()),
            color: Color::WHITE,
            image_handle: handle,
        }
    }
}

impl Default for Sprite {
    fn default() -> Self {
        Self::colored(1.0, 1.0, 1.0)
    }
}

#[cfg(test)]
mod sprite_tests {
    use super::*;

    #[test]
    fn textured_with_handle_keeps_path_fallback() {
        let handle = crate::asset::AssetServer::new().load_image("__test_missing_sprite.png");
        let sprite = Sprite::textured_with_handle("fallback.png", Some(handle));

        assert_eq!(sprite.texture.as_deref(), Some("fallback.png"));
        assert!(sprite.image_handle.is_some());
    }

    #[test]
    fn textured_with_handle_accepts_no_handle() {
        let sprite = Sprite::textured_with_handle("fallback.png", None);

        assert_eq!(sprite.texture.as_deref(), Some("fallback.png"));
        assert!(sprite.image_handle.is_none());
    }
}

// ─── Reflect impls ─────────────────────────────────────────────────────────────

impl Reflect for Transform {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("x", ReflectValue::F32(self.position.x)),
            ("y", ReflectValue::F32(self.position.y)),
            ("rotation", ReflectValue::F32(self.rotation)),
            ("scale_x", ReflectValue::F32(self.scale.x)),
            ("scale_y", ReflectValue::F32(self.scale.y)),
            ("z", ReflectValue::F32(self.z)),
        ]
    }
    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("x", ReflectValue::F32(v)) => {
                self.position.x = v;
                true
            }
            ("y", ReflectValue::F32(v)) => {
                self.position.y = v;
                true
            }
            ("rotation", ReflectValue::F32(v)) => {
                self.rotation = v;
                true
            }
            ("scale_x", ReflectValue::F32(v)) => {
                self.scale.x = v;
                true
            }
            ("scale_y", ReflectValue::F32(v)) => {
                self.scale.y = v;
                true
            }
            ("z", ReflectValue::F32(v)) => {
                self.z = v;
                true
            }
            _ => false,
        }
    }
    fn type_name(&self) -> &'static str {
        "Transform"
    }
}

impl Reflect for Sprite {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("color", ReflectValue::Color(self.color.to_array())),
            (
                "texture",
                ReflectValue::String(self.texture.clone().unwrap_or_default()),
            ),
        ]
    }
    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("color", ReflectValue::Color(c)) => {
                self.color = Color::from(c);
                true
            }
            ("texture", ReflectValue::String(s)) => {
                self.texture = if s.is_empty() { None } else { Some(s) };
                true
            }
            _ => false,
        }
    }
    fn type_name(&self) -> &'static str {
        "Sprite"
    }
}

// ─── RenderLayer ──────────────────────────────────────────────────────────────

/// Sprite rendering layer (optional component, default 0).
///
/// Lower values are drawn first (further back). Within the same layer,
/// sprites are batched by texture key and then rendered in ascending z order.
///
/// # Example
/// ```rust,no_run
/// # use engine::{RenderLayer, ecs::World};
/// # let mut world = World::new();
/// # let bg = world.spawn();
/// # let effect = world.spawn();
/// // Background layer (-1): always drawn behind gameplay
/// // Default layer   ( 0): most game objects
/// // Foreground layer ( 1): HUD, effects, etc. that must always appear in front
/// world.add_component(bg, RenderLayer(-1));
/// world.add_component(effect, RenderLayer(1));
/// ```
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub struct RenderLayer(pub i32);

// ─── PointLight ───────────────────────────────────────────────────────────────

/// World-space point light component.
///
/// Used together with the `AmbientLight` resource. Adding it to an entity alongside
/// `Transform` causes `LightingRenderer` to include it in the lighting pass automatically.
///
/// ```rust,no_run
/// # use engine::{App, PointLight, AmbientLight, components::Transform};
/// # use glam::Vec2;
/// # let mut app = App::new();
/// # let e = app.world.spawn();
/// app.world.insert_resource(engine::AmbientLight { intensity: 0.05, ..Default::default() });
/// app.world.add_component(e, Transform { position: Vec2::new(400.0, 300.0), ..Default::default() });
/// app.world.add_component(e, PointLight {
///     color: engine::Color::rgb(1.0, 0.9, 0.6),
///     radius: 300.0,
///     intensity: 1.5,
///     ..Default::default()
/// });
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    /// RGB color (0.0–1.0)
    pub color: Color,
    /// Radius in world-space pixels
    pub radius: f32,
    /// Brightness multiplier
    pub intensity: f32,
    /// Virtual Z height of the light source (used for flat-normal lighting direction). Recommended range: 0.05–1.0.
    pub light_height: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            radius: 200.0,
            intensity: 1.0,
            light_height: 0.15,
        }
    }
}

// ─── OffscreenCamera ──────────────────────────────────────────────────────────

/// Attaching this component to an entity causes offscreen rendering every frame
/// to the specified `RenderTarget` from the perspective of `camera`.
///
/// # Layer Mask
///
/// `layer_mask` is a bitmask of `RenderLayer` values to render.
/// When 0 (the default), all layers are rendered without filtering.
///
/// ```rust,no_run
/// # use engine::{OffscreenCamera, RenderLayer};
/// // Render only layer 0 (game world) — excludes layer 1 (HUD/minimap UI)
/// let cam = OffscreenCamera {
///     target: "minimap".to_string(),
///     camera: Default::default(),
///     layer_mask: 1 << 0,  // bit 0 = RenderLayer(0)
/// };
/// ```
#[derive(Clone, Default)]
pub struct OffscreenCamera {
    /// Name registered with `App::create_render_target` (the RenderTarget key)
    pub target: String,
    /// Dedicated camera for this view (operates independently of the main camera)
    pub camera: crate::camera::Camera,
    /// Bitmask of RenderLayer values to render. 0 = allow all layers (default, backward-compatible).
    /// `RenderLayer(n)` (n ∈ 0..=31) maps to bit n. Layers outside 0..=31
    /// (negative or above 31) cannot be expressed in a 32-bit mask, so they are
    /// always excluded from non-zero masks (they render normally when mask is 0).
    /// Example: a `RenderLayer(-1)` background is excluded from a `layer_mask: 1 << 0`
    /// (layer-0-only) pass.
    pub layer_mask: u32,
}

// ─── Backward-compatible re-exports ─────────────────────────────────────────────────────────
// Migration facade: these types were formerly defined here but now live in their canonical
// modules (animation, renderer, resources).  The re-exports keep existing `components::*`
// import paths compiling.  Audited for removal in v5 — do not add new items here.
pub use crate::animation::player::{AnimationClip, AnimationPlayer};
pub use crate::renderer::uv::UvRect;
pub use crate::resources::{
    FontData, GameState, PendingResize, ShouldQuit, ViewportSize, WindowConfig,
};

// ─── Unit tests ───────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_default_z_is_zero() {
        assert_eq!(Transform::default().z, 0.0);
    }

    #[test]
    fn transform_z_assignable() {
        let t = Transform {
            z: 5.0,
            ..Default::default()
        };
        assert_eq!(t.z, 5.0);
    }
}
