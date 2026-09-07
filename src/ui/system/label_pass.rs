use crate::ecs::{Entity, World};
use crate::renderer::DrawText;
use crate::resources::ViewportSize;
use crate::ui::label::Label;
use crate::ui::node::UiNode;

use super::state::UiOutput;

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Label>().map(|(e, _, _)| e));

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match world.get::<UiNode>(entity) {
            Some(node) => (node.screen_pos(viewport), node.size, node.z, node.visible),
            None => continue,
        };
        if !visible {
            continue;
        }
        if let Some(label) = world.get::<Label>(entity) {
            let mut text = DrawText::new(label.text.clone(), pos, label.font_size, label.color)
                .with_bounds(size)
                .with_align(label.align)
                .with_z(z);
            if label.rich {
                text = text.rich();
            }
            output.texts.push(text);
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::ecs::{Events, System, World};
    use crate::input::InputState;
    use crate::renderer::{TextQueue, UiQueue};
    use crate::resources::ViewportSize;
    use crate::ui::focus::UiFocus;
    use crate::ui::label::Label;
    use crate::ui::node::UiNode;
    use crate::ui::{UiEvent, UiSystem};

    /// v0.156.29: `label_pass::run` had no test asserting its output — the body could be deleted
    /// and nothing went red; labels would simply stop drawing.
    #[test]
    fn a_visible_label_is_queued_at_its_node_z() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(UiQueue::default());
        world.insert_resource(TextQueue::default());
        world.insert_resource(UiFocus::default());
        world.insert_resource(InputState::default());
        let e = world.spawn();
        let mut node = UiNode::new(20.0, 30.0, 200.0, 24.0);
        node.z = 0.42;
        world.add_component(e, node);
        world.add_component(e, Label::new("score: 10"));

        UiSystem::default().run(&mut world, 0.016);

        let queued: Vec<_> = world
            .resource::<TextQueue>()
            .unwrap()
            .iter()
            .filter(|t| t.text == "score: 10")
            .collect();
        assert_eq!(queued.len(), 1, "the label draws exactly once");
        assert_eq!(queued[0].position, Vec2::new(20.0, 30.0));
        assert_eq!(queued[0].z, Some(0.42), "the label draws at its node's z");
    }

    #[test]
    fn a_hidden_label_is_not_queued() {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(UiQueue::default());
        world.insert_resource(TextQueue::default());
        world.insert_resource(UiFocus::default());
        world.insert_resource(InputState::default());
        let e = world.spawn();
        world.add_component(e, UiNode::new(20.0, 30.0, 200.0, 24.0).with_visible(false));
        world.add_component(e, Label::new("hidden"));

        UiSystem::default().run(&mut world, 0.016);
        assert!(world.resource::<TextQueue>().unwrap().iter().count() == 0);
    }
}
