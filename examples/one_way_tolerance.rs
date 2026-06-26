//! `CharacterController::one_way_tolerance` — the one-way-platform landing skin width.
//!
//! A one-way platform keeps blocking a downward-moving character until its bottom sinks
//! more than `one_way_tolerance` **physics units** below the platform's top surface, so a
//! resting character does not jitter or slip through on slight numerical penetration. Because
//! it is a *physics-unit* length, it must scale with your `pixels_per_unit`: the default
//! `0.05` suits the engine's nominal PPU (≈64), so this demo — which runs at a deliberately
//! coarse **PPU 24** (a large physics world, small pixels-per-unit) — sets a proportionally
//! larger value via `with_one_way_tolerance`, keeping the same *visual* skin. Before this knob
//! the skin width was hardcoded to `0.05`, so a fork at any other scale had to edit engine source.
//!
//! Play: **←/→** move · **↑/Space** jump (you can jump *up through* the one-way platform and
//! land on top) · **↓** drop through it onto the solid ground below. The HUD shows the live
//! `one_way_tolerance` and the grounded state.
//!
//! The deterministic proof that the value changes behavior (a too-tight skin lets a penetrated
//! character slip through, a wide one catches it) lives in the engine unit test
//! `one_way_tolerance_widens_the_landing_skin`.
use engine::{
    App, CharacterController, DrawText, Entity, InputState, KeyCode, PhysicsBody, PhysicsSystem,
    PhysicsWorld, ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 540;
/// Deliberately *not* the nominal ≈64: a coarse scale where the default skin width would be
/// too thin, so the demo must scale the tolerance to its PPU.
const PPU: f32 = 24.0;

const GRAVITY: f32 = 1400.0; // px/s²
const JUMP_SPEED: f32 = 760.0; // px/s — enough to rise from the ground through the one-way platform
const MOVE_SPEED: f32 = 240.0; // px/s

// Layout in pixels (world ≈ screen, top-left origin, +y down).
const GROUND_CENTER: Vec2 = Vec2::new(360.0, 500.0);
const GROUND_SIZE: Vec2 = Vec2::new(720.0, 40.0);
const ONEWAY_CENTER: Vec2 = Vec2::new(360.0, 320.0);
const ONEWAY_SIZE: Vec2 = Vec2::new(360.0, 20.0);
const PLAYER_START: Vec2 = Vec2::new(360.0, 120.0);
const PLAYER_SIZE: Vec2 = Vec2::new(36.0, 48.0);

struct Demo {
    player: Entity,
    vel: Vec2,
    grounded: bool,
    tolerance: f32,
}

/// Reads input, applies gravity, and drives the character through one-way collisions.
struct MovementSystem;

impl System for MovementSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let mut move_x = 0.0;
        let mut jump = false;
        let mut drop = false;
        let mut quit = false;
        if let Some(input) = world.resource::<InputState>() {
            if input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA) {
                move_x -= 1.0;
            }
            if input.is_pressed(KeyCode::ArrowRight) || input.is_pressed(KeyCode::KeyD) {
                move_x += 1.0;
            }
            jump = input.just_pressed(KeyCode::ArrowUp) || input.just_pressed(KeyCode::Space);
            drop = input.just_pressed(KeyCode::ArrowDown) || input.just_pressed(KeyCode::KeyS);
            quit = input.just_pressed(KeyCode::Escape);
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        let (player, grounded) = {
            let demo = world.resource::<Demo>().unwrap();
            (demo.player, demo.grounded)
        };

        // Integrate velocity: horizontal from input, vertical from gravity / jump.
        {
            let demo = world.resource_mut::<Demo>().unwrap();
            demo.vel.x = move_x * MOVE_SPEED;
            if jump && grounded {
                demo.vel.y = -JUMP_SPEED;
            } else {
                demo.vel.y = (demo.vel.y + GRAVITY * dt).min(1600.0);
            }
        }

        if drop && grounded {
            if let Some(ctrl) = world.get_mut::<CharacterController>(player) {
                ctrl.request_drop();
            }
        }

        let handles = world
            .get::<PhysicsBody>(player)
            .map(|b| (b.rigid_body_handle, b.collider_handle));
        let vel = world.resource::<Demo>().unwrap().vel;

        // move_character needs &mut PhysicsWorld and &mut CharacterController at once — pull the
        // world out of the resource map to split the borrow (the platformer pattern).
        if let (Some((rb, col)), Some(mut physics)) =
            (handles, world.remove_resource::<PhysicsWorld>())
        {
            if let Some(ctrl) = world.get_mut::<CharacterController>(player) {
                physics.move_character(ctrl, rb, col, vel * dt, dt, PPU);
                let g = ctrl.grounded;
                if let Some(demo) = world.resource_mut::<Demo>() {
                    demo.grounded = g;
                    if g && demo.vel.y > 0.0 {
                        demo.vel.y = 0.0; // landed — clear downward velocity
                    }
                }
            }
            world.insert_resource(physics);
        }
    }

    fn name(&self) -> &'static str {
        "MovementSystem"
    }
}

