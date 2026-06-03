use crate::ecs::{Entity, World};
use crate::renderer::DrawText;
use crate::resources::ViewportSize;
use crate::ui::label::Label;
use crate::ui::node::UiNode;

use super::state::UiOutput;

pub(super) fn run(world: &mut World, viewport: &ViewportSize, output: &mut UiOutput) {
    let label_entities: Vec<Entity> = world.query2::<UiNode, Label>().map(|(e, _, _)| e).collect();

    for entity in label_entities {
        let (pos, size, visible) = match world.get::<UiNode>(entity) {
            Some(node) => (node.screen_pos(viewport), node.size, node.visible),
            None => continue,
        };
        if !visible {
            continue;
        }
        if let Some(label) = world.get::<Label>(entity) {
            let mut text = DrawText::new(label.text.clone(), pos, label.font_size, label.color)
                .with_bounds(size)
                .with_align(label.align);
            if label.rich {
                text = text.rich();
            }
            output.texts.push(text);
        }
    }
}
