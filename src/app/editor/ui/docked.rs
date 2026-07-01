//! Docked editor UI — native only; excluded from wasm builds entirely.
#![cfg(not(target_arch = "wasm32"))]

use super::grid_overlay::draw_editor_grid;
use super::lighting_panel::ambient_light_control;
use super::*;
use crate::app::editor::theme;
use crate::app::editor::tr;
use crate::app::editor::EditorMode;
use crate::app::editor::EntitySortMode;

/// Full docked editor layout (Package 2).
///
/// Layout order (egui panel rules require this order):
///   1. Top toolbar panel
///   2. Bottom assets panel
///   3. Left entities/scene panel
///   4. Right inspector panel
///   5. Central panel  ← game image; also writes `EditorState::central_rect`
///
/// Panels are added in the order required by egui — top/bottom first, then
/// left/right, and CentralPanel last. egui 0.34 deprecates the top-level
/// `Panel::show(ctx, ...)` API in favour of `show_inside(ui, ...)`.  At the
/// top level there is no parent `Ui`, so we use the deprecated `show(ctx)`
/// path with `#[allow(deprecated)]`, matching Package 1's convention.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub(in crate::app) fn update_docked_ui(
    ctx: &egui::Context,
    app: &mut App,
    comp_fields: &mut super::InspectorCompFields,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
    selected_comp_names: &[&'static str],
    scene_graph_data: &[(Entity, Option<Entity>)],
    children_map: &HashMap<Entity, Vec<Entity>>,
    root_entities: &[Entity],
) {
    // ── 1. Top toolbar ───────────────────────────────────────────────────────
    // `Panel::top(...).show(ctx, ...)` is the only top-level path in egui 0.34;
    // `show_inside` requires a parent `Ui` that does not exist at this level.
    // The deprecation warning is suppressed — see Package 1 comment for details.
    #[allow(deprecated)]
    egui::Panel::top("docked_toolbar")
        .exact_size(28.0)
        .show(ctx, |ui| {
            docked_toolbar(ui, app);
        });

    // ── 2. Bottom panel: Assets | Data Tables | Audio ───────────────────────
    // High upper bound so the data-table grid can be dragged tall enough to read
    // many rows at once; egui still clamps the drag to the real available height,
    // so this is effectively "free" without letting the panel cover the toolbar.
    #[allow(deprecated)]
    egui::Panel::bottom("docked_assets")
        .default_size(theme::BOTTOM_PANEL_DEFAULT_H)
        .size_range(theme::BOTTOM_PANEL_MIN_H..=theme::BOTTOM_PANEL_MAX_H)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.editor.bottom_tab == 0, tr("Assets", "에셋"))
                    .clicked()
                {
                    app.editor.bottom_tab = 0;
                }
                if ui
                    .selectable_label(
                        app.editor.bottom_tab == 1,
                        tr("Data Tables", "데이터 테이블"),
                    )
                    .clicked()
                {
                    app.editor.bottom_tab = 1;
                }
                if ui
                    .selectable_label(app.editor.bottom_tab == 2, tr("Audio", "오디오"))
                    .clicked()
                {
                    app.editor.bottom_tab = 2;
                }
            });
            ui.separator();
            match app.editor.bottom_tab {
                1 => super::data_table_panel_body(ui, app),
                2 => super::audio_mixer_panel_body(ui, app),
                _ => assets_tab_body(ui, app),
            }
        });

    // ── 3. Left entities / scene panel ───────────────────────────────────────
    #[allow(deprecated)]
    egui::Panel::left("docked_left")
        .default_size(theme::LEFT_PANEL_DEFAULT_W)
        .size_range(theme::LEFT_PANEL_MIN_W..=theme::LEFT_PANEL_MAX_W)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(app.editor.inspector_tab == 0, tr("Entities", "엔티티"))
                    .clicked()
                {
                    app.editor.inspector_tab = 0;
                }
                if ui
                    .selectable_label(app.editor.inspector_tab == 2, tr("Scene", "씬"))
                    .clicked()
                {
                    app.editor.inspector_tab = 2;
                }
            });
            ui.separator();
            if app.editor.inspector_tab == 2 {
                scene_tab_body(
                    ui,
                    app,
                    tag_map,
                    scene_graph_data,
                    children_map,
                    root_entities,
                );
            } else {
                entities_tab_body(ui, app, entity_list, tag_map);
            }
        });

    // ── 4. Right inspector panel ─────────────────────────────────────────────
    #[allow(deprecated)]
    egui::Panel::right("docked_inspector")
        .default_size(theme::RIGHT_PANEL_DEFAULT_W)
        .size_range(theme::RIGHT_PANEL_MIN_W..=theme::RIGHT_PANEL_MAX_W)
        .resizable(true)
        .show(ctx, |ui| {
            ui.strong(tr("Inspector", "인스펙터"));
            ui.separator();
            inspector_tab_body(
                ui,
                app,
                comp_fields,
                selected_comp_names,
                tag_map,
                entity_list,
            );
            save_load_controls(ui, app, entity_list, tag_map);
        });

    // ── 5. Central panel (game image) ─────────────────────────────────────────
    // This must be last. After layout we capture the inner rect and write it to
    // `editor.central_rect` so that the RT/ViewportSize logic tracks real panel bounds.
    #[allow(deprecated)]
    let central_response = egui::CentralPanel::default()
        .frame(egui::Frame::NONE.fill(theme::VIEWPORT_FRAME_FILL))
        .show(ctx, |ui| {
            if let Some(tex) = app.editor.docked_texture_id {
                let avail = ui.available_size();
                let img_rect = ui.image((tex, avail)).rect;
                if app.editor.show_grid {
                    draw_editor_grid(ui, app, img_rect);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.label(tr("(no game frame yet)", "(게임 프레임 없음)"));
                });
            }
        });

    // Write the real central rect in LOGICAL points so the RT/ViewportSize
    // delegation in schedule.rs picks up the actual panel geometry.
    // `inner_rect` is the content area inside panel margins.
    app.editor.central_rect = Some(central_response.response.rect);
}

// ── Toolbar ─────────────────────────────────────────────────────────────────

