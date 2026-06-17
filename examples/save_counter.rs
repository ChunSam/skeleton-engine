//! save_counter — cross-platform persistence with `write_ron` / `read_ron`.
//!
//! Loads a launch counter at startup, increments it, and saves it back. The exact same code
//! persists to a **file** on native and to **`localStorage`** on the web (wasm) — the engine's
//! `save` module routes `write_ron` / `read_ron` to whichever backend the target has. Run it a few
//! times and watch the count climb.
//!
//! Run from the repo root:  `cargo run --example save_counter`   (ESC to quit)

use engine::{
    save, App, Color, DrawText, InputState, KeyCode, ShouldQuit, System, TextQueue, Vec2,
    WindowConfig, World,
};

struct HudSystem {
    count: u32,
}

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(q) = world.resource_mut::<ShouldQuit>() {
                    q.quit();
                }
            }
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("This app has been launched {} time(s).", self.count),
                Vec2::new(60.0, 250.0),
                28.0,
                Color::WHITE,
            ));
            tq.push(DrawText::new(
                "Saved via write_ron — a file natively, localStorage on the web. ESC to quit.",
                Vec2::new(60.0, 300.0),
                18.0,
                Color::rgb(0.6, 0.7, 0.85),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "hud_system"
    }
}

fn main() {
    // Load → increment → save. `read_ron` returns NotFound on the first run, so default to 0.
    let path = save::save_path("skeleton_save_demo", "launches.ron");
    let count: u32 = save::read_ron(&path).unwrap_or(0) + 1;
    if let Err(e) = save::write_ron(&path, &count) {
        // On a platform without persistence the count simply won't stick — log and carry on.
        log::warn!("save_counter: could not persist launch count: {e}");
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "save_counter — persistent launch count".to_string(),
        width: 760,
        height: 520,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.add_system(HudSystem { count });
    app.run();
}
