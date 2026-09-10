//! Timeline inspector panel — native only.
//!
//! Renders playback controls (duration / loop / play-pause / restart / time scrub)
//! plus a per-track keyframe list (position / rotation / scale / color / alpha / zoom).
//! Edits mutate the `Timeline` component in place via one `get_mut` (disjoint track
//! fields edited sequentially).
//!
//! ⚠️ **These edits are not undoable.** They mutate the component directly and push nothing onto
//! `EditorHistory`, so Ctrl+Z after one undoes whatever gizmo or paint action came *before* it and
//! leaves this panel's change standing. That is the maintainer's decision, not an oversight —
//! making them undoable means an editor command per edit kind, and the panels edit structure
//! (states, keyframes, rows) rather than a value with a clean before/after. The
//! `the_three_panels_record_nothing_on_the_undo_stack` test pins it, so the day one of them starts
//! recording is the day this paragraph gets rewritten.

#![cfg(not(target_arch = "wasm32"))]

use crate::app::editor::tr;
use crate::app::App;

/// Returns all `Easing` variants in display order. Kept in sync with `src/tween.rs`.
fn easing_variants() -> [crate::tween::Easing; 10] {
    use crate::tween::Easing;
    [
        Easing::Linear,
        Easing::EaseIn,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::EaseInBack,
        Easing::EaseOutBack,
        Easing::EaseInBounce,
        Easing::EaseOutBounce,
        Easing::EaseInElastic,
        Easing::EaseOutElastic,
    ]
}

