use glam::Vec2;

use crate::color::Color;

// ─── Panic Recovery ──────────────────────────────────────────────────────────

/// List of systems disabled due to a panic.
///
/// When `App` catches a panic via `catch_unwind`, it records the system name here
/// and skips that system in subsequent frames.
///
/// ```rust,ignore
/// if let Some(ps) = world.resource::<PanickedSystems>() {
///     for name in &ps.disabled {
///         log::warn!("disabled system: {name}");
///     }
/// }
/// ```
#[derive(Default)]
pub struct PanickedSystems {
    /// Names of systems disabled due to a panic.
    pub disabled: Vec<String>,
}

// ─── General Debug Draw API ──────────────────────────────────────────────────

/// A single debug shape.
///
/// This enum is `#[non_exhaustive]`: external crates matching on it must include a
/// wildcard (`_ =>`) arm to remain forward-compatible as new variants are added.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DebugShape {
    /// Axis-aligned rectangle (outline)
    Rect { min: Vec2, max: Vec2, color: Color },
    /// Line segment (start → end, `thickness` px wide)
    Line {
        start: Vec2,
        end: Vec2,
        color: Color,
        thickness: f32,
    },
    /// Circle (24-sided polygon approximation)
    Circle {
        center: Vec2,
        radius: f32,
        color: Color,
    },
    /// Cross marker (two intersecting lines)
    Cross { pos: Vec2, size: f32, color: Color },
}

/// A filled, z-ordered rectangle collected via [`DebugDraw::rect_filled_z`].
/// Engine-internal: the renderer drains these into `UiQueue` alongside shapes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FilledRect {
    pub min: Vec2,
    pub max: Vec2,
    pub color: Color,
    pub z: f32,
}

/// Resource that collects debug shapes each frame.
///
/// `App` automatically calls `clear()` after rendering, so simply re-draw each frame.
///
/// # Example
/// ```rust,ignore
/// // inside a system
/// if let Some(dbg) = world.resource_mut::<DebugDraw>() {
///     dbg.rect(Vec2::new(0., 0.), Vec2::new(64., 64.), [1., 0., 0., 1.]);
///     dbg.circle(player_pos, 32., [0., 1., 0., 0.8]);
///     dbg.line(from, to, [1., 1., 0., 1.]);
///     dbg.rect_filled_z(min, max, [0.2, 0.2, 0.3, 1.0], 0.5); // filled, z-ordered
/// }
/// ```
#[derive(Debug, Default)]
pub struct DebugDraw {
    pub(crate) shapes: Vec<DebugShape>,
    pub(crate) filled_rects: Vec<FilledRect>,
}

impl DebugDraw {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws an axis-aligned rectangle outline.
    pub fn rect(&mut self, min: Vec2, max: Vec2, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Rect {
            min,
            max,
            color: color.into(),
        });
    }

    /// Draws a line segment (default thickness 1.5 px).
    pub fn line(&mut self, start: Vec2, end: Vec2, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Line {
            start,
            end,
            color: color.into(),
            thickness: 1.5,
        });
    }

    /// Draws a line segment with the given thickness.
    pub fn line_thick(&mut self, start: Vec2, end: Vec2, color: impl Into<Color>, thickness: f32) {
        self.shapes.push(DebugShape::Line {
            start,
            end,
            color: color.into(),
            thickness,
        });
    }

    /// Draws a circle (24-sided polygon approximation).
    pub fn circle(&mut self, center: Vec2, radius: f32, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Circle {
            center,
            radius,
            color: color.into(),
        });
    }

    /// Draws a cross marker.
    pub fn cross(&mut self, pos: Vec2, size: f32, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Cross {
            pos,
            size,
            color: color.into(),
        });
    }

    /// Draws a filled rectangle (z = 0.0).
    pub fn rect_filled(&mut self, min: Vec2, max: Vec2, color: impl Into<Color>) {
        self.rect_filled_z(min, max, color, 0.0);
    }

    /// Draws a filled rectangle at the given z-order (higher = drawn on top).
    ///
    /// This covers what the pre-v5 `DebugRect`/`DebugDrawQueue` pair did —
    /// translucent collision overlays, editor selection highlights, or quick
    /// rect-based prototype rendering (see the `sokoban` example).
    pub fn rect_filled_z(&mut self, min: Vec2, max: Vec2, color: impl Into<Color>, z: f32) {
        self.filled_rects.push(FilledRect {
            min,
            max,
            color: color.into(),
            z,
        });
    }

    /// Clears all shapes for this frame. Called automatically by `App` after rendering.
    pub fn clear(&mut self) {
        self.shapes.clear();
        self.filled_rects.clear();
    }

    /// Returns the slice of collected shapes.
    pub fn shapes(&self) -> &[DebugShape] {
        &self.shapes
    }
}

