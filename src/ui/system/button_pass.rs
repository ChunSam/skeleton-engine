use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText, TextAlign};
use crate::resources::ViewportSize;
use crate::ui::button::{Button, ButtonState};
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
    scratch.extend(world.query2::<UiNode, Button>().map(|(e, _, _)| e));
    let button_entities = &*scratch;

    // The shared pointer-capture set decides which widget owns each point across *all* widget kinds
    // (so a button covered by a panel or another widget no longer fires). A button is hovered /
    // pressed / clicked only while it is the topmost pointer-opaque surface under the cursor. Because
    // `topmost_at` already resolves z-order, at most one button can satisfy the click — no separate
    // candidate-resolution pass is needed.
    let hover_owner = capture.topmost_at(input.cursor);
    let pressed_owner = capture.topmost_at(input.press_cursor);
    let released_owner = capture.topmost_at(input.release_cursor);

    for entity in button_entities.iter().copied() {
        let hover = hover_owner == Some(entity);
        let clicked =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);

        let btn = match world.get_mut::<Button>(entity) {
            Some(b) => b,
            None => continue,
        };
        if btn.state != ButtonState::Disabled {
            btn.state = if hover {
                if input.is_held {
                    ButtonState::Pressed
                } else {
                    ButtonState::Hovered
                }
            } else {
                ButtonState::Normal
            };
            if clicked {
                output.events.push(UiEvent::ButtonClicked(entity));
            }
        }
    }

    // Second pass: render each button (borrow immutably now that state mutations are done).
    for entity in button_entities.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };
        if !visible {
            continue;
        }

        let (color, label_text, text_color, font_size) = {
            let btn = match world.get::<Button>(entity) {
                Some(b) => b,
                None => continue,
            };
            (
                btn.current_color(),
                btn.label.clone(),
                btn.text_color,
                btn.font_size,
            )
        };

        output
            .rects
            .push(DrawRect::new(pos.x, pos.y, size.x, size.y, color).with_z(z));

        if !label_text.is_empty() {
            let text_y = pos.y + (size.y - font_size) / 2.0;
            output.texts.push(
                DrawText::new(label_text, Vec2::new(pos.x, text_y), font_size, text_color)
                    .with_bounds(Vec2::new(size.x, size.y))
                    .with_align(TextAlign::Center)
                    .with_z(z),
            );
        }
    }
}
