use engine::{
    AnimationClip, AnimationPlayer, AnimationStateMachine, AnimationSystem, App, AtlasSprite,
    Camera, CharacterController, DrawText, Entity, Events, InputState, KeyCode, PhysicsBody,
    PhysicsSystem, PhysicsWorld, ShouldQuit, Sprite, StateMachineSystem, System, TextQueue,
    TileCollider, Tilemap, TilemapAtlas, TilemapSystem, Transform, TransitionCond, TriggerEvent,
    UvRect, WindowConfig, World,
};
use glam::Vec2;
use rapier2d::na as nalgebra;
use rapier2d::prelude::vector;

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 540;
const PPU: f32 = 64.0;
const TILE_SIZE: f32 = 64.0;

const PLAYER_START: Vec2 = Vec2::new(160.0, 380.0);
const PLAYER_SIZE: Vec2 = Vec2::new(64.0, 64.0);
const PLAYER_HALF_PHYSICS: Vec2 = Vec2::new(0.32, 0.46);
const GOAL_POS: Vec2 = Vec2::new(1312.0, 224.0);
const FALL_Y: f32 = 700.0;

const MOVE_ACCEL: f32 = 2600.0;
const GROUND_DECEL: f32 = 3200.0;
const AIR_DECEL: f32 = 900.0;
const MAX_SPEED_X: f32 = 270.0;
const GRAVITY: f32 = 1450.0;
const JUMP_SPEED: f32 = 560.0;
const COYOTE_TIME: f32 = 0.10;
const JUMP_BUFFER: f32 = 0.11;

// Level as a uniform tile grid (one char per 64px tile). The collider geometry is
// generated from this same map by `PhysicsWorld::add_static_from_tilemap`, so the
// level is defined in exactly one place.
//   '.' = empty   'G' = ground   'B' = block   'O' = one-way platform
const LEVEL: &[&str] = &[
    "........................",
    "........................",
    "........................",
    "........................",
    "...............BBBBBBBB.",
    "..OOO......BBB..........",
    ".......BBB..............",
    "GGGGGG..................",
    "........................",
    "........................",
];

// Tilemap tile ids (atlas index + 1; 0 = empty).
const TILE_GROUND: u32 = 1; // atlas index 0
const TILE_BLOCK: u32 = 2; // atlas index 1
const TILE_ONEWAY: u32 = 9; // atlas index 8