/// Toolbar contents: ▶/⏸, ⏭ step, Snap, scene path + Save/Load, Exit(F2).
#[cfg(not(target_arch = "wasm32"))]
fn docked_toolbar(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        // ▶ / ⏸ toggle
        let pause_label = if app.editor.paused {
            tr("▶ Resume", "▶ 재개")
        } else {
            tr("⏸ Pause", "⏸ 일시정지")
        };
        if ui.button(pause_label).clicked() {
            app.editor.paused = !app.editor.paused;
            if !app.editor.paused {
                app.editor.step_once = false;
            }
        }

        // ⏭ step-one-frame — only when paused
        ui.add_enabled_ui(app.editor.paused, |ui| {
            if ui.button(tr("⏭ Step", "⏭ 스텝")).clicked() {
                app.editor.step_once = true;
            }
        });

        ui.separator();

        // Snap toggle + grid size
        ui.checkbox(&mut app.editor.snap_enabled, tr("Snap", "스냅"));
        if app.editor.snap_enabled {
            ui.add(
                egui::DragValue::new(&mut app.editor.snap_size)
                    .range(1.0..=128.0)
                    .speed(1.0)
                    .suffix(" px"),
            );
        }
        // Grid overlay toggle (world-aligned to the snap size).
        ui.checkbox(&mut app.editor.show_grid, tr("Grid", "그리드"));
        // Debug bounds/colliders overlay toggle.
        ui.checkbox(&mut app.editor.show_bounds, tr("Bounds", "경계"));
        // Pathfinding-grid overlay toggle (per-Tilemap walkable/blocked cells).
        ui.checkbox(&mut app.editor.show_pathgrid, tr("Path", "패스"))
            .on_hover_text(tr(
                "show the pathfinding grid (non-zero tile = blocked) for each Tilemap",
                "각 타일맵의 경로 탐색 그리드 표시 (0이 아닌 타일 = 막힘)",
            ));
        // Persist current editor preferences now (also auto-saved on closing the editor).
        if ui
            .button(tr("💾 Set.", "💾 설정"))
            .on_hover_text(tr(
                "save editor settings (snap / grid / paint tool)",
                "에디터 설정 저장 (스냅 / 그리드 / 페인트 도구)",
            ))
            .clicked()
        {
            app.save_editor_settings();
        }
        // Editor UI language toggle (English / Korean); persists immediately.
        if ui
            .button(app.editor.locale.label())
            .on_hover_text(tr(
                "switch editor language (EN / 한국어)",
                "에디터 언어 전환 (EN / 한국어)",
            ))
            .clicked()
        {
            app.editor.locale = app.editor.locale.toggled();
            app.save_editor_settings();
        }
        // Keyboard-shortcuts cheatsheet toggle (also bound to the `?` key).
        if ui
            .selectable_label(app.editor.show_shortcuts, tr("? Keys", "? 단축키"))
            .on_hover_text(tr(
                "show the keyboard-shortcuts cheatsheet (or press ?)",
                "키보드 단축키 안내 표시 (또는 ? 키)",
            ))
            .clicked()
        {
            app.editor.show_shortcuts = !app.editor.show_shortcuts;
        }

        ui.separator();

        // Scene path + Save + Load
        ui.add(egui::TextEdit::singleline(&mut app.editor.editor_save_path).desired_width(160.0));

        if ui.button(tr("💾 Save", "💾 저장")).clicked() {
            do_save_scene(app);
        }
        if ui.button(tr("📂 Load", "📂 불러오기")).clicked() {
            do_load_scene(app);
        }

        if let Some(msg) = &app.editor.editor_save_status {
            ui.small(msg.as_str());
        }
        if let Some(msg) = &app.editor.editor_load_status {
            ui.small(msg.as_str());
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(tr("Exit (F2)", "종료 (F2)")).clicked() {
                app.editor.mode = EditorMode::Off;
                app.editor.paused = false;
                app.editor.step_once = false;
                if let Some(debug_ui) = app.world.resource_mut::<DebugUi>() {
                    debug_ui.set_enabled(false);
                }
            }
        });
    });
}

// ── Shared tab-body functions ────────────────────────────────────────────────
//
// Each of these renders a self-contained piece of UI into the given `ui`.
// They are called from BOTH the docked panels (above) and the overlay windows
// (`ui/mod.rs`).  The overlay windows must NOT change behaviour: the bodies
// only mutate `app.editor.*` and `app.world` through well-defined paths.

/// An entity's editor "kind", derived from its most salient component. Backs both the per-row
/// type-icon (via [`EntityKind::icon`]) and the "sort by kind" order (the variant order = the group
/// order). The classification is a single priority ladder ([`entity_kind`]) so icon and sort never
/// drift apart.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EntityKind {
    Light,
    Tilemap,
    Particles,
    Camera,
    Animation,
    Ui,
    Sprite,
    Transform,
    Bare,
}

#[cfg(not(target_arch = "wasm32"))]
impl EntityKind {
    /// The one-glyph editor icon for this kind. Glyphs are chosen from egui's bundled emoji set
    /// (verified to render, not □ tofu, in the headless docked capture).
    fn icon(self) -> &'static str {
        match self {
            EntityKind::Light => "💡",
            EntityKind::Tilemap => "🗺",
            EntityKind::Particles => "✨",
            EntityKind::Camera => "🎥",
            EntityKind::Animation => "🎬",
            EntityKind::Ui => "🔘",
            EntityKind::Sprite => "🖼",
            EntityKind::Transform => "🔹",
            EntityKind::Bare => "·",
        }
    }
}

/// Classify an entity by its most salient component, checked in priority order (first match wins) —
/// so a light that also has a sprite still classifies as [`EntityKind::Light`]. A transform-only
/// entity is [`EntityKind::Transform`]; a bare / marker-only one is [`EntityKind::Bare`]. Native-only
/// (the whole docked UI is); a pure `world.get` scan, so it never mutates.
#[cfg(not(target_arch = "wasm32"))]
fn entity_kind(world: &crate::World, e: Entity) -> EntityKind {
    use crate::{
        AnimationPlayer, AnimationStateMachine, AtlasSprite, Button, CameraTarget, CheckBox, Label,
        NineSlice, Panel, ParticleEmitter, PointLight, ShaderMaterial, Slider, Sprite, TextInput,
        Tilemap, Transform, UiNode,
    };
    if world.get::<PointLight>(e).is_some() {
        EntityKind::Light
    } else if world.get::<Tilemap>(e).is_some() {
        EntityKind::Tilemap
    } else if world.get::<ParticleEmitter>(e).is_some() {
        EntityKind::Particles
    } else if world.get::<CameraTarget>(e).is_some() {
        EntityKind::Camera
    } else if world.get::<AnimationPlayer>(e).is_some()
        || world.get::<AnimationStateMachine>(e).is_some()
    {
        EntityKind::Animation
    } else if world.get::<UiNode>(e).is_some()
        || world.get::<Button>(e).is_some()
        || world.get::<Label>(e).is_some()
        || world.get::<TextInput>(e).is_some()
        || world.get::<Slider>(e).is_some()
        || world.get::<CheckBox>(e).is_some()
        || world.get::<Panel>(e).is_some()
    {
        EntityKind::Ui
    } else if world.get::<Sprite>(e).is_some()
        || world.get::<AtlasSprite>(e).is_some()
        || world.get::<NineSlice>(e).is_some()
        || world.get::<ShaderMaterial>(e).is_some()
    {
        EntityKind::Sprite
    } else if world.get::<Transform>(e).is_some() {
        EntityKind::Transform
    } else {
        EntityKind::Bare
    }
}

