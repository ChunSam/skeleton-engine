//! Fork-friendly **custom RON-loaded config registry** — using the public
//! [`RonRegistry`](engine::RonRegistry) + [`RonLoadable`](engine::RonLoadable).
//!
//! The engine's particle / dialogue / animation-clip config registries are all thin wrappers over
//! one generic registry. This example shows a *game* using that same registry for its **own** RON
//! config type — no engine fork: define a config struct, implement `RonLoadable` for it, and load
//! named instances from RON files with `RonRegistry::load` / `get` / `names`.
//!
//! Here the config is a `CreatureStats` (name / hp / speed / colour) loaded from
//! `examples/assets/creatures/*.ron`. The window lists each loaded creature as a colour bar sized by
//! HP. **Edit a `.ron` file and press `R`** to re-read them from disk (the registry's load is
//! hot-reload-capable) and watch the bars change live.
//!
//! Native-only: `RonLoadable` / hot-reload don't run on wasm (no filesystem).
//!
//! Run: `cargo run --example ron_registry`

use engine::{
    App, DrawText, FontData, InputState, KeyCode, RonRegistry, ShouldQuit, System, TextQueue, Vec2,
    WindowConfig, World,
};

/// A game's own config type — nothing engine-special, just `serde::Deserialize`.
#[derive(serde::Deserialize, Clone)]
struct CreatureStats {
    name: String,
    hp: i32,
    speed: f32,
    r: f32,
    g: f32,
    b: f32,
}

impl CreatureStats {
    fn color(&self) -> [u8; 4] {
        let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0) as u8;
        [c(self.r), c(self.g), c(self.b), 255]
    }
}

// Implementing the public `RonLoadable` trait is all it takes to get `RonRegistry::load` +
// canonical-path hot-reload for a custom type. We parse via the engine's `read_ron` (RON → T).
impl engine::RonLoadable for CreatureStats {
    type Err = engine::SaveError;
    fn load_ron(path: &str) -> Result<Self, Self::Err> {
        engine::save::read_ron(std::path::Path::new(path))
    }
}

/// `(registry name, RON path)` — absolute paths so the example runs from any CWD.
const CREATURES: &[(&str, &str)] = &[
    (
        "goblin",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/assets/creatures/goblin.ron"
        ),
    ),
    (
        "dragon",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/assets/creatures/dragon.ron"
        ),
    ),
    (
        "slime",
        concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/examples/assets/creatures/slime.ron"
        ),
    ),
];

struct CreatureDemo {
    registry: RonRegistry<CreatureStats>,
    status: String,
}

impl CreatureDemo {
    fn load_all(&mut self) -> usize {
        let mut ok = 0;
        for (name, path) in CREATURES {
            match self.registry.load(*name, path) {
                Ok(()) => ok += 1,
                Err(e) => log::warn!("ron_registry: failed to load {name}: {e}"),
            }
        }
        ok
    }
}

impl Default for CreatureDemo {
    fn default() -> Self {
        let mut demo = Self {
            registry: RonRegistry::default(),
            status: String::new(),
        };
        let n = demo.load_all();
        demo.status = format!("loaded {n} creatures");
        demo
    }
}

impl System for CreatureDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let reload = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::KeyR))
            .unwrap_or(false);
        let quit = world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false);

        if reload {
            let n = self.load_all();
            self.status = format!("reloaded {n} creatures from disk");
        }
        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let x = 24.0;
            tq.push(DrawText::new(
                "RonRegistry — a game's own RON-loaded config type (no engine fork)",
                Vec2::new(x, 24.0),
                21.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                "examples/assets/creatures/*.ron  ->  RonRegistry<CreatureStats>    edit a .ron + press R to hot-reload",
                Vec2::new(x, 54.0),
                13.0,
                [180, 200, 220, 255],
            ));

            let mut y = 100.0;
            for name in self.registry.names() {
                if let Some(c) = self.registry.get(&name) {
                    let col = c.color();
                    // A colour bar sized by HP (1 block ≈ 8 HP, capped).
                    let blocks = ((c.hp / 8).clamp(1, 46)) as usize;
                    tq.push(DrawText::new(
                        format!("{:<8} HP {:>3}  SPD {:>3.0}", c.name, c.hp, c.speed),
                        Vec2::new(x, y),
                        16.0,
                        col,
                    ));
                    tq.push(DrawText::new(
                        "\u{2588}".repeat(blocks),
                        Vec2::new(x + 230.0, y + 1.0),
                        15.0,
                        col,
                    ));
                    y += 30.0;
                }
            }

            tq.push(DrawText::new(
                format!("registry.names() = {:?}", self.registry.names()),
                Vec2::new(x, y + 14.0),
                13.0,
                [120, 130, 150, 255],
            ));
            tq.push(DrawText::new(
                format!("last: {}", self.status),
                Vec2::new(x, y + 36.0),
                14.0,
                [160, 255, 170, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "CreatureDemo"
    }
}

fn main() {
    let _ = env_logger::try_init();
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — RonRegistry custom config".to_string(),
        width: 720,
        height: 320,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });
    app.world.insert_resource(FontData(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/DejaVuSans.ttf"
        ))
        .to_vec(),
    ));
    app.add_system(CreatureDemo::default());
    app.run();
}
