//! Triggerable cutscene — dogfoods the engine's `Timeline` keyframe system in a real
//! **playable** scene (not a unit test).
//!
//! You move a character through a room toward a sealed gate. Walk into the glowing rune and
//! a **cutscene** plays: the camera pans and zooms in on the gate, the two gate panels slide
//! apart, and a black overlay dips in and out for a "scene transition" feel — all driven by
//! `Timeline` keyframe tracks. When it finishes (or you press Space to skip), control returns
//! and you walk through the now-open gate to the exit.
//!
//! What this proves that the unit tests don't: `Timeline` driving **multiple** entities at
//! once (two gate panels via `position` tracks, an overlay via an `alpha` track) AND the
//! **camera** via the new `CameraTarget` marker (a `Timeline` on a marker-tagged rig entity
//! writes its `position`/`zoom` tracks straight into the `Camera` resource). Before this,
//! `Timeline` could only animate an entity's own `Transform`/`Sprite`, never the camera.
//!
//! Coordinate convention (see `Camera` docs): the camera is **top-left anchored, Y-down** —
//! the default camera shows world `[0, WINDOW_W] × [0, WINDOW_H]`, +Y pointing down.
//!
//! Run:
//!     cargo run --example timeline_cutscene
//!
//! Controls: WASD / arrows to move · Space to skip the cutscene · R to replay · Esc to quit.

use engine::{
    App, Camera, CameraTarget, DrawText, Easing, Entity, InputState, KeyCode, RenderLayer,
    ShouldQuit, Sprite, System, TextQueue, Timeline, TimelineSystem, Transform, WindowConfig,
    World,
};
use glam::Vec2;

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 600;

// ── Room / player (visible world is x∈[0,960], y∈[0,600], Y-down) ─────────────────
const PLAYER_HALF: f32 = 17.0;
const PLAYER_SPEED: f32 = 240.0;
const PLAYER_START: Vec2 = Vec2::new(140.0, 300.0);
const ROOM_X: (f32, f32) = (24.0, 936.0);
const ROOM_Y: (f32, f32) = (60.0, 540.0);

// The closed gate blocks the right side until the cutscene opens it.
const GATE_BLOCK_X: f32 = 575.0; // player can't pass this while the gate is shut

// ── Gate (a wall strip at x≈620 with a doorway hole the panels fill) ──────────────
const DOOR_X: f32 = 620.0;
const GATE_TOP_CLOSED: Vec2 = Vec2::new(DOOR_X, 257.0);
const GATE_TOP_OPEN: Vec2 = Vec2::new(DOOR_X, 128.0);
const GATE_BOT_CLOSED: Vec2 = Vec2::new(DOOR_X, 343.0);
const GATE_BOT_OPEN: Vec2 = Vec2::new(DOOR_X, 472.0);

// ── Rune trigger + exit ──────────────────────────────────────────────────────────
const RUNE_POS: Vec2 = Vec2::new(430.0, 300.0);
const RUNE_HALF: Vec2 = Vec2::new(46.0, 60.0);
const EXIT_POS: Vec2 = Vec2::new(740.0, 300.0);
const EXIT_HALF: Vec2 = Vec2::new(42.0, 72.0);

// ── Cutscene timing / framing ──────────────────────────────────────────────────────
const CUT_DUR: f32 = 4.4;
const CAM_GATE_POS: Vec2 = Vec2::new(330.0, 107.0); // top-left anchored framing of the gate
const CAM_GATE_ZOOM: f32 = 1.55; // visible ≈ 619×387, centered on the gate + exit

const COL_FLOOR: [f32; 4] = [0.10, 0.11, 0.15, 1.0];
const COL_WALL: [f32; 4] = [0.22, 0.23, 0.30, 1.0];
const COL_GATE: [f32; 4] = [0.58, 0.44, 0.22, 1.0];
const COL_PLAYER: [f32; 4] = [0.30, 0.75, 1.0, 1.0];
const COL_EXIT: [f32; 4] = [0.22, 0.72, 0.36, 1.0];
const COL_RUNE: [f32; 4] = [0.74, 0.46, 0.95, 1.0];

