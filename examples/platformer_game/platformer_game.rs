//! `platformer_game` — the platformer genre-game of the rebuilt examples tree.
//!
//! Phase 1 of `plans/2026-08-19-examples-rebuild-plan.md`. The old tree grew one example per
//! feature and 11 of its 22 games carried no acceptance test at all; this one is built the other
//! way round — **the selftest came first** (phase 0 rebuilt `scripts/selftests.sh` before any game
//! existed) and the level is shaped so that removing any subsystem it names stops the game rather
//! than costing it a decoration.
//!
//! ```text
//! cargo run --example platformer_game                          # play it
//! PLATFORMER_SELFTEST=1 cargo run --example platformer_game    # the acceptance test (headless)
//! PLATFORMER_GEN_ASSETS=1 cargo run --example platformer_game  # regenerate assets/tiles.png
//! ```
//!
//! # What the route forces
//!
//! Every subsystem below is load-bearing on the *only* path from spawn to goal:
//!
//! | Subsystem | Why the game stops without it |
//! |---|---|
//! | `CharacterController` + `PhysicsWorld` + `TilemapColliders` | nothing to stand on |
//! | `TilemapAutotile` | the terrain draws as one flat tile — cosmetic, but see `tiles.png` below |
//! | one-way platform + `request_drop` | the switch corridor is sealed except through its ceiling |
//! | runtime `set_tile` + collider resync | the door never opens, or opens into an invisible wall |
//! | `add_prismatic_joint` | the moving platform falls out of the world and the chasm is uncrossable |
//! | `InputBuffer` | the ledge jumps stop being forgiving (and check 1 goes red) |
//! | `TriggerZone` + `ZoneEvent` | no switch, no checkpoint, no goal, no hazards |
//! | `AnimationStateMachine` driven by physics state | the player animates wrong, which check 2 sees |
//!
//! # Two placements that look arbitrary and are not
//!
//! - **The player is a `Sprite`, not an `AtlasSprite`.** `HitFlashSystem` tints a `Sprite` and is
//!   inert on an `AtlasSprite`, so the hit flash would silently never appear. The animation system
//!   writes `UvRect` either way, so a plain `Sprite` + `AnimationPlayer` animates identically.
//! - **`BlendTree1D` is on the walker, not on the player.** Its own docs say not to put a blend tree
//!   and an `AnimationStateMachine` on one entity unless you want the SM to interrupt the crossfade.
//!   The player owns the SM (its states come from physics); the walker owns the blend tree (its clip
//!   comes from one scalar, its speed).
//!
//! # assets/tiles.png
//!
//! A 4×4 grid of 32 px cells where **the cell index is the edge-4 neighbour mask**
//! (N=1, E=2, S=4, W=8) — the layout `TilemapAutotile::edge_16(0)` expects. Regenerate it with
//! `PLATFORMER_GEN_ASSETS=1`; the generator is deterministic, so the committed PNG is reproducible
//! byte-for-byte. `assets/player.png` is a 4×4 sheet of 64 px frames (row = idle / run / jump /
//! fall), carried over from the deleted tree.

use engine::{
    AnimationClip, AnimationPlayer, AnimationStateMachine, AnimationSystem, App, AtlasSprite,
    BlendEntry, BlendTree1D, BlendTreeSystem, BlendWeight, Camera, CharacterController, Collider,
    CollisionGridSystem, CollisionLayer, Color, DrawText, Entity, Events, HitFlash, HitFlashSystem,
    InputAction, InputBuffer, InputScript, InputState, KeyCode, ParallaxLayer, ParallaxSystem,
    PhysicsBody, PhysicsSystem, PhysicsWorld, ShouldQuit, SolidTiles, Sprite, SpriteFlip,
    StateMachineSystem, System, SystemConfig, TextQueue, Tilemap, TilemapAtlas, TilemapAutotile,
    TilemapColliders, TilemapSystem, Transform, TransitionCond, TriggerZone, TriggerZoneSystem,
    UvRect, WindowConfig, World, ZoneEvent, DEFAULT_COYOTE_SECS,
};
use glam::Vec2;
use rapier2d::na as nalgebra;
use rapier2d::prelude::vector;

// ── Window / scale ──────────────────────────────────────────────────────────────────────────────

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 540;
/// One tile is one physics unit: `TILE == PPU` keeps every conversion in this file a single scale.
const TILE: f32 = 32.0;
const PPU: f32 = 32.0;

// ── Tuning ──────────────────────────────────────────────────────────────────────────────────────
//
// These four numbers and the level geometry are one design, not two: check 5 of the selftest
// re-derives the jump arc from them and asserts it still clears the two climbs the route needs,
// still cannot clear the door, and still cannot cross the chasm.

const MOVE_ACCEL: f32 = 1400.0;
const GROUND_DECEL: f32 = 1800.0;
const AIR_DECEL: f32 = 520.0;
const MAX_SPEED_X: f32 = 190.0;
const GRAVITY: f32 = 1600.0;
const MAX_FALL_SPEED: f32 = 900.0;
const JUMP_SPEED: f32 = 610.0;
/// Downward speed at which the animation state machine calls it a fall (px/s).
const FALL_ANIM_VY: f32 = 60.0;
/// Horizontal speed above which the player animates as running (px/s).
const RUN_ANIM_VX: f32 = 15.0;

/// Peak height of a standing jump, in pixels.
const JUMP_APEX_PX: f32 = JUMP_SPEED * JUMP_SPEED / (2.0 * GRAVITY);
/// Horizontal distance covered by a full-speed jump, in pixels.
const JUMP_REACH_PX: f32 = 2.0 * JUMP_SPEED / GRAVITY * MAX_SPEED_X;

const PLAYER_HALF_PX: Vec2 = Vec2::new(11.0, 14.0);
const PLAYER_DRAW_PX: Vec2 = Vec2::new(38.0, 44.0);
/// One-way landing skin, in **physics units**. The engine default (0.05) is tuned for PPU ≈ 64;
/// this game runs at PPU 32, so the same visual skin needs twice the physics-unit value.
const ONE_WAY_TOLERANCE: f32 = 0.10;
/// Half-height of a one-way plank, in pixels (its top surface sits on its tile row's top edge).
const PLANK_HALF_H: f32 = 6.0;
const PLATFORM_HALF_PX: Vec2 = Vec2::new(48.0, 8.0);
const PLATFORM_SPEED: f32 = 70.0;
const WALKER_SPEED: f32 = 55.0;
const WALKER_HALF_PX: Vec2 = Vec2::new(14.0, 16.0);
const GRID_CELL: f32 = 64.0;

const LAYER_PLAYER: CollisionLayer = CollisionLayer(1 << 0);

const TILES_PATH: &str = "examples/platformer_game/assets/tiles.png";
const PLAYER_PATH: &str = "examples/platformer_game/assets/player.png";

// ── Level ───────────────────────────────────────────────────────────────────────────────────────
//
//   '#' terrain      'D' door tiles (terrain until the switch opens them)   '=' one-way plank
//   'P' spawn        'S' switch     'C' checkpoint   'W' walker   'G' goal
//   '-' moving-platform rail        '^' hazard
//
// The corridor under the shelf (rows 14-15, cols 18-25) is walled at both ends and capped by the
// shelf, so its only opening is the one-way plank in the ceiling: the switch is reached by dropping
// through it and left by jumping back up through it. That is the whole point of the shape.
const LEVEL: &[&str] = &[
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "................................................................",
    "..............................D.................................",
    "..............................D.................................",
    ".................####===###...D.................................",
    ".................#........#...D.................................",
    "..P.............##.S......#...D.C.....W.......--------....G.....",
    "############..################################........##########",
    "############..################################........##########",
    "############..################################........##########",
    "############^^################################^^^^^^^^##########",
];

/// Cell char → tile value. Terrain and door are the same terrain for autotiling purposes; the door
/// cells are the ones the switch sets back to `0`.
fn tile_value(c: char) -> u32 {
    match c {
        '#' | 'D' => 1,
        _ => 0,
    }
}

