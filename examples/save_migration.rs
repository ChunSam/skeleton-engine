//! Versioned save migration demo.
//!
//! On startup this example:
//!   1. Writes a simulated **old v1 save** (`PlayerSaveV1 { level: 7, score: 1200 }`)
//!      to a temp file using [`save_versioned`] tagged at schema version 1.
//!   2. Loads it with [`load_migrated`] through a migrator whose step 1→2 inserts
//!      a `coins` field defaulted to 0, producing `PlayerSave { level, score, coins }`.
//!   3. Displays the migrated result on screen.
//!   4. On Space: re-saves at the current schema version and reloads to confirm a
//!      round-trip with no migration.
//!   5. Cleans up the temp file when the window closes.
//!
//! Note: this demo inserts its resources and adds its system directly (no `set_scene`),
//! because `set_scene` resets the `World` — resources inserted before it would be dropped.
use std::path::PathBuf;

use engine::save::{delete, load_migrated, save_versioned, SaveMigrator};
use engine::{
    App, DrawText, InputState, KeyCode, System, TextQueue, ViewportSize, WindowConfig, World,
};
use serde::{Deserialize, Serialize};

// ── Data types ────────────────────────────────────────────────────────────────

/// Old save format: v1 (no `coins` field).
#[derive(Debug, Serialize, Deserialize)]
struct PlayerSaveV1 {
    level: u32,
    score: u32,
}

/// Current save format: v2 (added `coins`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct PlayerSave {
    level: u32,
    score: u32,
    coins: u32,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn temp_save_path() -> PathBuf {
    std::env::temp_dir().join(format!("save_migration_demo_{}.save", std::process::id()))
}

fn make_migrator() -> SaveMigrator {
    // Steps must be registered from version 0 upward. This migrator covers v0→v1→v2,
    // so `current_version()` is 2; loading a v1 save applies only the 1→2 step.
    SaveMigrator::new()
        // Step 0→1: no schema change (v0 and v1 both lack `coins`).
        .step(0, |v| v)
        .step(1, |mut v| {
            // Step 1→2: add `coins` with default 0 for saves that pre-date the field.
            if let ron::Value::Map(ref mut m) = v {
                m.insert(
                    ron::Value::String("coins".into()),
                    ron::Value::Number(ron::value::Number::new(0i64)),
                );
            }
            v
        })
}

// ── DemoState resource ────────────────────────────────────────────────────────

struct DemoState {
    save_path: PathBuf,
    line1: String,
    line2: String,
    line3: String,
}

// ── System ────────────────────────────────────────────────────────────────────

struct DemoSystem;

impl System for DemoSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Space → re-save at the current schema version + reload (no migration).
        let space_pressed = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Space))
            .unwrap_or(false);

        if space_pressed {
            if let Some(state) = world.resource_mut::<DemoState>() {
                let current_save = PlayerSave {
                    level: 7,
                    score: 1200,
                    coins: 0,
                };
                let migrator = make_migrator();
                save_versioned(&state.save_path, migrator.current_version(), &current_save)
                    .expect("re-save failed");
                let reloaded: PlayerSave =
                    load_migrated(&state.save_path, &migrator).expect("reload failed");
                assert_eq!(current_save, reloaded, "round-trip mismatch");
                state.line3 = format!(
                    "Re-saved and reloaded at v{}: level {}, score {}, coins {} — round-trip OK!",
                    migrator.current_version(),
                    reloaded.level,
                    reloaded.score,
                    reloaded.coins
                );
            }
        }

        // Collect state strings before borrowing TextQueue (borrow-checker workaround).
        let (vw, _vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((900.0, 400.0));
        let cx = vw / 2.0;
        let (line1, line2, line3) = world
            .resource::<DemoState>()
            .map(|s| (s.line1.clone(), s.line2.clone(), s.line3.clone()))
            .unwrap_or_default();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::centered(
                "Save Migration Demo",
                glam::Vec2::new(cx, 80.0),
                28.0,
                [255, 220, 80, 255],
            ));
            tq.push(DrawText::centered(
                line1,
                glam::Vec2::new(cx, 160.0),
                18.0,
                [200, 255, 200, 255],
            ));
            tq.push(DrawText::centered(
                line2,
                glam::Vec2::new(cx, 220.0),
                16.0,
                [180, 180, 255, 255],
            ));
            if !line3.is_empty() {
                tq.push(DrawText::centered(
                    line3,
                    glam::Vec2::new(cx, 280.0),
                    18.0,
                    [100, 255, 150, 255],
                ));
            }
        }
    }
}

// ── Entry Point ───────────────────────────────────────────────────────────────

fn main() {
    let path = temp_save_path();

    // (1) Write a simulated old v1 save.
    let v1 = PlayerSaveV1 {
        level: 7,
        score: 1200,
    };
    save_versioned(&path, 1, &v1).expect("failed to write v1 save");

    // (2) Load and migrate from v1 → v2.
    let migrator = make_migrator();
    let loaded: PlayerSave = load_migrated(&path, &migrator).expect("migration failed");

    let line1 = format!(
        "Loaded v1 save -> migrated to v2: level {}, score {}, coins {} (defaulted)",
        loaded.level, loaded.score, loaded.coins
    );
    let line2 = "Press SPACE to re-save at v2 and reload (round-trip, no migration).".to_string();

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Save Migration Demo".to_string(),
        width: 900,
        height: 400,
        clear_color: [0.05, 0.05, 0.10, 1.0],
    });
    app.world.insert_resource(DemoState {
        save_path: path.clone(),
        line1,
        line2,
        line3: String::new(),
    });
    app.add_system(DemoSystem);
    app.run();

    // (5) Clean up the temp file after the window closes.
    delete(&path).ok();
}
