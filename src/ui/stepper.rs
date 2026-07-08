use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Which button of a [`Stepper`] a point falls on. Returned by [`Stepper::zone_at`], the single
/// geometry source shared by the pass's render and click resolution so they can never disagree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepButton {
    /// The left decrement (`-`) button.
    Dec,
    /// The right increment (`+`) button.
    Inc,
}

/// A numeric spinner — a single `value` in `min..=max` adjusted by a decrement (`-`) and an
/// increment (`+`) button flanking a centered value label.
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode); **one entity is the whole
/// widget** (like a [`Slider`](crate::ui::Slider), which it resembles — a bounded `f32` value — but
/// stepped discretely by `step` rather than dragged). Clicking `-`/`+` moves the value by `step`,
/// clamped to `min..=max`, and emits
/// [`UiEvent::StepperChanged`](crate::ui::UiEvent::StepperChanged) with the new value **only when it
/// actually changed** (clicking `+` at `max` is silent).
///
/// Keyboard/gamepad: the stepper is one focus stop (Tab / D-pad / stick). **←/→** (keyboard, D-pad,
/// or left stick) step the value down/up, clamped like a click — so it is fully operable without a
/// pointer.
///
/// # Example
/// ```
/// # use engine::ui::Stepper;
/// let s = Stepper::new(0.0, 10.0, 3.0).with_step(2.0);
/// assert_eq!(s.value, 3.0);
/// assert_eq!(s.stepped(true), 5.0); // + button
/// assert_eq!(s.label(), "3"); // 0 decimals by default
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stepper {
    /// The current value. Read via [`clamped_value`](Self::clamped_value), which clamps into
    /// `min..=max`; the raw field is left as set so an out-of-range value is not silently mutated.
    pub value: f32,
    /// Lower bound (inclusive).
    pub min: f32,
    /// Upper bound (inclusive).
    pub max: f32,
    /// Amount added/subtracted per `+`/`-` press (or ←/→ nudge).
    pub step: f32,
    /// Decimal places shown in the value label (`0` = integer display).
    pub decimals: u32,
    /// Value-label / button-glyph font size in pixels.
    pub font_size: f32,
    /// Background fill of the whole widget (behind the centered value label).
    pub bg_color: Color,
    /// Fill of the `-`/`+` buttons.
    pub button_color: Color,
    /// Fill of the button under the cursor.
    pub button_hover_color: Color,
    /// Value label and button glyph color.
    pub text_color: Color,
    /// Corner radius of the background/border in pixels (`0.0` = sharp).
    pub corner_radius: f32,
    /// Border (outline) thickness in pixels (`0.0` = no border).
    pub border: f32,
    /// Border color (used when [`border`](Self::border) `> 0.0`).
    pub border_color: Color,
}

impl Default for Stepper {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            decimals: 0,
            font_size: 16.0,
            bg_color: Color::rgba(0.10, 0.11, 0.15, 1.0),
            button_color: Color::rgba(0.20, 0.22, 0.28, 1.0),
            button_hover_color: Color::rgba(0.30, 0.34, 0.44, 1.0),
            text_color: Color::rgba_u8(212, 214, 222, 255),
            corner_radius: 6.0,
            border: 1.0,
            border_color: Color::rgba(0.34, 0.36, 0.44, 1.0),
        }
    }
}

impl Stepper {
    /// A stepper over `min..=max` starting at `value` (clamped into range) with `step` 1 and the
    /// default style.
    pub fn new(min: f32, max: f32, value: f32) -> Self {
        Self {
            min,
            max,
            value: value.max(min).min(max),
            ..Self::default()
        }
    }

    /// Set the step amount. Builder form.
    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    /// Set the number of decimal places shown in the value label. Builder form.
    pub fn with_decimals(mut self, decimals: u32) -> Self {
        self.decimals = decimals;
        self
    }

    /// Set the value-label / button-glyph font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the background, button, button-hover, and text colors. Builder form.
    pub fn with_colors(
        mut self,
        bg: Color,
        button: Color,
        button_hover: Color,
        text: Color,
    ) -> Self {
        self.bg_color = bg;
        self.button_color = button;
        self.button_hover_color = button_hover;
        self.text_color = text;
        self
    }

    /// Set the corner radius in pixels. Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set the border thickness and color. Builder form (`thickness` `0.0` = no border).
    pub fn with_border(mut self, thickness: f32, color: Color) -> Self {
        self.border = thickness;
        self.border_color = color;
        self
    }

