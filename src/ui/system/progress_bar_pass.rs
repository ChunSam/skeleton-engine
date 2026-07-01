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
