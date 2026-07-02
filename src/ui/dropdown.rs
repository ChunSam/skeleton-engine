use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::reflect::{Reflect, ReflectValue};

/// Z at which an **open** dropdown's item list draws (and captures the pointer) — far above the
/// recommended `0.0..=1.0` [`UiNode`] z range so the list overlays normal UI, but below
/// [`TOOLTIP_Z`](crate::ui::TOOLTIP_Z) so tooltips stay on top of everything.
///
/// [`UiNode`]: crate::ui::UiNode
pub const DROPDOWN_LIST_Z: f32 = 90.0;

/// A dropdown / combobox — click to open an item list, click an item to select it.
///
/// Attach it to an entity alongside a [`UiNode`](crate::ui::UiNode). The closed widget shows the
/// selected item + an arrow; clicking it opens the list just below (flipping **above** when it
/// would overflow the viewport bottom). Clicking an item selects it, closes the list, and emits
/// [`UiEvent::DropdownChanged`](crate::ui::UiEvent::DropdownChanged) when the selection actually
/// changed; pressing anywhere else closes the list without selecting. Press-drag-release onto an
/// item also selects (native combobox feel). While open, the whole box+list area is a
/// pointer-opaque capture surface at [`DROPDOWN_LIST_Z`], so widgets underneath neither fire nor
/// hover through it.
///
/// Keyboard/gamepad: the widget is focusable (Tab / D-pad / stick); **Enter/Space/A** toggles the
/// list open, **←/→** (or D-pad Left/Right) steps the selection directly — a settings row is fully
/// operable without a pointer, without opening the list.
///
/// `open` is runtime-only state (never saved to a scene). An empty `items` list never opens.
///
/// # Example
/// ```
/// # use engine::ui::Dropdown;
/// let dd = Dropdown::new(["Low", "Medium", "High"]).with_selected(1);
/// assert_eq!(dd.selected_item(), Some("Medium"));
/// ```
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Dropdown {
    /// The selectable items, rendered top-to-bottom in the open list.
    pub items: Vec<String>,
    /// Index of the selected item. Read via [`selected_index`](Self::selected_index), which clamps
    /// into range; the raw field is left as set so an out-of-range value is not silently mutated.
    pub selected: usize,
    /// Whether the item list is open. Runtime-only (transient — not saved; toggled by clicks /
    /// Enter; a game may also set it directly, e.g. to open programmatically).
    #[serde(skip)]
    pub open: bool,
    /// Closed-box and list-row background color.
    pub bg_color: Color,
    /// Background of the hovered list row (and the closed box while hovered).
    pub hover_color: Color,
    /// Item text color.
    pub text_color: Color,
    /// Font size in pixels.
    pub font_size: f32,
    /// Corner radius in pixels for the closed box and the open list (`0.0` = sharp).
    pub corner_radius: f32,
    /// Height of one list row in pixels; `0.0` (default) uses the widget's own node height.
    pub item_height: f32,
}

impl Default for Dropdown {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            open: false,
            bg_color: Color::rgba(0.20, 0.20, 0.25, 1.0),
            hover_color: Color::rgba(0.30, 0.30, 0.40, 1.0),
            text_color: Color::rgba_u8(220, 220, 220, 255),
            font_size: 16.0,
            corner_radius: 0.0,
            item_height: 0.0,
        }
    }
}

impl Dropdown {
    /// A dropdown over `items` with the first item selected and the default (button-like) style.
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

    /// Select item `index` (clamped into range on read). Builder form.
    pub fn with_selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Set the text and background colors. Builder form.
    pub fn with_colors(mut self, text_color: Color, bg_color: Color) -> Self {
        self.text_color = text_color;
        self.bg_color = bg_color;
        self
    }

    /// Set the hovered-row background color. Builder form.
    pub fn with_hover_color(mut self, color: Color) -> Self {
        self.hover_color = color;
        self
    }