/// A small per-row glyph hinting at an entity's kind — a light 💡 vs a sprite 🖼 vs a tilemap 🗺 vs a
/// UI widget 🔘 — drawn before the label in the Entities list and the Scene tree so an entity's type
/// is legible at a glance. Thin wrapper over [`entity_kind`] + [`EntityKind::icon`].
#[cfg(not(target_arch = "wasm32"))]
fn entity_type_icon(world: &crate::World, e: Entity) -> &'static str {
    entity_kind(world, e).icon()
}

/// A display-only ordering of `entity_list` for the Entities tab. Sorts a **copy** — the world's
/// entity order and the scene-save order are untouched. `Insertion` returns the raw order unchanged;
/// `Name` sorts case-insensitively by label; `Kind` groups by [`EntityKind`] (variant order) then by
/// name. The sort is stable, so equal keys keep their insertion order.
#[cfg(not(target_arch = "wasm32"))]
fn sorted_entity_list(
    entity_list: &[Entity],
    mode: EntitySortMode,
    world: &crate::World,
    tag_map: &HashMap<Entity, String>,
) -> Vec<Entity> {
    let mut v = entity_list.to_vec();
    let name_key = |e: Entity| entity_label(e, tag_map).to_lowercase();
    // `sort_by_key` is stable, so within a sort group equal keys keep their insertion order.
    match mode {
        EntitySortMode::Insertion => {}
        EntitySortMode::Name => v.sort_by_key(|&e| name_key(e)),
        EntitySortMode::Kind => v.sort_by_key(|&e| (entity_kind(world, e), name_key(e))),
    }
    v
}

/// A right-click action offered on an Entities-list row. Dispatched through
/// [`App::editor_apply_entity_context_action`] so the wiring stays testable without egui.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntityContextAction {
    Rename,
    Duplicate,
    Focus,
    Delete,
}

#[cfg(not(target_arch = "wasm32"))]
impl App {
    /// Apply an Entities-list right-click [`EntityContextAction`] to `entity`. Selects `entity`
    /// first, so the selection-scoped duplicate/delete/focus ops act on the right-clicked row (not
    /// whatever happened to be selected before), then runs the op. A dead entity is a no-op. Drives
    /// the same public ops the toolbar/shortcuts use; native-only. Module-private (its only callers —
    /// `entities_tab_body` and the tests — live in this file), so the private `EntityContextAction`
    /// never leaks through a more-public signature.
    fn editor_apply_entity_context_action(&mut self, entity: Entity, action: EntityContextAction) {
        if !self.world.is_alive(entity) {
            return;
        }
        self.editor.inspector_selected = Some(entity);
        self.editor.selected_entities = vec![entity];
        match action {
            EntityContextAction::Rename => self.editor_begin_rename(entity),
            EntityContextAction::Duplicate => self.editor_duplicate_selection(),
            EntityContextAction::Focus => self.editor_focus_camera_on_selection(),
            EntityContextAction::Delete => self.editor_delete_selection(),
        }
    }
}

/// Entity list tab body.  Shows a flat, multi-selectable list of all entities.
///
/// Used in: docked left panel (Entities tab), overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn entities_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    // Action row: + New Entity, Delete, Duplicate
    ui.horizontal(|ui| {
        if ui.button(tr("＋ New Entity", "＋ 새 엔티티")).clicked() {
            let e = app.world.spawn();
            app.world
                .add_component(e, crate::components::Transform::default());
            app.world
                .add_component(e, crate::prefab::Tag("New Entity".into()));
            app.editor.inspector_selected = Some(e);
            app.editor.selected_entities = vec![e];
            app.editor
                .cmd_history
                .push(super::super::EditorCmd::CreateEntity {
                    entity: e,
                    def: None,
                });
        }
        if let Some(sel) = app.editor.inspector_selected {
            if ui
                .add_enabled(true, egui::Button::new(tr("🗑 Delete", "🗑 삭제")))
                .clicked()
            {
                // Capture the full entity def before despawning so undo can restore
                // all components (not just tag/transform/sprite).
                let def = super::super::entity_to_def(&app.world, sel).unwrap_or_default();
                app.editor
                    .cmd_history
                    .push(super::super::EditorCmd::DeleteEntity { entity: None, def });
                app.world.despawn(sel);
                app.editor.inspector_selected = None;
                app.editor.selected_entities.retain(|&x| x != sel);
            }
            if ui
                .add_enabled(true, egui::Button::new(tr("⎘ Duplicate", "⎘ 복제")))
                .clicked()
            {
                if let Some(new_entity) = app.world.clone_entity(sel) {
                    if let Some(t) = app
                        .world
                        .get_mut::<crate::components::Transform>(new_entity)
                    {
                        t.position += glam::Vec2::new(16.0, 16.0);
                    }
                    app.editor.inspector_selected = Some(new_entity);
                    app.editor.selected_entities = vec![new_entity];
                    // Capture the def after offset so redo restores the same position.
                    let def = super::super::entity_to_def(&app.world, new_entity);
                    app.editor
                        .cmd_history
                        .push(super::super::EditorCmd::CreateEntity {
                            entity: new_entity,
                            def,
                        });
                }
            }
        }
    });
    ui.separator();

    // Search box: filters the list by entity label (case-insensitive substring).
    ui.horizontal(|ui| {
        ui.label("🔍");
        ui.text_edit_singleline(&mut app.editor.entity_filter);
        if !app.editor.entity_filter.is_empty() && ui.small_button("✕").clicked() {
            app.editor.entity_filter.clear();
        }
    });

    // Sort control: order the displayed list by raw insertion (Default) / name (A–Z) / kind (Type).
    // Display-only — the world's entity order and scene-save order are untouched.
    ui.horizontal(|ui| {
        ui.label(tr("Sort:", "정렬:"));
        let mut mode = app.editor.entity_sort;
        for (m, en, ko) in [
            (EntitySortMode::Insertion, "Default", "기본"),
            (EntitySortMode::Name, "A–Z", "이름"),
            (EntitySortMode::Kind, "Type", "종류"),
        ] {
            if ui.selectable_label(mode == m, tr(en, ko)).clicked() {
                mode = m;
            }
        }
        app.editor.entity_sort = mode;
    });

    // Flat entity list with multi-select, ordered by the chosen sort mode (a display-only copy).
    let filter = app.editor.entity_filter.clone();
    let display = sorted_entity_list(entity_list, app.editor.entity_sort, &app.world, tag_map);
    // A right-click menu writes its chosen (entity, action) here; applied after the list is drawn
    // (collect-then-apply, so the menu closure never has to mutate `app` mid-iteration).
    let mut ctx_action: Option<(Entity, EntityContextAction)> = None;
    egui::ScrollArea::vertical()
        .id_salt("docked_ent_list")
        .show(ui, |ui| {
            for &e in &display {
                let label = entity_label(e, tag_map);
                if !super::super::entity_matches_filter(&label, &filter) {
                    continue;
                }
                let is_sel = app.editor.selected_entities.contains(&e);
                let hidden = app.world.get::<crate::components::Hidden>(e).is_some();
                let icon = entity_type_icon(&app.world, e);
                ui.horizontal(|ui| {
                    // Per-entity visibility toggle: a filled eye = visible, slashed = hidden. Adds /
                    // removes the `Hidden` component (the sprite pass skips Hidden entities).
                    let (eye, hint) = if hidden {
                        (
                            "\u{1F648}",
                            tr("hidden — click to show", "숨김 — 클릭하여 표시"),
                        )
                    } else {
                        (
                            "\u{1F441}",
                            tr("visible — click to hide", "표시 — 클릭하여 숨김"),
                        )
                    };
                    if ui.small_button(eye).on_hover_text(hint).clicked() {
                        if hidden {
                            app.world.remove_component::<crate::components::Hidden>(e);
                        } else {
                            app.world.add_component(e, crate::components::Hidden);
                        }
                    }
                    // Type glyph: a subtle per-row hint at the entity's kind (light / sprite /
                    // tilemap / UI / …), drawn between the eye toggle and the label.
                    ui.label(egui::RichText::new(icon).weak());
                    // Inline rename: while this row is being renamed, draw a focused text box in
                    // place of the label. Enter or clicking away commits (writes the Tag); Escape
                    // cancels. Otherwise draw the (selectable) label; a double-click starts a rename.
                    let renaming = matches!(&app.editor.entity_rename, Some(r) if r.entity == e);
                    if renaming {
                        let (mut commit, mut cancel) = (false, false);
                        if let Some(rn) = app.editor.entity_rename.as_mut() {
                            let resp = ui.text_edit_singleline(&mut rn.buffer);
                            if rn.focus_pending {
                                resp.request_focus();
                                rn.focus_pending = false;
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                cancel = true;
                            } else if resp.lost_focus() {
                                commit = true;
                            }
                        }
                        if commit {
                            app.editor_commit_rename();
                        } else if cancel {
                            app.editor_cancel_rename();
                        }
                    } else {
                        // Dim the label of a hidden entity.
                        let label_rt = if hidden {
                            egui::RichText::new(&label).weak()
                        } else {
                            egui::RichText::new(&label)
                        };
                        let resp = ui
                            .selectable_label(is_sel, label_rt)
                            .on_hover_text(tr("double-click to rename", "더블클릭하여 이름 변경"));
                        // Right-click context menu: the same rename/duplicate/focus/delete ops as the
                        // toolbar + shortcuts, per-row and discoverable. Each button records the chosen
                        // action (applied after the list is drawn) and closes the menu.
                        resp.context_menu(|ui| {
                            if ui.button(tr("Rename", "이름 변경")).clicked() {
                                ctx_action = Some((e, EntityContextAction::Rename));
                                ui.close();
                            }
                            if ui.button(tr("⎘ Duplicate", "⎘ 복제")).clicked() {
                                ctx_action = Some((e, EntityContextAction::Duplicate));
                                ui.close();
                            }
                            if ui
                                .button(tr("🎯 Focus camera", "🎯 카메라 포커스"))
                                .clicked()
                            {
                                ctx_action = Some((e, EntityContextAction::Focus));
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(tr("🗑 Delete", "🗑 삭제")).clicked() {
                                ctx_action = Some((e, EntityContextAction::Delete));
                                ui.close();
                            }
                        });
                        if resp.clicked() {
                            apply_multiselect(
                                e,
                                ui.input(|i| i.modifiers.ctrl),
                                &mut app.editor.selected_entities,
                                &mut app.editor.inspector_selected,
                            );
                        }
                        if resp.double_clicked() {
                            app.editor_begin_rename(e);
                        }
                    }
                });
            }
        });
    // Apply the right-click menu action chosen this frame (collect-then-apply keeps the menu closure
    // free of `app` mutation during iteration).
    if let Some((e, action)) = ctx_action {
        app.editor_apply_entity_context_action(e, action);
    }
}

