//! Data-driven Zone→Effect bindings — author a level's zones, particles, AND reactions in RON.
//!
//! This builds on `examples/data_trigger_zones.rs`: the same heal / damage / goal zones are loaded
//! from RON ([`zone_effects_zones.ron`](./zone_effects_zones.ron)), but instead of the game wiring
//! reactions in Rust, a [`ZoneEffectSystem`](engine::ZoneEffectSystem) reads each [`ZoneEvent`],
//! resolves the zone to its [`Tag`] name, and runs the [`zone_effects.ron`](./zone_effects.ron)
//! bindings — a [`HitFlash`](engine::HitFlash) on the player, a particle burst (visuals from
//! [`zone_effects_particles.ron`](./zone_effects_particles.ron)), and a tone. Three RON files,
//! composed by tag, **zero reaction glue in code**:
//!
//! - **heal** → sparkle burst at the zone + a high tone
//! - **damage** → the player flashes red + a blood burst at the player + a low tone
//! - **goal** → a big sparkle burst at the player + a fanfare tone
//!
//! Controls: **← / →** move horizontally, **↑ / ↓** move vertically, **Esc** quits.
//!
//! Headless (`HEADLESS_SHOT=/tmp/zone_effects.png cargo run --example zone_effects`): the player
//! auto-drifts across the zones, firing each binding. `HEADLESS_FRAMES=N` overrides the default 130.
use engine::{
    App, Audio, AudioFacadeSystem, Camera, Collider, CollisionGridSystem, CollisionLayer, Color,
    DrawText, Entity, Events, HitFlashSystem, InputState, KeyCode, ParticleSystem, ShouldQuit,
    Sprite, System, Tag, TextQueue, Transform, TriggerShape, TriggerZone, TriggerZoneSystem, Vec2,
    WindowConfig, World, ZoneEffectSystem, ZoneEvent,
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

/// Dim base color for a named zone quad (the *reaction* is the feedback now, not a tint).
fn color_for(tag: &str) -> Color {
    match tag {
        "heal" => Color::rgb(0.16, 0.34, 0.20),
        "damage" => Color::rgb(0.38, 0.16, 0.16),
        "goal" => Color::rgb(0.16, 0.22, 0.40),
        _ => Color::rgb(0.25, 0.25, 0.28),
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
    zones: Vec<(Entity, String)>,
    auto: bool,
    dir: f32,
    entries: u32,
    last: String,
}

impl Demo {
    fn zone_name(&self, e: Entity) -> &str {
        self.zones
            .iter()
            .find(|(ze, _)| *ze == e)
            .map(|(_, n)| n.as_str())
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

        // --- move the player (auto-drift + bounce when headless) ---
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

        // --- report zone events (the EFFECTS are applied by ZoneEffectSystem before us) ---
        if let Some(ev) = world.resource::<Events<ZoneEvent>>() {
            let mut logs: Vec<(bool, String)> = Vec::new();
            for e in ev.read() {
                match *e {
                    ZoneEvent::Entered { zone, other } if other == self.player => {
                        logs.push((true, self.zone_name(zone).to_string()));
                    }
                    ZoneEvent::Exited { zone, other } if other == self.player => {
                        logs.push((false, self.zone_name(zone).to_string()));
                    }
                    _ => {}
                }
            }
            for (entered, name) in logs {
                if entered {
                    self.entries += 1;
                    self.last = name.clone();
                    println!(
                        "entered {name} → fired its effects  (total: {})",
                        self.entries
                    );
                } else {
                    println!("exited {name}");
                }
            }
        }

        // --- HUD ---
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Zone → Effect bindings — zones + particles + reactions, all authored in RON",
                Vec2::new(24.0, 22.0),
                16.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!(
                    "last entered: {}     zone entries: {}",
                    self.last, self.entries
                ),
                Vec2::new(24.0, 48.0),
                15.0,
                Color::rgb(0.75, 0.92, 0.95),
            ));
            tq.push(DrawText::new(
                "heal: sparkle+tone   damage: flash+blood+tone   goal: sparkle+fanfare",
                Vec2::new(24.0, WIN_H as f32 - 50.0),
                14.0,
                Color::rgb(0.7, 0.78, 0.82),
            ));
            tq.push(DrawText::new(
                "Arrows: move   Esc: quit   (edit zone_effects.ron to retune reactions)",
                Vec2::new(24.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "zone_effects_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "zone_effects — RON zones + particles + reactions".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<ZoneEvent>();

    // Three data-driven layers, composed by the zone's Tag name:
    //   1. the zones themselves (where),  2. the particle emitters they reference,
    //   3. the effect bindings (what happens on enter/stay/exit).
    app.load_trigger_zones("level", "examples/zone_effects_zones.ron");
    app.load_particle_configs("fx", "examples/zone_effects_particles.ron");
    app.load_zone_effects("effects", "examples/zone_effects.ron");

    // Spawn the zones and decorate each with a debug quad sized to its shape.
    let spawned = app.spawn_trigger_zones("level");
    let mut zones: Vec<(Entity, String)> = Vec::new();
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
        if let Some(t) = app.world.get_mut::<Transform>(e) {
            t.scale = scale;
            t.z = -1.0; // behind the player + particles
        }
        let c = color_for(&name);
        app.world.add_component(e, Sprite::colored(c.r, c.g, c.b));
        zones.push((e, name));
    }

    // Player: a small quad with a circle collider on the watched layer. It needs a Sprite so the
    // damage zone's `Flash` effect (a HitFlash) is visible.
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

    // Audio is optional: skip it cleanly when there's no device (headless / locked session). The
    // PlayTone effects no-op without an `Audio` resource.
    if let Some(audio) = Audio::new() {
        app.world.insert_resource(audio);
        app.add_system(AudioFacadeSystem);
    }

    let headless = std::env::var("HEADLESS_SHOT").is_ok();

    // Order: grid → zones (emit events) → zone-effects (apply effects) → demo (reports + moves) →
    // particle/flash systems (advance the spawned bursts + the flash).
    app.add_system(CollisionGridSystem::new(128.0));
    app.add_system(TriggerZoneSystem::default());
    app.add_system(ZoneEffectSystem::new("effects"));
    app.add_system(Demo {
        player,
        zones,
        auto: headless,
        dir: 1.0,
        entries: 0,
        last: "—".to_string(),
    });
    app.add_system(ParticleSystem);
    app.add_system(HitFlashSystem);

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
