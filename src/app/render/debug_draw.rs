use super::super::App;
use crate::renderer::{DrawRect, UiQueue};

// Debug-draw visual policy. These were inline literals; promoted to named constants so the
// intent is documented in one place and a future `DebugDrawConfig` resource (if a fork needs
// runtime control) can lift them without hunting through the match arms. Behavior-preserving.
/// Z depth debug shapes draw at — above gameplay/UI so overlays stay visible.
const DEBUG_Z: f32 = 999.0;
/// Default stroke width (world px) for rect outlines, circles, and crosses.
const LINE_THICKNESS: f32 = 1.5;
/// Number of line segments approximating a debug circle.
const CIRCLE_SEGMENTS: u32 = 24;
/// Lower bound on the per-dot step size when filling a line (avoids div-by-tiny → huge step counts).
const MIN_STEP_THICKNESS: f32 = 0.5;
/// Segments shorter than this are skipped (degenerate, nothing visible to draw).
const MIN_SEGMENT_LEN: f32 = 0.001;

impl App {
    pub(in crate::app) fn debug_shape_to_draw_rects(
        shape: crate::resources::DebugShape,
        q: &mut UiQueue,
    ) {
        use crate::resources::DebugShape;
        const Z: f32 = DEBUG_Z;

        // Line-segment helper: one quad if the segment is axis-aligned (see below), otherwise a fill
        // of thickness×thickness dots approximating it.
        let mut push_line =
            |start: glam::Vec2, end: glam::Vec2, color: crate::color::Color, thickness: f32| {
                let delta = end - start;
                let len = delta.length();
                if len < MIN_SEGMENT_LEN {
                    return;
                }
                let half = thickness / 2.0;

                // An axis-aligned segment is exactly one quad, so draw one. The dotted fill below
                // steps by `len / ceil(len / thickness)`, which is `<= thickness`, so consecutive
                // dots always touch or overlap and their union *is* this rect — an identity, not an
                // approximation. It matters: a 410 px guide column went from 275 quads to 1, and a
                // cross from 30 to 2. Two conditions are load-bearing:
                //   - exactly on-axis. One float off and the union is a staircase, not a rect.
                //   - `thickness >= MIN_STEP_THICKNESS`. Below it the step is clamped while the
                //     dots stay `thickness` wide, so they stop touching and the line is *meant* to
                //     read as dotted. `DebugDraw::line_thick` can ask for that.
                // Diagonals and circle chords cannot follow either way: `DrawRect` has no rotation.
                if thickness >= MIN_STEP_THICKNESS && (delta.x == 0.0 || delta.y == 0.0) {
                    let min = start.min(end);
                    q.push(
                        DrawRect::new(
                            min.x - half,
                            min.y - half,
                            delta.x.abs() + thickness,
                            delta.y.abs() + thickness,
                            color,
                        )
                        .with_z(Z),
                    );
                    return;
                }

                let steps = (len / thickness.max(MIN_STEP_THICKNESS)).ceil() as usize;
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
                let t = LINE_THICKNESS;
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
                let n = CIRCLE_SEGMENTS;
                for i in 0..n {
                    let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
                    let p0 = center + glam::Vec2::new(a0.cos(), a0.sin()) * radius;
                    let p1 = center + glam::Vec2::new(a1.cos(), a1.sin()) * radius;
                    push_line(p0, p1, color, LINE_THICKNESS);
                }
            }
            DebugShape::Cross { pos, size, color } => {
                let half = size / 2.0;
                push_line(
                    pos - glam::Vec2::X * half,
                    pos + glam::Vec2::X * half,
                    color,
                    LINE_THICKNESS,
                );
                push_line(
                    pos - glam::Vec2::Y * half,
                    pos + glam::Vec2::Y * half,
                    color,
                    LINE_THICKNESS,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;
    use crate::resources::DebugShape;
    use glam::Vec2;

    const C: Color = Color {
        r: 1.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };

    fn quads(shape: DebugShape) -> Vec<DrawRect> {
        let mut q = UiQueue::default();
        App::debug_shape_to_draw_rects(shape, &mut q);
        q.items
    }

    fn line(start: Vec2, end: Vec2, thickness: f32) -> Vec<DrawRect> {
        quads(DebugShape::Line {
            start,
            end,
            color: C,
            thickness,
        })
    }

    /// The union of the dotted approximation, as an (x, y, w, h) tuple — what a collapsed
    /// axis-aligned segment must cover exactly.
    fn bounds(items: &[DrawRect]) -> (f32, f32, f32, f32) {
        let x0 = items.iter().fold(f32::MAX, |a, r| a.min(r.x));
        let y0 = items.iter().fold(f32::MAX, |a, r| a.min(r.y));
        let x1 = items.iter().fold(f32::MIN, |a, r| a.max(r.x + r.w));
        let y1 = items.iter().fold(f32::MIN, |a, r| a.max(r.y + r.h));
        (x0, y0, x1 - x0, y1 - y0)
    }

    // --- the collapse -----------------------------------------------------------------
    //
    // `push_line` steps by `len / ceil(len / thickness)`, which is always `<= thickness`, so the
    // dots of an axis-aligned segment always touch or overlap and their union is *exactly* one
    // rect. That is an identity, not an approximation, so one quad may replace N.

    #[test]
    fn vertical_line_is_one_quad_covering_the_whole_segment() {
        // The `centered_text` guide column: 410 px at the default 1.5 thickness used to be 275
        // quads. Nothing about a straight line needs more than one.
        let items = line(Vec2::new(120.0, 70.0), Vec2::new(120.0, 480.0), 1.5);
        assert_eq!(items.len(), 1, "a vertical segment is one quad");
        let r = items[0];
        assert_eq!((r.x, r.y, r.w, r.h), (119.25, 69.25, 1.5, 411.5));
        assert_eq!(r.z, DEBUG_Z);
    }

    #[test]
    fn horizontal_line_is_one_quad_covering_the_whole_segment() {
        let items = line(Vec2::new(10.0, 50.0), Vec2::new(210.0, 50.0), 4.0);
        assert_eq!(items.len(), 1);
        let r = items[0];
        assert_eq!((r.x, r.y, r.w, r.h), (8.0, 48.0, 204.0, 4.0));
    }

    #[test]
    fn a_reversed_segment_collapses_to_the_same_quad() {
        let fwd = line(Vec2::new(0.0, 0.0), Vec2::new(0.0, 100.0), 2.0);
        let rev = line(Vec2::new(0.0, 100.0), Vec2::new(0.0, 0.0), 2.0);
        assert_eq!(rev.len(), 1);
        assert_eq!(
            (fwd[0].x, fwd[0].y, fwd[0].w, fwd[0].h),
            (rev[0].x, rev[0].y, rev[0].w, rev[0].h),
            "direction must not change the covered area"
        );
    }

    /// The collapsed quad covers exactly what the dots covered — computed from the stepping rule
    /// rather than restated from the implementation.
    #[test]
    fn the_collapsed_quad_matches_the_dotted_union_it_replaces() {
        let (start, end, t) = (Vec2::new(31.0, 12.0), Vec2::new(31.0, 419.0), 1.5);
        let collapsed = line(start, end, t);
        assert_eq!(collapsed.len(), 1);

        // Re-derive the pre-collapse dots: `steps + 1` squares of side `t` centred along the
        // segment. Their union is the claim under test.
        let len = (end - start).length();
        let steps = (len / t.max(MIN_STEP_THICKNESS)).ceil() as usize;
        let dots: Vec<DrawRect> = (0..=steps)
            .map(|i| {
                let p = start + (end - start) * (i as f32 / steps.max(1) as f32);
                DrawRect::new(p.x - t / 2.0, p.y - t / 2.0, t, t, C)
            })
            .collect();
        assert!(dots.len() > 100, "control: the old path really was N quads");

        let (bx, by, bw, bh) = bounds(&dots);
        let r = collapsed[0];
        for (got, want, what) in [
            (r.x, bx, "x"),
            (r.y, by, "y"),
            (r.w, bw, "w"),
            (r.h, bh, "h"),
        ] {
            assert!(
                (got - want).abs() < 1e-3,
                "{what}: collapsed {got} vs dotted union {want}"
            );
        }
    }

    // --- what does *not* collapse (the positive controls) ------------------------------
    //
    // Without a rotation field on `DrawRect` a diagonal cannot be one quad. These pin that, so a
    // green collapse test above is not green because the shape produced nothing.

    #[test]
    fn a_diagonal_line_still_steps() {
        let diag = line(Vec2::new(0.0, 0.0), Vec2::new(300.0, 300.0), 1.5);
        assert!(
            diag.len() > 100,
            "a diagonal has no axis-aligned collapse (DrawRect has no rotation): {}",
            diag.len()
        );
    }

    #[test]
    fn a_near_axis_aligned_line_is_not_collapsed() {
        // The collapse is exact-zero only. One float off axis and it must fall back to stepping,
        // because the union of the dots is no longer a rect.
        let items = line(Vec2::new(0.0, 0.0), Vec2::new(0.01, 300.0), 1.5);
        assert!(items.len() > 100, "got {}", items.len());
    }

    #[test]
    fn a_hairline_axis_aligned_segment_stays_dotted() {
        // Below `MIN_STEP_THICKNESS` the step is clamped to 0.5 while the dots stay `thickness`
        // wide, so they no longer touch — the line really is a dotted line, and collapsing it to a
        // solid rect would be a visual change, not an optimisation. `line_thick` is public, so a
        // fork can ask for this.
        let items = line(Vec2::new(0.0, 0.0), Vec2::new(0.0, 100.0), 0.2);
        assert!(
            items.len() > 100,
            "a sub-step-width segment is genuinely dotted: {}",
            items.len()
        );
        // ...and the gap between dots is real, which is the whole reason it must not collapse.
        assert!(items[1].y - items[0].y > items[0].h);
    }

    #[test]
    fn a_circle_is_still_one_quad_run_per_segment() {
        // 24 chords, none of them axis-aligned in general — a circle needs rotation to shrink.
        // Pinned so the cost is visible: this is the shape that is still expensive.
        let items = quads(DebugShape::Circle {
            center: Vec2::ZERO,
            radius: 24.0,
            color: C,
        });
        assert_eq!(items.len(), 144, "24 chords x 6 dots at radius 24");
    }

    // --- the shapes built out of segments ----------------------------------------------

    #[test]
    fn a_cross_is_two_quads() {
        let items = quads(DebugShape::Cross {
            pos: Vec2::new(50.0, 50.0),
            size: 20.0,
            color: C,
        });
        assert_eq!(items.len(), 2, "a cross is two axis-aligned segments");
    }

    #[test]
    fn a_rect_outline_is_four_quads() {
        // Already hand-written as four quads; pinned so the collapse cannot regress it.
        let items = quads(DebugShape::Rect {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(100.0, 60.0),
            color: C,
        });
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn a_degenerate_segment_draws_nothing() {
        assert!(line(Vec2::new(5.0, 5.0), Vec2::new(5.0, 5.0), 1.5).is_empty());
    }
}
