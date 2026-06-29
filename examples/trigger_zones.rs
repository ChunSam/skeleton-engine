//! `TriggerZone` — fire enter/stay/exit events when entities overlap a region.
//!
//! The 2D "Area2D" pattern: a [`TriggerZone`] watches an area and emits a [`ZoneEvent`] whenever a
//! tracked entity enters, stays in, or exits it — for pickups, damage fields, room/door triggers,
//! aggro ranges, checkpoints — without hand-polling overlaps. [`TriggerZoneSystem`] reads the
//! [`SpatialGrid`](engine::SpatialGrid) built by
//! [`CollisionGridSystem`](engine::CollisionGridSystem), so a watched entity just needs a
//! `Transform` + `Collider` (+ a matching `CollisionLayer`).
//!
//! This demo walks a player rectangle through three zones laid out left to right — a green **heal**,
//! a red **damage**, a blue **goal**. Each zone lights up while the player is inside it (read by
//! *polling* `TriggerZone::occupants`), and entering one is logged + counted (read from the
//! *event bus*). It shows both ways to consume zones: events for transitions, the component for
//! "who is inside right now".
//!
//! - **← / →** — move the player left / right through the zones
//! - **↑ / ↓** — move the player up / down (skim a zone or pass clear of it)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/trigger_zones.png cargo run --example trigger_zones`): with no
//! input the player auto-drifts right across the zones so the capture shows a live overlap; the
//! console logs each enter/exit. `HEADLESS_FRAMES=N` overrides the default 130 warm-up frames.
use engine::{
    App, Camera, Collider, CollisionGridSystem, CollisionLayer, Color, DrawText, Entity, Events,
    InputState, KeyCode, ShouldQuit, Sprite, System, TextQueue, Transform, TriggerZone,
    TriggerZoneSystem, Vec2, WindowConfig, World, ZoneEvent,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 470;
const PLAYER_SPEED: f32 = 150.0;
const PLAYER_MIN_X: f32 = 30.0;
const PLAYER_MAX_X: f32 = WIN_W as f32 - 30.0;
const PLAYER_MIN_Y: f32 = 70.0;
const PLAYER_MAX_Y: f32 = 400.0;
const ZONE_Y: f32 = 250.0;

/// Only the player is on this layer; every zone watches exactly it.
const LAYER_PLAYER: CollisionLayer = CollisionLayer(1 << 0);

/// A zone's identity + its dim/lit colors, kept by the demo so events can be named and the sprite
/// can be tinted by occupancy.
struct ZoneInfo {
    entity: Entity,
    name: &'static str,
    base: Color,
    hot: Color,
}

struct Demo {
    player: Entity,
    zones: Vec<ZoneInfo>,
    /// Headless / "plays itself" mode: drift the player horizontally regardless of input.
    auto: bool,
    /// Auto-drift direction (+1 right, -1 left); ping-pongs at the edges.
    dir: f32,
    /// Total times the player has entered any zone (counted from `ZoneEvent::Entered`).
    entries: u32,
}

impl Demo {
    fn zone_name(&self, e: Entity) -> &'static str {
        self.zones
            .iter()
            .find(|z| z.entity == e)
            .map(|z| z.name)
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

        // --- react to zone events (the transition stream) ---
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
        let mut inside: Vec<&'static str> = Vec::new();
        let mut tints: Vec<(Entity, Color)> = Vec::new();
        for z in &self.zones {
            let hot = world
                .get::<TriggerZone>(z.entity)
                .map(|tz| tz.contains(self.player))
                .unwrap_or(false);
            if hot {
                inside.push(z.name);
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
                "TriggerZone — Area2D-style enter/stay/exit zones",
                Vec2::new(36.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("inside: {inside_label}     zone entries: {}", self.entries),
                Vec2::new(36.0, 50.0),
                15.0,
                Color::rgb(0.75, 0.92, 0.95),
            ));
            tq.push(DrawText::new(
                "Arrows: move player    Esc: quit",
                Vec2::new(36.0, WIN_H as f32 - 28.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "trigger_zones_demo"
    }
}

/// Spawns a rectangular zone (a dim colored quad) that watches the player layer.
fn spawn_zone(
    app: &mut App,
    name: &'static str,
    center_x: f32,
    half: Vec2,
    base: Color,
    hot: Color,
) -> ZoneInfo {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(center_x, ZONE_Y),
            scale: half * 2.0,
            z: -1.0, // behind the player
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::colored(base.r, base.g, base.b));
    app.world
        .add_component(e, TriggerZone::rect(half).with_mask(LAYER_PLAYER));
    ZoneInfo {
        entity: e,
        name,
        base,
        hot,
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "trigger_zones — Area2D enter/stay/exit".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<ZoneEvent>();

    let zones = vec![
        spawn_zone(
            &mut app,
            "heal",
            190.0,
            Vec2::new(48.0, 64.0),
            Color::rgb(0.16, 0.34, 0.20),
            Color::rgb(0.32, 0.78, 0.42),
        ),
        spawn_zone(
            &mut app,
            "damage",
            420.0,
            Vec2::new(58.0, 74.0),
            Color::rgb(0.38, 0.16, 0.16),
            Color::rgb(0.88, 0.32, 0.30),
        ),
        spawn_zone(
            &mut app,
            "goal",
            640.0,
            Vec2::new(48.0, 64.0),
            Color::rgb(0.16, 0.22, 0.40),
            Color::rgb(0.36, 0.52, 0.92),
        ),
    ];

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

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
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
