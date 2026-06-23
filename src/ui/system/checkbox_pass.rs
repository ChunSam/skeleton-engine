use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::checkbox::CheckBox;
use crate::ui::node::UiNode;

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
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
    scratch.extend(world.query2::<UiNode, CheckBox>().map(|(e, _, _)| e));

    // Toggle only when this checkbox owns the pointer at both press and release (shared capture →
    // a checkbox covered by another widget kind does not toggle through it).
    let pressed_owner = capture.topmost_at(input.press_cursor);
    let released_owner = capture.topmost_at(input.release_cursor);

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };
        if !visible {
            continue;
        }

        // Toggle on release, just like a Button (only when both press and release land on this box).
        // Dragging onto another widget before releasing cancels the toggle.
        let toggled =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);
        if toggled {
            if let Some(cb) = world.get_mut::<CheckBox>(entity) {
                cb.checked = !cb.checked;
                let checked = cb.checked;
                output
                    .events
                    .push(UiEvent::CheckBoxToggled(entity, checked));
            }
        }

        let (
            checked,
            box_size,
            border_col,
            checked_col,
            unchecked_col,
            label,
            text_color,
            font_size,
        ) = {
            let cb = match world.get::<CheckBox>(entity) {
                Some(c) => c,
                None => continue,
            };
            (
                cb.checked,
                cb.box_size,
                cb.border_color,
                cb.checked_color,
                cb.unchecked_color,
                cb.label.clone(),
                cb.text_color,
                cb.font_size,
            )
        };

        let box_y = pos.y + (size.y - box_size) / 2.0;
        let pad = 2.0;
        output
            .rects
            .push(DrawRect::new(pos.x, box_y, box_size, box_size, border_col).with_z(z));
        let inner_col = if checked { checked_col } else { unchecked_col };
        output.rects.push(
            DrawRect::new(
                pos.x + pad,
                box_y + pad,
                box_size - pad * 2.0,
                box_size - pad * 2.0,
                inner_col,
            )
            .with_z(z + 0.001),
        );
        if !label.is_empty() {
            output.texts.push(
                DrawText::new(
                    label,
                    Vec2::new(pos.x + box_size + 6.0, pos.y + (size.y - font_size) / 2.0),
                    font_size,
                    text_color,
                )
                .with_bounds(Vec2::new((size.x - box_size - 6.0).max(0.0), size.y)),
            );
        }
    }
}
