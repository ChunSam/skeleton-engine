//! General debug-draw API — collect outline/filled shapes each frame for the renderer.

use glam::Vec2;

use crate::color::Color;

/// A single debug shape.
///
/// This enum is `#[non_exhaustive]`: external crates matching on it must include a
/// wildcard (`_ =>`) arm to remain forward-compatible as new variants are added.
#[non_exhaustive]
#[derive(Debug, Clone)]
pub enum DebugShape {
    /// Axis-aligned rectangle (outline)
    Rect { min: Vec2, max: Vec2, color: Color },
    /// Line segment (start → end, `thickness` px wide)
    Line {
        start: Vec2,
        end: Vec2,
        color: Color,
        thickness: f32,
    },
    /// Circle (24-sided polygon approximation)
    Circle {
        center: Vec2,
        radius: f32,
        color: Color,
    },
    /// Cross marker (two intersecting lines)
    Cross { pos: Vec2, size: f32, color: Color },
}

/// A filled, z-ordered rectangle collected via [`DebugDraw::rect_filled_z`].
/// Engine-internal: the renderer drains these into `UiQueue` alongside shapes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FilledRect {
    pub min: Vec2,
    pub max: Vec2,
    pub color: Color,
    pub z: f32,
}

/// Resource that collects debug shapes each frame.
///
/// `App` automatically calls `clear()` after rendering, so simply re-draw each frame.
///
/// # Example
/// ```rust,ignore
/// // inside a system
/// if let Some(dbg) = world.resource_mut::<DebugDraw>() {
///     dbg.rect(Vec2::new(0., 0.), Vec2::new(64., 64.), [1., 0., 0., 1.]);
///     dbg.circle(player_pos, 32., [0., 1., 0., 0.8]);
///     dbg.line(from, to, [1., 1., 0., 1.]);
///     dbg.rect_filled_z(min, max, [0.2, 0.2, 0.3, 1.0], 0.5); // filled, z-ordered
/// }
/// ```
#[derive(Debug, Default)]
pub struct DebugDraw {
    pub(crate) shapes: Vec<DebugShape>,
    pub(crate) filled_rects: Vec<FilledRect>,
}

impl DebugDraw {
    pub fn new() -> Self {
        Self::default()
    }

    /// Draws an axis-aligned rectangle outline.
    pub fn rect(&mut self, min: Vec2, max: Vec2, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Rect {
            min,
            max,
            color: color.into(),
        });
    }

    /// Draws a line segment (default thickness 1.5 px).
    pub fn line(&mut self, start: Vec2, end: Vec2, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Line {
            start,
            end,
            color: color.into(),
            thickness: 1.5,
        });
    }

    /// Draws a line segment with the given thickness.
    pub fn line_thick(&mut self, start: Vec2, end: Vec2, color: impl Into<Color>, thickness: f32) {
        self.shapes.push(DebugShape::Line {
            start,
            end,
            color: color.into(),
            thickness,
        });
    }

    /// Draws a circle (24-sided polygon approximation).
    pub fn circle(&mut self, center: Vec2, radius: f32, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Circle {
            center,
            radius,
            color: color.into(),
        });
    }

    /// Draws a cross marker.
    pub fn cross(&mut self, pos: Vec2, size: f32, color: impl Into<Color>) {
        self.shapes.push(DebugShape::Cross {
            pos,
            size,
            color: color.into(),
        });
    }

    /// Draws a filled rectangle (z = 0.0).
    pub fn rect_filled(&mut self, min: Vec2, max: Vec2, color: impl Into<Color>) {
        self.rect_filled_z(min, max, color, 0.0);
    }

    /// Draws a filled rectangle at the given z-order (higher = drawn on top).
    ///
    /// This covers what the pre-v5 `DebugRect`/`DebugDrawQueue` pair did —
    /// translucent collision overlays, editor selection highlights, or quick
    /// rect-based prototype rendering (see the `sokoban` example).
    pub fn rect_filled_z(&mut self, min: Vec2, max: Vec2, color: impl Into<Color>, z: f32) {
        self.filled_rects.push(FilledRect {
            min,
            max,
            color: color.into(),
            z,
        });
    }

    /// Clears all shapes for this frame. Called automatically by `App` after rendering.
    pub fn clear(&mut self) {
        self.shapes.clear();
        self.filled_rects.clear();
    }

    /// Returns the slice of collected shapes.
    pub fn shapes(&self) -> &[DebugShape] {
        &self.shapes
    }
}

#[cfg(test)]
mod debug_draw_tests {
    use super::*;
    use glam::Vec2;

    #[test]
    fn debug_draw_accumulates_shapes() {
        let mut dbg = DebugDraw::new();
        dbg.rect(Vec2::ZERO, Vec2::ONE * 64., [1., 0., 0., 1.]);
        dbg.circle(Vec2::new(100., 100.), 32., [0., 1., 0., 1.]);
        dbg.line(Vec2::ZERO, Vec2::new(50., 50.), [0., 0., 1., 1.]);
        assert_eq!(dbg.shapes().len(), 3);
    }

    #[test]
    fn debug_draw_clear_empties() {
        let mut dbg = DebugDraw::new();
        dbg.rect(Vec2::ZERO, Vec2::ONE, [1.; 4]);
        dbg.clear();
        assert!(dbg.shapes().is_empty());
    }

    #[test]
    fn debug_draw_cross_is_correct_shape() {
        let mut dbg = DebugDraw::new();
        dbg.cross(Vec2::new(50., 50.), 20., [1.; 4]);
        assert_eq!(dbg.shapes().len(), 1);
        matches!(&dbg.shapes()[0], DebugShape::Cross { .. });
    }

    #[test]
    fn debug_draw_line_thick() {
        let mut dbg = DebugDraw::new();
        dbg.line_thick(Vec2::ZERO, Vec2::new(100., 0.), [1.; 4], 3.0);
        assert_eq!(dbg.shapes().len(), 1);
        if let DebugShape::Line { thickness, .. } = &dbg.shapes()[0] {
            assert_eq!(*thickness, 3.0);
        } else {
            panic!("expected Line shape");
        }
    }
}