#[cfg(test)]
mod debug_draw_tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn debug_draw_accumulates_shapes() {
        let mut dbg = DebugDraw::new();
        dbg.rect(Vec2::ZERO, Vec2::ONE * 64., [1., 0., 0., 1.]);
        dbg.circle(Vec2::new(100., 100.), 32., [0., 1., 0., 1.]);
        dbg.line(Vec2::ZERO, Vec2::new(50., 50.), [0., 0., 1., 1.]);
        assert_eq!(dbg.shapes().len(), 3);
    }

    #[test]
    fn debug_draw_clear_empties() {
        let mut dbg = DebugDraw::new();
        dbg.rect(Vec2::ZERO, Vec2::ONE, [1.; 4]);
        dbg.clear();
        assert!(dbg.shapes().is_empty());
    }

    #[test]
    fn debug_draw_cross_is_correct_shape() {
        let mut dbg = DebugDraw::new();
        dbg.cross(Vec2::new(50., 50.), 20., [1.; 4]);
        assert_eq!(dbg.shapes().len(), 1);
        matches!(&dbg.shapes()[0], DebugShape::Cross { .. });
    }

    #[test]
    fn debug_draw_line_thick() {
        let mut dbg = DebugDraw::new();
        dbg.line_thick(Vec2::ZERO, Vec2::new(100., 0.), [1.; 4], 3.0);
        assert_eq!(dbg.shapes().len(), 1);
        if let DebugShape::Line { thickness, .. } = &dbg.shapes()[0] {
            assert_eq!(*thickness, 3.0);
        } else {
            panic!("expected Line shape");
        }
    }
}

// ─── Async Asset Loading Progress ───────────────────────────────────────────

/// Async asset loading progress.
///
/// Tracks the completion ratio of images requested via `App::load_image_async()`.
///
/// # Example
/// ```rust,ignore
/// let prog = world.resource::<LoadProgress>().unwrap();
/// draw_bar(prog.fraction()); // 0.0 ~ 1.0
/// if prog.is_complete() { /* loading done → transition to game scene */ }
/// ```
#[derive(Debug, Clone, Default)]
pub struct LoadProgress {
    /// Total number of async load requests.
    pub total: usize,
    /// Number completed (includes both Loaded and Failed).
    pub loaded: usize,
}

impl LoadProgress {
    /// Returns progress in the range 0.0–1.0. Returns 1.0 when there are no requests.
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            1.0
        } else {
            self.loaded as f32 / self.total as f32
        }
    }

    /// Returns `true` when all async loads have completed.
    pub fn is_complete(&self) -> bool {
        self.loaded >= self.total
    }
}

// ─── Game State Resource ─────────────────────────────────────────────────────

/// Game state machine value (inserted as an ECS resource).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Playing,
    Paused,
    GameOver,
}

/// Quit-request resource. When a system sets this to `true`, `App` exits on the next frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShouldQuit(pub bool);

impl ShouldQuit {
    /// Signal that the application should quit on the next frame.
    pub fn quit(&mut self) {
        self.0 = true;
    }

    /// Returns `true` if a quit has been requested.
    pub fn is_quitting(&self) -> bool {
        self.0
    }
}

