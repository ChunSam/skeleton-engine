use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// A scrollable text list widget.
///
/// Attach to an entity alongside `UiNode`.
/// Renders the `items` Vec directly — no child entities required.
/// Scrollable via the mouse wheel when the cursor is over the widget.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollView {
    pub items: Vec<String>,
    /// Vertical scroll offset (pixels, 0 = top) — not serialized (runtime state).
    #[serde(skip)]
    pub scroll_offset: f32,
    pub item_height: f32,
    pub font_size: f32,
    pub color: Color,
    pub background_color: Color,
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl Reflect for ScrollView {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("item_height", ReflectValue::F32(self.item_height)),
            ("font_size", ReflectValue::F32(self.font_size)),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("item_height", ReflectValue::F32(v)) => {
                self.item_height = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "ScrollView"
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_view_serde_roundtrip() {
        let sv = ScrollView::new()
            .with_items(vec!["a".into(), "b".into()])
            .with_item_height(30.0);
        let ron = ron::to_string(&sv).expect("serialize");
        let back: ScrollView = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.items, vec!["a", "b"]);
        assert!((back.item_height - 30.0).abs() < f32::EPSILON);
        // scroll_offset is runtime-only; defaults to 0.0 after deserialization.
        assert!((back.scroll_offset - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn scroll_view_reflect_roundtrip() {
        let mut sv = ScrollView::new();
        assert!(sv.set_field("item_height", ReflectValue::F32(32.0)));
        assert!((sv.item_height - 32.0).abs() < f32::EPSILON);
        let fields = sv.fields();
        assert!(fields.iter().any(|(n, _)| *n == "font_size"));
    }
}
