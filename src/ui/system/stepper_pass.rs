use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText, TextAlign};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::stepper::{StepButton, Stepper};

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Handles every [`UiNode`] + [`Stepper`]: a completed click on the `-`/`+` button (press and
/// release both owned by this widget through the shared [`PointerCapture`], like a
/// [`CheckBox`](crate::ui::CheckBox) toggle) steps the value by `step`, clamped to `min..=max`, and
/// emits [`UiEvent::StepperChanged`] when the value actually changed (a click at a bound is silent).
/// Renders a rounded background, the two flanking buttons (hover-tinted, `-`/`+` glyphs), the
/// centered value label, and an optional border — all clipped to the node rect.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Stepper>().map(|(e, _, _)| e));

    let hover_owner = capture.topmost_at(input.cursor);
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

        // Step on release, like a Button/CheckBox click (only when both press and release land on
        // this widget — dragging onto another widget before releasing cancels).
        let clicked =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);
        if clicked {
            if let Some(st) = world.get_mut::<Stepper>(entity) {
                if let Some(btn) = st.zone_at(input.release_cursor, pos, size) {
                    let nv = st.stepped(btn == StepButton::Inc);
                    if (nv - st.clamped_value()).abs() > f32::EPSILON {
                        st.value = nv;
                        output.events.push(UiEvent::StepperChanged(entity, nv));
                    }
                }
            }
        }

        // ── Render (immutable reads now that state is settled) ────────────────
        let st = match world.get::<Stepper>(entity) {
            Some(s) => s,
            None => continue,
        };
        let step_z = super::UI_SUBLAYER_Z_STEP;

        // Background fill (rounded).
        output.rects.push(
            DrawRect::new(pos.x, pos.y, size.x, size.y, st.bg_color)
                .with_corner_radius(st.corner_radius)
                .with_z(z),
        );

        let bw = st.button_width(size);
        if bw > 0.0 {
            let hovered = if hover_owner == Some(entity) {
                st.zone_at(input.cursor, pos, size)
            } else {
                None
            };
            let dec_color = button_color(st, hovered == Some(StepButton::Dec));
            let inc_color = button_color(st, hovered == Some(StepButton::Inc));

            // Button backgrounds.
            output
                .rects
                .push(DrawRect::new(pos.x, pos.y, bw, size.y, dec_color).with_z(z + step_z));
            output.rects.push(
                DrawRect::new(pos.x + size.x - bw, pos.y, bw, size.y, inc_color).with_z(z + step_z),
            );

            // Button glyphs, centered in each button.
            push_centered(output, "-", pos.x, bw, pos.y, size.y, st, z + step_z * 2.0);
            push_centered(
                output,
                "+",
                pos.x + size.x - bw,
                bw,
                pos.y,
                size.y,
                st,
                z + step_z * 2.0,
            );

            // Value label, centered in the area between the two buttons.
            let mid_w = (size.x - bw * 2.0).max(0.0);
            if mid_w > 0.0 {
                push_centered(
                    output,
                    &st.label(),
                    pos.x + bw,
                    mid_w,
                    pos.y,
                    size.y,
                    st,
                    z + step_z * 2.0,
                );
            }
        }

        // Border outline on top, framing the whole widget.
        if st.border > 0.0 {
            output.rects.push(
                DrawRect::new(pos.x, pos.y, size.x, size.y, st.border_color)
                    .with_corner_radius(st.corner_radius)
                    .with_border(st.border)
                    .with_z(z + step_z * 3.0),
            );
        }
    }
}

/// The fill color for a stepper button: the hover tint when the cursor is over it, else the base.
fn button_color(st: &Stepper, hovered: bool) -> crate::color::Color {
    if hovered {
        st.button_hover_color
    } else {
        st.button_color
    }
}

/// Queues `text` horizontally centered within `[x, x + width]` and vertically centered in the node,
/// at the widget's layered z (so an occluding surface hides it — text z-ordering, v0.110.0).
#[allow(clippy::too_many_arguments)]
fn push_centered(
    output: &mut UiOutput,
    text: &str,
    x: f32,
    width: f32,
    node_y: f32,
    node_h: f32,
    st: &Stepper,
    z: f32,
) {
    if text.is_empty() {
        return;
    }
    let text_y = node_y + (node_h - st.font_size) / 2.0;
    output.texts.push(
        DrawText::new(
            text.to_string(),
            Vec2::new(x, text_y),
            st.font_size,
            st.text_color,
        )
        .with_bounds(Vec2::new(width, node_h))
        .with_align(TextAlign::Center)
        .with_z(z),
    );
}

#[cfg(test)]
mod tests {
    use glam::Vec2;
    use winit::event::MouseButton;
    use winit::keyboard::KeyCode;

    use crate::ecs::{Entity, Events, System, World};
    use crate::input::InputState;
    use crate::renderer::{TextQueue, UiQueue};
    use crate::resources::ViewportSize;
    use crate::ui::focus::UiFocus;
    use crate::ui::node::UiNode;
    use crate::ui::panel::{LayoutDir, Panel};
    use crate::ui::stepper::Stepper;
    use crate::ui::system::{UiEvent, UiSystem};

    fn setup() -> World {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(UiQueue::default());
        world.insert_resource(TextQueue::default());
        world.insert_resource(UiFocus::default());
        world.insert_resource(InputState::default());
        world
    }

