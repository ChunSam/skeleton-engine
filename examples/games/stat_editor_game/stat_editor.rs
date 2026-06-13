//! stat_editor_game — Phase C acceptance example: `register_editable_component` + DataTable.
//!
//! Proves the full editor integration loop for a game-side stats component:
//!   • `Stats` is visible and editable per-entity in the Inspector (F2 → select entity).
//!   • Editing `Stats.hp` in the Inspector immediately updates the HUD text drawn above
//!     each entity, so the live-edit effect is visible without running the game.
//!   • "Save Scene" in the editor toolbar writes `saved_scene.ron`; restarting the
//!     example reloads it, restoring all Stats values (serde persistence).
//!   • The "enemies" DataTable is hot-reloaded: edit
//!     `examples/games/stat_editor_game/enemies.ron`, then click the Data Tables tab
//!     in the editor bottom panel to see the changes without restarting.
//!
//! # Workflow
//!   1. Run — three enemy entities appear, seeded from `enemies.ron` (Goblin, Orc, + default).
//!   2. Press F2 — docked editor opens.  Select an entity; edit Stats fields in the Inspector.
//!   3. The HP value drawn above the entity updates immediately.
//!   4. Click "Save Scene" (editor toolbar) → saves `saved_scene.ron` in CWD.
//!   5. Quit (Esc) and rerun → the edited Stats values are restored from the scene file.
//!   6. Edit `enemies.ron` on disk → the Data Tables panel reflects the change live.

use engine::{
    App, Camera, DataTableRegistry, DrawText, Entity, Scene, Sprite, System, SystemRegistrar, Tag,
    TextAnchor, TextQueue, Transform, WindowConfig, World,
};
use engine_reflect_derive::Reflect;
use serde::{Deserialize, Serialize};

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 600;

// ── Stats component ───────────────────────────────────────────────────────────

#[derive(Reflect, Serialize, Deserialize, Clone, Default, Debug)]
struct Stats {
    hp: f32,
    stamina: f32,
    strength: i32,
    agility: i32,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn configure_window(world: &mut World) {
    world.insert_resource(WindowConfig {
        title: "stat_editor_game — register_editable_component demo".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
}

/// Read an integer cell from the enemies table and return it as f32.
fn table_int_as_f32(registry: &DataTableRegistry, row: usize, col: &str) -> Option<f32> {
    match registry.get("enemies")?.get(row, col)? {
        ron::Value::Number(ron::Number::Integer(n)) => Some(*n as f32),
        ron::Value::Number(ron::Number::Float(f)) => Some(f.get() as f32),
        _ => None,
    }
}

/// Read an integer cell from the enemies table and return it as i32.
fn table_int(registry: &DataTableRegistry, row: usize, col: &str) -> Option<i32> {
    match registry.get("enemies")?.get(row, col)? {
        ron::Value::Number(ron::Number::Integer(n)) => Some(*n as i32),
        _ => None,
    }
}

/// Seed a `Stats` from the enemies DataTable row, falling back to the default on any miss.
fn stats_from_table(registry: &DataTableRegistry, row: usize) -> Stats {
    Stats {
        hp: table_int_as_f32(registry, row, "hp").unwrap_or(10.0),
        stamina: 100.0,
        strength: table_int(registry, row, "strength").unwrap_or(1),
        agility: table_int(registry, row, "agility").unwrap_or(1),
    }
}

// ── Scene ─────────────────────────────────────────────────────────────────────

struct GameScene {
    entities: Vec<Entity>,
}

impl GameScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

// Name / position / tint for the three enemy entities.
const ENEMY_DEFS: &[(&str, f32, [u8; 4])] = &[
    ("Goblin", -240.0, [80, 200, 100, 255]),
    ("Orc", 0.0, [200, 100, 80, 255]),
    ("Skeleton", 240.0, [180, 180, 220, 255]),
];

impl Scene for GameScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        configure_window(world);

        // Snapshot table data before mutably borrowing world for spawning.
        // The registry may not exist on wasm (load_data_table is a no-op there).
        let seeded: Vec<Stats> = {
            if let Some(reg) = world.resource::<DataTableRegistry>() {
                ENEMY_DEFS
                    .iter()
                    .enumerate()
                    .map(|(i, _)| stats_from_table(reg, i))
                    .collect()
            } else {
                ENEMY_DEFS.iter().map(|_| Stats::default()).collect()
            }
        };

        for (i, &(name, x, tint)) in ENEMY_DEFS.iter().enumerate() {
            let e = world.spawn();
            world.add_component(
                e,
                Transform {
                    position: glam::Vec2::new(x, 0.0),
                    scale: glam::Vec2::splat(48.0),
                    ..Default::default()
                },
            );
            world.add_component(
                e,
                Sprite::colored(
                    tint[0] as f32 / 255.0,
                    tint[1] as f32 / 255.0,
                    tint[2] as f32 / 255.0,
                ),
            );
            world.add_component(e, Tag(name.to_string()));
            world.add_component(e, seeded[i].clone());
            self.entities.push(e);
        }

        systems.add(HudSystem);
    }

    fn on_exit(&mut self, world: &mut World) {
        for e in self.entities.drain(..) {
            world.despawn(e);
        }
    }
}

// ── HUD system: draw hp above each entity ────────────────────────────────────

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Collect (screen_pos, hp, name) without holding world borrow across the push.
        let entries: Vec<(glam::Vec2, f32, String)> = {
            let camera = match world.resource::<Camera>() {
                Some(c) => *c,
                None => return,
            };
            world
                .query2::<Transform, Stats>()
                .map(|(e, tf, stats)| {
                    let name = world.get::<Tag>(e).map(|t| t.0.clone()).unwrap_or_default();
                    // Place text 40px above the entity sprite in screen space.
                    let screen = camera.world_to_screen(tf.position) + glam::Vec2::new(0.0, -60.0);
                    (screen, stats.hp, name)
                })
                .collect()
        };

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for (pos, hp, name) in entries {
                tq.push(
                    DrawText::new(
                        format!("{name}\nHP {:.0}", hp),
                        pos,
                        16.0,
                        [220u8, 240, 200, 255],
                    )
                    .with_anchor(TextAnchor::Center),
                );
            }
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = env_logger::try_init();

    println!();
    println!("=== stat_editor_game ===");
    println!("Demonstrates register_editable_component + DataTable live editing.");
    println!();
    println!("  F2                       — open docked editor");
    println!("  Select entity → Inspector — edit Stats (hp / stamina / strength / agility)");
    println!("  HP number above entity     — updates immediately as you type");
    println!("  Editor toolbar 'Save Scene'— saves layout+stats to saved_scene.ron in CWD");
    println!("  Restart                    — saved Stats values are restored");
    println!(
        "  Editor → Data Tables tab   — view enemies / items tables; edit on disk → hot-reload"
    );
    println!("  Esc                        — quit");
    println!();

    let mut app = App::new();
    app.register_editable_component::<Stats>("Stats", None);
    app.load_data_table("enemies", "examples/games/stat_editor_game/enemies.ron");
    app.load_data_table("items", "examples/games/stat_editor_game/items.ron");
    configure_window(&mut app.world);
    app.set_scene(Box::new(GameScene::new()));
    app.run();
}