/// egui drag-and-drop payload for the Scene-tree: the entity being dragged to a new parent.
/// `'static + Send + Sync` (a plain `Entity` id) as egui's `dnd_drag_source` requires.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
struct DragEntity(Entity);

/// Scene graph tab body.  Shows a parent→children indented tree.
///
/// Used in: docked left panel (Scene tab), overlay Inspector window (tab 2).
///
/// **Drag-to-reparent:** each tree node is an egui drag source AND a drop target — dragging a node
/// onto another re-parents it under that node; dragging onto the bottom "unparent" zone detaches it
/// to a root. Edits go through the cycle-safe [`App::editor_reparent`], so a drop that would create
/// a cycle (onto self or a descendant) is a no-op.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn scene_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    tag_map: &HashMap<Entity, String>,
    scene_graph_data: &[(Entity, Option<Entity>)],
    children_map: &HashMap<Entity, Vec<Entity>>,
    root_entities: &[Entity],
) {
    let _ = scene_graph_data; // used implicitly through root_entities + children_map

    let mut clicked_entity: Option<Entity> = None;
    let mut ctrl_clicked: bool = false;
    // A node drop sets `(dragged_child, Some(target_parent))`; the unparent zone sets `(_, None)`.
    let mut dropped: Option<(Entity, Option<Entity>)> = None;

    egui::ScrollArea::vertical()
        .id_salt("docked_scene_graph")
        .show(ui, |ui| {
            let mut stack: Vec<(Entity, usize)> =
                root_entities.iter().rev().map(|&e| (e, 0)).collect();
            while let Some((entity, depth)) = stack.pop() {
                let name = entity_label(entity, tag_map);
                let is_selected = app.editor.selected_entities.contains(&entity);
                let has_children = children_map
                    .get(&entity)
                    .map(|c| !c.is_empty())
                    .unwrap_or(false);
                let prefix = if has_children { "▶ " } else { "  " };
                let icon = entity_type_icon(&app.world, entity);
                let indent = format!("{}{}{}", "  ".repeat(depth), prefix, icon);
                // Inline rename in the Scene tree: while this node is being renamed, draw a focused
                // text box (bound to the shared `entity_rename` buffer, same as the Entities list)
                // in place of the label — and NOT wrapped in a drag source, so typing/dragging in
                // the field never starts a reparent DnD. Otherwise draw the draggable node; a
                // double-click starts a rename via the shared `editor_begin_rename`.
                let renaming = matches!(&app.editor.entity_rename, Some(r) if r.entity == entity);
                if renaming {
                    ui.horizontal(|ui| {
                        ui.label(&indent);
                        let (mut commit, mut cancel) = (false, false);
                        if let Some(rn) = app.editor.entity_rename.as_mut() {
                            let resp = ui.text_edit_singleline(&mut rn.buffer);
                            if rn.focus_pending {
                                resp.request_focus();
                                rn.focus_pending = false;
                            }
                            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                cancel = true;
                            } else if resp.lost_focus() {
                                commit = true;
                            }
                        }
                        if commit {
                            app.editor_commit_rename();
                        } else if cancel {
                            app.editor_cancel_rename();
                        }
                    });
                } else {
                    let label_text = format!("{indent} {name}");
                    // Each node is a drag source (so it can be picked up) whose returned response
                    // also ORs in the inner selectable_label — so `.clicked()` still selects.
                    // `dnd_drag_source` sets the payload while dragged; `dnd_release_payload` fires
                    // when a drag is dropped over this node, making the node a drop target too.
                    let dnd_id = egui::Id::new(("scene_dnd", entity));
                    let response = ui
                        .dnd_drag_source(dnd_id, DragEntity(entity), |ui| {
                            // Inner response is unused — selection comes from the OR'd outer
                            // response below; this draws only the selection highlight.
                            let _ = ui.selectable_label(is_selected, &label_text);
                        })
                        .response;
                    if response.clicked() {
                        clicked_entity = Some(entity);
                        ctrl_clicked = ui.input(|i| i.modifiers.ctrl);
                    }
                    if response.double_clicked() {
                        app.editor_begin_rename(entity);
                    }
                    if let Some(payload) = response.dnd_release_payload::<DragEntity>() {
                        if payload.0 != entity {
                            dropped = Some((payload.0, Some(entity)));
                        }
                    }
                }
                if let Some(ch) = children_map.get(&entity) {
                    for &child in ch.iter().rev() {
                        stack.push((child, depth + 1));
                    }
                }
            }
        });

    if let Some(e) = clicked_entity {
        apply_multiselect(
            e,
            ctrl_clicked,
            &mut app.editor.selected_entities,
            &mut app.editor.inspector_selected,
        );
    }

    // Drop a node here (outside any parent) to detach it to a root.
    let unparent_frame = egui::Frame::default()
        .inner_margin(4.0)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke);
    let (_, root_payload) = ui.dnd_drop_zone::<DragEntity, _>(unparent_frame, |ui| {
        ui.label(tr("⤴ drop here to unparent", "⤴ 여기에 놓아 부모 해제"));
    });
    if let Some(payload) = root_payload {
        dropped = Some((payload.0, None));
    }

    if let Some((child, new_parent)) = dropped {
        app.editor_reparent(child, new_parent);
    }

    ui.separator();
    if let Some(sel) = app.editor.inspector_selected {
        tag_name_editor(ui, sel, tag_map, &mut app.world);
    } else {
        ui.label(tr("(no entity selected)", "(선택된 엔티티 없음)"));
    }
}

