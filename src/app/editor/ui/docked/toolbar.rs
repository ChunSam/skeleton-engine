//! Docked editor top toolbar.

use super::*;

/// Toolbar contents, left to right: ▶/⏸ play-pause, ⏭ step, Snap (toggle + size), Grid, Bounds,
/// Path (the pathfinding overlay), 💾 Set. (save settings), the EN/한국어 language toggle,
/// ? Keys (the cheatsheet), the scene path with 💾 Save / 📂 Load and their status, and
/// Exit (F2). Eleven controls; this listed five until 2026-09-03.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn docked_toolbar(ui: &mut egui::Ui, app: &mut App) {
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
                // The same transition the F2 key makes — including the settings save this
                // button used to leave out.
                app.set_editor_mode(EditorMode::Off);
            }
        });
    });
}

// ── Where the shared tab bodies went ─────────────────────────────────────────
//
// This header sat above nothing: the tab bodies it describes live in their own modules
// (`entities_tab`, `inspector_tab`, `scene_tab`, `assets_tab`, `save_load`), re-exported from
// `docked/mod.rs`. The property it states still holds and still matters — each renders a
// self-contained piece of UI into the given `ui`, and each is called from BOTH the docked
// panels and the overlay windows in `ui/mod.rs`, so a body must not change behaviour based on
// which one is drawing it — so it is kept here, pointing at where they are (2026-09-03).
