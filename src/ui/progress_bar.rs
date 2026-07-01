use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// A read-only horizontal progress bar / gauge — health, mana, loading, XP, a cast timer.
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode); [`UiSystem`](crate::ui::UiSystem)
/// draws a background rect filling the node's rect and a fill rect whose width is
/// [`fraction`](Self::fraction) × the node width. Unlike [`Slider`](crate::ui::Slider) it takes **no
/// input** — the game drives [`value`](Self::value) each frame (e.g. `hp / max_hp`); the bar just
/// reflects it. Genre-agnostic and fork-friendly.
///
/// `corner_radius` rounds the bar (both background and fill) via the UI SDF pipeline; `border` (with
/// [`border_color`](Self::border_color)) draws an outline ring on top. Both default to `0.0`
/// (sharp, borderless), matching the sprite/UI fast path.
///
/// # Example
/// ```
/// # use engine::ui::ProgressBar;
/// # use engine::Color;
/// let hp = ProgressBar::new(0.75)
///     .with_colors(Color::rgb(0.2, 0.8, 0.35), Color::rgba(0.1, 0.1, 0.12, 1.0))
///     .with_corner_radius(6.0);
/// assert_eq!(hp.fraction(), 0.75);
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProgressBar {
    /// Fill fraction. Read via [`fraction`](Self::fraction), which clamps to `0.0..=1.0`; the raw
    /// field is left as the game sets it so an out-of-range value is not silently mutated.
    pub value: f32,
    /// The track (unfilled) color, drawn across the whole node rect.
    pub bg_color: Color,
    /// The fill color, drawn from the left over `fraction × width`.
    pub fill_color: Color,
    /// Corner radius in pixels for the background + fill (`0.0` = sharp).
    pub corner_radius: f32,
    /// Outline width in pixels drawn on top of the bar (`0.0` = no border).
    pub border: f32,
    /// Outline color when [`border`](Self::border) `> 0`.
    pub border_color: Color,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            value: 0.0,
            bg_color: Color::rgba(0.16, 0.16, 0.20, 1.0),
            fill_color: Color::rgba(0.28, 0.62, 0.88, 1.0),
            corner_radius: 0.0,
            border: 0.0,
            border_color: Color::rgba(0.0, 0.0, 0.0, 0.6),
        }
    }
}

impl ProgressBar {
    /// A bar at fill fraction `value` (clamped into `0.0..=1.0`) with the default colors.
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
            ..Self::default()
        }
    }

    /// Set the fill and background colors. Builder form.
    pub fn with_colors(mut self, fill_color: Color, bg_color: Color) -> Self {
        self.fill_color = fill_color;
        self.bg_color = bg_color;
        self
    }

    /// Round the bar's corners (both background and fill) to `radius` pixels. Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Draw an outline of `width` pixels in `color` on top of the bar. Builder form.
    pub fn with_border(mut self, width: f32, color: Color) -> Self {
        self.border = width;
        self.border_color = color;
        self
    }

    /// The fill fraction clamped to `0.0..=1.0` — what the renderer uses.
    pub fn fraction(&self) -> f32 {
        self.value.clamp(0.0, 1.0)
    }
}

impl Reflect for ProgressBar {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("value", ReflectValue::F32(self.value)),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
            (
                "fill_color",
                ReflectValue::Color(self.fill_color.to_array()),
            ),
            ("bg_color", ReflectValue::Color(self.bg_color.to_array())),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("value", ReflectValue::F32(v)) => {
                self.value = v;
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
                true
            }
            ("fill_color", ReflectValue::Color(c)) => {
                self.fill_color = Color::from(c);
                true
            }
            ("bg_color", ReflectValue::Color(c)) => {
                self.bg_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "ProgressBar"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_clamps_without_mutating_the_field() {
        let over = ProgressBar {
            value: 1.5,
            ..Default::default()
        };
        assert_eq!(over.fraction(), 1.0);
        assert_eq!(over.value, 1.5, "raw field is left as set");

        let under = ProgressBar {
            value: -0.3,
            ..Default::default()
        };
        assert_eq!(under.fraction(), 0.0);
    }

    #[test]
    fn new_clamps_the_initial_value() {
        assert_eq!(ProgressBar::new(2.0).value, 1.0);
        assert_eq!(ProgressBar::new(-1.0).value, 0.0);
        assert_eq!(ProgressBar::new(0.4).value, 0.4);
    }

    #[test]
    fn builders_set_colors_radius_border() {
        let b = ProgressBar::new(0.5)
            .with_colors(Color::RED, Color::BLUE)
            .with_corner_radius(8.0)
            .with_border(2.0, Color::WHITE);
        assert_eq!(b.fill_color, Color::RED);
        assert_eq!(b.bg_color, Color::BLUE);
        assert_eq!(b.corner_radius, 8.0);
        assert_eq!(b.border, 2.0);
        assert_eq!(b.border_color, Color::WHITE);
    }

    #[test]
    fn serde_roundtrip_keeps_value_and_style() {
        let b = ProgressBar::new(0.6).with_corner_radius(5.0);
        let ron = ron::to_string(&b).expect("serialize");
        assert!(ron.contains("value"), "value missing: {ron}");
        let back: ProgressBar = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back, b);
    }

    #[test]
    fn reflect_roundtrip() {
        let mut b = ProgressBar::new(0.2);
        assert!(b.set_field("value", ReflectValue::F32(0.9)));
        assert!((b.value - 0.9).abs() < f32::EPSILON);
        assert!(b.set_field("corner_radius", ReflectValue::F32(4.0)));
        assert!((b.corner_radius - 4.0).abs() < f32::EPSILON);
        let fields = b.fields();
        assert!(fields.iter().any(|(n, _)| *n == "fill_color"));
        assert!(fields.iter().any(|(n, _)| *n == "value"));
    }
}