/// Parse the ASCII `LEVEL` into a `tiles[row][col]` grid for `Tilemap`.
fn level_tiles() -> Vec<Vec<u32>> {
    LEVEL
        .iter()
        .map(|row| {
            row.chars()
                .map(|c| match c {
                    'G' => TILE_GROUND,
                    'B' => TILE_BLOCK,
                    'O' => TILE_ONEWAY,
                    _ => 0,
                })
                .collect()
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PlayStatus {
    Playing,
    Won,
    Failed,
}

struct Player;
struct Goal;

struct PlatformerSession {
    player: Entity,
    goal: Entity,
    camera_anchor: Entity,
    status: PlayStatus,
    velocity: Vec2,
    coyote_timer: f32,
    jump_buffer_timer: f32,
}

struct PlatformerPhysicsSystem {
    physics: PhysicsSystem,
}

impl PlatformerPhysicsSystem {
    fn new(physics: PhysicsWorld) -> Self {
        Self {
            physics: PhysicsSystem::new(physics, PPU),
        }
    }

    fn reset_player(&mut self, world: &mut World) {
        let Some(player) = world.resource::<PlatformerSession>().map(|s| s.player) else {
            return;
        };
        let Some(body) = world.get::<PhysicsBody>(player) else {
            return;
        };

        let physics_pos = PLAYER_START / PPU;
        if let Some(rb) = self.physics.physics.rigid_body_mut(body.rigid_body_handle) {
            rb.set_translation(vector![physics_pos.x, physics_pos.y], true);
            rb.set_next_kinematic_translation(vector![physics_pos.x, physics_pos.y]);
        }
        if let Some(transform) = world.get_mut::<Transform>(player) {
            transform.position = PLAYER_START;
        }
        if let Some(controller) = world.get_mut::<CharacterController>(player) {
            controller.grounded = false;
        }
        if let Some(session) = world.resource_mut::<PlatformerSession>() {
            session.status = PlayStatus::Playing;
            session.velocity = Vec2::ZERO;
            session.coyote_timer = 0.0;
            session.jump_buffer_timer = 0.0;
        }
    }
}

impl System for PlatformerPhysicsSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (move_axis, jump_pressed, drop_pressed, restart_pressed, quit_pressed) = world
            .resource::<InputState>()
            .map(|input| {
                let mut axis = 0.0;
                if input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft) {
                    axis -= 1.0;
                }
                if input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight) {
                    axis += 1.0;
                }
                (
                    axis,
                    input.just_pressed(KeyCode::Space)
                        || input.just_pressed(KeyCode::KeyW)
                        || input.just_pressed(KeyCode::ArrowUp),
                    input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown),
                    input.just_pressed(KeyCode::KeyR),
                    input.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((0.0, false, false, false, false));

        if quit_pressed {
            if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                quit.0 = true;
            }
        }
        if restart_pressed {
            self.reset_player(world);
        }

        let (player, status) = match world.resource::<PlatformerSession>() {
            Some(session) => (session.player, session.status),
            None => return,
        };

        if status == PlayStatus::Playing {
            let grounded = world
                .get::<CharacterController>(player)
                .map(|c| c.grounded)
                .unwrap_or(false);

            let mut jump_started = false;
            let velocity_y;
            {
                let Some(session) = world.resource_mut::<PlatformerSession>() else {
                    return;
                };

                if grounded {
                    session.coyote_timer = COYOTE_TIME;
                    if session.velocity.y > 0.0 {
                        session.velocity.y = 0.0;
                    }
                } else {
                    session.coyote_timer = (session.coyote_timer - dt).max(0.0);
                }

                if jump_pressed {
                    session.jump_buffer_timer = JUMP_BUFFER;
                } else {
                    session.jump_buffer_timer = (session.jump_buffer_timer - dt).max(0.0);
                }

                let target_x = move_axis * MAX_SPEED_X;
                if move_axis != 0.0 {
                    session.velocity.x = approach(session.velocity.x, target_x, MOVE_ACCEL * dt);
                } else {
                    let decel = if grounded { GROUND_DECEL } else { AIR_DECEL };
                    session.velocity.x = approach(session.velocity.x, 0.0, decel * dt);
                }

                if session.jump_buffer_timer > 0.0 && session.coyote_timer > 0.0 {
                    session.velocity.y = -JUMP_SPEED;
                    session.jump_buffer_timer = 0.0;
                    session.coyote_timer = 0.0;
                    jump_started = true;
                } else {
                    session.velocity.y = (session.velocity.y + GRAVITY * dt).min(900.0);
                }
                velocity_y = session.velocity.y;
            }

            let handles = world.get::<PhysicsBody>(player).map(|body| {
                (
                    body.rigid_body_handle,
                    body.collider_handle,
                    world
                        .resource::<PlatformerSession>()
                        .map(|s| s.velocity)
                        .unwrap_or(Vec2::ZERO),
                )
            });

            if let Some((rb, col, velocity)) = handles {
                if let Some(controller) = world.get_mut::<CharacterController>(player) {
                    // Pressing down while grounded drops through one-way platforms.
                    if drop_pressed && controller.grounded {
                        controller.request_drop();
                    }
                    self.physics.physics.move_character(
                        controller,
                        rb,
                        col,
                        velocity * dt,
                        dt,
                        PPU,
                    );
                    if controller.grounded && velocity_y > 0.0 {
                        if let Some(session) = world.resource_mut::<PlatformerSession>() {
                            session.velocity.y = 0.0;
                        }
                    }
                }
            }

            let running = move_axis != 0.0
                && world
                    .resource::<PlatformerSession>()
                    .map(|s| s.velocity.x.abs() > 20.0)
                    .unwrap_or(false);
            let grounded_now = world
                .get::<CharacterController>(player)
                .map(|c| c.grounded)
                .unwrap_or(false);
            if let Some(sm) = world.get_mut::<AnimationStateMachine>(player) {
                sm.set_bool("is_running", running);
                sm.set_bool("is_grounded", grounded_now);
                sm.set_float("vertical_velocity", velocity_y);
                if jump_started {
                    sm.fire_trigger("jump");
                }
            }
        }

        self.physics.run(world, dt);

        if let Some((player, pos)) = world
            .resource::<PlatformerSession>()
            .and_then(|s| Some((s.player, world.get::<Transform>(s.player)?.position)))
        {
            if pos.y > FALL_Y {
                if let Some(session) = world.resource_mut::<PlatformerSession>() {
                    if session.player == player && session.status == PlayStatus::Playing {
                        session.status = PlayStatus::Failed;
                    }
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "PlatformerPhysicsSystem"
    }
}

struct GoalSystem;

impl System for GoalSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, goal, status)) = world
            .resource::<PlatformerSession>()
            .map(|s| (s.player, s.goal, s.status))
        else {
            return;
        };
        if status != PlayStatus::Playing {
            return;
        }

        let reached_goal = world
            .resource::<Events<TriggerEvent>>()
            .map(|events| {
                events.read().iter().any(|event| match *event {
                    TriggerEvent::Entered(a, b) => {
                        (a == player && b == goal) || (a == goal && b == player)
                    }
                    TriggerEvent::Exited(_, _) => false,
                })
            })
            .unwrap_or(false);

        if reached_goal {
            if let Some(session) = world.resource_mut::<PlatformerSession>() {
                session.status = PlayStatus::Won;
                session.velocity = Vec2::ZERO;
            }
        }
    }

    fn name(&self) -> &'static str {
        "GoalSystem"
    }
}

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(status) = world.resource::<PlatformerSession>().map(|s| s.status) else {
            return;
        };

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Move: A/D   Jump: Space/W   Drop: S/Down (one-way)   Restart: R   Quit: Esc",
                Vec2::new(-(WINDOW_W as f32) * 0.48, -(WINDOW_H as f32) * 0.46),
                20.0,
                [235, 245, 255, 255],
            ));

            match status {
                PlayStatus::Playing => {}
                PlayStatus::Won => tq.push(DrawText::new(
                    "Goal reached! Press R to play again.",
                    Vec2::new(-180.0, -30.0),
                    30.0,
                    [120, 255, 170, 255],
                )),
                PlayStatus::Failed => tq.push(DrawText::new(
                    "You fell. Press R to restart.",
                    Vec2::new(-160.0, -30.0),
                    30.0,
                    [255, 180, 120, 255],
                )),
            }
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