fn level_dims() -> (usize, usize) {
    (LEVEL.len(), LEVEL[0].len())
}

fn world_size() -> Vec2 {
    let (rows, cols) = level_dims();
    Vec2::new(cols as f32 * TILE, rows as f32 * TILE)
}

/// World position of a cell's centre — the same formula `Tilemap::cell_center_world` uses.
fn cell_center(row: usize, col: usize) -> Vec2 {
    Vec2::new(
        col as f32 * TILE + TILE * 0.5,
        row as f32 * TILE + TILE * 0.5,
    )
}

/// First cell holding `marker`, as `(row, col)`.
fn find_cell(marker: char) -> Option<(usize, usize)> {
    LEVEL.iter().enumerate().find_map(|(r, line)| {
        line.chars()
            .position(|c| c == marker)
            .map(|c_idx| (r, c_idx))
    })
}

/// Every maximal horizontal run of `marker`, as `(row, first_col, last_col)`.
fn find_runs(marker: char) -> Vec<(usize, usize, usize)> {
    let mut runs = Vec::new();
    for (r, line) in LEVEL.iter().enumerate() {
        let cells: Vec<char> = line.chars().collect();
        let mut c = 0;
        while c < cells.len() {
            if cells[c] == marker {
                let start = c;
                while c + 1 < cells.len() && cells[c + 1] == marker {
                    c += 1;
                }
                runs.push((r, start, c));
            }
            c += 1;
        }
    }
    runs
}

fn cells_with(marker: char) -> Vec<(usize, usize)> {
    LEVEL
        .iter()
        .enumerate()
        .flat_map(|(r, line)| {
            line.chars()
                .enumerate()
                .filter(move |(_, c)| *c == marker)
                .map(move |(c_idx, _)| (r, c_idx))
        })
        .collect()
}

/// Centre and half-extents (pixels) of a horizontal run of cells.
fn run_box(row: usize, c0: usize, c1: usize) -> (Vec2, Vec2) {
    let half = Vec2::new((c1 + 1 - c0) as f32 * TILE * 0.5, TILE * 0.5);
    let centre = Vec2::new(c0 as f32 * TILE + half.x, row as f32 * TILE + half.y);
    (centre, half)
}

/// Feet-on-the-floor position for an entity standing on top of `row`.
fn standing_on(row: usize, col: usize, half_h: f32) -> Vec2 {
    Vec2::new(cell_center(row, col).x, row as f32 * TILE - half_h)
}

// ── Session ─────────────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    Playing,
    Won,
}

#[derive(Clone, Copy)]
struct MovingPlatform {
    entity: Entity,
    body: engine::BodyHandle,
    min_x: f32,
    max_x: f32,
    dir: f32,
}

struct Session {
    player: Entity,
    map: Entity,
    camera_anchor: Entity,
    walker: Entity,
    switch_zone: Entity,
    checkpoint_zone: Entity,
    goal_zone: Entity,
    hazard_zones: Vec<Entity>,
    door_cells: Vec<(usize, usize)>,
    platform: Option<MovingPlatform>,
    /// How far the moving platform travelled this frame — added to a rider's movement.
    platform_delta: Vec2,
    status: Status,
    velocity: Vec2,
    jump: InputBuffer,
    facing_left: bool,
    spawn: Vec2,
    checkpoint: Vec2,
    door_open: bool,
    /// Monotonic: every jump the `InputBuffer` actually granted. Never reset by a respawn — the
    /// coyote check reads the difference across a few frames, so a reset would fake a pass.
    jumps_fired: u32,
    deaths: u32,
    walker_time: f32,
}

// ── Systems ─────────────────────────────────────────────────────────────────────────────────────

const L_PLATFORM: &str = "game::platform";
const L_PLAYER: &str = "game::player";
const L_WALKER: &str = "game::walker";
const L_ZONES: &str = "game::zones";
const L_RULES: &str = "game::rules";
const L_CAMERA: &str = "game::camera";

fn approach(current: f32, target: f32, max_delta: f32) -> f32 {
    if (target - current).abs() <= max_delta {
        target
    } else {
        current + (target - current).signum() * max_delta
    }
}

/// Drives the moving platform along its rail and records the distance it covered, which
/// [`PlayerSystem`] adds to a rider's movement.
///
/// The platform is a **dynamic** body held on its rail by a prismatic joint, and its vertical
/// velocity is deliberately left alone here: if the joint ever stopped constraining it, gravity
/// would take it out of the world instead of the joint silently becoming decoration.
struct PlatformSystem;

impl System for PlatformSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(mut plat) = world.resource::<Session>().and_then(|s| s.platform) else {
            return;
        };
        let x = world
            .get::<Transform>(plat.entity)
            .map(|t| t.position.x)
            .unwrap_or(plat.min_x);
        if x <= plat.min_x && plat.dir < 0.0 {
            plat.dir = 1.0;
        } else if x >= plat.max_x && plat.dir > 0.0 {
            plat.dir = -1.0;
        }

        let vx = plat.dir * PLATFORM_SPEED;
        let body = plat.body;
        world.with_resource_mut::<PhysicsWorld, _>(|physics, _| {
            if let Some(rb) = physics.rigid_body_mut(body) {
                let vy = rb.linvel().y;
                rb.set_linvel(vector![vx / PPU, vy], true);
            }
        });

        if let Some(session) = world.resource_mut::<Session>() {
            session.platform = Some(plat);
            session.platform_delta = Vec2::new(vx * dt, 0.0);
        }
    }
}

/// Reads input, runs the jump/gravity model through [`InputBuffer`], moves the character, and
/// hands the resulting physics state to the animation state machine.
struct PlayerSystem;

impl PlayerSystem {
    /// How far the moving platform should carry the player this frame (zero unless standing on it).
    fn ride(world: &World, player: Entity, grounded: bool) -> Vec2 {
        if !grounded {
            return Vec2::ZERO;
        }
        let Some(session) = world.resource::<Session>() else {
            return Vec2::ZERO;
        };
        let Some(plat) = session.platform else {
            return Vec2::ZERO;
        };
        let (Some(p), Some(q)) = (
            world.get::<Transform>(player),
            world.get::<Transform>(plat.entity),
        ) else {
            return Vec2::ZERO;
        };
        let feet = p.position.y + PLAYER_HALF_PX.y;
        let deck = q.position.y - PLATFORM_HALF_PX.y;
        let overlap_x =
            (p.position.x - q.position.x).abs() <= PLATFORM_HALF_PX.x + PLAYER_HALF_PX.x;
        if overlap_x && (feet - deck).abs() <= 6.0 {
            session.platform_delta
        } else {
            Vec2::ZERO
        }
    }
}

