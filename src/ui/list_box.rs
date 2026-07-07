use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// A scrollable, selectable single-column list — inventory lists, level select, file lists,
/// dialogue-choice lists.
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode); **one entity is the whole
/// list** (like [`RadioGroup`](crate::ui::RadioGroup) / [`ScrollView`](crate::ui::ScrollView), not
/// one entity per row), holding all `items` and the single `selected` index. Unlike a `RadioGroup`
/// (which sizes every option into a fixed rect) a `ListBox` renders **fixed-height rows** and
/// **scrolls** when the items overflow the node's height — the mouse wheel scrolls it, and stepping
/// the selection keeps the selected row in view.
///
/// Clicking a *visible* row selects it and emits
/// [`UiEvent::ListBoxChanged`](crate::ui::UiEvent::ListBoxChanged) when the selection actually
/// changed (re-picking the current row is silent).
///
/// Keyboard/gamepad: the list is one focus stop (Tab / D-pad / stick). **↑/↓** (keyboard) or
/// **←/→** (keyboard, D-pad, or left stick) step the selection, clamped like a
/// [`Slider`](crate::ui::Slider) nudge, and auto-scroll it into view — so a list is fully operable
/// without a pointer. (Up/Down come from the keyboard only: on a gamepad, D-pad/stick Up/Down cycle
/// *focus* between widgets, so a pad steps the selection with Left/Right instead.)
///
/// # Example
/// ```
/// # use engine::ui::ListBox;
/// let lb = ListBox::new(["Sword", "Shield", "Potion"]).with_selected(2);
/// assert_eq!(lb.selected_item(), Some("Potion"));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ListBox {
    /// The rows, rendered top-to-bottom.
    pub items: Vec<String>,
    /// Index of the selected row. Read via [`selected_index`](Self::selected_index), which clamps
    /// into range; the raw field is left as set so an out-of-range value is not silently mutated.
    pub selected: usize,
    /// Vertical scroll offset in pixels (`0` = top). Runtime state — **not** serialized (like
    /// [`ScrollView::scroll_offset`](crate::ui::ScrollView)); a loaded list starts at the top.
    #[serde(skip)]
    pub scroll_offset: f32,
    /// Height of one row in pixels.
    pub row_height: f32,
    /// Row-label font size in pixels.
    pub font_size: f32,
    /// Background fill of the whole list box.
    pub bg_color: Color,
    /// Background tint of the row under the cursor (use alpha for a subtle highlight).
    pub hover_color: Color,
    /// Background fill of the selected row.
    pub selected_color: Color,
    /// Row-label color.
    pub text_color: Color,
    /// Corner radius of the background/border in pixels (`0.0` = sharp).
    pub corner_radius: f32,
    /// Border (outline) thickness in pixels (`0.0` = no border).
    pub border: f32,
    /// Border color (used when [`border`](Self::border) `> 0.0`).
    pub border_color: Color,
}

impl Default for ListBox {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0.0,
            row_height: 28.0,
            font_size: 16.0,
            bg_color: Color::rgba(0.10, 0.11, 0.15, 1.0),
            hover_color: Color::rgba(1.0, 1.0, 1.0, 0.06),
            selected_color: Color::rgba(0.24, 0.46, 0.78, 0.85),
            text_color: Color::rgba_u8(212, 214, 222, 255),
            corner_radius: 6.0,
            border: 1.0,
            border_color: Color::rgba(0.34, 0.36, 0.44, 1.0),
        }
    }
}

impl ListBox {
    /// A list box over `items` with the first row selected and the default style.
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

    /// Select row `index` (clamped into range on read). Builder form.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Set the row height in pixels. Builder form.
    pub fn with_row_height(mut self, height: f32) -> Self {
        self.row_height = height;
        self
    }

    /// Set the row-label font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Set the background, hover, selected, and text colors. Builder form.
    pub fn with_colors(mut self, bg: Color, hover: Color, selected: Color, text: Color) -> Self {
        self.bg_color = bg;
        self.hover_color = hover;
        self.selected_color = selected;
        self.text_color = text;
        self
    }

    /// Set the corner radius in pixels. Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set the border thickness and color. Builder form (`thickness` `0.0` = no border).
    pub fn with_border(mut self, thickness: f32, color: Color) -> Self {
        self.border = thickness;
        self.border_color = color;
        self
    }

    /// The selected index clamped into `items` range — what the renderer and
    /// [`selected_item`](Self::selected_item) use. `0` when `items` is empty.
    pub fn selected_index(&self) -> usize {
        self.selected.min(self.items.len().saturating_sub(1))
    }

