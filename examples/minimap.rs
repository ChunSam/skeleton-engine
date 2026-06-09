//! Minimap example (Phase 46): offscreen `RenderTarget`.
//!
//! Renders the world normally with the main camera, and an `OffscreenCamera`
//! builds a zoomed-out 256x256 minimap shown in the top-right corner.
use engine::{
    App, Camera, Color, DrawText, Entity, KeyCode, OffscreenCamera, RenderLayer, Sprite, System,
    TextQueue, Transform, ViewportSize, WindowConfig, World,
};
use glam::Vec2;

// ─── System: player movement ─────────────────────────────────────────────────
struct MoveSystem;

impl System for MoveSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let speed = 150.0;
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;

        {
            if let Some(input) = world.resource::<engine::InputState>() {
                if input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA) {
                    dx -= 1.0;
                }
                if input.is_pressed(KeyCode::ArrowRight) || input.is_pressed(KeyCode::KeyD) {
                    dx += 1.0;
                }
                if input.is_pressed(KeyCode::ArrowUp) || input.is_pressed(KeyCode::KeyW) {
                    dy += 1.0;
                }
                if input.is_pressed(KeyCode::ArrowDown) || input.is_pressed(KeyCode::KeyS) {
                    dy -= 1.0;
                }
            }
        }

        let entities: Vec<Entity> = world.query::<PlayerTag>().map(|(e, _)| e).collect();
        let dt = _dt;
        for e in &entities {
            if let Some(t) = world.get_mut::<Transform>(*e) {
                t.position.x += dx * speed * dt;
                t.position.y += dy * speed * dt;
            }
        }

        // Track the main camera to the player position
        let player_pos = entities
            .first()
            .and_then(|&e| world.get::<Transform>(e))
            .map(|t| t.position);
        if let (Some(pos), Some(cam)) = (player_pos, world.resource_mut::<Camera>()) {
            cam.position = pos;
        }
    }

    fn name(&self) -> &'static str {
        "MoveSystem"
    }
}

// ─── Tag components ──────────────────────────────────────────────────────────
#[derive(Clone)]
struct PlayerTag;

/// World-anchored label marker that floats above enemies.
#[derive(Clone)]
struct EnemyTag;

// ─── System: draw screen labels above world entities ─────────────────────────
/// Demonstrates [`Camera::world_to_screen`] + [`DrawText::centered`]: a nameplate
/// is anchored to each enemy's *world* position, projected to screen pixels each
/// frame so it tracks the enemy as the camera follows the player.
struct WorldLabelSystem;

impl System for WorldLabelSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Snapshot the camera (collect first to release the immutable borrow before mutably borrowing TextQueue).
        let Some(camera) = world.resource::<Camera>().copied() else {
            return;
        };
        // Place labels slightly above each enemy's world position in screen space (Y increases downward).
        let enemies: Vec<Entity> = world.query::<EnemyTag>().map(|(e, _)| e).collect();
        let labels: Vec<Vec2> = enemies
            .iter()
            .filter_map(|&e| world.get::<Transform>(e))
            .map(|t| camera.world_to_screen(t.position + Vec2::new(0.0, -28.0)))
            .collect();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for screen_pos in labels {
                tq.push(DrawText::centered(
                    "ENEMY",
                    screen_pos,
                    14.0,
                    [255, 180, 180, 230],
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "WorldLabelSystem"
    }
}

/// Minimap display sprite marker — the HUD system updates its position every frame.
#[derive(Clone)]
struct MinimapTag;

// ─── System: lock the minimap HUD position ───────────────────────────────────
struct MinimapHudSystem;

impl System for MinimapHudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // The main camera has a top-left anchor and follows the player every frame. Pin the
        // minimap sprite to the top-right corner of the camera's visible rect (a world-fixed
        // position would drift off-screen as the camera moves — this fixes that old bug).
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((800.0, 600.0));
        let Some((min, max)) = world
            .resource::<Camera>()
            .map(|cam| cam.visible_rect(vw, vh))
        else {
            return;
        };
        let inset = 90.0 + 16.0; // half sprite size + margin (zoom=1 → world=pixels)
        let target = Vec2::new(max.x - inset, min.y + inset);
        let entities: Vec<Entity> = world.query::<MinimapTag>().map(|(e, _)| e).collect();
        for e in entities {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position = target;
            }
        }
    }

    fn name(&self) -> &'static str {
        "MinimapHudSystem"
    }
}