/// Global time-scale multiplier applied to the `dt` that gameplay (scene) systems receive.
///
/// `1.0` = normal speed, `0.0` = frozen (hit-stop / pause-with-rendering), `0.5` = slow motion,
/// `2.0` = fast forward. Set it via [`crate::App::set_time_scale`] or
/// `world.resource_mut::<TimeScale>()`.
///
/// Only **scene** systems are scaled. Built-in tail systems (hierarchy/gizmo) and engine
/// post-frame work (fades, hot-reload, asset upload, camera) always run at real time, so the
/// editor and screen transitions stay responsive even at `time_scale = 0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeScale(pub f32);

impl Default for TimeScale {
    fn default() -> Self {
        Self(1.0)
    }
}

impl TimeScale {
    /// The current multiplier, clamped to be non-negative (negative scales make no sense).
    pub fn get(&self) -> f32 {
        self.0.max(0.0)
    }

    /// Sets the multiplier. Negative values are clamped to `0.0` when read via [`get`](Self::get).
    pub fn set(&mut self, scale: f32) {
        self.0 = scale;
    }
}

/// The real (unscaled) frame delta-time in seconds, written every frame before [`TimeScale`]
/// is applied.
///
/// Most systems should just use the `dt` argument (already time-scaled). Read this when a system
/// must run in real time *regardless* of the time scale — e.g. a hit-stop controller that sets
/// `TimeScale(0.0)` still needs real time to count down its own freeze window, otherwise it would
/// freeze itself forever.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct RealDt(pub f32);

// ─── Viewport / Window Config ────────────────────────────────────────────────

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

/// Font bytes used by the game. Insert before `App::run()` for `TextRenderer` to pick it up.
pub struct FontData(pub Vec<u8>);

/// Additional font blobs loaded alongside [`FontData`] for multi-script coverage — e.g. a Latin UI
/// font in `FontData` plus an RTL-script font (Hebrew/Arabic) here. cosmic-text falls back across all
/// loaded fonts by script, so a single `DrawText` containing mixed LTR + RTL text shapes correctly.
/// Insert before `App::run()` for `TextRenderer` to pick it up.
#[derive(Default)]
pub struct ExtraFonts(pub Vec<Vec<u8>>);

/// Pending resize request. When a game system sets this to `Some((w, h))`, `App` resizes the window.
#[derive(Debug, Clone, Copy, Default)]
pub struct PendingResize(pub Option<(u32, u32)>);

// ─── Rendering Optimization ──────────────────────────────────────────────────

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

// ─── Lighting ────────────────────────────────────────────────────────────────

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

/// Exposes the currently selected entity in the Inspector as a World resource.
///
/// `App` synchronizes this with `inspector_selected` every frame.
/// Read from systems for selection highlighting, path planning, and other editor integrations.
///
/// ```text
/// if let Some(e) = world.resource::<SelectedEntity>().and_then(|s| s.0) {
///     // e is the entity currently selected in the Inspector
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectedEntity(pub Option<crate::ecs::world::Entity>);

// ─── Profiler ────────────────────────────────────────────────────────────────

/// Profiling entry for a single system.
#[derive(Debug, Clone, Default)]
pub struct SystemProfile {
    pub name: String,
    /// Execution time of the previous frame (microseconds).
    pub last_us: u64,
    /// Exponential moving average over the last 60 frames (microseconds).
    pub avg_us: f32,
}

/// Renderer pass statistics.
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    /// Number of texture switches (draw call count).
    pub draw_calls: u32,
    /// Number of sprite instances submitted to the GPU.
    pub sprites_rendered: u32,
    /// Number of sprites skipped by view culling / LOD.
    pub sprites_culled: u32,
}

/// Complete profiler data. Updated every frame by `App` and read by the Engine Stats panel.
#[derive(Debug, Clone, Default)]
pub struct ProfilerData {
    pub systems: Vec<SystemProfile>,
    pub render: RenderStats,
    /// Total frame time (ms).
    pub frame_ms: f32,
}

impl ProfilerData {
    /// EMA α = 1/60
    const ALPHA: f32 = 1.0 / 60.0;

