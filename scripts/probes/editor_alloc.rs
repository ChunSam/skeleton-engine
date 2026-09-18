//! Injected only into the disposable editor copy by `scripts/editor_alloc_probe.py`.
use super::*;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::time::Instant;

thread_local! {
    static ACTIVE: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
    static BYTES: Cell<usize> = const { Cell::new(0) };
}
struct Counting;
fn record(size: usize) {
    let _ = ACTIVE.try_with(|active| {
        if active.get() {
            ALLOCS.with(|n| n.set(n.get() + 1));
            BYTES.with(|n| n.set(n.get() + size));
        }
    });
}
// SAFETY: all pointer/layout operations delegate unchanged to the system allocator.
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        System.alloc(layout)
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        System.alloc_zeroed(layout)
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record(size);
        System.realloc(ptr, layout, size)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn sample(mut frame: impl FnMut()) -> (usize, usize, u128) {
    for _ in 0..3 {
        frame();
    }
    let mut results = Vec::new();
    for _ in 0..9 {
        ALLOCS.with(|n| n.set(0));
        BYTES.with(|n| n.set(0));
        ACTIVE.with(|n| n.set(true));
        let start = Instant::now();
        frame();
        let elapsed = start.elapsed().as_micros();
        ACTIVE.with(|n| n.set(false));
        results.push((ALLOCS.with(Cell::get), BYTES.with(Cell::get), elapsed));
    }
    results.sort_by_key(|r| r.2);
    results[4]
}
fn report(name: &str, frame: impl FnMut()) -> (usize, usize, u128) {
    let result = sample(frame);
    println!(
        "EDITOR_ALLOC {name} allocs={} bytes={} median_us={}",
        result.0, result.1, result.2
    );
    result
}
fn frame(ctx: &egui::Context, mut body: impl FnMut(&mut egui::Ui)) {
    std::hint::black_box(render(ctx, |ui| body(ui)));
}
fn render(ctx: &egui::Context, mut body: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
    let output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(900.0, 500.0),
            )),
            ..Default::default()
        },
        |ui| body(ui),
    );
    assert!(!output.shapes.is_empty(), "the panel must actually draw");
    output
}
fn contains_text(output: &egui::FullOutput, needle: &str) -> bool {
    fn matches(shape: &egui::Shape, needle: &str) -> bool {
        match shape {
            egui::Shape::Text(text) => text.galley.text().contains(needle),
            egui::Shape::Vec(shapes) => shapes.iter().any(|shape| matches(shape, needle)),
            _ => false,
        }
    }
    output
        .shapes
        .iter()
        .any(|shape| matches(&shape.shape, needle))
}
fn table(rows: usize) -> crate::DataTable {
    crate::DataTable {
        columns: vec!["enabled".into(), "hp".into(), "name".into()],
        rows: (0..rows)
            .map(|i| {
                vec![
                    ("enabled".into(), ron::Value::Bool(true)),
                    (
                        "hp".into(),
                        ron::Value::Number(ron::Number::Integer(i as i64)),
                    ),
                    ("name".into(), ron::Value::String(format!("Goblin {i:05}"))),
                ]
            })
            .collect(),
        path: String::new(),
        dirty: false,
    }
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct ClipboardData {
    values: Vec<String>,
}

#[test]
fn editor_allocations() {
    let check = std::env::var("EDITOR_ALLOC_CHECK").as_deref() == Ok("1");
    let mut table_costs = Vec::new();
    let mut clipboard_costs = Vec::new();
    let mut sort_costs = Vec::new();
    for rows in [0, 10, 100, 1_000, 10_000] {
        let mut app = App::new();
        let mut registry = crate::DataTableRegistry::default();
        registry.insert("test", table(rows));
        app.world.insert_resource(registry);
        app.editor.selected_data_table = Some("test".into());
        let ctx = egui::Context::default();
        table_costs.push(report(&format!("table/{rows}"), || {
            frame(&ctx, |ui| {
                data_table_panel::data_table_panel_body(ui, &mut app)
            })
        }));
        let drawn = render(&ctx, |ui| {
            data_table_panel::data_table_panel_body(ui, &mut app)
        });
        assert!(
            contains_text(&drawn, if rows == 0 { "enabled" } else { "Goblin 00000" }),
            "table control must reach the cell/header, not an empty-state label"
        );
        assert_eq!(
            app.world
                .resource::<crate::DataTableRegistry>()
                .unwrap()
                .get("test")
                .unwrap()
                .rows
                .len(),
            rows
        );
    }
    for entries in [0, 10, 10_000] {
        let mut app = App::new();
        app.register_serde_component::<ClipboardData>("ClipboardData", None);
        let source = app.world.spawn();
        app.world.add_component(
            source,
            ClipboardData {
                values: vec!["payload".repeat(10); entries],
            },
        );
        app.copy_component(source, "ClipboardData");
        let selected = app.world.spawn();
        app.editor.inspector_selected = Some(selected);
        let ctx = egui::Context::default();
        clipboard_costs.push(report(&format!("clipboard/{entries}"), || {
            frame(&ctx, |ui| {
                docked::inspector_tab_body(ui, &mut app, &mut vec![], &[], &HashMap::new(), &[])
            })
        }));
        let drawn = render(&ctx, |ui| {
            docked::inspector_tab_body(ui, &mut app, &mut vec![], &[], &HashMap::new(), &[])
        });
        assert!(
            contains_text(&drawn, "ClipboardData"),
            "clipboard paste button must be drawn"
        );
        // The actual paste still processes the payload; displaying the button must not.
        report(&format!("paste/{entries}"), || {
            app.paste_component(selected)
        });
        assert_eq!(
            app.world
                .get::<ClipboardData>(selected)
                .unwrap()
                .values
                .len(),
            entries
        );
    }
    // A populated selection exercises the copyable-name HashSet as well as factory sorting.
    // Large component data must not be serialized merely to list that component.
    let mut app = App::new();
    app.register_serde_component::<ClipboardData>("ClipboardData", None);
    let selected = app.world.spawn();
    app.world.add_component(
        selected,
        ClipboardData {
            values: vec!["payload".repeat(10); 10_000],
        },
    );
    app.editor.inspector_selected = Some(selected);
    assert!(!app.editor.component_factories.is_empty());
    let ctx = egui::Context::default();
    report("component-list/10000", || {
        frame(&ctx, |ui| {
            docked::inspector_tab_body(
                ui,
                &mut app,
                &mut vec![],
                &["ClipboardData"],
                &HashMap::new(),
                &[selected],
            )
        })
    });
    let drawn = render(&ctx, |ui| {
        docked::inspector_tab_body(
            ui,
            &mut app,
            &mut vec![],
            &["ClipboardData"],
            &HashMap::new(),
            &[selected],
        )
    });
    assert!(
        contains_text(&drawn, "⧉"),
        "populated selection must offer a copy button"
    );

    for count in [0, 100, 1_000] {
        let app = App::new();
        let mut assets = crate::AssetServer::new();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        for i in 0..count {
            assets.load_image_bytes(format!("asset/{i:05}.png"), png.get_ref());
        }
        let mut app = app;
        app.world.insert_resource(assets);
        let ctx = egui::Context::default();
        report(&format!("assets/{count}"), || {
            frame(&ctx, |ui| docked::assets_tab_body(ui, &app))
        });
        if count > 0 {
            assert!(contains_text(
                &render(&ctx, |ui| docked::assets_tab_body(ui, &app)),
                ".png"
            ));
        }
    }
    for count in [10, 100, 1_000] {
        let mut app = App::new();
        let mut tags = HashMap::new();
        let entities: Vec<_> = (0..count)
            .map(|i| {
                let e = app.world.spawn();
                tags.insert(e, format!("entity {:05}", (i * 7919) % count));
                e
            })
            .collect();
        for mode in [
            crate::app::editor::EntitySortMode::Index,
            crate::app::editor::EntitySortMode::Name,
            crate::app::editor::EntitySortMode::Kind,
        ] {
            let cost = report(&format!("sort/{mode:?}/{count}"), || {
                let sorted =
                    docked::entity_kind::sorted_entity_list(&entities, mode, &app.world, &tags);
                assert_eq!(sorted.len(), entities.len());
                std::hint::black_box(sorted);
            });
            if mode != crate::app::editor::EntitySortMode::Index {
                sort_costs.push((count, cost));
            }
        }
        report(&format!("filter/{count}"), || {
            assert!(tags
                .values()
                .any(|label| crate::app::editor::entity_matches_filter(label, "entity")));
            for label in tags.values() {
                std::hint::black_box(crate::app::editor::entity_matches_filter(label, " ENtity "));
            }
        });
        let untagged = HashMap::new();
        report(&format!("sort-untagged/{count}"), || {
            std::hint::black_box(docked::entity_kind::sorted_entity_list(
                &entities,
                crate::app::editor::EntitySortMode::Kind,
                &app.world,
                &untagged,
            ));
        });
        for &e in &entities {
            app.world.add_component(e, crate::Transform::default());
        }
        report(&format!("bounds/{count}"), || {
            app.world
                .resource_mut::<crate::DebugDraw>()
                .unwrap()
                .clear();
            app.draw_debug_bounds();
        });
        assert_eq!(
            app.world
                .resource::<crate::DebugDraw>()
                .unwrap()
                .shapes
                .len(),
            count
        );
        for &e in &entities {
            app.world
                .add_component(e, crate::Collider::Circle { radius: 4.0 });
        }
        report(&format!("bounds-colliders/{count}"), || {
            app.world
                .resource_mut::<crate::DebugDraw>()
                .unwrap()
                .clear();
            app.draw_debug_bounds();
        });
        assert_eq!(
            app.world
                .resource::<crate::DebugDraw>()
                .unwrap()
                .shapes
                .len(),
            2 * count
        );
        app.editor.selected_entities = entities.iter().copied().collect();
        app.editor.gizmo_dragging = true;
        app.editor.snap_enabled = false;
        let selected = entities[0];
        report(&format!("gizmo-hold/{count}"), || {
            let tr = app.world.get::<crate::Transform>(selected).unwrap().clone();
            let cursor = tr.position + glam::Vec2::ONE;
            app.update_transform_gizmo_native(selected, tr, cursor, false, true, false, 1.0);
        });
        for &e in &entities {
            assert_eq!(
                app.world.get::<crate::Transform>(e).unwrap().position,
                glam::Vec2::splat(12.0)
            );
        }
    }
    let app = App::new();
    let ctx = egui::Context::default();
    report("grid/default", || {
        frame(&ctx, |ui| {
            grid_overlay::draw_editor_grid(ui, &app, ui.max_rect())
        })
    });
    report("grid/cursor", || {
        let output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(900.0, 500.0),
                )),
                events: vec![egui::Event::PointerMoved(egui::pos2(100.0, 100.0))],
                ..Default::default()
            },
            |ui| grid_overlay::draw_editor_grid(ui, &app, ui.max_rect()),
        );
        assert!(!output.shapes.is_empty());
        std::hint::black_box(output);
    });
    if check {
        assert!(
            table_costs[4].1 < 250_000,
            "table viewport must not build or clone all 10000 rows: {:?}",
            table_costs[4]
        );
        assert!(
            table_costs[4].1 <= table_costs[2].1 + 4096,
            "table bytes must stay bounded by visible rows"
        );
        assert!(
            clipboard_costs[2].1 <= clipboard_costs[0].1 + 4096,
            "clipboard display must not clone the payload: {clipboard_costs:?}"
        );
        for (count, cost) in sort_costs {
            assert!(
                cost.0 <= count + 8,
                "sort must compute each named key once: {count}: {cost:?}"
            );
            assert!(
                cost.1 <= count * 80,
                "sort requested bytes exceeded key storage budget: {count}: {cost:?}"
            );
        }
    }
}
