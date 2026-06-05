//! `BlendTree1D` locomotion demo — dogfoods 1D parameter-driven clip blending.
//!
//! A single character's animation is driven entirely by one number: **speed**. Holding
//! an accelerate key ramps speed up; releasing decelerates it. A [`BlendTree1D`] maps
//! speed to idle / walk / run clips (thresholds 0.0 / 0.6 / 1.6) and the engine
//! crossfades between them. This is the first time `BlendTree1D` is exercised by a
//! real interactive loop (it shipped with zero game/example coverage; see
//! `docs/NEXT_WORK.md`).
//!
//! It also showcases the two engine changes made alongside it:
//!   * **True 2-UV crossfade blend** — the transition is a smooth per-pixel cross-dissolve
//!     (`mix(from_frame, to_frame, weight)` in `sprite.wgsl`), not the old 50% hard swap.
//!     The `BlendWeight` readout in the HUD sweeps 0→1 during each transition.
//!   * **Stranding fix** — accel is fast enough that speed crosses idle→walk→run within a
//!     single crossfade, the exact case that used to strand `BlendTreeSystem` on the
//!     intermediate clip; here it reaches run correctly.
//!
//! Run (generate the spritesheet once, then play):
//!     cargo run --example gen_blend_sheet
//!     cargo run --example blend_locomotion
//!
//! Controls: hold Right / D / Space to accelerate · release to slow down ·
//! Shift = instant sprint · Esc quit.

use engine::{
    AnimationClip, AnimationPlayer, AnimationSystem, App, BlendEntry, BlendTree1D, BlendTreeSystem,
    BlendWeight, Camera, DrawText, Entity, InputState, KeyCode, ShouldQuit, Sprite, System,
    TextQueue, Transform, UvRect, WindowConfig, World,
};
use glam::Vec2;

const WINDOW_W: u32 = 800;
const WINDOW_H: u32 = 600;

const SHEET: &str = "examples/assets/blend_locomotion.png";

// Speed parameter shaping. Accel is deliberately brisk so a hold crosses two thresholds
// (idle→walk→run) inside one crossfade — the case the stranding fix targets.
const MAX_SPEED: f32 = 2.4;
const ACCEL: f32 = 4.5;
const DECEL: f32 = 3.5;

const WALK_THRESHOLD: f32 = 0.6;
const RUN_THRESHOLD: f32 = 1.6;
const CROSSFADE: f32 = 0.3;

/// Holds the player entity and its current locomotion speed (the blend parameter).
struct Loco {
    player: Entity,
    speed: f32,
}

fn clip(row: u32, fps: f32) -> AnimationClip {
    AnimationClip {
        frames: (0..4)
            .map(|col| UvRect::from_grid(col, row, 4, 3))
            .collect(),
        fps,
        looping: true,
    }
}

fn clip_name(index: usize) -> &'static str {
    match index {
        0 => "idle",
        1 => "walk",
        _ => "run",
    }
}

// ─── Input → speed → blend parameter ───────────────────────────────────────────
struct ControlSystem;
impl System for ControlSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(input) = world.resource::<InputState>() else {
            return;
        };
        if input.just_pressed(KeyCode::Escape) {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
            return;
        }
        let accelerate = input.is_pressed(KeyCode::ArrowRight)
            || input.is_pressed(KeyCode::KeyD)
            || input.is_pressed(KeyCode::Space);
        let sprint = input.is_pressed(KeyCode::ShiftLeft) || input.is_pressed(KeyCode::ShiftRight);

        let Some(loco) = world.resource_mut::<Loco>() else {
            return;
        };
        if sprint {
            loco.speed = MAX_SPEED;
        } else if accelerate {
            loco.speed = (loco.speed + ACCEL * dt).min(MAX_SPEED);
        } else {
            loco.speed = (loco.speed - DECEL * dt).max(0.0);
        }
        let (player, speed) = (loco.player, loco.speed);

        if let Some(tree) = world.get_mut::<BlendTree1D>(player) {
            tree.set_param(speed);
        }
    }
}