#[derive(PartialEq)]
enum Phase {
    Explore,  // free roam; walking into the rune starts the cutscene
    Cutscene, // timelines play; player frozen; Space skips
    Play,     // gate open; walk to the exit
    Won,
}

/// Drives player movement, the rune trigger, the cutscene state machine, and the HUD.
/// The cutscene itself (camera + gate panels + overlay) is authored entirely as `Timeline`s;
/// this system only starts/skips them and reads the rig timeline's `is_finished()`.
struct CutsceneSystem {
    player: Entity,
    rune: Entity,
    rig: Entity, // CameraTarget + Timeline → drives the Camera resource
    gate_top: Entity,
    gate_bottom: Entity,
    overlay: Entity,
    phase: Phase,
    elapsed: f32, // for the rune's idle pulse
}

impl CutsceneSystem {
    fn timelines(&self) -> [Entity; 4] {
        [self.rig, self.gate_top, self.gate_bottom, self.overlay]
    }

    fn start_cutscene(&mut self, world: &mut World) {
        for e in self.timelines() {
            if let Some(tl) = world.get_mut::<Timeline>(e) {
                tl.restart();
            }
        }
        self.phase = Phase::Cutscene;
    }

    /// Jump every cutscene timeline to its end; `TimelineSystem` then samples the final
    /// keyframe (camera home, gate open, overlay clear) and reports `is_finished()`.
    fn skip_cutscene(&mut self, world: &mut World) {
        for e in self.timelines() {
            if let Some(tl) = world.get_mut::<Timeline>(e) {
                tl.time = tl.duration;
            }
        }
    }

    fn reset(&mut self, world: &mut World) {
        if let Some(t) = world.get_mut::<Transform>(self.player) {
            t.position = PLAYER_START;
        }
        for e in self.timelines() {
            if let Some(tl) = world.get_mut::<Timeline>(e) {
                tl.time = 0.0;
                tl.playing = false; // hold each timeline's t=0 frame (gate shut, overlay clear)
            }
        }
        self.phase = Phase::Explore;
    }

    fn player_pos(&self, world: &World) -> Vec2 {
        world
            .get::<Transform>(self.player)
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO)
    }

    fn move_player(&self, world: &mut World, mv: Vec2, dt: f32, block_gate: bool) {
        if mv == Vec2::ZERO {
            return;
        }
        let step = mv.normalize() * PLAYER_SPEED * dt;
        let max_x = if block_gate { GATE_BLOCK_X } else { ROOM_X.1 };
        if let Some(t) = world.get_mut::<Transform>(self.player) {
            t.position.x = (t.position.x + step.x).clamp(ROOM_X.0, max_x);
            t.position.y = (t.position.y + step.y).clamp(ROOM_Y.0, ROOM_Y.1);
        }
    }
}

fn in_zone(p: Vec2, center: Vec2, half: Vec2) -> bool {
    (p.x - center.x).abs() <= half.x && (p.y - center.y).abs() <= half.y
}

