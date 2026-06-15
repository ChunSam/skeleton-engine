use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ecs::{Entity, System, World};
use crate::reflect::{Reflect, ReflectValue};
use crate::renderer::{DrawRect, UiQueue};
use crate::resources::ViewportSize;

use super::node::{Anchor, UiNode};

/// Layout direction for child entities.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayoutDir {
    Vertical,
    Horizontal,
}

impl LayoutDir {
    /// Maps each variant to a stable integer index (0 = Vertical, 1 = Horizontal).
    pub fn to_i32(self) -> i32 {
        match self {
            LayoutDir::Vertical => 0,
            LayoutDir::Horizontal => 1,
        }
    }

    /// Converts a stable integer index back to a `LayoutDir` (unknown values → `Vertical`).
    pub fn from_i32(i: i32) -> LayoutDir {
        match i {
            1 => LayoutDir::Horizontal,
            _ => LayoutDir::Vertical,
        }
    }
}

/// Layout container that automatically positions child entities.
///
/// Attach alongside a `UiNode` on the same entity.
/// `LayoutSystem` repositions the `children`'s `UiNode`s every frame.
/// `UiSystem` renders the background rectangle.
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Panel {
    /// Runtime child entity list — not serialized (entities are resolved at spawn time).
    #[serde(skip)]
    pub children: Vec<Entity>,
    pub gap: f32,
    pub direction: LayoutDir,
    pub padding: f32,
    pub background_color: Color,
}

impl Default for Panel {
    fn default() -> Self {
        Self::new(LayoutDir::Vertical)
    }
}

impl Reflect for Panel {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("gap", ReflectValue::F32(self.gap)),
            ("padding", ReflectValue::F32(self.padding)),
            (
                "background_color",
                ReflectValue::Color(self.background_color.to_array()),
            ),
            ("direction", ReflectValue::I32(self.direction.to_i32())),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("gap", ReflectValue::F32(v)) => {
                self.gap = v;
                true
            }
            ("padding", ReflectValue::F32(v)) => {
                self.padding = v;
                true
            }
            ("background_color", ReflectValue::Color(c)) => {
                self.background_color = Color::from(c);
                true
            }
            ("direction", ReflectValue::I32(v)) => {
                self.direction = LayoutDir::from_i32(v);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "Panel"
    }
}

impl Panel {
    pub fn new(direction: LayoutDir) -> Self {
        Self {
            children: Vec::new(),
            gap: 8.0,
            direction,
            padding: 8.0,
            background_color: Color::rgba(0.12, 0.12, 0.18, 0.9),
        }
    }

    pub fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

/// System that updates Panel child entity positions before UiSystem runs.
///
/// Register with `app.add_system(Box::new(LayoutSystem))` before `UiSystem`.
pub struct LayoutSystem;

impl LayoutSystem {
    /// Schedule label. Recommended order: **before** `UiSystem::LABEL`
    /// (`SystemConfig::new().label(LayoutSystem::LABEL).before(UiSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::ui_layout";
}

impl System for LayoutSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let viewport = match world.resource::<ViewportSize>().copied() {
            Some(v) => v,
            None => return,
        };

        // Step 1: collect panel layout + background data in a single pass.
        // Can't call get_mut while the query iterator is live, so collect first
        // (the repo's standard borrow-checker workaround).
        struct PanelSnapshot {
            children: Vec<Entity>,
            gap: f32,
            direction: LayoutDir,
            padding: f32,
            panel_pos: Vec2,
            // background draw data
            size: Vec2,
            z: f32,
            visible: bool,
            bg_color: Color,
        }
        let snapshots: Vec<PanelSnapshot> = world
            .query2::<UiNode, Panel>()
            .map(|(_, node, panel)| {
                let panel_pos = node.screen_pos(&viewport);
                PanelSnapshot {
                    children: panel.children.clone(),
                    gap: panel.gap,
                    direction: panel.direction,
                    padding: panel.padding,
                    panel_pos,
                    size: node.size,
                    z: node.z,
                    visible: node.visible,
                    bg_color: panel.background_color,
                }
            })
            .collect();