/// Draws the HUD (controls + live tolerance + grounded state).
struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (tolerance, grounded) = {
            let demo = world.resource::<Demo>().unwrap();
            (demo.tolerance, demo.grounded)
        };
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "one_way_tolerance — one-way platform landing skin (physics units)",
                Vec2::new(16.0, 14.0),
                17.0,
                [235, 225, 200, 255],
            ));
            tq.push(DrawText::new(
                "<- / -> move    Up/Space jump (through from below)    Down drop through",
                Vec2::new(16.0, 40.0),
                14.0,
                [180, 195, 215, 255],
            ));
            tq.push(DrawText::new(
                format!(
                    "PPU {PPU:.0}   one_way_tolerance {tolerance:.3}  (default 0.05 x 64/PPU)   grounded: {}",
                    if grounded { "yes" } else { "no" }
                ),
                Vec2::new(16.0, WIN_H as f32 - 26.0),
                14.0,
                [160, 220, 255, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

fn spawn_box(app: &mut App, center: Vec2, size: Vec2, color: Sprite) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: center,
            scale: size,
            ..Default::default()
        },
    );
    app.world.add_component(e, color);
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "CharacterController::one_way_tolerance".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let mut physics = PhysicsWorld::new(Vec2::ZERO);

    // Solid ground (blocks from every direction) — the drop-through landing surface.
    spawn_box(
        &mut app,
        GROUND_CENTER,
        GROUND_SIZE,
        Sprite::colored(0.25, 0.27, 0.34),
    );
    let (_g_rb, _g_col) = physics.add_static_box(
        GROUND_CENTER / PPU,
        GROUND_SIZE.x * 0.5 / PPU,
        GROUND_SIZE.y * 0.5 / PPU,
    );

    // One-way platform: blocks from above, passes from below, drop-through with Down.
    spawn_box(
        &mut app,
        ONEWAY_CENTER,
        ONEWAY_SIZE,
        Sprite::colored(0.3, 0.55, 0.9),
    );
    let (_o_rb, o_col) = physics.add_static_box(
        ONEWAY_CENTER / PPU,
        ONEWAY_SIZE.x * 0.5 / PPU,
        ONEWAY_SIZE.y * 0.5 / PPU,
    );
    physics.set_one_way(o_col, true);

    // Player — a kinematic box driven by the CharacterController.
    let player = spawn_box(
        &mut app,
        PLAYER_START,
        PLAYER_SIZE,
        Sprite::colored(0.95, 0.6, 0.25),
    );
    let (p_rb, p_col) = physics.add_kinematic_box(
        PLAYER_START / PPU,
        PLAYER_SIZE.x * 0.5 / PPU,
        PLAYER_SIZE.y * 0.5 / PPU,
    );
    app.world.add_component(
        player,
        PhysicsBody {
            rigid_body_handle: p_rb,
            collider_handle: p_col,
        },
    );
    // Scale the physics-unit skin width to THIS demo's PPU so the visual skin matches what the
    // default 0.05 gives at the nominal PPU 64. This is the whole point of the knob.
    let tolerance = CharacterController::DEFAULT_ONE_WAY_TOLERANCE * 64.0 / PPU;
    app.world.add_component(
        player,
        CharacterController::new().with_one_way_tolerance(tolerance),
    );

    app.world.insert_resource(physics);
    app.world.insert_resource(Demo {
        player,
        vel: Vec2::ZERO,
        grounded: false,
        tolerance,
    });

    app.add_system(MovementSystem); // moves the character (sets next kinematic translation)
    app.add_system(PhysicsSystem::new(PPU)); // steps physics + syncs body -> Transform
    app.add_system(HudSystem);
    app.run();
}
