//! Split-screen example (Phase 46): two `OffscreenCamera`s.
//!
//! P1 (green) and P2 (blue) view different locations in a left/right split:
//! - Left half: "left_view" RenderTarget (P1's view)
//! - Right half: "right_view" RenderTarget (P2's view)
use engine::{
    App, Camera, Color, KeyCode, OffscreenCamera, RenderLayer, Sprite, System, Transform,
    WindowConfig, World,
};
use glam::Vec2;

// ─── Tag components ──────────────────────────────────────────────────────────
#[derive(Clone)]
struct Player1;

#[derive(Clone)]
struct Player2;

// ─── System: P1 movement (WASD) + P2 movement (arrow keys) ───────────────────
struct MoveSystem;

impl System for MoveSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let speed = 200.0;

        // P1: WASD, P2: arrow keys
        let mut d1 = Vec2::ZERO;
        let mut d2 = Vec2::ZERO;

        if let Some(input) = world.resource::<engine::InputState>() {
            if input.is_pressed(KeyCode::KeyA) {
                d1.x -= 1.0;
            }
            if input.is_pressed(KeyCode::KeyD) {
                d1.x += 1.0;
            }
            if input.is_pressed(KeyCode::KeyW) {
                d1.y += 1.0;
            }
            if input.is_pressed(KeyCode::KeyS) {
                d1.y -= 1.0;
            }
            if input.is_pressed(KeyCode::ArrowLeft) {
                d2.x -= 1.0;
            }
            if input.is_pressed(KeyCode::ArrowRight) {
                d2.x += 1.0;
            }
            if input.is_pressed(KeyCode::ArrowUp) {
                d2.y += 1.0;
            }
            if input.is_pressed(KeyCode::ArrowDown) {
                d2.y -= 1.0;
            }
        }

        // Move P1
        let p1_entities: Vec<_> = world.query::<Player1>().map(|(e, _)| e).collect();
        for e in &p1_entities {
            if let Some(t) = world.get_mut::<Transform>(*e) {
                t.position += d1 * speed * dt;
            }
        }

        // Move P2
        let p2_entities: Vec<_> = world.query::<Player2>().map(|(e, _)| e).collect();
        for e in &p2_entities {
            if let Some(t) = world.get_mut::<Transform>(*e) {
                t.position += d2 * speed * dt;
            }
        }

        // Update OffscreenCamera to follow each player's position
        let p1_pos = p1_entities
            .first()
            .and_then(|&e| world.get::<Transform>(e))
            .map(|t| t.position);
        let p2_pos = p2_entities
            .first()
            .and_then(|&e| world.get::<Transform>(e))
            .map(|t| t.position);

        let oc_entities: Vec<_> = world.query::<OffscreenCamera>().map(|(e, _)| e).collect();
        for e in oc_entities {
            if let Some(oc) = world.get_mut::<OffscreenCamera>(e) {
                if oc.target == "left_view" {
                    if let Some(pos) = p1_pos {
                        oc.camera.position = pos;
                    }
                } else if oc.target == "right_view" {
                    if let Some(pos) = p2_pos {
                        oc.camera.position = pos;
                    }
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "MoveSystem"
    }
}

fn main() {
    let mut app = App::new();

    // Window settings (800×600)
    app.world.insert_resource(WindowConfig {
        title: "Phase 46 — Split Screen".into(),
        width: 800,
        height: 600,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });

    // ─── Register offscreen render targets ───────────────────────────────────
    // Each view covers half the screen width × full height
    app.create_render_target("left_view", 400, 600);
    app.create_render_target("right_view", 400, 600);

    // ─── Player 1 (green, left side) ─────────────────────────────────────────
    let p1 = app.world.spawn();
    app.world.add_component(
        p1,
        Transform {
            position: Vec2::new(-200.0, 0.0),
            scale: Vec2::new(40.0, 40.0),
            z: 1.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        p1,
        Sprite {
            color: Color::rgba(0.2, 0.9, 0.3, 1.0),
            ..Default::default()
        },
    );
    app.world.add_component(p1, Player1);

    // ─── Player 2 (blue, right side) ─────────────────────────────────────────
    let p2 = app.world.spawn();
    app.world.add_component(
        p2,
        Transform {
            position: Vec2::new(200.0, 0.0),
            scale: Vec2::new(40.0, 40.0),
            z: 1.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        p2,
        Sprite {
            color: Color::rgba(0.2, 0.4, 1.0, 1.0),
            ..Default::default()
        },
    );
    app.world.add_component(p2, Player2);

    // ─── Background objects ───────────────────────────────────────────────────
    let bg_objects = [
        (Vec2::new(0.0, 150.0), Color::rgba(0.8, 0.7, 0.2, 1.0)),
        (Vec2::new(-100.0, -100.0), Color::rgba(0.7, 0.3, 0.8, 1.0)),
        (Vec2::new(100.0, -150.0), Color::rgba(0.3, 0.7, 0.8, 1.0)),
        (Vec2::new(-300.0, 50.0), Color::rgba(0.9, 0.5, 0.2, 1.0)),
        (Vec2::new(300.0, 100.0), Color::rgba(0.5, 0.9, 0.5, 1.0)),
    ];
    for (pos, color) in bg_objects {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::new(50.0, 50.0),
                z: 0.0,
                ..Default::default()
            },
        );
        app.world.add_component(
            e,
            Sprite {
                color,
                ..Default::default()
            },
        );
    }

    // ─── Background tiles ─────────────────────────────────────────────────────
    for i in -6..=6 {
        for j in -4..=4 {
            let bg = app.world.spawn();
            app.world.add_component(
                bg,
                Transform {
                    position: Vec2::new(i as f32 * 100.0, j as f32 * 100.0),
                    scale: Vec2::new(90.0, 90.0),
                    z: -1.0,
                    ..Default::default()
                },
            );
            let shade = if (i + j) % 2 == 0 { 0.12 } else { 0.18 };
            app.world.add_component(
                bg,
                Sprite {
                    color: Color::rgba(shade, shade, shade, 1.0),
                    ..Default::default()
                },
            );
            app.world.add_component(bg, RenderLayer(-1));
        }
    }

    // ─── OffscreenCamera entities ────────────────────────────────────────────
    // left_view: P1 initial position
    let oc1 = app.world.spawn();
    app.world.add_component(
        oc1,
        OffscreenCamera {
            target: "left_view".to_string(),
            camera: Camera::new(Vec2::new(-200.0, 0.0), 1.0),
            layer_mask: 1 << 0, // world content only (layer ≤0) — prevents self-capture of display sprites (layer 20)
        },
    );

    // right_view: P2 initial position
    let oc2 = app.world.spawn();
    app.world.add_component(
        oc2,
        OffscreenCamera {
            target: "right_view".to_string(),
            camera: Camera::new(Vec2::new(200.0, 0.0), 1.0),
            layer_mask: 1 << 0, // world content only (layer ≤0) — prevents self-capture of display sprites (layer 20)
        },
    );

    // ─── View sprites displayed on screen ────────────────────────────────────
    // Positioned in world coordinates relative to screen center (0,0).
    // left_view: left half of screen (-200, 0), size 400×600
    // right_view: right half of screen (+200, 0), size 400×600
    let left_sprite = app.world.spawn();
    app.world.add_component(
        left_sprite,
        Transform {
            position: Vec2::new(-200.0, 0.0),
            scale: Vec2::new(400.0, 600.0),
            z: 200.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        left_sprite,
        Sprite {
            texture: Some("left_view".to_string()),
            color: Color::WHITE,
            ..Default::default()
        },
    );
    app.world.add_component(left_sprite, RenderLayer(20));

    let right_sprite = app.world.spawn();
    app.world.add_component(
        right_sprite,
        Transform {
            position: Vec2::new(200.0, 0.0),
            scale: Vec2::new(400.0, 600.0),
            z: 200.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        right_sprite,
        Sprite {
            texture: Some("right_view".to_string()),
            color: Color::WHITE,
            ..Default::default()
        },
    );
    app.world.add_component(right_sprite, RenderLayer(20));

    app.add_system(MoveSystem);
    app.run();
}
