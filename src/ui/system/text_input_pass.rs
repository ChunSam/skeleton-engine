use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::text_input::TextInput;

use super::state::{in_bounds, InputSnapshot, UiOutput};
use super::UiEvent;

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    dt: f32,
    output: &mut UiOutput,
) {
    let text_input_entities: Vec<Entity> = world
        .query2::<UiNode, TextInput>()
        .map(|(e, _, _)| e)
        .collect();

    let mut newly_focused: Option<Entity> = None;
    if input.just_pressed {
        for &entity in &text_input_entities {
            let (pos, size) = match world.get::<UiNode>(entity) {
                Some(n) => (n.screen_pos(viewport), n.size),
                None => continue,
            };
            if in_bounds(input.press_cursor, pos, size) {
                newly_focused = Some(entity);
                break;
            }
        }
    }

    for &entity in &text_input_entities {
        let (pos, size, z, visible) = match world.get::<UiNode>(entity) {
            Some(n) => (n.screen_pos(viewport), n.size, n.z, n.visible),
            None => continue,
        };
        if !visible {
            continue;
        }

        if input.just_pressed {
            let ti = match world.get_mut::<TextInput>(entity) {
                Some(t) => t,
                None => continue,
            };
            let was_focused = ti.focused;
            ti.focused = newly_focused == Some(entity);
            if !was_focused && ti.focused {
                output.events.push(UiEvent::TextFocused(entity));
                ti.cursor_blink = 0.0;
                ti.cursor_visible = true;
            } else if was_focused && !ti.focused {
                output.events.push(UiEvent::TextBlurred(entity));
            }
        }

        {
            let focused = world.get::<TextInput>(entity).is_some_and(|t| t.focused);
            if focused {
                if let Some(ti) = world.get_mut::<TextInput>(entity) {
                    ti.cursor_blink += dt;
                    if ti.cursor_blink >= 0.5 {
                        ti.cursor_blink -= 0.5;
                        ti.cursor_visible = !ti.cursor_visible;
                    }
                }

                if let Some(ti) = world.get_mut::<TextInput>(entity) {
                    if input.nav_left {
                        ti.move_left();
                    }
                    if input.nav_right {
                        ti.move_right();
                    }
                    if input.nav_home {
                        ti.move_home();
                    }
                    if input.nav_end {
                        ti.move_end();
                    }
                    if input.nav_delete {
                        ti.delete_forward();
                    }
                }

                for &c in &input.chars {
                    match c {
                        '\x08' => {
                            if let Some(ti) = world.get_mut::<TextInput>(entity) {
                                ti.backspace();
                                let text = ti.text.clone();
                                output.events.push(UiEvent::TextChanged(entity, text));
                            }
                        }
                        '\n' => {
                            if let Some(ti) = world.get_mut::<TextInput>(entity) {
                                let text = ti.text.clone();
                                ti.focused = false;
                                output.events.push(UiEvent::TextSubmitted(entity, text));
                                output.events.push(UiEvent::TextBlurred(entity));
                            }
                        }
                        ch => {
                            if let Some(ti) = world.get_mut::<TextInput>(entity) {
                                ti.insert_char(ch);
                                let text = ti.text.clone();
                                output.events.push(UiEvent::TextChanged(entity, text));
                            }
                        }
                    }
                }
                if let Some(ti) = world.get_mut::<TextInput>(entity) {
                    ti.preedit = if ti.remaining_capacity() >= input.ime_preedit.len() {
                        input.ime_preedit.clone()
                    } else {
                        String::new()
                    };
                }
            }
        }

        let (bg_color, display_text, text_color, font_size, caret_byte) = {
            let ti = match world.get::<TextInput>(entity) {
                Some(t) => t,
                None => continue,
            };
            let display = ti.display_with_caret(ti.focused, ti.cursor_visible);
            let caret_byte = if ti.focused {
                ti.caret_display_offset()
            } else {
                0
            };
            (
                ti.current_color(),
                display,
                ti.text_color,
                ti.font_size,
                caret_byte,
            )
        };

        output
            .rects
            .push(DrawRect::new(pos.x, pos.y, size.x, size.y, bg_color).with_z(z));
        output.texts.push(
            DrawText::new(
                display_text,
                Vec2::new(pos.x + 6.0, pos.y + (size.y - font_size) / 2.0),
                font_size,
                text_color,
            )
            .with_bounds(Vec2::new((size.x - 12.0).max(0.0), size.y))
            .with_single_line_caret(caret_byte),
        );
    }
}
