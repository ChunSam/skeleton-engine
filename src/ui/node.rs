use glam::Vec2;

use crate::resources::ViewportSize;

/// Anchor point for a UI node. Positions are computed relative to a viewport corner or center.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
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

/// Screen-space UI position and size component.
///
/// `offset` is the pixel offset from the `anchor` point.
/// `z` determines the relative depth among UI nodes (higher value = drawn on top).
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
