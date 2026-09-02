//! Lighting editor panel — native only.
//!
//! Renders the inspector controls for the 2D lighting pass: per-entity [`PointLight`] field
//! drags and the global [`AmbientLight`] resource control. Extracted verbatim from `docked.rs`;
//! `point_light_grid` is called from the per-component inspector dispatch in
//! `component_registry.rs`, and `ambient_light_control` from `inspector_tab_body` in `docked.rs`.
//!
//! ⚠️ Every `DragValue` here binds the component's field directly and carries
//! `.clamp_existing_to_range(false)`: egui's default clamps the *existing* value to the widget's
//! range on display, so merely selecting a light at `intensity 20` used to write `10` into the
//! world with no interaction (v0.156.13). The range still bounds what a drag produces.
//!
//! [`PointLight`]: crate::components::PointLight
//! [`AmbientLight`]: crate::resources::AmbientLight

#![cfg(not(target_arch = "wasm32"))]

use crate::app::editor::tr;
use crate::app::App;

/// Render the live-tunable fields of the selected entity's `PointLight` (color / radius /
/// intensity / light_height). Edits mutate the component so the lighting pass reflects them next
/// frame. No-op if the entity has no `PointLight`.
pub(in crate::app) fn point_light_grid(ui: &mut egui::Ui, app: &mut App, sel: crate::ecs::Entity) {
    let Some(l) = app.world.get_mut::<crate::components::PointLight>(sel) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label(tr("color", "색상"));
        color_rgb_drags(ui, &mut l.color);
    });
    ui.horizontal(|ui| {
        ui.label(tr("radius", "반지름"));
        ui.add(
            egui::DragValue::new(&mut l.radius)
                .clamp_existing_to_range(false)
                .range(0.0..=4000.0)
                .speed(2.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("intensity", "강도"));
        ui.add(
            egui::DragValue::new(&mut l.intensity)
                .clamp_existing_to_range(false)
                .range(0.0..=10.0)
                .speed(0.05),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("light_height", "빛 높이"));
        ui.add(
            egui::DragValue::new(&mut l.light_height)
                .clamp_existing_to_range(false)
                .range(0.01..=2.0)
                .speed(0.01),
        );
    });
}

/// Edit the global `AmbientLight` resource (color + intensity), inserting a default one first if
/// the game never set it, so the control is always usable.
pub(in crate::app) fn ambient_light_control(ui: &mut egui::Ui, app: &mut App) {
    app.ensure_ambient_light();
    if let Some(amb) = app.world.resource_mut::<crate::resources::AmbientLight>() {
        ui.horizontal(|ui| {
            ui.label(tr("color", "색상"));
            color_rgb_drags(ui, &mut amb.color);
        });
        ui.horizontal(|ui| {
            ui.label(tr("intensity", "강도"));
            ui.add(
                egui::DragValue::new(&mut amb.intensity)
                    .clamp_existing_to_range(false)
                    .range(0.0..=1.0)
                    .speed(0.01),
            );
        });
    }
}

/// Three compact 0..=1 drag editors (r/g/b) for a light [`Color`](crate::color::Color).
/// Lights ignore alpha, so only the RGB channels are exposed.
fn color_rgb_drags(ui: &mut egui::Ui, c: &mut crate::color::Color) {
    for (label, ch) in [("r", &mut c.r), ("g", &mut c.g), ("b", &mut c.b)] {
        ui.add(
            egui::DragValue::new(ch)
                .clamp_existing_to_range(false)
                .range(0.0..=1.0)
                .speed(0.01)
                .prefix(label),
        );
    }
}
