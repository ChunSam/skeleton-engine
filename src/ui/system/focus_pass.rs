use crate::ecs::{Entity, World};
use crate::renderer::DrawRect;
use crate::resources::ViewportSize;
use crate::ui::button::{Button, ButtonState};
use crate::ui::checkbox::CheckBox;
use crate::ui::dropdown::Dropdown;
use crate::ui::focus::{FocusRingStyle, UiFocus};
use crate::ui::slider::Slider;
use crate::ui::text_input::TextInput;

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Keyboard focus pass: Tab/Shift+Tab cycle focus across focusable widgets, clicking moves focus to
/// the clicked widget, Enter/Space activate (button click / checkbox toggle), Left/Right nudge a
/// focused slider, and a focus ring is drawn around the focused widget. Runs before the widget
/// passes so a Tab-focused `TextInput` receives this frame's typed characters.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    elapsed: f32,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    collect_focusables(world, viewport, scratch);
    if scratch.is_empty() {
        if let Some(f) = world.resource_mut::<UiFocus>() {
            f.entity = None;
        }
        // Clear focus on any TextInputs that may still have focused=true.
        let all: Vec<_> = world.query::<TextInput>().map(|(e, _)| e).collect();
        for e in all {
            let was_focused = world.get::<TextInput>(e).is_some_and(|ti| ti.focused);
            if was_focused {
                if let Some(ti) = world.get_mut::<TextInput>(e) {
                    ti.focused = false;
                }
                output.events.push(UiEvent::TextBlurred(e));
            }
        }
        return;
    }
    let focusables = &*scratch;

    // Current focus, dropped if it is no longer focusable (despawned / hidden / disabled).
    let mut focus = world
        .resource::<UiFocus>()
        .and_then(|f| f.entity)
        .filter(|&e| is_focusable(focusables, e));

    // A click moves focus to the clicked widget, so Tab resumes from there. The shared pointer
    // capture decides which widget the click landed on across *all* widget kinds (the same topmost
    // surface the button/checkbox/etc. passes act on), so a focusable covered by another widget is
    // not focused through it. The click counts only when press and release land on the same surface.
    if input.just_released {
        let owner = capture.topmost_at(input.press_cursor);
        if owner.is_some() && owner == capture.topmost_at(input.release_cursor) {
            if let Some(e) = owner {
                if is_focusable(focusables, e) {
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
    // Detect focus transitions and emit TextFocused / TextBlurred events.
    for &e in focusables {
        if world.get::<TextInput>(e).is_some() {
            let should_focus = focus == Some(e);
            let was_focused = world.get::<TextInput>(e).is_some_and(|ti| ti.focused);
            if let Some(ti) = world.get_mut::<TextInput>(e) {
                ti.focused = should_focus;
                if !was_focused && should_focus {
                    // Reset caret on focus-in so the cursor appears immediately.
                    ti.cursor_blink = 0.0;
                    ti.cursor_visible = true;
                }
            }
            if !was_focused && should_focus {
                output.events.push(UiEvent::TextFocused(e));
            } else if was_focused && !should_focus {
                output.events.push(UiEvent::TextBlurred(e));
            }
        }
    }

    // Also sync TextInputs that are NOT in the focusables list (e.g. invisible / despawned):
    // they may still hold `focused = true` from a previous frame and need to be cleared.
    let mut all_text_inputs: Vec<_> = world.query::<TextInput>().map(|(e, _)| e).collect();
    for e in all_text_inputs.drain(..) {
        if is_focusable(focusables, e) {
            continue; // already handled above
        }
        let was_focused = world.get::<TextInput>(e).is_some_and(|ti| ti.focused);
        if was_focused {
            if let Some(ti) = world.get_mut::<TextInput>(e) {
                ti.focused = false;
            }
            output.events.push(UiEvent::TextBlurred(e));
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
                } else if world.get::<Dropdown>(e).is_some() {
                    let others: Vec<Entity> = world
                        .query::<Dropdown>()
                        .map(|(en, _)| en)
                        .filter(|&en| en != e)
                        .collect();
                    let mut opened = false;
                    if let Some(dd) = world.get_mut::<Dropdown>(e) {
                        if !dd.items.is_empty() {
                            dd.open = !dd.open;
                            opened = dd.open;
                        }
                    }
                    // Only one list open at a time: opening via Enter closes any other open
                    // dropdown, matching the pointer path (where pressing one dropdown is a
                    // press-away for every other).
                    if opened {
                        for other in others {
                            if let Some(od) = world.get_mut::<Dropdown>(other) {
                                od.open = false;
                            }
                        }
                    }
                }
            }
            if input.nav_left || input.nav_right {
                if let Some(sl) = world.get_mut::<Slider>(e) {
                    let step = sl.resolved_keyboard_step();
                    let delta = if input.nav_right { step } else { -step };
                    let new_val = (sl.value + delta).clamp(sl.min, sl.max);
                    if (new_val - sl.value).abs() > f32::EPSILON {
                        sl.value = new_val;
                        output.events.push(UiEvent::SliderChanged(e, new_val));
                    }
                } else if let Some(dd) = world.get_mut::<Dropdown>(e) {
                    // Step the selection directly (clamped, like a Slider nudge) — a settings row
                    // is fully keyboard/gamepad-operable without opening the list.
                    if !dd.items.is_empty() {
                        let cur = dd.selected_index();
                        let next = if input.nav_right {
                            (cur + 1).min(dd.items.len() - 1)
                        } else {
                            cur.saturating_sub(1)
                        };
                        if next != cur {
                            dd.selected = next;
                            output.events.push(UiEvent::DropdownChanged(e, next));
                        }
                    }
                }
            }
        }
        if let Some((pos, size, z, visible)) = node_layout(world, e, viewport) {
            if visible {
                let style = world
                    .resource::<FocusRingStyle>()
                    .copied()
                    .unwrap_or_default();
                push_ring(output, pos, size, z, elapsed, &style);
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
    out.extend(world.query::<Dropdown>().map(|(e, _)| e));
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

/// Membership test against the index-sorted `focusables` slice (built by [`collect_focusables`],
/// which sorts by [`Entity::index`]). `O(log n)` instead of a linear `contains` scan.
fn is_focusable(focusables: &[Entity], e: Entity) -> bool {
    focusables
        .binary_search_by_key(&e.index(), |x| x.index())
        .is_ok()
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

/// Pushes a focus ring around `(pos, size)`, just above the widget's `z`, styled by `style`.
/// Draws nothing when the style is not visible (disabled / non-positive thickness).
/// `elapsed` (seconds) drives the optional alpha pulse; it has no effect on a non-pulsing style.
///
/// When `style.corner_radius > 0.0` the ring is a single rounded outline quad (SDF-rendered by
/// the UI pipeline). Otherwise it is the historical four axis-aligned border bars — byte-identical
/// to before.
fn push_ring(
    output: &mut UiOutput,
    pos: glam::Vec2,
    size: glam::Vec2,
    z: f32,
    elapsed: f32,
    style: &FocusRingStyle,
) {
    if !style.is_visible() {
        return;
    }
    let zr = z + 0.5;
    let t = style.thickness;
    let mut color = style.color;
    color.a *= style.pulse_alpha(elapsed);

    if style.corner_radius > 0.0 {
        // One rounded outline quad covering the widget bounds; the shader insets the `t`-wide ring.
        output.rects.push(
            DrawRect::new(pos.x, pos.y, size.x, size.y, color)
                .with_z(zr)
                .with_corner_radius(style.corner_radius)
                .with_border(t),
        );
        return;
    }

    output
        .rects
        .push(DrawRect::new(pos.x, pos.y, size.x, t, color).with_z(zr));
    output
        .rects
        .push(DrawRect::new(pos.x, pos.y + size.y - t, size.x, t, color).with_z(zr));
    output
        .rects
        .push(DrawRect::new(pos.x, pos.y, t, size.y, color).with_z(zr));
    output
        .rects
        .push(DrawRect::new(pos.x + size.x - t, pos.y, t, size.y, color).with_z(zr));
}

#[cfg(test)]
mod tests {
    use winit::keyboard::KeyCode;

    use crate::color::Color;
    use crate::ecs::{Events, System, World};
    use crate::input::{GamepadAxis, GamepadButton, GamepadState, InputState};
    use crate::renderer::UiQueue;
    use crate::resources::ViewportSize;
    use crate::ui::{
        Button, ButtonState, CheckBox, FocusRingStyle, Slider, UiEvent, UiFocus, UiNode, UiSystem,
    };

    use super::{push_ring, UiOutput};

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

    /// Sets the left-stick `axis` to `value` on slot 0 (connecting it if needed) and clears the
    /// keyboard, so the next `UiSystem::run` sees only that analog input. Reuses any existing
    /// `GamepadState` so axis values persist across frames — mirroring the real input flush, which
    /// clears just-pressed buttons but keeps axis values.
    fn set_stick(w: &mut World, axis: GamepadAxis, value: f32) {
        w.insert_resource(InputState::default());
        if w.resource::<GamepadState>().is_none() {
            w.insert_resource(GamepadState::default());
        }
        w.resource_mut::<GamepadState>()
            .unwrap()
            .test_axis(0, axis, value);
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
    fn left_stick_down_advances_focus_and_wraps() {
        // Pushing the left stick down advances focus like Tab / D-pad Down, but the stick must
        // return to neutral between pushes (it does not auto-repeat). Engine convention: down = +Y.
        let (mut w, e) = world_with_widgets();
        let mut sys = UiSystem::new();
        for expected in [e[0], e[1], e[2], e[0]] {
            set_stick(&mut w, GamepadAxis::LeftStickY, 0.8); // push down (+Y)
            sys.run(&mut w, 0.0);
            assert_eq!(focus(&w), Some(expected));
            set_stick(&mut w, GamepadAxis::LeftStickY, 0.0); // back to neutral so the next push fires
            sys.run(&mut w, 0.0);
        }
    }

    #[test]
    fn left_stick_up_reverses_focus() {
        // First stick-up from no focus lands on the last widget (like Shift+Tab / D-pad Up). Engine
        // convention: up = -Y.
        let (mut w, e) = world_with_widgets();
        let mut sys = UiSystem::new();
        set_stick(&mut w, GamepadAxis::LeftStickY, -0.8); // push up (-Y)
        sys.run(&mut w, 0.0);
        assert_eq!(focus(&w), Some(e[2]));
    }

    #[test]
    fn held_left_stick_does_not_auto_repeat_focus() {
        let (mut w, e) = world_with_widgets();
        let mut sys = UiSystem::new();
        // First push (down = +Y) advances to e[0].
        set_stick(&mut w, GamepadAxis::LeftStickY, 0.8);
        sys.run(&mut w, 0.0);
        assert_eq!(focus(&w), Some(e[0]));
        // Holding the stick down (same value) must NOT advance again.
        sys.run(&mut w, 0.0);
        assert_eq!(
            focus(&w),
            Some(e[0]),
            "a held stick must not auto-repeat focus"
        );
        // Relaxing into the hysteresis band (still past the release threshold) must not fire.
        set_stick(&mut w, GamepadAxis::LeftStickY, 0.5);
        sys.run(&mut w, 0.0);
        assert_eq!(
            focus(&w),
            Some(e[0]),
            "stick in the hysteresis band must not fire"
        );
        // Return to neutral, then push again → advances.
        set_stick(&mut w, GamepadAxis::LeftStickY, 0.0);
        sys.run(&mut w, 0.0);
        set_stick(&mut w, GamepadAxis::LeftStickY, 0.8);
        sys.run(&mut w, 0.0);
        assert_eq!(
            focus(&w),
            Some(e[1]),
            "pushing again after returning to neutral advances focus"
        );
    }

    #[test]
    fn left_stick_right_nudges_focused_slider() {
        // A focused Slider is nudged by the left stick X axis like D-pad Left/Right: +X is right.
        let mut w = World::new();
        w.insert_resource(ViewportSize::new(400, 300));
        w.insert_resource(InputState::default());
        w.insert_resource(UiFocus::default());
        w.insert_resource(Events::<UiEvent>::default());
        let e = w.spawn();
        w.add_component(e, UiNode::new(10.0, 10.0, 200.0, 30.0));
        w.add_component(e, Slider::new(0.0, 100.0, 50.0));
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e);

        let mut sys = UiSystem::new();
        set_stick(&mut w, GamepadAxis::LeftStickX, 0.8); // push right
        sys.run(&mut w, 0.0);
        let v = w.get::<Slider>(e).unwrap().value;
        assert!(
            v > 50.0,
            "stick-right should nudge the focused slider up, got {v}"
        );
    }

    #[test]
    fn focused_slider_honors_a_custom_keyboard_step() {
        // A 0..3 slider with an absolute step of 1.0 moves exactly one level per ←/→,
        // not the default 5%-of-range (which would be 0.15 here).
        let mut w = World::new();
        w.insert_resource(ViewportSize::new(400, 300));
        w.insert_resource(InputState::default());
        w.insert_resource(UiFocus::default());
        w.insert_resource(Events::<UiEvent>::default());
        let e = w.spawn();
        w.add_component(e, UiNode::new(10.0, 10.0, 200.0, 30.0));
        w.add_component(e, Slider::new(0.0, 3.0, 0.0).with_keyboard_step(1.0));
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e);

        let mut sys = UiSystem::new();
        press(&mut w, KeyCode::ArrowRight);
        sys.run(&mut w, 0.0);
        let v = w.get::<Slider>(e).unwrap().value;
        assert!(
            (v - 1.0).abs() < f32::EPSILON,
            "one ArrowRight should step exactly one level (1.0), got {v}"
        );
    }

    // ── Focus-ring styling tests ─────────────────────────────────────────────

    /// `push_ring` emits four border rects in the style's color, sized by its thickness.
    #[test]
    fn push_ring_uses_custom_color_and_thickness() {
        let style = FocusRingStyle {
            color: Color::rgb(0.3, 0.9, 1.0),
            thickness: 6.0,
            enabled: true,
            ..Default::default()
        };
        let mut out = UiOutput::default();
        push_ring(
            &mut out,
            glam::Vec2::new(10.0, 20.0),
            glam::Vec2::new(100.0, 40.0),
            0.0,
            0.0,
            &style,
        );
        assert_eq!(out.rects.len(), 4, "a ring is four border rects");
        assert!(
            out.rects.iter().all(|r| r.color == style.color),
            "every ring rect uses the style color"
        );
        // Top/bottom borders are `thickness` tall; left/right are `thickness` wide.
        assert!(out.rects.iter().any(|r| r.h == 6.0 && r.w == 100.0));
        assert!(out.rects.iter().any(|r| r.w == 6.0 && r.h == 40.0));
    }

    /// A `corner_radius > 0.0` style draws a single rounded *outline* quad covering the widget
    /// bounds (radius + border carried on the rect) instead of the four sharp bars.
    #[test]
    fn push_ring_rounded_emits_single_outline_rect() {
        let style = FocusRingStyle {
            color: Color::rgb(0.3, 0.9, 1.0),
            thickness: 5.0,
            corner_radius: 12.0,
            enabled: true,
            ..Default::default()
        };
        let pos = glam::Vec2::new(10.0, 20.0);
        let size = glam::Vec2::new(100.0, 40.0);
        let mut out = UiOutput::default();
        push_ring(&mut out, pos, size, 0.0, 0.0, &style);

        assert_eq!(
            out.rects.len(),
            1,
            "a rounded ring is a single outline rect"
        );
        let r = &out.rects[0];
        // Covers the full widget bounds, carrying the radius + border for the SDF shader.
        assert_eq!((r.x, r.y, r.w, r.h), (10.0, 20.0, 100.0, 40.0));
        assert_eq!(r.corner_radius, 12.0);
        assert_eq!(r.border, 5.0, "border width == ring thickness");
    }

    /// A disabled style — or a non-positive thickness — draws no ring at all.
    #[test]
    fn push_ring_disabled_or_zero_thickness_draws_nothing() {
        let pos = glam::Vec2::new(10.0, 20.0);
        let size = glam::Vec2::new(100.0, 40.0);
        for style in [
            FocusRingStyle {
                enabled: false,
                ..Default::default()
            },
            FocusRingStyle {
                thickness: 0.0,
                ..Default::default()
            },
        ] {
            let mut out = UiOutput::default();
            push_ring(&mut out, pos, size, 0.0, 0.0, &style);
            assert!(out.rects.is_empty(), "no ring rects for {style:?}");
        }
    }

    /// A pulsing style modulates the ring's alpha through `push_ring`: full alpha at the pulse
    /// peak (quarter period), `pulse_min_alpha` at the trough (three-quarter period).
    #[test]
    fn push_ring_applies_pulse_alpha() {
        let style = FocusRingStyle {
            color: Color::rgba(0.3, 0.9, 1.0, 1.0),
            thickness: 4.0,
            pulse_hz: 1.0,
            pulse_min_alpha: 0.25,
            ..Default::default()
        };
        let pos = glam::Vec2::new(10.0, 20.0);
        let size = glam::Vec2::new(100.0, 40.0);

        let mut trough = UiOutput::default();
        push_ring(&mut trough, pos, size, 0.0, 0.75, &style); // trough → min alpha
        assert!(
            trough.rects.iter().all(|r| (r.color.a - 0.25).abs() < 1e-5),
            "ring alpha at the pulse trough should be pulse_min_alpha"
        );

        let mut peak = UiOutput::default();
        push_ring(&mut peak, pos, size, 0.0, 0.25, &style); // peak → full alpha
        assert!(
            peak.rects.iter().all(|r| (r.color.a - 1.0).abs() < 1e-5),
            "ring alpha at the pulse peak should be the full color alpha"
        );
    }

    /// The default style (no resource inserted) reproduces the historical amber 3px ring exactly.
    #[test]
    fn default_focus_ring_matches_historical_appearance() {
        let style = FocusRingStyle::default();
        assert_eq!(style.color, Color::rgba(1.0, 0.85, 0.3, 1.0));
        assert_eq!(style.thickness, 3.0);
        assert!(style.is_visible());
    }

    /// End-to-end: `UiSystem` reads the `FocusRingStyle` resource and the focused widget's ring
    /// reaches the `UiQueue` in the configured color.
    #[test]
    fn ui_system_consumes_focus_ring_style_resource() {
        let (mut w, e) = world_with_widgets();
        w.insert_resource(UiQueue::default());
        let ring = Color::rgb(0.3, 0.9, 1.0);
        w.insert_resource(FocusRingStyle {
            color: ring,
            thickness: 5.0,
            enabled: true,
            ..Default::default()
        });
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e[0]);

        UiSystem::new().run(&mut w, 0.0);

        let ring_rects = w
            .resource::<UiQueue>()
            .unwrap()
            .items
            .iter()
            .filter(|r| r.color == ring)
            .count();
        assert_eq!(
            ring_rects, 4,
            "the focused widget's ring should reach the UiQueue in the configured color"
        );
    }

    /// End-to-end: a disabled ring style suppresses the ring — no rects in the focused widget's
    /// ring color reach the queue (a plain Button draws no other rects).
    #[test]
    fn ui_system_disabled_ring_style_draws_no_ring() {
        let (mut w, e) = world_with_widgets();
        w.insert_resource(UiQueue::default());
        w.insert_resource(FocusRingStyle {
            enabled: false,
            ..Default::default()
        });
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e[0]);

        UiSystem::new().run(&mut w, 0.0);

        let amber = FocusRingStyle::default().color;
        assert!(
            !w.resource::<UiQueue>()
                .unwrap()
                .items
                .iter()
                .any(|r| r.color == amber),
            "a disabled ring style should draw no focus ring"
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

    // ── TextInput focus authority tests ──────────────────────────────────────

    use crate::ui::text_input::TextInput;
    use winit::event::MouseButton;

    /// Spawns a TextInput at the given screen position/size and z level.
    fn spawn_text_input(
        w: &mut World,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        z: f32,
    ) -> crate::ecs::Entity {
        let e = w.spawn();
        let mut node = UiNode::new(x, y, width, height).with_z(z);
        node.visible = true;
        w.add_component(e, node);
        w.add_component(e, TextInput::new(""));
        e
    }

    /// Simulate a complete click (press + release) at `pos` in one frame.
    fn click_at(w: &mut World, pos: glam::Vec2) {
        let mut input = InputState::default();
        input.set_cursor(pos);
        input.press_mouse(MouseButton::Left);
        input.release_mouse(MouseButton::Left);
        w.insert_resource(input);
    }

    fn world_with_focus_resources() -> World {
        let mut w = World::new();
        w.insert_resource(ViewportSize::new(800, 600));
        w.insert_resource(InputState::default());
        w.insert_resource(UiFocus::default());
        w.insert_resource(Events::<UiEvent>::default());
        w
    }

    /// Two overlapping TextInputs at different z: clicking the shared area focuses the one drawn on
    /// top (greater z), matching what the player sees — focus now follows the shared pointer capture
    /// (z-order), consistent with the button/checkbox/etc. passes. The TextFocused event is emitted
    /// only for that entity. (Previously focus ignored z and picked the last entity index; the new
    /// occlusion-aware capture makes z decide.)
    #[test]
    fn click_on_overlapping_text_inputs_focuses_topmost_z() {
        let mut w = world_with_focus_resources();
        // `first` has the lower entity index but the HIGHER z (drawn on top), so it must win even
        // though `second` was spawned later.
        let first = spawn_text_input(&mut w, 0.0, 0.0, 100.0, 30.0, 0.9);
        let second = spawn_text_input(&mut w, 0.0, 0.0, 100.0, 30.0, 0.5);

        click_at(&mut w, glam::Vec2::new(50.0, 15.0));
        UiSystem::new().run(&mut w, 0.0);

        assert!(
            w.get::<TextInput>(first).unwrap().focused,
            "the higher-z TextInput (drawn on top) should be focused after click"
        );
        assert!(
            !w.get::<TextInput>(second).unwrap().focused,
            "the lower-z TextInput (behind) should not be focused"
        );

        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        let focused_entities: Vec<_> = events
            .iter()
            .filter_map(|ev| {
                if let UiEvent::TextFocused(e) = ev {
                    Some(*e)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            focused_entities,
            vec![first],
            "TextFocused should be emitted only for the focused entity"
        );
    }

    /// Two overlapping TextInputs at the SAME z: the tie is broken by entity index (later-spawned /
    /// higher index draws on top in painter's order), so the last-spawned one is focused.
    #[test]
    fn click_on_same_z_text_inputs_focuses_last_spawned() {
        let mut w = world_with_focus_resources();
        let first = spawn_text_input(&mut w, 0.0, 0.0, 100.0, 30.0, 0.5);
        let second = spawn_text_input(&mut w, 0.0, 0.0, 100.0, 30.0, 0.5);

        click_at(&mut w, glam::Vec2::new(50.0, 15.0));
        UiSystem::new().run(&mut w, 0.0);

        assert!(
            w.get::<TextInput>(second).unwrap().focused,
            "on a z tie, the higher-entity-index (last-spawned) TextInput should be focused"
        );
        assert!(
            !w.get::<TextInput>(first).unwrap().focused,
            "the lower-index TextInput should not be focused on a z tie"
        );
    }

    /// Clicking the position of an invisible TextInput must not focus it; no TextFocused event.
    #[test]
    fn invisible_text_input_does_not_receive_focus_on_click() {
        let mut w = world_with_focus_resources();
        let e = spawn_text_input(&mut w, 0.0, 0.0, 100.0, 30.0, 0.9);
        w.get_mut::<UiNode>(e).unwrap().visible = false;

        click_at(&mut w, glam::Vec2::new(50.0, 15.0));
        UiSystem::new().run(&mut w, 0.0);

        assert!(
            !w.get::<TextInput>(e).unwrap().focused,
            "invisible TextInput must not be focused"
        );
        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextFocused(_))),
            "no TextFocused event expected for invisible widget"
        );
    }

    /// A focused TextInput that becomes invisible has its focus cleared + TextBlurred emitted.
    #[test]
    fn focus_cleared_when_text_input_becomes_invisible() {
        let mut w = world_with_focus_resources();
        let e = spawn_text_input(&mut w, 0.0, 0.0, 100.0, 30.0, 0.9);
        // Pre-focus the entity via UiFocus + ti.focused so focus_pass sees the transition.
        w.resource_mut::<UiFocus>().unwrap().entity = Some(e);
        w.get_mut::<TextInput>(e).unwrap().focused = true;
        // Now hide it.
        w.get_mut::<UiNode>(e).unwrap().visible = false;

        w.insert_resource(InputState::default()); // no input this frame
        UiSystem::new().run(&mut w, 0.0);

        assert!(
            !w.get::<TextInput>(e).unwrap().focused,
            "focus must be cleared when the widget becomes invisible"
        );
        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextBlurred(_))),
            "TextBlurred should be emitted when invisible widget loses focus"
        );
    }

    /// Clicking outside all focusable widgets does NOT blur a Tab-focused TextInput
    /// (no spurious TextBlurred).
    #[test]
    fn click_outside_does_not_blur_tab_focused_text_input() {
        let mut w = world_with_focus_resources();
        let e = spawn_text_input(&mut w, 10.0, 10.0, 100.0, 30.0, 0.5);

        // Tab-focus the TextInput first.
        {
            let mut input = InputState::default();
            input.press(KeyCode::Tab);
            w.insert_resource(input);
            UiSystem::new().run(&mut w, 0.0);
            // Flush events before the click frame.
            w.resource_mut::<Events<UiEvent>>().unwrap().flush();
        }

        assert_eq!(focus(&w), Some(e), "TextInput should be focused after Tab");

        // Now click far outside the widget.
        click_at(&mut w, glam::Vec2::new(500.0, 500.0));
        UiSystem::new().run(&mut w, 0.0);

        assert!(
            w.get::<TextInput>(e).unwrap().focused,
            "TextInput should still be focused — click outside must not blur"
        );
        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            !events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextBlurred(_))),
            "no spurious TextBlurred when clicking outside"
        );
    }

    /// TextFocused is emitted on focus-in and caret is reset.
    #[test]
    fn tab_focus_on_text_input_emits_text_focused_and_resets_caret() {
        let mut w = world_with_focus_resources();
        let e = spawn_text_input(&mut w, 10.0, 10.0, 100.0, 30.0, 0.5);
        // Give the TextInput some pre-existing blink state.
        {
            let ti = w.get_mut::<TextInput>(e).unwrap();
            ti.cursor_blink = 0.42;
            ti.cursor_visible = false;
        }

        press(&mut w, KeyCode::Tab);
        UiSystem::new().run(&mut w, 0.0);

        assert_eq!(focus(&w), Some(e));
        let ti = w.get::<TextInput>(e).unwrap();
        assert!(ti.focused, "TextInput should be focused after Tab");
        assert_eq!(
            ti.cursor_blink, 0.0,
            "caret blink should be reset on focus-in"
        );
        assert!(ti.cursor_visible, "caret should be visible on focus-in");

        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextFocused(en) if *en == e)),
            "TextFocused should be emitted on Tab focus-in"
        );
    }

    /// TextBlurred is emitted when Tab moves focus away from a TextInput.
    #[test]
    fn tab_away_from_text_input_emits_text_blurred() {
        let mut w = world_with_focus_resources();
        let ti_e = spawn_text_input(&mut w, 10.0, 10.0, 100.0, 30.0, 0.5);
        // Add a second focusable (Button) so Tab has somewhere to go.
        let btn_e = w.spawn();
        w.add_component(btn_e, UiNode::new(10.0, 60.0, 100.0, 30.0));
        w.add_component(btn_e, Button::new("next"));

        // Tab once → focus TextInput.
        press(&mut w, KeyCode::Tab);
        UiSystem::new().run(&mut w, 0.0);
        w.resource_mut::<Events<UiEvent>>().unwrap().flush();

        assert!(w.get::<TextInput>(ti_e).unwrap().focused);

        // Tab again → focus moves to the Button; TextInput should emit TextBlurred.
        press(&mut w, KeyCode::Tab);
        UiSystem::new().run(&mut w, 0.0);

        assert!(!w.get::<TextInput>(ti_e).unwrap().focused);
        let events = w.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            events
                .iter()
                .any(|ev| matches!(ev, UiEvent::TextBlurred(en) if *en == ti_e)),
            "TextBlurred should be emitted when Tab moves focus away from TextInput"
        );
    }
}
