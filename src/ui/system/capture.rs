use glam::Vec2;

use crate::ecs::{Entity, World};
use crate::resources::ViewportSize;
use crate::ui::button::Button;
use crate::ui::checkbox::CheckBox;
use crate::ui::dropdown::{Dropdown, DROPDOWN_LIST_Z};
use crate::ui::list_box::ListBox;
use crate::ui::node::UiNode;
use crate::ui::panel::{Panel, PANEL_BG_Z_OFFSET};
use crate::ui::radio_group::RadioGroup;
use crate::ui::scroll_view::ScrollView;
use crate::ui::slider::Slider;
use crate::ui::stepper::Stepper;
use crate::ui::switch::Switch;
use crate::ui::tab_bar::TabBar;
use crate::ui::text_input::TextInput;

use super::state::in_bounds;

/// A visible UI surface that is opaque to the pointer: it occludes whatever is drawn behind it,
/// so a click / hover / scroll at a point is owned by the *topmost* such surface there.
struct CaptureItem {
    entity: Entity,
    pos: Vec2,
    size: Vec2,
    /// The z at which this surface's opaque background is actually drawn, so occlusion matches what
    /// the player sees. Most widgets draw their background at [`UiNode::z`]; a [`Panel`] draws its
    /// background at `z - PANEL_BG_Z_OFFSET` (see [`LayoutSystem`]), so it registers there.
    ///
    /// [`LayoutSystem`]: crate::ui::panel::LayoutSystem
    z: f32,
}

/// Per-frame map of pointer-opaque UI surfaces, rebuilt once per [`UiSystem`](super::UiSystem) run
/// and shared by the focus / button / checkbox / slider / scroll passes so that a single, consistent
/// "who is on top here?" decision drives every pointer interaction.
///
/// Without it each widget pass hit-tested only its own kind, so a button covered by a panel (or any
/// other widget kind drawn on top) could still fire. With it, an interaction is granted only to the
/// widget that actually owns the pixel under the cursor. See [`Self::topmost_at`].
#[derive(Default)]
pub(super) struct PointerCapture {
    items: Vec<CaptureItem>,
}

impl PointerCapture {
    /// Rebuilds the capture set from every visible pointer-opaque widget (buttons, checkboxes,
    /// sliders, radio groups, tab bars, text inputs, scroll views, and panels). Reuses the backing
    /// allocation across frames (cleared, not reallocated), like the passes' scratch buffers.
    /// Labels are intentionally excluded — text is not pointer-opaque and must not block clicks
    /// behind it.
    pub(super) fn rebuild(&mut self, world: &World, viewport: &ViewportSize) {
        self.items.clear();
        self.extend_kind::<Button>(world, viewport, 0.0);
        self.extend_kind::<CheckBox>(world, viewport, 0.0);
        self.extend_kind::<Slider>(world, viewport, 0.0);
        self.extend_kind::<RadioGroup>(world, viewport, 0.0);
        self.extend_kind::<TabBar>(world, viewport, 0.0);
        self.extend_kind::<ListBox>(world, viewport, 0.0);
        self.extend_kind::<Stepper>(world, viewport, 0.0);
        self.extend_kind::<Switch>(world, viewport, 0.0);
        self.extend_kind::<TextInput>(world, viewport, 0.0);
        self.extend_kind::<ScrollView>(world, viewport, 0.0);
        self.extend_kind::<Panel>(world, viewport, PANEL_BG_Z_OFFSET);
        self.extend_dropdowns(world, viewport);
    }

    /// Appends every visible dropdown. A **closed** dropdown captures like any widget (its node
    /// rect at its node z); an **open** one captures its whole expanded rect (closed box + item
    /// list, [`Dropdown::expanded_rect`]) at [`DROPDOWN_LIST_Z`], so the overlaid list wins the
    /// pointer over — and blocks clicks/hover reaching — everything drawn underneath it.
    fn extend_dropdowns(&mut self, world: &World, viewport: &ViewportSize) {
        self.items.extend(
            world
                .query2::<UiNode, Dropdown>()
                .filter(|(_, node, _)| node.visible)
                .map(|(e, node, dd)| {
                    let pos = node.screen_pos(viewport);
                    let (cpos, csize, cz) = if dd.open {
                        let (epos, esize) = dd.expanded_rect(pos, node.size, viewport.height);
                        (epos, esize, DROPDOWN_LIST_Z)
                    } else {
                        (pos, node.size, node.z)
                    };
                    CaptureItem {
                        entity: e,
                        pos: cpos,
                        size: csize,
                        z: cz,
                    }
                }),
        );
    }

    /// Appends every visible `(UiNode, W)` entity as a capture item, drawing its capture z at
    /// `node.z - z_offset` (non-zero only for panels, whose background draws below `node.z`).
    fn extend_kind<W: 'static>(&mut self, world: &World, viewport: &ViewportSize, z_offset: f32) {
        self.items.extend(
            world
                .query2::<UiNode, W>()
                .filter(|(_, node, _)| node.visible)
                .map(|(e, node, _)| CaptureItem {
                    entity: e,
                    pos: node.screen_pos(viewport),
                    size: node.size,
                    z: node.z - z_offset,
                }),
        );
    }

    /// True when a visible pointer-opaque surface **other than `entity`**, drawn strictly above
    /// `z`, contains `cursor` — i.e. the widget at (`entity`, `z`) is covered at that point. Used
    /// by the tooltip pass, whose host widgets (labels, progress bars, …) may not be capture items
    /// themselves, so [`topmost_at`](Self::topmost_at) alone cannot answer "is my widget covered?".
    /// An equal-z surface does not occlude (forgiving for widgets stacked at the same depth).
    pub(super) fn occludes(&self, cursor: Vec2, entity: Entity, z: f32) -> bool {
        self.items
            .iter()
            .any(|it| it.entity != entity && it.z > z && in_bounds(cursor, it.pos, it.size))
    }

    /// Returns the entity of the topmost pointer-opaque surface containing `cursor`, or `None` if no
    /// surface is there. "Topmost" is the greatest capture z; ties (equal z) are broken by the
    /// greater entity index, which matches painter's order (later-submitted draws on top) and the
    /// pre-existing focus tie-break.
    pub(super) fn topmost_at(&self, cursor: Vec2) -> Option<Entity> {
        self.items
            .iter()
            .filter(|it| in_bounds(cursor, it.pos, it.size))
            .max_by(|a, b| {
                a.z.partial_cmp(&b.z)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.entity.index().cmp(&b.entity.index()))
            })
            .map(|it| it.entity)
    }
}