/// Inspector (component fields) tab body for the currently selected entity.
///
/// Used in: docked right panel, overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn inspector_tab_body(
    ui: &mut egui::Ui,
    app: &mut App,
    comp_fields: &mut super::InspectorCompFields,
    selected_comp_names: &[&'static str],
    tag_map: &HashMap<Entity, String>,
    _entity_list: &[Entity],
) {
    egui::ScrollArea::vertical()
        .id_salt("docked_inspector_fields")
        .show(ui, |ui| {
            // ── Ambient Light (global scene lighting) ─────────────────────────
            egui::CollapsingHeader::new(tr("Ambient Light", "환경광"))
                .default_open(false)
                .show(ui, |ui| {
                    ambient_light_control(ui, app);
                });

            // Component field editor
            for (_, comp_name, fields) in comp_fields.iter_mut() {
                ui.collapsing(*comp_name, |ui| {
                    egui::Grid::new(*comp_name)
                        .num_columns(2)
                        .spacing([4.0, 2.0])
                        .show(ui, |ui| {
                            for (fname, fval) in fields.iter_mut() {
                                ui.label(*fname);
                                reflect_value_editor(ui, fval);
                                ui.end_row();
                            }
                        });
                });
            }

            // Add / remove components
            if let Some(sel) = app.editor.inspector_selected {
                // ── Tile Paint (shown only for Tilemap entities) ─────────────
                let atlas_dims = app
                    .world
                    .get::<crate::tilemap::Tilemap>(sel)
                    .map(|tm| (tm.atlas.columns, tm.atlas.rows));
                if let Some((cols, rows)) = atlas_dims {
                    let tile_count = cols.saturating_mul(rows);
                    ui.separator();
                    egui::CollapsingHeader::new(tr("Tile Paint", "타일 페인트"))
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.checkbox(
                                &mut app.editor.paint_mode,
                                tr("Paint mode (suppresses gizmo)", "페인트 모드 (기즈모 숨김)"),
                            );
                            // Tool selector (freehand brush / rectangle / bucket fill).
                            ui.horizontal(|ui| {
                                use crate::app::editor::PaintTool;
                                let tool = app.editor.paint_tool;
                                if ui
                                    .selectable_label(tool == PaintTool::Freehand, tr("Brush", "브러시"))
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Freehand;
                                }
                                if ui
                                    .selectable_label(tool == PaintTool::Rectangle, tr("Rect", "사각형"))
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Rectangle;
                                }
                                if ui
                                    .selectable_label(tool == PaintTool::Bucket, tr("Bucket", "채우기"))
                                    .clicked()
                                {
                                    app.editor.paint_tool = PaintTool::Bucket;
                                }
                            });
                            // Brush size (freehand only).
                            if app.editor.paint_tool == crate::app::editor::PaintTool::Freehand {
                                ui.horizontal(|ui| {
                                    ui.label(tr("Brush:", "브러시:"));
                                    for size in [1u32, 3, 5] {
                                        if ui
                                            .selectable_label(
                                                app.editor.paint_brush == size,
                                                format!("{size}×{size}"),
                                            )
                                            .clicked()
                                        {
                                            app.editor.paint_brush = size;
                                        }
                                    }
                                });
                            }
                            if app.editor.paint_value > tile_count {
                                app.editor.paint_value = tile_count;
                            }
                            // The atlas texture (if registered with egui) lets us draw
                            // each tile as a real thumbnail; else fall back to numbers.
                            let swatch_tex = app.editor.paint_atlas_tex.as_ref().map(|(_, id)| *id);
                            const SWATCH: f32 = 26.0;
                            ui.horizontal_wrapped(|ui| {
                                let cur = app.editor.paint_value;
                                if ui.selectable_label(cur == 0, tr("Erase", "지우기")).clicked() {
                                    app.editor.paint_value = 0;
                                }
                                for tile_id in 0..tile_count {
                                    let value = tile_id + 1;
                                    let clicked = if let Some(tex) = swatch_tex {
                                        let uv = crate::renderer::uv::UvRect::from_grid(
                                            tile_id % cols,
                                            tile_id / cols,
                                            cols,
                                            rows,
                                        );
                                        let img = egui::Image::new(egui::load::SizedTexture::new(
                                            tex,
                                            egui::vec2(SWATCH, SWATCH),
                                        ))
                                        .uv(uv_rect_to_egui(uv));
                                        ui.add(egui::Button::image(img).selected(cur == value))
                                            .on_hover_text(format!("{} {value}", tr("tile", "타일")))
                                            .clicked()
                                    } else {
                                        ui.selectable_label(cur == value, format!("{value}"))
                                            .clicked()
                                    };
                                    if clicked {
                                        app.editor.paint_value = value;
                                    }
                                }
                            });
                            ui.label(
                                egui::RichText::new(tr(
                                    "L paint · R erase · Alt+click pick · 1–9 value · Ctrl+Z undo",
                                    "L 페인트 · R 지우기 · Alt+클릭 스포이드 · 1–9 값 · Ctrl+Z 실행 취소",
                                ))
                                .weak(),
                            );
                        });
                } else {
                    // Selection is not a Tilemap — ensure paint mode is off.
                    app.editor.paint_mode = false;
                }

                // ── Registered inspector panels (built-in + user-defined) ─────
                // Take the vec off `app.editor` to avoid holding a borrow of
                // `app.editor` while the draw closure receives `&mut app`.
                let panels = std::mem::take(&mut app.editor.inspector_panels);
                for p in &panels {
                    if (p.presence)(&app.world, sel) {
                        ui.separator();
                        egui::CollapsingHeader::new(&p.title)
                            .default_open(true)
                            .show(ui, |ui| (p.draw)(ui, app, sel));
                    }
                }
                app.editor.inspector_panels = panels;

                ui.separator();
                ui.strong(tr("Component List", "컴포넌트 목록"));

                // Serde-registered components on this entity can be copied to the clipboard.
                // `component_names_for` checks presence only (no RON serialization), so this
                // is O(registry size) instead of serializing every component value per frame.
                let copyable: std::collections::HashSet<String> = app
                    .world
                    .resource::<crate::prefab::SerdeComponentRegistry>()
                    .map(|r| r.component_names_for(&app.world, sel).into_iter().collect())
                    .unwrap_or_default();

                let mut to_remove: Option<&'static str> = None;
                let mut to_copy: Option<&'static str> = None;
                for &comp_name in selected_comp_names {
                    let removable = comp_name != "Transform"
                        && app.editor.component_removers.contains_key(comp_name);
                    let copyable_now = copyable.contains(comp_name);
                    ui.horizontal(|ui| {
                        ui.label(comp_name);
                        if removable && ui.small_button("✕").on_hover_text(tr("remove", "제거")).clicked() {
                            to_remove = Some(comp_name);
                        }
                        if copyable_now
                            && ui
                                .small_button("⧉")
                                .on_hover_text(tr("copy component", "컴포넌트 복사"))
                                .clicked()
                        {
                            to_copy = Some(comp_name);
                        }
                    });
                }
                if let Some(name) = to_copy {
                    app.copy_component(sel, name);
                }
                if let Some(name) = to_remove {
                    if let Some(remover) = app.editor.component_removers.get(name) {
                        remover(&mut app.world, sel);
                    }
                }
                // Paste the clipboard component onto this entity.
                if let Some((clip_name, _)) = app.editor.component_clipboard.clone() {
                    if ui
                        .button(format!("⧉ {} {clip_name}", tr("Paste", "붙여넣기")))
                        .on_hover_text(tr("apply the copied component to this entity", "복사한 컴포넌트를 이 엔티티에 적용"))
                        .clicked()
                    {
                        app.paste_component(sel);
                    }
                }

                ui.separator();
                // Add component dropdown
                let factory_names: Vec<String> = {
                    let mut names: Vec<String> =
                        app.editor.component_factories.keys().cloned().collect();
                    names.sort();
                    names
                };
                if !factory_names.is_empty() {
                    // Reset the selection if it is empty or stale (e.g. after a scene reload
                    // where the factory map can change and the stored name may no longer be
                    // a valid key — silently no-op-ing the "+ Add" button otherwise).
                    if app.editor.add_component_selected.is_empty()
                        || !factory_names
                            .iter()
                            .any(|n| n == &app.editor.add_component_selected)
                    {
                        app.editor.add_component_selected = factory_names[0].clone();
                    }
                    let cur = app.editor.add_component_selected.clone();
                    egui::ComboBox::from_id_salt("docked_add_comp")
                        .selected_text(&cur)
                        .show_ui(ui, |ui| {
                            for name in &factory_names {
                                ui.selectable_value(
                                    &mut app.editor.add_component_selected,
                                    name.clone(),
                                    name,
                                );
                            }
                        });
                    if ui.button(tr("+ Add", "+ 추가")).clicked() {
                        let chosen = app.editor.add_component_selected.clone();
                        if let Some(factory) = app.editor.component_factories.get(&chosen) {
                            factory(&mut app.world, sel);
                        }
                    }
                }

                // Prefab create / spawn
                ui.separator();
                ui.collapsing(tr("Prefab", "프리팹"), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(tr("Path:", "경로:"));
                        ui.add(
                            egui::TextEdit::singleline(&mut app.editor.prefab_path)
                                .desired_width(140.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        if ui.button(tr("💾 Save Selected", "💾 선택 항목 저장")).clicked() {
                            let path = app.editor.prefab_path.clone();
                            app.save_selected_as_prefab(sel, &path);
                        }
                        if ui.button(tr("➕ Spawn", "➕ 생성")).clicked() {
                            let path = app.editor.prefab_path.clone();
                            app.spawn_prefab(&path);
                        }
                    });
                    if let Some(status) = &app.editor.prefab_status {
                        ui.label(egui::RichText::new(status).weak());
                    }
                });

                // PrefabInstance / Break Prefab
                let prefab_path = app
                    .world
                    .get::<crate::prefab::PrefabInstance>(sel)
                    .map(|pi| pi.source_path.clone());
                if let Some(path) = prefab_path {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: {path}", tr("Prefab", "프리팹")));
                        if ui.button(tr("Break Prefab", "프리팹 분리")).clicked() {
                            crate::prefab::break_prefab_instance(&mut app.world, sel);
                        }
                    });
                }

                // Tag/name editor
                ui.separator();
                tag_name_editor(ui, sel, tag_map, &mut app.world);
            } else {
                ui.label(tr("(no entity selected)", "(선택된 엔티티 없음)"));
            }
        });
}