    /// Set the font size in pixels. Builder form.
    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    /// Round the closed box and list corners to `radius` pixels. Builder form.
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius;
        self
    }

    /// Set an explicit list-row height in pixels (`0.0` = the node's own height). Builder form.
    pub fn with_item_height(mut self, height: f32) -> Self {
        self.item_height = height;
        self
    }

    /// The selected index clamped into `items` range — what the renderer and
    /// [`selected_item`](Self::selected_item) use. `0` when `items` is empty.
    pub fn selected_index(&self) -> usize {
        self.selected.min(self.items.len().saturating_sub(1))
    }

    /// The selected item's text, or `None` when `items` is empty.
    pub fn selected_item(&self) -> Option<&str> {
        self.items.get(self.selected_index()).map(String::as_str)
    }

    /// Resolved list-row height: [`item_height`](Self::item_height) if positive, else
    /// `node_height` (the widget's own height).
    pub fn resolved_item_height(&self, node_height: f32) -> f32 {
        if self.item_height > 0.0 {
            self.item_height
        } else {
            node_height
        }
    }

    /// Total height of the open item list for a widget `node_height` tall.
    pub fn list_height(&self, node_height: f32) -> f32 {
        self.items.len() as f32 * self.resolved_item_height(node_height)
    }

    /// Whether the open list flips **above** the closed box: true when a below-the-box list would
    /// overflow the viewport bottom (and there is room above).
    pub fn flips_up(&self, pos: Vec2, node_size: Vec2, viewport_h: f32) -> bool {
        let list_h = self.list_height(node_size.y);
        pos.y + node_size.y + list_h > viewport_h && pos.y - list_h >= 0.0
    }

    /// Top-left of the open item list (just below the box, or just above when it flips).
    /// The same geometry drives rendering, click row resolution, and the pointer-capture surface,
    /// so they can never disagree.
    pub fn list_pos(&self, pos: Vec2, node_size: Vec2, viewport_h: f32) -> Vec2 {
        if self.flips_up(pos, node_size, viewport_h) {
            Vec2::new(pos.x, pos.y - self.list_height(node_size.y))
        } else {
            Vec2::new(pos.x, pos.y + node_size.y)
        }
    }

    /// The pointer-opaque rect of the widget while **open**: the union of the closed box and the
    /// item list. Returns `(top_left, size)`.
    pub fn expanded_rect(&self, pos: Vec2, node_size: Vec2, viewport_h: f32) -> (Vec2, Vec2) {
        let list_h = self.list_height(node_size.y);
        let top = if self.flips_up(pos, node_size, viewport_h) {
            pos.y - list_h
        } else {
            pos.y
        };
        (
            Vec2::new(pos.x, top),
            Vec2::new(node_size.x, node_size.y + list_h),
        )
    }
}

