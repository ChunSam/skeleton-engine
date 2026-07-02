use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::dropdown::{Dropdown, DROPDOWN_LIST_Z};
use crate::ui::node::UiNode;

use super::capture::PointerCapture;
use super::state::{in_bounds, node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Horizontal inset for the closed box's item text and the ▼/▲ arrow.
const TEXT_PAD_X: f32 = 8.0;

/// Handles every [`UiNode`] + [`Dropdown`]: clicking the closed box opens the item list (below the
/// box, flipping above at the viewport bottom), clicking an item selects it + closes + emits
/// [`UiEvent::DropdownChanged`] when the selection changed, and a press anywhere else closes the
/// list without selecting. Clicks resolve through the shared [`PointerCapture`] — while open, the
/// dropdown registers its whole expanded rect at [`DROPDOWN_LIST_Z`], so it wins the pointer over
/// (and its rows draw over) everything underneath. Press-drag-release onto a row also selects,
/// matching native comboboxes.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Dropdown>().map(|(e, _, _)| e));

    let hover_owner = capture.topmost_at(input.cursor);
    let pressed_owner = capture.topmost_at(input.press_cursor);
    let released_owner = capture.topmost_at(input.release_cursor);

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };
        let clicked =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);

        // ── Interaction ───────────────────────────────────────────────────────
        {
            let dd = match world.get_mut::<Dropdown>(entity) {
                Some(d) => d,
                None => continue,
            };
            if !visible || dd.items.is_empty() {
                dd.open = false; // a hidden or empty dropdown can never stay open
            } else if dd.open {
                if clicked {
                    // A completed click on the expanded surface: on the closed box → just close;
                    // on a row → select it (the row under the RELEASE point, so press-drag-release
                    // selects like a native combobox) and close.
                    let list_pos = dd.list_pos(pos, size, viewport.height);
                    let item_h = dd.resolved_item_height(size.y);
                    let list_size = Vec2::new(size.x, dd.list_height(size.y));
                    if item_h > 0.0 && in_bounds(input.release_cursor, list_pos, list_size) {
                        let row = ((input.release_cursor.y - list_pos.y) / item_h) as usize;
                        let row = row.min(dd.items.len() - 1);
                        let changed = row != dd.selected_index();
                        dd.selected = row;
                        dd.open = false;
                        if changed {
                            output.events.push(UiEvent::DropdownChanged(entity, row));
                        }
                    } else {
                        dd.open = false;
                    }
                } else if input.just_pressed && pressed_owner != Some(entity) {
                    // Press-away closes without selecting (the press still reaches whatever
                    // surface it landed on — there is no modal grab).
                    dd.open = false;
                }
            } else if clicked {
                dd.open = true;
            }
        }

        // ── Render (immutable reads now that state is settled) ────────────────
        if !visible {
            continue;
        }
        let dd = match world.get::<Dropdown>(entity) {
            Some(d) => d,
            None => continue,
        };
        let hovered = hover_owner == Some(entity);

        // Closed box: background (hover-tinted), selected item text, and an open/close arrow.
        let box_color = if hovered { dd.hover_color } else { dd.bg_color };
        let box_z = if dd.open { DROPDOWN_LIST_Z } else { z };
        output.rects.push(
            DrawRect::new(pos.x, pos.y, size.x, size.y, box_color)
                .with_corner_radius(dd.corner_radius)
                .with_z(box_z),
        );
        let text_y = pos.y + (size.y - dd.font_size) / 2.0;
        if let Some(item) = dd.selected_item() {
            output.texts.push(DrawText::new(
                item.to_string(),
                Vec2::new(pos.x + TEXT_PAD_X, text_y),
                dd.font_size,
                dd.text_color,
            ));
        }
        output.texts.push(DrawText::new(
            if dd.open { "▲" } else { "▼" },
            Vec2::new(pos.x + size.x - dd.font_size - TEXT_PAD_X, text_y),
            dd.font_size,
            dd.text_color,
        ));

        // Open list: one row per item, the row under the cursor highlighted.
        if dd.open {
            let list_pos = dd.list_pos(pos, size, viewport.height);
            let item_h = dd.resolved_item_height(size.y);
            let hovered_row = if item_h > 0.0
                && hovered
                && in_bounds(
                    input.cursor,
                    list_pos,
                    Vec2::new(size.x, dd.list_height(size.y)),
                ) {
                Some(((input.cursor.y - list_pos.y) / item_h) as usize)
            } else {
                None
            };
            for (i, item) in dd.items.iter().enumerate() {
                let row_y = list_pos.y + i as f32 * item_h;
                let row_color = if hovered_row == Some(i) {
                    dd.hover_color
                } else {
                    dd.bg_color
                };
                output.rects.push(
                    DrawRect::new(pos.x, row_y, size.x, item_h, row_color)
                        .with_corner_radius(dd.corner_radius)
                        .with_z(DROPDOWN_LIST_Z + super::UI_SUBLAYER_Z_STEP),
                );
                let marker = if i == dd.selected_index() { "• " } else { "" };
                output.texts.push(DrawText::new(
                    format!("{marker}{item}"),
                    Vec2::new(pos.x + TEXT_PAD_X, row_y + (item_h - dd.font_size) / 2.0),
                    dd.font_size,
                    dd.text_color,
                ));
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
    use crate::ui::button::Button;
    use crate::ui::dropdown::{Dropdown, DROPDOWN_LIST_Z};
    use crate::ui::focus::UiFocus;
    use crate::ui::node::UiNode;
    use crate::ui::panel::{LayoutDir, Panel};
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

    /// A 3-item dropdown in a 120×30 node at (50, 50). Rows open at y = 80/110/140.
    fn spawn_dropdown(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 120.0, 30.0));
        world.add_component(e, Dropdown::new(["Low", "Medium", "High"]));
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
                if let UiEvent::DropdownChanged(en, i) = e {
                    Some((*en, *i))
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn click_opens_then_row_click_selects_and_emits() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // on the closed box
        assert!(world.get::<Dropdown>(dd).unwrap().open, "click opens");

        click(&mut world, &mut system, Vec2::new(60.0, 125.0)); // row 1 ("Medium", y 110..140)
        let d = world.get::<Dropdown>(dd).unwrap();
        assert!(!d.open, "selecting closes the list");
        assert_eq!(d.selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(dd, 1)]);
    }

    #[test]
    fn reselecting_the_same_item_closes_without_an_event() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // open
        click(&mut world, &mut system, Vec2::new(60.0, 95.0)); // row 0 = already selected
        assert!(!world.get::<Dropdown>(dd).unwrap().open);
        assert!(
            changed_events(&world).is_empty(),
            "no event when the selection did not change"
        );
    }

    #[test]
    fn press_away_closes_without_selecting() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // open
        click(&mut world, &mut system, Vec2::new(300.0, 250.0)); // far away
        let d = world.get::<Dropdown>(dd).unwrap();
        assert!(!d.open, "press-away closes");
        assert_eq!(d.selected_index(), 0, "selection unchanged");
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn open_list_occludes_a_button_underneath() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        // A button sitting exactly under row 1 of the open list (list rows y 80..170, z 0.5).
        let btn = world.spawn();
        let mut node = UiNode::new(50.0, 110.0, 120.0, 30.0);
        node.z = 0.5;
        world.add_component(btn, node);
        world.add_component(btn, Button::new("under"));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // open the list over the button
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        click(&mut world, &mut system, Vec2::new(60.0, 125.0)); // click "through" row 1

        let events = world.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, UiEvent::ButtonClicked(b) if *b == btn)),
            "the open list must absorb the click — got {events:?}"
        );
        assert_eq!(
            world.get::<Dropdown>(dd).unwrap().selected_index(),
            1,
            "the dropdown row took the click"
        );

        // Control: with the list closed, the same button click fires normally.
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        click(&mut world, &mut system, Vec2::new(60.0, 125.0));
        let events = world.resource::<Events<UiEvent>>().unwrap().read().to_vec();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, UiEvent::ButtonClicked(b) if *b == btn)),
            "closed dropdown must not block the button — got {events:?}"
        );
    }

    #[test]
    fn bottom_edge_dropdown_opens_upward() {
        let mut world = setup();
        let e = world.spawn();
        // Node at y=260 in a 300-tall viewport; a 90-tall list below would overflow → flips above.
        world.add_component(e, UiNode::new(50.0, 260.0, 120.0, 30.0));
        world.add_component(e, Dropdown::new(["a", "b", "c"]));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 270.0)); // open
        assert!(world.get::<Dropdown>(e).unwrap().open);

        // Rows sit above the box: y 170..260. Click the first row ("a" → index 0 is selected;
        // click row 2 = "c" at y 230..260? No — rows top-down: row 0 at 170, row 2 at 230.)
        click(&mut world, &mut system, Vec2::new(60.0, 245.0)); // row 2 ("c")
        let d = world.get::<Dropdown>(e).unwrap();
        assert_eq!(d.selected_index(), 2, "flipped list rows resolve top-down");
        assert!(!d.open);
    }

    #[test]
    fn covered_dropdown_does_not_open() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        // Drop the widget below the panel bg (UiNode::new defaults z to 0.9; the panel bg draws at
        // 0.9 - 0.01 = 0.89, so the default would sit ON TOP of the panel).
        world.get_mut::<UiNode>(dd).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 60.0));
        assert!(
            !world.get::<Dropdown>(dd).unwrap().open,
            "a covered dropdown must not receive the click"
        );
    }

    /// Inserts a fresh `InputState` with `key` just-pressed (a held key never re-fires
    /// `just_pressed`, so reusing the same resource across frames would go quiet).
    fn press_key(world: &mut World, key: KeyCode) {
        let mut input = InputState::default();
        input.press(key);
        world.insert_resource(input);
    }

    #[test]
    fn focused_dropdown_enter_toggles_and_arrows_step_selection() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(dd);
        let mut system = UiSystem::default();

        // ArrowRight steps the selection without opening.
        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        let d = world.get::<Dropdown>(dd).unwrap();
        assert_eq!(d.selected_index(), 1);
        assert!(!d.open, "arrow-stepping does not open the list");
        assert_eq!(changed_events(&world), vec![(dd, 1)]);

        // At the last item, ArrowRight clamps (no wrap, like a Slider) — and no event.
        world.get_mut::<Dropdown>(dd).unwrap().selected = 2;
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<Dropdown>(dd).unwrap().selected_index(), 2);
        assert!(
            changed_events(&world).is_empty(),
            "clamped step emits nothing"
        );

        // Enter toggles the list open, Enter again closes.
        press_key(&mut world, KeyCode::Enter);
        system.run(&mut world, 0.016);
        assert!(world.get::<Dropdown>(dd).unwrap().open, "Enter opens");
        press_key(&mut world, KeyCode::Enter);
        system.run(&mut world, 0.016);
        assert!(
            !world.get::<Dropdown>(dd).unwrap().open,
            "Enter again closes"
        );
    }

    #[test]
    fn open_list_rows_reach_the_ui_queue_at_the_list_z() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        let mut system = UiSystem::default();
        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // open
        assert!(world.get::<Dropdown>(dd).unwrap().open);

        let rows = world
            .resource::<UiQueue>()
            .unwrap()
            .items
            .iter()
            .filter(|r| r.z >= DROPDOWN_LIST_Z)
            .count();
        // Closed box redrawn at the list z + 3 rows above it.
        assert!(
            rows >= 4,
            "expected the box + 3 rows at the list z, got {rows}"
        );
    }
}
