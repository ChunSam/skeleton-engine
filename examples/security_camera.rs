//! Security-camera stealth — dogfoods the engine's `RenderTarget` / `OffscreenCamera`
//! in a real **playable game** (not a tech demo).
//!
//! You cross a corridor toward the exit on the right. Partway across is a doorway that a
//! guard watches — but the guard patrols a room that is **entirely offscreen** (off the
//! right edge of the world the main camera shows). The only window into that room is the
//! wall **monitor** at the top of the screen: an `OffscreenCamera` renders the guard room
//! into a `RenderTarget`, and the monitor is a `Sprite` that samples that texture. Read the
//! monitor, wait until the guard's red shape has moved away from the yellow door stripe,
//! then dash across the doorway. Get caught in the doorway while the guard is watching it
//! and you snap back to the start.
//!
//! This is the first time `RenderTarget` / `OffscreenCamera` are exercised by a playable
//! game: the existing `minimap` and `split_screen` examples are tech demos (no goal), and
//! both only frame the *same* world region the main camera shows. Here the offscreen camera
//! is the sole view of a **disjoint** region, so the render target is load-bearing for play.
//!
//! Coordinate convention (see `Camera` docs): the camera is **top-left anchored, Y-down** —
//! the default camera shows world `[0, WINDOW_W] × [0, WINDOW_H]`, with +Y pointing down.
//!
//! Run:
//!     cargo run --example security_camera
//!
//! Controls: WASD / arrows to move · R to replay · Esc to quit.

