use crate::ecs::{Entity, World};
use crate::renderer::DrawRect;
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::progress_bar::ProgressBar;

use super::state::{node_layout, UiOutput};

/// Draws every [`UiNode`] + [`ProgressBar`] as a background track rect plus a fill rect scaled to the
/// bar's clamped [`fraction`](ProgressBar::fraction). Read-only — no input, no capture (mirrors
/// `label_pass`). The fill sits one Z sub-layer above the background so it composites on top; an
/// optional border outline is drawn above both.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, ProgressBar>().map(|(e, _, _)| e));

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };
        if !visible {
            continue;
        }
        let bar = match world.get::<ProgressBar>(entity) {
            Some(bar) => bar,
            None => continue,
        };
        let radius = bar.corner_radius;

        // Background track (whole node rect).
        output.rects.push(
            DrawRect::new(pos.x, pos.y, size.x, size.y, bar.bg_color)
                .with_corner_radius(radius)
                .with_z(z),
        );

        // Fill, from the left, `fraction × width` wide. Skip a zero-width fill so an empty bar draws
        // nothing over the track.
        let fill_w = size.x * bar.fraction();
        if fill_w > 0.0 {
            output.rects.push(
                DrawRect::new(pos.x, pos.y, fill_w, size.y, bar.fill_color)
                    .with_corner_radius(radius)
                    .with_z(z + super::UI_SUBLAYER_Z_STEP),
            );
        }

        // Optional outline on top of the whole bar.
        if bar.border > 0.0 {
            output.rects.push(
                DrawRect::new(pos.x, pos.y, size.x, size.y, bar.border_color)
                    .with_corner_radius(radius)
                    .with_border(bar.border)
                    .with_z(z + 2.0 * super::UI_SUBLAYER_Z_STEP),
            );
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::color::Color;
    use crate::ecs::{Events, System, World};
    use crate::input::InputState;
    use crate::renderer::{TextQueue, UiQueue};
    use crate::resources::ViewportSize;
    use crate::ui::focus::UiFocus;
    use crate::ui::node::UiNode;
    use crate::ui::progress_bar::ProgressBar;
    use crate::ui::{UiEvent, UiSystem};

    fn world_with_bar(bar: ProgressBar) -> World {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(UiQueue::default());
        world.insert_resource(TextQueue::default());
        world.insert_resource(UiFocus::default());
        world.insert_resource(InputState::default());
        let e = world.spawn();
        world.add_component(e, UiNode::new(10.0, 10.0, 200.0, 20.0));
        world.add_component(e, bar);
        world
    }

    /// v0.156.29: `progress_bar_pass::run` had no test asserting its output either.
    #[test]
    fn a_half_full_bar_draws_track_fill_and_border() {
        let mut world =
            world_with_bar(ProgressBar::new(0.5).with_border(2.0, Color::rgba(1.0, 1.0, 1.0, 1.0)));
        UiSystem::default().run(&mut world, 0.016);

        let rects = &world.resource::<UiQueue>().unwrap().items;
        assert_eq!(
            rects.len(),
            3,
            "track + fill + border, got {} rects",
            rects.len()
        );
        let fill = rects
            .iter()
            .find(|r| (r.w - 100.0).abs() < f32::EPSILON)
            .expect("a fill at half the 200px width");
        assert!(fill.z > rects[0].z, "the fill composites over the track");
    }

    /// The "skip a zero-width fill" branch: an empty bar draws the track alone.
    #[test]
    fn an_empty_bar_draws_only_its_track() {
        let mut world = world_with_bar(ProgressBar::new(0.0));
        UiSystem::default().run(&mut world, 0.016);
        assert_eq!(world.resource::<UiQueue>().unwrap().items.len(), 1);
    }
}
