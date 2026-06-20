//! Keyboard focus for UI widgets.
//!
//! [`UiFocus`] is a `World` resource holding the currently keyboard-focused widget. [`UiSystem`]'s
//! focus pass cycles it with **Tab / Shift+Tab** across focusable widgets ([`Button`], [`TextInput`],
//! [`Slider`], [`CheckBox`]), draws a focus ring around it, and activates it on **Enter/Space**
//! (click a button, toggle a checkbox) or adjusts a focused [`Slider`] with **Left/Right**. Clicking
//! a widget also moves focus to it, so Tab resumes from there.
//!
//! It is inserted automatically (see `insert_core_resources`); read it to know what's focused.
//! The ring's appearance is configurable via the [`FocusRingStyle`] resource (also auto-inserted).
//!
//! [`UiSystem`]: crate::ui::UiSystem
//! [`Button`]: crate::ui::Button
//! [`TextInput`]: crate::ui::TextInput
//! [`Slider`]: crate::ui::Slider
//! [`CheckBox`]: crate::ui::CheckBox

use crate::color::Color;
use crate::ecs::Entity;

/// The currently keyboard-focused UI widget, or `None` when nothing is focused.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct UiFocus {
    /// The focused widget entity (`None` = no focus).
    pub entity: Option<Entity>,
}

impl UiFocus {
    /// Convenience: whether `entity` is the focused widget.
    pub fn is_focused(&self, entity: Entity) -> bool {
        self.entity == Some(entity)
    }
}

/// Appearance of the focus ring [`UiSystem`]'s focus pass draws around the focused widget.
///
/// A `World` resource (auto-inserted with the default amber 3px ring, see `insert_core_resources`).
/// Insert your own to restyle it, e.g. a thicker cyan ring:
///
/// ```
/// # use engine::{Color, FocusRingStyle};
/// let style = FocusRingStyle {
///     color: Color::rgb(0.3, 0.9, 1.0),
///     thickness: 5.0,
///     corner_radius: 8.0,   // round the ring's corners (0.0 = sharp, the default)
///     pulse_hz: 1.5,        // gently "breathe" the ring 1.5×/sec
///     pulse_min_alpha: 0.4, // fading to 40% of `color`'s alpha at the trough
///     ..Default::default()
/// };
/// assert!(style.is_visible());
/// ```
///
/// Set `enabled = false` (or `thickness <= 0.0`) to suppress the engine ring entirely — useful when
/// a game draws its own focus indicator. The default matches the historical hardcoded ring exactly
/// (`pulse_hz = 0.0` → no pulse).
///
/// [`UiSystem`]: crate::ui::UiSystem
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRingStyle {
    /// Ring border color.
    pub color: Color,
    /// Border thickness in screen pixels. `<= 0.0` draws no ring.
    pub thickness: f32,
    /// Corner radius in screen pixels. `0.0` (the default) draws the historical sharp ring as four
    /// axis-aligned bars; a positive value draws a single rounded outline quad (SDF-rendered),
    /// clamped to half the widget's smaller side.
    pub corner_radius: f32,
    /// Whether to draw the ring at all. `false` draws no ring (the focus logic still runs).
    pub enabled: bool,
    /// Pulse ("breathing") frequency in cycles/second. `0.0` (the default) disables the pulse: the
    /// ring draws at a constant alpha, byte-identical to a non-pulsing ring. A small value such as
    /// `1.0`–`2.0` gives a gentle fade that draws the eye to the focused widget.
    pub pulse_hz: f32,
    /// Ring alpha at the pulse's dimmest point, as a fraction of `color`'s own alpha (`1.0` = no
    /// dimming, `0.0` = fully transparent at the trough). Clamped to `[0, 1]`. Only has an effect
    /// when `pulse_hz > 0.0`.
    pub pulse_min_alpha: f32,
}

