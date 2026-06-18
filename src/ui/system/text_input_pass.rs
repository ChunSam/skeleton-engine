use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::text_input::TextInput;

use super::state::{in_bounds, node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    dt: f32,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, TextInput>().map(|(e, _, _)| e));

    // Determine which TextInput (if any) should receive focus on this press.
    // Collect all in-bounds candidates with their z values, then pick the
    // topmost (greatest z) — mirroring the button_pass z-order pattern.
    // Invisible widgets are excluded so they cannot steal focus.
    let mut newly_focused: Option<Entity> = None;
    if input.just_pressed {
        let mut best: Option<(Entity, f32)> = None;
        for &entity in scratch.iter() {
            let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
                Some(layout) => layout,
                None => continue,
            };
            if !visible {
                continue;
            }
            if in_bounds(input.press_cursor, pos, size) {
                let is_better = match best {
                    None => true,
                    Some((_, best_z)) => z > best_z,
                };
                if is_better {
                    best = Some((entity, z));
                }
            }
        }
        newly_focused = best.map(|(e, _)| e);
    }

    for &entity in scratch.iter() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };

        // Always clear focus on invisible widgets so they cannot hold or receive focus.
        if !visible {
            if let Some(ti) = world.get_mut::<TextInput>(entity) {
                if ti.focused {
                    ti.focused = false;
                    output.events.push(UiEvent::TextBlurred(entity));
                }
            }
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

                // Hoist get_mut outside the per-character loop; collect emitted events
                // in a temporary vec so we can push them to `output` after the borrow ends.
                let mut char_events: Vec<UiEvent> = Vec::new();
                if let Some(ti) = world.get_mut::<TextInput>(entity) {
                    for &c in &input.chars {
                        match c {
                            '\x08' => {
                                ti.backspace();
                                let text = ti.text.clone();
                                char_events.push(UiEvent::TextChanged(entity, text));
                            }
                            '\n' => {
                                let text = ti.text.clone();
                                ti.focused = false;
                                char_events.push(UiEvent::TextSubmitted(entity, text));
                                char_events.push(UiEvent::TextBlurred(entity));
                            }
                            ch => {
                                ti.insert_char(ch);
                                let text = ti.text.clone();
                                char_events.push(UiEvent::TextChanged(entity, text));
                            }
                        }
                    }
                }
                output.events.extend(char_events);
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

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::ecs::World;
    use crate::resources::ViewportSize;
    use crate::ui::node::UiNode;
    use crate::ui::text_input::TextInput;

    use super::super::state::{InputSnapshot, UiOutput};
    use super::super::UiEvent;

    /// Builds a minimal `InputSnapshot` representing a left-mouse press at `press_pos`.
    fn press_at(press_pos: Vec2) -> InputSnapshot {
        InputSnapshot {
            cursor: press_pos,
            just_pressed: true,
            just_released: false,
            is_held: true,
            scroll_delta: 0.0,
            chars: vec![],
            ime_preedit: String::new(),
            press_cursor: press_pos,
            release_cursor: Vec2::ZERO,
            nav_left: false,
            nav_right: false,
            nav_home: false,
            nav_end: false,
            nav_delete: false,
            tab: false,
            shift: false,
            activate: false,
        }
    }

    /// Builds a minimal `InputSnapshot` with no press events.
    fn no_press() -> InputSnapshot {
        InputSnapshot {
            cursor: Vec2::ZERO,
            just_pressed: false,
            just_released: false,
            is_held: false,
            scroll_delta: 0.0,
            chars: vec![],
            ime_preedit: String::new(),
            press_cursor: Vec2::ZERO,
            release_cursor: Vec2::ZERO,
            nav_left: false,
            nav_right: false,
            nav_home: false,
            nav_end: false,
            nav_delete: false,
            tab: false,
            shift: false,
            activate: false,
        }
    }

    fn viewport() -> ViewportSize {
        ViewportSize {
            width: 800.0,
            height: 600.0,
        }
    }

    /// Spawns a TextInput at the given screen position/size and z level.
    /// The UiNode uses TopLeft anchor so `offset` == screen position directly.
    fn spawn_text_input(
        world: &mut World,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        z: f32,
    ) -> crate::ecs::Entity {
        let e = world.spawn();
        let mut node = UiNode::new(x, y, w, h).with_z(z);
        node.visible = true;
        world.add_component(e, node);
        world.add_component(e, TextInput::new(""));
        e
    }

    /// Two overlapping TextInputs: the one with higher z should receive focus.
    #[test]
    fn topmost_z_receives_focus_on_click() {
        let mut world = World::new();
        // Both widgets occupy the same area (0,0)→(100,30).
        let low_z = spawn_text_input(&mut world, 0.0, 0.0, 100.0, 30.0, 0.5);
        let high_z = spawn_text_input(&mut world, 0.0, 0.0, 100.0, 30.0, 0.9);

        let vp = viewport();
        let input = press_at(Vec2::new(50.0, 15.0)); // inside both widgets
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(&mut world, &vp, &input, 0.016, &mut output, &mut scratch);

        assert!(
            world.get::<TextInput>(high_z).unwrap().focused,
            "higher-z widget should be focused"
        );
        assert!(
            !world.get::<TextInput>(low_z).unwrap().focused,
            "lower-z widget should not be focused"
        );

        // TextFocused event should be emitted for the winner only.
        let focused_events: Vec<_> = output
            .events
            .iter()
            .filter_map(|e| {
                if let UiEvent::TextFocused(entity) = e {
                    Some(*entity)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(focused_events, vec![high_z]);
    }

    /// Clicking the position of a `visible=false` TextInput must not focus it.
    #[test]
    fn invisible_text_input_does_not_receive_focus() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 100.0, 30.0, 0.9);
        // Hide the widget.
        world.get_mut::<UiNode>(e).unwrap().visible = false;

        let vp = viewport();
        let input = press_at(Vec2::new(50.0, 15.0)); // inside widget bounds
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(&mut world, &vp, &input, 0.016, &mut output, &mut scratch);

        assert!(
            !world.get::<TextInput>(e).unwrap().focused,
            "invisible widget must not receive focus"
        );
        // No TextFocused event should be emitted.
        let focused_any = output
            .events
            .iter()
            .any(|e| matches!(e, UiEvent::TextFocused(_)));
        assert!(
            !focused_any,
            "no TextFocused event expected for invisible widget"
        );
    }

    /// A focused TextInput that becomes invisible should have focus cleared.
    #[test]
    fn focus_cleared_when_widget_becomes_invisible() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 100.0, 30.0, 0.9);
        // Manually give it focus.
        world.get_mut::<TextInput>(e).unwrap().focused = true;
        // Now hide it.
        world.get_mut::<UiNode>(e).unwrap().visible = false;

        let vp = viewport();
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(
            &mut world,
            &vp,
            &no_press(),
            0.016,
            &mut output,
            &mut scratch,
        );

        assert!(
            !world.get::<TextInput>(e).unwrap().focused,
            "focus must be cleared when the widget becomes invisible"
        );
        // TextBlurred should be emitted.
        let blurred = output
            .events
            .iter()
            .any(|e| matches!(e, UiEvent::TextBlurred(_)));
        assert!(
            blurred,
            "TextBlurred event expected when focus is cleared by visibility change"
        );
    }
}
