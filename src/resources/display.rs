//! Display resources — viewport size, fixed design resolution + letterbox transform, HiDPI scale,
//! window configuration / options, IME, and the pending-resize request.

use glam::Vec2;

/// Current viewport size in game-coordinate space.
///
/// On native Retina/HiDPI displays, the GPU surface is in physical pixels while game
/// coordinates are in logical pixels. Keep this value in logical pixels so sprites and
/// UI render at the intended size.
#[derive(Debug, Clone, Copy)]
pub struct ViewportSize {
    pub width: f32,
    pub height: f32,
}

impl Default for ViewportSize {
    fn default() -> Self {
        Self {
            width: 1280.0,
            height: 720.0,
        }
    }
}

impl ViewportSize {
    pub fn new(w: u32, h: u32) -> Self {
        Self {
            width: w as f32,
            height: h as f32,
        }
    }
}

/// Optional **fixed design (virtual) resolution**. Insert before `App::run()` to author the
/// whole game at one logical canvas size (e.g. `1280×720`) regardless of the real window size:
/// the engine reports this size as [`ViewportSize`] to game systems, renders all content
/// (`DrawRect`/`DrawText`/`UiImageQueue`/sprites/particles) in that design space, then scales
/// it to the window with a **uniform, centered scale and letterbox bars** (the [`Letterbox`]
/// resource carries the computed transform). Cursor input and `Camera::screen_to_world` are
/// mapped back into design space, so input still lines up.
///
/// Absent (the default), `ViewportSize` equals the real window size and nothing is scaled —
/// existing games are unaffected. Ignored in the docked editor (the editor owns the viewport).
///
/// ```no_run
/// # use engine::{App, DesignResolution};
/// let mut app = App::new();
/// // Author at 1280×720; ship at any 16:9 window (e.g. 1920×1080) with identical layout.
/// app.world.insert_resource(DesignResolution::new(1280.0, 720.0));
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DesignResolution {
    pub width: f32,
    pub height: f32,
}

impl DesignResolution {
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// The computed **design-resolution → window** letterbox transform, inserted every frame by the
/// engine (identity when no [`DesignResolution`] is set, so it is always a safe no-op to apply).
///
/// - `clip_scale` — a clip-space scale applied to each scene projection. Because the letterbox is
///   centered, the transform is a pure scale (no translation): one axis is `1.0` and the other
///   `< 1.0` (pillarbox / letterbox). `(1.0, 1.0)` = no scaling.
/// - `px_scale` / `px_offset` — the uniform logical-pixel scale and centering offset used to map a
///   design-space point to a window-logical point (for text rendering and cursor mapping).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Letterbox {
    pub clip_scale: Vec2,
    pub px_scale: f32,
    pub px_offset: Vec2,
}

impl Default for Letterbox {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Letterbox {
    /// The no-op transform: design space == window space.
    pub const IDENTITY: Self = Self {
        clip_scale: Vec2::ONE,
        px_scale: 1.0,
        px_offset: Vec2::ZERO,
    };

    /// Computes the uniform centered letterbox that fits a `design_w × design_h` canvas into a
    /// `win_w × win_h` window (all in logical pixels).
    pub fn compute(design_w: f32, design_h: f32, win_w: f32, win_h: f32) -> Self {
        if design_w <= 0.0 || design_h <= 0.0 || win_w <= 0.0 || win_h <= 0.0 {
            return Self::IDENTITY;
        }
        // Uniform scale that fits the design canvas inside the window (the smaller axis ratio).
        let s = (win_w / design_w).min(win_h / design_h);
        let content_w = design_w * s;
        let content_h = design_h * s;
        Self {
            // Fraction of the window each axis fills → the centered clip-space scale.
            clip_scale: Vec2::new(content_w / win_w, content_h / win_h),
            px_scale: s,
            px_offset: Vec2::new((win_w - content_w) * 0.5, (win_h - content_h) * 0.5),
        }
    }

