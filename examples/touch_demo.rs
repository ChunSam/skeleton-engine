/// Phase 47 example: touch input + virtual joystick demo
///
/// - Virtual joystick in the bottom-left corner (radius 60px) drives player movement
/// - Active touch points are visualized as circles (DebugDraw)
/// - Pinch zoom: two-finger spread adjusts the camera zoom
/// - Swipe: direction is logged to the console
///
/// On desktop, mouse clicks are emulated as touches so the joystick can be operated.
use engine::{
    ecs::{Entity, System, World},
    App, Camera, Color, DebugDraw, Sprite, TouchState, Transform, VirtualJoystick, WindowConfig,
};
use glam::Vec2;

// ─── Components ───────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Player;

#[derive(Clone)]
struct JoystickTag;

// ─── System: touch feedback visualization ─────────────────────────────────────

struct TouchVisualSystem;

impl System for TouchVisualSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Collect active touch points as owned values
        let touches: Vec<Vec2> = world
            .resource::<TouchState>()
            .map(|ts| ts.active_touches().map(|(_, pos)| pos).collect())
            .unwrap_or_default();

        // Log swipe detection
        let swipe = world.resource::<TouchState>().and_then(|ts| ts.swipe());
        if let Some(dir) = swipe {
            let label = if dir.x.abs() > dir.y.abs() {
                if dir.x > 0.0 {
                    "right"
                } else {
                    "left"
                }
            } else if dir.y > 0.0 {
                "down"
            } else {
                "up"
            };
            log::info!("swipe: {} ({:.0}, {:.0})", label, dir.x, dir.y);
        }

        if let Some(dbg) = world.resource_mut::<DebugDraw>() {
            for pos in touches {
                dbg.circle(pos, 24.0, [1.0, 0.6, 0.0, 0.7]);
            }
        }
    }

    fn name(&self) -> &'static str {
        "TouchVisualSystem"
    }
}

// ─── System: pinch zoom ──────────────────────────────────────────────────────

struct PinchZoomSystem;

impl System for PinchZoomSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let pinch_delta = world
            .resource::<TouchState>()
            .map(|ts| ts.pinch_delta())
            .unwrap_or(0.0);

        if pinch_delta.abs() > 0.5 {
            if let Some(cam) = world.resource_mut::<Camera>() {
                // 1px pinch delta ≈ 0.002 zoom change (empirically tuned)
                let zoom_change = pinch_delta * 0.002;
                cam.zoom = (cam.zoom + zoom_change).clamp(0.1, 5.0);
            }
        }
    }

    fn name(&self) -> &'static str {
        "PinchZoomSystem"
    }
}

// ─── System: joystick update + player movement ───────────────────────────────

struct JoystickMoveSystem;

impl System for JoystickMoveSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let speed = 200.0;

        type TouchVec = Vec<(u64, Vec2)>;
        // 1. Extract touch data as owned values (releases borrow)
        let (began, ended, active): (TouchVec, TouchVec, TouchVec) = world
            .resource::<TouchState>()
            .map(|ts| {
                (
                    ts.began().to_vec(),
                    ts.ended().to_vec(),
                    ts.active_touches().collect(),
                )
            })
            .unwrap_or_default();

        // 2. Collect joystick entity list
        let joy_entities: Vec<Entity> = world.query::<VirtualJoystick>().map(|(e, _)| e).collect();

        // 3. Update joysticks (use update_raw, no borrow conflict)
        for &e in &joy_entities {
            if let Some(joy) = world.get_mut::<VirtualJoystick>(e) {
                joy.update_raw(&began, &ended, &active);
            }
        }

        // 4. Read output from the first joystick
        let move_dir: Vec2 = joy_entities
            .first()
            .and_then(|&e| world.get::<VirtualJoystick>(e))
            .map(|joy| joy.output)
            .unwrap_or(Vec2::ZERO);

        // 5. Move player
        if move_dir.length() > 0.01 {
            let player_entities: Vec<Entity> = world.query::<Player>().map(|(e, _)| e).collect();

            for e in player_entities {
                if let Some(t) = world.get_mut::<Transform>(e) {
                    t.position += move_dir * speed * dt;
                    // Clamp to screen bounds (800x600)
                    t.position.x = t.position.x.clamp(24.0, 776.0);
                    t.position.y = t.position.y.clamp(24.0, 576.0);
                }
            }
        }

        // 6. DebugDraw visualization for joystick
        let joy_visuals: Vec<(Vec2, f32, Vec2)> = world
            .query::<VirtualJoystick>()
            .filter_map(|(_, joy)| {
                if joy.visible {
                    Some((joy.center, joy.radius, joy.stick_pos))
                } else {
                    None
                }
            })
            .collect();

        if let Some(dbg) = world.resource_mut::<DebugDraw>() {
            for (center, radius, stick_pos) in joy_visuals {
                // Base circle (white, semi-transparent)
                dbg.circle(center, radius, [1.0, 1.0, 1.0, 0.3]);
                // Stick nub (yellow)
                dbg.circle(stick_pos, 18.0, [1.0, 0.9, 0.1, 0.8]);
            }
        }
    }

    fn name(&self) -> &'static str {
        "JoystickMoveSystem"
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "Phase 47 — Touch Input & Virtual Joystick".into(),
        width: 800,
        height: 600,
        clear_color: [0.07, 0.08, 0.14, 1.0],
    });

    // ─── Player (blue square) ────────────────────────────────────────────────
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(400.0, 300.0),
            scale: Vec2::splat(48.0),
            ..Default::default()
        },
    );
    app.world.add_component(
        player,
        Sprite {
            color: Color::rgba(0.3, 0.6, 1.0, 1.0),
            ..Default::default()
        },
    );
    app.world.add_component(player, Player);

    // ─── Background grid ─────────────────────────────────────────────────────
    for i in 0..8 {
        for j in 0..6 {
            let bg = app.world.spawn();
            app.world.add_component(
                bg,
                Transform {
                    position: Vec2::new(50.0 + i as f32 * 100.0, 50.0 + j as f32 * 100.0),
                    scale: Vec2::splat(90.0),
                    z: -1.0,
                    ..Default::default()
                },
            );
            let shade = if ((i + j) as u32).is_multiple_of(2) {
                0.10
            } else {
                0.14
            };
            app.world.add_component(
                bg,
                Sprite {
                    color: Color::rgba(shade, shade, shade + 0.02, 1.0),
                    ..Default::default()
                },
            );
        }
    }

    // ─── Virtual joystick entity (bottom-left of screen) ─────────────────────
    // Coordinate system: top-left (0,0), bottom-right (800,600)
    let joy_entity = app.world.spawn();
    app.world.add_component(joy_entity, JoystickTag);
    app.world.add_component(
        joy_entity,
        VirtualJoystick::new(Vec2::new(120.0, 480.0), 60.0),
    );

    // ─── Register systems ────────────────────────────────────────────────────
    app.add_system(TouchVisualSystem);
    app.add_system(PinchZoomSystem);
    app.add_system(JoystickMoveSystem);

    app.run();
}