    /// The selected row's text, or `None` when `items` is empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected_index()).map(String::as_str)
    }

    /// Total pixel height of all rows (`items.len() * row_height`).
    pub fn content_height(&self) -> f32 {
        self.items.len() as f32 * self.row_height
    }

    /// The row under `cursor` for a list drawn at `(pos, node_size)`, accounting for the current
    /// [`scroll_offset`](Self::scroll_offset), or `None` when the cursor is outside the node rect or
    /// past the last row. The same geometry drives rendering and click resolution, so they can never
    /// disagree. Rows are hit only where they lie inside the node rect (the pointer-capture surface):
    /// a row scrolled partly out of view is clickable only over its visible sliver, and dead space
    /// below the last row selects nothing.
    pub fn row_at(&self, cursor: Vec2, pos: Vec2, node_size: Vec2) -> Option<usize> {
        if self.items.is_empty() || self.row_height <= 0.0 {
            return None;
        }
        let inside = cursor.x >= pos.x
            && cursor.x < pos.x + node_size.x
            && cursor.y >= pos.y
            && cursor.y < pos.y + node_size.y;
        if !inside {
            return None;
        }
        let idx = ((cursor.y - pos.y + self.scroll_offset) / self.row_height) as usize;
        (idx < self.items.len()).then_some(idx)
    }

    /// Clamps [`scroll_offset`](Self::scroll_offset) into `0..=(content_height - view_height)` so it
    /// can never scroll past the ends. `view_height` is the node's height.
    pub fn clamp_scroll(&mut self, view_height: f32) {
        let max_offset = (self.content_height() - view_height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_offset);
    }

    /// Scrolls the minimum amount so the selected row is fully visible in a `view_height`-tall
    /// viewport: scrolls up if the row is above the view, down if below, and leaves the offset
    /// untouched if it is already visible. Then clamps.
    pub fn scroll_to_selected(&mut self, view_height: f32) {
        if self.items.is_empty() || self.row_height <= 0.0 {
            return;
        }
        let top = self.selected_index() as f32 * self.row_height;
        let bottom = top + self.row_height;
        if top < self.scroll_offset {
            self.scroll_offset = top;
        } else if bottom > self.scroll_offset + view_height {
            self.scroll_offset = bottom - view_height;
        }
        self.clamp_scroll(view_height);
    }
}

