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
