use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::radio_group::RadioGroup;

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Outline thickness of the option ring in pixels.
const RING_THICKNESS: f32 = 2.0;
/// Inset fraction of the circle diameter for the selected inner dot (0.3 → dot = 0.4 × diameter).
const DOT_INSET_FRAC: f32 = 0.3;
/// Gap in pixels between the option circle and its label.
const LABEL_GAP: f32 = 6.0;

/// Handles every [`UiNode`] + [`RadioGroup`]: a completed click on an option row (press and
/// release both owned by this widget through the shared [`PointerCapture`], like a
/// [`CheckBox`](crate::ui::CheckBox) toggle) selects the row under the **release** point and emits
/// [`UiEvent::RadioChanged`] when the selection actually changed. Each row renders a ring
/// (SDF-rounded rect at full corner radius = a circle), a filled dot on the selected row, the
/// option label, and a subtle hover tint under the cursor's row.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, RadioGroup>().map(|(e, _, _)| e));

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

        // Select on release, like a Button/CheckBox click (only when both press and release land
        // on this widget — dragging onto another widget before releasing cancels).
        let clicked =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);
        if clicked {
            if let Some(rg) = world.get_mut::<RadioGroup>(entity) {
                if let Some(row) = rg.row_at(input.release_cursor, pos, size) {
                    if row != rg.selected_index() {
                        rg.selected = row;
                        output.events.push(UiEvent::RadioChanged(entity, row));
                    }
                }
            }
        }

        // ── Render (immutable reads now that state is settled) ────────────────
        let rg = match world.get::<RadioGroup>(entity) {
            Some(r) => r,
            None => continue,
        };
        let item_h = rg.resolved_item_height(size.y);
        if rg.items.is_empty() || item_h <= 0.0 {
            continue;
        }
        let hovered_row = if hover_owner == Some(entity) {
            rg.row_at(input.cursor, pos, size)
        } else {
            None
        };

        let d = rg.circle_size.min(item_h);
        for (i, item) in rg.items.iter().enumerate() {
            let row_y = pos.y + i as f32 * item_h;
            if hovered_row == Some(i) {
                output
                    .rects
                    .push(DrawRect::new(pos.x, row_y, size.x, item_h, rg.hover_color).with_z(z));
            }
            // Ring: an SDF rounded-rect outline at full corner radius = a circle outline.
            let cy = row_y + (item_h - d) / 2.0;
            output.rects.push(
                DrawRect::new(pos.x, cy, d, d, rg.circle_color)
                    .with_corner_radius(d / 2.0)
                    .with_border(RING_THICKNESS)
                    .with_z(z + super::UI_SUBLAYER_Z_STEP),
            );
            if i == rg.selected_index() {
                let inset = d * DOT_INSET_FRAC;
                let dot = d - inset * 2.0;
                output.rects.push(
                    DrawRect::new(pos.x + inset, cy + inset, dot, dot, rg.fill_color)
                        .with_corner_radius(dot / 2.0)
                        .with_z(z + super::UI_SUBLAYER_Z_STEP * 2.0),
                );
            }
            if !item.is_empty() {
                output.texts.push(
                    DrawText::new(
                        item.clone(),
                        Vec2::new(pos.x + d + LABEL_GAP, row_y + (item_h - rg.font_size) / 2.0),
                        rg.font_size,
                        rg.text_color,
                    )
                    .with_bounds(Vec2::new((size.x - d - LABEL_GAP).max(0.0), item_h))
                    .with_z(z + super::UI_SUBLAYER_Z_STEP),
                );
            }
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
    use crate::ui::radio_group::RadioGroup;
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

    /// A 3-option group in a 140×90 node at (50, 50) → rows at y 50..80, 80..110, 110..140.
    fn spawn_radio(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 140.0, 90.0));
        world.add_component(e, RadioGroup::new(["Easy", "Normal", "Hard"]));
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

    fn changed_events(world: &World) -> Vec<(Entity, usize)> {
        world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter_map(|e| {
                if let UiEvent::RadioChanged(en, i) = e {
                    Some((*en, *i))
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn clicking_a_row_selects_it_and_emits() {
        let mut world = setup();
        let rg = spawn_radio(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 95.0)); // row 1 ("Normal")
        let r = world.get::<RadioGroup>(rg).unwrap();
        assert_eq!(r.selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(rg, 1)]);
    }

    #[test]
    fn reselecting_the_current_row_is_silent() {
        let mut world = setup();
        let rg = spawn_radio(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 65.0)); // row 0 = already selected
        assert_eq!(world.get::<RadioGroup>(rg).unwrap().selected_index(), 0);
        assert!(
            changed_events(&world).is_empty(),
            "no event when the selection did not change"
        );
    }

    #[test]
    fn covered_radio_group_does_not_select() {
        let mut world = setup();
        let rg = spawn_radio(&mut world);
        // Drop the widget below the panel bg (UiNode::new defaults z to 0.9; the panel bg draws
        // at 0.9 - 0.01 = 0.89, so the default would sit ON TOP of the panel).
        world.get_mut::<UiNode>(rg).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 95.0));
        assert_eq!(
            world.get::<RadioGroup>(rg).unwrap().selected_index(),
            0,
            "a covered radio group must not receive the click"
        );
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn drag_off_the_widget_cancels_the_selection() {
        let mut world = setup();
        let rg = spawn_radio(&mut world);
        let mut system = UiSystem::default();

        // Press on row 1, drag off the widget, release: no selection change.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(60.0, 95.0));
            input.press_mouse(MouseButton::Left);
            input.set_cursor(Vec2::new(300.0, 250.0));
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<RadioGroup>(rg).unwrap().selected_index(), 0);
        assert!(changed_events(&world).is_empty());
    }

    /// Inserts a fresh `InputState` with `key` just-pressed (a held key never re-fires
    /// `just_pressed`, so reusing the same resource across frames would go quiet).
    fn press_key(world: &mut World, key: KeyCode) {
        let mut input = InputState::default();
        input.press(key);
        world.insert_resource(input);
    }

    #[test]
    fn focused_arrows_step_selection_clamped() {
        let mut world = setup();
        let rg = spawn_radio(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(rg);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<RadioGroup>(rg).unwrap().selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(rg, 1)]);

        // At the last option, ArrowRight clamps (no wrap) — and no event.
        world.get_mut::<RadioGroup>(rg).unwrap().selected = 2;
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<RadioGroup>(rg).unwrap().selected_index(), 2);
        assert!(
            changed_events(&world).is_empty(),
            "clamped step emits nothing"
        );

        press_key(&mut world, KeyCode::ArrowLeft);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<RadioGroup>(rg).unwrap().selected_index(), 1);
    }

    #[test]
    fn rows_render_rings_and_one_dot() {
        let mut world = setup();
        spawn_radio(&mut world);
        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let q = world.resource::<UiQueue>().unwrap();
        let rings = q
            .items
            .iter()
            .filter(|r| r.border > 0.0 && r.corner_radius > 0.0)
            .count();
        assert_eq!(rings, 3, "one outline ring per option");
        let dots = q
            .items
            .iter()
            .filter(|r| r.border == 0.0 && r.corner_radius > 0.0)
            .count();
        assert_eq!(dots, 1, "exactly one selected dot");
        let texts = world.resource::<TextQueue>().unwrap();
        assert!(texts.iter().any(|t| t.text == "Normal"), "labels queued");
    }
}