struct CameraAnchorSystem;

impl System for CameraAnchorSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, anchor)) = world
            .resource::<PlatformerSession>()
            .map(|s| (s.player, s.camera_anchor))
        else {
            return;
        };
        let Some(player_pos) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        if let Some(anchor_transform) = world.get_mut::<Transform>(anchor) {
            anchor_transform.position =
                player_pos - Vec2::new(WINDOW_W as f32, WINDOW_H as f32) * 0.5;
            anchor_transform.position.x = anchor_transform.position.x.max(0.0);
            anchor_transform.position.y = anchor_transform.position.y.clamp(0.0, 120.0);
        }
    }

    fn name(&self) -> &'static str {
        "CameraAnchorSystem"
    }
}

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

fn frames(row: u32) -> Vec<UvRect> {
    (0..4)
        .map(|col| UvRect::from_grid(col, row, 4, 4))
        .collect()
}

fn animation_player() -> AnimationPlayer {
    AnimationPlayer::new(vec![
        AnimationClip {
            frames: frames(0),
            fps: 4.0,
            looping: true,
        },
        AnimationClip {
            frames: frames(1),
            fps: 10.0,
            looping: true,
        },
        AnimationClip {
            frames: frames(2),
            fps: 8.0,
            looping: false,
        },
        AnimationClip {
            frames: frames(3),
            fps: 8.0,
            looping: true,
        },
    ])
}

fn animation_state_machine() -> AnimationStateMachine {
    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("run", 1)
        .add_state("jump", 2)
        .add_state("fall", 3);
    sm.set_bool("is_running", false);
    sm.set_bool("is_grounded", false);
    sm.set_float("vertical_velocity", 0.0);
    sm.add_trigger("jump");

    sm.add_transition(
        "idle",
        "run",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), true),
            TransitionCond::BoolEq("is_running".into(), true),
        ],
    )
    .add_transition(
        "run",
        "idle",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), true),
            TransitionCond::BoolEq("is_running".into(), false),
        ],
    )
    .add_transition("idle", "jump", vec![TransitionCond::Trigger("jump".into())])
    .add_transition("run", "jump", vec![TransitionCond::Trigger("jump".into())])
    .add_transition(
        "idle",
        "fall",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), false),
            TransitionCond::FloatGt("vertical_velocity".into(), 30.0),
        ],
    )
    .add_transition(
        "run",
        "fall",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), false),
            TransitionCond::FloatGt("vertical_velocity".into(), 30.0),
        ],
    )
    .add_transition(
        "jump",
        "fall",
        vec![TransitionCond::FloatGt("vertical_velocity".into(), 30.0)],
    )
    .add_transition(
        "jump",
        "idle",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), true),
            TransitionCond::BoolEq("is_running".into(), false),
        ],
    )
    .add_transition(
        "jump",
        "run",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), true),
            TransitionCond::BoolEq("is_running".into(), true),
        ],
    )
    .add_transition(
        "fall",
        "idle",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), true),
            TransitionCond::BoolEq("is_running".into(), false),
        ],
    )
    .add_transition(
        "fall",
        "run",
        vec![
            TransitionCond::BoolEq("is_grounded".into(), true),
            TransitionCond::BoolEq("is_running".into(), true),
        ],
    );
    sm
}

