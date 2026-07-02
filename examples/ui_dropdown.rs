//! `Dropdown` — a combobox for game UI (settings menus, loadout pickers).
//!
//! Attach a [`Dropdown`] next to a [`UiNode`]; click it to open the item list, click an item to
//! select (emits [`UiEvent::DropdownChanged`]), press anywhere else to close. The open list draws
//! above — and absorbs the pointer over — everything underneath it, and flips upward near the
//! bottom of the window. With a keyboard/gamepad the widget is focusable: **Tab** focus,
//! **Enter/Space** open/close, **←/→** step the selection directly.
//!
//! This demo is a small settings panel: quality + difficulty dropdowns over a row of buttons
//! (proving the open list blocks them — the HUD's Apply click counter never moves while you click
//! "through" the open list), plus a bottom-edge dropdown that opens upward. The HUD echoes every
//! `DropdownChanged` event.
//!
//! Known engine-wide limitation (predates this widget, same as a `Panel` over a `Button`): text
//! renders in its own pass after all UI rects, so a covered widget's *label* still shows through
//! an overlay. The open list absorbs the pointer correctly; only the text bleeds.
//!
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_dropdown.png cargo run --example ui_dropdown`): the quality
//! list is opened programmatically (`open = true`) with the synthetic cursor on a row, so the
//! capture lands with the list open + a row hover-highlighted. `HEADLESS_FRAMES=N` overrides the
//! default 30 warm-up frames.
use engine::{
    App, Button, Color, DrawText, Dropdown, Entity, Events, InputState, KeyCode, ShouldQuit,
    System, TextQueue, UiEvent, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

struct Demo {
    quality: Entity,
    difficulty: Entity,
    apply_button: Entity,
    apply_clicks: u32,
    last_event: String,
    /// Headless capture mode: hold the quality list open with the cursor on a row.
    auto_open: Option<Vec2>,
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

        // Headless: keep the quality list open and the synthetic cursor on its second row.
        if let Some(target) = self.auto_open {
            if let Some(dd) = world.get_mut::<Dropdown>(self.quality) {
                dd.open = true;
            }
            if let Some(input) = world.resource_mut::<InputState>() {
                input.set_cursor(target);
            }
        }

        // Echo selection events into the HUD line; count Apply clicks (they must NOT increase
        // while the open quality list covers the button).
        let mut events: Vec<(Entity, usize)> = Vec::new();
        if let Some(ev) = world.resource::<Events<UiEvent>>() {
            for e in ev.read() {
                match e {
                    UiEvent::DropdownChanged(entity, index) => events.push((*entity, *index)),
                    UiEvent::ButtonClicked(b) if *b == self.apply_button => {
                        self.apply_clicks += 1;
                    }
                    _ => {}
                }
            }
        }
        for (entity, index) in events {
            let (who, item) = if entity == self.quality {
                ("quality", world.get::<Dropdown>(entity))
            } else if entity == self.difficulty {
                ("difficulty", world.get::<Dropdown>(entity))
            } else {
                ("bottom", world.get::<Dropdown>(entity))
            };
            let item = item
                .and_then(|d| d.items.get(index))
                .cloned()
                .unwrap_or_default();
            self.last_event = format!("DropdownChanged: {who} -> [{index}] {item}");
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Dropdown — click to open, click an item to select",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                "Tab: focus   Enter: open/close   Left/Right: step selection   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
            tq.push(DrawText::new(
                format!(
                    "{}   |   Apply clicks: {}",
                    self.last_event, self.apply_clicks
                ),
                Vec2::new(28.0, WIN_H as f32 - 56.0),
                14.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_dropdown_demo"
    }
}

fn main() {
    let mut app = App::new();
    // Without this the Events<UiEvent> bus does not exist and every DropdownChanged /
    // ButtonClicked is silently dropped (the HUD line + Apply counter would never move).
    app.register_event::<UiEvent>();
    app.world.insert_resource(WindowConfig {
        title: "ui_dropdown — combobox".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    // Quality dropdown — rounded, custom hover tint.
    let quality = app.world.spawn();
    app.world
        .add_component(quality, UiNode::new(80.0, 90.0, 180.0, 32.0));
    app.world.add_component(
        quality,
        Dropdown::new(["Low", "Medium", "High", "Ultra"])
            .with_selected(1)
            .with_corner_radius(5.0),
    );

    // Difficulty dropdown.
    let difficulty = app.world.spawn();
    app.world
        .add_component(difficulty, UiNode::new(320.0, 90.0, 180.0, 32.0));
    app.world.add_component(
        difficulty,
        Dropdown::new(["Easy", "Normal", "Hard"]).with_corner_radius(5.0),
    );

    // A row of buttons UNDER where the quality list opens — the open list must absorb clicks.
    let mut apply_button = None;
    for (i, name) in ["Apply", "Reset", "Back"].iter().enumerate() {
        let b = app.world.spawn();
        let mut node = UiNode::new(80.0, 150.0 + i as f32 * 44.0, 180.0, 36.0);
        node.z = 0.5;
        app.world.add_component(b, node);
        app.world.add_component(b, Button::new(*name));
        if i == 0 {
            apply_button = Some(b);
        }
    }

    // Bottom-edge dropdown — opens upward (viewport flip). Far right so it clears the HUD
    // text lines in the bottom-left corner (playtest note: they overlapped at x=320).
    let bottom = app.world.spawn();
    app.world
        .add_component(bottom, UiNode::new(520.0, 360.0, 180.0, 32.0));
    app.world.add_component(
        bottom,
        Dropdown::new(["Opens", "Upward", "Here"]).with_corner_radius(5.0),
    );

    let headless = std::env::var("HEADLESS_SHOT").is_ok();
    app.add_system(UiSystem::new());
    app.add_system(Demo {
        quality,
        difficulty,
        apply_button: apply_button.expect("apply button spawned"),
        apply_clicks: 0,
        last_event: "(no selection yet)".into(),
        // Quality list opens below (80,122); park the cursor on row 2 ("High", y 186..218).
        auto_open: headless.then_some(Vec2::new(150.0, 200.0)),
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(30);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
