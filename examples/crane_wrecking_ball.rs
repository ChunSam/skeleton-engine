//! Crane wrecking-ball — dogfoods the engine's physics **joints** in real play.
//!
//! A kinematic crane cart hangs a long rigid arm (pinned by a **revolute joint**) with a
//! heavy ball tethered on the end (a **distance joint**). Drive the cart left/right to swing
//! the ball and smash a stack of blocks off its pedestal. This is the first time the joint
//! API (`PhysicsWorld::add_revolute_joint` / `add_distance_joint`) is exercised by a playable
//! example — it shipped with unit tests but zero game/example coverage (see `docs/NEXT_WORK.md`).
//!
//! It also showcases the engine fix the example forced out: `PhysicsSystem` now syncs each
//! body's **rotation** into `Transform.rotation`, not just its position. Without it the
//! swinging arm would render bolt-upright while the physics rotates underneath it; here the
//! arm (and tumbling blocks) visibly rotate with the simulation. The HUD shows the live arm
//! angle so you can watch the sync.
//!
//! Run:
//!     cargo run --example crane_wrecking_ball
//!
//! Controls: hold Left/A and Right/D to roll the cart and swing the ball · R reset · Esc quit.
//!
//! (Native-only: the `physics` module is gated off wasm, like the platformer example.)

use engine::{
    App, BodyHandle, Camera, Color, DrawText, Entity, InputState, KeyCode, PhysicsBody,
    PhysicsSystem, PhysicsWorld, ShouldQuit, Sprite, System, TextQueue, Transform, WindowConfig,
    World,
};
use glam::Vec2;
use rapier2d::na as nalgebra;
use rapier2d::prelude::vector;

const WINDOW_W: u32 = 1000;
const WINDOW_H: u32 = 720;
const PPU: f32 = 50.0; // pixels per physics unit (visible world ≈ 20 × 14.4 units)

const GRAVITY: f32 = 14.0;

// Crane cart (kinematic): rolls along a horizontal rail near the top.
const CART_Y: f32 = 2.0;
const CART_START_X: f32 = 4.0;
const CART_HALF: Vec2 = Vec2::new(0.7, 0.3);
const RAIL_MIN_X: f32 = 1.2;
const RAIL_MAX_X: f32 = 15.0;
const CART_SPEED: f32 = 6.0; // units / second

// Arm (dynamic, revolute-pinned to the cart) + ball (dynamic, distance-tethered to the arm).
const ARM_POS: Vec2 = Vec2::new(CART_START_X, 4.3);
const ARM_HALF: Vec2 = Vec2::new(0.13, 2.0);
const BALL_POS: Vec2 = Vec2::new(CART_START_X, 6.9);
const BALL_RADIUS: f32 = 0.6;
const BALL_EXTRA_MASS: f32 = 3.0; // make it a *wrecking* ball vs. the light blocks

// Block stack on a pedestal off to the right.
const PEDESTAL_POS: Vec2 = Vec2::new(14.0, 8.2);
const PEDESTAL_HALF: Vec2 = Vec2::new(1.5, 0.4);
const BLOCK_HALF: f32 = 0.38;
const BLOCK_COUNT: usize = 4;
const KNOCK_DIST: f32 = 1.5; // a block this far from its start counts as knocked off

const BLOCK_COLORS: [[f32; 3]; BLOCK_COUNT] = [
    [0.95, 0.80, 0.25],
    [0.30, 0.80, 0.45],
    [0.30, 0.65, 0.95],
    [0.85, 0.45, 0.85],
];
const KNOCKED_COLOR: Color = Color::rgba(0.40, 0.40, 0.45, 1.0);

/// A dynamic body whose position/rotation we restore on reset.
struct Dynamic {
    handle: BodyHandle,
    home: Vec2,
}

/// A stack block: tracked for the win condition and recolored once knocked off.
struct Block {
    entity: Entity,
    handle: BodyHandle,
    home: Vec2,
    color: [f32; 3],
    knocked: bool,
}

/// Layers crane control and win/reset logic on top of the engine physics step.
/// Calls `self.physics.run` to drive the `PhysicsWorld` resource (step + Transform sync) —
/// the code path that syncs rotation, which this example exists to prove.
struct CraneSystem {
    physics: PhysicsSystem,
    cart: BodyHandle,
    cart_x: f32,
    arm_entity: Entity,
    dynamics: Vec<Dynamic>,
    blocks: Vec<Block>,
    won: bool,
}

