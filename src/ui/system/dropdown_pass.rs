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

/// Handles every [`UiNode`] + [`Dropdown`]: **pressing** the closed box opens the item list (below
/// the box, flipping above at the viewport bottom; opening on press — not on the completed click —
/// gives the native one-gesture press-drag-release flow), clicking an item selects it + closes +
/// emits [`UiEvent::DropdownChanged`] when the selection changed, and a press anywhere else closes
/// the list without selecting. Clicks resolve through the shared [`PointerCapture`] — while open,
/// the dropdown registers its whole expanded rect at [`DROPDOWN_LIST_Z`], so it wins the pointer
/// over (and its list draws over) everything underneath.
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
                // A hidden or empty dropdown can never stay open.
                dd.open = false;
                dd.press_opened = false;
            } else if dd.open {
                if clicked {
                    // A completed click on the expanded surface: on a row → select it (the row
                    // under the RELEASE point, so press-drag-release selects like a native
                    // combobox) and close; on the closed box → close, UNLESS this release ends
                    // the very press that opened the list (press-open + slow release must not
                    // immediately toggle it back shut).
                    let list_pos = dd.list_pos(pos, size, viewport.height);
                    let item_h = dd.resolved_item_height(size.y);
                    let list_size = Vec2::new(size.x, dd.list_height(size.y));
                    if item_h > 0.0 && in_bounds(input.release_cursor, list_pos, list_size) {
                        let row = ((input.release_cursor.y - list_pos.y) / item_h) as usize;
                        let row = row.min(dd.items.len() - 1);
                        let changed = row != dd.selected_index();
                        dd.selected = row;
                        dd.open = false;
                        dd.press_opened = false;
                        if changed {
                            output.events.push(UiEvent::DropdownChanged(entity, row));
                        }
                    } else if dd.press_opened {
                        dd.press_opened = false; // opening gesture ended on the box — stay open
                    } else {
                        dd.open = false;
                    }
                } else if input.just_pressed && pressed_owner != Some(entity) {
                    // Press-away closes without selecting (the press still reaches whatever
                    // surface it landed on — there is no modal grab). Checked BEFORE the bare
                    // release cleanup below, or a same-frame press+release away would only
                    // clear the flag and leave the list open.
                    dd.open = false;
                    dd.press_opened = false;
                } else if input.just_released {
                    // The opening gesture ended somewhere off the widget — stays open, but the
                    // next click on the box closes it normally.
                    dd.press_opened = false;
                }
            } else if input.just_pressed && pressed_owner == Some(entity) {
                // Open on PRESS (not on the completed click) so the native one-gesture flow
                // works: press the box, drag onto a row, release to select. If the release
                // already landed in this same frame, the gesture is over — a later box click
                // must close, not be swallowed as the opening release.
                dd.open = true;
                dd.press_opened = !input.just_released;
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
            output.texts.push(
                DrawText::new(
                    item.to_string(),
                    Vec2::new(pos.x + TEXT_PAD_X, text_y),
                    dd.font_size,
                    dd.text_color,
                )
                .with_z(box_z),
            );
        }
        output.texts.push(
            DrawText::new(
                if dd.open { "▲" } else { "▼" },
                Vec2::new(pos.x + size.x - dd.font_size - TEXT_PAD_X, text_y),
                dd.font_size,
                dd.text_color,
            )
            .with_z(box_z),
        );

        // Open list: ONE rounded background for the whole list (per-row rounded rects leave
        // notch seams between stacked corners), then a separate non-rounded highlight for the
        // row under the cursor, inset by the corner radius so its square corners never poke
        // out of the background's rounding on the first/last row.
        if dd.open {
            let list_pos = dd.list_pos(pos, size, viewport.height);
            let item_h = dd.resolved_item_height(size.y);
            let list_h = dd.list_height(size.y);
            let hovered_row = if item_h > 0.0
                && hovered
                && in_bounds(input.cursor, list_pos, Vec2::new(size.x, list_h))
            {
                Some(((input.cursor.y - list_pos.y) / item_h) as usize)
            } else {
                None
            };
            output.rects.push(
                DrawRect::new(list_pos.x, list_pos.y, size.x, list_h, dd.bg_color)
                    .with_corner_radius(dd.corner_radius)
                    .with_z(DROPDOWN_LIST_Z + super::UI_SUBLAYER_Z_STEP),
            );
            if let Some(row) = hovered_row {
                let inset = dd.corner_radius.clamp(0.0, size.x.min(list_h) / 2.0);
                output.rects.push(
                    DrawRect::new(
                        list_pos.x + inset,
                        list_pos.y + row as f32 * item_h,
                        size.x - 2.0 * inset,
                        item_h,
                        dd.hover_color,
                    )
                    .with_z(DROPDOWN_LIST_Z + 2.0 * super::UI_SUBLAYER_Z_STEP),
                );
            }
            for (i, item) in dd.items.iter().enumerate() {
                let row_y = list_pos.y + i as f32 * item_h;
                let marker = if i == dd.selected_index() { "• " } else { "" };
                // At the highlight's z: an equal-z tie renders text over the surface, so the
                // hovered row's label stays visible above its highlight.
                output.texts.push(
                    DrawText::new(
                        format!("{marker}{item}"),
                        Vec2::new(pos.x + TEXT_PAD_X, row_y + (item_h - dd.font_size) / 2.0),
                        dd.font_size,
                        dd.text_color,
                    )
                    .with_z(DROPDOWN_LIST_Z + 2.0 * super::UI_SUBLAYER_Z_STEP),
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
    fn press_drag_release_selects_in_one_gesture() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        let mut system = UiSystem::default();

        // Frame 1: press on the closed box (no release yet) → the list opens immediately.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.flush();
            input.set_cursor(Vec2::new(60.0, 60.0));
            input.press_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert!(
            world.get::<Dropdown>(dd).unwrap().open,
            "the list opens on press, not on the completed click"
        );

        // Frame 2: still holding, drag onto row 2 ("High", rows open at y 80/110/140) and release.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.flush();
            input.set_cursor(Vec2::new(60.0, 155.0));
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        let d = world.get::<Dropdown>(dd).unwrap();
        assert_eq!(d.selected_index(), 2, "release row selected in one gesture");
        assert!(!d.open);
        assert_eq!(changed_events(&world), vec![(dd, 2)]);
    }

    #[test]
    fn opening_press_released_on_the_box_keeps_it_open_then_click_closes() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        let mut system = UiSystem::default();

        // Frame 1: press on the box → opens.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.flush();
            input.set_cursor(Vec2::new(60.0, 60.0));
            input.press_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert!(world.get::<Dropdown>(dd).unwrap().open);

        // Frame 2: release still on the box — the opening gesture must NOT toggle it back shut.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.flush();
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert!(
            world.get::<Dropdown>(dd).unwrap().open,
            "the opening press's own release keeps the list open"
        );

        // Frame 3: a fresh click on the box closes it.
        click(&mut world, &mut system, Vec2::new(60.0, 60.0));
        assert!(
            !world.get::<Dropdown>(dd).unwrap().open,
            "a later click on the box closes the list"
        );
    }

    #[test]
    fn enter_opening_one_dropdown_closes_another() {
        let mut world = setup();
        let first = spawn_dropdown(&mut world);
        // A second dropdown well away from the first.
        let second = world.spawn();
        world.add_component(second, UiNode::new(250.0, 50.0, 120.0, 30.0));
        world.add_component(second, Dropdown::new(["x", "y"]));
        let mut system = UiSystem::default();

        // Open the first with the pointer.
        click(&mut world, &mut system, Vec2::new(60.0, 60.0));
        assert!(world.get::<Dropdown>(first).unwrap().open);

        // Focus the second and open it with Enter → the first must close.
        world.resource_mut::<UiFocus>().unwrap().entity = Some(second);
        press_key(&mut world, KeyCode::Enter);
        system.run(&mut world, 0.016);
        assert!(
            world.get::<Dropdown>(second).unwrap().open,
            "Enter opens the focused one"
        );
        assert!(
            !world.get::<Dropdown>(first).unwrap().open,
            "opening via Enter closes the other open list"
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
        // Closed box redrawn at the list z + the one-piece list background above it.
        assert!(
            rows >= 2,
            "expected the box + the list background at the list z, got {rows}"
        );
    }

    #[test]
    fn open_list_draws_one_background_plus_an_inset_hover_highlight() {
        let mut world = setup();
        let dd = spawn_dropdown(&mut world);
        world.get_mut::<Dropdown>(dd).unwrap().corner_radius = 6.0;
        let mut system = UiSystem::default();
        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // open
        assert!(world.get::<Dropdown>(dd).unwrap().open);

        // Hover row 1 ("Medium", y 110..140) without clicking. Drop the opening frame's
        // queued rects first — the test reads the queue raw, frame.rs normally drains it.
        world.resource_mut::<UiQueue>().unwrap().items.clear();
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.flush();
            input.set_cursor(Vec2::new(60.0, 125.0));
        }
        system.run(&mut world, 0.016);

        let queue = world.resource::<UiQueue>().unwrap();
        let mut list_rects: Vec<_> = queue
            .items
            .iter()
            .filter(|r| r.z > DROPDOWN_LIST_Z)
            .collect();
        list_rects.sort_by(|a, b| a.z.total_cmp(&b.z));
        // ONE full-list rounded background + ONE hover highlight — NOT one rounded rect per
        // row (stacked rounded rows leave seam notches between their corners).
        assert_eq!(
            list_rects.len(),
            2,
            "expected background + highlight, got {}",
            list_rects.len()
        );
        let bg = list_rects[0];
        assert_eq!((bg.x, bg.y, bg.w, bg.h), (50.0, 80.0, 120.0, 90.0));
        assert_eq!(bg.corner_radius, 6.0);
        let hl = list_rects[1];
        assert_eq!(
            (hl.x, hl.y, hl.w, hl.h),
            (56.0, 110.0, 108.0, 30.0),
            "highlight inset by the corner radius on the hovered row"
        );
        assert_eq!(
            hl.corner_radius, 0.0,
            "highlight is square — it sits inside the background's rounding"
        );
        assert!(hl.z > bg.z, "highlight draws over the background");
    }
}