fn main() {
    let mut app = App::new();

    // Window configuration
    app.world.insert_resource(WindowConfig {
        title: "Phase 46 — Minimap".into(),
        width: 800,
        height: 600,
        clear_color: [0.08, 0.10, 0.15, 1.0],
    });

    // ─── Register offscreen render target (minimap 256×256) ─────────────────
    app.create_render_target("minimap", 256, 256);

    // ─── Player (green box) ──────────────────────────────────────────────────
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(0.0, 0.0),
            scale: Vec2::new(40.0, 40.0),
            ..Default::default()
        },
    );
    app.world.add_component(
        player,
        Sprite {
            color: Color::rgba(0.2, 0.9, 0.3, 1.0),
            ..Default::default()
        },
    );
    app.world.add_component(player, PlayerTag);

    // ─── Enemies (red boxes) ─────────────────────────────────────────────────
    let enemy_positions = [
        Vec2::new(200.0, 100.0),
        Vec2::new(-150.0, 200.0),
        Vec2::new(300.0, -100.0),
        Vec2::new(-250.0, -180.0),
        Vec2::new(120.0, 280.0),
    ];
    for pos in enemy_positions {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::new(30.0, 30.0),
                ..Default::default()
            },
        );
        app.world.add_component(
            e,
            Sprite {
                color: Color::rgba(0.9, 0.2, 0.2, 1.0),
                ..Default::default()
            },
        );
        app.world.add_component(e, EnemyTag);
    }

    // ─── Background tiles (gray boxes) ───────────────────────────────────────
    for i in -5..=5 {
        for j in -5..=5 {
            let bg = app.world.spawn();
            app.world.add_component(
                bg,
                Transform {
                    position: Vec2::new(i as f32 * 80.0, j as f32 * 80.0),
                    scale: Vec2::new(70.0, 70.0),
                    z: -1.0,
                    ..Default::default()
                },
            );
            let shade = if ((i + j) as u32).is_multiple_of(2) {
                0.15
            } else {
                0.20
            };
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

    // ─── OffscreenCamera entity (for minimap, zoomed out) ────────────────────
    let oc_entity = app.world.spawn();
    app.world.add_component(
        oc_entity,
        OffscreenCamera {
            target: "minimap".to_string(),
            camera: Camera::new(Vec2::ZERO, 0.15),
            layer_mask: 1 << 0, // game world (layer 0) only — excludes the minimap display sprite (layer 1)
        },
    );

    // ─── Minimap display sprite (pinned to top-right of screen) ──────────────
    // Because the main camera follows the player, a world-fixed position causes
    // the minimap to drift off-screen (old bug). MinimapHudSystem updates the
    // position to the top-right of the camera's visible rect every frame so it
    // stays anchored to the corner.
    let minimap_sprite = app.world.spawn();
    app.world.add_component(
        minimap_sprite,
        Transform {
            position: Vec2::new(694.0, 106.0), // overwritten by the HUD system on frame 1
            scale: Vec2::new(180.0, 180.0),
            z: 100.0, // topmost layer
            ..Default::default()
        },
    );
    app.world.add_component(
        minimap_sprite,
        Sprite {
            texture: Some("minimap".to_string()), // RT key
            color: Color::rgba(1.0, 1.0, 1.0, 0.9),
            ..Default::default()
        },
    );
    app.world.add_component(minimap_sprite, RenderLayer(10));
    app.world.add_component(minimap_sprite, MinimapTag);

    app.add_system(MoveSystem);
    app.add_system(MinimapHudSystem);
    app.add_system(WorldLabelSystem);
    app.run();
}
