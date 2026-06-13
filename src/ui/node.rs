use glam::Vec2;
use serde::{Deserialize, Serialize};

use crate::reflect::{Reflect, ReflectValue};
use crate::resources::ViewportSize;

/// Anchor point for a UI node. Positions are computed relative to a viewport corner or center.
#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub enum Anchor {
    #[default]
    TopLeft,
    TopCenter,
    TopRight,
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

impl Anchor {
    /// Maps each variant to a stable integer index (declaration order: 0 = TopLeft … 6 = BottomRight).
    pub fn to_i32(self) -> i32 {
        match self {
            Anchor::TopLeft => 0,
            Anchor::TopCenter => 1,
            Anchor::TopRight => 2,
            Anchor::Center => 3,
            Anchor::BottomLeft => 4,
            Anchor::BottomCenter => 5,
            Anchor::BottomRight => 6,
        }
    }

    /// Converts a stable integer index back to an `Anchor` (unknown values → `TopLeft`).
    pub fn from_i32(i: i32) -> Anchor {
        match i {
            0 => Anchor::TopLeft,
            1 => Anchor::TopCenter,
            2 => Anchor::TopRight,
            3 => Anchor::Center,
            4 => Anchor::BottomLeft,
            5 => Anchor::BottomCenter,
            6 => Anchor::BottomRight,
            _ => Anchor::TopLeft,
        }
    }
}

/// Screen-space UI position and size component.
///
/// `offset` is the pixel offset from the `anchor` point.
/// `z` determines the relative depth among UI nodes (higher value = drawn on top).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiNode {
    /// Pixel offset from the anchor point (top-left origin).
    pub offset: Vec2,
    /// Width and height of the node (pixels).
    pub size: Vec2,
    /// Rendering depth. Higher value = drawn in front (recommended range 0.0 ~ 1.0).
    pub z: f32,
    pub anchor: Anchor,
    pub visible: bool,
}

impl Default for UiNode {
    fn default() -> Self {
        Self::new(0.0, 0.0, 100.0, 30.0)
    }
}

impl Reflect for UiNode {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![
            ("offset", ReflectValue::Vec2(self.offset)),
            ("size", ReflectValue::Vec2(self.size)),
            ("z", ReflectValue::F32(self.z)),
            ("visible", ReflectValue::Bool(self.visible)),
            ("anchor", ReflectValue::I32(self.anchor.to_i32())),
        ]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("offset", ReflectValue::Vec2(v)) => {
                self.offset = v;
                true
            }
            ("size", ReflectValue::Vec2(v)) => {
                self.size = v;
                true
            }
            ("z", ReflectValue::F32(v)) => {
                self.z = v;
                true
            }
            ("visible", ReflectValue::Bool(v)) => {
                self.visible = v;
                true
            }
            ("anchor", ReflectValue::I32(v)) => {
                self.anchor = Anchor::from_i32(v);
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "UiNode"
    }
}

impl UiNode {
    /// Creates a node with top-left anchor and z=0.9 as defaults.
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            offset: Vec2::new(x, y),
            size: Vec2::new(w, h),
            z: 0.9,
            anchor: Anchor::TopLeft,
            visible: true,
        }
    }

    pub fn with_anchor(mut self, anchor: Anchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn with_z(mut self, z: f32) -> Self {
        self.z = z;
        self
    }

    /// Returns the absolute top-left screen pixel coordinate of this node given the viewport size.
    pub fn screen_pos(&self, viewport: &ViewportSize) -> Vec2 {
        let (vw, vh) = (viewport.width, viewport.height);
        let (w, h) = (self.size.x, self.size.y);
        let base = match self.anchor {
            Anchor::TopLeft => Vec2::ZERO,
            Anchor::TopCenter => Vec2::new((vw - w) / 2.0, 0.0),
            Anchor::TopRight => Vec2::new(vw - w, 0.0),
            Anchor::Center => Vec2::new((vw - w) / 2.0, (vh - h) / 2.0),
            Anchor::BottomLeft => Vec2::new(0.0, vh - h),
            Anchor::BottomCenter => Vec2::new((vw - w) / 2.0, vh - h),
            Anchor::BottomRight => Vec2::new(vw - w, vh - h),
        };
        base + self.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anchor_to_i32_from_i32_roundtrip() {
        let variants = [
            Anchor::TopLeft,
            Anchor::TopCenter,
            Anchor::TopRight,
            Anchor::Center,
            Anchor::BottomLeft,
            Anchor::BottomCenter,
            Anchor::BottomRight,
        ];
        for &a in &variants {
            assert_eq!(Anchor::from_i32(a.to_i32()), a);
        }
    }

    #[test]
    fn ui_node_serde_roundtrip() {
        let node = UiNode::new(10.0, 20.0, 200.0, 50.0)
            .with_anchor(Anchor::Center)
            .with_z(0.5);
        let ron = ron::to_string(&node).expect("serialize");
        let back: UiNode = ron::from_str(&ron).expect("deserialize");
        assert_eq!(back.offset, node.offset);
        assert_eq!(back.size, node.size);
        assert_eq!(back.z, node.z);
        assert_eq!(back.anchor, Anchor::Center);
        assert!(back.visible);
    }

    #[test]
    fn ui_node_reflect_roundtrip() {
        let mut node = UiNode::new(5.0, 10.0, 100.0, 40.0);
        let fields = node.fields();
        assert!(fields.iter().any(|(n, _)| *n == "z"));
        assert!(node.set_field("z", ReflectValue::F32(0.3)));
        assert!((node.z - 0.3).abs() < f32::EPSILON);
        assert!(node.set_field("visible", ReflectValue::Bool(false)));
        assert!(!node.visible);
        assert!(node.set_field("anchor", ReflectValue::I32(3)));
        assert_eq!(node.anchor, Anchor::Center);
    }

    #[test]
    fn center_anchor_positions_correctly() {
        let vp = ViewportSize {
            width: 800.0,
            height: 600.0,
        };
        let node = UiNode::new(0.0, 0.0, 200.0, 50.0).with_anchor(Anchor::Center);
        let pos = node.screen_pos(&vp);
        assert_eq!(pos, Vec2::new(300.0, 275.0));
    }

    #[test]
    fn bottom_right_anchor() {
        let vp = ViewportSize {
            width: 800.0,
            height: 600.0,
        };
        let node = UiNode::new(-10.0, -10.0, 100.0, 40.0).with_anchor(Anchor::BottomRight);
        let pos = node.screen_pos(&vp);
        assert_eq!(pos, Vec2::new(690.0, 550.0));
    }
}
