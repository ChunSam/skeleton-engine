//! `FrameConfig::max_dt` — the per-frame delta-time cap.
//!
//! The main loop clamps each frame's `dt` to `FrameConfig::max_dt` (default **0.1 s**) so a
//! single stall — a window drag, a debugger breakpoint, a backgrounded tab — can't hand
//! systems a huge `dt` that tunnels physics or leaps animations. Before this knob the cap was
//! hardcoded to `0.1`.
//!
//! This demo hitches on purpose (a `thread::sleep` every ~1.2 s, or press **S**) and moves two
//! markers right at the same speed:
//!
//! - **Top "capped (dt)"** — uses the engine `dt`, which is clamped to `max_dt`. After a stall
//!   it advances only `speed * max_dt`, so it never leaps.
//! - **Bottom "raw (wall-clock)"** — uses the real elapsed time. After a stall it lurches
//!   forward — exactly the teleport the cap prevents.
//!
//! Press **↑/↓** to change `max_dt` live: raise it and the capped marker tracks the raw one
//! (less protection, more catch-up); lower it and stalls are bounded even harder. The cap is
//! also unit-tested in `FrameConfig` (`cap_clamps_long_frames_but_passes_short_ones`).
use engine::{
    App, DrawText, Entity, FrameConfig, InputState, KeyCode, ShouldQuit, Sprite, System, TextQueue,
    Transform, Vec2, WindowConfig, World,
};
use std::time::{Duration, Instant};

const WIN_W: u32 = 760;
const WIN_H: u32 = 400;
const START_X: f32 = 40.0;
const END_X: f32 = 720.0;
const SPEED: f32 = 200.0; // px/s
const CAPPED_Y: f32 = 150.0;
const RAW_Y: f32 = 250.0;
const STALL_INTERVAL: f32 = 1.2; // auto-hitch cadence (real seconds)
const STALL_MS: u64 = 200; // simulated frame stall

struct Demo {
    capped: Entity,
    raw: Entity,
    last: Option<Instant>,
    last_dt: f32,
    last_real: f32,
    since_stall: f32,
    stall_flash: f32,
}

struct CapSystem;

impl System for CapSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Real wall-clock elapsed (uncapped) — what the raw marker uses.
        let now = Instant::now();
        let (capped, raw, last) = {
            let d = world.resource::<Demo>().unwrap();
            (d.capped, d.raw, d.last)
        };
        let real = last.map(|l| (now - l).as_secs_f32()).unwrap_or(dt);

        // Input: quit, manual stall, max_dt nudge. Read all keys first (ends the InputState
        // borrow) before mutating other resources.
        let (quit, manual_stall, up, down) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Escape),
                    i.just_pressed(KeyCode::KeyS),
                    i.just_pressed(KeyCode::ArrowUp),
                    i.just_pressed(KeyCode::ArrowDown),
                )
            })
            .unwrap_or((false, false, false, false));
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if up || down {
            if let Some(cfg) = world.resource_mut::<FrameConfig>() {
                cfg.max_dt = if up {
                    (cfg.max_dt + 0.05).min(1.0)
                } else {
                    (cfg.max_dt - 0.05).max(0.02)
                };
            }
        }

        // Advance markers: capped uses engine dt (clamped); raw uses wall-clock elapsed.
        advance(world, capped, SPEED * dt);
        advance(world, raw, SPEED * real);

        // Update timers + decide whether to hitch this frame.
        let mut do_stall = manual_stall;
        if let Some(d) = world.resource_mut::<Demo>() {
            d.last = Some(now);
            d.last_dt = dt;
            d.last_real = real;
            d.stall_flash = (d.stall_flash - real).max(0.0);
            d.since_stall += real;
            if d.since_stall >= STALL_INTERVAL {
                d.since_stall = 0.0;
                do_stall = true;
            }
            if do_stall {
                d.stall_flash = 0.5;
            }
        }

        draw_hud(world);

        // Simulate the frame hitch LAST, so the *next* frame sees the long gap.
        if do_stall {
            std::thread::sleep(Duration::from_millis(STALL_MS));
        }
    }

    fn name(&self) -> &'static str {
        "CapSystem"
    }
}

/// Moves a marker right by `dx`, wrapping back to the start past the right edge.
fn advance(world: &mut World, e: Entity, dx: f32) {
    if let Some(tr) = world.get_mut::<Transform>(e) {
        tr.position.x += dx;
        if tr.position.x > END_X {
            tr.position.x = START_X;
        }
    }
}

fn draw_hud(world: &mut World) {
    let max_dt = world
        .resource::<FrameConfig>()
        .map(|c| c.max_dt)
        .unwrap_or(0.1);
    let (last_dt, last_real, flash) = {
        let d = world.resource::<Demo>().unwrap();
        (d.last_dt, d.last_real, d.stall_flash > 0.0)
    };
    if let Some(tq) = world.resource_mut::<TextQueue>() {
        tq.push(DrawText::new(
            "FrameConfig::max_dt — per-frame delta cap (hitches every ~1.2s; S to stall, Up/Down max_dt)",
            Vec2::new(14.0, 12.0),
            15.0,
            [235, 225, 200, 255],
        ));
        tq.push(DrawText::new(
            "capped (dt) — clamped to max_dt, never leaps",
            Vec2::new(14.0, CAPPED_Y - 34.0),
            14.0,
            [150, 220, 255, 255],
        ));
        tq.push(DrawText::new(
            "raw (wall-clock) — lurches on a stall (what the cap prevents)",
            Vec2::new(14.0, RAW_Y - 34.0),
            14.0,
            [255, 170, 150, 255],
        ));
        let stall = if flash { "   << STALL" } else { "" };
        tq.push(DrawText::new(
            format!("max_dt {max_dt:.2}s   engine dt {last_dt:.3}s   wall-clock dt {last_real:.3}s{stall}"),
            Vec2::new(14.0, WIN_H as f32 - 26.0),
            14.0,
            [200, 215, 235, 255],
        ));
    }
}

fn spawn_marker(app: &mut App, y: f32, color: Sprite) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(START_X, y),
            scale: Vec2::splat(28.0),
            ..Default::default()
        },
    );
    app.world.add_component(e, color);
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "FrameConfig::max_dt — per-frame delta cap".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let capped = spawn_marker(&mut app, CAPPED_Y, Sprite::colored(0.4, 0.75, 0.95));
    let raw = spawn_marker(&mut app, RAW_Y, Sprite::colored(0.95, 0.5, 0.4));

    app.world.insert_resource(Demo {
        capped,
        raw,
        last: None,
        last_dt: 0.0,
        last_real: 0.0,
        since_stall: 0.0,
        stall_flash: 0.0,
    });

    app.add_system(CapSystem);
    app.run();
}
