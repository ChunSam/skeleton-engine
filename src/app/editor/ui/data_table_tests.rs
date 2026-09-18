//! Exercise the real panel with headless egui input, including a scrolled viewport.
use super::*;

struct Panel {
    app: App,
    ctx: egui::Context,
    time: f64,
}

impl Panel {
    fn new(rows: usize) -> Self {
        let mut app = App::new();
        let mut registry = DataTableRegistry::default();
        registry.insert(
            "test",
            crate::DataTable {
                columns: vec!["name".into()],
                rows: (0..rows)
                    .map(|i| vec![("name".into(), ron::Value::String(format!("row_{i:05}")))])
                    .collect(),
                path: String::new(),
                dirty: false,
            },
        );
        app.world.insert_resource(registry);
        app.editor.selected_data_table = Some("test".into());
        Self {
            app,
            ctx: egui::Context::default(),
            time: 0.0,
        }
    }

    fn frame(&mut self, events: Vec<egui::Event>) -> Vec<(String, egui::Rect)> {
        self.time += 0.1;
        let output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 350.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ui| data_table_panel_body(ui, &mut self.app),
        );
        fn collect(shape: &egui::Shape, clip: egui::Rect, texts: &mut Vec<(String, egui::Rect)>) {
            match shape {
                egui::Shape::Text(text) => {
                    let rect = egui::Rect::from_min_size(text.pos, text.galley.size());
                    if clip.contains_rect(rect) {
                        texts.push((text.galley.text().to_owned(), rect));
                    }
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, clip, texts);
                    }
                }
                _ => {}
            }
        }
        let mut texts = Vec::new();
        for shape in output.shapes {
            collect(&shape.shape, shape.clip_rect, &mut texts);
        }
        texts
    }

    fn click(&mut self, position: egui::Pos2) {
        let button = |pressed| egui::Event::PointerButton {
            pos: position,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        // Establish hover before pressing: a pointer jump while held is a scroll drag.
        self.frame(vec![egui::Event::PointerMoved(position)]);
        self.frame(vec![button(true)]);
        self.frame(vec![button(false)]);
    }

    fn scroll(&mut self, delta: f32) -> Vec<(String, egui::Rect)> {
        self.frame(vec![
            egui::Event::PointerMoved(egui::pos2(200.0, 230.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, delta),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::default(),
            },
        ]);
        for _ in 0..8 {
            self.frame(vec![]);
        }
        self.frame(vec![])
    }

    fn table(&self) -> &crate::DataTable {
        self.app
            .world
            .resource::<DataTableRegistry>()
            .unwrap()
            .get("test")
            .unwrap()
    }
}

fn center(texts: &[(String, egui::Rect)], label: &str) -> egui::Pos2 {
    texts
        .iter()
        .find(|(text, _)| text == label)
        .unwrap_or_else(|| panic!("missing {label}: {texts:?}"))
        .1
        .center()
}

#[test]
fn scrolled_table_keeps_cell_focus_and_edits_deletes_and_adds_the_right_row() {
    scrolled_edit(14.0);
}

#[test]
fn scrolled_table_edits_also_work_with_a_larger_font() {
    scrolled_edit(24.0);
}

fn scrolled_edit(font_size: f32) {
    let mut panel = Panel::new(10_000);
    panel.ctx.global_style_mut(|style| {
        style
            .text_styles
            .get_mut(&egui::TextStyle::Body)
            .unwrap()
            .size = font_size;
    });
    for _ in 0..3 {
        panel.frame(vec![]);
    }
    let shown = panel.scroll(-1200.0);
    let rows: Vec<_> = shown
        .iter()
        .filter(|(text, _)| text.starts_with("row_"))
        .collect();
    assert!(
        rows.len() >= 3 && rows.len() < 30,
        "a nonempty viewport: {rows:?}"
    );
    let (label, rect) = rows[2];
    let row: usize = label.strip_prefix("row_").unwrap().parse().unwrap();
    assert!(row > 20, "the test must reach an offscreen row, got {row}");
    panel.click(rect.center());
    let focus = panel
        .ctx
        .memory(|m| m.focused())
        .expect("text cell focused");
    let shifted = panel.scroll(-22.0);
    assert!(
        shifted.iter().any(|(text, _)| text == label),
        "editing row stays visible"
    );
    assert_ne!(
        center(&shifted, label).y,
        rect.center().y,
        "viewport really moved"
    );
    assert_eq!(panel.ctx.memory(|m| m.focused()), Some(focus));
    panel.frame(vec![egui::Event::Text("!".into())]);
    for (index, cells) in panel.table().rows.iter().enumerate() {
        let ron::Value::String(value) = &cells[0].1 else {
            panic!("string cell")
        };
        if index == row {
            assert!(
                value.contains('!'),
                "focused cell receives input after scrolling"
            );
        } else {
            assert_eq!(
                value,
                &format!("row_{index:05}"),
                "other rows must not change"
            );
        }
    }
    assert!(panel.table().dirty);
    let shown = panel.frame(vec![]);
    let edited = shown.iter().find(|(text, _)| text.contains('!')).unwrap().1;
    let delete = shown
        .iter()
        .filter(|(text, _)| text == "✕")
        .min_by(|(_, a), (_, b)| {
            (a.center().y - edited.center().y)
                .abs()
                .total_cmp(&(b.center().y - edited.center().y).abs())
        })
        .unwrap()
        .1
        .center();
    panel.click(delete);
    assert_eq!(panel.table().rows.len(), 9_999);
    assert_eq!(
        panel.table().rows[row][0].1,
        ron::Value::String(format!("row_{:05}", row + 1))
    );
    let shown = panel.frame(vec![]);
    panel.click(center(&shown, tr("+ Add Row", "+ 행 추가")));
    assert_eq!(panel.table().rows.len(), 10_000);
    assert_eq!(
        panel.table().rows.last().unwrap()[0].1,
        ron::Value::String(String::new())
    );
}

#[test]
fn an_empty_table_can_add_its_first_row() {
    let mut panel = Panel::new(0);
    for _ in 0..3 {
        panel.frame(vec![]);
    }
    let shown = panel.frame(vec![]);
    assert!(
        shown.iter().any(|(text, _)| text == "name"),
        "empty table still draws its header"
    );
    panel.click(center(&shown, tr("+ Add Row", "+ 행 추가")));
    assert_eq!(panel.table().rows.len(), 1);
}

#[test]
fn scrolling_to_the_end_can_reach_the_last_row() {
    let mut panel = Panel::new(10_000);
    for _ in 0..3 {
        panel.frame(vec![]);
    }
    let shown = panel.scroll(-1_000_000.0);
    assert!(
        shown.iter().any(|(text, _)| text == "row_09999"),
        "last row must be fully visible at the end: {shown:?}"
    );
}

#[test]
fn scroll_distance_matches_the_rendered_row_spacing() {
    let mut panel = Panel::new(10_000);
    for _ in 0..3 {
        panel.frame(vec![]);
    }
    let initial = panel.frame(vec![]);
    let first_y = center(&initial, "row_00000").y;
    let pitch = center(&initial, "row_00001").y - first_y;
    let scrolled = panel.scroll(-50.0 * pitch);
    let actual_y = center(&scrolled, "row_00050").y;
    assert!(
        (actual_y - first_y).abs() < 1.0,
        "scroll geometry must match visible row spacing: {first_y} vs {actual_y}"
    );
}
