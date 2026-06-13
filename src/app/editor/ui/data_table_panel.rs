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
//! Borrow strategy: the registry lives in `app.world` behind a `resource_mut`
//! borrow. To avoid holding the borrow while calling back into `app` we use the
//! collect-then-apply pattern: gather pending edits into local structs, drop the
//! registry borrow, then re-borrow and apply.

#![cfg(not(target_arch = "wasm32"))]

use crate::app::App;
use crate::data_table::DataTableRegistry;

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
        ui.strong("Tables:");
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
        ui.label("Name:");
        ui.add(
            egui::TextEdit::singleline(&mut app.editor.data_table_open_name).desired_width(80.0),
        );
        ui.label("Path:");
        ui.add(
            egui::TextEdit::singleline(&mut app.editor.data_table_open_path).desired_width(160.0),
        );
        if ui.button("Open").clicked() {
            let name = app.editor.data_table_open_name.trim().to_string();
            let path = app.editor.data_table_open_path.trim().to_string();
            if !name.is_empty() && !path.is_empty() {
                app.load_data_table(name.clone(), path);
                app.editor.selected_data_table = Some(name);
                app.editor.data_table_status = None;
                app.editor.data_table_open_name.clear();
                app.editor.data_table_open_path.clear();
            }
        }
    });

    ui.separator();

    // ── Table status message ──────────────────────────────────────────────────
    if let Some(msg) = &app.editor.data_table_status.clone() {
        ui.small(msg.as_str());
    }

    // ── Table editor grid ─────────────────────────────────────────────────────
    let Some(sel_name) = app.editor.selected_data_table.clone() else {
        ui.label("(no table selected)");
        return;
    };

    // Read current table metadata (columns + row count) without holding a long borrow.
    let (columns, row_count) = {
        let reg = match app.world.resource::<DataTableRegistry>() {
            Some(r) => r,
            None => {
                ui.label("(no DataTableRegistry)");
                return;
            }
        };
        let table = match reg.get(&sel_name) {
            Some(t) => t,
            None => {
                ui.label(format!("(table '{sel_name}' not found)"));
                return;
            }
        };
        (table.columns.clone(), table.rows.len())
    };

    // ── Collect all current cell values for display ───────────────────────────
    // We read out every cell value, let egui produce edits, then apply them in a
    // second borrow. This avoids holding the mutable registry borrow while the UI
    // closure runs.

    // cell_values[row][col_idx] = current ron::Value
    let mut cell_values: Vec<Vec<ron::Value>> = Vec::with_capacity(row_count);
    {
        let reg = app
            .world
            .resource::<DataTableRegistry>()
            .expect("registry exists — checked above");
        let table = reg.get(&sel_name).expect("table exists — checked above");
        for row in &table.rows {
            let vals: Vec<ron::Value> = columns
                .iter()
                .map(|col| {
                    row.iter()
                        .find(|(c, _)| c == col)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(ron::Value::Unit)
                })
                .collect();
            cell_values.push(vals);
        }
    }

    // Pending mutations collected during egui rendering.
    let mut edits: Vec<(usize, usize, ron::Value)> = Vec::new(); // (row, col_idx, new_val)
    let mut delete_row: Option<usize> = None;
    let mut add_row_requested = false;
    let mut save_requested = false;
    let mut reload_requested = false;

    // ── Button row ────────────────────────────────────────────────────────────
    ui.horizontal(|ui| {
        if ui.button("+ Add Row").clicked() {
            add_row_requested = true;
        }
        if ui.button("💾 Save").clicked() {
            save_requested = true;
        }
        if ui.button("↺ Reload").clicked() {
            reload_requested = true;
        }
    });

    // ── Scroll area with grid ─────────────────────────────────────────────────
    egui::ScrollArea::both()
        .id_salt("dt_editor_scroll")
        .show(ui, |ui| {
            egui::Grid::new("dt_editor_grid")
                .num_columns(columns.len() + 2) // +1 for row-index, +1 for delete btn
                .spacing([4.0, 2.0])
                .striped(true)
                .show(ui, |ui| {
                    // Header row
                    ui.label("#");
                    for col in &columns {
                        ui.strong(col);
                    }
                    ui.label(""); // delete column header
                    ui.end_row();

                    // Data rows
                    for (row_idx, row_vals) in cell_values.iter_mut().enumerate() {
                        ui.label(format!("{}", row_idx));

                        for (col_idx, val) in row_vals.iter_mut().enumerate() {
                            let changed_val = cell_editor(ui, val, row_idx, col_idx);
                            if let Some(new_val) = changed_val {
                                edits.push((row_idx, col_idx, new_val));
                            }
                        }

                        // Delete button
                        if ui.small_button("✕").clicked() {
                            delete_row = Some(row_idx);
                        }

                        ui.end_row();
                    }
                });
        });

    // ── Apply collected mutations ─────────────────────────────────────────────

    // Cell edits
    if !edits.is_empty() {
        if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
            if let Some(table) = reg.get_mut(&sel_name) {
                for (row_idx, col_idx, new_val) in edits {
                    let col_name = &columns[col_idx].clone();
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
                    Ok(()) => format!("✓ saved to {}", table.path),
                    Err(e) => format!("✗ {e}"),
                }
            } else {
                format!("✗ table '{sel_name}' not found")
            }
        } else {
            "✗ no registry".to_string()
        };
        app.editor.data_table_status = Some(status);
    }

    // Reload
    if reload_requested {
        let path_opt = app
            .world
            .resource::<DataTableRegistry>()
            .and_then(|r| r.get(&sel_name))
            .map(|t| t.path.clone());

        if let Some(path) = path_opt {
            if let Some(reg) = app.world.resource_mut::<DataTableRegistry>() {
                reg.reload_path(&path);
            }
            app.editor.data_table_status = Some(format!("↺ reloaded from {path}"));
        }
    }
}

// ── Per-cell editor widget ────────────────────────────────────────────────────

/// Render one editable cell for `val`.  Returns `Some(new_val)` when the user
/// made a change, `None` otherwise.
fn cell_editor(
    ui: &mut egui::Ui,
    val: &mut ron::Value,
    row: usize,
    col: usize,
) -> Option<ron::Value> {
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
            let resp = ui.text_edit_singleline(&mut buf);
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
            let _ = (row, col);
            ui.label("(complex)");
            None
        }
    }
}
