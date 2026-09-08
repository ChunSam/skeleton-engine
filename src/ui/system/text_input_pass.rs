use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::focus::UiFocus;
use crate::ui::node::UiNode;
use crate::ui::text_input::TextInput;

use super::state::{node_layout, InputSnapshot, UiOutput};
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

    for &entity in scratch.iter() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };

        if !visible {
            continue;
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
                let mut submitted = false;
                if let Some(ti) = world.get_mut::<TextInput>(entity) {
                    for &c in &input.chars {
                        match c {
                            '\x08' => {
                                // Emit-on-change, like every sibling widget: a backspace at
                                // cursor 0 deletes nothing. A real edit always moves the byte
                                // length, so comparing it needs no second clone of the text.
                                let before = ti.text.len();
                                ti.backspace();
                                if ti.text.len() != before {
                                    let text = ti.text.clone();
                                    char_events.push(UiEvent::TextChanged(entity, text));
                                }
                            }
                            '\n' => {
                                let text = ti.text.clone();
                                ti.focused = false;
                                submitted = true;
                                char_events.push(UiEvent::TextSubmitted(entity, text));
                                char_events.push(UiEvent::TextBlurred(entity));
                                // Stop at the first Enter. `InputState`'s char buffer accumulates
                                // over a whole frame and `dt` is capped at `max_dt`, so a stalled
                                // frame can carry a double-tap: without this, one blur emitted two
                                // TextSubmitted + two TextBlurred, and every char after the '\n'
                                // was inserted into the field it had just unfocused.
                                break;
                            }
                            ch => {
                                let before = ti.text.len();
                                ti.insert_char(ch);
                                if ti.text.len() != before {
                                    let text = ti.text.clone();
                                    char_events.push(UiEvent::TextChanged(entity, text));
                                }
                            }
                        }
                    }
                }
                output.events.extend(char_events);
                // Enter both submits and blurs: clear the shared `UiFocus` so the focus pass
                // (the single owner of `ti.focused`) doesn't re-focus this field next frame.
                if submitted {
                    if let Some(focus) = world.resource_mut::<UiFocus>() {
                        if focus.entity == Some(entity) {
                            focus.entity = None;
                        }
                    }
                }
            }

            // Outside the `focused` branch, and reading `focused` fresh: the refresh used to run
            // only while focused, so Tab/click away mid-composition left the stale preedit on the
            // component forever and `display_with_caret` drew it instead of the placeholder.
            // Reading it fresh also covers the Enter above, which blurs within this same frame.
            if let Some(ti) = world.get_mut::<TextInput>(entity) {
                ti.preedit = if ti.focused && ti.remaining_capacity() >= input.ime_preedit.len() {
                    input.ime_preedit.clone()
                } else {
                    String::new()
                };
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
            .with_single_line_caret(caret_byte)
            .with_z(z),
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

    fn viewport() -> ViewportSize {
        ViewportSize {
            width: 800.0,
            height: 600.0,
        }
    }

    /// Builds a minimal `InputSnapshot` with no press events but with typed characters.
    fn chars_input(chars: Vec<char>) -> InputSnapshot {
        InputSnapshot {
            cursor: Vec2::ZERO,
            cursor_inside: true,
            just_pressed: false,
            just_released: false,
            is_held: false,
            scroll_delta: 0.0,
            chars,
            ime_preedit: String::new(),
            press_cursor: Vec2::ZERO,
            release_cursor: Vec2::ZERO,
            nav_left: false,
            nav_right: false,
            nav_up: false,
            nav_down: false,
            nav_home: false,
            nav_end: false,
            nav_delete: false,
            tab: false,
            shift: false,
            activate: false,
        }
    }

    fn nav_input(left: bool, right: bool) -> InputSnapshot {
        InputSnapshot {
            cursor: Vec2::ZERO,
            cursor_inside: true,
            just_pressed: false,
            just_released: false,
            is_held: false,
            scroll_delta: 0.0,
            chars: vec![],
            ime_preedit: String::new(),
            press_cursor: Vec2::ZERO,
            release_cursor: Vec2::ZERO,
            nav_left: left,
            nav_right: right,
            nav_up: false,
            nav_down: false,
            nav_home: false,
            nav_end: false,
            nav_delete: false,
            tab: false,
            shift: false,
            activate: false,
        }
    }

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

    /// A pre-focused TextInput receives typed characters and emits TextChanged.
    #[test]
    fn focused_text_input_receives_chars() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        world.get_mut::<TextInput>(e).unwrap().focused = true;

        let vp = viewport();
        let input = chars_input(vec!['h', 'i']);
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(&mut world, &vp, &input, 0.016, &mut output, &mut scratch);

        assert_eq!(
            world.get::<TextInput>(e).unwrap().text,
            "hi",
            "typed chars should be inserted into the focused TextInput"
        );
        let changed: Vec<_> = output
            .events
            .iter()
            .filter(|ev| matches!(ev, UiEvent::TextChanged(_, _)))
            .collect();
        assert_eq!(changed.len(), 2, "one TextChanged per char");
    }

    /// Enter on a focused TextInput emits TextSubmitted + TextBlurred and clears focus.
    #[test]
    fn enter_submits_and_blurs_focused_text_input() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        {
            let ti = world.get_mut::<TextInput>(e).unwrap();
            ti.focused = true;
            ti.insert_str("hello");
        }

        let vp = viewport();
        let input = chars_input(vec!['\n']);
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(&mut world, &vp, &input, 0.016, &mut output, &mut scratch);

        assert!(
            !world.get::<TextInput>(e).unwrap().focused,
            "focus should be cleared after Enter"
        );
        assert!(
            output
                .events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextSubmitted(_, _))),
            "TextSubmitted should be emitted"
        );
        assert!(
            output
                .events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextBlurred(_))),
            "TextBlurred should be emitted after Enter"
        );
    }

    /// Cursor blink advances each frame while focused.
    #[test]
    fn cursor_blink_advances_while_focused() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        world.get_mut::<TextInput>(e).unwrap().focused = true;

        let vp = viewport();
        let input = chars_input(vec![]);
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        // dt=0.3 — blink not yet toggled (< 0.5)
        super::run(&mut world, &vp, &input, 0.3, &mut output, &mut scratch);
        let blink_after_first = world.get::<TextInput>(e).unwrap().cursor_blink;
        assert!(
            (blink_after_first - 0.3).abs() < 1e-5,
            "cursor_blink should be ~0.3 after first frame"
        );

        // dt=0.3 — blink passes 0.5, wraps around
        super::run(&mut world, &vp, &input, 0.3, &mut output, &mut scratch);
        let blink_after_second = world.get::<TextInput>(e).unwrap().cursor_blink;
        assert!(
            blink_after_second < 0.5,
            "cursor_blink should have wrapped after 0.6s total, got {blink_after_second}"
        );
    }

    /// Nav keys move the cursor within a focused TextInput.
    #[test]
    fn nav_keys_move_cursor_when_focused() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        {
            let ti = world.get_mut::<TextInput>(e).unwrap();
            ti.focused = true;
            ti.insert_str("abc"); // cursor at 3
        }

        let vp = viewport();
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(
            &mut world,
            &vp,
            &nav_input(true, false),
            0.016,
            &mut output,
            &mut scratch,
        );
        assert_eq!(
            world.get::<TextInput>(e).unwrap().cursor,
            2,
            "ArrowLeft should move cursor left"
        );
    }

    /// An unfocused TextInput (focused=false) does not receive typed characters.
    #[test]
    fn unfocused_text_input_ignores_chars() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        // focused = false (default)

        let vp = viewport();
        let input = chars_input(vec!['a', 'b', 'c']);
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(&mut world, &vp, &input, 0.016, &mut output, &mut scratch);

        assert!(
            world.get::<TextInput>(e).unwrap().text.is_empty(),
            "unfocused TextInput must not receive chars"
        );
    }

    /// Two Enters arriving in one frame are still **one** blur. `InputState`'s char buffer
    /// accumulates over a whole frame and `dt` is capped at 0.1 s, so a stalled frame absorbs a
    /// double-tap; the per-char loop has to stop at the first `'\n'` or the field submits twice
    /// and every later char is inserted into the now-unfocused field.
    #[test]
    fn a_second_enter_in_the_same_frame_does_not_submit_again() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        {
            let ti = world.get_mut::<TextInput>(e).unwrap();
            ti.focused = true;
            ti.insert_str("hello");
        }

        let vp = viewport();
        let input = chars_input(vec!['\n', '\n', 'x']);
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(&mut world, &vp, &input, 0.016, &mut output, &mut scratch);

        let submitted = output
            .events
            .iter()
            .filter(|ev| matches!(ev, UiEvent::TextSubmitted(_, _)))
            .count();
        let blurred = output
            .events
            .iter()
            .filter(|ev| matches!(ev, UiEvent::TextBlurred(_)))
            .count();
        assert_eq!(submitted, 1, "one blur is one TextSubmitted");
        assert_eq!(blurred, 1, "one blur is one TextBlurred");
        assert_eq!(
            world.get::<TextInput>(e).unwrap().text,
            "hello",
            "a char after the Enter must not reach the blurred field"
        );
    }

    /// Blurring mid-composition drops the IME preedit. The refresh used to sit inside the
    /// `if focused` block, so Tab/click away left the stale string on the component and
    /// `display_with_caret` rendered it instead of the placeholder — forever.
    #[test]
    fn blurring_clears_a_stale_ime_preedit() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        {
            let ti = world.get_mut::<TextInput>(e).unwrap();
            ti.focused = true;
            ti.placeholder = "type here".to_string();
        }

        let vp = viewport();
        let mut composing = chars_input(vec![]);
        composing.ime_preedit = "한".to_string();
        let mut output = UiOutput::default();
        let mut scratch = Vec::new();

        super::run(
            &mut world,
            &vp,
            &composing,
            0.016,
            &mut output,
            &mut scratch,
        );
        assert_eq!(
            world.get::<TextInput>(e).unwrap().preedit,
            "한",
            "the focused field takes the preedit"
        );

        // Focus moves away (the focus pass owns `focused`) and the composition ends with it.
        world.get_mut::<TextInput>(e).unwrap().focused = false;
        let mut output = UiOutput::default();
        super::run(
            &mut world,
            &vp,
            &chars_input(vec![]),
            0.016,
            &mut output,
            &mut scratch,
        );

        assert_eq!(
            world.get::<TextInput>(e).unwrap().preedit,
            "",
            "a blurred field must not keep its preedit"
        );
        assert!(
            output.texts.iter().any(|t| t.text == "type here"),
            "the placeholder is drawn again, not the stale preedit"
        );
    }

    /// `TextChanged` means the text changed. A full field rejecting a char and a backspace at
    /// cursor 0 both leave it byte-identical, and every sibling widget in this subsystem is
    /// emit-on-change.
    #[test]
    fn input_that_changes_nothing_emits_no_text_changed() {
        let mut world = World::new();
        let e = spawn_text_input(&mut world, 0.0, 0.0, 200.0, 30.0, 0.5);
        {
            let ti = world.get_mut::<TextInput>(e).unwrap();
            ti.focused = true;
            ti.max_len = 2;
            ti.insert_str("ab"); // full
        }

        let vp = viewport();
        let mut scratch = Vec::new();

        // A char the field has no room for.
        let mut output = UiOutput::default();
        super::run(
            &mut world,
            &vp,
            &chars_input(vec!['c']),
            0.016,
            &mut output,
            &mut scratch,
        );
        assert_eq!(
            world.get::<TextInput>(e).unwrap().text,
            "ab",
            "a full field rejects the char"
        );
        assert_eq!(
            changed_count(&output),
            0,
            "a rejected char changed nothing, so it must not emit TextChanged"
        );

        // A backspace with nothing before the cursor.
        world.get_mut::<TextInput>(e).unwrap().cursor = 0;
        let mut output = UiOutput::default();
        super::run(
            &mut world,
            &vp,
            &chars_input(vec!['\x08']),
            0.016,
            &mut output,
            &mut scratch,
        );
        assert_eq!(
            world.get::<TextInput>(e).unwrap().text,
            "ab",
            "backspace at cursor 0 deletes nothing"
        );
        assert_eq!(
            changed_count(&output),
            0,
            "a no-op backspace must not emit TextChanged"
        );
    }

    fn changed_count(output: &UiOutput) -> usize {
        output
            .events
            .iter()
            .filter(|ev| matches!(ev, UiEvent::TextChanged(_, _)))
            .count()
    }
}
