use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText, TextAlign};
use crate::resources::ViewportSize;
use crate::ui::button::{Button, ButtonState};
use crate::ui::node::UiNode;

use super::state::{in_bounds, node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Button>().map(|(e, _, _)| e));
    let button_entities = &*scratch;

    // Collect click candidates: (entity, z) for buttons that pass the hit-test this frame.
    // Only the topmost (greatest z) candidate fires ButtonClicked; visual state updates
    // still happen for every button regardless.
    // TODO: a button beneath a *different* widget type (e.g. a Panel) can still fire here;
    // shared pointer-consumption across widget kinds is a broader concern left for a future pass.
    let mut click_candidate: Option<(Entity, f32)> = None;

    for entity in button_entities.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
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
            // Register as a click candidate; the winner is resolved after the loop.
            if clicked {
                let is_better = match click_candidate {
                    None => true,
                    Some((_, best_z)) => z > best_z,
                };
                if is_better {
                    click_candidate = Some((entity, z));
                }
            }
        }
    }

    // Fire ButtonClicked for the single topmost candidate (if any).
    if let Some((winner, _)) = click_candidate {
        output.events.push(UiEvent::ButtonClicked(winner));
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
                    .with_align(TextAlign::Center),
            );
        }
    }
}
