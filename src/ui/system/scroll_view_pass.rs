use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::scroll_view::ScrollView;

use super::capture::PointerCapture;
use super::state::{InputSnapshot, UiOutput};

pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, ScrollView>().map(|(e, _, _)| e));

    // The wheel scrolls only the scroll view that owns the pointer (shared capture → a scroll view
    // covered by another widget kind doesn't capture the wheel through it).
    let hover_owner = capture.topmost_at(input.cursor);

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match world.get::<UiNode>(entity) {
            Some(n) => (n.screen_pos(viewport), n.size, n.z, n.visible),
            None => continue,
        };
        if !visible {
            continue;
        }

        if input.scroll_delta != 0.0 && hover_owner == Some(entity) {
            if let Some(sv) = world.get_mut::<ScrollView>(entity) {
                sv.scroll_offset -= input.scroll_delta * sv.item_height;
                sv.clamp_scroll(size.y);
            }
        }

        let (scroll_offset, item_height, font_size, color, bg_color, item_count) = {
            let sv = match world.get::<ScrollView>(entity) {
                Some(s) => s,
                None => continue,
            };
            (
                sv.scroll_offset,
                sv.item_height,
                sv.font_size,
                sv.color,
                sv.background_color,
                sv.items.len(),
            )
        };

        // Guard: item_height == 0 would produce inf/NaN indices (division by zero).
        // Skip rendering this scroll view rather than producing out-of-range accesses.
        if item_height <= 0.0 {
            output
                .rects
                .push(DrawRect::new(pos.x, pos.y, size.x, size.y, bg_color).with_z(z));
            continue;
        }

        output
            .rects
            .push(DrawRect::new(pos.x, pos.y, size.x, size.y, bg_color).with_z(z));

        let first = (scroll_offset / item_height).floor() as usize;
        let last = (first + (size.y / item_height).ceil() as usize + 1).min(item_count);

        let sv = match world.get::<ScrollView>(entity) {
            Some(s) => s,
            None => continue,
        };
        for i in first..last {
            let y = pos.y + (i as f32 * item_height) - scroll_offset;
            if y + item_height < pos.y || y > pos.y + size.y {
                continue;
            }
            output.texts.push(
                DrawText::new(
                    sv.items[i].clone(),
                    Vec2::new(pos.x + 4.0, y),
                    font_size,
                    color,
                )
                .with_bounds(Vec2::new((size.x - 8.0).max(0.0), item_height))
                .with_z(z),
            );
        }
    }
}