impl CraneSystem {
    fn reset(&mut self, world: &mut World) {
        let ident = nalgebra::UnitComplex::new(0.0); // angle 0 = identity rotation

        // Cart back to the rail start.
        self.cart_x = CART_START_X;
        if let Some(physics) = world.resource_mut::<PhysicsWorld>() {
            if let Some(rb) = physics.rigid_body_mut(self.cart) {
                rb.set_translation(vector![CART_START_X, CART_Y], true);
                rb.set_next_kinematic_translation(vector![CART_START_X, CART_Y]);
            }
            // Arm + ball + blocks back home, motionless.
            for d in &self.dynamics {
                if let Some(rb) = physics.rigid_body_mut(d.handle) {
                    rb.set_translation(vector![d.home.x, d.home.y], true);
                    rb.set_rotation(ident, true);
                    rb.set_linvel(vector![0.0, 0.0], true);
                    rb.set_angvel(0.0, true);
                }
            }
        }
        for b in &mut self.blocks {
            b.knocked = false;
            if let Some(sprite) = world.get_mut::<Sprite>(b.entity) {
                sprite.color = Color::rgb(b.color[0], b.color[1], b.color[2]);
            }
        }
        self.won = false;
    }
}

impl System for CraneSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── input ────────────────────────────────────────────────────────────
        let (quit, reset, dir) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            let left = input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA);
            let right = input.is_pressed(KeyCode::ArrowRight) || input.is_pressed(KeyCode::KeyD);
            let dir = (right as i32 - left as i32) as f32;
            (
                input.just_pressed(KeyCode::Escape),
                input.just_pressed(KeyCode::KeyR),
                dir,
            )
        };
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
            return;
        }
        if reset {
            self.reset(world);
        }

        // ── drive the kinematic cart along the rail ──────────────────────────
        self.cart_x = (self.cart_x + dir * CART_SPEED * dt).clamp(RAIL_MIN_X, RAIL_MAX_X);
        if let Some(physics) = world.resource_mut::<PhysicsWorld>() {
            if let Some(rb) = physics.rigid_body_mut(self.cart) {
                rb.set_next_kinematic_translation(vector![self.cart_x, CART_Y]);
            }
        }

        // ── engine step + Transform sync (position AND rotation) ─────────────
        self.physics.run(world, dt);

        // ── win check: latch blocks that left their stack, recolor them ──────
        for b in &mut self.blocks {
            if b.knocked {
                continue;
            }
            let pos = {
                let Some(physics) = world.resource::<PhysicsWorld>() else {
                    continue;
                };
                let Some(rb) = physics.rigid_body(b.handle) else {
                    continue;
                };
                let p = rb.translation();
                Vec2::new(p.x, p.y)
            };
            if (pos - b.home).length() > KNOCK_DIST {
                b.knocked = true;
                if let Some(sprite) = world.get_mut::<Sprite>(b.entity) {
                    sprite.color = KNOCKED_COLOR;
                }
            }
        }
        let knocked = self.blocks.iter().filter(|b| b.knocked).count();
        self.won = knocked == self.blocks.len();

        // ── HUD ──────────────────────────────────────────────────────────────
        let arm_angle = world
            .get::<Transform>(self.arm_entity)
            .map(|t| t.rotation)
            .unwrap_or(0.0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("blocks knocked off: {knocked}/{}", self.blocks.len()),
                Vec2::new(14.0, 12.0),
                24.0,
                [235, 235, 245, 255],
            ));
            // The arm angle is written by PhysicsSystem's rotation sync — watch it track the swing.
            tq.push(DrawText::new(
                format!("arm angle (physics → Transform.rotation): {arm_angle:+.2} rad"),
                Vec2::new(14.0, 42.0),
                18.0,
                [170, 205, 255, 235],
            ));
            if self.won {
                tq.push(DrawText::new(
                    "WRECKED IT!  press R to reset",
                    Vec2::new(14.0, 78.0),
                    30.0,
                    [120, 255, 160, 255],
                ));
            }
            tq.push(DrawText::new(
                "hold Left/A + Right/D to swing the ball   R reset   Esc quit",
                Vec2::new(14.0, WINDOW_H as f32 - 30.0),
                16.0,
                [170, 185, 210, 200],
            ));
        }
    }
}