/// Asset browser body.
///
/// Used in: docked bottom panel, overlay Inspector window (tab 1).
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn assets_tab_body(ui: &mut egui::Ui, app: &App) {
    let entries = app
        .world
        .resource::<AssetServer>()
        .map(|a| a.image_list())
        .unwrap_or_default();
    if entries.is_empty() {
        ui.label(tr("(No images loaded)", "(로드된 이미지 없음)"));
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("docked_assets_browser")
        .show(ui, |ui| {
            egui::Grid::new("docked_asset_grid")
                .num_columns(2)
                .spacing([8.0, 4.0])
                .show(ui, |ui| {
                    for entry in &entries {
                        let filename = std::path::Path::new(&entry.path)
                            .file_name()
                            .map(|f| f.to_string_lossy().into_owned())
                            .unwrap_or_else(|| entry.path.clone());
                        ui.label("[ ]");
                        ui.vertical(|ui| {
                            ui.label(&filename);
                            ui.small(format!("{}×{}", entry.width, entry.height));
                        });
                        ui.end_row();
                    }
                });
        });
}

/// Scene save controls (path text field + Save/Load buttons + status messages).
///
/// Used in: docked right panel footer, overlay Inspector window.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn save_load_controls(
    ui: &mut egui::Ui,
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    ui.separator();
    ui.horizontal(|ui| {
        ui.label(tr("Path:", "경로:"));
        ui.add(egui::TextEdit::singleline(&mut app.editor.editor_save_path).desired_width(160.0));
        if ui.button(tr("📂 Load", "📂 불러오기")).clicked() {
            do_load_scene(app);
        }
        if ui.button(tr("💾 Save", "💾 저장")).clicked() {
            do_save_scene_with_list(app, entity_list, tag_map);
        }
    });
    if let Some(msg) = &app.editor.editor_save_status {
        ui.small(msg.as_str());
    }
    if let Some(msg) = &app.editor.editor_load_status {
        ui.small(msg.as_str());
    }
}

