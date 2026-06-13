use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};
use crate::renderer::TextAlign;

/// Text label component.
///
/// Use alongside `UiNode`. `UiSystem` submits a `DrawText` to `TextQueue` every frame
/// to render it.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Label {
    pub text: String,
    /// RGBA (0~255)
    pub color: Color,
    pub font_size: f32,
    pub align: TextAlign,
    pub rich: bool,
}

impl Default for Label {
    fn default() -> Self {
        Self::new("")
    }
}

impl Reflect for Label {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("text", ReflectValue::String(self.text.clone())),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("color", ReflectValue::Color(self.color.to_array())),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("text", ReflectValue::String(v)) => {
                self.text = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("color", ReflectValue::Color(c)) => {
                self.color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Label"
    }
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::rgba_u8(220, 220, 220, 255),
            font_size: 16.0,
            align: TextAlign::Left,
            rich: false,
        }
    }

    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = color.into();
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    pub fn rich(mut self) -> Self {
        self.rich = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_serde_roundtrip() {
        let l = Label::new("Hello").with_font_size(20.0);
        let ron = ron::to_string(&l).expect("serialize");
        let back: Label = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.text, "Hello");
        assert!((back.font_size - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn label_reflect_roundtrip() {
        let mut l = Label::new("Test");
        assert!(l.set_field("text", ReflectValue::String("Updated".into())));
        assert_eq!(l.text, "Updated");
        assert!(l.set_field("font_size", ReflectValue::F32(32.0)));
        assert!((l.font_size - 32.0).abs() < f32::EPSILON);
        let fields = l.fields();
        assert!(fields.iter().any(|(n, _)| *n == "color"));
    }
}
