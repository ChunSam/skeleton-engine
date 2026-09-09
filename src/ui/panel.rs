use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::ecs::{Entity, System, World};
use crate::reflect::{Reflect, ReflectValue};
use crate::renderer::{DrawRect, UiQueue};
use crate::resources::ViewportSize;

use super::node::{Anchor, UiNode};

/// Z-offset below a panel's own z at which [`LayoutSystem`] draws the panel background, so the
/// background renders beneath the panel's child widgets. The pointer-capture pass mirrors this
/// (see `crate::ui::system::capture`), so both must agree — hence one shared constant.
pub(crate) const PANEL_BG_Z_OFFSET: f32 = 0.01;

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

/// One panel's layout inputs, read once per frame so `get_mut` is legal afterwards.
///
/// `children` is a **range into [`LayoutSystem::children`]**, not a `Vec` of its own: the child
/// list used to be cloned per panel per frame, which is one allocation per panel in the steady
/// state a game actually runs in.
struct PanelSnapshot {
    entity: Entity,
    children: std::ops::Range<usize>,
    gap: f32,
    direction: LayoutDir,
    padding: f32,
    // background draw data
    size: Vec2,
    z: f32,
    visible: bool,
    bg_color: Color,
}

/// Index of the snapshot describing `e`, or `None` if `e` is not itself a panel.
///
/// A free function rather than a closure so it borrows only the slice, leaving the sibling scratch
/// fields free to be written in the same loop.
fn index_of(snapshots: &[PanelSnapshot], e: Entity) -> Option<usize> {
    snapshots.iter().position(|s| s.entity == e)
}

/// System that updates Panel child entity positions before UiSystem runs.
///
/// Register with `app.add_system(LayoutSystem::default())` before `UiSystem`.
///
/// ⚠️ **This was a unit struct until v0.157.0** — a bare `LayoutSystem` no longer compiles as a
/// value and becomes `LayoutSystem::default()` (or `LayoutSystem::new()`). The five buffers below are per-frame temporaries
/// held across frames (`clear()` + refill), which is this repo's rule for anything running every
/// frame; measured at 50 panels × 8 children, the old body allocated 58 times / 10,066 bytes on an
/// unchanged frame.
#[derive(Default)]
pub struct LayoutSystem {
    snapshots: Vec<PanelSnapshot>,
    /// Flattened child lists — `PanelSnapshot::children` indexes into this.
    children: Vec<Entity>,
    /// Per snapshot: false while it is known to be some other panel's child, then reused as
    /// "already placed in `order`" by the parents-first walk.
    queued: Vec<bool>,
    /// Snapshot indices, parents before children.
    order: Vec<usize>,
    /// Panel background rects, drained into `UiQueue` at the end of the frame.
    rects: Vec<DrawRect>,
}

impl LayoutSystem {
    /// Schedule label. Recommended order: **before** `UiSystem::LABEL`
    /// (`SystemConfig::new().label(LayoutSystem::LABEL).before(UiSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::ui_layout";