// ─── HUD ───────────────────────────────────────────────────────────────────────
struct HudSystem;
impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, speed)) = world.resource::<Loco>().map(|l| (l.player, l.speed)) else {
            return;
        };
        let clip = world
            .get::<AnimationPlayer>(player)
            .map(|p| p.current_clip)
            .unwrap_or(0);
        let crossfading = world
            .get::<AnimationPlayer>(player)
            .map(|p| p.is_crossfading())
            .unwrap_or(false);
        let weight = world.get::<BlendWeight>(player).map(|w| w.0).unwrap_or(1.0);

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        // speed bar
        let filled = (speed / MAX_SPEED * 20.0).round().clamp(0.0, 20.0) as usize;
        let bar: String = format!("{}{}", "#".repeat(filled), "-".repeat(20 - filled));
        tq.push(DrawText::new(
            format!("speed [{bar}] {speed:.2}"),
            Vec2::new(12.0, 12.0),
            22.0,
            [235, 235, 245, 255],
        ));
        tq.push(DrawText::new(
            format!("clip: {}", clip_name(clip)),
            Vec2::new(12.0, 40.0),
            22.0,
            [200, 225, 255, 240],
        ));
        // BlendWeight: 1.0 when settled, sweeps during a crossfade.
        let blend_disp = if crossfading { weight } else { 0.0 };
        let bfilled = (blend_disp * 20.0).round().clamp(0.0, 20.0) as usize;
        let bbar: String = format!("{}{}", "#".repeat(bfilled), "-".repeat(20 - bfilled));
        let bcolor = if crossfading {
            [255, 210, 130, 255]
        } else {
            [120, 130, 145, 220]
        };
        tq.push(DrawText::new(
            format!("blend [{bbar}] {blend_disp:.2}"),
            Vec2::new(12.0, 68.0),
            22.0,
            bcolor,
        ));

        tq.push(DrawText::new(
            "thresholds:  idle 0.0   walk 0.6   run 1.6",
            Vec2::new(12.0, WINDOW_H as f32 - 50.0),
            16.0,
            [170, 185, 210, 200],
        ));
        tq.push(DrawText::new(
            "hold Right/D/Space accelerate   Shift sprint   Esc quit",
            Vec2::new(12.0, WINDOW_H as f32 - 26.0),
            16.0,
            [170, 185, 210, 200],
        ));
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine blend locomotion".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    let sheet = app.load_image(SHEET);

    // The engine camera is top-left anchored (camera.position = viewport top-left, see
    // `src/camera.rs`), so place the character at the screen-center world coordinate.
    let center = Vec2::new(WINDOW_W as f32 / 2.0, WINDOW_H as f32 / 2.0);
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: center,
            scale: Vec2::splat(280.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world
        .add_component(player, Sprite::textured_with_handle(SHEET, Some(sheet)));
    app.world.add_component(
        player,
        AnimationPlayer::new(vec![
            clip(0, 4.0),  // idle
            clip(1, 8.0),  // walk
            clip(2, 14.0), // run
        ]),
    );
    app.world.add_component(
        player,
        BlendTree1D::new(
            vec![
                BlendEntry {
                    threshold: 0.0,
                    clip_index: 0,
                },
                BlendEntry {
                    threshold: WALK_THRESHOLD,
                    clip_index: 1,
                },
                BlendEntry {
                    threshold: RUN_THRESHOLD,
                    clip_index: 2,
                },
            ],
            CROSSFADE,
        ),
    );

    app.world.insert_resource(Loco { player, speed: 0.0 });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // Order: input sets the blend param → tree picks the clip → animation advances/blends → HUD.
    app.add_system(ControlSystem);
    app.add_system(BlendTreeSystem);
    app.add_system(AnimationSystem);
    app.add_system(HudSystem);

    app.run();
}
