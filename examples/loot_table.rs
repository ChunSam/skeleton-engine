//! Loot table — deterministic weighted drops with a live distribution histogram.
//!
//! Composes [`Rng`] (a seedable, deterministic PRNG) with [`WeightedTable`] (weighted random
//! selection) into the classic loot-drop / spawn-table loop. Four rarities are added to a
//! `WeightedTable<usize>` with relative weights (Common 60 · Uncommon 25 · Rare 12 · Legendary 3);
//! each "chest" draws one with `table.pick(&mut rng)`. A histogram shows the observed distribution
//! converging to the target weights (a thin marker line marks each target), and because the draws
//! are driven by a seeded `Rng`, **R** replays the identical sequence — the determinism a game
//! relies on to reproduce a run from just its seed.
//!
//! - **Space** — open one chest (draw one item)
//! - **A** — open 100 chests at once (watch the histogram converge)
//! - **R** — replay: reset to the same seed and re-draw the identical sequence
//! - **N** — new seed (a different sequence)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/loot.png cargo run --example loot_table`): pre-rolls 200 draws so
//! the histogram is populated. `HEADLESS_FRAMES=N` overrides the default 6 frames.
use engine::{
    App, Color, DrawRect, DrawText, InputState, KeyCode, Rng, Scene, ShouldQuit, System,
    SystemRegistrar, TextQueue, UiQueue, WeightedTable, WindowConfig, World,
};
use glam::Vec2;

struct Rarity {
    name: &'static str,
    color: Color,
    weight: f32,
}

const RARITIES: [Rarity; 4] = [
    Rarity {
        name: "Common",
        color: Color::rgba(0.62, 0.64, 0.70, 1.0),
        weight: 60.0,
    },
    Rarity {
        name: "Uncommon",
        color: Color::rgba(0.40, 0.80, 0.42, 1.0),
        weight: 25.0,
    },
    Rarity {
        name: "Rare",
        color: Color::rgba(0.34, 0.62, 0.95, 1.0),
        weight: 12.0,
    },
    Rarity {
        name: "Legendary",
        color: Color::rgba(0.98, 0.78, 0.24, 1.0),
        weight: 3.0,
    },
];

const ROW_H: f32 = 64.0;
const TOP: f32 = 70.0;
const LEFT: f32 = 24.0;
const SWATCH: f32 = 22.0;
const BAR_X: f32 = 150.0;
const BAR_MAX: f32 = 380.0;

fn build_table() -> WeightedTable<usize> {
    let mut table = WeightedTable::new();
    for (i, r) in RARITIES.iter().enumerate() {
        table.add(i, r.weight);
    }
    table
}

struct LootScene {
    preroll: u32,
}

impl Scene for LootScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(LootDemo::new(1, self.preroll));
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct LootDemo {
    rng: Rng,
    table: WeightedTable<usize>,
    seed: u64,
    counts: [u32; 4],
    total: u32,
    last: Option<usize>,
    flash: f32,
}

impl LootDemo {
    fn new(seed: u64, preroll: u32) -> Self {
        let mut demo = Self {
            rng: Rng::new(seed),
            table: build_table(),
            seed,
            counts: [0; 4],
            total: 0,
            last: None,
            flash: 0.0,
        };
        for _ in 0..preroll {
            demo.roll_one();
        }
        demo.flash = 0.0; // don't leave a headless preroll flashing
        demo
    }

    fn roll_one(&mut self) {
        if let Some(&i) = self.table.pick(&mut self.rng) {
            self.counts[i] += 1;
            self.total += 1;
            self.last = Some(i);
            self.flash = 1.0;
        }
    }

    /// Resets the draw state to the current seed — re-running produces the identical sequence.
    fn replay(&mut self) {
        self.rng = Rng::new(self.seed);
        self.counts = [0; 4];
        self.total = 0;
        self.last = None;
        self.flash = 0.0;
    }

    fn new_seed(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        self.replay();
    }
}

