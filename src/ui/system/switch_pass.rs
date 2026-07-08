use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::switch::Switch;

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Handles every [`UiNode`] + [`Switch`]: a completed click on the widget (press and release both
/// owned by this switch through the shared [`PointerCapture`], like a
/// [`CheckBox`](crate::ui::CheckBox) toggle) flips `on` and emits [`UiEvent::SwitchToggled`] with the
/// new state; dragging off before releasing cancels. Renders the pill track (colored by state), the
/// sliding round knob, and the optional label.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Switch>().map(|(e, _, _)| e));

    // Toggle only when this switch owns the pointer at both press and release (shared capture → a
    // switch covered by another widget kind does not toggle through it).
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

        // Toggle on release, like a Button/CheckBox click (only when both press and release land on
        // this widget — dragging onto another widget before releasing cancels).
        let toggled =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);
        if toggled {
            if let Some(sw) = world.get_mut::<Switch>(entity) {
                sw.on = !sw.on;
                let on = sw.on;
                output.events.push(UiEvent::SwitchToggled(entity, on));
            }
        }

        // ── Render (immutable reads now that state is settled) ────────────────
        let sw = match world.get::<Switch>(entity) {
            Some(s) => s,
            None => continue,
        };
        let step_z = super::UI_SUBLAYER_Z_STEP;

        // Track (pill), colored by state.
        let (track_pos, track_size) = sw.track_rect(pos, size);
        output.rects.push(
            DrawRect::new(
                track_pos.x,
                track_pos.y,
                track_size.x,
                track_size.y,
                sw.track_color(),
            )
            .with_corner_radius(sw.track_radius(size))
            .with_z(z),
        );

        // Sliding round knob, just above the track.
        let (knob_pos, knob_size) = sw.knob_rect(pos, size);
        output.rects.push(
            DrawRect::new(
                knob_pos.x,
                knob_pos.y,
                knob_size.x,
                knob_size.y,
                sw.knob_color,
            )
            .with_corner_radius(sw.knob_radius(size))
            .with_z(z + step_z),
        );

        // Label to the right of the track.
        if !sw.label.is_empty() {
            let label_x = track_pos.x + track_size.x + 8.0;
            output.texts.push(
                DrawText::new(
                    sw.label.clone(),
                    Vec2::new(label_x, pos.y + (size.y - sw.font_size) / 2.0),
                    sw.font_size,
                    sw.text_color,
                )
                .with_bounds(Vec2::new((size.x - track_size.x - 8.0).max(0.0), size.y))
                .with_z(z + step_z),
            );
        }
    }
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
    use crate::ui::switch::Switch;
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

    /// A switch in a 160×40 node at (50, 50).
    fn spawn_switch(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 160.0, 40.0));
        world.add_component(e, Switch::new("Sound"));
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

    fn toggle_events(world: &World) -> Vec<(Entity, bool)> {
        world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter_map(|e| {
                if let UiEvent::SwitchToggled(en, on) = e {
                    Some((*en, *on))
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
    fn clicking_toggles_on_and_emits() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(70.0, 70.0));
        assert!(world.get::<Switch>(sw).unwrap().on);
        assert_eq!(toggle_events(&world), vec![(sw, true)]);
    }

    #[test]
    fn clicking_again_toggles_back_off() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        world.get_mut::<Switch>(sw).unwrap().on = true;
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(70.0, 70.0));
        assert!(!world.get::<Switch>(sw).unwrap().on);
        assert_eq!(toggle_events(&world), vec![(sw, false)]);
    }

    #[test]
    fn clicking_the_label_area_also_toggles() {
        // The whole node is the click surface (like CheckBox) — clicking to the right of the track,
        // over the label, still toggles.
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(180.0, 70.0)); // node is x 50..210, track ~50..96
        assert!(world.get::<Switch>(sw).unwrap().on);
        assert_eq!(toggle_events(&world), vec![(sw, true)]);
    }

    #[test]
    fn covered_switch_does_not_toggle() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        // Drop the widget below the panel bg (UiNode::new defaults z to 0.9; the panel bg draws at
        // 0.9 - 0.01 = 0.89, so the default would sit ON TOP of the panel).
        world.get_mut::<UiNode>(sw).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(70.0, 70.0));
        assert!(
            !world.get::<Switch>(sw).unwrap().on,
            "a covered switch must not receive the click"
        );
        assert!(toggle_events(&world).is_empty());
    }

    #[test]
    fn drag_off_the_widget_cancels_the_toggle() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        let mut system = UiSystem::default();

        // Press on the switch, drag off the widget, release: no toggle.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(70.0, 70.0));
            input.press_mouse(MouseButton::Left);
            input.set_cursor(Vec2::new(350.0, 280.0));
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert!(!world.get::<Switch>(sw).unwrap().on);
        assert!(toggle_events(&world).is_empty());
    }

    #[test]
    fn focused_space_toggles_and_emits() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(sw);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::Space);
        system.run(&mut world, 0.016);
        assert!(world.get::<Switch>(sw).unwrap().on);
        assert_eq!(toggle_events(&world), vec![(sw, true)]);
    }

    #[test]
    fn focused_right_turns_on_left_turns_off() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(sw);
        let mut system = UiSystem::default();

        // → turns it on.
        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert!(world.get::<Switch>(sw).unwrap().on);
        assert_eq!(toggle_events(&world), vec![(sw, true)]);

        // ← turns it off.
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowLeft);
        system.run(&mut world, 0.016);
        assert!(!world.get::<Switch>(sw).unwrap().on);
        assert_eq!(toggle_events(&world), vec![(sw, false)]);
    }

    #[test]
    fn focused_right_when_already_on_is_silent() {
        // ← / → set the state absolutely (off / on); pressing → when already on changes nothing and
        // must stay silent (emit-on-change).
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        world.get_mut::<Switch>(sw).unwrap().on = true;
        world.resource_mut::<UiFocus>().unwrap().entity = Some(sw);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert!(world.get::<Switch>(sw).unwrap().on);
        assert!(
            toggle_events(&world).is_empty(),
            "→ on an already-on switch must not emit"
        );
    }

    #[test]
    fn renders_track_knob_and_label() {
        let mut world = setup();
        let sw = spawn_switch(&mut world);
        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let q = world.resource::<UiQueue>().unwrap();
        let off = world.get::<Switch>(sw).unwrap().off_color;
        let knob = world.get::<Switch>(sw).unwrap().knob_color;
        assert!(
            q.items
                .iter()
                .any(|r| r.color == off && r.corner_radius > 0.0),
            "the off-state track pill"
        );
        assert!(
            q.items
                .iter()
                .any(|r| r.color == knob && r.corner_radius > 0.0),
            "the round knob"
        );
        let texts = world.resource::<TextQueue>().unwrap();
        assert!(texts.iter().any(|t| t.text == "Sound"), "the label");
    }

    #[test]
    fn degenerate_node_renders_without_panicking() {
        let mut world = setup();
        let e = world.spawn();
        world.add_component(e, UiNode::new(10.0, 10.0, 0.0, 0.0));
        world.add_component(e, Switch::new("x"));
        let mut system = UiSystem::default();
        // Must complete without panicking (zero-size node → clamped geometry, still draws).
        system.run(&mut world, 0.016);
        let q = world.resource::<UiQueue>().unwrap();
        assert!(!q.items.is_empty(), "still draws its track");
    }
}