use engine::{
    App, Camera, Color, DrawText, Entity, InputState, KeyCode, OffscreenCamera, RenderLayer,
    ShouldQuit, Sprite, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

const WINDOW_W: u32 = 1000;
const WINDOW_H: u32 = 720;

// ── Player corridor (visible region is x∈[0,1000], y∈[0,720], Y-down) ─────────────
const PLAYER_SPEED: f32 = 240.0;
const PLAYER_HALF: f32 = 17.0;
const PLAYER_START: Vec2 = Vec2::new(70.0, 540.0);
const CORRIDOR_X: (f32, f32) = (40.0, 962.0);
const CORRIDOR_Y: (f32, f32) = (502.0, 600.0);
const EXIT_X: f32 = 905.0; // reach this x to escape

// The doorway crossing: a band of the corridor the offscreen guard can see into.
const DOOR_X: f32 = 500.0;
const THRESH_HALF: f32 = 55.0; // player within DOOR_X ± this is "in the doorway"

// ── Guard room (entirely OFFSCREEN, off the right edge; only the monitor shows it) ─
const ROOM_DOOR_X: f32 = 1400.0; // the guard watches the doorway when it lines up here
const GUARD_Y: f32 = 360.0;
const GUARD_HALF: f32 = 22.0;
const GUARD_SPEED: f32 = 200.0;
const GUARD_MIN_X: f32 = 1180.0;
const GUARD_MAX_X: f32 = 1620.0;
const DOOR_WATCH_HALF: f32 = 100.0; // guard within ROOM_DOOR_X ± this is watching the doorway

// OffscreenCamera framing (top-left anchored): shows x∈[1100,1700], y∈[210,510].
const OC_POS: Vec2 = Vec2::new(1100.0, 210.0);
const OC_ZOOM: f32 = 0.64;

const CAUGHT_FLASH: f32 = 1.1; // seconds the "CAUGHT" feedback holds, player frozen at start

const COL_THRESH_SAFE: Color = Color::rgba(0.22, 0.26, 0.34, 1.0);
const COL_THRESH_CAUGHT: Color = Color::rgba(0.85, 0.25, 0.22, 1.0);

/// Drives the guard patrol, player movement, detection, and win/lose loop.
struct StealthSystem {
    player: Entity,
    guard: Entity,
    thresh: Entity, // doorway floor tile, recolored on catch
    guard_x: f32,
    guard_dir: f32,
    caught_flash: f32,
    attempts: u32,
    won: bool,
}

impl StealthSystem {
    fn reset_player(&mut self, world: &mut World) {
        if let Some(t) = world.get_mut::<Transform>(self.player) {
            t.position = PLAYER_START;
        }
    }

    fn replay(&mut self, world: &mut World) {
        self.reset_player(world);
        self.guard_x = GUARD_MIN_X;
        self.guard_dir = 1.0;
        self.caught_flash = 0.0;
        self.attempts = 0;
        self.won = false;
    }
}

impl System for StealthSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── input (Y-down: up = -y) ──────────────────────────────────────────────
        let (quit, replay, mv) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            let mut mv = Vec2::ZERO;
            if input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft) {
                mv.x -= 1.0;
            }
            if input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight) {
                mv.x += 1.0;
            }
            if input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp) {
                mv.y -= 1.0;
            }
            if input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown) {
                mv.y += 1.0;
            }
            (
                input.just_pressed(KeyCode::Escape),
                input.just_pressed(KeyCode::KeyR),
                mv,
            )
        };
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
            return;
        }
        if replay {
            self.replay(world);
        }

        // ── patrol the (offscreen) guard; only the monitor reveals where it is ───
        self.guard_x += self.guard_dir * GUARD_SPEED * dt;
        if self.guard_x > GUARD_MAX_X {
            self.guard_x = GUARD_MAX_X;
            self.guard_dir = -1.0;
        } else if self.guard_x < GUARD_MIN_X {
            self.guard_x = GUARD_MIN_X;
            self.guard_dir = 1.0;
        }
        if let Some(t) = world.get_mut::<Transform>(self.guard) {
            t.position.x = self.guard_x;
        }
        let door_watched = (self.guard_x - ROOM_DOOR_X).abs() <= DOOR_WATCH_HALF;

        // ── move the player (frozen while the catch feedback plays, or after a win) ─
        if self.caught_flash > 0.0 {
            self.caught_flash -= dt;
        } else if !self.won && mv != Vec2::ZERO {
            let step = mv.normalize() * PLAYER_SPEED * dt;
            if let Some(t) = world.get_mut::<Transform>(self.player) {
                t.position.x = (t.position.x + step.x).clamp(CORRIDOR_X.0, CORRIDOR_X.1);
                t.position.y = (t.position.y + step.y).clamp(CORRIDOR_Y.0, CORRIDOR_Y.1);
            }
        }

        let player_x = world
            .get::<Transform>(self.player)
            .map(|t| t.position.x)
            .unwrap_or(0.0);
        let in_doorway = (player_x - DOOR_X).abs() <= THRESH_HALF;

        // ── detection: in the doorway while the guard watches it → caught ────────
        if !self.won && self.caught_flash <= 0.0 && in_doorway && door_watched {
            self.caught_flash = CAUGHT_FLASH;
            self.attempts += 1;
            self.reset_player(world);
        }

        // ── win: reached the exit ────────────────────────────────────────────────
        if !self.won && player_x >= EXIT_X {
            self.won = true;
        }

        // ── doorway tile feedback (red only at the moment of a catch) ────────────
        if let Some(s) = world.get_mut::<Sprite>(self.thresh) {
            s.color = if self.caught_flash > 0.0 {
                COL_THRESH_CAUGHT
            } else {
                COL_THRESH_SAFE
            };
        }

        // ── HUD ──────────────────────────────────────────────────────────────────
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "WALL MONITOR  (only view of the guard room)",
                Vec2::new(WINDOW_W as f32 / 2.0 - 205.0, 16.0),
                18.0,
                [150, 200, 255, 230],
            ));
            tq.push(DrawText::new(
                format!("caught: {}", self.attempts),
                Vec2::new(16.0, 16.0),
                22.0,
                [235, 235, 245, 255],
            ));
            if self.caught_flash > 0.0 {
                tq.push(DrawText::new(
                    "CAUGHT!  back to the start",
                    Vec2::new(16.0, 300.0),
                    26.0,
                    [255, 120, 110, 255],
                ));
            } else if self.won {
                tq.push(DrawText::new(
                    "ESCAPED!  press R to replay",
                    Vec2::new(16.0, 300.0),
                    30.0,
                    [120, 255, 160, 255],
                ));
            } else {
                tq.push(DrawText::new(
                    "watch the monitor — cross the doorway when the guard is away from the yellow stripe",
                    Vec2::new(16.0, 300.0),
                    16.0,
                    [200, 210, 230, 220],
                ));
            }
            tq.push(DrawText::new(
                "WASD / arrows to move    R replay    Esc quit",
                Vec2::new(16.0, WINDOW_H as f32 - 30.0),
                16.0,
                [170, 185, 210, 200],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "StealthSystem"
    }
}

