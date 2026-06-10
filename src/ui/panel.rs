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