impl Reflect for Dropdown {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("selected", ReflectValue::I32(self.selected as i32)),
            ("font_size", ReflectValue::F32(self.font_size)),
            ("item_height", ReflectValue::F32(self.item_height)),
            ("corner_radius", ReflectValue::F32(self.corner_radius)),
            ("bg_color", ReflectValue::Color(self.bg_color.to_array())),
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
            ("item_height", ReflectValue::F32(v)) => {
                self.item_height = v;
                true
            }
            ("corner_radius", ReflectValue::F32(v)) => {
                self.corner_radius = v;
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
            ("text_color", ReflectValue::Color(c)) => {
                self.text_color = Color::from(c);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Dropdown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_clamps_on_read_without_mutating() {
        let dd = Dropdown::new(["a", "b", "c"]).with_selected(10);
        assert_eq!(dd.selected, 10, "raw field left as set");
        assert_eq!(dd.selected_index(), 2, "clamped to the last item");
        assert_eq!(dd.selected_item(), Some("c"));

        let empty = Dropdown::new(Vec::<String>::new());
        assert_eq!(empty.selected_index(), 0);
        assert_eq!(empty.selected_item(), None);
    }

    #[test]
    fn item_height_falls_back_to_the_node_height() {
        let dd = Dropdown::new(["a", "b"]);
        assert_eq!(dd.resolved_item_height(30.0), 30.0, "0.0 = node height");
        assert_eq!(dd.list_height(30.0), 60.0);

        let tall = Dropdown::new(["a", "b"]).with_item_height(24.0);
        assert_eq!(tall.resolved_item_height(30.0), 24.0);
        assert_eq!(tall.list_height(30.0), 48.0);
    }

    #[test]
    fn list_opens_below_and_flips_above_at_the_viewport_bottom() {
        let dd = Dropdown::new(["a", "b", "c"]); // list = 3 × 30 = 90 tall
        let size = Vec2::new(120.0, 30.0);

        // Plenty of room below → opens below the box.
        let top = Vec2::new(10.0, 10.0);
        assert!(!dd.flips_up(top, size, 300.0));
        assert_eq!(dd.list_pos(top, size, 300.0), Vec2::new(10.0, 40.0));
        let (epos, esize) = dd.expanded_rect(top, size, 300.0);
        assert_eq!((epos, esize), (top, Vec2::new(120.0, 120.0)));

        // Near the bottom (list would overflow) → flips above.
        let low = Vec2::new(10.0, 260.0);
        assert!(dd.flips_up(low, size, 300.0));
        assert_eq!(dd.list_pos(low, size, 300.0), Vec2::new(10.0, 170.0));
        let (fpos, fsize) = dd.expanded_rect(low, size, 300.0);
        assert_eq!(
            (fpos, fsize),
            (Vec2::new(10.0, 170.0), Vec2::new(120.0, 120.0))
        );

        // Near the bottom but no room above either → stays below (no flip into negative space).
        let cramped = Dropdown::new(["a", "b", "c"]);
        assert!(!cramped.flips_up(Vec2::new(10.0, 40.0), size, 100.0));
    }

    #[test]
    fn builders_set_fields() {
        let dd = Dropdown::new(["x"])
            .with_selected(0)
            .with_colors(Color::WHITE, Color::RED)
            .with_hover_color(Color::BLUE)
            .with_font_size(20.0)
            .with_corner_radius(6.0)
            .with_item_height(28.0);
        assert_eq!(dd.items, vec!["x"]);
        assert_eq!(dd.text_color, Color::WHITE);
        assert_eq!(dd.bg_color, Color::RED);
        assert_eq!(dd.hover_color, Color::BLUE);
        assert_eq!(dd.font_size, 20.0);
        assert_eq!(dd.corner_radius, 6.0);
        assert_eq!(dd.item_height, 28.0);
    }

    #[test]
    fn serde_roundtrip_keeps_items_and_selection_but_drops_open() {
        let mut dd = Dropdown::new(["Low", "High"]).with_selected(1);
        dd.open = true;
        let ron = ron::to_string(&dd).expect("serialize");
        let back: Dropdown = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.items, vec!["Low", "High"]);
        assert_eq!(back.selected, 1);
        assert!(!back.open, "open state is transient");
    }

    #[test]
    fn reflect_roundtrip() {
        let mut dd = Dropdown::new(["a", "b"]);
        assert!(dd.set_field("selected", ReflectValue::I32(1)));
        assert_eq!(dd.selected, 1);
        assert!(dd.set_field("selected", ReflectValue::I32(-3)));
        assert_eq!(dd.selected, 0, "negative reflect writes clamp to 0");
        assert!(dd.set_field("font_size", ReflectValue::F32(22.0)));
        assert_eq!(dd.font_size, 22.0);
        let fields = dd.fields();
        assert!(fields.iter().any(|(n, _)| *n == "selected"));
        assert!(fields.iter().any(|(n, _)| *n == "hover_color"));
    }
}