    /// A 0..10 stepper (value 5, step 1) in a 120×40 node at (50, 50) → button width 40, so the
    /// `-` button spans x 50..90, the value area 90..130, the `+` button 130..170.
    fn spawn_stepper(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 120.0, 40.0));
        world.add_component(e, Stepper::new(0.0, 10.0, 5.0).with_step(1.0));
        e
    }

    /// One frame with a full click (press + release) at `pos`.
    fn click(world: &mut World, system: &mut UiSystem, pos: Vec2) {
        let input = world.resource_mut::<InputState>().unwrap();
        input.flush();
        input.set_cursor(pos);
        input.press_mouse(MouseButton::Left);
        input.release_mouse(MouseButton::Left);
        system.run(world, 0.016);
    }

    fn changed_events(world: &World) -> Vec<(Entity, f32)> {
        world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter_map(|e| {
                if let UiEvent::StepperChanged(en, v) = e {
                    Some((*en, *v))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Inserts a fresh `InputState` with `key` just-pressed (a held key never re-fires
    /// `just_pressed`, so reusing the same resource across frames would go quiet).
    fn press_key(world: &mut World, key: KeyCode) {
        let mut input = InputState::default();
        input.press(key);
        world.insert_resource(input);
    }

    #[test]
    fn clicking_the_plus_button_increments_and_emits() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(150.0, 70.0)); // + button (x 130..170)
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 6.0);
        assert_eq!(changed_events(&world), vec![(st, 6.0)]);
    }

    #[test]
    fn clicking_the_minus_button_decrements_and_emits() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 70.0)); // - button (x 50..90)
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 4.0);
        assert_eq!(changed_events(&world), vec![(st, 4.0)]);
    }

    #[test]
    fn clicking_the_center_value_area_does_nothing() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(110.0, 70.0)); // value area (x 90..130)
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 5.0);
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn plus_at_max_is_silent() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        world.get_mut::<Stepper>(st).unwrap().value = 10.0; // at max
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(150.0, 70.0));
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 10.0);
        assert!(
            changed_events(&world).is_empty(),
            "+ at max must not change the value or emit"
        );
    }

    #[test]
    fn minus_at_min_is_silent() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        world.get_mut::<Stepper>(st).unwrap().value = 0.0; // at min
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 70.0));
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 0.0);
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn covered_stepper_does_not_step() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        // Drop the widget below the panel bg (UiNode::new defaults z to 0.9; the panel bg draws at
        // 0.9 - 0.01 = 0.89, so the default would sit ON TOP of the panel).
        world.get_mut::<UiNode>(st).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(150.0, 70.0));
        assert_eq!(
            world.get::<Stepper>(st).unwrap().value,
            5.0,
            "a covered stepper must not receive the click"
        );
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn drag_off_the_widget_cancels_the_step() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        let mut system = UiSystem::default();

        // Press on the + button, drag off the widget, release: no change.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(150.0, 70.0));
            input.press_mouse(MouseButton::Left);
            input.set_cursor(Vec2::new(350.0, 280.0));
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 5.0);
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn press_plus_release_minus_uses_the_release_button() {
        // Press on +, then move to - and release: the release-point button (-) decides, so it
        // decrements. (Both press and release land on the same widget, so the click is owned.)
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        let mut system = UiSystem::default();

        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(150.0, 70.0)); // press on +
            input.press_mouse(MouseButton::Left);
            input.set_cursor(Vec2::new(60.0, 70.0)); // release on -
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 4.0);
        assert_eq!(changed_events(&world), vec![(st, 4.0)]);
    }

    #[test]
    fn focused_arrows_step_the_value_clamped() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(st);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 6.0);
        assert_eq!(changed_events(&world), vec![(st, 6.0)]);

        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowLeft);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 5.0);
        assert_eq!(changed_events(&world), vec![(st, 5.0)]);
    }

    #[test]
    fn focused_arrow_at_bound_is_silent() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        world.get_mut::<Stepper>(st).unwrap().value = 10.0; // at max
        world.resource_mut::<UiFocus>().unwrap().entity = Some(st);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<Stepper>(st).unwrap().value, 10.0);
        assert!(
            changed_events(&world).is_empty(),
            "a clamped step emits nothing"
        );
    }

    #[test]
    fn renders_bg_two_buttons_border_and_value_label() {
        let mut world = setup();
        let st = spawn_stepper(&mut world);
        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let q = world.resource::<UiQueue>().unwrap();
        // Exactly one border ring (border > 0 && corner_radius > 0).
        let rings = q
            .items
            .iter()
            .filter(|r| r.border > 0.0 && r.corner_radius > 0.0)
            .count();
        assert_eq!(rings, 1, "one border outline for the stepper");
        // Two button backgrounds in the button color.
        let btn = world.get::<Stepper>(st).unwrap().button_color;
        let buttons = q
            .items
            .iter()
            .filter(|r| r.color == btn && r.border == 0.0)
            .count();
        assert_eq!(buttons, 2, "the - and + button backgrounds");
        let texts = world.resource::<TextQueue>().unwrap();
        assert!(texts.iter().any(|t| t.text == "5"), "the value label");
        assert!(texts.iter().any(|t| t.text == "-"), "the - glyph");
        assert!(texts.iter().any(|t| t.text == "+"), "the + glyph");
    }

    #[test]
    fn degenerate_node_renders_bg_without_panicking() {
        let mut world = setup();
        let e = world.spawn();
        world.add_component(e, UiNode::new(10.0, 10.0, 0.0, 0.0));
        world.add_component(e, Stepper::new(0.0, 10.0, 5.0));
        let mut system = UiSystem::default();
        // Must complete without panicking (button_width == 0 → no buttons, still draws its bg).
        system.run(&mut world, 0.016);
        let q = world.resource::<UiQueue>().unwrap();
        assert!(!q.items.is_empty(), "still draws its bg/border");
    }
}