impl System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (axis, jump_pressed, drop_pressed, restart, quit) = world
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
                    input.just_pressed(KeyCode::Space) || input.just_pressed(KeyCode::KeyW),
                    input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown),
                    input.just_pressed(KeyCode::KeyR),
                    input.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((0.0, false, false, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if restart {
            respawn(world, true);
        }

        let Some((player, status)) = world.resource::<Session>().map(|s| (s.player, s.status))
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }

        let grounded = world
            .get::<CharacterController>(player)
            .map(|c| c.grounded)
            .unwrap_or(false);
        let ride = Self::ride(world, player, grounded);

        // Order matters and is the one the InputBuffer docs specify: ground state, press, consume,
        // tick last — so a press on the landing frame still jumps.
        let (desired, jumped, vy) = {
            let Some(s) = world.resource_mut::<Session>() else {
                return;
            };
            s.jump.set_grounded(grounded);
            if jump_pressed {
                s.jump.press();
            }
            let jumped = s.jump.try_consume();
            s.jump.tick(dt);

            if grounded && s.velocity.y > 0.0 {
                s.velocity.y = 0.0;
            }
            s.velocity.x = if axis != 0.0 {
                approach(s.velocity.x, axis * MAX_SPEED_X, MOVE_ACCEL * dt)
            } else {
                let decel = if grounded { GROUND_DECEL } else { AIR_DECEL };
                approach(s.velocity.x, 0.0, decel * dt)
            };
            if jumped {
                s.velocity.y = -JUMP_SPEED;
                s.jumps_fired += 1;
            } else {
                s.velocity.y = (s.velocity.y + GRAVITY * dt).min(MAX_FALL_SPEED);
            }
            if axis != 0.0 {
                s.facing_left = axis < 0.0;
            }
            (s.velocity * dt + ride, jumped, s.velocity.y)
        };

        if drop_pressed && grounded {
            if let Some(c) = world.get_mut::<CharacterController>(player) {
                c.request_drop();
            }
        }

        let handles = world
            .get::<PhysicsBody>(player)
            .map(|b| (b.rigid_body_handle, b.collider_handle));
        if let Some((rb, col)) = handles {
            world.with_resource_mut::<PhysicsWorld, _>(|physics, world| {
                if let Some(ctrl) = world.get_mut::<CharacterController>(player) {
                    physics.move_character(ctrl, rb, col, desired, dt, PPU);
                }
            });
        }

        let grounded_now = world
            .get::<CharacterController>(player)
            .map(|c| c.grounded)
            .unwrap_or(false);
        let running = {
            let Some(s) = world.resource_mut::<Session>() else {
                return;
            };
            if grounded_now && vy > 0.0 {
                s.velocity.y = 0.0;
            }
            axis != 0.0 && s.velocity.x.abs() > RUN_ANIM_VX
        };
        let facing_left = world
            .resource::<Session>()
            .map(|s| s.facing_left)
            .unwrap_or(false);

        if let Some(flip) = world.get_mut::<SpriteFlip>(player) {
            flip.x = facing_left;
        }
        if let Some(sm) = world.get_mut::<AnimationStateMachine>(player) {
            sm.set_bool("is_running", running);
            sm.set_bool("is_grounded", grounded_now);
            sm.set_float("vy", vy);
            if jumped {
                sm.fire_trigger("jump");
            }
        }
    }

    fn name(&self) -> &'static str {
        "PlayerSystem"
    }
}

/// Patrols the walker and feeds its speed to the blend tree. Physics-free on purpose: it is a
/// hazard with a `TriggerZone`, and giving it a body would only add a second thing to keep in sync.
struct WalkerSystem;

impl System for WalkerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some((walker, min_x, max_x)) = world.resource::<Session>().map(|s| {
            let (_, c0, c1) = walker_patrol();
            (
                s.walker,
                c0 as f32 * TILE + WALKER_HALF_PX.x,
                (c1 + 1) as f32 * TILE - WALKER_HALF_PX.x,
            )
        }) else {
            return;
        };

        let t = {
            let Some(s) = world.resource_mut::<Session>() else {
                return;
            };
            s.walker_time += dt;
            s.walker_time
        };
        // Speed swings between a saunter and a scurry so the blend tree actually crosses its
        // thresholds — a blend tree pinned to one clip proves nothing about blending.
        let speed = WALKER_SPEED * (0.25 + 0.95 * (t * 0.9).sin().abs());

        let Some(pos) = world.get::<Transform>(walker).map(|t| t.position) else {
            return;
        };
        let dir = world
            .get::<SpriteFlip>(walker)
            .map(|f| if f.x { -1.0 } else { 1.0 })
            .unwrap_or(1.0);
        let mut next_x = pos.x + dir * speed * dt;
        let mut next_dir = dir;
        if next_x <= min_x {
            next_x = min_x;
            next_dir = 1.0;
        } else if next_x >= max_x {
            next_x = max_x;
            next_dir = -1.0;
        }

        if let Some(t) = world.get_mut::<Transform>(walker) {
            t.position.x = next_x;
        }
        if let Some(flip) = world.get_mut::<SpriteFlip>(walker) {
            flip.x = next_dir < 0.0;
        }
        if let Some(tree) = world.get_mut::<BlendTree1D>(walker) {
            tree.set_param(speed);
        }
    }

    fn name(&self) -> &'static str {
        "WalkerSystem"
    }
}

/// Turns `ZoneEvent`s into the game's rules: the switch opens the door, the checkpoint moves the
/// respawn point, hazards and the walker hurt, the goal ends the run.
struct RulesSystem;

impl System for RulesSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, switch_zone, cp_zone, goal_zone, walker, hazards)) =
            world.resource::<Session>().map(|s| {
                (
                    s.player,
                    s.switch_zone,
                    s.checkpoint_zone,
                    s.goal_zone,
                    s.walker,
                    s.hazard_zones.clone(),
                )
            })
        else {
            return;
        };

        let (mut hit_switch, mut checkpoint, mut goal, mut hurt) = (false, false, false, false);
        if let Some(events) = world.resource::<Events<ZoneEvent>>() {
            for event in events.read() {
                let (zone, other) = match *event {
                    ZoneEvent::Entered { zone, other } => (zone, other),
                    // The walker moves onto a standing player as often as the reverse, so a hazard
                    // that only reacted to `Entered` would miss half the collisions.
                    ZoneEvent::Stayed { zone, other } if zone == walker => (zone, other),
                    _ => continue,
                };
                if other != player {
                    continue;
                }
                if zone == switch_zone {
                    hit_switch = true;
                } else if zone == cp_zone {
                    checkpoint = true;
                } else if zone == goal_zone {
                    goal = true;
                } else if zone == walker || hazards.contains(&zone) {
                    hurt = true;
                }
            }
        }

        if hit_switch {
            open_door(world);
        }
        if checkpoint {
            let flag = world.get::<Transform>(cp_zone).map(|t| t.position);
            if let (Some(pos), Some(s)) = (flag, world.resource_mut::<Session>()) {
                // Stand on the cell the flag sits in, not inside it.
                s.checkpoint = Vec2::new(pos.x, pos.y + TILE * 0.5 - PLAYER_HALF_PX.y);
            }
        }
        if goal {
            if let Some(s) = world.resource_mut::<Session>() {
                s.status = Status::Won;
                s.velocity = Vec2::ZERO;
            }
        }
        if hurt {
            if world.get::<HitFlash>(player).is_none() {
                world.add_component(player, HitFlash::new(Color::rgb(1.0, 0.35, 0.35), 0.3));
            }
            if let Some(s) = world.resource_mut::<Session>() {
                s.deaths += 1;
            }
            respawn(world, false);
        }

        // Backstop: the chasm has no floor, so a fall that outruns the hazard band still resets.
        let fell = world
            .resource::<Session>()
            .and_then(|s| world.get::<Transform>(s.player))
            .map(|t| t.position.y > world_size().y + 200.0)
            .unwrap_or(false);
        if fell {
            respawn(world, false);
        }
    }

    fn name(&self) -> &'static str {
        "RulesSystem"
    }
}

/// The camera follows this anchor rather than the player: `Camera::position` is the viewport's
/// top-left corner, so following the player directly would pin them to that corner.
struct CameraAnchorSystem;

