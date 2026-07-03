use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Interaction state of a button.
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum ButtonState {
    #[default]
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

/// A clickable button component.
///
/// Attach alongside `UiNode` on an entity.
/// `UiSystem` updates `state` each frame after hit-testing and renders
/// the background rectangle and label text.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Button {
    pub label: String,
    /// Runtime hit-test state — not serialized.
    #[serde(skip)]
    pub state: ButtonState,
    pub color_normal: Color,
    pub color_hovered: Color,
    pub color_pressed: Color,
    pub color_disabled: Color,
    pub text_color: Color,
    pub font_size: f32,
    /// Corner radius in pixels for the background rect (`0.0` = sharp, the historical look).
    pub corner_radius: f32,
}

impl Default for Button {
    fn default() -> Self {
        Self::new("")
    }
}

impl Reflect for Button {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("label", ReflectValue::String(self.label.clone())),
            ("font_size", ReflectValue::F32(self.font_size)),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
            (
                "color_normal",
                ReflectValue::Color(self.color_normal.to_array()),
            ),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("label", ReflectValue::String(v)) => {
                self.label = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            ("color_normal", ReflectValue::Color(c)) => {
                self.color_normal = Color::from(c);
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Button"
    }
}

impl Button {
    /// Creates a button with the default color preset.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            state: ButtonState::Normal,
            color_normal: Color::rgba(0.20, 0.20, 0.25, 1.0),
            color_hovered: Color::rgba(0.30, 0.30, 0.40, 1.0),
            color_pressed: Color::rgba(0.12, 0.12, 0.18, 1.0),
            color_disabled: Color::rgba(0.15, 0.15, 0.15, 0.6),
            text_color: Color::rgba_u8(220, 220, 220, 255),
            font_size: 18.0,
            corner_radius: 0.0,
        }
    }

    /// Set the normal / hovered / pressed background colors. Builder form.
    pub fn with_colors(mut self, normal: Color, hovered: Color, pressed: Color) -> Self {
        self.color_normal = normal;
        self.color_hovered = hovered;
        self.color_pressed = pressed;
        self
    }

    /// Set the disabled-state background color. Builder form.
    pub fn with_disabled_color(mut self, color: Color) -> Self {
        self.color_disabled = color;
        self
    }

    /// Set the label color. Builder form.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the label font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Round the background-rect corners to `radius` pixels (SDF, like
    /// [`DrawRect::with_corner_radius`](crate::renderer::DrawRect::with_corner_radius)). Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Returns the background color corresponding to the current state.
    pub fn current_color(&self) -> Color {
        match self.state {
            ButtonState::Normal => self.color_normal,
            ButtonState::Hovered => self.color_hovered,
            ButtonState::Pressed => self.color_pressed,
            ButtonState::Disabled => self.color_disabled,
        }
    }

    pub fn is_interactive(&self) -> bool {
        self.state != ButtonState::Disabled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn button_serde_roundtrip() {
        let b = Button::new("Click me");
        let ron = ron::to_string(&b).expect("serialize");
        let back: Button = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.label, "Click me");
        assert_eq!(back.font_size, 18.0);
        // Runtime state is not serialized; it defaults to Normal on deserialization.
        assert_eq!(back.state, ButtonState::Normal);
    }

    #[test]
    fn button_reflect_roundtrip() {
        let mut b = Button::new("Hi");
        assert!(b.set_field("label", ReflectValue::String("Bye".into())));
        assert_eq!(b.label, "Bye");
        assert!(b.set_field("font_size", ReflectValue::F32(24.0)));
        assert!((b.font_size - 24.0).abs() < f32::EPSILON);
        assert!(b.set_field("corner_radius", ReflectValue::F32(6.0)));
        assert!((b.corner_radius - 6.0).abs() < f32::EPSILON);
        let fields = b.fields();
        assert!(fields.iter().any(|(n, _)| *n == "text_color"));
        assert!(fields.iter().any(|(n, _)| *n == "corner_radius"));
    }

    #[test]
    fn builder_chain_sets_every_field() {
        // The EW-005 acceptance shape: builders chain off `new` like the newer widgets.
        let b = Button::new("Play")
            .with_colors(Color::RED, Color::GREEN, Color::BLUE)
            .with_disabled_color(Color::BLACK)
            .with_text_color(Color::WHITE)
            .with_font_size(22.0)
            .with_corner_radius(8.0);
        assert_eq!(b.color_normal, Color::RED);
        assert_eq!(b.color_hovered, Color::GREEN);
        assert_eq!(b.color_pressed, Color::BLUE);
        assert_eq!(b.color_disabled, Color::BLACK);
        assert_eq!(b.text_color, Color::WHITE);
        assert_eq!(b.font_size, 22.0);
        assert_eq!(b.corner_radius, 8.0);
    }

    #[test]
    fn old_ron_without_corner_radius_still_loads() {
        // A scene saved before 0.115 has no corner_radius — `#[serde(default)]` fills it as sharp.
        let b = Button::new("Old");
        let ron = ron::to_string(&b).expect("serialize");
        let stripped = ron.replace(",corner_radius:0.0", "");
        assert_ne!(ron, stripped, "fixture must actually drop the field");
        let back: Button = ron::from_str(&stripped).expect("deserialize old RON");
        assert_eq!(back.label, "Old");
        assert_eq!(back.corner_radius, 0.0);
    }
}
