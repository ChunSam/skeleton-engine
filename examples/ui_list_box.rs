//! `ListBox` — a scrollable, selectable list, one entity per list.
//!
//! Attach a [`ListBox`] next to a [`UiNode`]; it holds all rows and the single selected index,
//! renders fixed-height rows, and **scrolls** when the rows overflow the node. Click a visible row
//! to select it — [`UiEvent::ListBoxChanged`] fires only when the selection actually changed. The
//! list is one focus stop: **Tab** focuses it, **↑/↓** (or **←/→**) step the selection and scroll it
//! into view.
//!
//! This demo wires two lists — a 12-item inventory (scrolls) and a level select — to a live HUD
//! readout and a change counter.
//!
//! - **Click** a row — select it
//! - **Wheel** over a list — scroll it
//! - **Tab** / **Shift+Tab** — move focus between the two lists
//! - **↑/↓** or **←/→** — step the focused list's selection (auto-scrolls into view)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_list_box.png cargo run --example ui_list_box`): captures the two
//! lists mid-scroll with non-default selections. `HEADLESS_FRAMES=N` overrides the default 12 frames.
use engine::{
    App, Color, DrawText, Entity, Events, InputState, KeyCode, ListBox, ShouldQuit, System,
    TextQueue, UiEvent, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 460;

struct Demo {
    inventory: Entity,
    levels: Entity,
    changes: usize,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Count selection changes from the event bus (fires only on an actual change).
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            self.changes += events
                .read()
                .iter()
                .filter(|e| matches!(e, UiEvent::ListBoxChanged(_, _)))
                .count();
        }

        let picked = |e: Entity, world: &World| {
            world
                .get::<ListBox>(e)
                .and_then(|lb| lb.selected_item().map(str::to_owned))
                .unwrap_or_default()
        };
        let item = picked(self.inventory, world);
        let level = picked(self.levels, world);

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "ListBox — scrollable selectable list (click / wheel / Tab + arrows)",
                Vec2::new(28.0, 22.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                "Inventory (12 items — scrolls)",
                Vec2::new(60.0, 66.0),
                15.0,
                Color::rgb(0.78, 0.82, 0.9),
            ));
            tq.push(DrawText::new(
                "Level select",
                Vec2::new(430.0, 66.0),
                15.0,
                Color::rgb(0.78, 0.82, 0.9),
            ));
            tq.push(DrawText::new(
                format!(
                    "Item: {item}    Level: {level}    changes: {}",
                    self.changes
                ),
                Vec2::new(60.0, 356.0),
                16.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
            tq.push(DrawText::new(
                "Tab: focus list   \u{2191}/\u{2193} or \u{2190}/\u{2192}: step   wheel: scroll   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_list_box_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_list_box — scrollable list".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.register_event::<UiEvent>();

    // Inventory: 12 items in a 196-tall node (7 rows visible) → it scrolls.
    let inventory = app.world.spawn();
    app.world
        .add_component(inventory, UiNode::new(60.0, 90.0, 280.0, 196.0));
    app.world.add_component(
        inventory,
        ListBox::new([
            "Iron Sword",
            "Wooden Shield",
            "Health Potion",
            "Mana Potion",
            "Leather Boots",
            "Silver Ring",
            "Torch",
            "Rope (50ft)",
            "Ancient Map",
            "Gold Key",
            "Dragon Scale",
            "Mystic Orb",
        ])
        .with_selected(2),
    );

    // Level select: 6 rows, custom colors, taller rows.
    let levels = app.world.spawn();
    app.world
        .add_component(levels, UiNode::new(430.0, 90.0, 260.0, 200.0));
    app.world.add_component(
        levels,
        ListBox::new([
            "1 — The Village",
            "2 — Dark Forest",
            "3 — Old Mine",
            "4 — Frozen Peak",
            "5 — Lava Caves",
            "6 — Castle",
        ])
        .with_selected(0)
        .with_row_height(34.0)
        .with_colors(
            Color::rgba(0.09, 0.13, 0.12, 1.0),
            Color::rgba(1.0, 1.0, 1.0, 0.06),
            Color::rgba(0.30, 0.60, 0.40, 0.85),
            Color::rgba_u8(210, 222, 212, 255),
        ),
    );

    app.add_system(UiSystem::new());
    app.add_system(Demo {
        inventory,
        levels,
        changes: 0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        // Scroll the inventory partway and select a row down the list so the capture shows the
        // scrolled state + the selected highlight in view.
        if let Some(lb) = app.world.get_mut::<ListBox>(inventory) {
            lb.selected = 6;
            lb.scroll_offset = 84.0; // 3 rows down
        }
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(12);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