// ── Save / Load helpers ──────────────────────────────────────────────────────

/// Execute a scene save using the current `entity_list` and `tag_map`.
///
/// Uses a topological sort so parents appear before children in the RON output.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_save_scene_with_list(
    app: &mut App,
    entity_list: &[Entity],
    tag_map: &HashMap<Entity, String>,
) {
    let mut scene_def = crate::prefab::SceneDef::default();
    let sorted = crate::hierarchy::topological_sort_entities(entity_list, &app.world);
    let mut dropped_parent_links: u32 = 0;
    for &e in &sorted {
        let tag = app.world.get::<crate::prefab::Tag>(e).map(|t| t.0.clone());
        let transform = app.world.get::<crate::components::Transform>(e).cloned();
        let sprite = app.world.get::<crate::components::Sprite>(e).cloned();
        let parent_entity = app.world.get::<crate::hierarchy::Parent>(e).map(|p| p.0);
        let parent = parent_entity.and_then(|p| tag_map.get(&p)).cloned();
        // Warn when a parent exists but has no tag — the hierarchy link cannot
        // be represented in the RON format and will be silently lost on reload.
        if parent_entity.is_some() && parent.is_none() {
            let child_name = tag.as_deref().unwrap_or("(untagged)");
            log::warn!(
                "scene save: parent of '{}' has no Tag — parent link dropped (will not restore on load)",
                child_name
            );
            dropped_parent_links += 1;
        }
        let components = app
            .world
            .resource::<crate::prefab::SerdeComponentRegistry>()
            .map(|r| r.serialize_entity(&app.world, e))
            .unwrap_or_default();
        if tag.is_some() || transform.is_some() || sprite.is_some() || !components.is_empty() {
            scene_def.entities.push(crate::prefab::EntityDef {
                tag,
                transform,
                sprite,
                parent,
                components,
            });
        }
    }
    let count = scene_def.entities.len();
    let path = app.editor.editor_save_path.clone();
    let save_result = scene_def.save(std::path::Path::new(&path));
    // Surface the outcome as a toast (covers both the toolbar button and Ctrl+S).
    match &save_result {
        Ok(()) => app.push_editor_toast(
            format!("{} ({count})", tr("Scene saved", "씬 저장됨")),
            crate::app::editor::state::ToastKind::Success,
        ),
        Err(e) => app.push_editor_toast(
            format!("{}: {e}", tr("Save failed", "저장 실패")),
            crate::app::editor::state::ToastKind::Error,
        ),
    }
    app.editor.editor_save_status = match save_result {
        Ok(()) => {
            if dropped_parent_links > 0 {
                Some(format!(
                    "✓ {count} {} → {path} ({dropped_parent_links} {})",
                    tr("entities", "엔티티"),
                    tr(
                        "parent link(s) dropped: untagged parent",
                        "부모 링크 누락: 태그 없는 부모"
                    ),
                ))
            } else {
                Some(format!("✓ {count} {} → {path}", tr("entities", "엔티티")))
            }
        }
        Err(e) => Some(format!("✗ {e}")),
    };
    app.editor.editor_load_status = None;
}

/// Execute a scene save without an explicit entity list (queries the world).
///
/// Used by the toolbar "💾 Save" which runs before the entity list is built.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_save_scene(app: &mut App) {
    let entity_list: Vec<Entity> = app.world.entities().to_vec();
    let tag_map: HashMap<Entity, String> = app
        .world
        .query::<Tag>()
        .map(|(e, t)| (e, t.0.clone()))
        .collect();
    do_save_scene_with_list(app, &entity_list, &tag_map);
}

/// Execute a scene load from `editor.editor_save_path`.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::app) fn do_load_scene(app: &mut App) {
    let path_str = app.editor.editor_save_path.clone();
    let path = std::path::Path::new(&path_str);
    match crate::prefab::SceneDef::load(path) {
        Ok(scene_def) => {
            // Despawn ALL current entities before loading so that UiNode-only
            // entities (menus/HUD without a Transform) don't accumulate on reload.
            let to_remove: Vec<Entity> = app.world.entities().to_vec();
            for e in to_remove {
                app.world.despawn(e);
            }
            app.editor.inspector_selected = None;
            app.editor.selected_entities.clear();
            let count = scene_def.entities.len();
            crate::prefab::spawn_scene_def(&mut app.world, &scene_def);
            app.editor.editor_load_status = Some(format!(
                "✓ {count} {} ← {path_str}",
                tr("entities", "엔티티")
            ));
            app.editor.editor_save_status = None;
        }
        Err(e) => {
            app.editor.editor_load_status = Some(format!("✗ {e}"));
        }
    }
}

// reflect_value_editor is defined in super (ui/mod.rs) without a cfg gate,
// so both native and wasm can call it.  docked.rs calls it via `super::reflect_value_editor`
// through `use super::*` at the top of this file.

