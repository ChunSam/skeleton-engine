//! Engine-level ECS resources, grouped by concern.
//!
//! These were historically one flat `resources.rs`; they are split into focused submodules but
//! all re-exported here, so `crate::resources::Foo` (and the `engine::Foo` re-exports in `lib.rs`)
//! resolve unchanged.

mod debug_draw;
mod display;
mod fonts;
mod lifecycle;
mod profiling;
mod render;
mod time;

pub use debug_draw::{DebugDraw, DebugShape};
pub use display::{
    DesignResolution, DisplayScaleFactor, ImeConfig, Letterbox, PendingResize, ViewportSize,
    WindowConfig, WindowMode, WindowOptions, DEFAULT_CANVAS_ID, DEFAULT_CLEAR_COLOR,
};
pub use fonts::{ExtraFonts, FontData};
pub use lifecycle::{FadeTransition, GameState, LoadProgress, PanickedSystems, ShouldQuit};
pub use profiling::{ProfilerData, RenderStats, SelectedEntity, SystemProfile};
pub use render::{AmbientLight, CullConfig};
pub use time::{RealDt, TimeScale};
