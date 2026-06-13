/// Checkbox component.
///
/// Attach to an entity alongside `UiNode`.
/// `UiSystem` toggles `checked` on click and emits `UiEvent::CheckBoxToggled`.
///
/// Render: \[box\] label text
///
/// # Example
/// ```ignore
/// let entity = world.spawn();
/// world.insert(entity, UiNode::new(50.0, 200.0, 160.0, 24.0));
/// world.insert(entity, CheckBox::new("Enable sound"));
/// ```
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CheckBox {
    pub checked: bool,
    pub label: String,
    pub checked_color: Color,
    pub unchecked_color: Color,
    pub border_color: Color,
    pub text_color: Color,
    pub font_size: f32,
    /// Side length of the checkbox square (pixels). Must be smaller than the UiNode height.
    pub box_size: f32,
}

impl Default for CheckBox {
    fn default() -> Self {
        Self::new("")
    }
}

impl Reflect for CheckBox {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("checked", ReflectValue::Bool(self.checked)),
            ("label", ReflectValue::String(self.label.clone())),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("box_size", ReflectValue::F32(self.box_size)),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("checked", ReflectValue::Bool(v)) => {
                self.checked = v;
                true
            }
            ("label", ReflectValue::String(v)) => {
                self.label = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("box_size", ReflectValue::F32(v)) => {
                self.box_size = v;
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "CheckBox"
    }
}

impl CheckBox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            checked: false,
            label: label.into(),
            checked_color: Color::rgba(0.28, 0.56, 0.90, 1.0),
            unchecked_color: Color::rgba(0.18, 0.18, 0.22, 1.0),
            border_color: Color::rgba(0.50, 0.52, 0.62, 1.0),
            text_color: Color::rgba_u8(210, 210, 220, 255),
            font_size: 16.0,
            box_size: 20.0,
        }
    }

    /// Sets the initial checked state.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkbox_serde_roundtrip() {
        let cb = CheckBox::new("Sound").with_checked(true);
        let ron = ron::to_string(&cb).expect("serialize");
        let back: CheckBox = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.label, "Sound");
        assert!(back.checked);
    }

    #[test]
    fn checkbox_reflect_roundtrip() {
        let mut cb = CheckBox::new("Foo");
        assert!(cb.set_field("checked", ReflectValue::Bool(true)));
        assert!(cb.checked);
        assert!(cb.set_field("box_size", ReflectValue::F32(24.0)));
        assert!((cb.box_size - 24.0).abs() < f32::EPSILON);
        let fields = cb.fields();
        assert!(fields.iter().any(|(n, _)| *n == "label"));
    }
}
