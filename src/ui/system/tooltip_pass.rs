use glam::Vec2;

use crate::color::Color;
use crate::ecs::{Entity, World};
use crate::renderer::{DrawRect, DrawText};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;
use crate::ui::tooltip::{Tooltip, TOOLTIP_Z};

use super::capture::PointerCapture;
use super::state::{in_bounds, node_layout, InputSnapshot, UiOutput};

/// Gap in pixels between the cursor and the box when the tooltip flips above the cursor to avoid
/// running off the bottom of the viewport.
const FLIP_GAP: f32 = 8.0;

/// Ages every [`UiNode`] + [`Tooltip`]'s hover timer and, once the delay elapses, draws the popup
/// next to the cursor at [`TOOLTIP_Z`] (background box + optional border via the UI SDF pipeline,
/// then the text). Hover requires the cursor inside the node's rect **and** not covered by a
/// pointer-opaque widget drawn above it ([`PointerCapture::occludes`]) — so a tooltip under an
/// overlay panel stays silent, while one on a widget that merely sits *on* a lower-z panel works.
/// The box is clamped to the viewport: right overflow slides it left, bottom overflow flips it
/// above the cursor. Runs last so tooltip text is queued after (= drawn over) other widget text.
pub(super) fn run(
    world: &mut World,
    viewport: &ViewportSize,
    input: &InputSnapshot,
    capture: &PointerCapture,
    dt: f32,
    output: &mut UiOutput,
    scratch: &mut Vec<Entity>,
) {
    scratch.clear();
    scratch.extend(world.query2::<UiNode, Tooltip>().map(|(e, _, _)| e));

    for entity in scratch.iter().copied() {
        let (pos, size, z, visible) = match node_layout(world, entity, viewport) {
            Some(layout) => layout,
            None => continue,
        };
        let hovered = visible
            && in_bounds(input.cursor, pos, size)
            && !capture.occludes(input.cursor, entity, z);

        let tip = match world.get_mut::<Tooltip>(entity) {
            Some(t) => t,
            None => continue,
        };
        if !hovered || tip.text.is_empty() {
            tip.reset_hover();
            continue;
        }
        tip.tick_hover(dt);
        if !tip.is_showing() {
            continue;
        }

        let alpha = tip.fade_alpha();
        let content = tip.estimated_size();
        let box_size = content + Vec2::splat(2.0 * tip.padding);

        // Anchor at cursor + offset; keep the whole box on screen (slide left on right overflow,
        // flip above the cursor on bottom overflow).
        let mut p = input.cursor + tip.offset;
        if p.x + box_size.x > viewport.width {
            p.x = (viewport.width - box_size.x).max(0.0);
        }
        if p.y + box_size.y > viewport.height {
            p.y = (input.cursor.y - box_size.y - FLIP_GAP).max(0.0);
        }

        output.rects.push(
            DrawRect::new(p.x, p.y, box_size.x, box_size.y, faded(tip.bg_color, alpha))
                .with_corner_radius(tip.corner_radius)
                .with_z(TOOLTIP_Z),
        );
        if tip.border > 0.0 {
            output.rects.push(
                DrawRect::new(
                    p.x,
                    p.y,
                    box_size.x,
                    box_size.y,
                    faded(tip.border_color, alpha),
                )
                .with_corner_radius(tip.corner_radius)
                .with_border(tip.border)
                .with_z(TOOLTIP_Z + super::UI_SUBLAYER_Z_STEP),
            );
        }
        // No layout bounds: lines break only at `\n`, so a slightly-narrow width estimate can never
        // trigger a surprise wrap that overflows the box vertically.
        output.texts.push(DrawText::new(
            tip.text.clone(),
            p + Vec2::splat(tip.padding),
            tip.font_size,
            faded(tip.text_color, alpha),
        ));
    }
}