/// Spawn a colored box centered at `pos` with full size `size`. `layer` is its `RenderLayer`
/// bit; pass `None` to leave it on the default layer 0 (which the offscreen camera captures).
fn spawn_box(
    app: &mut App,
    pos: Vec2,
    size: Vec2,
    color: impl Into<Color>,
    z: f32,
    layer: Option<i32>,
) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: pos,
            scale: size,
            rotation: 0.0,
            z,
        },
    );
    app.world.add_component(
        e,
        Sprite {
            color: color.into(),
            ..Default::default()
        },
    );
    if let Some(l) = layer {
        app.world.add_component(e, RenderLayer(l));
    }
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine security camera".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    // Offscreen render target the monitor samples (2:1 monitor).
    app.create_render_target("camfeed", 384, 192);

    // ── visible corridor (layer 0, but positioned far from the offscreen frame) ───
    spawn_box(
        &mut app,
        Vec2::new(500.0, 560.0),
        Vec2::new(980.0, 150.0),
        [0.13, 0.14, 0.18, 1.0],
        0.0,
        None,
    );
    // top wall split into two segments, leaving the doorway gap at x≈DOOR_X
    spawn_box(
        &mut app,
        Vec2::new(230.0, 470.0),
        Vec2::new(420.0, 36.0),
        [0.20, 0.21, 0.26, 1.0],
        0.5,
        None,
    );
    spawn_box(
        &mut app,
        Vec2::new(770.0, 470.0),
        Vec2::new(420.0, 36.0),
        [0.20, 0.21, 0.26, 1.0],
        0.5,
        None,
    );
    // doorway floor tile (recolored on catch)
    let thresh = spawn_box(
        &mut app,
        Vec2::new(DOOR_X, 555.0),
        Vec2::new(THRESH_HALF * 2.0, 150.0),
        COL_THRESH_SAFE,
        0.4,
        None,
    );
    // exit zone (right)
    spawn_box(
        &mut app,
        Vec2::new(935.0, 555.0),
        Vec2::new(70.0, 150.0),
        [0.20, 0.70, 0.35, 1.0],
        0.6,
        None,
    );

    // ── player ────────────────────────────────────────────────────────────────────
    let player = spawn_box(
        &mut app,
        PLAYER_START,
        Vec2::splat(PLAYER_HALF * 2.0),
        [0.30, 0.75, 1.0, 1.0],
        1.0,
        None,
    );

    // ── guard room: ENTIRELY OFFSCREEN at x≈1400; only the monitor renders it ──────
    // backdrop + crates for a visual reference on the feed
    spawn_box(
        &mut app,
        Vec2::new(1400.0, GUARD_Y),
        Vec2::new(640.0, 320.0),
        [0.14, 0.15, 0.20, 1.0],
        0.0,
        None,
    );
    spawn_box(
        &mut app,
        Vec2::new(1220.0, GUARD_Y - 96.0),
        Vec2::new(50.0, 50.0),
        [0.34, 0.31, 0.26, 1.0],
        0.2,
        None,
    );
    spawn_box(
        &mut app,
        Vec2::new(1585.0, GUARD_Y + 86.0),
        Vec2::new(46.0, 46.0),
        [0.31, 0.29, 0.25, 1.0],
        0.2,
        None,
    );
    // the yellow "door" stripe the guard watches when it lines up with x≈ROOM_DOOR_X
    spawn_box(
        &mut app,
        Vec2::new(ROOM_DOOR_X, GUARD_Y),
        Vec2::new(14.0, 300.0),
        [0.95, 0.85, 0.25, 1.0],
        0.3,
        None,
    );
    // the guard itself (red); its x is driven each frame by StealthSystem
    let guard = spawn_box(
        &mut app,
        Vec2::new(GUARD_MIN_X, GUARD_Y),
        Vec2::splat(GUARD_HALF * 2.0),
        [0.90, 0.25, 0.22, 1.0],
        1.0,
        None,
    );

    // ── monitor: OffscreenCamera frames the guard room → "camfeed" RT ──────────────
    let oc = app.world.spawn();
    app.world.add_component(
        oc,
        OffscreenCamera {
            target: "camfeed".to_string(),
            camera: Camera::new(OC_POS, OC_ZOOM),
            layer_mask: 1 << 0, // only layer-0 world content (the monitor itself is layer 1)
        },
    );

    // monitor bezel + feed sprite, mounted on the wall at the top of the screen.
    // layer 1 keeps them out of the offscreen capture.
    spawn_box(
        &mut app,
        Vec2::new(500.0, 150.0),
        Vec2::new(452.0, 236.0),
        [0.04, 0.05, 0.07, 1.0],
        49.0,
        Some(1),
    );
    let monitor = app.world.spawn();
    app.world.add_component(
        monitor,
        Transform {
            position: Vec2::new(500.0, 150.0),
            scale: Vec2::new(420.0, 210.0),
            rotation: 0.0,
            z: 50.0,
        },
    );
    app.world.add_component(
        monitor,
        Sprite {
            texture: Some("camfeed".into()),
            color: Color::WHITE,
            ..Default::default()
        },
    );
    app.world.add_component(monitor, RenderLayer(1));

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(StealthSystem {
        player,
        guard,
        thresh,
        guard_x: GUARD_MIN_X,
        guard_dir: 1.0,
        caught_flash: 0.0,
        attempts: 0,
        won: false,
    });

    app.run();
}
