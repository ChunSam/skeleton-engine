//! Parallax scrolling demo.
//!
//! A/D or ←/→ to move the player horizontally.
//! Esc to quit.
//!
//! Four background layers scroll at different speeds as the camera follows the player,
//! making depth visible. Layer factors (horizontal):
//!   0.10 — far sky (barely moves)
//!   0.35 — distant mountains
//!   0.65 — mid trees
//!   1.10 — foreground bushes (moves slightly faster than the world)

use engine::{
    App, Camera, DrawText, Entity, InputState, KeyCode, ParallaxLayer, ParallaxSystem, ShouldQuit,
    Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: f32 = 960.0;
const WIN_H: f32 = 540.0;
const WORLD_W: f32 = 3000.0;
const PLAYER_SPEED: f32 = 300.0;

// ─── Marker components ────────────────────────────────────────────────────────

#[derive(Clone)]
struct Player;

/// Invisible anchor entity whose position is the camera top-left (player centred).
#[derive(Clone)]
struct CameraAnchor;

// ─── Systems ──────────────────────────────────────────────────────────────────

struct PlayerSystem;

impl System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let mut dx = 0.0f32;
        let mut quit = false;

        if let Some(input) = world.resource::<InputState>() {
            if input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft) {
                dx -= 1.0;
            }
            if input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight) {
                dx += 1.0;
            }
            if input.just_pressed(KeyCode::Escape) {
                quit = true;
            }
        }

        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.quit();
            }
        }

        let vel = Vec2::new(dx * PLAYER_SPEED * dt, 0.0);
        let entities: Vec<Entity> = world.query::<Player>().map(|(e, _)| e).collect();
        for e in entities {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position.x = (t.position.x + vel.x).clamp(0.0, WORLD_W);
            }
        }
    }
}

/// Keeps the camera anchor centred on the player, clamped to the world edges.
struct CameraFollowSystem {
    player: Entity,
    anchor: Entity,
}

impl System for CameraFollowSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(player_pos) = world.get::<Transform>(self.player).map(|t| t.position) else {
            return;
        };
        // Camera top-left = player_pos - half viewport (centring the player).
        let target_x = (player_pos.x - WIN_W * 0.5).clamp(0.0, WORLD_W - WIN_W);
        let target_y = 0.0f32; // horizontal-only scrolling; keep y=0
        if let Some(t) = world.get_mut::<Transform>(self.anchor) {
            t.position = Vec2::new(target_x, target_y);
        }
    }
}

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        tq.push(DrawText::new(
            "A/D or ←/→ to move    Esc quit",
            Vec2::new(16.0, 14.0),
            18.0,
            [240, 240, 255, 220],
        ));

        tq.push(DrawText::new(
            "Parallax layers (horizontal factor):",
            Vec2::new(16.0, 44.0),
            15.0,
            [210, 220, 230, 200],
        ));

        // Layer legend
        let legends = [
            ("0.10  far sky         (dim blue)", [120u8, 140, 200, 200]),
            ("0.35  distant mountains (brown)", [180, 140, 100, 220]),
            ("0.65  mid trees        (green)", [80, 170, 80, 220]),
            ("1.10  foreground bushes (bright green)", [60, 220, 80, 230]),
        ];
        for (i, (label, color)) in legends.iter().enumerate() {
            tq.push(DrawText::new(
                *label,
                Vec2::new(16.0, 66.0 + i as f32 * 20.0),
                14.0,
                *color,
            ));
        }
    }
}

// ─── Setup helpers ────────────────────────────────────────────────────────────

/// Spawns several quads spread across the world x-range for a single parallax layer.
fn spawn_layer(
    world: &mut engine::World,
    factor: f32,
    y: f32,
    h: f32,
    color: [f32; 3],
    z: f32,
    count: usize,
) {
    let spacing = WORLD_W / count as f32;
    for i in 0..count {
        let x = spacing * i as f32 + spacing * 0.5;
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(x, y),
                scale: Vec2::new(spacing * 0.9, h),
                rotation: 0.0,
                z,
            },
        );
        world.add_component(e, Sprite::colored(color[0], color[1], color[2]));
        world.add_component(e, ParallaxLayer::horizontal(factor));
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "parallax_scroll — skeleton-engine".to_string(),
        width: WIN_W as u32,
        height: WIN_H as u32,
        clear_color: [0.07, 0.08, 0.14, 1.0],
    });

    // ── Parallax layers ────────────────────────────────────────────────────────
    // Y-axis: 0 = top, WIN_H = bottom (top-left origin, Y increases downward).

    // Layer 1: far sky / stars — a thin strip at the top, very slow.
    spawn_layer(
        &mut app.world,
        0.10,
        80.0, // top strip
        120.0,
        [0.16, 0.20, 0.42],
        0.1,
        12,
    );

    // Layer 2: distant mountains — wide, mid-height band.
    spawn_layer(
        &mut app.world,
        0.35,
        260.0,
        180.0,
        [0.44, 0.32, 0.18],
        0.2,
        8,
    );

    // Layer 3: mid trees — medium, lower half.
    spawn_layer(
        &mut app.world,
        0.65,
        360.0,
        130.0,
        [0.18, 0.52, 0.18],
        0.3,
        10,
    );

    // Layer 4: foreground bushes — bottom strip, faster than world.
    spawn_layer(
        &mut app.world,
        1.10,
        470.0,
        80.0,
        [0.20, 0.78, 0.22],
        0.4,
        14,
    );

    // ── Ground (static world object, no parallax) ──────────────────────────────
    {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(WORLD_W * 0.5, WIN_H - 10.0),
                scale: Vec2::new(WORLD_W, 20.0),
                rotation: 0.0,
                z: 0.5,
            },
        );
        app.world
            .add_component(e, Sprite::colored(0.35, 0.30, 0.20));
    }

    // ── Player ─────────────────────────────────────────────────────────────────
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(WIN_W * 0.5, WIN_H - 80.0), // stands on ground
            scale: Vec2::new(40.0, 60.0),
            rotation: 0.0,
            z: 1.0,
        },
    );
    app.world
        .add_component(player, Sprite::colored(1.0, 0.85, 0.2));
    app.world.add_component(player, Player);

    // ── Camera anchor (invisible; camera lerps toward it) ──────────────────────
    let anchor = app.world.spawn();
    app.world.add_component(
        anchor,
        Transform {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(anchor, CameraAnchor);

    // ── Camera resource ────────────────────────────────────────────────────────
    let mut cam = Camera::new(Vec2::ZERO, 1.0);
    cam.follow_entity = Some(anchor);
    cam.lerp_factor = 8.0;
    cam.bounds = Some((Vec2::ZERO, Vec2::new(WORLD_W, WIN_H)));
    app.world.insert_resource(cam);

    // ── Systems ────────────────────────────────────────────────────────────────
    app.add_system(PlayerSystem);
    app.add_system(CameraFollowSystem { player, anchor });
    app.add_system(ParallaxSystem);
    app.add_system(HudSystem);

    app.run();
}
