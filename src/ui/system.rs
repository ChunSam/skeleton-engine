use crate::ecs::{Entity, System, World};

mod button_pass;
mod checkbox_pass;
mod event;
mod focus_pass;
mod label_pass;
mod scroll_view_pass;
mod slider_pass;
mod state;
mod text_input_pass;

pub use event::UiEvent;
use state::{submit_output, viewport_from_world, InputSnapshot, UiOutput};

/// System that processes `UiNode` + `Button` / `Label` / `TextInput` / `ScrollView` / `Slider` / `CheckBox` entities.
///
/// Per-frame execution order:
/// 1. Input state snapshot
/// 2. Button hit-test → update `ButtonState` + emit `UiEvent`
/// 3. TextInput pass — focus, character input, cursor blink
/// 4. ScrollView pass — wheel scroll, item render
/// 5. Label pass
/// 6. Slider pass
/// 7. CheckBox pass
/// 8. Submit render queue
/// 9. Batch-emit events
#[derive(Default)]
pub struct UiSystem {
    button_scratch: Vec<Entity>,
    checkbox_scratch: Vec<Entity>,
    focus_scratch: Vec<Entity>,
    label_scratch: Vec<Entity>,
    scroll_view_scratch: Vec<Entity>,
    slider_scratch: Vec<Entity>,
    text_input_scratch: Vec<Entity>,
}

impl UiSystem {
    /// Schedule label. Recommended order: **after** `LayoutSystem::LABEL`
    /// (`SystemConfig::new().label(UiSystem::LABEL).after(LayoutSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::ui";

    /// Creates a new `UiSystem`. Equivalent to `UiSystem::default()`.
    pub fn new() -> Self {
        Self::default()
    }
}

