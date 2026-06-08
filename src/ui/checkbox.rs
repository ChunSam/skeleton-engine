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
use crate::color::Color;

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
}
