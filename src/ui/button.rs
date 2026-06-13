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
        }
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
        let fields = b.fields();
        assert!(fields.iter().any(|(n, _)| *n == "text_color"));
    }
}
