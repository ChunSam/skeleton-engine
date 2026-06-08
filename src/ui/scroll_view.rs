use crate::color::Color;

/// A scrollable text list widget.
///
/// Attach to an entity alongside `UiNode`.
/// Renders the `items` Vec directly — no child entities required.
/// Scrollable via the mouse wheel when the cursor is over the widget.
pub struct ScrollView {
    pub items: Vec<String>,
    /// Vertical scroll offset (pixels, 0 = top).
    pub scroll_offset: f32,
    pub item_height: f32,
    pub font_size: f32,
    pub color: Color,
    pub background_color: Color,
}

impl ScrollView {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            scroll_offset: 0.0,
            item_height: 24.0,
            font_size: 14.0,
            color: Color::rgba_u8(200, 200, 200, 255),
            background_color: Color::rgba(0.10, 0.10, 0.15, 1.0),
        }
    }

    pub fn with_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    pub fn with_item_height(mut self, h: f32) -> Self {
        self.item_height = h;
        self
    }

    /// Clamps `scroll_offset` to the valid range.
    pub fn clamp_scroll(&mut self, view_height: f32) {
        let total = self.items.len() as f32 * self.item_height;
        let max_offset = (total - view_height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_offset);
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}