    /// Creates a new `LayoutSystem`. Equivalent to `LayoutSystem::default()`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl System for LayoutSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let viewport = match world.resource::<ViewportSize>().copied() {
            Some(v) => v,
            None => return,
        };

        // Step 1: collect panel layout + background data in a single pass.
        // Can't call get_mut while the query iterator is live, so collect first
        // (the repo's standard borrow-checker workaround). Both buffers are scratch fields:
        // `children` flattens every panel's child list so none of them is cloned per frame.
        self.snapshots.clear();
        self.children.clear();
        for (entity, node, panel) in world.query2::<UiNode, Panel>() {
            let start = self.children.len();
            self.children.extend_from_slice(&panel.children);
            self.snapshots.push(PanelSnapshot {
                entity,
                children: start..self.children.len(),
                gap: panel.gap,
                direction: panel.direction,
                padding: panel.padding,
                size: node.size,
                z: node.z,
                visible: node.visible,
                bg_color: panel.background_color,
            });
        }
        let n = self.snapshots.len();

        // Step 2: order panels parents-first. A nested panel's own position is written by its
        // parent's pass below, so laying the inner one out first would place its children against
        // the position the inner panel held at the *start* of the frame — one frame of lag per
        // nesting level, for the children and for the panel background alike.
        self.queued.clear();
        self.queued.resize(n, true);
        for i in 0..n {
            for k in self.snapshots[i].children.clone() {
                if let Some(j) = index_of(&self.snapshots, self.children[k]) {
                    self.queued[j] = false; // has a panel parent → not a root
                }
            }
        }
        self.order.clear();
        for i in 0..n {
            if self.queued[i] {
                self.order.push(i);
            }
        }
        let mut head = 0;
        while head < self.order.len() {
            let i = self.order[head];
            head += 1;
            for k in self.snapshots[i].children.clone() {
                if let Some(j) = index_of(&self.snapshots, self.children[k]) {
                    if !self.queued[j] {
                        self.queued[j] = true;
                        self.order.push(j);
                    }
                }
            }
        }
        // A cycle in the child links (nothing forbids one — `children` is a free `pub` field)
        // leaves panels unreached. Lay each of those out once, in collect order, rather than
        // looping forever or dropping it.
        for i in 0..n {
            if !self.queued[i] {
                self.order.push(i);
            }
        }

        // Step 3: iterator released after collect → get_mut is safe.
        self.rects.clear();
        for oi in 0..self.order.len() {
            let i = self.order[oi];
            let (entity, child_range, gap, direction, padding, size, z, visible, bg_color) = {
                let snap = &self.snapshots[i];
                (
                    snap.entity,
                    snap.children.clone(),
                    snap.gap,
                    snap.direction,
                    snap.padding,
                    snap.size,
                    snap.z,
                    snap.visible,
                    snap.bg_color,
                )
            };
            // Read the panel's position *now*, not at collect time: an outer panel earlier in
            // this same loop may have just moved it.
            let panel_pos = match world.get::<UiNode>(entity) {
                Some(node) => node.screen_pos(&viewport),
                None => continue,
            };

            // Layout children.
            let start_x = panel_pos.x + padding;
            let start_y = panel_pos.y + padding;
            let mut cursor_x = start_x;
            let mut cursor_y = start_y;

            for k in child_range {
                let child_entity = self.children[k];
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

            // Collect background rect (rendered beneath children via z − 0.01).
            //
            // Note: panel background emission intentionally happens in LayoutSystem
            // rather than UiSystem. LayoutSystem runs first and pushes background rects
            // into UiQueue before UiSystem pushes widget rects. Combined with the
            // `z − 0.01` offset this guarantees backgrounds are drawn under all widgets
            // without requiring a separate panel pass inside UiSystem.
            if visible {
                self.rects.push(
                    DrawRect::new(panel_pos.x, panel_pos.y, size.x, size.y, bg_color)
                        .with_z(z - PANEL_BG_Z_OFFSET),
                );
            }
        }

        if let Some(ui_queue) = world.resource_mut::<UiQueue>() {
            for rect in self.rects.drain(..) {
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

    /// Nested panels used to lag one frame per level: every panel's position was snapshotted
    /// before any child was moved, so an inner panel laid its own children out against the
    /// position it held at the start of the frame. Spawn order matters here — the inner panel is
    /// spawned first on purpose, which is the order that used to lag.
    #[test]
    fn a_nested_panel_lays_out_its_children_in_the_frame_the_outer_panel_moves() {
        fn pos(world: &World, e: Entity) -> Vec2 {
            world.get::<UiNode>(e).expect("node").offset
        }

        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));

        let leaf = world.spawn();
        world.add_component(leaf, UiNode::new(0.0, 0.0, 20.0, 20.0));

        let inner = world.spawn();
        world.add_component(inner, UiNode::new(0.0, 0.0, 100.0, 100.0));
        let mut inner_panel = Panel::new(LayoutDir::Vertical);
        inner_panel.children.push(leaf);
        world.add_component(inner, inner_panel);

        let outer = world.spawn();
        world.add_component(outer, UiNode::new(0.0, 0.0, 200.0, 200.0));
        let mut outer_panel = Panel::new(LayoutDir::Vertical);
        outer_panel.children.push(inner);
        world.add_component(outer, outer_panel);

        let mut sys = LayoutSystem::default();
        sys.run(&mut world, 1.0 / 60.0);
        // Default padding is 8: outer at 0 puts inner at 8, inner at 8 puts the leaf at 16.
        assert_eq!(pos(&world, inner), Vec2::new(8.0, 8.0));
        assert_eq!(pos(&world, leaf), Vec2::new(16.0, 16.0));

        // Move the outer panel. One frame later every level below it must have followed.
        world.get_mut::<UiNode>(outer).expect("outer node").offset = Vec2::new(100.0, 0.0);
        sys.run(&mut world, 1.0 / 60.0);

        assert_eq!(pos(&world, inner), Vec2::new(108.0, 8.0));
        assert_eq!(
            pos(&world, leaf),
            Vec2::new(116.0, 16.0),
            "the leaf lagged a frame behind its grandparent"
        );
    }

    /// The background rect is emitted from the same position the children are laid out against,
    /// so it must not lag either.
    #[test]
    fn a_nested_panel_draws_its_background_at_the_position_it_moved_to() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(UiQueue::default());

        let inner = world.spawn();
        world.add_component(inner, UiNode::new(0.0, 0.0, 100.0, 100.0));
        world.add_component(inner, Panel::new(LayoutDir::Vertical));

        let outer = world.spawn();
        world.add_component(outer, UiNode::new(0.0, 0.0, 200.0, 200.0));
        let mut outer_panel = Panel::new(LayoutDir::Vertical);
        outer_panel.children.push(inner);
        world.add_component(outer, outer_panel);

        let mut sys = LayoutSystem::default();
        sys.run(&mut world, 1.0 / 60.0);
        world
            .resource_mut::<UiQueue>()
            .expect("queue")
            .items
            .clear();

        world.get_mut::<UiNode>(outer).expect("outer node").offset = Vec2::new(100.0, 0.0);
        sys.run(&mut world, 1.0 / 60.0);

        let rects = &world.resource::<UiQueue>().expect("queue").items;
        assert!(
            rects.iter().any(|r| r.x == 108.0 && r.y == 8.0),
            "inner panel background is not at the position its parent just moved it to: {:?}",
            rects.iter().map(|r| (r.x, r.y)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_cycle_in_the_child_links_does_not_hang_the_layout_pass() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));

        let a = world.spawn();
        world.add_component(a, UiNode::new(0.0, 0.0, 100.0, 100.0));
        let b = world.spawn();
        world.add_component(b, UiNode::new(0.0, 0.0, 100.0, 100.0));

        let mut pa = Panel::new(LayoutDir::Vertical);
        pa.children.push(b);
        world.add_component(a, pa);
        let mut pb = Panel::new(LayoutDir::Vertical);
        pb.children.push(a);
        world.add_component(b, pb);

        // Every panel is somebody's child, so the root set is empty and the pass has to fall back
        // to collect order rather than skipping them or looping.
        let mut sys = LayoutSystem::default();
        sys.run(&mut world, 1.0 / 60.0);
        assert!(world.get::<UiNode>(a).is_some());
        assert!(world.get::<UiNode>(b).is_some());
    }

    #[test]
    fn layout_dir_to_i32_from_i32_roundtrip() {
        assert_eq!(
            LayoutDir::from_i32(LayoutDir::Vertical.to_i32()),
            LayoutDir::Vertical
        );
        assert_eq!(
            LayoutDir::from_i32(LayoutDir::Horizontal.to_i32()),
            LayoutDir::Horizontal
        );
        // Unknown values fall back to Vertical.
        assert_eq!(LayoutDir::from_i32(99), LayoutDir::Vertical);
    }

    #[test]
    fn panel_direction_reflect_set_field() {
        // Start Vertical, switch to Horizontal via set_field.
        let mut p = Panel::new(LayoutDir::Vertical);
        assert_eq!(p.direction, LayoutDir::Vertical);

        assert!(p.set_field("direction", ReflectValue::I32(1)));
        assert_eq!(
            p.direction,
            LayoutDir::Horizontal,
            "set_field(\"direction\", I32(1)) should switch layout to Horizontal"
        );

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
