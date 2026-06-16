//! State Machine inspector panel — native only.
//!
//! Lists the selected entity's `AnimationStateMachine` states (current highlighted)
//! with their transitions and parameters, and offers edits — set current, edit clip
//! index, remove state/transition, add state. Snapshots display data under an
//! immutable borrow, collects edit intents during render, then applies them under a
//! fresh mutable borrow.

#![cfg(not(target_arch = "wasm32"))]

use crate::app::App;

/// One-line summary of a transition condition for the State Machine panel.
fn cond_summary(c: &crate::animation::TransitionCond) -> String {
    use crate::animation::TransitionCond as C;
    match c {
        C::BoolEq(n, v) => format!("{n}=={v}"),
        C::FloatGt(n, t) => format!("{n}>{t}"),
        C::FloatLt(n, t) => format!("{n}<{t}"),
        C::Trigger(n) => format!("trig:{n}"),
        C::AnimationEnd => "anim-end".to_string(),
    }
}

/// Display string for a state-machine parameter value.
fn param_display(p: Option<&crate::animation::AnimParam>) -> String {
    use crate::animation::AnimParam as P;
    match p {
        Some(P::Bool(v)) => format!("bool {v}"),
        Some(P::Float(v)) => format!("float {v:.2}"),
        Some(P::Trigger(v)) => format!("trigger {v}"),
        None => "?".to_string(),
    }
}