    /// `true` when this is the identity transform (no design resolution active).
    pub fn is_identity(&self) -> bool {
        self.clip_scale == Vec2::ONE && self.px_offset == Vec2::ZERO && self.px_scale == 1.0
    }

    /// Maps a window-logical point (e.g. the raw cursor position) back into design space — the
    /// inverse of the centered scale. Used so cursor hit-testing lines up with the scaled UI.
    pub fn window_to_design(&self, window_logical: Vec2) -> Vec2 {
        if self.px_scale <= 0.0 {
            return window_logical;
        }
        (window_logical - self.px_offset) / self.px_scale
    }
}

/// Scale factor: how many physical pixels correspond to one logical pixel.
#[derive(Debug, Clone, Copy)]
pub struct DisplayScaleFactor(pub f32);

impl Default for DisplayScaleFactor {
    fn default() -> Self {
        Self(1.0)
    }
}

/// HTML element id the wasm backend binds the winit window / wgpu surface to.
///
/// The single source of truth shared by the window setup and the GPU context (forks that
/// embed the canvas under a different id only change it here). Mirror the value in the page's
/// `<canvas id="...">`.
pub const DEFAULT_CANVAS_ID: &str = "game-canvas";

/// Initial window configuration. Insert before `App::run()` to open the window with these settings.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    pub width: u32,
    pub height: u32,
    pub title: String,
    /// Background clear color (RGBA, wgpu linear-space f64).
    pub clear_color: [f64; 4],
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1280,
            height: 720,
            title: "Game".to_string(),
            clear_color: [0.08, 0.08, 0.12, 1.0],
        }
    }
}

/// How the OS window is displayed. Default [`WindowMode::Windowed`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowMode {
    /// A normal windowed window at the [`WindowConfig`] size.
    #[default]
    Windowed,
    /// Borderless fullscreen on the current monitor (covers the screen at its native
    /// resolution, no mode switch). The [`WindowConfig`] width/height still set the initial
    /// inner size used before fullscreen is applied.
    BorderlessFullscreen,
}

/// Optional window behavior — resizability, fullscreen mode, and aspect-ratio lock.
///
/// Like [`ImeConfig`], this is a separate opt-in resource: **insert it before `App::run()`**
/// to override the defaults. When absent, the engine uses the same defaults as
/// [`WindowOptions::default`] — a normal resizable windowed window — so existing games are
/// unaffected (no [`WindowConfig`] field was added, so no call site breaks).
///
/// ```no_run
/// # use engine::{App, WindowOptions};
/// let mut app = App::new();
/// // A fixed-aspect game: a non-resizable window can't be dragged to a new aspect.
/// app.world.insert_resource(WindowOptions { resizable: false, ..Default::default() });
/// ```
#[derive(Debug, Clone)]
pub struct WindowOptions {
    /// Whether the OS window can be resized by dragging its edges. Default `true`
    /// (winit's default). Set `false` for a fixed-size window whose aspect can never be
    /// distorted by a resize — the simplest, most reliable way to keep a fixed-aspect UI
    /// aligned.
    pub resizable: bool,
    /// Windowed vs. borderless-fullscreen. Default [`WindowMode::Windowed`].
    pub mode: WindowMode,
    /// Optional aspect-ratio lock as `width / height` (e.g. `16.0 / 9.0`). When `Some`, a
    /// live resize is corrected back to this ratio (height re-derived from the new width) so
    /// the window can grow and shrink but never changes aspect. `None` (default) = no lock.
    /// Ignored when `resizable == false` (a fixed window already can't change aspect) and on
    /// wasm (the canvas size is owned by the HTML page).
    pub lock_aspect: Option<f32>,
}

impl Default for WindowOptions {
    fn default() -> Self {
        // Mirrors the engine's no-resource behavior: a normal resizable windowed window with
        // no aspect lock. `#[derive(Default)]` would give `resizable: false`, the opposite of
        // winit's default — so spell it out.
        Self {
            resizable: true,
            mode: WindowMode::Windowed,
            lock_aspect: None,
        }
    }
}

