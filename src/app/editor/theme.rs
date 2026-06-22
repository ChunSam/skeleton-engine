//! Central editor theming constants — the visual magic numbers (colors, z-biases, overlay
//! line/font sizes, docked-panel dimensions) shared across the editor chrome. Collected here
//! so the look is tweakable in one place instead of being scattered as inline literals across
//! `ui/gizmo.rs`, `ui/grid_overlay.rs`, and `ui/docked.rs`.
//!
//! Behavior-preserving: every constant equals the literal it replaced. (The game-facing
//! `SliderStyle` / `CheckBoxStyle` widget defaults in `src/ui/` are deliberately NOT pulled in
//! here — they are reusable widget styling, not editor chrome, and already live as named
//! `Default` fields.)
//!
//! Gating mirrors the call sites: nearly all editor chrome is native-only, so most constants are
//! `#[cfg(not(target_arch = "wasm32"))]`. The one exception is [`GIZMO_SELECT_COLOR`], used by the
//! screen-space UI-node selection highlight in `gizmo.rs::update_ui_node_gizmo`, which compiles
//! (dead) on wasm — so that one stays cross-platform.

use crate::color::Color;

// ── Transform gizmo overlay (DebugDraw, engine `Color`) ──────────────────────

/// Selection-highlight fill (cyan) drawn over the selected entity. Cross-platform: the
/// screen-space UI-node gizmo that uses it compiles on wasm (`allow(dead_code)`).
pub(in crate::app::editor) const GIZMO_SELECT_COLOR: Color = Color::rgba(0.2, 0.85, 1.0, 0.65);
/// Resize-handle fill (white).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const GIZMO_HANDLE_COLOR: Color = Color::rgba(1.0, 1.0, 1.0, 0.9);
/// Rotation-handle fill (green).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const GIZMO_ROTATE_COLOR: Color = Color::rgba(0.3, 1.0, 0.4, 0.95);
/// Z-bias added to the entity's z so the selection highlight draws above it. Cross-platform
/// (paired with [`GIZMO_SELECT_COLOR`] in the wasm-compiled selection-highlight path).
pub(in crate::app::editor) const GIZMO_SELECT_Z_BIAS: f32 = 999.0;
/// Z-bias for the resize / rotation handles (above the highlight).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const GIZMO_HANDLE_Z_BIAS: f32 = 1000.0;

// ── Viewport grid overlay (egui) ─────────────────────────────────────────────

/// Grid-line stroke width (logical px).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const GRID_LINE_WIDTH: f32 = 1.0;
/// Grid-line color alpha (white), fed to `Color32::from_white_alpha`.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const GRID_LINE_ALPHA: u8 = 26;
/// Cursor coordinate-readout text alpha (white).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const CURSOR_READOUT_ALPHA: u8 = 190;
/// Cursor coordinate-readout monospace font size (logical px).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const CURSOR_READOUT_FONT_SIZE: f32 = 12.0;

// ── Docked panels (egui) ─────────────────────────────────────────────────────

/// Central viewport panel background fill.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const VIEWPORT_FRAME_FILL: egui::Color32 =
    egui::Color32::from_rgb(20, 20, 25);
/// Bottom assets/data/audio panel: default height + drag range (logical px). The high upper
/// bound lets the data-table grid be dragged tall; egui still clamps to the available height.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const BOTTOM_PANEL_DEFAULT_H: f32 = 200.0;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const BOTTOM_PANEL_MIN_H: f32 = 60.0;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const BOTTOM_PANEL_MAX_H: f32 = 2000.0;
/// Left entities/scene panel: default width + drag range (logical px).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const LEFT_PANEL_DEFAULT_W: f32 = 260.0;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const LEFT_PANEL_MIN_W: f32 = 120.0;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const LEFT_PANEL_MAX_W: f32 = 500.0;
/// Right inspector panel: default width + drag range (logical px).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const RIGHT_PANEL_DEFAULT_W: f32 = 300.0;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const RIGHT_PANEL_MIN_W: f32 = 120.0;
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app::editor) const RIGHT_PANEL_MAX_W: f32 = 600.0;
