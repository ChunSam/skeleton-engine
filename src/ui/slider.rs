use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Default keyboard/gamepad nudge step for a [`Slider`], as a fraction of its `max - min`
/// range, used when [`Slider::keyboard_step`] is `None`. A focused slider moves by this
/// much per ←/→ (or D-pad / left-stick) press. `0.05` = 5% of the range = 20 presses end
/// to end — the historical hardcoded value.
pub const DEFAULT_SLIDER_STEP_FRAC: f32 = 0.05;

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
    /// Absolute value step for a keyboard/gamepad nudge (←/→, D-pad, left stick) while the
    /// slider is focused. `None` (default) falls back to [`DEFAULT_SLIDER_STEP_FRAC`] × the
    /// `max - min` range. `Some(s)` overrides it with an absolute step in value units — set
    /// it to e.g. `1.0` for a slider that selects discrete integer levels, so each press
    /// moves exactly one level instead of a fixed 5% of the range.
    #[serde(default)]
    pub keyboard_step: Option<f32>,
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
            keyboard_step: None,
        }
    }

    /// Sets the absolute keyboard/gamepad nudge step (value units per ←/→ press),
    /// overriding the default [`DEFAULT_SLIDER_STEP_FRAC`] fraction-of-range. Builder form.
    pub fn with_keyboard_step(mut self, step: f32) -> Self {
        self.keyboard_step = Some(step);
        self
    }

    /// The absolute value step a keyboard/gamepad nudge moves this slider by — the
    /// [`keyboard_step`](Self::keyboard_step) override if set, else
    /// [`DEFAULT_SLIDER_STEP_FRAC`] × the `max - min` range.
    pub fn resolved_keyboard_step(&self) -> f32 {
        self.keyboard_step
            .unwrap_or((self.max - self.min) * DEFAULT_SLIDER_STEP_FRAC)
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
    fn keyboard_step_defaults_to_frac_of_range_and_honors_override() {
        // Default: 5% of the range.
        let s = Slider::new(0.0, 100.0, 50.0);
        assert!(s.keyboard_step.is_none());
        assert!((s.resolved_keyboard_step() - 5.0).abs() < f32::EPSILON);

        // Override: an absolute step, independent of the range.
        let s = Slider::new(0.0, 3.0, 0.0).with_keyboard_step(1.0);
        assert_eq!(s.keyboard_step, Some(1.0));
        assert!((s.resolved_keyboard_step() - 1.0).abs() < f32::EPSILON);

        // The override survives a serde round-trip (it's a persisted field).
        let back: Slider = ron::from_str(&ron::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.keyboard_step, Some(1.0));
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
        assert!(
            (s.initial_value - 75.0).abs() < f32::EPSILON,
            "initial_value not updated"
        );
        assert!(
            (s.value - 75.0).abs() < f32::EPSILON,
            "value (live thumb) must match initial_value after set_field"
        );
    }

    #[test]
    fn set_field_initial_value_clamps_to_range() {
        let mut s = Slider::new(0.0, 100.0, 50.0);

        // Value above max should be clamped to max.
        assert!(s.set_field("initial_value", ReflectValue::F32(200.0)));
        assert!(
            (s.value - 100.0).abs() < f32::EPSILON,
            "value must be clamped to max"
        );

        // Value below min should be clamped to min.
        assert!(s.set_field("initial_value", ReflectValue::F32(-50.0)));
        assert!(
            (s.value - 0.0).abs() < f32::EPSILON,
            "value must be clamped to min"
        );
    }
}
