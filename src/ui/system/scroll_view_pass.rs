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

        // The wheel moves the offset; the clamp runs **every** frame regardless, because `items`
        // can shrink under a scrolled view (a cleared log, a filtered inventory) and nothing else
        // re-clamps it. Until v0.156.27 the clamp lived inside the wheel branch, so a stale
        // offset survived: `first` landed past the end, `first..last` was empty, and the view
        // rendered its background and *no rows at all* until the player happened to scroll.
        if let Some(sv) = world.get_mut::<ScrollView>(entity) {
            if input.scroll_delta != 0.0 && hover_owner == Some(entity) {
                sv.scroll_offset -= input.scroll_delta * sv.item_height;
            }
            sv.clamp_scroll(size.y);
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

        // `saturating_add` because `item_height` is only guarded against `<= 0.0`: a tiny positive
        // height (`f32::MIN_POSITIVE`, reachable from the inspector) makes `size.y / item_height`
        // infinite, which saturates to `usize::MAX` on the cast and overflows a plain `+`. The
        // per-frame clamp above already keeps `first` inside the list.
        let first = (scroll_offset / item_height).floor().max(0.0) as usize;
        let visible = (size.y / item_height).ceil() as usize;
        let last = first
            .saturating_add(visible)
            .saturating_add(1)
            .min(item_count);

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

#[cfg(test)]
mod tests {
    use crate::ecs::{Events, System, World};
    use crate::input::InputState;
    use crate::renderer::{TextQueue, UiQueue};
    use crate::resources::ViewportSize;
    use crate::ui::focus::UiFocus;
    use crate::ui::node::UiNode;
    use crate::ui::scroll_view::ScrollView;
    use crate::ui::{UiEvent, UiSystem};

    /// A world holding one `ScrollView` of `n` 24px items in a 100-tall node.
    fn world_with_scroll_view(n: usize) -> (World, crate::ecs::Entity) {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(UiQueue::default());
        world.insert_resource(TextQueue::default());
        world.insert_resource(UiFocus::default());
        world.insert_resource(InputState::default());
        let e = world.spawn();
        world.add_component(e, UiNode::new(10.0, 10.0, 200.0, 100.0));
        let items: Vec<String> = (0..n).map(|i| format!("item{i}")).collect();
        world.add_component(
            e,
            ScrollView::new().with_items(items).with_item_height(24.0),
        );
        (world, e)
    }

    fn drawn_texts(world: &World) -> Vec<String> {
        world
            .resource::<TextQueue>()
            .unwrap()
            .iter()
            .map(|t| t.text.clone())
            .collect()
    }

    /// v0.156.27: `clamp_scroll` ran only inside the wheel branch, so shrinking `items` under a
    /// scrolled view left a stale `scroll_offset` and the view rendered no rows at all.
    #[test]
    fn shrinking_items_under_a_scrolled_view_still_renders_rows() {
        let (mut world, e) = world_with_scroll_view(100);
        {
            let sv = world.get_mut::<ScrollView>(e).unwrap();
            sv.scroll_offset = 2300.0; // the legal max for 100 items
            sv.items.truncate(3);
        }
        UiSystem::default().run(&mut world, 0.016);

        let drawn = drawn_texts(&world);
        assert_eq!(
            drawn.len(),
            3,
            "a view whose items shrank must still draw them, got {drawn:?}"
        );
    }

    /// `scroll_offset = f32::MAX` is the idiomatic "pin to bottom"; before v0.156.27 the
    /// unclamped `first` saturated to `usize::MAX` and the render window's addition overflowed.
    #[test]
    fn a_huge_scroll_offset_pins_to_the_bottom_without_overflowing() {
        let (mut world, e) = world_with_scroll_view(100);
        world.get_mut::<ScrollView>(e).unwrap().scroll_offset = f32::MAX;
        UiSystem::default().run(&mut world, 0.016); // must not panic

        let drawn = drawn_texts(&world);
        assert!(
            drawn.iter().any(|t| t == "item99"),
            "pinning to the bottom must show the last page, got {drawn:?}"
        );
    }

    /// A tiny positive `item_height` slips past the `<= 0.0` guard and makes the visible-row
    /// count saturate; the window arithmetic must not overflow.
    #[test]
    fn a_tiny_item_height_does_not_overflow_the_render_window() {
        let (mut world, e) = world_with_scroll_view(10);
        world.get_mut::<ScrollView>(e).unwrap().item_height = f32::MIN_POSITIVE;
        UiSystem::default().run(&mut world, 0.016); // must not panic
    }
}
