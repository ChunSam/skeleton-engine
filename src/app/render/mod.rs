//! Frame-rendering orchestration for [`App`](crate::App), split across focused
//! submodules:
//!
//! - [`debug_draw`] — `DebugShape` → `DrawRect` conversion.
//! - [`offscreen`] — `OffscreenCamera`/`RenderTarget` offscreen rendering.
//! - [`docked`] — docked-editor render-target handling (native-only).
//! - [`post_lighting`] — post-process + lighting setup/run.
//! - [`frame`] — top-level frame orchestration (`render`, `step_frame`).
//!
//! Each submodule adds methods to the same `App` via separate `impl App` blocks.

mod debug_draw;
mod frame;
mod offscreen;
mod post_lighting;

#[cfg(not(target_arch = "wasm32"))]
mod docked;

/// Clear color for the docked-editor surface (the area egui draws the editor chrome onto, both
/// the warm-up placeholder and the post-scene docked clear).
///
/// Holds the same value as [`DEFAULT_CLEAR_COLOR`](crate::resources::DEFAULT_CLEAR_COLOR) but is
/// intentionally a separate constant: a game changing its own `WindowConfig::clear_color` should
/// not repaint the editor letterbox to match.
const EDITOR_SURFACE_CLEAR: wgpu::Color = wgpu::Color {
    r: 0.08,
    g: 0.08,
    b: 0.12,
    a: 1.0,
};