impl System for CameraAnchorSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, anchor)) = world
            .resource::<Session>()
            .map(|s| (s.player, s.camera_anchor))
        else {
            return;
        };
        let Some(pos) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        if let Some(t) = world.get_mut::<Transform>(anchor) {
            t.position = pos - Vec2::new(WINDOW_W as f32, WINDOW_H as f32) * 0.5;
        }
    }

    fn name(&self) -> &'static str {
        "CameraAnchorSystem"
    }
}

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((status, deaths, door_open)) = world
            .resource::<Session>()
            .map(|s| (s.status, s.deaths, s.door_open))
        else {
            return;
        };
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        tq.push(DrawText::new(
            "A/D move   Space jump   S drop through   R restart   Esc quit",
            Vec2::new(16.0, 14.0),
            18.0,
            [225, 236, 250, 235],
        ));
        tq.push(DrawText::new(
            format!(
                "door: {}   falls: {deaths}",
                if door_open { "open" } else { "locked" }
            ),
            Vec2::new(16.0, 38.0),
            18.0,
            [170, 210, 255, 220],
        ));
        if status == Status::Won {
            tq.push(DrawText::new(
                "Goal reached — press R to run it again",
                Vec2::new(WINDOW_W as f32 * 0.5 - 190.0, WINDOW_H as f32 * 0.5),
                28.0,
                [130, 255, 175, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ── Shared game actions ─────────────────────────────────────────────────────────────────────────

fn walker_patrol() -> (usize, usize, usize) {
    // The walker owns the stretch of ground between the checkpoint and the chasm.
    let (row, col) = find_cell('W').expect("level has no walker marker");
    (row, col.saturating_sub(4), col + 6)
}

/// Moves a body-carrying entity, keeping the rapier body, the transform and the controller's
/// ground state in agreement. Used by respawn and by the selftest to set up a scenario.
fn teleport(world: &mut World, entity: Entity, pos: Vec2) {
    let body = world
        .get::<PhysicsBody>(entity)
        .map(|b| b.rigid_body_handle);
    if let Some(rb) = body {
        world.with_resource_mut::<PhysicsWorld, _>(|physics, _| {
            if let Some(b) = physics.rigid_body_mut(rb) {
                b.set_translation(vector![pos.x / PPU, pos.y / PPU], true);
                b.set_next_kinematic_translation(vector![pos.x / PPU, pos.y / PPU]);
            }
        });
    }
    if let Some(t) = world.get_mut::<Transform>(entity) {
        t.position = pos;
    }
    if let Some(c) = world.get_mut::<CharacterController>(entity) {
        c.grounded = false;
    }
}

fn respawn(world: &mut World, to_spawn: bool) {
    let Some((player, pos)) = world
        .resource::<Session>()
        .map(|s| (s.player, if to_spawn { s.spawn } else { s.checkpoint }))
    else {
        return;
    };
    teleport(world, player, pos);
    if let Some(s) = world.resource_mut::<Session>() {
        s.velocity = Vec2::ZERO;
        s.jump = InputBuffer::default();
        s.status = Status::Playing;
    }
}

/// The switch's effect: clear the door tiles and resync the tilemap's colliders.
///
/// Both halves matter. Clearing the tiles alone leaves the colliders behind — an invisible wall,
/// which is exactly the failure check 4 drives from both sides.
fn open_door(world: &mut World) {
    let Some((map, cells, already)) = world
        .resource::<Session>()
        .map(|s| (s.map, s.door_cells.clone(), s.door_open))
    else {
        return;
    };
    if already {
        return;
    }
    if let Some(tilemap) = world.get_mut::<Tilemap>(map) {
        for &(row, col) in &cells {
            tilemap.set_tile(row, col, 0);
        }
    }
    engine::sync_tilemap_entity_colliders(world, map);
    if let Some(s) = world.resource_mut::<Session>() {
        s.door_open = true;
    }
}

// ── Animation ───────────────────────────────────────────────────────────────────────────────────

const CLIP_IDLE: usize = 0;
const CLIP_RUN: usize = 1;
const CLIP_JUMP: usize = 2;
const CLIP_FALL: usize = 3;
const CLIP_LAND: usize = 4;

fn row_frames(row: u32, count: u32) -> Vec<UvRect> {
    (0..count)
        .map(|c| UvRect::from_grid(c, row, 4, 4))
        .collect()
}

fn player_clips() -> AnimationPlayer {
    AnimationPlayer::new(vec![
        AnimationClip {
            frames: row_frames(0, 4),
            fps: 4.0,
            looping: true,
        },
        AnimationClip {
            frames: row_frames(1, 4),
            fps: 11.0,
            looping: true,
        },
        AnimationClip {
            frames: row_frames(2, 4),
            fps: 9.0,
            looping: false,
        },
        AnimationClip {
            frames: row_frames(3, 4),
            fps: 8.0,
            looping: true,
        },
        // Landing squash: the tail of the fall sheet, played once. Its end is what returns the
        // state machine to idle/run, via `TransitionCond::AnimationEnd`.
        AnimationClip {
            frames: row_frames(3, 2),
            fps: 14.0,
            looping: false,
        },
    ])
}

/// Every airborne return routes through `land`, so "did the player land" is a state the machine
/// visits rather than something a reader has to infer from `grounded` flipping.
fn player_state_machine() -> AnimationStateMachine {
    let mut sm = AnimationStateMachine::new("idle", CLIP_IDLE);
    sm.add_state("run", CLIP_RUN)
        .add_state("jump", CLIP_JUMP)
        .add_state("fall", CLIP_FALL)
        .add_state("land", CLIP_LAND);
    sm.set_bool("is_running", false);
    sm.set_bool("is_grounded", false);
    sm.set_float("vy", 0.0);
    sm.add_trigger("jump");

    let grounded = |v: bool| TransitionCond::BoolEq("is_grounded".into(), v);
    let running = |v: bool| TransitionCond::BoolEq("is_running".into(), v);
    let falling = || TransitionCond::FloatGt("vy".into(), FALL_ANIM_VY);

    sm.add_transition("idle", "run", vec![grounded(true), running(true)])
        .add_transition("run", "idle", vec![grounded(true), running(false)])
        .add_transition("idle", "jump", vec![TransitionCond::Trigger("jump".into())])
        .add_transition("run", "jump", vec![TransitionCond::Trigger("jump".into())])
        .add_transition("idle", "fall", vec![grounded(false), falling()])
        .add_transition("run", "fall", vec![grounded(false), falling()])
        .add_transition("jump", "fall", vec![falling()])
        .add_transition("jump", "land", vec![grounded(true)])
        .add_transition("fall", "land", vec![grounded(true)])
        .add_transition(
            "land",
            "run",
            vec![TransitionCond::AnimationEnd, running(true)],
        )
        .add_transition(
            "land",
            "idle",
            vec![TransitionCond::AnimationEnd, running(false)],
        );
    sm
}

fn walker_blend_tree() -> BlendTree1D {
    BlendTree1D::new(
        vec![
            BlendEntry {
                threshold: 0.0,
                clip_index: CLIP_IDLE,
            },
            BlendEntry {
                threshold: WALKER_SPEED * 0.5,
                clip_index: CLIP_RUN,
            },
            BlendEntry {
                threshold: WALKER_SPEED * 0.95,
                clip_index: CLIP_FALL,
            },
        ],
        0.18,
    )
}

// ── Setup ───────────────────────────────────────────────────────────────────────────────────────

/// Builds the whole game. The selftest drives *this*, not a reduced copy of it — a check that runs
/// against a hand-built world proves things about the world the check built.
fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — platformer_game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.06, 0.09, 0.15, 1.0],
    });
    app.register_event::<ZoneEvent>();

    let player_image = app.load_image(PLAYER_PATH);
    // The walker borrows the player sheet, tinted: this game is about the systems under it, and a
    // second sprite sheet would be a second thing to keep in step with the clip indices.
    let walker_atlas = app.load_atlas(PLAYER_PATH, 4, 4);
    // Loaded so the renderer can resolve the path the tilemap's tile sprites carry.
    app.load_atlas(TILES_PATH, 4, 4);

    let mut physics = PhysicsWorld::new(Vec2::new(0.0, GRAVITY / PPU));

    // ── Terrain ────────────────────────────────────────────────────────────────────────────────
    let tiles: Vec<Vec<u32>> = LEVEL
        .iter()
        .map(|row| row.chars().map(tile_value).collect())
        .collect();
    let tilemap = Tilemap::new(TilemapAtlas::new(TILES_PATH, 4, 4), tiles, TILE, Vec2::ZERO);
    let map = app.world.spawn();
    app.world.add_component(map, tilemap);
    app.world
        .add_component(map, TilemapAutotile::edge_16(0).with_oob_filled(false));
    app.world
        .add_component(map, TilemapColliders::new(PPU, SolidTiles::NonZero));

    // ── One-way planks ─────────────────────────────────────────────────────────────────────────
    //
    // Not tiles. `TilemapColliders` derives its colliders from `SolidTiles`, which can only say
    // "solid" — there is no way to spell `TileCollider::one_way()` through it — so the planks are
    // their own static bodies and the tilemap owns exactly the cells it can describe.
    for (row, c0, c1) in find_runs('=') {
        let half = Vec2::new((c1 + 1 - c0) as f32 * TILE * 0.5, PLANK_HALF_H);
        let centre = Vec2::new(c0 as f32 * TILE + half.x, row as f32 * TILE + half.y);
        let (_, col) = physics.add_static_box(centre / PPU, half.x / PPU, half.y / PPU);
        physics.set_one_way(col, true);

        let plank = app.world.spawn();
        app.world.add_component(
            plank,
            Transform {
                position: centre,
                scale: half * 2.0,
                z: 0.1,
                ..Default::default()
            },
        );
        app.world
            .add_component(plank, Sprite::colored(0.72, 0.51, 0.28));
    }

    // ── Moving platform on a prismatic rail ────────────────────────────────────────────────────
    let platform = find_runs('-').first().copied().map(|(row, c0, c1)| {
        let deck_y = row as f32 * TILE + PLATFORM_HALF_PX.y;
        let min_x = c0 as f32 * TILE + PLATFORM_HALF_PX.x;
        let max_x = (c1 + 1) as f32 * TILE - PLATFORM_HALF_PX.x;
        let centre = Vec2::new(min_x, deck_y);

        let (body, body_col) = physics.add_dynamic_box(
            centre / PPU,
            PLATFORM_HALF_PX.x / PPU,
            PLATFORM_HALF_PX.y / PPU,
            true,
        );
        // The rail: a fixed body parked far outside the level (the joint's axis is X, so its x is
        // free) whose Y is what the joint holds the platform to.
        let (anchor, _) = physics.add_static_box(Vec2::new(-200.0, deck_y) / PPU, 0.05, 0.05);
        physics.add_prismatic_joint(anchor, body, Vec2::ZERO, Vec2::ZERO, Vec2::X);

        let entity = app.world.spawn();
        app.world.add_component(
            entity,
            Transform {
                position: centre,
                scale: PLATFORM_HALF_PX * 2.0,
                z: 0.1,
                ..Default::default()
            },
        );
        app.world
            .add_component(entity, Sprite::colored(0.55, 0.62, 0.78));
        app.world.add_component(
            entity,
            PhysicsBody {
                rigid_body_handle: body,
                collider_handle: body_col,
            },
        );
        MovingPlatform {
            entity,
            body,
            min_x,
            max_x,
            dir: 1.0,
        }
    });

    // ── Player ─────────────────────────────────────────────────────────────────────────────────
    let (spawn_row, spawn_col) = find_cell('P').expect("level has no spawn marker");
    let spawn = standing_on(spawn_row + 1, spawn_col, PLAYER_HALF_PX.y);
    let (player_body, player_col) =
        physics.add_kinematic_box(spawn / PPU, PLAYER_HALF_PX.x / PPU, PLAYER_HALF_PX.y / PPU);
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: spawn,
            scale: PLAYER_DRAW_PX,
            z: 0.4,
            ..Default::default()
        },
    );
    app.world.add_component(
        player,
        Sprite::textured_with_handle(PLAYER_PATH, Some(player_image)),
    );
    app.world.add_component(player, SpriteFlip::default());
    app.world.add_component(
        player,
        PhysicsBody {
            rigid_body_handle: player_body,
            collider_handle: player_col,
        },
    );
    app.world.add_component(
        player,
        CharacterController::new().with_one_way_tolerance(ONE_WAY_TOLERANCE),
    );
    app.world.add_component(player, player_clips());
    app.world.add_component(player, player_state_machine());
    app.world.add_component(
        player,
        Collider::Aabb {
            half_extents: PLAYER_HALF_PX,
        },
    );
    app.world.add_component(player, LAYER_PLAYER);

    // ── Walker ─────────────────────────────────────────────────────────────────────────────────
    let (walker_row, walker_col) = find_cell('W').expect("level has no walker marker");
    let walker_pos = standing_on(walker_row + 1, walker_col, WALKER_HALF_PX.y);
    let walker = app.world.spawn();
    app.world.add_component(
        walker,
        Transform {
            position: walker_pos,
            scale: WALKER_HALF_PX * 2.0,
            z: 0.3,
            ..Default::default()
        },
    );
    app.world.add_component(
        walker,
        AtlasSprite::new(walker_atlas, 0).with_color(Color::rgb(1.0, 0.55, 0.45)),
    );
    app.world.add_component(walker, SpriteFlip::default());
    app.world.add_component(walker, player_clips());
    app.world.add_component(walker, walker_blend_tree());
    app.world.add_component(
        walker,
        TriggerZone::rect(WALKER_HALF_PX).with_mask(LAYER_PLAYER),
    );

    // ── Zones ──────────────────────────────────────────────────────────────────────────────────
    let zone = |app: &mut App, row: usize, col: usize, tint: [f32; 3]| -> Entity {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: cell_center(row, col),
                scale: Vec2::splat(TILE * 0.75),
                z: 0.2,
                ..Default::default()
            },
        );
        app.world
            .add_component(e, Sprite::colored(tint[0], tint[1], tint[2]));
        app.world.add_component(
            e,
            TriggerZone::rect(Vec2::splat(TILE * 0.5)).with_mask(LAYER_PLAYER),
        );
        e
    };
    let (sr, sc) = find_cell('S').expect("level has no switch marker");
    let switch_zone = zone(&mut app, sr, sc, [0.95, 0.82, 0.25]);
    let (cr, cc) = find_cell('C').expect("level has no checkpoint marker");
    let checkpoint_zone = zone(&mut app, cr, cc, [0.35, 0.85, 0.95]);
    let (gr, gc) = find_cell('G').expect("level has no goal marker");
    let goal_zone = zone(&mut app, gr, gc, [0.4, 0.95, 0.55]);

    let mut hazard_zones = Vec::new();
    for (row, c0, c1) in find_runs('^') {
        let (centre, half) = run_box(row, c0, c1);
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: centre,
                scale: half * 2.0,
                z: 0.15,
                ..Default::default()
            },
        );
        app.world.add_component(e, Sprite::colored(0.75, 0.22, 0.3));
        app.world
            .add_component(e, TriggerZone::rect(half).with_mask(LAYER_PLAYER));
        hazard_zones.push(e);
    }

    // ── Parallax backdrop ──────────────────────────────────────────────────────────────────────
    // Two rows of distant blocks, standing on a baseline the terrain hides. The spread has to cover
    // the screen *plus* the distance the layer lags the camera across the level, or the far row
    // runs out mid-level: at `factor`, a camera that travels `d` leaves the layer `d * (1 - factor)`
    // behind.
    let camera_travel = world_size().x - WINDOW_W as f32;
    for (i, &(factor, gap, w, tall, tint)) in [
        (0.25_f32, 430.0_f32, 190.0_f32, 260.0_f32, 0.105_f32),
        (0.55, 360.0, 130.0, 170.0, 0.145),
    ]
    .iter()
    .enumerate()
    {
        let needed = WINDOW_W as f32 + camera_travel * (1.0 - factor);
        let count = (needed / gap).ceil() as usize + 1;
        for step in 0..count {
            // Deterministic height variation, so the row reads as a skyline rather than a wall.
            let h = tall * (0.45 + 0.55 * ((step * 7 + i * 3) % 5) as f32 / 4.0);
            let e = app.world.spawn();
            app.world.add_component(
                e,
                Transform {
                    position: Vec2::new(
                        120.0 + step as f32 * gap + i as f32 * 150.0,
                        560.0 - h * 0.5,
                    ),
                    scale: Vec2::new(w, h),
                    z: -2.0 + i as f32 * 0.1,
                    ..Default::default()
                },
            );
            app.world
                .add_component(e, Sprite::colored(tint * 0.7, tint, tint * 1.4));
            app.world
                .add_component(e, ParallaxLayer::horizontal(factor));
        }
    }

    // ── Camera ─────────────────────────────────────────────────────────────────────────────────
    let camera_anchor = app.world.spawn();
    app.world.add_component(
        camera_anchor,
        Transform {
            position: spawn - Vec2::new(WINDOW_W as f32, WINDOW_H as f32) * 0.5,
            scale: Vec2::ZERO,
            ..Default::default()
        },
    );
    let mut camera = Camera::new(Vec2::ZERO, 1.0);
    camera.follow_entity = Some(camera_anchor);
    camera.lerp_factor = 6.0;
    camera.lookahead = 130.0;
    camera.bounds = Some((Vec2::ZERO, world_size()));
    app.world.insert_resource(camera);

    app.world.insert_resource(Session {
        player,
        map,
        camera_anchor,
        walker,
        switch_zone,
        checkpoint_zone,
        goal_zone,
        hazard_zones,
        door_cells: cells_with('D'),
        platform,
        platform_delta: Vec2::ZERO,
        status: Status::Playing,
        velocity: Vec2::ZERO,
        jump: InputBuffer::default(),
        facing_left: false,
        spawn,
        checkpoint: spawn,
        door_open: false,
        jumps_fired: 0,
        deaths: 0,
        walker_time: 0.0,
    });
    app.world.insert_resource(physics);
    // The initial collider build goes through the same resync path the switch uses, so there is one
    // owner of every tile collider and the door's removal is an incremental diff rather than a
    // rebuild that could quietly double up.
    engine::sync_tilemap_entity_colliders(&mut app.world, map);

    // ── Schedule ───────────────────────────────────────────────────────────────────────────────
    //
    // Spelled out rather than left to insertion order: the player must move before physics steps,
    // zones need the grid rebuilt from post-step transforms, and the state machine reads frame
    // state the animation tick produces.
    app.add_system_labeled(PlatformSystem, SystemConfig::new().label(L_PLATFORM));
    app.add_system_labeled(
        PlayerSystem,
        SystemConfig::new().label(L_PLAYER).after(L_PLATFORM),
    );
    app.add_system_labeled(WalkerSystem, SystemConfig::new().label(L_WALKER));
    app.add_system_labeled(
        PhysicsSystem::new(PPU),
        SystemConfig::new()
            .label(PhysicsSystem::LABEL)
            .after(L_PLAYER)
            .after(L_WALKER),
    );
    app.add_system_labeled(
        CollisionGridSystem::new(GRID_CELL),
        SystemConfig::new()
            .label(CollisionGridSystem::LABEL)
            .after(PhysicsSystem::LABEL),
    );
    app.add_system_labeled(
        TriggerZoneSystem::default(),
        SystemConfig::new()
            .label(L_ZONES)
            .after(CollisionGridSystem::LABEL),
    );
    app.add_system_labeled(
        RulesSystem,
        SystemConfig::new().label(L_RULES).after(L_ZONES),
    );
    app.add_system_labeled(
        TilemapSystem::new(),
        SystemConfig::new()
            .label(TilemapSystem::LABEL)
            .after(L_RULES),
    );
    app.add_system_labeled(
        CameraAnchorSystem,
        SystemConfig::new()
            .label(L_CAMERA)
            .after(PhysicsSystem::LABEL),
    );
    app.add_system_labeled(
        ParallaxSystem,
        SystemConfig::new().label("game::parallax").after(L_CAMERA),
    );
    app.add_system_labeled(
        BlendTreeSystem::new(),
        SystemConfig::new()
            .label(BlendTreeSystem::LABEL)
            .before(AnimationSystem::LABEL),
    );
    app.add_system_labeled(
        AnimationSystem::new(),
        SystemConfig::new().label(AnimationSystem::LABEL),
    );
    app.add_system_labeled(
        StateMachineSystem::new(),
        SystemConfig::new()
            .label(StateMachineSystem::LABEL)
            .after(AnimationSystem::LABEL),
    );
    app.add_system(HitFlashSystem);
    app.add_system(HudSystem);
    app
}