/// Convert an engine [`UvRect`](crate::renderer::uv::UvRect) (offset + size) into the
/// `min..max` [`egui::Rect`] that `egui::Image::uv` expects, so a Tile Paint swatch
/// samples exactly its atlas tile.
#[cfg(not(target_arch = "wasm32"))]
fn uv_rect_to_egui(uv: crate::renderer::uv::UvRect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(uv.u_offset, uv.v_offset),
        egui::pos2(uv.u_offset + uv.u_size, uv.v_offset + uv.v_size),
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod swatch_tests {
    use super::uv_rect_to_egui;
    use crate::renderer::uv::UvRect;

    #[test]
    fn uv_rect_maps_offset_size_to_min_max() {
        // Tile (col 1, row 0) of a 2×2 atlas → offset (0.5, 0.0), size (0.5, 0.5).
        let uv = UvRect::from_grid(1, 0, 2, 2);
        let r = uv_rect_to_egui(uv);
        assert!((r.min.x - 0.5).abs() < 1e-6, "min.x");
        assert!((r.min.y - 0.0).abs() < 1e-6, "min.y");
        assert!((r.max.x - 1.0).abs() < 1e-6, "max.x");
        assert!((r.max.y - 0.5).abs() < 1e-6, "max.y");
    }

    #[test]
    fn uv_rect_full_covers_whole_texture() {
        let r = uv_rect_to_egui(UvRect::FULL);
        assert_eq!(r.min, egui::pos2(0.0, 0.0));
        assert_eq!(r.max, egui::pos2(1.0, 1.0));
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod icon_tests {
    use super::{entity_type_icon, sorted_entity_list};
    use crate::app::editor::EntitySortMode;
    use crate::prefab::Tag;
    use crate::{App, CameraTarget, Entity, PointLight, Sprite, Transform};
    use std::collections::HashMap;

    /// Build the `entity -> label` map the editor derives from `Tag`s (what `entity_label` reads).
    fn tag_map(app: &App, ents: &[Entity]) -> HashMap<Entity, String> {
        ents.iter()
            .filter_map(|&e| app.world.get::<Tag>(e).map(|t| (e, t.0.clone())))
            .collect()
    }

    #[test]
    fn bare_entity_gets_the_generic_dot() {
        let mut app = App::new();
        let e = app.world.spawn();
        assert_eq!(entity_type_icon(&app.world, e), "·");
    }

    #[test]
    fn transform_only_entity_gets_the_diamond() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        assert_eq!(entity_type_icon(&app.world, e), "🔹");
    }

    #[test]
    fn a_sprite_entity_gets_the_picture_glyph() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        app.world.add_component(e, Sprite::colored(0.5, 0.5, 0.5));
        assert_eq!(entity_type_icon(&app.world, e), "🖼");
    }

    #[test]
    fn a_light_entity_gets_the_bulb_glyph() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        app.world.add_component(e, PointLight::default());
        assert_eq!(entity_type_icon(&app.world, e), "💡");
    }

    #[test]
    fn a_camera_rig_gets_the_camera_glyph() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, CameraTarget);
        assert_eq!(entity_type_icon(&app.world, e), "🎥");
    }

    #[test]
    fn priority_a_light_that_also_has_a_sprite_still_reads_as_a_light() {
        // Priority order: PointLight is checked before Sprite, so the more specific "kind" wins even
        // when both components are present.
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        app.world.add_component(e, Sprite::colored(1.0, 1.0, 1.0));
        app.world.add_component(e, PointLight::default());
        assert_eq!(entity_type_icon(&app.world, e), "💡");
    }

    #[test]
    fn sort_insertion_preserves_raw_order() {
        let mut app = App::new();
        let ents: Vec<Entity> = ["Zebra", "apple", "Mango"]
            .iter()
            .map(|n| {
                let e = app.world.spawn();
                app.world.add_component(e, Tag((*n).into()));
                e
            })
            .collect();
        let tm = tag_map(&app, &ents);
        let out = sorted_entity_list(&ents, EntitySortMode::Insertion, &app.world, &tm);
        assert_eq!(out, ents, "Default sort is the raw entity_list order");
    }

    #[test]
    fn sort_name_is_case_insensitive_alphabetical() {
        let mut app = App::new();
        let ents: Vec<Entity> = ["Zebra", "apple", "Mango"]
            .iter()
            .map(|n| {
                let e = app.world.spawn();
                app.world.add_component(e, Tag((*n).into()));
                e
            })
            .collect();
        let tm = tag_map(&app, &ents);
        let out = sorted_entity_list(&ents, EntitySortMode::Name, &app.world, &tm);
        let labels: Vec<&str> = out.iter().map(|e| tm[e].as_str()).collect();
        assert_eq!(labels, ["apple", "Mango", "Zebra"], "case-insensitive A–Z");
    }

    #[test]
    fn sort_kind_groups_by_entity_kind_then_name() {
        // Kinds rank Light < Sprite < Transform < Bare (the EntityKind variant order); within a kind,
        // ties fall back to case-insensitive name.
        let mut app = App::new();
        let bare = app.world.spawn();
        app.world.add_component(bare, Tag("bare".into()));
        let sprite = app.world.spawn();
        app.world.add_component(sprite, Tag("sprite".into()));
        app.world
            .add_component(sprite, Sprite::colored(0.5, 0.5, 0.5));
        let light = app.world.spawn();
        app.world.add_component(light, Tag("light".into()));
        app.world.add_component(light, PointLight::default());
        let xform = app.world.spawn();
        app.world.add_component(xform, Tag("xform".into()));
        app.world.add_component(xform, Transform::default());

        let ents = vec![bare, sprite, light, xform];
        let tm = tag_map(&app, &ents);
        let out = sorted_entity_list(&ents, EntitySortMode::Kind, &app.world, &tm);
        assert_eq!(
            out,
            vec![light, sprite, xform, bare],
            "grouped Light → Sprite → Transform → Bare"
        );
    }
}

/// Tests for the Entities-list right-click context menu's action dispatch. The egui menu buttons only
/// record `(entity, action)`; `editor_apply_entity_context_action` does the real work — so testing it
/// directly covers the behaviour without driving egui.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod context_action_tests {
    use super::EntityContextAction;
    use crate::prefab::Tag;
    use crate::{App, Transform};

    #[test]
    fn rename_selects_the_target_and_starts_a_rename() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Goblin".into()));

        app.editor_apply_entity_context_action(e, EntityContextAction::Rename);

        assert_eq!(app.editor.inspector_selected, Some(e), "target is selected");
        let rn = app.editor.entity_rename.as_ref().expect("rename active");
        assert_eq!(rn.entity, e);
        assert_eq!(rn.buffer, "Goblin", "rename buffer seeded from the Tag");
    }

    #[test]
    fn delete_acts_on_the_right_clicked_row_even_if_it_was_not_selected() {
        // The dispatch selects the target first, so Delete removes the right-clicked entity, not the
        // previously-selected one — the whole point of select-then-op.
        let mut app = App::new();
        let selected = app.world.spawn();
        let clicked = app.world.spawn();
        app.editor.inspector_selected = Some(selected);
        app.editor.selected_entities = vec![selected];

        app.editor_apply_entity_context_action(clicked, EntityContextAction::Delete);

        assert!(!app.world.is_alive(clicked), "right-clicked entity deleted");
        assert!(
            app.world.is_alive(selected),
            "the old selection is untouched"
        );
    }

    #[test]
    fn duplicate_clones_the_target() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Tag("Src".into()));
        app.world.add_component(e, Transform::default());
        let before = app.world.entities().len();

        app.editor_apply_entity_context_action(e, EntityContextAction::Duplicate);

        assert_eq!(app.world.entities().len(), before + 1, "one clone spawned");
        // Selection moved to the clone (not the original).
        assert_ne!(app.editor.inspector_selected, Some(e));
        assert!(app.editor.inspector_selected.is_some());
    }

    #[test]
    fn action_on_a_dead_entity_is_a_noop() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.despawn(e);
        // Must not panic or start a rename on a despawned entity.
        app.editor_apply_entity_context_action(e, EntityContextAction::Rename);
        assert!(app.editor.entity_rename.is_none());
    }
}