impl Default for FocusRingStyle {
    fn default() -> Self {
        Self {
            color: Color::rgba(1.0, 0.85, 0.3, 1.0),
            thickness: 3.0,
            corner_radius: 0.0,
            enabled: true,
            pulse_hz: 0.0,
            pulse_min_alpha: 1.0,
        }
    }
}

impl FocusRingStyle {
    /// Whether a ring should actually be drawn (enabled, with a positive thickness).
    pub fn is_visible(&self) -> bool {
        self.enabled && self.thickness > 0.0
    }

    /// Alpha multiplier (in `[pulse_min_alpha, 1.0]`) for the ring at elapsed time `t` seconds.
    ///
    /// Returns `1.0` when no pulse is configured (`pulse_hz <= 0.0` or `pulse_min_alpha >= 1.0`), so
    /// a default style is left completely unmodulated. Otherwise it oscillates smoothly — a raised
    /// sine, peaking at `1.0` and dipping to `pulse_min_alpha` — at `pulse_hz` cycles per second.
    pub fn pulse_alpha(&self, t: f32) -> f32 {
        if self.pulse_hz <= 0.0 || self.pulse_min_alpha >= 1.0 {
            return 1.0;
        }
        let min = self.pulse_min_alpha.clamp(0.0, 1.0);
        let phase = 0.5 + 0.5 * (std::f32::consts::TAU * self.pulse_hz * t).sin();
        min + (1.0 - min) * phase
    }
}

#[cfg(test)]
mod tests {
    use super::FocusRingStyle;

    /// The default (and any `pulse_min_alpha >= 1.0`) style is never modulated — `pulse_alpha`
    /// is a flat `1.0` for all `t`, so a non-pulsing ring is byte-identical to before.
    #[test]
    fn pulse_alpha_unity_when_disabled() {
        let def = FocusRingStyle::default();
        for &t in &[0.0, 0.25, 1.0, 12.34] {
            assert_eq!(def.pulse_alpha(t), 1.0);
        }
        // pulse_hz set but min_alpha == 1.0 → still no visible pulse.
        let no_dim = FocusRingStyle {
            pulse_hz: 2.0,
            pulse_min_alpha: 1.0,
            ..Default::default()
        };
        assert_eq!(no_dim.pulse_alpha(0.3), 1.0);
    }

    /// A configured pulse is a raised sine: midpoint at `t=0`, peak at a quarter period, trough at
    /// three-quarters, and always within `[pulse_min_alpha, 1.0]`.
    #[test]
    fn pulse_alpha_oscillates_in_range() {
        let s = FocusRingStyle {
            pulse_hz: 1.0,
            pulse_min_alpha: 0.2,
            ..Default::default()
        };
        // t=0     → sin 0   = 0  → phase 0.5 → 0.2 + 0.8*0.5 = 0.6
        assert!((s.pulse_alpha(0.0) - 0.6).abs() < 1e-5);
        // t=0.25  → sin π/2 = 1  → phase 1.0 → 1.0 (peak)
        assert!((s.pulse_alpha(0.25) - 1.0).abs() < 1e-5);
        // t=0.75  → sin 3π/2=-1  → phase 0.0 → 0.2 (trough = min)
        assert!((s.pulse_alpha(0.75) - 0.2).abs() < 1e-5);
        for i in 0..200 {
            let a = s.pulse_alpha(i as f32 * 0.013);
            assert!((0.2..=1.0).contains(&a), "pulse_alpha out of range: {a}");
        }
    }

    /// A negative `pulse_min_alpha` is clamped to `0.0` rather than driving alpha negative.
    #[test]
    fn pulse_alpha_clamps_negative_min() {
        let s = FocusRingStyle {
            pulse_hz: 1.0,
            pulse_min_alpha: -0.5,
            ..Default::default()
        };
        // trough clamps to 0.0, not -0.5
        assert!((s.pulse_alpha(0.75) - 0.0).abs() < 1e-5);
        assert!(s.pulse_alpha(0.5) >= 0.0);
    }
}
