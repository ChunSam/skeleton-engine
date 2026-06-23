use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::DrawRect;
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::slider::Slider;

use super::capture::PointerCapture;
use super::state::{in_bounds, node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Slider>().map(|(e, _, _)| e));

    // A press starts a drag only when the slider owns the pointer (shared capture → a slider covered
    // by another widget kind doesn't grab the press through it). An in-progress drag keeps following
    // the cursor even off the track, as before.
    let pressed_owner = capture.topmost_at(input.press_cursor);
    let hover_owner = capture.topmost_at(input.cursor);

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };
        if !visible {
            continue;
        }

        let thumb_w = world.get::<Slider>(entity).map_or(14.0, |s| s.thumb_width);
        let track_len = (size.x - thumb_w).max(0.0);

        let just_pressed_hit = input.just_pressed && pressed_owner == Some(entity);

        if just_pressed_hit {
            if let Some(slider) = world.get_mut::<Slider>(entity) {
                let t = ((input.press_cursor.x - pos.x - thumb_w / 2.0)
                    / track_len.max(f32::EPSILON))
                .clamp(0.0, 1.0);
                slider.set_normalized(t);
                slider.dragging = true;
                let v = slider.value;
                output.events.push(UiEvent::SliderChanged(entity, v));
            }
        } else {
            // Only run the drag-update path when this is NOT the press frame, so we never
            // emit two SliderChanged events in the same frame (one from press, one from drag).
            let dragging = world.get::<Slider>(entity).is_some_and(|s| s.dragging);
            if dragging {
                if input.is_held {
                    if let Some(slider) = world.get_mut::<Slider>(entity) {
                        let t = ((input.cursor.x - pos.x - thumb_w / 2.0)
                            / track_len.max(f32::EPSILON))
                        .clamp(0.0, 1.0);
                        let new_val = slider.min + t * (slider.max - slider.min);
                        if (new_val - slider.value).abs() > f32::EPSILON {
                            slider.value = new_val;
                            let v = slider.value;
                            output.events.push(UiEvent::SliderChanged(entity, v));
                        }
                    }
                } else if let Some(slider) = world.get_mut::<Slider>(entity) {
                    slider.dragging = false;
                }
            }
        }

        let (norm, track_col, fill_col, thumb_col, thumb_hover_col) = {
            let s = match world.get::<Slider>(entity) {
                Some(s) => s,
                None => continue,
            };
            (
                s.normalized(),
                s.track_color,
                s.fill_color,
                s.thumb_color,
                s.thumb_hovered_color,
            )
        };

        let thumb_x = pos.x + norm * track_len;
        let thumb_hovered = hover_owner == Some(entity)
            && in_bounds(
                input.cursor,
                Vec2::new(thumb_x, pos.y),
                Vec2::new(thumb_w, size.y),
            );

        output
            .rects
            .push(DrawRect::new(pos.x, pos.y, size.x, size.y, track_col).with_z(z));
        output.rects.push(
            DrawRect::new(
                pos.x,
                pos.y,
                thumb_x - pos.x + thumb_w / 2.0,
                size.y,
                fill_col,
            )
            .with_z(z + 0.001),
        );
        let tc = if thumb_hovered {
            thumb_hover_col
        } else {
            thumb_col
        };
        output
            .rects
            .push(DrawRect::new(thumb_x, pos.y, thumb_w, size.y, tc).with_z(z + 0.002));
    }
}