    /// The current value clamped into `min..=max` — what rendering and stepping use. Uses
    /// `max/min` rather than [`f32::clamp`] so a reflect-edited `min > max` can't panic.
    pub fn clamped_value(&self) -> f32 {
        self.value.max(self.min).min(self.max)
    }

    /// The value one step away from the current (clamped) value: up when `forward`, down otherwise,
    /// clamped to `min..=max`. Stepping past a bound returns the bound (so it equals the current
    /// value there — the pass uses that to stay silent).
    pub fn stepped(&self, forward: bool) -> f32 {
        let delta = if forward { self.step } else { -self.step };
        (self.clamped_value() + delta).max(self.min).min(self.max)
    }

    /// `true` when the value is at (or past) the lower bound — the `-` button is a no-op.
    pub fn at_min(&self) -> bool {
        self.clamped_value() <= self.min
    }

    /// `true` when the value is at (or past) the upper bound — the `+` button is a no-op.
    pub fn at_max(&self) -> bool {
        self.clamped_value() >= self.max
    }

    /// The value label formatted with [`decimals`](Self::decimals) decimal places.
    pub fn label(&self) -> String {
        format!("{:.*}", self.decimals as usize, self.clamped_value())
    }

    /// Width in pixels of each `-`/`+` button for a widget of `node_size`: a square (button width =
    /// node height), capped at a third of the node width so the two buttons and the value label
    /// always fit. `0` for a degenerate node.
    pub fn button_width(&self, node_size: Vec2) -> f32 {
        node_size.y.min(node_size.x / 3.0).max(0.0)
    }

    /// The button under `cursor` for a widget drawn at `(pos, node_size)`, or `None` when the cursor
    /// is over the central value area or outside the node. The same geometry drives rendering and
    /// click resolution, so they can never disagree.
    pub fn zone_at(&self, cursor: Vec2, pos: Vec2, node_size: Vec2) -> Option<StepButton> {
        let bw = self.button_width(node_size);
        if bw <= 0.0 {
            return None;
        }
        if cursor.y < pos.y || cursor.y >= pos.y + node_size.y {
            return None;
        }
        if cursor.x >= pos.x && cursor.x < pos.x + bw {
            Some(StepButton::Dec)
        } else if cursor.x >= pos.x + node_size.x - bw && cursor.x < pos.x + node_size.x {
            Some(StepButton::Inc)
        } else {
            None
        }
    }
}