/// Whether text input (IME) is allowed. Default is **off**.
///
/// Most games do not need text input. When IME is enabled, on macOS and similar
/// platforms, keyUp events for game keys can be absorbed by CJK composition (Korean,
/// Japanese, Chinese), leaving a key "stuck" (e.g. an acceleration key never releases
/// and the character keeps running). Therefore the default is off; only apps that
/// actually need text input — `TextInput` widgets, dialog boxes, etc. — should insert
/// `ImeConfig { allowed: true }` before `App::run()`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ImeConfig {
    pub allowed: bool,
}

/// Pending resize request. When a game system sets this to `Some((w, h))`, `App` resizes the window.
#[derive(Debug, Clone, Copy, Default)]
pub struct PendingResize(pub Option<(u32, u32)>);

#[cfg(test)]
mod letterbox_tests {
    use super::*;

    #[test]
    fn same_aspect_larger_window_has_no_bars() {
        // 1280×720 design into a 1920×1080 window (both 16:9): uniform 1.5× scale, no letterbox.
        let lb = Letterbox::compute(1280.0, 720.0, 1920.0, 1080.0);
        assert!((lb.clip_scale.x - 1.0).abs() < 1e-5);
        assert!((lb.clip_scale.y - 1.0).abs() < 1e-5);
        assert!((lb.px_scale - 1.5).abs() < 1e-5);
        assert_eq!(lb.px_offset, Vec2::ZERO);
    }

    #[test]
    fn taller_window_letterboxes_top_bottom() {
        // 1280×720 design into a 1000×1000 window (taller than 16:9): top/bottom bars.
        let lb = Letterbox::compute(1280.0, 720.0, 1000.0, 1000.0);
        let s = 1000.0 / 1280.0; // width-limited
        assert!((lb.px_scale - s).abs() < 1e-5);
        assert!((lb.clip_scale.x - 1.0).abs() < 1e-5, "x fills the window");
        let content_h = 720.0 * s;
        assert!((lb.clip_scale.y - content_h / 1000.0).abs() < 1e-5);
        assert!((lb.px_offset.x).abs() < 1e-5);
        assert!((lb.px_offset.y - (1000.0 - content_h) / 2.0).abs() < 1e-5);
    }

    #[test]
    fn wider_window_pillarboxes_left_right() {
        // 1280×720 design into a 2000×600 window (wider than 16:9): left/right bars.
        let lb = Letterbox::compute(1280.0, 720.0, 2000.0, 600.0);
        let s = 600.0 / 720.0; // height-limited
        assert!((lb.px_scale - s).abs() < 1e-5);
        assert!((lb.clip_scale.y - 1.0).abs() < 1e-5, "y fills the window");
        let content_w = 1280.0 * s;
        assert!((lb.clip_scale.x - content_w / 2000.0).abs() < 1e-5);
    }

    #[test]
    fn window_to_design_round_trips_window_center() {
        // The window center maps to the design center under any letterbox.
        let lb = Letterbox::compute(1280.0, 720.0, 1000.0, 1000.0);
        let design = lb.window_to_design(Vec2::new(500.0, 500.0));
        assert!((design.x - 640.0).abs() < 1e-3, "x={}", design.x);
        assert!((design.y - 360.0).abs() < 1e-3, "y={}", design.y);
    }

    #[test]
    fn identity_is_a_noop() {
        let lb = Letterbox::IDENTITY;
        assert!(lb.is_identity());
        let p = Vec2::new(123.0, 456.0);
        assert_eq!(lb.window_to_design(p), p);
        // No design resolution / degenerate inputs fall back to identity.
        assert!(Letterbox::compute(0.0, 720.0, 1920.0, 1080.0).is_identity());
    }

    #[test]
    fn degenerate_window_to_design_does_not_divide_by_zero() {
        let lb = Letterbox {
            clip_scale: Vec2::ONE,
            px_scale: 0.0,
            px_offset: Vec2::ZERO,
        };
        let p = Vec2::new(10.0, 20.0);
        assert_eq!(lb.window_to_design(p), p);
    }
}
