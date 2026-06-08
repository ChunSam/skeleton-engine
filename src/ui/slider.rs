use crate::color::Color;

/// Horizontal slider component.
///
/// Attach to an entity alongside `UiNode`.
/// `UiSystem` processes drag input and emits `UiEvent::SliderChanged`.
///
/// # Example
/// ```ignore
/// let entity = world.spawn();
/// world.insert(entity, UiNode::new(100.0, 300.0, 200.0, 20.0));
/// world.insert(entity, Slider::new(0.0, 100.0, 50.0));
/// ```
pub struct Slider {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    /// Whether a drag is in progress (internal state; no need to modify directly).
    pub(crate) dragging: bool,
    pub track_color: Color,
    pub fill_color: Color,
    pub thumb_color: Color,
    pub thumb_hovered_color: Color,
    /// Thumb width in pixels. Height matches `UiNode.size.y`.
    pub thumb_width: f32,
}

impl Slider {
    pub fn new(min: f32, max: f32, value: f32) -> Self {
        Self {
            value: value.clamp(min, max),
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
