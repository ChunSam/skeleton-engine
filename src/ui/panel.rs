use glam::Vec2;

use crate::color::Color;
use crate::ecs::{Entity, System, World};
use crate::renderer::{DrawRect, UiQueue};
use crate::resources::ViewportSize;

use super::node::{Anchor, UiNode};

/// Layout direction for child entities.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LayoutDir {
    Vertical,
    Horizontal,
}

/// Layout container that automatically positions child entities.
///
/// Attach alongside a `UiNode` on the same entity.
/// `LayoutSystem` repositions the `children`'s `UiNode`s every frame.
/// `UiSystem` renders the background rectangle.
pub struct Panel {
    pub children: Vec<Entity>,
    pub gap: f32,
    pub direction: LayoutDir,
    pub padding: f32,
    pub background_color: Color,
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
        let viewport = match world.resource::<ViewportSize>() {
            Some(v) => ViewportSize {
                width: v.width,
                height: v.height,
            },
            None => return,
        };

        // Step 1: collect panel data — can't call get_mut while the iterator is live, so collect first
        let panel_data: Vec<(Vec<Entity>, f32, LayoutDir, f32, Vec2)> = world
            .query2::<UiNode, Panel>()
            .map(|(_, node, panel)| {
                let pos = node.screen_pos(&viewport);
                (
                    panel.children.clone(),
                    panel.gap,
                    panel.direction,
                    panel.padding,
                    pos,
                )
            })
            .collect();

        // Step 2: iterator released after collect → get_mut is safe
        for (children, gap, direction, padding, panel_pos) in panel_data {
            let start_x = panel_pos.x + padding;
            let start_y = panel_pos.y + padding;
            let mut cursor_x = start_x;
            let mut cursor_y = start_y;

            for child_entity in children {
                let child_size = match world.get::<UiNode>(child_entity) {
                    Some(n) => n.size,
                    None => continue,
                };
                if let Some(child_node) = world.get_mut::<UiNode>(child_entity) {
                    child_node.anchor = Anchor::TopLeft;
                    match direction {
                        LayoutDir::Vertical => {
                            child_node.offset = Vec2::new(start_x, cursor_y);
                            cursor_y += child_size.y + gap;
                        }
                        LayoutDir::Horizontal => {
                            child_node.offset = Vec2::new(cursor_x, start_y);
                            cursor_x += child_size.x + gap;
                        }
                    }
                }
            }
        }

        // Step 3: render panel backgrounds (at a lower z than children)
        let panel_entities: Vec<Entity> =
            world.query2::<UiNode, Panel>().map(|(e, _, _)| e).collect();

        let mut rects: Vec<DrawRect> = Vec::new();
        for entity in panel_entities {
            let (pos, size, z, visible) = match world.get::<UiNode>(entity) {
                Some(n) => (n.screen_pos(&viewport), n.size, n.z, n.visible),
                None => continue,
            };
            if !visible {
                continue;
            }
            let bg_color = match world.get::<Panel>(entity) {
                Some(p) => p.background_color,
                None => continue,
            };
            rects.push(DrawRect::new(pos.x, pos.y, size.x, size.y, bg_color).with_z(z - 0.01));
        }

        if let Some(ui_queue) = world.resource_mut::<UiQueue>() {
            for rect in rects {
                ui_queue.push(rect);
            }
        }
    }
}
