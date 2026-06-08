use crate::color::Color;

/// Interaction state of a button.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
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
pub struct Button {
    pub label: String,
    pub state: ButtonState,
    pub color_normal: Color,
    pub color_hovered: Color,
    pub color_pressed: Color,
    pub color_disabled: Color,
    pub text_color: Color,
    pub font_size: f32,
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
