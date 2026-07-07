use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::list_box::ListBox;
use crate::ui::node::UiNode;

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Horizontal inset in pixels of a row label from the list-box edges.
const LABEL_PAD: f32 = 8.0;

/// Handles every [`UiNode`] + [`ListBox`]: the mouse wheel scrolls the list under the pointer, and a
/// completed click on a *visible* row (press and release both owned by this widget through the
/// shared [`PointerCapture`], like a [`RadioGroup`](crate::ui::RadioGroup)) selects the row under the
/// **release** point and emits [`UiEvent::ListBoxChanged`] when the selection actually changed —
/// keeping the newly selected row scrolled into view. Renders a rounded background, an optional
/// border, the visible rows (selected fill / hover tint / label), all clipped to the node rect.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, ListBox>().map(|(e, _, _)| e));

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

        // The wheel scrolls only the list that owns the pointer (shared capture → a list covered by
        // another widget kind doesn't capture the wheel through it).
        if input.scroll_delta != 0.0 && hover_owner == Some(entity) {
            if let Some(lb) = world.get_mut::<ListBox>(entity) {
                lb.scroll_offset -= input.scroll_delta * lb.row_height;
                lb.clamp_scroll(size.y);
            }
        }

        // Select on release, like a RadioGroup/CheckBox click (only when both press and release land
        // on this widget — dragging onto another widget before releasing cancels).
        let clicked =
            input.just_released && pressed_owner == Some(entity) && released_owner == Some(entity);
        if clicked {
            if let Some(lb) = world.get_mut::<ListBox>(entity) {
                if let Some(row) = lb.row_at(input.release_cursor, pos, size) {
                    if row != lb.selected_index() {
                        lb.selected = row;
                        lb.scroll_to_selected(size.y);
                        output.events.push(UiEvent::ListBoxChanged(entity, row));
                    }
                }
            }
        }

        // ── Render (immutable reads now that state is settled) ────────────────
        let lb = match world.get::<ListBox>(entity) {
            Some(l) => l,
            None => continue,
        };

        // Background fill (rounded).
        output.rects.push(
            DrawRect::new(pos.x, pos.y, size.x, size.y, lb.bg_color)
                .with_corner_radius(lb.corner_radius)
                .with_z(z),
        );

        if lb.row_height > 0.0 && !lb.items.is_empty() {
            let selected = lb.selected_index();
            let hovered_row = if hover_owner == Some(entity) {
                lb.row_at(input.cursor, pos, size)
            } else {
                None
            };
            let first = (lb.scroll_offset / lb.row_height).floor().max(0.0) as usize;
            let visible_rows = (size.y / lb.row_height).ceil() as usize + 2;
            let last = (first + visible_rows).min(lb.items.len());

            for i in first..last {
                let row_top = pos.y + i as f32 * lb.row_height - lb.scroll_offset;
                // Clip the row's drawn band to the node rect so a partly-scrolled row never spills.
                let vis_top = row_top.max(pos.y);
                let vis_bot = (row_top + lb.row_height).min(pos.y + size.y);
                if vis_bot <= vis_top {
                    continue;
                }

                // Selected fill wins over the hover tint.
                let fill = if i == selected {
                    Some(lb.selected_color)
                } else if hovered_row == Some(i) {
                    Some(lb.hover_color)
                } else {
                    None
                };
                if let Some(color) = fill {
                    output.rects.push(
                        DrawRect::new(pos.x, vis_top, size.x, vis_bot - vis_top, color)
                            .with_z(z + super::UI_SUBLAYER_Z_STEP),
                    );
                }

                // Label. The text scissor's top edge equals the text position (see the text
                // renderer), so a row scrolled ABOVE the box top would spill upward — draw the label
                // only once its top has entered the box. The bottom partial row is clipped by the
                // bounds height. The selected/keyboard-stepped row is always scrolled fully in view.
                if row_top >= pos.y - 0.5 && !lb.items[i].is_empty() {
                    let bounds_h = (pos.y + size.y - row_top).min(lb.row_height).max(0.0);
                    output.texts.push(
                        DrawText::new(
                            lb.items[i].clone(),
                            Vec2::new(
                                pos.x + LABEL_PAD,
                                row_top + (lb.row_height - lb.font_size) / 2.0,
                            ),
                            lb.font_size,
                            lb.text_color,
                        )
                        .with_bounds(Vec2::new((size.x - LABEL_PAD * 2.0).max(0.0), bounds_h))
                        .with_z(z + super::UI_SUBLAYER_Z_STEP * 2.0),
                    );
                }
            }
        }

        // Border outline on top, framing the whole list.
        if lb.border > 0.0 {
            output.rects.push(
                DrawRect::new(pos.x, pos.y, size.x, size.y, lb.border_color)
                    .with_corner_radius(lb.corner_radius)
                    .with_border(lb.border)
                    .with_z(z + super::UI_SUBLAYER_Z_STEP * 3.0),
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
    use crate::ui::list_box::ListBox;
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

    /// A 6-row list (28-px rows) in a 200×84 node at (50, 50) — the node shows 3 whole rows, so the
    /// list scrolls. Rows (no scroll): 0 at y 50..78, 1 78..106, 2 106..134.
    fn spawn_list(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 200.0, 84.0));
        world.add_component(
            e,
            ListBox::new(["A", "B", "C", "D", "E", "F"]).with_row_height(28.0),
        );
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
                if let UiEvent::ListBoxChanged(en, i) = e {
                    Some((*en, *i))
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
    fn clicking_a_visible_row_selects_it_and_emits() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 90.0)); // row 1 ("B")
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(lb, 1)]);
    }

    #[test]
    fn reselecting_the_current_row_is_silent() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 60.0)); // row 0 = already selected
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 0);
        assert!(
            changed_events(&world).is_empty(),
            "no event when the selection did not change"
        );
    }

    #[test]
    fn clicking_a_scrolled_row_selects_the_right_index() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        // Scroll down two rows so row 2 sits at the top of the view.
        world.get_mut::<ListBox>(lb).unwrap().scroll_offset = 56.0;
        let mut system = UiSystem::default();

        // A click near the top of the node now lands on row 2, not row 0.
        click(&mut world, &mut system, Vec2::new(60.0, 60.0));
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 2);
        assert_eq!(changed_events(&world), vec![(lb, 2)]);
    }

    #[test]
    fn covered_list_box_does_not_select() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        // Drop the list below the panel bg (UiNode::new defaults z to 0.9; the panel bg draws at
        // 0.9 - 0.01 = 0.89, so the default would sit ON TOP of the panel).
        world.get_mut::<UiNode>(lb).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(60.0, 90.0));
        assert_eq!(
            world.get::<ListBox>(lb).unwrap().selected_index(),
            0,
            "a covered list box must not receive the click"
        );
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn drag_off_the_widget_cancels_the_selection() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        let mut system = UiSystem::default();

        // Press on row 1, drag off the widget, release: no selection change.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(60.0, 90.0));
            input.press_mouse(MouseButton::Left);
            input.set_cursor(Vec2::new(350.0, 280.0));
            input.release_mouse(MouseButton::Left);
        }
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 0);
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn wheel_scrolls_the_hovered_list_and_clamps() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        let mut system = UiSystem::default();

        // Wheel down (negative delta) over the list scrolls toward the bottom.
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(60.0, 90.0));
            input.add_scroll(-2.0);
        }
        system.run(&mut world, 0.016);
        let off = world.get::<ListBox>(lb).unwrap().scroll_offset;
        assert!(off > 0.0, "wheel-down scrolls the list, got {off}");

        // Wheel far past the bottom clamps to the max offset (6×28 − 84 = 84).
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.flush();
            input.set_cursor(Vec2::new(60.0, 90.0));
            input.add_scroll(-100.0);
        }
        system.run(&mut world, 0.016);
        assert!(
            (world.get::<ListBox>(lb).unwrap().scroll_offset - 84.0).abs() < f32::EPSILON,
            "scroll clamps to the max offset"
        );
    }

    #[test]
    fn wheel_does_not_scroll_a_non_hovered_list() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        let mut system = UiSystem::default();

        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.set_cursor(Vec2::new(350.0, 280.0)); // cursor off the list
            input.add_scroll(-5.0);
        }
        system.run(&mut world, 0.016);
        assert_eq!(
            world.get::<ListBox>(lb).unwrap().scroll_offset,
            0.0,
            "the wheel must not scroll a list the cursor is not over"
        );
    }

    #[test]
    fn focused_arrows_step_selection_and_scroll_into_view() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(lb);
        let mut system = UiSystem::default();

        // ArrowRight steps to row 1 and emits.
        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(lb, 1)]);

        // Jump selection to the last row and step once more: it scrolls into view.
        world.get_mut::<ListBox>(lb).unwrap().selected = 4;
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowRight); // → row 5 (last)
        system.run(&mut world, 0.016);
        let l = world.get::<ListBox>(lb).unwrap();
        assert_eq!(l.selected_index(), 5);
        assert!(
            l.scroll_offset > 0.0,
            "stepping to the last row scrolls it into view, got {}",
            l.scroll_offset
        );
    }

    #[test]
    fn focused_up_down_keys_step_selection() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        world.get_mut::<ListBox>(lb).unwrap().selected = 2;
        world.resource_mut::<UiFocus>().unwrap().entity = Some(lb);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::ArrowDown);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 3);
        assert_eq!(changed_events(&world), vec![(lb, 3)]);

        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowUp);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 2);
        assert_eq!(changed_events(&world), vec![(lb, 2)]);
    }

    #[test]
    fn step_at_the_bounds_clamps_silently() {
        let mut world = setup();
        let lb = spawn_list(&mut world); // selected = 0
        world.resource_mut::<UiFocus>().unwrap().entity = Some(lb);
        let mut system = UiSystem::default();

        // ArrowUp at the top row clamps (no wrap) and emits nothing.
        press_key(&mut world, KeyCode::ArrowUp);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<ListBox>(lb).unwrap().selected_index(), 0);
        assert!(
            changed_events(&world).is_empty(),
            "a clamped step emits nothing"
        );
    }

    #[test]
    fn rows_render_bg_border_and_one_selected_highlight() {
        let mut world = setup();
        let lb = spawn_list(&mut world);
        world.get_mut::<ListBox>(lb).unwrap().selected = 1;
        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let q = world.resource::<UiQueue>().unwrap();
        // Exactly one border ring (border > 0 && corner_radius > 0).
        let rings = q
            .items
            .iter()
            .filter(|r| r.border > 0.0 && r.corner_radius > 0.0)
            .count();
        assert_eq!(rings, 1, "one border outline for the list box");
        // The selected-row fill is drawn (a filled rect in the selected color).
        let sel = world.get::<ListBox>(lb).unwrap().selected_color;
        assert!(
            q.items.iter().any(|r| r.color == sel && r.border == 0.0),
            "the selected row's fill should be queued"
        );
        let texts = world.resource::<TextQueue>().unwrap();
        assert!(texts.iter().any(|t| t.text == "B"), "row labels queued");
    }

    #[test]
    fn empty_list_box_renders_bg_without_panicking() {
        let mut world = setup();
        let e = world.spawn();
        world.add_component(e, UiNode::new(10.0, 10.0, 200.0, 100.0));
        world.add_component(e, ListBox::new(Vec::<String>::new()));
        let mut system = UiSystem::default();
        // Must complete without panicking and still draw the background.
        system.run(&mut world, 0.016);
        let q = world.resource::<UiQueue>().unwrap();
        assert!(
            !q.items.is_empty(),
            "an empty list still draws its bg/border"
        );
    }
}