/// Render one [`Track`](crate::timeline::Track) of a `Timeline` as a collapsible keyframe list:
/// each keyframe shows an editable time (re-sorts once the drag is released — see below), a value
/// widget (via `value_edit`), an easing ComboBox, and a remove button. Renders even when empty (shows an "+kf" add button).
/// `make_default` supplies a default value for new keyframes; `value_edit` renders a mutable widget
/// and returns `true` when the value changed.
fn timeline_track_ui<T: Clone + crate::tween::Lerp>(
    ui: &mut egui::Ui,
    id_key: &'static str,
    label: &str,
    track: &mut crate::timeline::Track<T>,
    at_time: f32,
    make_default: impl Fn() -> T,
    value_edit: impl Fn(&mut egui::Ui, &mut T) -> bool,
) {
    let header_text = if track.is_empty() {
        format!("{label} ({})", tr("empty", "비어있음"))
    } else {
        format!("{label} ({} {})", track.len(), tr("kf", "키프레임"))
    };
    egui::CollapsingHeader::new(header_text)
        .id_salt(id_key)
        .show(ui, |ui| {
            // "+kf" button (available whether or not the track has keyframes)
            if ui
                .small_button(tr("+kf", "+키프레임"))
                .on_hover_text(tr(
                    "add keyframe at current time",
                    "현재 시간에 키프레임 추가",
                ))
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

            // A live time drag is held here instead of being written to the track, because
            // `set_time` re-sorts and egui keeps a drag on the widget id it started on — which
            // is the *row index*, not the keyframe. Re-sorting under a live drag therefore hands
            // the drag to whichever keyframe took that row, and the two then advance together,
            // locked at the gap they had when they met: dragging one keyframe past its neighbour
            // pushes the neighbour along instead of passing it. So the drag's value lives here
            // until the pointer is released, and the track is re-sorted exactly once, at the end.
            let pending_id = ui.id().with("pending_retime");
            let mut pending: Option<(usize, f32)> = ui.data(|d| d.get_temp(pending_id));
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
                    // Editable keyframe time. While it is dragged the new time is parked in
                    // `pending` and shown from there; the track is only re-sorted on release.
                    let mut t = match pending {
                        Some((held_row, held_t)) if held_row == i => held_t,
                        _ => t_snap,
                    };
                    let time_resp = ui.add(
                        egui::DragValue::new(&mut t)
                            .speed(0.02)
                            .range(0.0..=3600.0)
                            .suffix("s"),
                    );
                    if time_resp.drag_stopped() {
                        // The release frame carries the last delta, so `t` is the final time.
                        pending = None;
                        retime = Some((i, t));
                    } else if time_resp.dragged() {
                        if time_resp.changed() {
                            pending = Some((i, t));
                        }
                    } else if let Some((_, held_t)) = pending.filter(|(row, _)| *row == i) {
                        // The drag ended while this panel was not being rendered (tab switched,
                        // selection changed). Apply what it had reached rather than dropping it.
                        pending = None;
                        retime = Some((i, held_t));
                    } else if time_resp.changed() {
                        // Keyboard entry — no drag, so re-sorting immediately is safe.
                        retime = Some((i, t));
                    }

                    // Value widget — type-specific, supplied by caller.
                    let mut v = v_snap;
                    if value_edit(ui, &mut v) {
                        revalue = Some((i, v));
                    }

                    // Easing ComboBox — editable for all track types via the Easing enum.
                    let easing_label = format!("{e_snap:?}");
                    let combo_id = egui::Id::new(id_key).with(i).with("ease");
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
                        .on_hover_text(tr("remove keyframe", "키프레임 제거"))
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
                pending = None;
            }
            ui.data_mut(|d| match pending {
                Some(p) => {
                    d.insert_temp(pending_id, p);
                }
                None => d.remove::<(usize, f32)>(pending_id),
            });
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
        ui.label(tr("duration", "지속시간"));
        ui.add(
            egui::DragValue::new(&mut tl.duration)
                .speed(0.05)
                .clamp_existing_to_range(false)
                .range(0.0..=3600.0)
                .suffix("s"),
        );
        ui.checkbox(&mut tl.looping, tr("loop", "반복"));
    });
    let dur = tl.duration.max(0.0);
    ui.horizontal(|ui| {
        let label = if tl.playing {
            tr("⏸ Pause", "⏸ 일시정지")
        } else {
            tr("▶ Play", "▶ 재생")
        };
        if ui.button(label).clicked() {
            tl.playing = !tl.playing;
        }
        if ui.button(tr("⏮ Restart", "⏮ 재시작")).clicked() {
            tl.restart();
        }
        ui.label(tr("time", "시간"));
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
        tr("position", "위치"),
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
        tr("rotation", "회전"),
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
        tr("scale", "스케일"),
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
        tr("color", "색상"),
        &mut tl.color,
        cur_time,
        || crate::color::Color::WHITE,
        |ui, c| {
            let mut changed = false;
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.r)
                        .speed(0.01)
                        .clamp_existing_to_range(false)
                        .range(0.0..=1.0)
                        .prefix("r:"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.g)
                        .speed(0.01)
                        .clamp_existing_to_range(false)
                        .range(0.0..=1.0)
                        .prefix("g:"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.b)
                        .speed(0.01)
                        .clamp_existing_to_range(false)
                        .range(0.0..=1.0)
                        .prefix("b:"),
                )
                .changed();
            changed |= ui
                .add(
                    egui::DragValue::new(&mut c.a)
                        .speed(0.01)
                        .clamp_existing_to_range(false)
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
        tr("alpha", "알파"),
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
        tr("zoom", "줌"),
        &mut tl.zoom,
        cur_time,
        || 1.0f32,
        |ui, v| {
            ui.add(egui::DragValue::new(v).speed(0.01).range(0.01..=32.0))
                .changed()
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One persistent `egui::Context` driven frame by frame with synthesized pointer input.
    ///
    /// The editor's other headless harness (`editor::tests::editor_frame`) builds a **fresh**
    /// context per call, which is enough for a keystroke but blind to anything stateful: a drag
    /// exists only *between* frames. Everything below needs that, so it keeps one context and
    /// advances `time` itself.
    struct Frames {
        ctx: egui::Context,
        time: f64,
    }

    impl Frames {
        fn new() -> Self {
            Self {
                ctx: egui::Context::default(),
                time: 0.0,
            }
        }

        fn run(
            &mut self,
            events: Vec<egui::Event>,
            build: impl FnMut(&mut egui::Ui),
        ) -> egui::FullOutput {
            self.time += 1.0 / 60.0;
            let raw = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            self.ctx.run_ui(raw, build)
        }
    }

    /// Render one frame of a `f32` position-style track. A macro rather than a helper so the
    /// test can read `$track` between frames — a closure holding `&mut track` could not.
    macro_rules! frame {
        ($f:expr, $track:expr, $events:expr) => {
            $f.run($events, |ui| {
                timeline_track_ui(
                    ui,
                    "position",
                    "position",
                    &mut $track,
                    0.0,
                    || 0.0f32,
                    |ui, v| ui.add(egui::DragValue::new(v).speed(1.0)).changed(),
                );
            })
        };
    }

    fn mv(p: egui::Pos2) -> egui::Event {
        egui::Event::PointerMoved(p)
    }

    fn btn(p: egui::Pos2, pressed: bool) -> egui::Event {
        egui::Event::PointerButton {
            pos: p,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        }
    }

    /// Every text drawn this frame, with the rect it was drawn in. This is how a widget is
    /// located without a window: egui hands the frame's shapes back from `run_ui`, and the
    /// keyframe time `DragValue` draws its number ("0.00") next to its suffix ("s").
    fn texts(out: &egui::FullOutput) -> Vec<(String, egui::Rect)> {
        fn walk(s: &egui::Shape, into: &mut Vec<(String, egui::Rect)>) {
            match s {
                egui::Shape::Text(t) => into.push((
                    t.galley.text().to_owned(),
                    egui::Rect::from_min_size(t.pos, t.galley.size()),
                )),
                egui::Shape::Vec(v) => v.iter().for_each(|s| walk(s, into)),
                _ => {}
            }
        }
        let mut v = Vec::new();
        for clipped in &out.shapes {
            walk(&clipped.shape, &mut v);
        }
        v
    }

    fn center_of(out: &egui::FullOutput, text: &str) -> egui::Pos2 {
        texts(out)
            .into_iter()
            .find(|(t, _)| t == text)
            .unwrap_or_else(|| {
                panic!(
                    "no widget drawing {text:?} this frame; drawn: {:?}",
                    texts(out).into_iter().map(|(t, _)| t).collect::<Vec<_>>()
                )
            })
            .1
            .center()
    }

    /// The numbers the keyframe time widgets are showing, top row first.
    fn shown_times(out: &egui::FullOutput) -> Vec<String> {
        let mut rows: Vec<(i32, String)> = texts(out)
            .into_iter()
            .filter(|(t, _)| t.contains('.') && t.parse::<f32>().is_ok())
            .map(|(t, r)| (r.center().y as i32, t))
            .collect();
        rows.sort_by_key(|(y, _)| *y);
        rows.into_iter().map(|(_, t)| t).collect()
    }

    fn two_keyframes() -> crate::timeline::Track<f32> {
        let mut track = crate::timeline::Track::<f32>::new();
        // The values double as name tags: whichever keyframe moved, the value says which.
        track.add(0.0, 0.0, crate::tween::Easing::Linear);
        track.add(1.0, 10.0, crate::tween::Easing::Linear);
        track
    }

    /// Lay out, click the collapsing header open, and return the grab point of the first row's
    /// time widget. Costs four frames — the header's open/close tween needs one to finish.
    fn open_and_grab(f: &mut Frames, track: &mut crate::timeline::Track<f32>) -> egui::Pos2 {
        let out = frame!(f, *track, vec![]);
        let header = center_of(&out, "position (2 키프레임)");
        frame!(f, *track, vec![mv(header), btn(header, true)]);
        frame!(f, *track, vec![btn(header, false)]);
        let out = frame!(f, *track, vec![]);
        center_of(&out, "0.00")
    }

    /// `set_time` re-sorts and egui keeps a drag on the widget id it started on — the row
    /// index. Re-sorting under a live drag therefore handed the drag to whichever keyframe
    /// took that row, and the pair then advanced together at the gap they met with: dragging
    /// the first keyframe from 0.0 to 2.4 s pushed its neighbour from 1.0 to 2.2 s instead of
    /// being passed by it, the two swapping which one the widget drove on every frame.
    #[test]
    fn dragging_a_keyframe_past_its_neighbour_leaves_the_neighbour_where_it_was() {
        let mut track = two_keyframes();
        let mut f = Frames::new();
        let grab = open_and_grab(&mut f, &mut track);

        frame!(f, track, vec![mv(grab), btn(grab, true)]);
        // 0.02 s per point, so 120 px to the right is +2.4 s — well past the neighbour at 1.0.
        for step in 1..=12 {
            frame!(
                f,
                track,
                vec![mv(grab + egui::vec2(10.0 * step as f32, 0.0))]
            );
            let neighbour = track.keyframes().iter().find(|k| k.value == 10.0).unwrap();
            assert_eq!(
                neighbour.time, 1.0,
                "the keyframe that was not grabbed moved during the drag (step {step})"
            );
        }
        frame!(f, track, vec![btn(grab + egui::vec2(120.0, 0.0), false)]);

        let kf = track.keyframes();
        assert_eq!(kf.len(), 2, "a drag must not add or drop a keyframe");
        assert_eq!(
            (kf[0].value, kf[0].time),
            (10.0, 1.0),
            "the neighbour keeps its time and is now first"
        );
        assert_eq!(
            kf[1].value, 0.0,
            "the keyframe that moved is the one that was grabbed"
        );
        assert!(
            (kf[1].time - 2.4).abs() < 1e-4,
            "the grabbed keyframe took the whole drag: {} s, expected 2.4",
            kf[1].time
        );
    }

    /// The fix parks a live drag instead of writing it through, so this pins the half that
    /// makes it a fix rather than a mute: the row keeps showing what the drag has reached
    /// while the track still holds the old time.
    #[test]
    fn a_parked_drag_still_shows_the_time_it_has_reached() {
        let mut track = two_keyframes();
        let mut f = Frames::new();
        let grab = open_and_grab(&mut f, &mut track);

        frame!(f, track, vec![mv(grab), btn(grab, true)]);
        let mut out = frame!(f, track, vec![mv(grab + egui::vec2(10.0, 0.0))]);
        for step in 2..=8 {
            out = frame!(
                f,
                track,
                vec![mv(grab + egui::vec2(10.0 * step as f32, 0.0))]
            );
        }

        // A frame's delta is applied after that frame's text is laid out, so the number drawn
        // is one 10 px step behind the eight the pointer has taken: 7 × 0.2 s, not 8 × 0.2 s.
        assert_eq!(
            shown_times(&out),
            vec!["1.40".to_owned(), "1.00".to_owned()],
            "the dragged row must show the new time, and the untouched row its own"
        );
        assert_eq!(
            track.keyframes()[0].time,
            0.0,
            "the track has not taken the drag yet — it lands on release"
        );
    }

    /// Typing a time is not a drag, so it must still re-sort at once — the branch the fix
    /// added for it is the one a "park everything" version of this fix would have lost.
    #[test]
    fn a_time_typed_into_the_row_re_sorts_immediately() {
        let mut track = two_keyframes();
        let mut f = Frames::new();
        let grab = open_and_grab(&mut f, &mut track);

        // A click with no movement puts the DragValue into keyboard-edit mode.
        frame!(f, track, vec![mv(grab), btn(grab, true)]);
        frame!(f, track, vec![btn(grab, false)]);
        frame!(f, track, vec![]);
        frame!(f, track, vec![egui::Event::Text("2.5".to_owned())]);
        frame!(
            f,
            track,
            vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::default(),
            }]
        );

        let times: Vec<f32> = track.keyframes().iter().map(|k| k.time).collect();
        assert_eq!(
            times,
            vec![1.0, 2.5],
            "the typed time must land and re-sort in the same frame"
        );
        assert_eq!(
            track.keyframes()[1].value,
            0.0,
            "and it must land on the keyframe whose row was typed into"
        );
    }

    /// A parked drag is held in `ui.data`, which is only read while this panel renders. If the
    /// pointer is released with the panel gone — another tab, another selection — the drag must
    /// still land the next time it renders, not be dropped.
    #[test]
    fn a_drag_released_while_the_panel_is_not_rendered_still_lands() {
        let mut track = two_keyframes();
        let mut f = Frames::new();
        let grab = open_and_grab(&mut f, &mut track);

        frame!(f, track, vec![mv(grab), btn(grab, true)]);
        for step in 1..=3 {
            frame!(
                f,
                track,
                vec![mv(grab + egui::vec2(10.0 * step as f32, 0.0))]
            );
        }
        let released_at = grab + egui::vec2(30.0, 0.0);
        // The panel is not built this frame; the release reaches egui all the same.
        f.run(vec![btn(released_at, false)], |_ui| {});
        frame!(f, track, vec![]);

        let dragged = track.keyframes().iter().find(|k| k.value == 0.0).unwrap();
        assert!(
            (dragged.time - 0.6).abs() < 1e-4,
            "the parked drag was dropped: {:?}",
            track.keyframes().iter().map(|k| k.time).collect::<Vec<_>>()
        );
    }
}