impl System for UiSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let input = match InputSnapshot::from_world(world) {
            Some(input) => input,
            None => return,
        };
        let viewport = match viewport_from_world(world) {
            Some(viewport) => viewport,
            None => return,
        };

        let mut output = UiOutput::default();
        // Keyboard focus first, so a Tab-focused TextInput receives this frame's typed characters.
        focus_pass::run(
            world,
            &viewport,
            &input,
            &mut output,
            &mut self.focus_scratch,
        );
        button_pass::run(
            world,
            &viewport,
            &input,
            &mut output,
            &mut self.button_scratch,
        );
        text_input_pass::run(
            world,
            &viewport,
            &input,
            dt,
            &mut output,
            &mut self.text_input_scratch,
        );
        scroll_view_pass::run(
            world,
            &viewport,
            &input,
            &mut output,
            &mut self.scroll_view_scratch,
        );
        label_pass::run(world, &viewport, &mut output, &mut self.label_scratch);
        slider_pass::run(
            world,
            &viewport,
            &input,
            &mut output,
            &mut self.slider_scratch,
        );
        checkbox_pass::run(
            world,
            &viewport,
            &input,
            &mut output,
            &mut self.checkbox_scratch,
        );
        submit_output(world, output);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;
    use winit::event::MouseButton;

    use crate::ecs::{Entity, Events, World};
    use crate::input::InputState;
    use crate::resources::ViewportSize;
    use crate::ui::button::{Button, ButtonState};
    use crate::ui::node::UiNode;
    use crate::ui::scroll_view::ScrollView;
    use crate::ui::slider::Slider;

    fn setup_button_world(cursor: Vec2) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(200, 120));
        world.insert_resource(Events::<UiEvent>::default());

        let mut input = InputState::default();
        input.set_cursor(cursor);
        world.insert_resource(input);

        let entity = world.spawn();
        world.add_component(entity, UiNode::new(10.0, 10.0, 80.0, 40.0));
        world.add_component(entity, Button::new("Click"));

        (world, entity)
    }

    fn click_count(world: &World, entity: Entity) -> usize {
        world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter(|event| matches!(event, UiEvent::ButtonClicked(clicked) if *clicked == entity))
            .count()
    }

    #[test]
    fn button_click_emits_once_on_release_in_bounds() {
        let (mut world, entity) = setup_button_world(Vec2::new(20.0, 20.0));
        let mut system = UiSystem::default();

        world
            .resource_mut::<InputState>()
            .unwrap()
            .press_mouse(MouseButton::Left);
        system.run(&mut world, 0.016);
        assert_eq!(click_count(&world, entity), 0);
        assert_eq!(
            world.get::<Button>(entity).unwrap().state,
            ButtonState::Pressed
        );

        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        world.resource_mut::<InputState>().unwrap().flush();
        world
            .resource_mut::<InputState>()
            .unwrap()
            .release_mouse(MouseButton::Left);
        system.run(&mut world, 0.016);
        assert_eq!(click_count(&world, entity), 1);
    }

    #[test]
    fn button_click_handles_press_and_release_in_same_frame_once() {
        let (mut world, entity) = setup_button_world(Vec2::new(20.0, 20.0));

        let input = world.resource_mut::<InputState>().unwrap();
        input.press_mouse(MouseButton::Left);
        input.release_mouse(MouseButton::Left);

        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        assert_eq!(click_count(&world, entity), 1);
    }

    #[test]
    fn button_click_does_not_emit_when_released_outside() {
        let (mut world, entity) = setup_button_world(Vec2::new(20.0, 20.0));
        let mut system = UiSystem::default();

        world
            .resource_mut::<InputState>()
            .unwrap()
            .press_mouse(MouseButton::Left);
        system.run(&mut world, 0.016);

        world.resource_mut::<Events<UiEvent>>().unwrap().flush();
        let input = world.resource_mut::<InputState>().unwrap();
        input.flush();
        input.set_cursor(Vec2::new(120.0, 20.0));
        input.release_mouse(MouseButton::Left);

        system.run(&mut world, 0.016);

        assert_eq!(click_count(&world, entity), 0);
    }

    #[test]
    fn click_uses_press_cursor_not_the_moved_cursor() {
        // Press OFF the button (button is 10..90 × 10..50), then move ONTO it in the
        // same frame, then release. The press was off the button, so no click fires —
        // this is the regression for "click applied at the moved-to position".
        let (mut world, entity) = setup_button_world(Vec2::new(150.0, 20.0));
        let mut system = UiSystem::default();

        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.press_mouse(MouseButton::Left); // press_cursor = (150,20), outside
            input.set_cursor(Vec2::new(20.0, 20.0)); // cursor moves onto the button
            input.release_mouse(MouseButton::Left); // release_cursor = (20,20), inside
        }
        system.run(&mut world, 0.016);
        assert_eq!(click_count(&world, entity), 0);
    }

    #[test]
    fn click_fires_when_press_and_release_on_button_then_cursor_leaves() {
        // Press + release on the button, then the cursor moves away afterwards.
        // The click must still register (it is decided by the press/release cursors).
        let (mut world, entity) = setup_button_world(Vec2::new(20.0, 20.0));
        let mut system = UiSystem::default();

        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.press_mouse(MouseButton::Left); // press_cursor = (20,20), inside
            input.release_mouse(MouseButton::Left); // release_cursor = (20,20), inside
            input.set_cursor(Vec2::new(150.0, 20.0)); // live cursor leaves afterwards
        }
        system.run(&mut world, 0.016);
        assert_eq!(click_count(&world, entity), 1);
    }

    // ── Fix 1: overlapping buttons — only topmost fires ──────────────────────

    /// Two overlapping buttons at different z: click in the overlap fires only the higher-z one.
    /// Without the fix, both buttons would emit ButtonClicked in the same frame.
    #[test]
    fn overlapping_buttons_only_topmost_fires_on_click() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());

        let mut input = InputState::default();
        // Both buttons cover (50, 50) – click right in the overlap
        input.set_cursor(Vec2::new(60.0, 60.0));
        world.insert_resource(input);

        // low_btn: z = 0.5, covers (50..150, 50..100)
        let low_btn = world.spawn();
        let mut low_node = UiNode::new(50.0, 50.0, 100.0, 50.0);
        low_node.z = 0.5;
        world.add_component(low_btn, low_node);
        world.add_component(low_btn, Button::new("Low"));

        // high_btn: z = 0.8, same rect – overlaps completely
        let high_btn = world.spawn();
        let mut high_node = UiNode::new(50.0, 50.0, 100.0, 50.0);
        high_node.z = 0.8;
        world.add_component(high_btn, high_node);
        world.add_component(high_btn, Button::new("High"));

        // Simulate a click: press + release at the same spot in one frame
        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.press_mouse(MouseButton::Left);
            input.release_mouse(MouseButton::Left);
        }

        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let events: Vec<_> = world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter(|e| matches!(e, UiEvent::ButtonClicked(_)))
            .cloned()
            .collect();

        assert_eq!(
            events.len(),
            1,
            "expected exactly 1 ButtonClicked, got {events:?}"
        );
        assert!(
            matches!(events[0], UiEvent::ButtonClicked(e) if e == high_btn),
            "expected high-z button to fire, got {events:?}"
        );
    }

    // ── Fix 2: ScrollView with item_height == 0 does not panic ───────────────

    /// A ScrollView with item_height = 0 must not panic or produce out-of-range accesses.
    /// Without the fix, item_height==0 leads to division-by-zero → usize::MAX index → panic.
    #[test]
    fn scroll_view_zero_item_height_does_not_panic() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(InputState::default());

        let entity = world.spawn();
        world.add_component(entity, UiNode::new(10.0, 10.0, 200.0, 100.0));
        let mut sv = ScrollView::new()
            .with_items(vec!["a".into(), "b".into(), "c".into()])
            .with_item_height(0.0);
        sv.item_height = 0.0; // explicitly zero (also tests the reflect/editor path)
        world.add_component(entity, sv);

        let mut system = UiSystem::default();
        // Must complete without panicking.
        system.run(&mut world, 0.016);
    }

    // ── Fix 3: Slider emits only one SliderChanged on the press frame ─────────

    fn count_slider_events(world: &World) -> Vec<f32> {
        world
            .resource::<Events<UiEvent>>()
            .unwrap()
            .read()
            .iter()
            .filter_map(|e| {
                if let UiEvent::SliderChanged(_, v) = e {
                    Some(*v)
                } else {
                    None
                }
            })
            .collect()
    }

    /// On the frame just_pressed fires AND the live cursor has moved, only ONE
    /// SliderChanged event must be emitted (from the press position, not the drag position).
    /// Without the fix, both the press block and the dragging block run in the same frame,
    /// emitting two events with different values.
    #[test]
    fn slider_press_frame_emits_exactly_one_changed_event() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());

        // Slider at x=100, y=50, width=200, height=20  →  track from 100 to 300
        let entity = world.spawn();
        world.add_component(entity, UiNode::new(100.0, 50.0, 200.0, 20.0));
        world.add_component(entity, Slider::new(0.0, 1.0, 0.0));

        // press at x=150 (left side of slider), but live cursor is at x=250 (right side)
        let mut input = InputState::default();
        input.set_cursor(Vec2::new(250.0, 60.0)); // live cursor — moved
        world.insert_resource(input);

        {
            let input = world.resource_mut::<InputState>().unwrap();
            // Simulate: cursor was at (150,60) when button went down
            // We achieve this by pressing first (which records press_cursor from current cursor),
            // then moving the live cursor AFTER.
            input.set_cursor(Vec2::new(150.0, 60.0));
            input.press_mouse(MouseButton::Left); // records press_cursor = (150,60)
            input.set_cursor(Vec2::new(250.0, 60.0)); // live cursor moves to the right
        }

        let mut system = UiSystem::default();
        system.run(&mut world, 0.016);

        let values = count_slider_events(&world);
        assert_eq!(
            values.len(),
            1,
            "expected exactly 1 SliderChanged on press frame, got {values:?}"
        );
    }
}
