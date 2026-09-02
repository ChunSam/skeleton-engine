//! Particle live-tuner panel — native only.
//!
//! Renders the live-tunable fields of a selected entity's [`ParticleEmitter`] as a column
//! of drag editors in the inspector. Extracted verbatim from `docked.rs`; called from the
//! per-component inspector dispatch in `component_registry.rs`.
//!
//! ⚠️ Every `DragValue` here binds the component's field directly and carries
//! `.clamp_existing_to_range(false)`: egui's default clamps the *existing* value to the widget's
//! range on display, so merely selecting an emitter at `spawn_rate 4000` used to write `2000`
//! into the world with no interaction (v0.156.13). The range still bounds what a drag produces.
//!
//! [`ParticleEmitter`]: crate::particle::ParticleEmitter

#![cfg(not(target_arch = "wasm32"))]

use crate::app::editor::tr;
use crate::app::App;

/// Render the live-tunable fields of the selected entity's `ParticleEmitter` as a column of
/// drag editors. Edits mutate the component in place, so they take effect on the next spawn while
/// the simulation runs. No-op (renders nothing) if the entity has no `ParticleEmitter`.
pub(in crate::app) fn particle_tuner_grid(
    ui: &mut egui::Ui,
    app: &mut App,
    sel: crate::ecs::Entity,
) {
    let Some(em) = app.world.get_mut::<crate::particle::ParticleEmitter>(sel) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.checkbox(&mut em.emit, tr("emit", "방출"));
    });
    ui.horizontal(|ui| {
        ui.label(tr("spawn_rate", "생성률"));
        ui.add(
            egui::DragValue::new(&mut em.spawn_rate)
                .clamp_existing_to_range(false)
                .range(0.0..=2000.0)
                .speed(1.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("max/frame", "프레임당 최대"));
        ui.add(
            egui::DragValue::new(&mut em.max_per_frame)
                .clamp_existing_to_range(false)
                .range(1..=8192)
                .speed(1.0),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("lifetime", "수명"));
        ui.add(
            egui::DragValue::new(&mut em.lifetime)
                .clamp_existing_to_range(false)
                .range(0.0..=60.0)
                .speed(0.05)
                .suffix(" s"),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("velocity", "속도"));
        ui.add(
            egui::DragValue::new(&mut em.velocity.x)
                .speed(1.0)
                .prefix("x "),
        );
        ui.add(
            egui::DragValue::new(&mut em.velocity.y)
                .speed(1.0)
                .prefix("y "),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("vel spread", "속도 분산"));
        ui.add(
            egui::DragValue::new(&mut em.velocity_spread.x)
                .clamp_existing_to_range(false)
                .range(0.0..=10000.0)
                .speed(1.0)
                .prefix("x "),
        );
        ui.add(
            egui::DragValue::new(&mut em.velocity_spread.y)
                .clamp_existing_to_range(false)
                .range(0.0..=10000.0)
                .speed(1.0)
                .prefix("y "),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("size", "크기"));
        ui.add(
            egui::DragValue::new(&mut em.size.x)
                .clamp_existing_to_range(false)
                .range(0.0..=512.0)
                .speed(0.5)
                .prefix("w "),
        );
        ui.add(
            egui::DragValue::new(&mut em.size.y)
                .clamp_existing_to_range(false)
                .range(0.0..=512.0)
                .speed(0.5)
                .prefix("h "),
        );
    });
    ui.horizontal(|ui| {
        ui.label(tr("color start", "시작 색상"));
        color_rgba_drags(ui, &mut em.color_start);
    });
    ui.horizontal(|ui| {
        ui.label(tr("color end", "끝 색상"));
        color_rgba_drags(ui, &mut em.color_end);
    });
}

/// Four compact 0..=1 drag editors (r/g/b/a) for an engine [`Color`](crate::color::Color).
/// Exact (no sRGB round-trip), so it is safe for the linear-space particle colors.
fn color_rgba_drags(ui: &mut egui::Ui, c: &mut crate::color::Color) {
    for (label, ch) in [
        ("r", &mut c.r),
        ("g", &mut c.g),
        ("b", &mut c.b),
        ("a", &mut c.a),
    ] {
        ui.add(
            egui::DragValue::new(ch)
                .clamp_existing_to_range(false)
                .range(0.0..=1.0)
                .speed(0.01)
                .prefix(label),
        );
    }
}