fn main() {
    if std::env::var("PLATFORMER_GEN_ASSETS").is_ok() {
        generate_tileset(TILES_PATH);
        return;
    }
    if std::env::var("PLATFORMER_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    build_app().run();
}

// ── Acceptance test ─────────────────────────────────────────────────────────────────────────────
//
// `PLATFORMER_SELFTEST=1 cargo run --example platformer_game`, and `scripts/selftests.sh` in the
// gate. Five checks, each driving the **real** app through `App::step_headless` — no window, no
// GPU, no hand-built stand-in world.
//
// Input comes from an `InputScript`: `InputState` has no public press setter, so a scripted key is
// the only way to exercise the code path a real key takes. Positions are set with the same
// `teleport` the respawn uses, which is setup rather than input.
//
// Exit codes: 0 pass · 1 coyote time is not the window it claims · 2 the state machine does not
// pass through `fall` before `land` · 3 the one-way plank does not pass a drop down or a jump up
// · 4 the door does not block before the switch or does not open after it · 5 the jump arc no
// longer matches the level the route was built around · 6 the rail does not hold the moving
// platform up or the platform does not carry its rider · 7 the walker's blend tree never blends.

const DT: f32 = 1.0 / 60.0;
/// Airborne frames the coyote window is supposed to last.
const COYOTE_FRAMES: u32 = (DEFAULT_COYOTE_SECS / DT + 0.5) as u32;

fn session(app: &App) -> &Session {
    app.world
        .resource::<Session>()
        .expect("Session resource missing")
}

fn player_pos(app: &App) -> Vec2 {
    let p = session(app).player;
    app.world
        .get::<Transform>(p)
        .map(|t| t.position)
        .unwrap_or(Vec2::ZERO)
}

fn feet_y(app: &App) -> f32 {
    player_pos(app).y + PLAYER_HALF_PX.y
}

fn grounded(app: &App) -> bool {
    let p = session(app).player;
    app.world
        .get::<CharacterController>(p)
        .map(|c| c.grounded)
        .unwrap_or(false)
}

fn sm_state(app: &App) -> String {
    let p = session(app).player;
    app.world
        .get::<AnimationStateMachine>(p)
        .map(|sm| sm.current_state().to_string())
        .unwrap_or_default()
}

fn place_player(app: &mut App, pos: Vec2) {
    let p = session(app).player;
    teleport(&mut app.world, p, pos);
    if let Some(s) = app.world.resource_mut::<Session>() {
        s.velocity = Vec2::ZERO;
        s.jump = InputBuffer::default();
    }
}

/// Topmost terrain row in `col` — the surface something standing there rests on.
fn surface_row(col: usize) -> usize {
    LEVEL
        .iter()
        .position(|line| line.as_bytes()[col] == b'#')
        .expect("column has no terrain")
}

/// Frame on which the player, walking right off the spawn ledge, first leaves the ground.
fn first_airborne_frame() -> Option<u32> {
    let mut app = build_app();
    app.set_input_script(InputScript::new([(
        0,
        InputAction::KeyDown(KeyCode::ArrowRight),
    )]));
    let mut settled = false;
    for frame in 0..600u32 {
        app.step_headless(DT);
        let on_ground = grounded(&app);
        if !settled {
            settled = on_ground;
            continue;
        }
        if !on_ground {
            return Some(frame);
        }
    }
    None
}

/// Walks off the same ledge and taps jump on `press_frame`; reports whether a jump was granted.
fn coyote_jump_granted(press_frame: u32) -> bool {
    let mut app = build_app();
    app.set_input_script(InputScript::new([
        (0, InputAction::KeyDown(KeyCode::ArrowRight)),
        (press_frame, InputAction::KeyPress(KeyCode::Space)),
    ]));
    for _ in 0..press_frame + 6 {
        app.step_headless(DT);
    }
    session(&app).jumps_fired > 0
}

fn self_test() -> i32 {
    // ── 1. Coyote time is the window it advertises, from both sides ────────────────────────────
    //
    // Only the success side is the check people write, and it passes on an engine with infinite
    // coyote time. The airborne frame is measured rather than assumed, because it depends on where
    // the ledge is and how fast the player accelerates.
    let Some(leave) = first_airborne_frame() else {
        eprintln!(
            "FAIL: the player never walked off the spawn ledge — the level or the run speed changed"
        );
        return 1;
    };
    // The buffer is fed `grounded == false` for the first time on the frame after the move that
    // left the ground, so the window's first airborne tick is `leave + 1`.
    let inside = leave + 1 + (COYOTE_FRAMES - 1);
    let outside = leave + 1 + (COYOTE_FRAMES + 1);
    let fires_inside = coyote_jump_granted(inside);
    let fires_outside = coyote_jump_granted(outside);
    // The exact boundary frame is deliberately not asserted: `coyote_secs` is consumed by repeated
    // float subtraction, so the last tick lands on a value that is zero only up to rounding.
    if !fires_inside || fires_outside {
        eprintln!(
            "FAIL: coyote window is not {COYOTE_FRAMES} frames — a press {} airborne frames after \
             the ledge {} (want fire), {} frames after {} (want no fire)",
            COYOTE_FRAMES - 1,
            if fires_inside {
                "fired"
            } else {
                "did NOT fire"
            },
            COYOTE_FRAMES + 1,
            if fires_outside {
                "fired"
            } else {
                "did not fire"
            },
        );
        return 1;
    }
    println!(
        "ok: coyote time spans {COYOTE_FRAMES} frames — a jump {} airborne frames off the ledge \
         fires, {} frames off does not (ledge left on frame {leave})",
        COYOTE_FRAMES - 1,
        COYOTE_FRAMES + 1
    );

    // ── 2. The state machine passes through `fall` before `land` ───────────────────────────────
    //
    // Asserts the transition ORDER, not the final state: the player is idle at the end of a fall
    // either way, so a machine that skipped straight from `idle` to `land` — or never left `idle`
    // because its parameters stopped being fed from physics — photographs identically.
    {
        let mut app = build_app();
        let drop_col = 8;
        let surface = surface_row(drop_col) as f32 * TILE;
        place_player(
            &mut app,
            Vec2::new(
                cell_center(0, drop_col).x,
                surface - PLAYER_HALF_PX.y - 100.0,
            ),
        );
        let mut trace: Vec<String> = Vec::new();
        for _ in 0..240 {
            app.step_headless(DT);
            let state = sm_state(&app);
            if trace.last() != Some(&state) {
                trace.push(state);
            }
        }
        let fall_at = trace.iter().position(|s| s == "fall");
        let land_at = trace.iter().position(|s| s == "land");
        let ordered = matches!((fall_at, land_at), (Some(f), Some(l)) if f < l);
        if !ordered || !trace.iter().any(|s| s == "idle") {
            eprintln!(
                "FAIL: a 100 px drop did not animate fall -> land -> idle; states visited: {trace:?}"
            );
            return 2;
        }
        println!("ok: a drop animates in order — states visited: {trace:?}");
    }

    // ── 3. The one-way plank passes a drop down and a jump up ──────────────────────────────────
    //
    // Both directions, because a plank that fails one way still looks right the other. Measured
    // while sabotage-checking this: the engine's `!moving_down` clause is NOT what lets the rise
    // through at this geometry — the position test alone already passes a character whose feet are
    // below the plank — so only a check that actually rises from underneath covers it.
    {
        let (plank_row, c0, c1) = *find_runs('=').first().expect("level has no one-way plank");
        let plank_top = plank_row as f32 * TILE;
        let plank_bottom = plank_top + PLANK_HALF_H * 2.0;
        let x = (c0 + c1 + 1) as f32 * TILE * 0.5;

        let mut app = build_app();
        app.set_input_script(InputScript::new([
            (20, InputAction::KeyPress(KeyCode::ArrowDown)),
            (75, InputAction::KeyPress(KeyCode::Space)),
        ]));
        place_player(&mut app, Vec2::new(x, plank_top - PLAYER_HALF_PX.y - 2.0));

        let (mut lowest, mut highest_after_jump) = (f32::MIN, f32::MAX);
        for frame in 0..170 {
            app.step_headless(DT);
            lowest = lowest.max(feet_y(&app));
            if frame > 75 {
                highest_after_jump = highest_after_jump.min(feet_y(&app));
            }
        }
        let dropped_through = lowest > plank_bottom + 8.0;
        let jumped_through = highest_after_jump < plank_top - 4.0;
        let back_on_top = grounded(&app) && (feet_y(&app) - plank_top).abs() < 4.0;
        if !dropped_through || !jumped_through || !back_on_top {
            eprintln!(
                "FAIL: the one-way plank at y {plank_top} did not pass both ways — feet reached \
                 {lowest:.1} below it (want > {:.1}), {highest_after_jump:.1} above it (want < \
                 {:.1}), and ended at {:.1} grounded={} (want {plank_top} standing)",
                plank_bottom + 8.0,
                plank_top - 4.0,
                feet_y(&app),
                grounded(&app),
            );
            return 3;
        }
        println!(
            "ok: the one-way plank passes both ways — dropped to {lowest:.1}, jumped back to \
             {highest_after_jump:.1}, landed on it at {:.1}",
            feet_y(&app)
        );
    }

    // ── 4. The door blocks before the switch and is gone after it ──────────────────────────────
    //
    // The half that matters is the second one. Clearing the tiles without resyncing the colliders
    // leaves an invisible wall: the door LOOKS open in every screenshot and the player still cannot
    // walk through it, so only a check that walks through catches it.
    {
        let door_cells = cells_with('D');
        let door_col = door_cells[0].1;
        let door_left = door_col as f32 * TILE;
        let approach_col = door_col - 1;
        let stand = standing_on(surface_row(approach_col), approach_col, PLAYER_HALF_PX.y);

        let mut app = build_app();
        app.set_input_script(InputScript::new([(
            0,
            InputAction::KeyDown(KeyCode::ArrowRight),
        )]));
        place_player(&mut app, stand);
        let colliders_before = app
            .world
            .get::<TilemapColliders>(session(&app).map)
            .map(|t| t.collider_count())
            .unwrap_or(0);

        let mut furthest = f32::MIN;
        for _ in 0..90 {
            app.step_headless(DT);
            furthest = furthest.max(player_pos(&app).x + PLAYER_HALF_PX.x);
        }
        if furthest > door_left + 1.0 {
            eprintln!(
                "FAIL: the closed door did not block — the player reached x {furthest:.1}, past the \
                 door face at {door_left}"
            );
            return 4;
        }

        // Hit the switch through the real rules path, then walk the same approach again.
        let (switch_row, switch_col) = find_cell('S').expect("level has no switch");
        place_player(
            &mut app,
            standing_on(switch_row + 1, switch_col, PLAYER_HALF_PX.y),
        );
        for _ in 0..8 {
            app.step_headless(DT);
        }
        let colliders_after = app
            .world
            .get::<TilemapColliders>(session(&app).map)
            .map(|t| t.collider_count())
            .unwrap_or(0);
        let removed = colliders_before.saturating_sub(colliders_after);
        if !session(&app).door_open || removed != door_cells.len() {
            eprintln!(
                "FAIL: the switch did not open the door — open={}, tile colliders \
                 {colliders_before} -> {colliders_after} ({removed} removed, wanted {})",
                session(&app).door_open,
                door_cells.len()
            );
            return 4;
        }

        place_player(&mut app, stand);
        let mut passed = false;
        for _ in 0..120 {
            app.step_headless(DT);
            if player_pos(&app).x > door_left + TILE {
                passed = true;
                break;
            }
        }
        if !passed {
            eprintln!(
                "FAIL: the door's tiles are gone but the player still cannot walk through — stopped \
                 at x {:.1}, door face at {door_left}. That is the collider resync, not the tiles.",
                player_pos(&app).x
            );
            return 4;
        }
        println!(
            "ok: the door blocks closed, and the switch removes exactly its {} tile colliders and \
             lets the player through",
            door_cells.len()
        );
    }

    // ── 5. The jump arc still fits the level it was tuned against ──────────────────────────────
    //
    // Level geometry and the four movement constants are one design. Each of these three has a
    // route consequence: too short a jump seals the switch corridor, too tall a one makes the door
    // pointless, too long a one makes the moving platform (and its joint) decoration.
    {
        let (plank_row, _, _) = *find_runs('=').first().expect("level has no one-way plank");
        let corridor_floor = surface_row(22) as f32 * TILE;
        let climb = corridor_floor - plank_row as f32 * TILE;
        let door_height = cells_with('D').len() as f32 * TILE;
        let chasm = find_runs('-')
            .iter()
            .map(|&(_, c0, c1)| (c1 + 1 - c0) as f32 * TILE)
            .fold(0.0_f32, f32::max);

        if JUMP_APEX_PX < climb + 8.0 || JUMP_APEX_PX >= door_height || JUMP_REACH_PX >= chasm {
            eprintln!(
                "FAIL: the jump arc no longer fits the level — apex {JUMP_APEX_PX:.1} px against a \
                 {climb} px climb (needs > {:.1}) and a {door_height} px door (must stay under it); \
                 reach {JUMP_REACH_PX:.1} px against a {chasm} px chasm (must stay under it)",
                climb + 8.0
            );
            return 5;
        }
        println!(
            "ok: the jump arc fits — apex {JUMP_APEX_PX:.1} px clears the {climb} px climb, stays \
             under the {door_height} px door; reach {JUMP_REACH_PX:.1} px stays inside the {chasm} \
             px chasm"
        );
    }

    // ── 6. The rail holds the platform up, and the platform carries the player ─────────────────
    //
    // The prismatic joint is the only thing between a dynamic body and the world's gravity, so a
    // joint that stopped constraining shows up as a platform that is simply gone — and the chasm
    // becomes uncrossable without a single error anywhere. Carrying the rider is the other half:
    // a platform the player slides off is a platform that does not work.
    {
        let mut app = build_app();
        let plat = session(&app)
            .platform
            .expect("level has no moving-platform rail");
        let deck = app
            .world
            .get::<Transform>(plat.entity)
            .map(|t| t.position)
            .expect("the platform has no transform");
        place_player(
            &mut app,
            Vec2::new(deck.x, deck.y - PLATFORM_HALF_PX.y - PLAYER_HALF_PX.y - 2.0),
        );
        // Settle first: the drop onto the deck is airborne, and an airborne frame is not carried.
        for _ in 0..10 {
            app.step_headless(DT);
        }
        let deck_settled = app
            .world
            .get::<Transform>(plat.entity)
            .map(|t| t.position)
            .unwrap_or(deck);
        let player_x0 = player_pos(&app).x;
        for _ in 0..60 {
            app.step_headless(DT);
        }
        let deck_now = app
            .world
            .get::<Transform>(plat.entity)
            .map(|t| t.position)
            .unwrap_or(deck);
        let travelled = deck_now.x - deck_settled.x;
        let carried = player_pos(&app).x - player_x0;
        let sagged = (deck_now.y - deck_settled.y).abs();
        if travelled < 30.0 || sagged > 1.0 || (carried - travelled).abs() > 6.0 {
            eprintln!(
                "FAIL: the moving platform did not carry its rider along the rail — the deck moved \
                 {travelled:.1} px across and {sagged:.2} px down (want > 30 across, none down), \
                 the player {carried:.1} px"
            );
            return 6;
        }
        println!(
            "ok: the rail holds the platform ({sagged:.2} px of sag over a second) and it carries \
             the player {carried:.1} px against its own {travelled:.1}"
        );
    }

    // ── 7. The walker's blend tree crosses clips rather than sitting on one ────────────────────
    //
    // A blend tree pinned to one clip animates fine and proves nothing. This watches the walker
    // over a full swing of its speed and wants both a clip change and a frame mid-crossfade —
    // `BlendWeight` reads 1.0 whenever no transition is in flight.
    {
        let mut app = build_app();
        let walker = session(&app).walker;
        let mut clips = std::collections::BTreeSet::new();
        let mut min_weight = f32::MAX;
        // Distinct clips alone is not enough: a tree pinned to one parameter still switches ONCE,
        // off the player's starting clip, and reads as two clips and a crossfade. Measured — that
        // sabotage passed a `clips.len() >= 2` check. Count the switches instead.
        let (mut switches, mut prev) = (0usize, None);
        for _ in 0..600 {
            app.step_headless(DT);
            if let Some(player) = app.world.get::<AnimationPlayer>(walker) {
                clips.insert(player.current_clip);
                if prev.is_some_and(|p| p != player.current_clip) {
                    switches += 1;
                }
                prev = Some(player.current_clip);
            }
            if let Some(w) = app.world.get::<BlendWeight>(walker) {
                min_weight = min_weight.min(w.0);
            }
        }
        if clips.len() < 3 || switches < 4 || min_weight >= 1.0 {
            eprintln!(
                "FAIL: the walker's blend tree did not blend — {} clip(s) over 10 s ({clips:?}) \
                 across {switches} switches (want 3 clips, 4+ switches), lowest blend weight \
                 {min_weight:.3} (1.0 means no crossfade ever ran)",
                clips.len()
            );
            return 7;
        }
        println!(
            "ok: the walker's blend tree crossed {} clips {clips:?} in {switches} switches, \
             dipping to weight {min_weight:.3} mid-fade",
            clips.len()
        );
    }

    0
}

// ── Asset generation ────────────────────────────────────────────────────────────────────────────

/// Writes `assets/tiles.png`: a 4×4 sheet whose **cell index is the edge-4 neighbour mask**
/// (N=1, E=2, S=4, W=8), which is the layout `TilemapAutotile::edge_16(0)` indexes into. A cell's
/// open sides — the bits that are *clear* — get the lit rim; an open top gets the grass cap.
///
/// Deterministic: the speckle is a pure hash of the pixel coordinate, so re-running reproduces the
/// committed PNG byte-for-byte.
fn generate_tileset(path: &str) {
    const CELL: u32 = 32;
    const RIM: u32 = 3;
    const CAP: u32 = 7;

    /// Cheap reproducible per-pixel hash → 0..=255.
    fn hash(x: u32, y: u32, salt: u32) -> u32 {
        let mut v = x
            .wrapping_mul(374_761_393)
            .wrapping_add(y.wrapping_mul(668_265_263))
            ^ salt.wrapping_mul(2_246_822_519);
        v ^= v >> 13;
        v = v.wrapping_mul(1_274_126_177);
        (v ^ (v >> 16)) & 0xff
    }

    let mut img = image::RgbaImage::new(CELL * 4, CELL * 4);
    for mask in 0u32..16 {
        let (ox, oy) = (mask % 4 * CELL, mask / 4 * CELL);
        let (open_n, open_e, open_s, open_w) =
            (mask & 1 == 0, mask & 2 == 0, mask & 4 == 0, mask & 8 == 0);
        for y in 0..CELL {
            for x in 0..CELL {
                let dirt = match hash(x, y, 3) {
                    n if n < 30 => [0x6a, 0x43, 0x1e, 0xff],
                    n if n > 230 => [0x9c, 0x69, 0x35, 0xff],
                    _ => [0x87, 0x58, 0x2a, 0xff],
                };
                let px = if open_n && y < CAP {
                    if y + 2 >= CAP {
                        [0x3d, 0x79, 0x22, 0xff]
                    } else if hash(x, y, 7) < 40 {
                        [0x4d, 0x99, 0x2a, 0xff]
                    } else {
                        [0x6a, 0xc1, 0x3e, 0xff]
                    }
                } else if (open_w && x < RIM) || (open_e && x + RIM >= CELL) {
                    [0x9e, 0x6d, 0x3a, 0xff]
                } else if open_s && y + RIM >= CELL {
                    [0x53, 0x33, 0x16, 0xff]
                } else {
                    dirt
                };
                img.put_pixel(ox + x, oy + y, image::Rgba(px));
            }
        }
    }
    img.save(path)
        .unwrap_or_else(|e| panic!("could not write {path}: {e}"));
    println!("wrote {path} — 4x4 edge-16 autotile sheet, {CELL} px cells");
}
