use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// A radio group — a vertical list of mutually exclusive options, exactly one selected.
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode); **one entity is the whole
/// group** (like [`Dropdown`](crate::ui::Dropdown) / [`ScrollView`](crate::ui::ScrollView), not one
/// entity per option), so the options can never disagree about which one is selected. The node's
/// rect is divided into evenly-spaced rows (or [`item_height`](Self::item_height)-tall rows), each
/// drawn as a ring + label with the selected row's ring filled by a dot. Clicking a row selects it
/// and emits [`UiEvent::RadioChanged`](crate::ui::UiEvent::RadioChanged) when the selection
/// actually changed (re-picking the current option is silent).
///
/// Keyboard/gamepad: the widget is focusable (Tab / D-pad / stick); **←/→** (or D-pad Left/Right)
/// steps the selection directly, clamped like a [`Slider`](crate::ui::Slider) nudge — a settings
/// row is fully operable without a pointer. Enter/Space does nothing (there is no "activate"
/// beyond selecting).
///
/// # Example
/// ```
/// # use engine::ui::RadioGroup;
/// let rg = RadioGroup::new(["Easy", "Normal", "Hard"]).with_selected(1);
/// assert_eq!(rg.selected_item(), Some("Normal"));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RadioGroup {
    /// The selectable options, rendered top-to-bottom.
    pub items: Vec<String>,
    /// Index of the selected option. Read via [`selected_index`](Self::selected_index), which
    /// clamps into range; the raw field is left as set so an out-of-range value is not silently
    /// mutated.
    pub selected: usize,
    /// Ring (outline) color of every option circle.
    pub circle_color: Color,
    /// Inner-dot color of the selected option.
    pub fill_color: Color,
    /// Background tint of the row under the cursor (use alpha for a subtle highlight).
    pub hover_color: Color,
    /// Option label color.
    pub text_color: Color,
    /// Label font size in pixels.
    pub font_size: f32,
    /// Diameter of the option circle in pixels.
    pub circle_size: f32,
    /// Height of one option row in pixels; `0.0` (default) divides the node height evenly across
    /// the options.
    pub item_height: f32,
}

impl Default for RadioGroup {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            circle_color: Color::rgba(0.50, 0.52, 0.62, 1.0),
            fill_color: Color::rgba(0.28, 0.56, 0.90, 1.0),
            hover_color: Color::rgba(1.0, 1.0, 1.0, 0.06),
            text_color: Color::rgba_u8(210, 210, 220, 255),
            font_size: 16.0,
            circle_size: 18.0,
            item_height: 0.0,
        }
    }
}

impl RadioGroup {
    /// A radio group over `items` with the first option selected and the default style.
    pub fn new<I, S>(items: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Select option `index` (clamped into range on read). Builder form.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Set the ring and selected-dot colors. Builder form.
    pub fn with_colors(mut self, circle_color: Color, fill_color: Color) -> Self {
        self.circle_color = circle_color;
        self.fill_color = fill_color;
        self
    }

    /// Set the label color. Builder form.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self
    }

    /// Set the label font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the option-circle diameter in pixels. Builder form.
    pub fn with_circle_size(mut self, size: f32) -> Self {
        self.circle_size = size;
        self
    }

    /// Set an explicit row height in pixels (`0.0` = divide the node height evenly). Builder form.
    pub fn with_item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }

    /// The selected index clamped into `items` range — what the renderer and
    /// [`selected_item`](Self::selected_item) use. `0` when `items` is empty.
    pub fn selected_index(&self) -> usize {
        self.selected.min(self.items.len().saturating_sub(1))
    }

    /// The selected option's text, or `None` when `items` is empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected_index()).map(String::as_str)
    }

    /// Resolved row height: [`item_height`](Self::item_height) if positive, else `node_height`
    /// divided evenly across the options (`0.0` when there are none).
    pub fn resolved_item_height(&self, node_height: f32) -> f32 {
        if self.item_height > 0.0 {
            self.item_height
        } else if self.items.is_empty() {
            0.0
        } else {
            node_height / self.items.len() as f32
        }
    }

    /// The option row under `cursor` for a group drawn at `(pos, node_size)`, or `None` when the
    /// cursor is outside every row (or there are no options). The same geometry drives rendering
    /// and click resolution, so they can never disagree. Rows are clickable only where they lie
    /// inside the node rect (the pointer-capture surface): rows overflowing a too-small node
    /// still render but don't select, and dead space below short fixed-height rows selects
    /// nothing.
    pub fn row_at(&self, cursor: Vec2, pos: Vec2, node_size: Vec2) -> Option<usize> {
        let item_h = self.resolved_item_height(node_size.y);
        if self.items.is_empty() || item_h <= 0.0 {
            return None;
        }
        let rows_h = (item_h * self.items.len() as f32).min(node_size.y);
        let inside = cursor.x >= pos.x
            && cursor.x < pos.x + node_size.x
            && cursor.y >= pos.y
            && cursor.y < pos.y + rows_h;
        if !inside {
            return None;
        }
        Some((((cursor.y - pos.y) / item_h) as usize).min(self.items.len() - 1))
    }
}