        // Step 2: iterator released after collect → get_mut is safe.
        let mut rects: Vec<DrawRect> = Vec::new();
        for snap in snapshots {
            // Layout children.
            let start_x = snap.panel_pos.x + snap.padding;
            let start_y = snap.panel_pos.y + snap.padding;
            let mut cursor_x = start_x;
            let mut cursor_y = start_y;

            for child_entity in snap.children {
                let child_size = match world.get::<UiNode>(child_entity) {
                    Some(n) => n.size,
                    None => continue,
                };
                if let Some(child_node) = world.get_mut::<UiNode>(child_entity) {
                    child_node.anchor = Anchor::TopLeft;
                    match snap.direction {
                        LayoutDir::Vertical => {
                            child_node.offset = Vec2::new(start_x, cursor_y);
                            cursor_y += child_size.y + snap.gap;
                        }
                        LayoutDir::Horizontal => {
                            child_node.offset = Vec2::new(cursor_x, start_y);
                            cursor_x += child_size.x + snap.gap;
                        }
                    }
                }
            }

            // Collect background rect (rendered beneath children via z − 0.01).
            //
            // Note: panel background emission intentionally happens in LayoutSystem
            // rather than UiSystem. LayoutSystem runs first and pushes background rects
            // into UiQueue before UiSystem pushes widget rects. Combined with the
            // `z − 0.01` offset this guarantees backgrounds are drawn under all widgets
            // without requiring a separate panel pass inside UiSystem.
            if snap.visible {
                rects.push(
                    DrawRect::new(
                        snap.panel_pos.x,
                        snap.panel_pos.y,
                        snap.size.x,
                        snap.size.y,
                        snap.bg_color,
                    )
                    .with_z(snap.z - 0.01),
                );
            }
        }

        if let Some(ui_queue) = world.resource_mut::<UiQueue>() {
            for rect in rects {
                ui_queue.push(rect);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_serde_roundtrip() {
        let p = Panel::new(LayoutDir::Horizontal)
            .with_gap(12.0)
            .with_padding(4.0);
        let ron = ron::to_string(&p).expect("serialize");
        // children (Vec<Entity>) must not be in the serialized output.
        assert!(!ron.contains("children"), "children leaked into RON: {ron}");
        let back: Panel = ron::from_str(&ron).expect("deserialize");
        assert!((back.gap - 12.0).abs() < f32::EPSILON);
        assert!((back.padding - 4.0).abs() < f32::EPSILON);
        assert_eq!(back.direction, LayoutDir::Horizontal);
        assert!(back.children.is_empty());
    }

    #[test]
    fn panel_reflect_roundtrip() {
        let mut p = Panel::new(LayoutDir::Vertical);
        assert!(p.set_field("gap", ReflectValue::F32(16.0)));
        assert!((p.gap - 16.0).abs() < f32::EPSILON);
        assert!(p.set_field("padding", ReflectValue::F32(8.0)));
        assert!((p.padding - 8.0).abs() < f32::EPSILON);
        let fields = p.fields();
        assert!(fields.iter().any(|(n, _)| *n == "background_color"));
    }

    #[test]
    fn layout_dir_to_i32_from_i32_roundtrip() {
        assert_eq!(LayoutDir::from_i32(LayoutDir::Vertical.to_i32()), LayoutDir::Vertical);
        assert_eq!(LayoutDir::from_i32(LayoutDir::Horizontal.to_i32()), LayoutDir::Horizontal);
        // Unknown values fall back to Vertical.
        assert_eq!(LayoutDir::from_i32(99), LayoutDir::Vertical);
    }

    #[test]
    fn panel_direction_reflect_set_field() {
        // Start Vertical, switch to Horizontal via set_field.
        let mut p = Panel::new(LayoutDir::Vertical);
        assert_eq!(p.direction, LayoutDir::Vertical);

        assert!(p.set_field("direction", ReflectValue::I32(1)));
        assert_eq!(p.direction, LayoutDir::Horizontal,
            "set_field(\"direction\", I32(1)) should switch layout to Horizontal");

        // Verify the field round-trips through fields() → I32.
        let fields = p.fields();
        let dir_field = fields.iter().find(|(n, _)| *n == "direction");
        assert!(dir_field.is_some(), "direction missing from fields()");
        assert_eq!(dir_field.unwrap().1, ReflectValue::I32(1));

        // Switch back to Vertical.
        assert!(p.set_field("direction", ReflectValue::I32(0)));
        assert_eq!(p.direction, LayoutDir::Vertical);
    }
}
