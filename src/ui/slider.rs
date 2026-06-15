use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Horizontal slider component.
///
/// Attach to an entity alongside `UiNode`.
/// `UiSystem` processes drag input and emits `UiEvent::SliderChanged`.
///
/// # Scene serialization note
/// `initial_value` holds the design-time starting position. `value` is the runtime
/// mutable field updated by drag input. A post-spawn hook (or the widget constructor)
/// is expected to seed `value` from `initial_value`. The runtime `value` and `dragging`
/// fields are not serialized.
///
/// # Example
/// ```ignore
/// let entity = world.spawn();
/// world.insert(entity, UiNode::new(100.0, 300.0, 200.0, 20.0));
/// world.insert(entity, Slider::new(0.0, 100.0, 50.0));
/// ```
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Slider {
    /// Design-time initial value. Seeded into `value` on construction.
    pub initial_value: f32,
    /// Runtime current value — not serialized.
    #[serde(skip)]
    pub value: f32,
    pub min: f32,
    pub max: f32,
    /// Whether a drag is in progress — not serialized.
    #[serde(skip)]
    pub(crate) dragging: bool,
    pub track_color: Color,
    pub fill_color: Color,
    pub thumb_color: Color,
    pub thumb_hovered_color: Color,
    /// Thumb width in pixels. Height matches `UiNode.size.y`.
    pub thumb_width: f32,
}

impl Default for Slider {
    fn default() -> Self {
        Self::new(0.0, 1.0, 0.0)
    }
}

impl Reflect for Slider {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("initial_value", ReflectValue::F32(self.initial_value)),
            ("min", ReflectValue::F32(self.min)),
            ("max", ReflectValue::F32(self.max)),
            (
                "fill_color",
                ReflectValue::Color(self.fill_color.to_array()),
            ),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("initial_value", ReflectValue::F32(v)) => {
                self.initial_value = v;
                // Sync the live thumb position so the slider immediately reflects the
                // inspector edit, rather than staying at the stale runtime value.
                self.value = v.clamp(self.min, self.max);
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
            ("fill_color", ReflectValue::Color(c)) => {
                self.fill_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Slider"
    }
}

impl Slider {
    pub fn new(min: f32, max: f32, value: f32) -> Self {
        let clamped = value.clamp(min, max);
        Self {
            initial_value: clamped,
            value: clamped,
            min,
            max,
            dragging: false,
            track_color: Color::rgba(0.20, 0.20, 0.25, 1.0),
            fill_color: Color::rgba(0.28, 0.52, 0.82, 1.0),
            thumb_color: Color::rgba(0.70, 0.70, 0.82, 1.0),
            thumb_hovered_color: Color::rgba(0.90, 0.90, 1.00, 1.0),
            thumb_width: 14.0,
        }
    }

    /// Normalizes the current value to [0.0, 1.0].
    pub fn normalized(&self) -> f32 {
        let range = self.max - self.min;
        if range.abs() < f32::EPSILON {
            0.0
        } else {
            (self.value - self.min) / range
        }
    }

    /// Sets the actual value from a normalized `t` ∈ [0, 1].
    pub(crate) fn set_normalized(&mut self, t: f32) {
        self.value = self.min + t.clamp(0.0, 1.0) * (self.max - self.min);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_serde_roundtrip() {
        let s = Slider::new(0.0, 100.0, 42.0);
        let ron = ron::to_string(&s).expect("serialize");
        // Runtime `value` must not appear; `initial_value` must be there.
        assert!(
            ron.contains("initial_value"),
            "initial_value missing: {ron}"
        );
        let back: Slider = ron::from_str(&ron).expect("deserialize");
        assert!((back.initial_value - 42.0).abs() < f32::EPSILON);
        assert!((back.min - 0.0).abs() < f32::EPSILON);
        assert!((back.max - 100.0).abs() < f32::EPSILON);
        // Runtime `value` defaults to 0.0 (skipped field default).
        assert!((back.value - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn slider_reflect_roundtrip() {
        let mut s = Slider::new(0.0, 1.0, 0.5);
        assert!(s.set_field("initial_value", ReflectValue::F32(0.8)));
        assert!((s.initial_value - 0.8).abs() < f32::EPSILON);
        assert!(s.set_field("min", ReflectValue::F32(-1.0)));
        assert!((s.min - -1.0).abs() < f32::EPSILON);
        let fields = s.fields();
        assert!(fields.iter().any(|(n, _)| *n == "fill_color"));
    }

    #[test]
    fn set_field_initial_value_syncs_live_value() {
        let mut s = Slider::new(0.0, 100.0, 0.0);
        assert!((s.value - 0.0).abs() < f32::EPSILON);

        // Setting initial_value must also move the live thumb position.
        assert!(s.set_field("initial_value", ReflectValue::F32(75.0)));
        assert!((s.initial_value - 75.0).abs() < f32::EPSILON,
            "initial_value not updated");
        assert!((s.value - 75.0).abs() < f32::EPSILON,
            "value (live thumb) must match initial_value after set_field");
    }

    #[test]
    fn set_field_initial_value_clamps_to_range() {
        let mut s = Slider::new(0.0, 100.0, 50.0);

        // Value above max should be clamped to max.
        assert!(s.set_field("initial_value", ReflectValue::F32(200.0)));
        assert!((s.value - 100.0).abs() < f32::EPSILON,
            "value must be clamped to max");

        // Value below min should be clamped to min.
        assert!(s.set_field("initial_value", ReflectValue::F32(-50.0)));
        assert!((s.value - 0.0).abs() < f32::EPSILON,
            "value must be clamped to min");
    }
}