impl Reflect for Stepper {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("value", ReflectValue::F32(self.value)),
            ("min", ReflectValue::F32(self.min)),
            ("max", ReflectValue::F32(self.max)),
            ("step", ReflectValue::F32(self.step)),
            ("decimals", ReflectValue::I32(self.decimals as i32)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
            ("border", ReflectValue::F32(self.border)),
            ("bg_color", ReflectValue::Color(self.bg_color.to_array())),
            (
                "button_color",
                ReflectValue::Color(self.button_color.to_array()),
            ),
            (
                "button_hover_color",
                ReflectValue::Color(self.button_hover_color.to_array()),
            ),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
            (
                "border_color",
                ReflectValue::Color(self.border_color.to_array()),
            ),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("value", ReflectValue::F32(v)) => {
                self.value = v;
                true
            }
            ("min", ReflectValue::F32(v)) => {
                self.min = v;
                true
            }
            ("max", ReflectValue::F32(v)) => {
                self.max = v;
                true
            }
            ("step", ReflectValue::F32(v)) => {
                self.step = v;
                true
            }
            ("decimals", ReflectValue::I32(v)) => {
                self.decimals = v.max(0) as u32;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
                true
            }
            ("border", ReflectValue::F32(v)) => {
                self.border = v;
                true
            }
            ("bg_color", ReflectValue::Color(c)) => {
                self.bg_color = Color::from(c);
                true
            }
            ("button_color", ReflectValue::Color(c)) => {
                self.button_color = Color::from(c);
                true
            }
            ("button_hover_color", ReflectValue::Color(c)) => {
                self.button_hover_color = Color::from(c);
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            ("border_color", ReflectValue::Color(c)) => {
                self.border_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Stepper"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_clamps_the_initial_value_into_range() {
        assert_eq!(Stepper::new(0.0, 10.0, 20.0).value, 10.0);
        assert_eq!(Stepper::new(0.0, 10.0, -5.0).value, 0.0);
        assert_eq!(Stepper::new(0.0, 10.0, 4.0).value, 4.0);
    }

    #[test]
    fn stepped_moves_by_step_and_clamps_at_bounds() {
        let s = Stepper::new(0.0, 10.0, 4.0).with_step(3.0);
        assert_eq!(s.stepped(true), 7.0);
        assert_eq!(s.stepped(false), 1.0);

        // At the top, + clamps to max (== current → the pass stays silent).
        let top = Stepper::new(0.0, 10.0, 9.0).with_step(3.0);
        assert_eq!(top.stepped(true), 10.0);
        assert!(top.stepped(false) < top.clamped_value());

        // At the bottom, - clamps to min.
        let bot = Stepper::new(0.0, 10.0, 1.0).with_step(3.0);
        assert_eq!(bot.stepped(false), 0.0);
    }

    #[test]
    fn at_min_at_max_report_bounds() {
        let s = Stepper::new(0.0, 10.0, 0.0);
        assert!(s.at_min());
        assert!(!s.at_max());
        let s = Stepper::new(0.0, 10.0, 10.0);
        assert!(s.at_max());
        assert!(!s.at_min());
        let s = Stepper::new(0.0, 10.0, 5.0);
        assert!(!s.at_min() && !s.at_max());
    }

    #[test]
    fn clamped_value_survives_inverted_bounds_without_panicking() {
        // A reflect edit could leave min > max; clamped_value must not panic (unlike f32::clamp).
        let mut s = Stepper::new(0.0, 10.0, 5.0);
        s.min = 8.0; // now min > max
        let _ = s.clamped_value();
    }

    #[test]
    fn label_formats_with_decimals() {
        assert_eq!(Stepper::new(0.0, 10.0, 3.0).label(), "3");
        assert_eq!(
            Stepper::new(0.0, 3.0, 1.25).with_decimals(2).label(),
            "1.25"
        );
    }

    #[test]
    fn zone_at_hits_the_flanking_buttons_only() {
        // 120-wide, 40-tall node at (10, 10) → button width = min(40, 40) = 40.
        let s = Stepper::new(0.0, 10.0, 5.0);
        let pos = Vec2::new(10.0, 10.0);
        let size = Vec2::new(120.0, 40.0);
        assert_eq!(
            s.zone_at(Vec2::new(20.0, 30.0), pos, size),
            Some(StepButton::Dec)
        );
        assert_eq!(
            s.zone_at(Vec2::new(120.0, 30.0), pos, size),
            Some(StepButton::Inc)
        );
        assert_eq!(
            s.zone_at(Vec2::new(70.0, 30.0), pos, size),
            None,
            "center value area"
        );
        assert_eq!(
            s.zone_at(Vec2::new(200.0, 30.0), pos, size),
            None,
            "outside"
        );
        assert_eq!(
            s.zone_at(Vec2::new(20.0, 5.0), pos, size),
            None,
            "above the node"
        );
    }

    #[test]
    fn button_width_caps_at_a_third_of_the_node_width() {
        let s = Stepper::default();
        // Tall-narrow: capped by width/3.
        assert_eq!(s.button_width(Vec2::new(60.0, 100.0)), 20.0);
        // Wide-short: capped by height.
        assert_eq!(s.button_width(Vec2::new(300.0, 30.0)), 30.0);
    }

    #[test]
    fn serde_roundtrip_keeps_value_and_bounds() {
        let s = Stepper::new(0.0, 10.0, 4.0).with_step(2.0).with_decimals(1);
        let ron = ron::to_string(&s).expect("serialize");
        let back: Stepper = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.value, 4.0);
        assert_eq!(back.min, 0.0);
        assert_eq!(back.max, 10.0);
        assert_eq!(back.step, 2.0);
        assert_eq!(back.decimals, 1);
    }

    #[test]
    fn reflect_roundtrip() {
        let mut s = Stepper::new(0.0, 10.0, 5.0);
        assert!(s.set_field("value", ReflectValue::F32(7.0)));
        assert_eq!(s.value, 7.0);
        assert!(s.set_field("step", ReflectValue::F32(2.5)));
        assert_eq!(s.step, 2.5);
        assert!(s.set_field("decimals", ReflectValue::I32(-1)));
        assert_eq!(s.decimals, 0, "negative reflect writes clamp to 0");
        let fields = s.fields();
        assert!(fields.iter().any(|(n, _)| *n == "value"));
        assert!(fields.iter().any(|(n, _)| *n == "button_hover_color"));
    }
}