    /// Records a system execution result. Automatically expands if `idx` is out of range.
    pub fn record_system(&mut self, idx: usize, name: &str, elapsed_us: u64) {
        if idx >= self.systems.len() {
            self.systems.resize(idx + 1, SystemProfile::default());
        }
        let s = &mut self.systems[idx];
        s.name = name.to_string();
        s.last_us = elapsed_us;
        s.avg_us = s.avg_us * (1.0 - Self::ALPHA) + elapsed_us as f32 * Self::ALPHA;
    }
}

// ─── Scene Transition Fade Effect ───────────────────────────────────────────

/// Scene transition fade effect resource.
///
/// Setting a `FadeState` causes `App` to automatically animate a full-screen color overlay.
///
/// **Platform note:** the fade render pass is native-only. On `wasm32` targets the transition
/// state is still tracked (so `finished` will fire), but the visual overlay is silently skipped.
///
/// # Example
/// ```rust,ignore
/// // fade out to black (0.5 s)
/// world.insert_resource(FadeTransition::fade_out(0.5));
///
/// // fade in from the current color to transparent (0.3 s)
/// world.insert_resource(FadeTransition::fade_in(0.3));
/// ```
#[derive(Debug, Clone)]
pub struct FadeTransition {
    /// Current alpha (0.0 = transparent, 1.0 = fully opaque).
    pub alpha: f32,
    /// Target alpha value.
    pub target_alpha: f32,
    /// Alpha change per second.
    pub speed: f32,
    /// Overlay RGB color.
    pub color: Color,
    /// Whether the fade has finished (updated by `App` each frame).
    pub finished: bool,
}

impl FadeTransition {
    /// Fade out from transparent to opaque (screen darkens).
    pub fn fade_out(duration: f32) -> Self {
        Self {
            alpha: 0.0,
            target_alpha: 1.0,
            speed: 1.0 / duration.max(0.001),
            color: Color::BLACK,
            finished: false,
        }
    }

    /// Fade in from opaque to transparent (screen brightens).
    pub fn fade_in(duration: f32) -> Self {
        Self {
            alpha: 1.0,
            target_alpha: 0.0,
            speed: 1.0 / duration.max(0.001),
            color: Color::BLACK,
            finished: false,
        }
    }

    /// Fade with a custom color.
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = Color::rgb(r, g, b);
        self
    }

    /// Advances the alpha by `dt` seconds. Called automatically by `App`.
    pub fn update(&mut self, dt: f32) {
        if self.finished {
            return;
        }
        let diff = self.target_alpha - self.alpha;
        let step = self.speed * dt;
        if diff.abs() <= step {
            self.alpha = self.target_alpha;
            self.finished = true;
        } else {
            self.alpha += diff.signum() * step;
        }
    }
}

impl Default for FadeTransition {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            target_alpha: 0.0,
            speed: 1.0,
            color: Color::BLACK,
            finished: true,
        }
    }
}

#[cfg(test)]
mod fade_tests {
    use super::*;

    #[test]
    fn fade_out_starts_at_zero() {
        let f = FadeTransition::fade_out(1.0);
        assert_eq!(f.alpha, 0.0);
        assert_eq!(f.target_alpha, 1.0);
        assert!(!f.finished);
    }

    #[test]
    fn fade_update_reaches_target() {
        let mut f = FadeTransition::fade_out(0.5); // speed = 2.0/sec
        f.update(0.6); // > duration → should finish
        assert_eq!(f.alpha, 1.0);
        assert!(f.finished);
    }

    #[test]
    fn fade_update_partial() {
        let mut f = FadeTransition::fade_out(1.0); // speed = 1.0/sec
        f.update(0.3);
        assert!((f.alpha - 0.3).abs() < 1e-5);
        assert!(!f.finished);
    }

    #[test]
    fn fade_finished_does_not_update() {
        let mut f = FadeTransition::default(); // finished = true
        f.update(1.0);
        assert_eq!(f.alpha, 0.0); // no change
    }
}

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
