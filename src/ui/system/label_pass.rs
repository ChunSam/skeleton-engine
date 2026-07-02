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