impl Reflect for ListBox {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("selected", ReflectValue::I32(self.selected as i32)),
            ("row_height", ReflectValue::F32(self.row_height)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
            ("border", ReflectValue::F32(self.border)),
            ("bg_color", ReflectValue::Color(self.bg_color.to_array())),
            (
                "hover_color",
                ReflectValue::Color(self.hover_color.to_array()),
            ),
            (
                "selected_color",
                ReflectValue::Color(self.selected_color.to_array()),
            ),
            (
                "text_color",
                ReflectValue::Color(self.text_color.to_array()),
            ),
            (
                "border_color",
                ReflectValue::Color(self.border_color.to_array()),
            ),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("selected", ReflectValue::I32(v)) => {
                self.selected = v.max(0) as usize;
                true
            }
            ("row_height", ReflectValue::F32(v)) => {
                self.row_height = v;
                true
            }
            ("font_size", ReflectValue::F32(v)) => {
                self.font_size = v;
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
                true
            }
            ("border", ReflectValue::F32(v)) => {
                self.border = v;
                true
            }
            ("bg_color", ReflectValue::Color(c)) => {
                self.bg_color = Color::from(c);
                true
            }
            ("hover_color", ReflectValue::Color(c)) => {
                self.hover_color = Color::from(c);
                true
            }
            ("selected_color", ReflectValue::Color(c)) => {
                self.selected_color = Color::from(c);
                true
            }
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            ("border_color", ReflectValue::Color(c)) => {
                self.border_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "ListBox"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_clamps_on_read_without_mutating() {
        let lb = ListBox::new(["a", "b", "c"]).with_selected(10);
        assert_eq!(lb.selected, 10, "raw field left as set");
        assert_eq!(lb.selected_index(), 2, "clamped to the last row");
        assert_eq!(lb.selected_item(), Some("c"));

        let empty = ListBox::new(Vec::<String>::new());
        assert_eq!(empty.selected_index(), 0);
        assert_eq!(empty.selected_item(), None);
    }

    #[test]
    fn row_at_maps_cursor_through_scroll_offset() {
        // 28-px rows in a 200×100 node at (10, 10); no scroll → row 0 at y 10..38, row 1 38..66.
        let mut lb = ListBox::new(["a", "b", "c", "d", "e"]);
        let pos = Vec2::new(10.0, 10.0);
        let size = Vec2::new(200.0, 100.0);
        assert_eq!(lb.row_at(Vec2::new(20.0, 20.0), pos, size), Some(0));
        assert_eq!(lb.row_at(Vec2::new(20.0, 50.0), pos, size), Some(1));
        assert_eq!(lb.row_at(Vec2::new(5.0, 50.0), pos, size), None, "left of");
        // Scroll down by one row: the cursor near the top now maps one row later.
        lb.scroll_offset = 28.0;
        assert_eq!(lb.row_at(Vec2::new(20.0, 20.0), pos, size), Some(1));
        assert_eq!(lb.row_at(Vec2::new(20.0, 50.0), pos, size), Some(2));
    }

    #[test]
    fn row_at_rejects_dead_space_below_last_row() {
        // 2 rows (28 px each = 56 tall) in a 100-tall node: below the last row selects nothing.
        let lb = ListBox::new(["a", "b"]);
        let pos = Vec2::new(0.0, 0.0);
        let size = Vec2::new(200.0, 100.0);
        assert_eq!(lb.row_at(Vec2::new(20.0, 40.0), pos, size), Some(1));
        assert_eq!(lb.row_at(Vec2::new(20.0, 80.0), pos, size), None);

        let empty = ListBox::new(Vec::<String>::new());
        assert_eq!(empty.row_at(Vec2::new(20.0, 10.0), pos, size), None);
    }

    #[test]
    fn clamp_scroll_bounds_the_offset() {
        // 10 rows × 28 = 280 content in a 100-tall view → max offset 180.
        let mut lb = ListBox::new((0..10).map(|i| i.to_string()));
        lb.scroll_offset = 1000.0;
        lb.clamp_scroll(100.0);
        assert!((lb.scroll_offset - 180.0).abs() < f32::EPSILON);
        lb.scroll_offset = -50.0;
        lb.clamp_scroll(100.0);
        assert_eq!(lb.scroll_offset, 0.0);

        // Content shorter than the view → pinned to the top.
        let mut short = ListBox::new(["a", "b"]);
        short.scroll_offset = 40.0;
        short.clamp_scroll(200.0);
        assert_eq!(short.scroll_offset, 0.0);
    }

    #[test]
    fn scroll_to_selected_keeps_the_row_in_view() {
        // 10 rows × 28 = 280 content; view 84 tall shows 3 whole rows.
        let mut lb = ListBox::new((0..10).map(|i| i.to_string()));
        // Selecting the last row scrolls down so it sits at the view bottom (max offset 196).
        lb.selected = 9;
        lb.scroll_to_selected(84.0);
        assert!((lb.scroll_offset - 196.0).abs() < f32::EPSILON);
        // A row already visible does not move the offset.
        lb.selected = 8;
        let before = lb.scroll_offset;
        lb.scroll_to_selected(84.0);
        assert_eq!(lb.scroll_offset, before, "already-visible row keeps offset");
        // Selecting the top row scrolls back up to it.
        lb.selected = 0;
        lb.scroll_to_selected(84.0);
        assert_eq!(lb.scroll_offset, 0.0);
    }

    #[test]
    fn builders_set_fields() {
        let lb = ListBox::new(["x"])
            .with_selected(0)
            .with_row_height(32.0)
            .with_font_size(18.0)
            .with_colors(Color::BLACK, Color::WHITE, Color::RED, Color::BLUE)
            .with_corner_radius(10.0)
            .with_border(2.0, Color::GREEN);
        assert_eq!(lb.items, vec!["x"]);
        assert_eq!(lb.row_height, 32.0);
        assert_eq!(lb.font_size, 18.0);
        assert_eq!(lb.bg_color, Color::BLACK);
        assert_eq!(lb.selected_color, Color::RED);
        assert_eq!(lb.text_color, Color::BLUE);
        assert_eq!(lb.corner_radius, 10.0);
        assert_eq!(lb.border, 2.0);
        assert_eq!(lb.border_color, Color::GREEN);
    }

    #[test]
    fn serde_roundtrip_keeps_items_and_selection_but_not_scroll() {
        let mut lb = ListBox::new(["Sword", "Shield"]).with_selected(1);
        lb.scroll_offset = 42.0;
        let ron = ron::to_string(&lb).expect("serialize");
        let back: ListBox = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.items, vec!["Sword", "Shield"]);
        assert_eq!(back.selected, 1);
        assert_eq!(back.scroll_offset, 0.0, "scroll is runtime-only");
    }

    #[test]
    fn reflect_roundtrip() {
        let mut lb = ListBox::new(["a", "b"]);
        assert!(lb.set_field("selected", ReflectValue::I32(1)));
        assert_eq!(lb.selected, 1);
        assert!(lb.set_field("selected", ReflectValue::I32(-3)));
        assert_eq!(lb.selected, 0, "negative reflect writes clamp to 0");
        assert!(lb.set_field("row_height", ReflectValue::F32(30.0)));
        assert_eq!(lb.row_height, 30.0);
        let fields = lb.fields();
        assert!(fields.iter().any(|(n, _)| *n == "selected"));
        assert!(fields.iter().any(|(n, _)| *n == "selected_color"));
    }
}