impl Reflect for RadioGroup {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("selected", ReflectValue::I32(self.selected as i32)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("circle_size", ReflectValue::F32(self.circle_size)),
            ("item_height", ReflectValue::F32(self.item_height)),
            (
                "circle_color",
                ReflectValue::Color(self.circle_color.to_array()),
            ),
            (
                "fill_color",
                ReflectValue::Color(self.fill_color.to_array()),
            ),
            (
                "hover_color",
                ReflectValue::Color(self.hover_color.to_array()),
            ),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("selected", ReflectValue::I32(v)) => {
                self.selected = v.max(0) as usize;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("circle_size", ReflectValue::F32(v)) => {
                self.circle_size = v;
                true
            }
            ("item_height", ReflectValue::F32(v)) => {
                self.item_height = v;
                true
            }
            ("circle_color", ReflectValue::Color(c)) => {
                self.circle_color = Color::from(c);
                true
            }
            ("fill_color", ReflectValue::Color(c)) => {
                self.fill_color = Color::from(c);
                true
            }
            ("hover_color", ReflectValue::Color(c)) => {
                self.hover_color = Color::from(c);
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "RadioGroup"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_clamps_on_read_without_mutating() {
        let rg = RadioGroup::new(["a", "b", "c"]).with_selected(10);
        assert_eq!(rg.selected, 10, "raw field left as set");
        assert_eq!(rg.selected_index(), 2, "clamped to the last option");
        assert_eq!(rg.selected_item(), Some("c"));

        let empty = RadioGroup::new(Vec::<String>::new());
        assert_eq!(empty.selected_index(), 0);
        assert_eq!(empty.selected_item(), None);
    }

    #[test]
    fn item_height_divides_the_node_evenly_by_default() {
        let rg = RadioGroup::new(["a", "b", "c"]);
        assert_eq!(rg.resolved_item_height(90.0), 30.0, "0.0 = node_h / n");

        let fixed = RadioGroup::new(["a", "b", "c"]).with_item_height(24.0);
        assert_eq!(fixed.resolved_item_height(90.0), 24.0);

        let empty = RadioGroup::new(Vec::<String>::new());
        assert_eq!(empty.resolved_item_height(90.0), 0.0, "no options, no rows");
    }

    #[test]
    fn row_at_maps_cursor_to_rows() {
        // 3 rows in a 120×90 node at (10, 10) → rows at y 10..40, 40..70, 70..100.
        let rg = RadioGroup::new(["a", "b", "c"]);
        let pos = Vec2::new(10.0, 10.0);
        let size = Vec2::new(120.0, 90.0);
        assert_eq!(rg.row_at(Vec2::new(20.0, 15.0), pos, size), Some(0));
        assert_eq!(rg.row_at(Vec2::new(20.0, 55.0), pos, size), Some(1));
        assert_eq!(rg.row_at(Vec2::new(20.0, 95.0), pos, size), Some(2));
        assert_eq!(rg.row_at(Vec2::new(5.0, 55.0), pos, size), None, "left of");
        assert_eq!(
            rg.row_at(Vec2::new(20.0, 105.0), pos, size),
            None,
            "below the rows"
        );

        let empty = RadioGroup::new(Vec::<String>::new());
        assert_eq!(empty.row_at(Vec2::new(20.0, 15.0), pos, size), None);
    }

    #[test]
    fn row_clicks_clamp_to_the_node_rect() {
        let pos = Vec2::new(0.0, 0.0);
        let size = Vec2::new(120.0, 90.0);

        // Short fixed rows (3 × 20 = 60 tall in a 90-tall node): the dead space below the last
        // row selects nothing.
        let short = RadioGroup::new(["a", "b", "c"]).with_item_height(20.0);
        assert_eq!(short.row_at(Vec2::new(20.0, 55.0), pos, size), Some(2));
        assert_eq!(short.row_at(Vec2::new(20.0, 75.0), pos, size), None);

        // Overflowing rows (3 × 40 = 120 tall in the same node): only the part inside the node
        // rect — the capture surface — is clickable.
        let tall = RadioGroup::new(["a", "b", "c"]).with_item_height(40.0);
        assert_eq!(tall.row_at(Vec2::new(20.0, 85.0), pos, size), Some(2));
        assert_eq!(tall.row_at(Vec2::new(20.0, 110.0), pos, size), None);
    }

    #[test]
    fn builders_set_fields() {
        let rg = RadioGroup::new(["x"])
            .with_selected(0)
            .with_colors(Color::WHITE, Color::RED)
            .with_text_color(Color::BLUE)
            .with_font_size(20.0)
            .with_circle_size(24.0)
            .with_item_height(28.0);
        assert_eq!(rg.items, vec!["x"]);
        assert_eq!(rg.circle_color, Color::WHITE);
        assert_eq!(rg.fill_color, Color::RED);
        assert_eq!(rg.text_color, Color::BLUE);
        assert_eq!(rg.font_size, 20.0);
        assert_eq!(rg.circle_size, 24.0);
        assert_eq!(rg.item_height, 28.0);
    }

    #[test]
    fn serde_roundtrip_keeps_items_and_selection() {
        let rg = RadioGroup::new(["Easy", "Hard"]).with_selected(1);
        let ron = ron::to_string(&rg).expect("serialize");
        let back: RadioGroup = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.items, vec!["Easy", "Hard"]);
        assert_eq!(back.selected, 1);
    }

    #[test]
    fn reflect_roundtrip() {
        let mut rg = RadioGroup::new(["a", "b"]);
        assert!(rg.set_field("selected", ReflectValue::I32(1)));
        assert_eq!(rg.selected, 1);
        assert!(rg.set_field("selected", ReflectValue::I32(-3)));
        assert_eq!(rg.selected, 0, "negative reflect writes clamp to 0");
        assert!(rg.set_field("circle_size", ReflectValue::F32(22.0)));
        assert_eq!(rg.circle_size, 22.0);
        let fields = rg.fields();
        assert!(fields.iter().any(|(n, _)| *n == "selected"));
        assert!(fields.iter().any(|(n, _)| *n == "fill_color"));
    }
}
