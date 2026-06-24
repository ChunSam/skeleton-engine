//! Rendering-config resources — view culling / LOD and scene-wide ambient light.

use crate::color::Color;

/// View frustum culling + distance-based LOD settings.
///
/// Insert before `App::run()`, or modify at runtime via `world.resource_mut::<CullConfig>()`.
/// If not inserted, the engine defaults apply (`frustum_culling: true, min_pixel_size: 0.0`).
///
/// ```text
/// world.insert_resource(CullConfig {
///     frustum_culling: true,
///     min_pixel_size: 1.0,  // skip sprites smaller than 1 px on screen
/// });
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CullConfig {
    /// When `true`, sprites outside the camera viewport are culled before GPU submission.
    pub frustum_culling: bool,
    /// Sprites whose screen-space size (min(w, h) in pixels) is below this value are skipped.
    /// `0.0` disables distance LOD.
    pub min_pixel_size: f32,
}

impl Default for CullConfig {
    fn default() -> Self {
        Self {
            frustum_culling: true,
            min_pixel_size: 0.0,
        }
    }
}

/// Scene-wide ambient light resource.
///
/// Registering via `world.insert_resource(AmbientLight::default())` activates
/// `LightingRenderer`. Use together with the `PointLight` component.
///
/// **Platform note:** lighting is native-only. On `wasm32` targets this resource is
/// accepted but the lighting render pass is silently skipped (no-op on wasm32).
///
/// ```rust,no_run
/// # use engine::{App, AmbientLight};
/// # let mut app = App::new();
/// app.world.insert_resource(AmbientLight {
///     color: engine::Color::rgb(0.2, 0.2, 0.3),
///     intensity: 0.05,
/// });
/// ```
#[derive(Debug, Clone, Copy)]
pub struct AmbientLight {
    /// Ambient light RGB color (0.0–1.0).
    pub color: Color,
    /// 0.0 = fully dark, 1.0 = original brightness.
    pub intensity: f32,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 0.1,
        }
    }
}
