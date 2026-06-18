use crate::color::Color;
use crate::ecs::{Entity, World};
use crate::renderer::DrawRect;
use crate::resources::ViewportSize;
use crate::ui::button::{Button, ButtonState};
use crate::ui::checkbox::CheckBox;
use crate::ui::focus::UiFocus;
use crate::ui::slider::Slider;
use crate::ui::text_input::TextInput;

use super::state::{in_bounds, node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Focus-ring appearance.
const RING_COLOR: Color = Color::rgba(1.0, 0.85, 0.3, 1.0);
const RING_THICKNESS: f32 = 3.0;
/// Fraction of a slider's range that one Left/Right press nudges the value.
const SLIDER_STEP_FRAC: f32 = 0.05;

/// Keyboard focus pass: Tab/Shift+Tab cycle focus across focusable widgets, clicking moves focus to
/// the clicked widget, Enter/Space activate (button click / checkbox toggle), Left/Right nudge a
/// focused slider, and a focus ring is drawn around the focused widget. Runs before the widget
/// passes so a Tab-focused `TextInput` receives this frame's typed characters.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    collect_focusables(world, viewport, scratch);
    if scratch.is_empty() {
        if let Some(f) = world.resource_mut::<UiFocus>() {
            f.entity = None;
        }
        return;
    }
    let focusables = &*scratch;

    // Current focus, dropped if it is no longer focusable (despawned / hidden / disabled).
    let mut focus = world
        .resource::<UiFocus>()
        .and_then(|f| f.entity)
        .filter(|e| focusables.contains(e));

    // A click moves focus to the clicked widget, so Tab resumes from there.
    if input.just_released {
        for &e in focusables {
            if let Some((pos, size, _, _)) = node_layout(world, e, viewport) {
                if in_bounds(input.press_cursor, pos, size)
                    && in_bounds(input.release_cursor, pos, size)
                {
                    focus = Some(e);
                }
            }
        }
    }

    // Tab / Shift+Tab cycles focus.
    if input.tab {
        focus = Some(advance(focusables, focus, input.shift));
    }

    // Sync each TextInput's `focused` flag with the focus so typing targets the focused field.
    for &e in focusables {
        if world.get::<TextInput>(e).is_some() {
            let should_focus = focus == Some(e);
            if let Some(ti) = world.get_mut::<TextInput>(e) {
                ti.focused = should_focus;
            }
        }
    }

    // Activate / adjust the focused widget. Skip when a TextInput is focused — Enter/Space/arrows
    // are text input there, handled by the text-input pass.
    if let Some(e) = focus {
        let is_text = world.get::<TextInput>(e).is_some();
        if !is_text {
            if input.activate {
                if world.get::<Button>(e).is_some() {
                    output.events.push(UiEvent::ButtonClicked(e));
                } else if let Some(cb) = world.get_mut::<CheckBox>(e) {
                    cb.checked = !cb.checked;
                    let checked = cb.checked;
                    output.events.push(UiEvent::CheckBoxToggled(e, checked));
                }
            }
            if input.nav_left || input.nav_right {
                if let Some(sl) = world.get_mut::<Slider>(e) {
                    let step = (sl.max - sl.min) * SLIDER_STEP_FRAC;
                    let delta = if input.nav_right { step } else { -step };
                    let new_val = (sl.value + delta).clamp(sl.min, sl.max);
                    if (new_val - sl.value).abs() > f32::EPSILON {
                        sl.value = new_val;
                        output.events.push(UiEvent::SliderChanged(e, new_val));
                    }
                }
            }
        }
        if let Some((pos, size, z, visible)) = node_layout(world, e, viewport) {
            if visible {
                push_ring(output, pos, size, z);
            }
        }
    }

    if let Some(f) = world.resource_mut::<UiFocus>() {
        f.entity = focus;
    }
}

/// Fills `out` with focusable widget entities (a `UiNode` + a focusable widget, visible, and — for
/// buttons — not disabled), ordered by entity index for a stable Tab sequence.
fn collect_focusables(world: &World, viewport: &ViewportSize, out: &mut Vec<Entity>) {
    out.clear();
    out.extend(world.query::<Button>().map(|(e, _)| e));
    out.extend(world.query::<TextInput>().map(|(e, _)| e));
    out.extend(world.query::<Slider>().map(|(e, _)| e));
    out.extend(world.query::<CheckBox>().map(|(e, _)| e));
    out.sort_by_key(|e| e.index());
    out.dedup();
    out.retain(|&e| {
        let visible = node_layout(world, e, viewport)
            .map(|l| l.3)
            .unwrap_or(false);
        let button_ok = world
            .get::<Button>(e)
            .map(|b| b.state != ButtonState::Disabled)
            .unwrap_or(true);
        visible && button_ok
    });
}

/// The next focusable after `current` (wrapping), or the first/last when nothing is focused yet.
fn advance(focusables: &[Entity], current: Option<Entity>, reverse: bool) -> Entity {
    let n = focusables.len();
    match current.and_then(|c| focusables.iter().position(|&e| e == c)) {
        Some(i) => {
            let next = if reverse {
                (i + n - 1) % n
            } else {
                (i + 1) % n
            };
            focusables[next]
        }
        None if reverse => focusables[n - 1],
        None => focusables[0],
    }
}

/// Pushes the four border rects of a focus ring around `(pos, size)`, just above the widget's `z`.
fn push_ring(output: &mut UiOutput, pos: glam::Vec2, size: glam::Vec2, z: f32) {
    let zr = z + 0.5;
    let t = RING_THICKNESS;
    output
        .rects
        .push(DrawRect::new(pos.x, pos.y, size.x, t, RING_COLOR).with_z(zr));
    output
        .rects
        .push(DrawRect::new(pos.x, pos.y + size.y - t, size.x, t, RING_COLOR).with_z(zr));
    output
        .rects
        .push(DrawRect::new(pos.x, pos.y, t, size.y, RING_COLOR).with_z(zr));
    output
        .rects
        .push(DrawRect::new(pos.x + size.x - t, pos.y, t, size.y, RING_COLOR).with_z(zr));
}