/// Spawn the level: one `Tilemap` entity for rendering and one static collider per
/// solid tile via `PhysicsWorld::add_static_from_tilemap`. Both read the same grid.
fn spawn_level(app: &mut App, physics: &mut PhysicsWorld) {
    let tiles = level_tiles();
    let tilemap = Tilemap::new(
        TilemapAtlas::new("examples/games/platformer/assets/platform_tiles.png", 4, 4),
        tiles,
        TILE_SIZE,
        Vec2::ZERO,
    );

    // One collider per solid tile; one-way tiles become pass-through-from-below platforms.
    physics.add_static_from_tilemap(&tilemap, PPU, |id| match id {
        0 => None,
        TILE_ONEWAY => Some(TileCollider::one_way()),
        _ => Some(TileCollider::solid()),
    });

    // The Tilemap component drives `TilemapSystem`, which spawns the tile sprites.
    let map_entity = app.world.spawn();
    app.world.add_component(map_entity, tilemap);
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine platformer game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.08, 0.12, 0.18, 1.0],
    });
    app.register_event::<TriggerEvent>();

    let player_atlas = app.load_atlas("examples/games/platformer/assets/player_atlas.png", 4, 4);
    // Load the seamless tile texture so `TilemapSystem`'s tile sprites resolve it by path.
    // (The original `tiles.png` is a set of discrete object sprites with transparent
    // margins — fine when stretched per-platform, but it shows gaps as 1:1 tiles.)
    // platform_tiles.png is generated by `examples/gen_platform_tiles.rs` (deterministic).
    app.load_atlas("examples/games/platformer/assets/platform_tiles.png", 4, 4);
    let goal_image = app.load_image("examples/games/platformer/assets/goal.png");

    let mut physics = PhysicsWorld::new(Vec2::ZERO);

    spawn_level(&mut app, &mut physics);

    let (goal_rb, goal_col) = physics.add_sensor_box(GOAL_POS / PPU, 0.36, 0.58);
    let goal = app.world.spawn();
    app.world.add_component(
        goal,
        Transform {
            position: GOAL_POS,
            scale: Vec2::splat(88.0),
            z: 0.2,
            ..Default::default()
        },
    );
    app.world.add_component(
        goal,
        Sprite::textured_with_handle(
            "examples/games/platformer/assets/goal.png",
            Some(goal_image),
        ),
    );
    app.world.add_component(
        goal,
        PhysicsBody {
            rigid_body_handle: goal_rb,
            collider_handle: goal_col,
        },
    );
    app.world.add_component(goal, Goal);

    let (player_rb, player_col) = physics.add_kinematic_box(
        PLAYER_START / PPU,
        PLAYER_HALF_PHYSICS.x,
        PLAYER_HALF_PHYSICS.y,
    );
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: PLAYER_START,
            scale: PLAYER_SIZE,
            z: 0.4,
            ..Default::default()
        },
    );
    app.world
        .add_component(player, AtlasSprite::new(player_atlas, 0));
    app.world.add_component(
        player,
        PhysicsBody {
            rigid_body_handle: player_rb,
            collider_handle: player_col,
        },
    );
    app.world.add_component(player, CharacterController::new());
    app.world.add_component(player, animation_player());
    app.world.add_component(player, animation_state_machine());
    app.world.add_component(player, Player);

    let camera_anchor = app.world.spawn();
    app.world.add_component(
        camera_anchor,
        Transform {
            position: PLAYER_START - Vec2::new(WINDOW_W as f32, WINDOW_H as f32) * 0.5,
            scale: Vec2::ZERO,
            ..Default::default()
        },
    );

    app.world.insert_resource(PlatformerSession {
        player,
        goal,
        camera_anchor,
        status: PlayStatus::Playing,
        velocity: Vec2::ZERO,
        coyote_timer: 0.0,
        jump_buffer_timer: 0.0,
    });
    let mut camera = Camera::new(Vec2::ZERO, 1.0);
    camera.follow_entity = Some(camera_anchor);
    camera.lerp_factor = 5.0;
    app.world.insert_resource(camera);

    app.add_system(TilemapSystem::new());
    app.add_system(PlatformerPhysicsSystem::new(physics));
    app.add_system(GoalSystem);
    app.add_system(CameraAnchorSystem);
    app.add_system(AnimationSystem);
    app.add_system(StateMachineSystem);
    app.add_system(HudSystem);
    app.run();
}
