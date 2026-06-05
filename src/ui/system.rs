use crate::ecs::{System, World};

mod button_pass;
mod checkbox_pass;
mod event;
mod label_pass;
mod scroll_view_pass;
mod slider_pass;
mod state;
mod text_input_pass;

pub use event::UiEvent;
use state::{submit_output, viewport_from_world, InputSnapshot, UiOutput};

/// `UiNode` + `Button` / `Label` / `TextInput` / `ScrollView` / `Slider` / `CheckBox` 엔티티를 처리하는 시스템.
///
/// 매 프레임 실행 순서:
/// 1. 입력 상태 스냅샷
/// 2. 버튼 히트 테스트 -> `ButtonState` 갱신 + `UiEvent` 발행
/// 3. TextInput 패스 - 포커스, 문자 입력, 커서 깜빡임
/// 4. ScrollView 패스 - 휠 스크롤, 아이템 렌더
/// 5. Label 패스
/// 6. Slider 패스
/// 7. CheckBox 패스
/// 8. 렌더 큐 제출
/// 9. 이벤트 일괄 발행
pub struct UiSystem;

impl UiSystem {
    /// 스케줄 라벨. 권장 순서: `LayoutSystem::LABEL` **이후**
    /// (`SystemConfig::new().label(UiSystem::LABEL).after(LayoutSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::ui";
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
        button_pass::run(world, &viewport, &input, &mut output);
        text_input_pass::run(world, &viewport, &input, dt, &mut output);
        scroll_view_pass::run(world, &viewport, &input, &mut output);
        label_pass::run(world, &viewport, &mut output);
        slider_pass::run(world, &viewport, &input, &mut output);
        checkbox_pass::run(world, &viewport, &input, &mut output);
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
        let mut system = UiSystem;

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

        let mut system = UiSystem;
        system.run(&mut world, 0.016);

        assert_eq!(click_count(&world, entity), 1);
    }

    #[test]
    fn button_click_does_not_emit_when_released_outside() {
        let (mut world, entity) = setup_button_world(Vec2::new(20.0, 20.0));
        let mut system = UiSystem;

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
        let mut system = UiSystem;

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
        let mut system = UiSystem;

        {
            let input = world.resource_mut::<InputState>().unwrap();
            input.press_mouse(MouseButton::Left); // press_cursor = (20,20), inside
            input.release_mouse(MouseButton::Left); // release_cursor = (20,20), inside
            input.set_cursor(Vec2::new(150.0, 20.0)); // live cursor leaves afterwards
        }
        system.run(&mut world, 0.016);
        assert_eq!(click_count(&world, entity), 1);
    }
}
