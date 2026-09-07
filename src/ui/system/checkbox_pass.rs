use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::checkbox::CheckBox;
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
    scratch.extend(world.query2::<UiNode, CheckBox>().map(|(e, _, _)| e));

    // Toggle only when this checkbox owns the pointer at both press and release (shared capture →
    // a checkbox covered by another widget kind does not toggle through it).
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

        // Toggle on release, just like a Button (only when both press and release land on this box).
        // Dragging onto another widget before releasing cancels the toggle.
        let toggled =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);
        if toggled {
            if let Some(cb) = world.get_mut::<CheckBox>(entity) {
                cb.checked = !cb.checked;
                let checked = cb.checked;
                output
                    .events
                    .push(UiEvent::CheckBoxToggled(entity, checked));
            }
        }

        let (
            checked,
            box_size,
            border_col,
            checked_col,
            unchecked_col,
            label,
            text_color,
            font_size,
        ) = {
            let cb = match world.get::<CheckBox>(entity) {
                Some(c) => c,
                None => continue,
            };
            (
                cb.checked,
                cb.box_size,
                cb.border_color,
                cb.checked_color,
                cb.unchecked_color,
                cb.label.clone(),
                cb.text_color,
                cb.font_size,
            )
        };

        let box_y = pos.y + (size.y - box_size) / 2.0;
        let pad = 2.0;
        output
            .rects
            .push(DrawRect::new(pos.x, box_y, box_size, box_size, border_col).with_z(z));
        let inner_col = if checked { checked_col } else { unchecked_col };
        output.rects.push(
            DrawRect::new(
                pos.x + pad,
                box_y + pad,
                box_size - pad * 2.0,
                box_size - pad * 2.0,
                inner_col,
            )
            .with_z(z + super::UI_SUBLAYER_Z_STEP),
        );
        if !label.is_empty() {
            output.texts.push(
                DrawText::new(
                    label,
                    Vec2::new(pos.x + box_size + 6.0, pos.y + (size.y - font_size) / 2.0),
                    font_size,
                    text_color,
                )
                .with_bounds(Vec2::new((size.x - box_size - 6.0).max(0.0), size.y))
                .with_z(z + super::UI_SUBLAYER_Z_STEP),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;
    use winit::event::MouseButton;

    use crate::ecs::{Entity, Events, System, World};
    use crate::input::InputState;
    use crate::renderer::{TextQueue, UiQueue};
    use crate::resources::ViewportSize;
    use crate::ui::checkbox::CheckBox;
    use crate::ui::focus::UiFocus;
    use crate::ui::node::UiNode;
    use crate::ui::panel::{LayoutDir, Panel};
    use crate::ui::{UiEvent, UiSystem};

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

    /// A 120x30 checkbox at (50, 50).
    fn spawn_checkbox(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 120.0, 30.0));
        world.add_component(e, CheckBox::new("agree"));
        e
    }

    /// One frame with a press at `press` and a release at `release`.
    fn drag(world: &mut World, system: &mut UiSystem, press: Vec2, release: Vec2) {
        let input = world.resource_mut::<InputState>().unwrap();
        input.flush();
        input.set_cursor(press);
        input.press_mouse(MouseButton::Left);
        input.set_cursor(release);
        input.release_mouse(MouseButton::Left);
        system.run(world, 0.016);
    }

    fn toggles(world: &World) -> Vec<(Entity, bool)> {
        world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter_map(|e| match e {
                UiEvent::CheckBoxToggled(en, v) => Some((*en, *v)),
                _ => None,
            })
            .collect()
    }

    /// v0.156.29: `checkbox_pass::run` had no test at all — its whole body could be deleted with
    /// `cargo test` staying green. The only CheckBox test in the repo exercised the *focus* pass.
    #[test]
    fn clicking_a_checkbox_toggles_it_and_emits_once() {
        let mut world = setup();
        let cb = spawn_checkbox(&mut world);
        let mut system = UiSystem::default();

        drag(
            &mut world,
            &mut system,
            Vec2::new(60.0, 60.0),
            Vec2::new(60.0, 60.0),
        );
        assert!(world.get::<CheckBox>(cb).unwrap().checked);
        assert_eq!(toggles(&world), vec![(cb, true)]);
    }

    #[test]
    fn a_checkbox_covered_by_a_higher_z_panel_does_not_toggle() {
        let mut world = setup();
        let cb = spawn_checkbox(&mut world);
        world.get_mut::<UiNode>(cb).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        drag(
            &mut world,
            &mut system,
            Vec2::new(60.0, 60.0),
            Vec2::new(60.0, 60.0),
        );
        assert!(!world.get::<CheckBox>(cb).unwrap().checked);
        assert!(toggles(&world).is_empty());
    }

    #[test]
    fn pressing_on_a_checkbox_and_releasing_elsewhere_cancels_the_toggle() {
        let mut world = setup();
        let cb = spawn_checkbox(&mut world);
        let mut system = UiSystem::default();

        drag(
            &mut world,
            &mut system,
            Vec2::new(60.0, 60.0),
            Vec2::new(350.0, 250.0),
        );
        assert!(!world.get::<CheckBox>(cb).unwrap().checked);
        assert!(toggles(&world).is_empty());
    }

    #[test]
    fn a_checkbox_draws_a_box_a_fill_and_its_label() {
        let mut world = setup();
        spawn_checkbox(&mut world);
        UiSystem::default().run(&mut world, 0.016);

        assert_eq!(
            world.resource::<UiQueue>().unwrap().items.len(),
            2,
            "border box + inner fill"
        );
        assert!(world
            .resource::<TextQueue>()
            .unwrap()
            .iter()
            .any(|t| t.text == "agree"));
    }
}