impl System for CutsceneSystem {
    fn name(&self) -> &'static str {
        "CutsceneSystem"
    }

    fn run(&mut self, world: &mut World, dt: f32) {
        self.elapsed += dt;

        // ── input (Y-down: up = -y) ──────────────────────────────────────────────
        let (quit, replay, skip, mv) = {
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
                input.just_pressed(KeyCode::Space),
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
            self.reset(world);
        }

        // ── state machine ────────────────────────────────────────────────────────
        match self.phase {
            Phase::Explore => {
                self.move_player(world, mv, dt, true);
                if in_zone(self.player_pos(world), RUNE_POS, RUNE_HALF) {
                    self.start_cutscene(world);
                }
            }
            Phase::Cutscene => {
                if skip {
                    self.skip_cutscene(world);
                }
                // The camera rig timeline is the cutscene's clock; when it ends, hand control back.
                let done = world
                    .get::<Timeline>(self.rig)
                    .map(|t| t.is_finished())
                    .unwrap_or(true);
                if done {
                    self.phase = Phase::Play;
                }
            }
            Phase::Play => {
                self.move_player(world, mv, dt, false);
                if in_zone(self.player_pos(world), EXIT_POS, EXIT_HALF) {
                    self.phase = Phase::Won;
                }
            }
            Phase::Won => {}
        }

        // ── rune idle pulse (signals it's interactable) ──────────────────────────
        if let Some(s) = world.get_mut::<Sprite>(self.rune) {
            let pulse = if self.phase == Phase::Explore {
                0.55 + 0.35 * (self.elapsed * 3.2).sin().abs()
            } else {
                0.3
            };
            s.color[3] = pulse;
        }

        // ── HUD (screen-space, so it's unaffected by the cutscene camera) ────────
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let banner: (&str, [u8; 4]) = match self.phase {
                Phase::Explore => (
                    "walk into the glowing rune to start the cutscene",
                    [205, 195, 235, 230],
                ),
                Phase::Cutscene => (
                    "\u{2726}  cutscene playing  \u{2014}  Space to skip",
                    [255, 245, 200, 255],
                ),
                Phase::Play => (
                    "the gate is open \u{2014} walk through to the exit",
                    [180, 230, 255, 235],
                ),
                Phase::Won => (
                    "YOU PASSED THE GATE  \u{2014}  press R to replay",
                    [130, 255, 170, 255],
                ),
            };
            tq.push(DrawText::new(
                banner.0,
                Vec2::new(20.0, 22.0),
                24.0,
                banner.1,
            ));
            tq.push(DrawText::new(
                "WASD / arrows move    Space skip    R replay    Esc quit",
                Vec2::new(20.0, WINDOW_H as f32 - 30.0),
                16.0,
                [170, 185, 210, 200],
            ));
        }
    }
}