#[cfg(test)]
mod tests {
    use winit::keyboard::KeyCode;

    use crate::ecs::{Events, System, World};
    use crate::input::{GamepadButton, GamepadState, InputState};
    use crate::resources::ViewportSize;
    use crate::ui::{Button, ButtonState, CheckBox, UiEvent, UiFocus, UiNode, UiSystem};

    /// World with three focusable widgets (button, checkbox, button) stacked vertically.
    fn world_with_widgets() -> (World, [crate::ecs::Entity; 3]) {
        let mut w = World::new();
        w.insert_resource(ViewportSize::new(400, 300));
        w.insert_resource(InputState::default());
        w.insert_resource(UiFocus::default());
        w.insert_resource(Events::<UiEvent>::default());
        let e0 = w.spawn();
        let e1 = w.spawn();
        let e2 = w.spawn();
        for (i, &e) in [e0, e1, e2].iter().enumerate() {
            w.add_component(e, UiNode::new(10.0, 10.0 + i as f32 * 40.0, 100.0, 30.0));
        }
        w.add_component(e0, Button::new("a"));
        w.add_component(e1, CheckBox::new("b"));
        w.add_component(e2, Button::new("c"));
        (w, [e0, e1, e2])
    }

    fn press(w: &mut World, key: KeyCode) {
        let mut input = InputState::default();
        input.press(key);
        w.insert_resource(input);
    }

    fn focus(w: &World) -> Option<crate::ecs::Entity> {
        w.resource::<UiFocus>().and_then(|f| f.entity)
    }

    /// Inserts a fresh `GamepadState` (slot 0) with `button` just-pressed, plus a cleared keyboard
    /// `InputState`, so the next `UiSystem::run` sees only that gamepad input.
    fn press_pad(w: &mut World, button: GamepadButton) {
        w.insert_resource(InputState::default());
        let mut gp = GamepadState::default();
        gp.test_press(0, button);
        w.insert_resource(gp);
    }

    #[test]
    fn gamepad_dpad_down_advances_focus_and_wraps() {
        let (mut w, e) = world_with_widgets();
        let mut sys = UiSystem::new();
        for expected in [e[0], e[1], e[2], e[0]] {
            press_pad(&mut w, GamepadButton::DPadDown);
            sys.run(&mut w, 0.0);
            assert_eq!(focus(&w), Some(expected));
        }
    }

    #[test]
    fn gamepad_dpad_up_reverses_focus() {
        let (mut w, e) = world_with_widgets();
        // First D-pad Up from no focus lands on the last widget (like Shift+Tab).
        press_pad(&mut w, GamepadButton::DPadUp);
        UiSystem::new().run(&mut w, 0.0);
        assert_eq!(focus(&w), Some(e[2]));
    }

    #[test]
    fn gamepad_south_activates_focused_button() {
        let (mut w, e) = world_with_widgets();
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e[0]);
        press_pad(&mut w, GamepadButton::South);
        UiSystem::new().run(&mut w, 0.0);
        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            events.contains(&UiEvent::ButtonClicked(e[0])),
            "A (South) on a focused button should emit ButtonClicked, got {events:?}"
        );
    }

    #[test]
    fn tab_cycles_focus_in_entity_order_and_wraps() {
        let (mut w, e) = world_with_widgets();
        let mut sys = UiSystem::new();
        for expected in [e[0], e[1], e[2], e[0]] {
            press(&mut w, KeyCode::Tab);
            sys.run(&mut w, 0.0);
            assert_eq!(focus(&w), Some(expected));
        }
    }

    #[test]
    fn shift_tab_cycles_backwards() {
        let (mut w, e) = world_with_widgets();
        let mut sys = UiSystem::new();
        // First Shift+Tab from no focus lands on the last widget.
        let mut input = InputState::default();
        input.press(KeyCode::Tab);
        input.press(KeyCode::ShiftLeft);
        w.insert_resource(input);
        sys.run(&mut w, 0.0);
        assert_eq!(focus(&w), Some(e[2]));
    }

    #[test]
    fn enter_activates_focused_button() {
        let (mut w, e) = world_with_widgets();
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e[0]);
        press(&mut w, KeyCode::Enter);
        UiSystem::new().run(&mut w, 0.0);
        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            events.contains(&UiEvent::ButtonClicked(e[0])),
            "Enter on a focused button should emit ButtonClicked, got {events:?}"
        );
    }

    #[test]
    fn space_toggles_focused_checkbox() {
        let (mut w, e) = world_with_widgets();
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e[1]);
        press(&mut w, KeyCode::Space);
        UiSystem::new().run(&mut w, 0.0);
        assert!(
            w.get::<CheckBox>(e[1]).unwrap().checked,
            "Space on a focused checkbox should toggle it on"
        );
    }

    #[test]
    fn disabled_button_is_skipped() {
        let (mut w, e) = world_with_widgets();
        w.get_mut::<Button>(e[2]).unwrap().state = ButtonState::Disabled;
        let mut sys = UiSystem::new();
        // From no focus, Tab→e0, Tab→e1, Tab should wrap to e0 (e2 disabled, skipped).
        for expected in [e[0], e[1], e[0]] {
            press(&mut w, KeyCode::Tab);
            sys.run(&mut w, 0.0);
            assert_eq!(focus(&w), Some(expected));
        }
    }
}
