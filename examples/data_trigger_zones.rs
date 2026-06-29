//! Data-driven `TriggerZone`s — author a level's zones in RON, load + spawn them.
//!
//! The code-built counterpart is `examples/trigger_zones.rs`. This version loads the same kind of
//! heal / damage / goal zones from [`data_trigger_zones.ron`](./data_trigger_zones.ron) via
//! [`App::load_trigger_zones`](engine::App::load_trigger_zones) and spawns them with
//! [`App::spawn_trigger_zones`](engine::App::spawn_trigger_zones) — each RON entry becomes an entity
//! with a [`Transform`], a [`TriggerZone`], and a [`Tag`] (its name). The game reacts to a
//! [`ZoneEvent`] by reading the zone entity's `Tag`, so zones can be added/moved/retuned by editing
//! the RON file (native hot-reload) without touching code.
//!
//! The zones carry no `Sprite` — this example sizes a debug quad from each zone's shape after
//! spawning, to show that rendering is the game's concern, separate from the logical zone.
//!
//! - **← / →** — move the player left / right through the zones
//! - **↑ / ↓** — move the player up / down
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/data_trigger_zones.png cargo run --example data_trigger_zones`):
//! the player auto-drifts right across the zones. `HEADLESS_FRAMES=N` overrides the default 130.
use engine::{
    App, Camera, Collider, CollisionGridSystem, CollisionLayer, Color, DrawText, Entity, Events,
    InputState, KeyCode, ShouldQuit, Sprite, System, Tag, TextQueue, Transform, TriggerShape,
    TriggerZone, TriggerZoneSystem, Vec2, WindowConfig, World, ZoneEvent,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 470;
const PLAYER_SPEED: f32 = 150.0;
const PLAYER_MIN_X: f32 = 30.0;
const PLAYER_MAX_X: f32 = WIN_W as f32 - 30.0;
const PLAYER_MIN_Y: f32 = 70.0;
const PLAYER_MAX_Y: f32 = 400.0;
const ZONE_Y: f32 = 250.0;

/// Only the player is on this layer; every zone watches exactly it (matches `mask: 1` in the RON).
const LAYER_PLAYER: CollisionLayer = CollisionLayer(1 << 0);

/// Per-zone visualization, built from the spawned entity's `Tag` + `TriggerZone` shape.
struct ZoneViz {
    entity: Entity,
    name: String,
    base: Color,
    hot: Color,
}

/// Dim/lit colors for a named zone (falls back to gray for an unknown tag).
fn colors_for(tag: &str) -> (Color, Color) {
    match tag {
        "heal" => (Color::rgb(0.16, 0.34, 0.20), Color::rgb(0.32, 0.78, 0.42)),
        "damage" => (Color::rgb(0.38, 0.16, 0.16), Color::rgb(0.88, 0.32, 0.30)),
        "goal" => (Color::rgb(0.16, 0.22, 0.40), Color::rgb(0.36, 0.52, 0.92)),
        _ => (Color::rgb(0.25, 0.25, 0.28), Color::rgb(0.6, 0.6, 0.65)),
    }
}

/// A square scale that covers the zone's detection area (so the debug quad matches it).
fn viz_scale(shape: TriggerShape) -> Vec2 {
    match shape {
        TriggerShape::Circle { radius } => Vec2::splat(radius * 2.0),
        TriggerShape::Rect { half_extents } => half_extents * 2.0,
    }
}

struct Demo {
    player: Entity,
    zones: Vec<ZoneViz>,
    auto: bool,
    dir: f32,
    entries: u32,
}

impl Demo {
    fn zone_name(&self, e: Entity) -> &str {
        self.zones
            .iter()
            .find(|z| z.entity == e)
            .map(|z| z.name.as_str())
            .unwrap_or("?")
    }
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        // --- input ---
        let (left, right, up, down, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.is_pressed(KeyCode::ArrowLeft),
                    i.is_pressed(KeyCode::ArrowRight),
                    i.is_pressed(KeyCode::ArrowUp),
                    i.is_pressed(KeyCode::ArrowDown),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((false, false, false, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // --- move the player ---
        let dx = if self.auto {
            self.dir * PLAYER_SPEED * dt
        } else {
            (right as i32 - left as i32) as f32 * PLAYER_SPEED * dt
        };
        let dy = (down as i32 - up as i32) as f32 * PLAYER_SPEED * dt;
        if let Some(t) = world.get_mut::<Transform>(self.player) {
            t.position.x += dx;
            t.position.y = (t.position.y + dy).clamp(PLAYER_MIN_Y, PLAYER_MAX_Y);
            if t.position.x <= PLAYER_MIN_X {
                t.position.x = PLAYER_MIN_X;
                self.dir = 1.0;
            } else if t.position.x >= PLAYER_MAX_X {
                t.position.x = PLAYER_MAX_X;
                self.dir = -1.0;
            }
        }

        // --- react to zone events (resolve the zone entity to its name via Tag) ---
        let mut entered_now = 0u32;
        let mut logs: Vec<String> = Vec::new();
        if let Some(ev) = world.resource::<Events<ZoneEvent>>() {
            for e in ev.read() {
                match *e {
                    ZoneEvent::Entered { zone, other } if other == self.player => {
                        entered_now += 1;
                        logs.push(format!("entered {}", self.zone_name(zone)));
                    }
                    ZoneEvent::Exited { zone, other } if other == self.player => {
                        logs.push(format!("exited {}", self.zone_name(zone)));
                    }
                    _ => {}
                }
            }
        }
        self.entries += entered_now;
        for l in logs {
            println!("{l}  (total entries: {})", self.entries);
        }

        // --- tint each zone by current occupancy (polling TriggerZone::occupants) ---
        let mut inside: Vec<String> = Vec::new();
        let mut tints: Vec<(Entity, Color)> = Vec::new();
        for z in &self.zones {
            let hot = world
                .get::<TriggerZone>(z.entity)
                .map(|tz| tz.contains(self.player))
                .unwrap_or(false);
            if hot {
                inside.push(z.name.clone());
            }
            tints.push((z.entity, if hot { z.hot } else { z.base }));
        }
        for (entity, color) in tints {
            if let Some(s) = world.get_mut::<Sprite>(entity) {
                s.color = color;
            }
        }

        // --- HUD ---
        let inside_label = if inside.is_empty() {
            "—".to_string()
        } else {
            inside.join(", ")
        };
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Data-driven TriggerZones — loaded from RON, spawned as entities",
                Vec2::new(28.0, 24.0),
                17.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("inside: {inside_label}     zone entries: {}", self.entries),
                Vec2::new(28.0, 50.0),
                15.0,
                Color::rgb(0.75, 0.92, 0.95),
            ));
            tq.push(DrawText::new(
                "Arrows: move player    Esc: quit    (edit data_trigger_zones.ron to retune)",
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "data_trigger_zones_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "data_trigger_zones — zones authored in RON".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<ZoneEvent>();

    // Data-driven: load the zone set from RON, then spawn it. Each entry → an entity with a
    // Transform + TriggerZone (+ a Tag with its name).
    app.load_trigger_zones("level", "examples/data_trigger_zones.ron");
    let spawned = app.spawn_trigger_zones("level");

    // Decorate each spawned zone with a debug quad sized to its shape (rendering is the game's job).
    let mut zones: Vec<ZoneViz> = Vec::new();
    for e in spawned {
        let (name, scale) = {
            let tag = app
                .world
                .get::<Tag>(e)
                .map(|t| t.0.clone())
                .unwrap_or_default();
            let shape = app
                .world
                .get::<TriggerZone>(e)
                .map(|z| z.shape)
                .unwrap_or_default();
            (tag, viz_scale(shape))
        };
        let (base, hot) = colors_for(&name);
        if let Some(t) = app.world.get_mut::<Transform>(e) {
            t.scale = scale;
            t.z = -1.0; // behind the player
        }
        app.world
            .add_component(e, Sprite::colored(base.r, base.g, base.b));
        zones.push(ZoneViz {
            entity: e,
            name,
            base,
            hot,
        });
    }

    // Player: a small quad with a circle collider on the watched layer.
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(40.0, ZONE_Y),
            scale: Vec2::new(34.0, 34.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.95, 0.93, 0.88));
    app.world
        .add_component(player, Collider::Circle { radius: 16.0 });
    app.world.add_component(player, LAYER_PLAYER);

    let headless = std::env::var("HEADLESS_SHOT").is_ok();

    // Order: grid → zones (emit events) → demo (reads events + moves player).
    app.add_system(CollisionGridSystem::new(128.0));
    app.add_system(TriggerZoneSystem::default());
    app.add_system(Demo {
        player,
        zones,
        auto: headless,
        dir: 1.0,
        entries: 0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(130);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
