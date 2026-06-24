//! App-lifecycle / runtime-state resources — panic recovery, async-load progress, the game-state
//! machine value, the quit signal, and the scene-transition fade.

use crate::color::Color;

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