/// Spawn a colored box centered at `pos` with full size `size` on render-`layer`.
fn spawn_box(app: &mut App, pos: Vec2, size: Vec2, color: [f32; 4], z: f32, layer: i32) -> Entity {
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
            color,
            ..Default::default()
        },
    );
    app.world.add_component(e, RenderLayer(layer));
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine timeline cutscene".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    // ── room floor + flanking walls around the doorway ───────────────────────────
    spawn_box(
        &mut app,
        Vec2::new(480.0, 300.0),
        Vec2::new(960.0, 600.0),
        COL_FLOOR,
        0.0,
        0,
    );
    // wall strip at x≈DOOR_X, split into a top and bottom block with a doorway hole between.
    spawn_box(
        &mut app,
        Vec2::new(DOOR_X, 107.0),
        Vec2::new(44.0, 214.0),
        COL_WALL,
        2.0,
        2,
    );
    spawn_box(
        &mut app,
        Vec2::new(DOOR_X, 493.0),
        Vec2::new(44.0, 214.0),
        COL_WALL,
        2.0,
        2,
    );

    // ── rune trigger + exit ──────────────────────────────────────────────────────
    let rune = spawn_box(&mut app, RUNE_POS, Vec2::new(50.0, 50.0), COL_RUNE, 0.5, 0);
    spawn_box(&mut app, EXIT_POS, Vec2::new(54.0, 96.0), COL_EXIT, 0.5, 0);

    // ── gate panels: layer 1 so they tuck behind the layer-2 walls when open ─────
    let gate_top = spawn_box(
        &mut app,
        GATE_TOP_CLOSED,
        Vec2::new(40.0, 86.0),
        COL_GATE,
        1.0,
        1,
    );
    app.world
        .add_component(gate_top, gate_timeline(GATE_TOP_CLOSED, GATE_TOP_OPEN));
    let gate_bottom = spawn_box(
        &mut app,
        GATE_BOT_CLOSED,
        Vec2::new(40.0, 86.0),
        COL_GATE,
        1.0,
        1,
    );
    app.world
        .add_component(gate_bottom, gate_timeline(GATE_BOT_CLOSED, GATE_BOT_OPEN));

    // ── player ───────────────────────────────────────────────────────────────────
    let player = spawn_box(
        &mut app,
        PLAYER_START,
        Vec2::splat(PLAYER_HALF * 2.0),
        COL_PLAYER,
        1.0,
        0,
    );

    // ── full-screen overlay (layer 5, on top); its alpha track drives the fade ───
    // Sized far larger than any in-room camera view so it covers the screen during the
    // pan/zoom regardless of where the camera is.
    let overlay = app.world.spawn();
    app.world.add_component(
        overlay,
        Transform {
            position: Vec2::new(480.0, 300.0),
            scale: Vec2::new(2400.0, 1600.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(
        overlay,
        Sprite {
            color: [0.0, 0.0, 0.0, 0.0],
            ..Default::default()
        },
    );
    app.world.add_component(overlay, RenderLayer(5));
    app.world.add_component(overlay, overlay_timeline());

    // ── camera rig: a Timeline + CameraTarget marker → drives the Camera resource ─
    let rig = app.world.spawn();
    app.world.add_component(rig, camera_timeline());
    app.world.add_component(rig, CameraTarget);

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // TimelineSystem first so CutsceneSystem reads the freshly-advanced rig state this frame.
    app.add_system(TimelineSystem);
    app.add_system(CutsceneSystem {
        player,
        rune,
        rig,
        gate_top,
        gate_bottom,
        overlay,
        phase: Phase::Explore,
        elapsed: 0.0,
    });

    app.run();
}

/// Camera pan + zoom: hold home → dolly in on the gate → hold → return home.
/// Starts paused (`playing = false`) so it holds the home framing until the cutscene fires.
fn camera_timeline() -> Timeline {
    let mut tl = Timeline::new(CUT_DUR);
    tl.playing = false;
    tl.position.add(0.0, Vec2::ZERO, Easing::Linear);
    tl.position.add(0.5, Vec2::ZERO, Easing::EaseInOut);
    tl.position.add(1.3, CAM_GATE_POS, Easing::EaseInOut);
    tl.position.add(3.2, CAM_GATE_POS, Easing::Linear);
    tl.position.add(CUT_DUR, Vec2::ZERO, Easing::EaseInOut);
    tl.zoom.add(0.0, 1.0, Easing::Linear);
    tl.zoom.add(0.5, 1.0, Easing::EaseInOut);
    tl.zoom.add(1.3, CAM_GATE_ZOOM, Easing::EaseInOut);
    tl.zoom.add(3.2, CAM_GATE_ZOOM, Easing::Linear);
    tl.zoom.add(CUT_DUR, 1.0, Easing::EaseInOut);
    tl
}

/// A gate panel: stay shut until the camera settles, then slide open and hold.
fn gate_timeline(closed: Vec2, open: Vec2) -> Timeline {
    let mut tl = Timeline::new(CUT_DUR);
    tl.playing = false;
    tl.position.add(0.0, closed, Easing::Linear);
    tl.position.add(1.7, closed, Easing::Linear);
    tl.position.add(2.7, open, Easing::EaseInOut);
    tl.position.add(CUT_DUR, open, Easing::Linear);
    tl
}

/// Overlay alpha: a quick fade-to-black-and-clear at the open, then again at the close.
fn overlay_timeline() -> Timeline {
    let mut tl = Timeline::new(CUT_DUR);
    tl.playing = false;
    tl.alpha.add(0.0, 0.0, Easing::Linear);
    tl.alpha.add(0.25, 0.7, Easing::EaseOut);
    tl.alpha.add(0.6, 0.0, Easing::EaseInOut);
    tl.alpha.add(3.6, 0.0, Easing::Linear);
    tl.alpha.add(4.0, 0.6, Easing::EaseOut);
    tl.alpha.add(CUT_DUR, 0.0, Easing::EaseInOut);
    tl
}