/// Spawn a render entity (Transform + colored Sprite + PhysicsBody) for a physics body.
/// `Transform.position` / `.rotation` are overwritten every frame by `PhysicsSystem`; the
/// scale is set once here from the collider's pixel size.
fn spawn_visual(
    app: &mut App,
    handle: BodyHandle,
    collider: engine::ColliderHandle,
    full_size_px: Vec2,
    color: [f32; 3],
    z: f32,
) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::ZERO,
            scale: full_size_px,
            rotation: 0.0,
            z,
        },
    );
    app.world
        .add_component(e, Sprite::colored(color[0], color[1], color[2]));
    app.world.add_component(
        e,
        PhysicsBody {
            rigid_body_handle: handle,
            collider_handle: collider,
        },
    );
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine crane wrecking-ball".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    let mut physics = PhysicsWorld::new(Vec2::new(0.0, GRAVITY));

    // ── static scenery ───────────────────────────────────────────────────────
    let (ground_rb, ground_col) = physics.add_static_box(Vec2::new(10.0, 14.0), 10.0, 0.4);
    spawn_visual(
        &mut app,
        ground_rb,
        ground_col,
        Vec2::new(20.0 * PPU, 0.8 * PPU),
        [0.18, 0.18, 0.22],
        0.0,
    );
    let (ped_rb, ped_col) = physics.add_static_box(PEDESTAL_POS, PEDESTAL_HALF.x, PEDESTAL_HALF.y);
    spawn_visual(
        &mut app,
        ped_rb,
        ped_col,
        PEDESTAL_HALF * 2.0 * PPU,
        [0.45, 0.32, 0.20],
        0.5,
    );

    // ── crane: kinematic cart + revolute arm + distance-tethered ball ─────────
    let (cart_rb, cart_col) =
        physics.add_kinematic_box(Vec2::new(CART_START_X, CART_Y), CART_HALF.x, CART_HALF.y);
    spawn_visual(
        &mut app,
        cart_rb,
        cart_col,
        CART_HALF * 2.0 * PPU,
        [0.70, 0.72, 0.78],
        2.5,
    );

    let (arm_rb, arm_col) = physics.add_dynamic_box(ARM_POS, ARM_HALF.x, ARM_HALF.y, false);
    let arm_entity = spawn_visual(
        &mut app,
        arm_rb,
        arm_col,
        ARM_HALF * 2.0 * PPU,
        [0.55, 0.60, 0.70],
        2.0,
    );

    let (ball_rb, ball_col) = physics.add_dynamic_circle(BALL_POS, BALL_RADIUS, false);
    if let Some(rb) = physics.rigid_body_mut(ball_rb) {
        rb.set_additional_mass(BALL_EXTRA_MASS, true);
    }
    spawn_visual(
        &mut app,
        ball_rb,
        ball_col,
        Vec2::splat(BALL_RADIUS * 2.0 * PPU),
        [0.75, 0.20, 0.18],
        3.0,
    );

    // Revolute: pivot at the cart's bottom edge. Local anchors — cart (0, +half_h),
    // arm (0, -half_h). The arm then swings freely around the pivot (and rotates,
    // which PhysicsSystem now syncs to its sprite).
    physics.add_revolute_joint(
        cart_rb,
        arm_rb,
        Vec2::new(0.0, CART_HALF.y),
        Vec2::new(0.0, -ARM_HALF.y),
    );
    // Distance: short rigid-ish tether from the arm tip to the ball center.
    physics.add_distance_joint(arm_rb, ball_rb, Vec2::new(0.0, ARM_HALF.y), Vec2::ZERO, 0.5);

    // ── block stack ──────────────────────────────────────────────────────────
    let mut dynamics = vec![
        Dynamic {
            handle: arm_rb,
            home: ARM_POS,
        },
        Dynamic {
            handle: ball_rb,
            home: BALL_POS,
        },
    ];
    let mut blocks = Vec::with_capacity(BLOCK_COUNT);
    let stack_top = PEDESTAL_POS.y - PEDESTAL_HALF.y - BLOCK_HALF; // bottom block center
    for (i, color) in BLOCK_COLORS.iter().enumerate() {
        let home = Vec2::new(
            PEDESTAL_POS.x,
            stack_top - i as f32 * (BLOCK_HALF * 2.0 + 0.02),
        );
        let (rb, col) = physics.add_dynamic_box(home, BLOCK_HALF, BLOCK_HALF, false);
        let entity = spawn_visual(
            &mut app,
            rb,
            col,
            Vec2::splat(BLOCK_HALF * 2.0 * PPU),
            *color,
            1.0,
        );
        dynamics.push(Dynamic { handle: rb, home });
        blocks.push(Block {
            entity,
            handle: rb,
            home,
            color: *color,
            knocked: false,
        });
    }

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.world.insert_resource(physics);
    app.add_system(CraneSystem {
        physics: PhysicsSystem::new(PPU),
        cart: cart_rb,
        cart_x: CART_START_X,
        arm_entity,
        dynamics,
        blocks,
        won: false,
    });

    app.run();
}
