use super::super::App;
use crate::renderer::{DrawRect, UiQueue};

impl App {
    pub(in crate::app) fn debug_shape_to_draw_rects(
        shape: crate::resources::DebugShape,
        q: &mut UiQueue,
    ) {
        use crate::resources::DebugShape;
        const Z: f32 = 999.0;

        // Line-segment approximation helper: fills the segment between two points with thickness×thickness dots.
        let mut push_line =
            |start: glam::Vec2, end: glam::Vec2, color: crate::color::Color, thickness: f32| {
                let delta = end - start;
                let len = delta.length();
                if len < 0.001 {
                    return;
                }
                let steps = (len / thickness.max(0.5)).ceil() as usize;
                let half = thickness / 2.0;
                for i in 0..=steps {
                    let t = i as f32 / steps.max(1) as f32;
                    let pos = start + delta * t;
                    q.push(
                        DrawRect::new(pos.x - half, pos.y - half, thickness, thickness, color)
                            .with_z(Z),
                    );
                }
            };

        match shape {
            DebugShape::Rect { min, max, color } => {
                let t = 1.5_f32;
                let w = max.x - min.x;
                let h = max.y - min.y;
                // top
                q.push(DrawRect::new(min.x, min.y, w, t, color).with_z(Z));
                // bottom
                q.push(DrawRect::new(min.x, max.y - t, w, t, color).with_z(Z));
                // left
                q.push(DrawRect::new(min.x, min.y, t, h, color).with_z(Z));
                // right
                q.push(DrawRect::new(max.x - t, min.y, t, h, color).with_z(Z));
            }
            DebugShape::Line {
                start,
                end,
                color,
                thickness,
            } => {
                push_line(start, end, color, thickness);
            }
            DebugShape::Circle {
                center,
                radius,
                color,
            } => {
                let n = 24u32;
                for i in 0..n {
                    let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
                    let p0 = center + glam::Vec2::new(a0.cos(), a0.sin()) * radius;
                    let p1 = center + glam::Vec2::new(a1.cos(), a1.sin()) * radius;
                    push_line(p0, p1, color, 1.5);
                }
            }
            DebugShape::Cross { pos, size, color } => {
                let half = size / 2.0;
                push_line(
                    pos - glam::Vec2::X * half,
                    pos + glam::Vec2::X * half,
                    color,
                    1.5,
                );
                push_line(
                    pos - glam::Vec2::Y * half,
                    pos + glam::Vec2::Y * half,
                    color,
                    1.5,
                );
            }
        }
    }
}
