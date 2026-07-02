use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText, TextAlign};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::tab_bar::TabBar;

use super::capture::PointerCapture;
use super::state::{node_layout, InputSnapshot, UiOutput};
use super::UiEvent;

/// Handles every [`UiNode`] + [`TabBar`]: a completed click on a tab header (press and release
/// both owned by this widget through the shared [`PointerCapture`], like a
/// [`CheckBox`](crate::ui::CheckBox) toggle) selects the header under the **release** point and
/// emits [`UiEvent::TabChanged`] when the selection actually changed. Each header renders an
/// active / hovered / inactive background and its centered title; the gaps between headers select
/// nothing. Content switching is the game's job (see [`TabBar`]).
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, TabBar>().map(|(e, _, _)| e));

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
            if let Some(tb) = world.get_mut::<TabBar>(entity) {
                if let Some(tab) = tb.tab_at(input.release_cursor, pos, size) {
                    if tab != tb.selected_index() {
                        tb.selected = tab;
                        output.events.push(UiEvent::TabChanged(entity, tab));
                    }
                }
            }
        }

        // ── Render (immutable reads now that state is settled) ────────────────
        let tb = match world.get::<TabBar>(entity) {
            Some(t) => t,
            None => continue,
        };
        if tb.tabs.is_empty() {
            continue;
        }
        let hovered_tab = if hover_owner == Some(entity) {
            tb.tab_at(input.cursor, pos, size)
        } else {
            None
        };

        for (i, title) in tb.tabs.iter().enumerate() {
            let (tpos, tsize) = tb.tab_rect(i, pos, size);
            let active = i == tb.selected_index();
            let bg = if active {
                tb.active_color
            } else if hovered_tab == Some(i) {
                tb.hover_color
            } else {
                tb.bg_color
            };
            output.rects.push(
                DrawRect::new(tpos.x, tpos.y, tsize.x, tsize.y, bg)
                    .with_corner_radius(tb.corner_radius)
                    .with_z(z),
            );
            if !title.is_empty() {
                let color = if active {
                    tb.active_text_color
                } else {
                    tb.text_color
                };
                output.texts.push(
                    DrawText::new(
                        title.clone(),
                        glam::Vec2::new(tpos.x, tpos.y + (tsize.y - tb.font_size) / 2.0),
                        tb.font_size,
                        color,
                    )
                    .with_bounds(tsize)
                    .with_align(TextAlign::Center)
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
    use crate::ui::system::{UiEvent, UiSystem};
    use crate::ui::tab_bar::TabBar;

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

    /// A 3-tab bar in a 304×30 node at (10, 10) → headers at x 10..110, 112..212, 214..314.
    fn spawn_tabs(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(10.0, 10.0, 304.0, 30.0));
        world.add_component(e, TabBar::new(["Stats", "Inventory", "Options"]));
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
                if let UiEvent::TabChanged(en, i) = e {
                    Some((*en, *i))
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn clicking_a_header_selects_it_and_emits() {
        let mut world = setup();
        let tb = spawn_tabs(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(150.0, 20.0)); // header 1 ("Inventory")
        assert_eq!(world.get::<TabBar>(tb).unwrap().selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(tb, 1)]);
    }

    #[test]
    fn reselecting_the_current_header_is_silent() {
        let mut world = setup();
        let tb = spawn_tabs(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(50.0, 20.0)); // header 0 = already active
        assert_eq!(world.get::<TabBar>(tb).unwrap().selected_index(), 0);
        assert!(
            changed_events(&world).is_empty(),
            "no event when the selection did not change"
        );
    }

    #[test]
    fn clicking_a_gap_selects_nothing() {
        let mut world = setup();
        let tb = spawn_tabs(&mut world);
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(111.0, 20.0)); // the 2 px gap
        assert_eq!(world.get::<TabBar>(tb).unwrap().selected_index(), 0);
        assert!(changed_events(&world).is_empty());
    }

    #[test]
    fn covered_tab_bar_does_not_select() {
        let mut world = setup();
        let tb = spawn_tabs(&mut world);
        // Drop the widget below the panel bg (UiNode::new defaults z to 0.9; the panel bg draws
        // at 0.9 - 0.01 = 0.89, so the default would sit ON TOP of the panel).
        world.get_mut::<UiNode>(tb).unwrap().z = 0.2;
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 400.0, 300.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));
        let mut system = UiSystem::default();

        click(&mut world, &mut system, Vec2::new(150.0, 20.0));
        assert_eq!(
            world.get::<TabBar>(tb).unwrap().selected_index(),
            0,
            "a covered tab bar must not receive the click"
        );
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
        let tb = spawn_tabs(&mut world);
        world.resource_mut::<UiFocus>().unwrap().entity = Some(tb);
        let mut system = UiSystem::default();

        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<TabBar>(tb).unwrap().selected_index(), 1);
        assert_eq!(changed_events(&world), vec![(tb, 1)]);

        // At the last tab, ArrowRight clamps (no wrap) — and no event.
        world.get_mut::<TabBar>(tb).unwrap().selected = 2;
        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        press_key(&mut world, KeyCode::ArrowRight);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<TabBar>(tb).unwrap().selected_index(), 2);
        assert!(
            changed_events(&world).is_empty(),
            "clamped step emits nothing"
        );

        press_key(&mut world, KeyCode::ArrowLeft);
        system.run(&mut world, 0.016);
        assert_eq!(world.get::<TabBar>(tb).unwrap().selected_index(), 1);
    }

    #[test]
    fn headers_render_one_rect_each_and_titles() {
        let mut world = setup();
        spawn_tabs(&mut world);
        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let rects = world.resource::<UiQueue>().unwrap().items.len();
        assert_eq!(rects, 3, "one background rect per header");
        let texts = world.resource::<TextQueue>().unwrap();
        assert!(texts.iter().any(|t| t.text == "Inventory"), "titles queued");
    }
}
