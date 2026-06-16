//! Timeline inspector panel — native only.
//!
//! Renders playback controls (duration / loop / play-pause / restart / time scrub)
//! plus a per-track keyframe list (position / rotation / scale / color / alpha / zoom).
//! Edits mutate the `Timeline` component in place via one `get_mut` (disjoint track
//! fields edited sequentially).

#![cfg(not(target_arch = "wasm32"))]

use crate::app::App;

/// Returns all `Easing` variants in display order. Kept in sync with `src/tween.rs`.
fn easing_variants() -> [crate::tween::Easing; 6] {
    use crate::tween::Easing;
    [
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::EaseInBack,
        Easing::EaseOutBack,
    ]
}

/// Render one [`Track`](crate::timeline::Track) of a `Timeline` as a collapsible keyframe list:
/// each keyframe shows an editable time (re-sorts on change), a value widget (via `value_edit`), an
/// easing ComboBox, and a remove button. Renders even when empty (shows an "+kf" add button).
/// `make_default` supplies a default value for new keyframes; `value_edit` renders a mutable widget
/// and returns `true` when the value changed.
fn timeline_track_ui<T: Clone + crate::tween::Lerp>(
    ui: &mut egui::Ui,
    label: &str,
    track: &mut crate::timeline::Track<T>,
    at_time: f32,
    make_default: impl Fn() -> T,
    value_edit: impl Fn(&mut egui::Ui, &mut T) -> bool,
) {
    let header_text = if track.is_empty() {
        format!("{label} (empty)")
    } else {
        format!("{label} ({} kf)", track.len())
    };
    egui::CollapsingHeader::new(header_text)
        .id_salt(label)
        .show(ui, |ui| {
            // "+kf" button (available whether or not the track has keyframes)
            if ui
                .small_button("+kf")
                .on_hover_text("add keyframe at current time")
                .clicked()
            {
                track.add(at_time, make_default(), crate::tween::Easing::Linear);
            }

            if track.is_empty() {
                return;
            }

            // Collect deferred mutations — must not mutate `track` while iterating
            // `track.keyframes()` since the slice borrow would conflict.
            let mut retime: Option<(usize, f32)> = None;
            let mut rease: Option<(usize, crate::tween::Easing)> = None;
            let mut remove: Option<usize> = None;
            let mut revalue: Option<(usize, T)> = None;

            // Clone all values so we can hand mutable temporaries to `value_edit` without
            // holding an immutable reference to `track` at the same time.
            let kf_snapshots: Vec<_> = track
                .keyframes()
                .iter()
                .map(|kf| (kf.time, kf.value.clone(), kf.easing.clone()))
                .collect();

            for (i, (t_snap, v_snap, e_snap)) in kf_snapshots.into_iter().enumerate() {
                ui.horizontal(|ui| {
                    // Editable keyframe time (re-sorts on change).
                    let mut t = t_snap;
                    if ui
                        .add(
                            egui::DragValue::new(&mut t)
                                .speed(0.02)
                                .range(0.0..=3600.0)
                                .suffix("s"),
                        )
                        .changed()
                    {
                        retime = Some((i, t));
                    }

                    // Value widget — type-specific, supplied by caller.
                    let mut v = v_snap;
                    if value_edit(ui, &mut v) {
                        revalue = Some((i, v));
                    }

                    // Easing ComboBox — editable for all track types via the Easing enum.
                    let easing_label = format!("{e_snap:?}");
                    let combo_id = egui::Id::new(label).with(i).with("ease");
                    egui::ComboBox::from_id_salt(combo_id)
                        .selected_text(&easing_label)
                        .width(100.0)
                        .show_ui(ui, |ui| {
                            for variant in easing_variants() {
                                let name = format!("{variant:?}");
                                // Compare by debug string since Easing is not PartialEq.
                                let selected = name == easing_label;
                                if ui.selectable_label(selected, &name).clicked() {
                                    // `variant` is owned (moved from the array), no clone needed.
                                    rease = Some((i, variant));
                                }
                            }
                        });

                    if ui
                        .small_button("✕")
                        .on_hover_text("remove keyframe")
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
            }
            // Apply deferred mutations in a safe order: value + easing first (index-stable),
            // retime next (may re-sort), remove last (changes indices).
            if let Some((i, v)) = revalue {
                track.set_value(i, v);
            }
            if let Some((i, e)) = rease {
                track.set_easing(i, e);
            }
            if let Some((i, t)) = retime {
                track.set_time(i, t);
            }
            if let Some(i) = remove {
                track.remove(i);
            }
        });
}

/// Timeline inspector panel: playback controls (duration / loop / play-pause / restart / time scrub)
/// plus a per-track keyframe list (position / rotation / scale / color / alpha / zoom). Edits mutate
/// the `Timeline` component in place via one `get_mut` (disjoint track fields edited sequentially).
pub(in crate::app) fn timeline_panel(ui: &mut egui::Ui, app: &mut App, sel: crate::ecs::Entity) {
    let Some(tl) = app.world.get_mut::<crate::timeline::Timeline>(sel) else {
        return;
    };
    ui.horizontal(|ui| {
        ui.label("duration");
        ui.add(
            egui::DragValue::new(&mut tl.duration)
                .speed(0.05)
                .range(0.0..=3600.0)
                .suffix("s"),
        );
        ui.checkbox(&mut tl.looping, "loop");
    });
    let dur = tl.duration.max(0.0);
    ui.horizontal(|ui| {
        let label = if tl.playing { "⏸ Pause" } else { "▶ Play" };
        if ui.button(label).clicked() {
            tl.playing = !tl.playing;
        }
        if ui.button("⏮ Restart").clicked() {
            tl.restart();
        }
        ui.label("time");
        ui.add(
            egui::DragValue::new(&mut tl.time)
                .speed(0.02)
                .range(0.0..=dur),
        );
    });
    let cur_time = tl.time;
    ui.separator();
    timeline_track_ui(
        ui,
        "position",
        &mut tl.position,
        cur_time,
        || glam::Vec2::ZERO,
        |ui, v| {
            let mut changed = false;
            changed |= ui
                .add(egui::DragValue::new(&mut v.x).speed(1.0).prefix("x:"))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut v.y).speed(1.0).prefix("y:"))
                .changed();
            changed
        },
    );
    timeline_track_ui(
        ui,
        "rotation",
        &mut tl.rotation,
        cur_time,
        || 0.0f32,
        |ui, v| {
            ui.add(egui::DragValue::new(v).speed(0.01).suffix("rad"))
                .changed()
        },
    );
    timeline_track_ui(
        ui,
        "scale",
        &mut tl.scale,
        cur_time,
        || glam::Vec2::ZERO,
        |ui, v| {
            let mut changed = false;
            changed |= ui
                .add(egui::DragValue::new(&mut v.x).speed(0.01).prefix("x:"))
                .changed();
            changed |= ui
                .add(egui::DragValue::new(&mut v.y).speed(0.01).prefix("y:"))
                .changed();
            changed
        },
    );
    timeline_track_ui(
        ui,
        "color",
        &mut tl.color,
        cur_time,
        || crate::color::Color::WHITE,
        |ui, c| {
            let mut changed = false;
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.r)
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("r:"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.g)
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("g:"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.b)
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("b:"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.a)
                        .speed(0.01)
                        .range(0.0..=1.0)
                        .prefix("a:"),
                )
                .changed();
            changed
        },
    );
    timeline_track_ui(
        ui,
        "alpha",
        &mut tl.alpha,
        cur_time,
        || 1.0f32,
        |ui, v| {
            ui.add(egui::DragValue::new(v).speed(0.01).range(0.0..=1.0))
                .changed()
        },
    );
    timeline_track_ui(
        ui,
        "zoom",
        &mut tl.zoom,
        cur_time,
        || 1.0f32,
        |ui, v| {
            ui.add(egui::DragValue::new(v).speed(0.01).range(0.01..=32.0))
                .changed()
        },
    );
}