impl System for LootDemo {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── Input (gather intents, then act after the InputState borrow drops) ──
        let (mut roll1, mut roll100, mut replay, mut new_seed, mut quit) =
            (false, false, false, false, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            roll1 = input.just_pressed(KeyCode::Space);
            roll100 = input.just_pressed(KeyCode::KeyA);
            replay = input.just_pressed(KeyCode::KeyR);
            new_seed = input.just_pressed(KeyCode::KeyN);
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if replay {
            self.replay();
        }
        if new_seed {
            self.new_seed();
        }
        if roll1 {
            self.roll_one();
        }
        if roll100 {
            for _ in 0..100 {
                self.roll_one();
            }
        }
        self.flash = (self.flash - dt * 2.5).max(0.0);

        let total_w: f32 = RARITIES.iter().map(|r| r.weight).sum();

        // ── Histogram: one row per rarity ──────────────────────────────────────
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            for (i, r) in RARITIES.iter().enumerate() {
                let y = TOP + i as f32 * ROW_H;
                let obs = if self.total > 0 {
                    self.counts[i] as f32 / self.total as f32
                } else {
                    0.0
                };
                let target = r.weight / total_w;
                let is_last = self.last == Some(i);

                // Colour swatch (brightens when this rarity was the most recent drop).
                let swatch = if is_last {
                    lerp_color(r.color, Color::rgba(1.0, 1.0, 1.0, 1.0), 0.35 * self.flash)
                } else {
                    r.color
                };
                ui.items
                    .push(DrawRect::new(LEFT, y, SWATCH, SWATCH, swatch).with_z(0.2));

                // Bar track + observed fill.
                ui.items.push(
                    DrawRect::new(
                        BAR_X,
                        y,
                        BAR_MAX,
                        SWATCH,
                        Color::rgba(0.14, 0.14, 0.17, 1.0),
                    )
                    .with_z(0.1),
                );
                ui.items.push(
                    DrawRect::new(BAR_X, y, (obs * BAR_MAX).max(0.0), SWATCH, r.color).with_z(0.2),
                );

                // Thin marker at the target fraction — the bar converges to it as draws pile up.
                ui.items.push(
                    DrawRect::new(
                        BAR_X + target * BAR_MAX - 1.0,
                        y - 3.0,
                        2.0,
                        SWATCH + 6.0,
                        Color::rgba(0.92, 0.92, 0.96, 0.9),
                    )
                    .with_z(0.3),
                );
            }
        }

        // ── Labels + HUD (text pass) ───────────────────────────────────────────
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Loot table — seed {}   ·   {} chests opened",
                    self.seed, self.total
                ),
                Vec2::new(LEFT, 28.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.75),
            ));
            for (i, r) in RARITIES.iter().enumerate() {
                let y = TOP + i as f32 * ROW_H;
                let obs = if self.total > 0 {
                    100.0 * self.counts[i] as f32 / self.total as f32
                } else {
                    0.0
                };
                let target = 100.0 * r.weight / total_w;
                tq.push(DrawText::new(
                    r.name,
                    Vec2::new(LEFT + SWATCH + 10.0, y + 1.0),
                    15.0,
                    r.color,
                ));
                tq.push(DrawText::new(
                    format!(
                        "{:>6}   obs {obs:>5.1}%   target {target:>4.1}%",
                        self.counts[i]
                    ),
                    Vec2::new(BAR_X, y + SWATCH + 5.0),
                    13.0,
                    Color::rgb(0.7, 0.72, 0.78),
                ));
            }
            tq.push(DrawText::new(
                "Space: +1   A: +100   R: replay (same seed)   N: new seed   Esc: quit",
                Vec2::new(LEFT, TOP + 4.0 * ROW_H + 6.0),
                14.0,
                Color::rgb(0.66, 0.68, 0.74),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "loot_table"
    }
}

/// Linear blend between two colours (`t` in `0..=1`).
fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    Color::rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        a.a + (b.a - a.a) * t,
    )
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Loot table — weighted deterministic drops".to_string(),
        width: 580,
        height: 380,
        clear_color: [0.05, 0.05, 0.07, 1.0],
    });

    let preroll = if std::env::var("HEADLESS_SHOT").is_ok() {
        200
    } else {
        0
    };
    app.set_scene(Box::new(LootScene { preroll }));

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(6);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
