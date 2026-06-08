use crate::color::Color;
use crate::renderer::TextAlign;

/// Text label component.
///
/// Use alongside `UiNode`. `UiSystem` submits a `DrawText` to `TextQueue` every frame
/// to render it.
pub struct Label {
    pub text: String,
    /// RGBA (0~255)
    pub color: Color,
    pub font_size: f32,
    pub align: TextAlign,
    pub rich: bool,
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