/// State Machine inspector panel: lists the selected entity's `AnimationStateMachine` states
/// (current highlighted) with their transitions and parameters, and offers edits — set current,
/// edit clip index, remove state/transition, add state. Snapshots display data under an immutable
/// borrow, collects edit intents during render, then applies them under a fresh mutable borrow.
pub(in crate::app) fn state_machine_panel(
    ui: &mut egui::Ui,
    app: &mut App,
    sel: crate::ecs::Entity,
) {
    use crate::animation::{AnimParam, AnimationStateMachine, TransitionCond};

    // ── Snapshot ─────────────────────────────────────────────────────────────
    struct TransView {
        to: String,
        crossfade: f32,
        conditions: Vec<TransitionCond>,
        cond_summary: String,
    }
    struct StateView {
        name: String,
        clip: usize,
        transitions: Vec<TransView>,
    }
    // Param: name, value clone (for mutable widget without holding borrow)
    struct ParamView {
        name: String,
        value: AnimParam,
    }

    let (current, states, param_views, all_state_names): (
        String,
        Vec<StateView>,
        Vec<ParamView>,
        Vec<String>,
    ) = {
        let Some(sm) = app.world.get::<AnimationStateMachine>(sel) else {
            return;
        };
        let current = sm.current_state().to_string();
        let all_state_names = sm.state_names();
        let states = sm
            .state_names()
            .into_iter()
            .map(|name| {
                let st = sm.state(&name).expect("listed state exists");
                let transitions = st
                    .transitions
                    .iter()
                    .map(|t| {
                        let cond_summary = if t.conditions.is_empty() {
                            "always".to_string()
                        } else {
                            t.conditions
                                .iter()
                                .map(cond_summary)
                                .collect::<Vec<_>>()
                                .join(" & ")
                        };
                        TransView {
                            to: t.to.clone(),
                            crossfade: t.crossfade_duration,
                            conditions: t.conditions.clone(),
                            cond_summary,
                        }
                    })
                    .collect();
                StateView {
                    name,
                    clip: st.clip_index,
                    transitions,
                }
            })
            .collect();
        let param_views = sm
            .param_names()
            .into_iter()
            .map(|n| {
                let value = sm.param(&n).cloned().unwrap_or(AnimParam::Bool(false));
                ParamView { name: n, value }
            })
            .collect();
        (current, states, param_views, all_state_names)
    };

    // ── Edit intents ─────────────────────────────────────────────────────────
    #[allow(clippy::enum_variant_names)]
    enum Edit {
        SetCurrent(String),
        RemoveState(String),
        SetClip(String, usize),
        RemoveTransition(String, usize),
        AddState(String),
        // New:
        SetBool(String, bool),
        SetFloat(String, f32),
        FireTrigger(String),
        AddTransition {
            from: String,
            to: String,
            crossfade: f32,
        },
        SetConditions {
            from: String,
            index: usize,
            conditions: Vec<TransitionCond>,
        },
        SetTransitionCrossfade {
            from: String,
            index: usize,
            seconds: f32,
        },
    }
    let mut edits: Vec<Edit> = Vec::new();

    ui.label(format!("current: {current}"));
    let state_count = states.len();
    for st in &states {
        let is_current = st.name == current;
        ui.separator();
        ui.horizontal(|ui| {
            let text = egui::RichText::new(st.name.clone());
            ui.label(if is_current { text.strong() } else { text });
            ui.label("clip");
            let mut clip = st.clip as i32;
            if ui
                .add(egui::DragValue::new(&mut clip).range(0..=4096))
                .changed()
            {
                edits.push(Edit::SetClip(st.name.clone(), clip.max(0) as usize));
            }
            if !is_current && ui.small_button("▶").on_hover_text("set current").clicked() {
                edits.push(Edit::SetCurrent(st.name.clone()));
            }
            if !is_current
                && state_count > 1
                && ui.small_button("✕").on_hover_text("remove state").clicked()
            {
                edits.push(Edit::RemoveState(st.name.clone()));
            }
        });

        // Transitions (with condition editing + crossfade editing + remove)
        for (i, tv) in st.transitions.iter().enumerate() {
            egui::CollapsingHeader::new(format!("→ {}  [{}]", tv.to, tv.cond_summary))
                .id_salt(egui::Id::new(&st.name).with(i).with("trans"))
                .show(ui, |ui| {
                    // Crossfade edit
                    ui.horizontal(|ui| {
                        ui.label("xf");
                        let mut xf = tv.crossfade;
                        if ui
                            .add(
                                egui::DragValue::new(&mut xf)
                                    .speed(0.01)
                                    .range(0.0..=60.0)
                                    .suffix("s"),
                            )
                            .changed()
                        {
                            edits.push(Edit::SetTransitionCrossfade {
                                from: st.name.clone(),
                                index: i,
                                seconds: xf,
                            });
                        }
                        if ui
                            .small_button("✕")
                            .on_hover_text("remove transition")
                            .clicked()
                        {
                            edits.push(Edit::RemoveTransition(st.name.clone(), i));
                        }
                    });

                    // Condition list (edit / remove each condition)
                    let mut new_conds: Option<Vec<TransitionCond>> = None;
                    for (ci, cond) in tv.conditions.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(cond_summary(cond));
                            if ui.small_button("✕").on_hover_text("remove cond").clicked() {
                                let mut c = tv.conditions.clone();
                                c.remove(ci);
                                new_conds = Some(c);
                            }
                        });
                    }
                    if let Some(c) = new_conds {
                        edits.push(Edit::SetConditions {
                            from: st.name.clone(),
                            index: i,
                            conditions: c,
                        });
                    }

                    // Add-condition row
                    let add_cond_key = egui::Id::new(&st.name).with(i).with("add_cond");
                    // Persist selected cond variant + param name + f32 value + bool value in
                    // egui memory (avoids adding more fields to EditorState for each combo).
                    let (mut cond_variant, mut cond_param, mut cond_f, mut cond_b) =
                        ui.memory_mut(|m| {
                            m.data
                                .get_temp_mut_or_insert_with::<(u8, String, f32, bool)>(
                                    add_cond_key,
                                    || (0u8, String::new(), 0.0, true),
                                )
                                .clone()
                        });
                    let variant_names = ["BoolEq", "FloatGt", "FloatLt", "Trigger", "AnimEnd"];
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt(add_cond_key.with("v"))
                            .selected_text(variant_names[cond_variant as usize])
                            .width(80.0)
                            .show_ui(ui, |ui| {
                                for (idx, name) in variant_names.iter().enumerate() {
                                    ui.selectable_value(&mut cond_variant, idx as u8, *name);
                                }
                            });
                        // Show param name input for variants that need it
                        if cond_variant < 4 {
                            ui.add(
                                egui::TextEdit::singleline(&mut cond_param)
                                    .hint_text("param")
                                    .desired_width(60.0),
                            );
                        }
                        // Show value inputs per variant
                        match cond_variant {
                            0 => {
                                // BoolEq
                                ui.checkbox(&mut cond_b, "");
                            }
                            1 | 2 => {
                                // FloatGt / FloatLt
                                ui.add(egui::DragValue::new(&mut cond_f).speed(0.05));
                            }
                            _ => {}
                        }
                        if ui
                            .small_button("+")
                            .on_hover_text("add condition")
                            .clicked()
                        {
                            let new_cond = match cond_variant {
                                0 => Some(TransitionCond::BoolEq(cond_param.clone(), cond_b)),
                                1 => Some(TransitionCond::FloatGt(cond_param.clone(), cond_f)),
                                2 => Some(TransitionCond::FloatLt(cond_param.clone(), cond_f)),
                                3 => Some(TransitionCond::Trigger(cond_param.clone())),
                                _ => Some(TransitionCond::AnimationEnd),
                            };
                            if let Some(c) = new_cond {
                                if matches!(c, TransitionCond::AnimationEnd)
                                    || !cond_param.is_empty()
                                {
                                    let mut updated = tv.conditions.clone();
                                    updated.push(c);
                                    edits.push(Edit::SetConditions {
                                        from: st.name.clone(),
                                        index: i,
                                        conditions: updated,
                                    });
                                }
                            }
                        }
                    });
                    // Write back transient state to egui memory
                    ui.memory_mut(|m| {
                        *m.data
                            .get_temp_mut_or_insert_with::<(u8, String, f32, bool)>(
                                add_cond_key,
                                || (0u8, String::new(), 0.0, true),
                            ) = (cond_variant, cond_param, cond_f, cond_b);
                    });
                });
        }

        // Add-transition row (per state)
        {
            let other_states: Vec<&str> = all_state_names
                .iter()
                .filter(|n| n.as_str() != st.name)
                .map(|n| n.as_str())
                .collect();
            if !other_states.is_empty() {
                let target = app
                    .editor
                    .sm_add_trans_target
                    .entry(st.name.clone())
                    .or_insert_with(|| other_states[0].to_string())
                    .clone();
                let xf = *app
                    .editor
                    .sm_add_trans_xf
                    .entry(st.name.clone())
                    .or_insert(0.0);
                let mut selected_target = target.clone();
                let mut selected_xf = xf;
                let mut do_add = false;
                ui.horizontal(|ui| {
                    ui.label("+trans→");
                    let combo_id = egui::Id::new(&st.name).with("add_trans_combo");
                    egui::ComboBox::from_id_salt(combo_id)
                        .selected_text(&selected_target)
                        .width(80.0)
                        .show_ui(ui, |ui| {
                            for &n in &other_states {
                                ui.selectable_value(&mut selected_target, n.to_string(), n);
                            }
                        });
                    ui.add(
                        egui::DragValue::new(&mut selected_xf)
                            .speed(0.01)
                            .range(0.0..=60.0)
                            .suffix("s"),
                    );
                    if ui
                        .small_button("+")
                        .on_hover_text("add transition")
                        .clicked()
                    {
                        do_add = true;
                    }
                });
                // Persist input state back into editor
                app.editor
                    .sm_add_trans_target
                    .insert(st.name.clone(), selected_target.clone());
                app.editor
                    .sm_add_trans_xf
                    .insert(st.name.clone(), selected_xf);
                if do_add {
                    edits.push(Edit::AddTransition {
                        from: st.name.clone(),
                        to: selected_target,
                        crossfade: selected_xf,
                    });
                }
            }
        }
    }

    // Add-state row
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("add state:");
        ui.add(egui::TextEdit::singleline(&mut app.editor.sm_add_state_name).desired_width(100.0));
        if ui.button("+").clicked() {
            let name = app.editor.sm_add_state_name.trim().to_string();
            if !name.is_empty() {
                edits.push(Edit::AddState(name));
            }
        }
    });

    // Parameters (now editable)
    if !param_views.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new("parameters").weak());
        for pv in &param_views {
            ui.horizontal(|ui| {
                ui.label(&pv.name);
                match &pv.value {
                    AnimParam::Bool(v) => {
                        let mut b = *v;
                        if ui.checkbox(&mut b, "").changed() {
                            edits.push(Edit::SetBool(pv.name.clone(), b));
                        }
                    }
                    AnimParam::Float(v) => {
                        let mut f = *v;
                        if ui.add(egui::DragValue::new(&mut f).speed(0.05)).changed() {
                            edits.push(Edit::SetFloat(pv.name.clone(), f));
                        }
                    }
                    AnimParam::Trigger(_) => {
                        if ui.small_button("fire").clicked() {
                            edits.push(Edit::FireTrigger(pv.name.clone()));
                        }
                        ui.label(param_display(Some(&pv.value)));
                    }
                }
            });
        }
    }

    // ── Apply edits ───────────────────────────────────────────────────────────
    if !edits.is_empty() {
        let added_state = edits.iter().any(|e| matches!(e, Edit::AddState(_)));
        if let Some(sm) = app.world.get_mut::<AnimationStateMachine>(sel) {
            for e in edits {
                match e {
                    Edit::SetCurrent(n) => {
                        sm.set_current_state(&n);
                    }
                    Edit::RemoveState(n) => {
                        sm.remove_state(&n);
                    }
                    Edit::SetClip(n, c) => {
                        sm.set_state_clip(&n, c);
                    }
                    Edit::RemoveTransition(from, i) => {
                        sm.remove_transition(&from, i);
                    }
                    Edit::AddState(n) => {
                        sm.add_state(n, 0);
                    }
                    Edit::SetBool(n, v) => {
                        sm.set_bool(n, v);
                    }
                    Edit::SetFloat(n, v) => {
                        sm.set_float(n, v);
                    }
                    Edit::FireTrigger(n) => {
                        sm.fire_trigger(&n);
                    }
                    Edit::AddTransition {
                        from,
                        to,
                        crossfade,
                    } => {
                        sm.add_transition_crossfade(&from, &to, vec![], crossfade);
                    }
                    Edit::SetConditions {
                        from,
                        index,
                        conditions,
                    } => {
                        sm.set_transition_conditions(&from, index, conditions);
                    }
                    Edit::SetTransitionCrossfade {
                        from,
                        index,
                        seconds,
                    } => {
                        sm.set_transition_crossfade(&from, index, seconds);
                    }
                }
            }
        }
        if added_state {
            app.editor.sm_add_state_name.clear();
        }
    }
}
