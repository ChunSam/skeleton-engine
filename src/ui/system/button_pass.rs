use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText, TextAlign};
use crate::resources::ViewportSize;
use crate::ui::button::{Button, ButtonState};
use crate::ui::node::UiNode;

use super::state::{in_bounds, InputSnapshot, UiOutput};
use super::UiEvent;

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    output: &mut UiOutput,
) {
    let button_entities: Vec<Entity> = world
        .query2::<UiNode, Button>()
        .map(|(e, _, _)| e)
        .collect();

    for entity in button_entities {
        let (pos, size, z, visible) = match world.get::<UiNode>(entity) {
            Some(node) => (node.screen_pos(viewport), node.size, node.z, node.visible),
            None => continue,
        };
        if !visible {
            continue;
        }

        let hover = in_bounds(input.cursor, pos, size);
        let clicked = input.just_released
            && in_bounds(input.press_cursor, pos, size)
            && in_bounds(input.release_cursor, pos, size);

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

        let (color, label_text, text_color, font_size) = {
            let btn = world.get::<Button>(entity).unwrap();
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
                    .with_align(TextAlign::Center),
            );
        }
    }
}
