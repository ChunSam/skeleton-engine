//! Data-table editor panel — native only.
//!
//! Renders the "Data Tables" tab of the bottom editor panel.  The panel allows
//! the user to:
//!
//! * Select an already-loaded [`DataTable`] from a list.
//! * Open a new table by supplying a name and a file path.
//! * Edit cells of the selected table in-place (numeric drag, text, bool toggle).
//! * Add or delete rows.
//! * Save the table back to disk or force a reload from disk.
//!
//! Borrow the table while drawing only the visible rows; collect pending edits
//! locally, then borrow the registry mutably to apply them. Offscreen cells are
//! neither cloned nor submitted to egui.
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
use crate::data_table::{DataTableRegistry, ReloadOutcome};

#[cfg(test)]
#[path = "data_table_tests.rs"]
mod tests;

/// Render the Data Tables panel body into `ui`.
///
/// Called from [`super::super::super::editor::ui::docked`] when the bottom tab is set to 1.
pub(in crate::app) fn data_table_panel_body(ui: &mut egui::Ui, app: &mut App) {
    // ── Table selector ────────────────────────────────────────────────────────
    let names: Vec<String> = app
        .world
        .resource::<DataTableRegistry>()
        .map(|r| r.names())
        .unwrap_or_default();

    ui.horizontal(|ui| {
        ui.strong(tr("Tables:", "테이블:"));
        egui::ScrollArea::horizontal()
            .id_salt("dt_selector_scroll")
            .show(ui, |ui| {
                for name in &names {
                    let is_sel = app.editor.selected_data_table.as_deref() == Some(name.as_str());
                    if ui.selectable_label(is_sel, name).clicked() {
                        app.editor.selected_data_table = Some(name.clone());
                        app.editor.data_table_status = None;
                    }
                }
            });
    });

    // ── Open-new-table row ────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        ui.label(tr("Name:", "이름:"));
        ui.add(
            egui::TextEdit::singleline(&mut app.editor.data_table_open_name).desired_width(80.0),
        );
        ui.label(tr("Path:", "경로:"));
        ui.add(
            egui::TextEdit::singleline(&mut app.editor.data_table_open_path).desired_width(160.0),
        );
        if ui.button(tr("Open", "열기")).clicked() {
            let name = app.editor.data_table_open_name.trim().to_string();
            let path = app.editor.data_table_open_path.trim().to_string();
            if !name.is_empty() && !path.is_empty() {
                // Validates first, so a typo reports here rather than through the asset-failure
                // path — which no panel reads and which panics under strict assets.
                app.editor_open_data_table(name, path);
            }
        }
    });

    ui.separator();

    // ── Table status message ──────────────────────────────────────────────────
    if let Some(msg) = &app.editor.data_table_status {
        ui.small(msg.as_str());
    }

    // ── Table editor grid ─────────────────────────────────────────────────────
    let Some(sel_name) = app.editor.selected_data_table.clone() else {
        ui.label(tr("(no table selected)", "(테이블 미선택)"));
        return;
    };

    let Some(registry) = app.world.resource::<DataTableRegistry>() else {
        ui.label(tr("(no DataTableRegistry)", "(DataTableRegistry 없음)"));
        return;
    };
    let Some(table) = registry.get(&sel_name) else {
        ui.label(format!(
            "{pre}'{sel_name}'{suf}",
            pre = tr("(table ", "(테이블 "),
            suf = tr(" not found)", " 찾을 수 없음)")
        ));
        return;
    };

    // Pending mutations collected during egui rendering.
    let mut edits: Vec<(usize, usize, ron::Value)> = Vec::new(); // (row, col_idx, new_val)
    let mut delete_row: Option<usize> = None;
    let mut add_row_requested = false;
    let mut save_requested = false;
    let mut reload_requested = false;

    // ── Button row ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        if ui.button(tr("+ Add Row", "+ 행 추가")).clicked() {
            add_row_requested = true;
        }
        if ui.button(tr("💾 Save", "💾 저장")).clicked() {
            save_requested = true;
        }
        if ui.button(tr("↺ Reload", "↺ 새로고침")).clicked() {
            reload_requested = true;
        }
    });

    // ── Scroll area with grid ─────────────────────────────────────────────────
    let cells_id = ui.make_persistent_id(("dt_editor_cells", &sel_name));
    ui.scope(|ui| {
        // show_rows and Grid must agree on both the row height and the gap.
        ui.spacing_mut().item_spacing.y = 2.0;
        let row_height = table_row_height(ui);
        egui::ScrollArea::both()
            .id_salt(("dt_editor_scroll", &sel_name))
            .show_rows(ui, row_height, table.rows.len() + 1, |ui, visible| {
                egui::Grid::new(("dt_editor_grid", &sel_name))
                    .num_columns(table.columns.len() + 2)
                    .spacing([4.0, 2.0])
                    .min_row_height(row_height)
                    .start_row(visible.start)
                    .striped(true)
                    .show(ui, |ui| {
                        for grid_row in visible {
                            if grid_row == 0 {
                                ui.label("#");
                                for col in &table.columns {
                                    ui.add(
                                        egui::Label::new(egui::RichText::new(col).strong())
                                            .truncate(),
                                    );
                                }
                                ui.label("");
                            } else {
                                let row_idx = grid_row - 1;
                                let row = &table.rows[row_idx];
                                ui.label(row_idx.to_string());
                                for (col_idx, col) in table.columns.iter().enumerate() {
                                    let val = row
                                        .iter()
                                        .find(|(name, _)| name == col)
                                        .map(|(_, val)| val)
                                        .unwrap_or(&ron::Value::Unit);
                                    // show_rows advances auto IDs with the visible range. Anchor
                                    // each cell outside that hierarchy to retain editing state.
                                    let changed = ui
                                        .scope_builder(
                                            egui::UiBuilder::new()
                                                .id(cells_id.with((row_idx, col_idx))),
                                            |ui| cell_editor(ui, val),
                                        )
                                        .inner;
                                    if let Some(new_val) = changed {
                                        edits.push((row_idx, col_idx, new_val));
                                    }
                                }
                                let delete = ui
                                    .scope_builder(
                                        egui::UiBuilder::new()
                                            .id(cells_id.with((row_idx, "delete"))),
                                        |ui| ui.small_button("✕"),
                                    )
                                    .inner;
                                if delete.clicked() {
                                    delete_row = Some(row_idx);
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
    });

    // ── Apply collected mutations ─────────────────────────────────────────────

    // Cell edits
    if !edits.is_empty() {
        if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
            if let Some(table) = reg.get_mut(&sel_name) {
                for (row_idx, col_idx, new_val) in edits {
                    let col_name = &table.columns[col_idx];
                    if let Some(cell) = table
                        .rows
                        .get_mut(row_idx)
                        .and_then(|r| r.iter_mut().find(|(c, _)| c == col_name))
                    {
                        cell.1 = new_val;
                        table.dirty = true;
                    }
                }
            }
        }
    }

    // Add row
    if add_row_requested {
        if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
            if let Some(table) = reg.get_mut(&sel_name) {
                table.add_row();
            }
        }
    }

    // Delete row
    if let Some(idx) = delete_row {
        if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
            if let Some(table) = reg.get_mut(&sel_name) {
                table.delete_row(idx);
            }
        }
    }

    // Save
    if save_requested {
        let status = if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
            if let Some(table) = reg.get_mut(&sel_name) {
                match table.save() {
                    Ok(()) => format!("✓ {} {}", tr("saved to", "저장 완료:"), table.path),
                    Err(e) => format!("✗ {e}"),
                }
            } else {
                format!(
                    "✗ {} '{sel_name}' {}",
                    tr("table", "테이블"),
                    tr("not found", "찾을 수 없음")
                )
            }
        } else {
            tr("✗ no registry", "✗ 레지스트리 없음").to_string()
        };
        app.editor.data_table_status = Some(status);
    }

    // Reload
    if reload_requested {
        let path_opt = app
            .world
            .resource::<DataTableRegistry>()
            .and_then(|r| r.get(&sel_name))
            .map(|t| (t.path.clone(), t.dirty));

        if let Some((path, _)) = path_opt {
            // By name, not by path: two names loaded from one file used to reload or skip
            // whichever `reload_path` found first, with this status naming the selected one.
            let outcome = if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
                reg.reload_name(&sel_name)
            } else {
                ReloadOutcome::NotFound
            };
            app.editor.data_table_status = Some(match outcome {
                ReloadOutcome::Reloaded => {
                    format!("↺ {} {path}", tr("reloaded from", "새로고침 완료:"))
                }
                ReloadOutcome::SkippedDirty => {
                    format!(
                        "{} '{sel_name}'",
                        tr(
                            "skipped reload — unsaved edits in",
                            "새로고침 건너뜀 — 미저장 편집 내용:"
                        )
                    )
                }
                ReloadOutcome::NotFound => format!(
                    "↺ {} '{path}' {}",
                    tr("path", "경로"),
                    tr("not registered", "등록되지 않음")
                ),
                ReloadOutcome::Err => {
                    format!("↺ {} '{path}'", tr("reload failed for", "새로고침 실패:"))
                }
            });
        }
    }
}

// ── Per-cell editor widget ────────────────────────────────────────────────────

/// Fixed width (logical px) of a string cell's text field, so longer sentences are
/// readable. Wide on purpose — the grid is in a horizontal ScrollArea, so over-wide
/// rows just scroll rather than squashing other columns.
const STRING_CELL_WIDTH: f32 = 260.0;
const STRING_CELL_MARGIN: egui::Margin = egui::Margin::symmetric(4, 2);

fn table_row_height(ui: &egui::Ui) -> f32 {
    // TextEdit's font height plus its margin can exceed interact_size, even at the
    // default font size. Round up so virtual scrolling matches the rendered row spacing.
    ui.spacing()
        .interact_size
        .y
        .max(ui.text_style_height(&egui::TextStyle::Body) + STRING_CELL_MARGIN.sum().y)
        .ceil()
}

/// Render one editable cell for `val`.  Returns `Some(new_val)` when the user
/// made a change, `None` otherwise.
fn cell_editor(ui: &mut egui::Ui, val: &ron::Value) -> Option<ron::Value> {
    match val {
        ron::Value::Number(ron::Number::Float(f)) => {
            let mut v = f.get();
            let resp = ui.add(egui::DragValue::new(&mut v).speed(0.1));
            if resp.changed() {
                Some(ron::Value::Number(ron::Number::Float(
                    ron::value::Float::new(v),
                )))
            } else {
                None
            }
        }
        ron::Value::Number(ron::Number::Integer(i)) => {
            let mut v = *i;
            let resp = ui.add(egui::DragValue::new(&mut v));
            if resp.changed() {
                Some(ron::Value::Number(ron::Number::Integer(v)))
            } else {
                None
            }
        }
        ron::Value::String(s) => {
            let mut buf = s.clone();
            // Wide field so longer sentences are readable without scrolling inside the
            // box. NOTE: inside an `egui::Grid`, a cell's `available_width()` is just the
            // default `min_col_width` (~40px), and `TextEdit::desired_width` is clamped by
            // `at_most(available_width)` — so `.desired_width(..)` alone has no effect here.
            // `add_sized` allocates a fixed-width region first, which both sizes the box and
            // grows the grid column; the grid lives in a both-direction ScrollArea, so the
            // extra width just adds horizontal scroll when several columns are wide.
            let h = table_row_height(ui);
            let resp = ui.add_sized(
                [STRING_CELL_WIDTH, h],
                egui::TextEdit::singleline(&mut buf).margin(STRING_CELL_MARGIN),
            );
            if resp.changed() {
                Some(ron::Value::String(buf))
            } else {
                None
            }
        }
        ron::Value::Bool(b) => {
            let mut v = *b;
            let resp = ui.checkbox(&mut v, "");
            if resp.changed() {
                Some(ron::Value::Bool(v))
            } else {
                None
            }
        }
        _ => {
            // Complex or unit values: display only, no edit
            ui.add(egui::Label::new(tr("(complex)", "(복합값)")).truncate());
            None
        }
    }
}