/// `color` with its alpha scaled by the fade-in factor.
fn faded(color: Color, alpha: f32) -> Color {
    Color::rgba(color.r, color.g, color.b, color.a * alpha)
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::ecs::{Events, System, World};
    use crate::input::InputState;
    use crate::renderer::{DrawRect, TextQueue, UiQueue};
    use crate::resources::ViewportSize;
    use crate::ui::node::UiNode;
    use crate::ui::panel::{LayoutDir, Panel};
    use crate::ui::system::{UiEvent, UiSystem};
    use crate::ui::tooltip::{Tooltip, TOOLTIP_Z};

    const DT: f32 = 0.3;

    fn setup(cursor: Vec2) -> World {
        let mut world = World::new();
        world.insert_resource(ViewportSize::new(400, 300));
        world.insert_resource(Events::<UiEvent>::default());
        world.insert_resource(UiQueue::default());
        world.insert_resource(TextQueue::default());
        let mut input = InputState::default();
        input.set_cursor(cursor);
        world.insert_resource(input);
        world
    }

    /// Spawns a 100×40 node at (50, 50) with `tip` and returns its entity.
    fn spawn_tooltip_widget(world: &mut World, tip: Tooltip) -> crate::ecs::Entity {
        let e = world.spawn();
        world.add_component(e, UiNode::new(50.0, 50.0, 100.0, 40.0));
        world.add_component(e, tip);
        e
    }

    /// Drains and returns the tooltip-layer rects queued so far (z >= TOOLTIP_Z).
    fn take_tooltip_rects(world: &mut World) -> Vec<DrawRect> {
        let q = world.resource_mut::<UiQueue>().unwrap();
        let rects = std::mem::take(&mut q.items);
        rects.into_iter().filter(|r| r.z >= TOOLTIP_Z).collect()
    }

    fn queued_text_contains(world: &World, needle: &str) -> bool {
        world
            .resource::<TextQueue>()
            .unwrap()
            .iter()
            .any(|t| t.text.contains(needle))
    }

    #[test]
    fn tooltip_appears_only_after_the_delay() {
        let mut world = setup(Vec2::new(60.0, 60.0));
        spawn_tooltip_widget(
            &mut world,
            Tooltip::new("hint").with_delay(0.4).with_fade(0.0),
        );
        let mut system = UiSystem::default();

        system.run(&mut world, DT); // 0.3s hovered < 0.4s delay
        assert!(take_tooltip_rects(&mut world).is_empty(), "still delayed");
        assert!(!queued_text_contains(&world, "hint"));

        system.run(&mut world, DT); // 0.6s hovered >= 0.4s delay
        assert!(!take_tooltip_rects(&mut world).is_empty(), "delay elapsed");
        assert!(queued_text_contains(&world, "hint"));
    }

    #[test]
    fn leaving_the_widget_resets_the_delay() {
        let mut world = setup(Vec2::new(60.0, 60.0));
        spawn_tooltip_widget(
            &mut world,
            Tooltip::new("hint").with_delay(0.4).with_fade(0.0),
        );
        let mut system = UiSystem::default();

        system.run(&mut world, DT); // 0.3s of hover
        world
            .resource_mut::<InputState>()
            .unwrap()
            .set_cursor(Vec2::new(300.0, 200.0)); // leave
        system.run(&mut world, DT); // timer resets
        world
            .resource_mut::<InputState>()
            .unwrap()
            .set_cursor(Vec2::new(60.0, 60.0)); // back
        system.run(&mut world, DT); // only 0.3s hovered again

        assert!(
            take_tooltip_rects(&mut world).is_empty(),
            "delay must restart after the cursor leaves"
        );
    }

    #[test]
    fn covered_widget_shows_no_tooltip() {
        let mut world = setup(Vec2::new(60.0, 60.0));
        let widget = spawn_tooltip_widget(
            &mut world,
            Tooltip::new("hidden hint").with_delay(0.0).with_fade(0.0),
        );
        world.get_mut::<UiNode>(widget).unwrap().z = 0.5;

        // A pointer-opaque panel drawn over the widget (bg at 0.9 - 0.01, above 0.5).
        let panel = world.spawn();
        let mut node = UiNode::new(0.0, 0.0, 300.0, 200.0);
        node.z = 0.9;
        world.add_component(panel, node);
        world.add_component(panel, Panel::new(LayoutDir::Vertical));

        let mut system = UiSystem::default();
        system.run(&mut world, DT);
        assert!(
            take_tooltip_rects(&mut world).is_empty(),
            "a covered widget must not show its tooltip"
        );

        // Control: drop the panel below the widget and the tooltip appears.
        world.get_mut::<UiNode>(panel).unwrap().z = 0.1;
        system.run(&mut world, DT);
        assert!(
            !take_tooltip_rects(&mut world).is_empty(),
            "a widget over a lower-z panel shows its tooltip"
        );
    }

    #[test]
    fn tooltip_clamps_inside_the_viewport() {
        // Widget at the bottom-right corner of the 400×300 viewport; cursor on it.
        let cursor = Vec2::new(390.0, 290.0);
        let mut world = setup(cursor);
        let e = world.spawn();
        world.add_component(e, UiNode::new(350.0, 260.0, 50.0, 40.0));
        world.add_component(
            e,
            Tooltip::new("a fairly long tooltip line")
                .with_delay(0.0)
                .with_fade(0.0),
        );

        let mut system = UiSystem::default();
        system.run(&mut world, DT);
        let rects = take_tooltip_rects(&mut world);
        let bg = rects.first().expect("tooltip visible");
        assert!(
            bg.x + bg.w <= 400.0 + 1e-3,
            "right edge clamped: x={} w={}",
            bg.x,
            bg.w
        );
        assert!(
            bg.y + bg.h <= cursor.y,
            "flipped above the cursor: y={} h={}",
            bg.y,
            bg.h
        );
    }
}
