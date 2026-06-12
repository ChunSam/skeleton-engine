//! `AnimationStateMachine` crossfade demo — shows hard switch vs. smooth blend side by side.
//!
//! Two characters share the same spritesheet and state machine layout (idle → walk → run
//! driven by a "speed" float param). The LEFT character uses plain `add_transition` (hard
//! clip switch, no blend); the RIGHT character uses `add_transition_crossfade` (200 ms
//! blend). Pressing keys ramps the shared speed value, so both react simultaneously — the
//! difference in clip pop vs. smooth dissolve is visible at every transition boundary.
//!
//! Run (generate the spritesheet once, then play):
//!     cargo run --example gen_blend_sheet
//!     cargo run --example sm_crossfade
//!
//! Controls:
//!   Right / D / Space — accelerate (ramps toward run)
//!   Shift             — instant sprint
//!   Left / A          — decelerate (back to idle)
//!   Esc               — quit

use engine::{
    AnimationClip, AnimationPlayer, AnimationStateMachine, AnimationSystem, App, BlendWeight,
    Camera, DrawText, Entity, InputState, KeyCode, ShouldQuit, Sprite, StateMachineSystem, System,
    SystemConfig, TextQueue, Transform, TransitionCond, UvRect, WindowConfig, World,
};
use glam::Vec2;

const WINDOW_W: u32 = 900;
const WINDOW_H: u32 = 500;

const SHEET: &str = "examples/assets/blend_locomotion.png";

// Speed thresholds matching the blend_locomotion demo.
const WALK_THRESHOLD: f32 = 0.6;
const RUN_THRESHOLD: f32 = 1.6;
const MAX_SPEED: f32 = 2.4;
const ACCEL: f32 = 3.0;
const DECEL: f32 = 2.5;

/// Crossfade duration applied to the RIGHT character's state-machine transitions.
const CROSSFADE_SECS: f32 = 0.2;

// ─── Animation clips ─────────────────────────────────────────────────────────

fn clip(row: u32, fps: f32) -> AnimationClip {
    AnimationClip {
        frames: (0..4)
            .map(|col| UvRect::from_grid(col, row, 4, 3))
            .collect(),
        fps,
        looping: true,
    }
}

fn clips() -> Vec<AnimationClip> {
    vec![
        clip(0, 4.0),  // 0: idle
        clip(1, 8.0),  // 1: walk
        clip(2, 14.0), // 2: run
    ]
}

fn clip_name(index: usize) -> &'static str {
    ["idle", "walk", "run"].get(index).copied().unwrap_or("?")
}

// ─── State machines ──────────────────────────────────────────────────────────

/// Hard-switch state machine: plain `add_transition` (no crossfade).
fn sm_hard() -> AnimationStateMachine {
    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("walk", 1).add_state("run", 2);
    sm.set_float("speed", 0.0);

    sm.add_transition(
        "idle",
        "walk",
        vec![TransitionCond::FloatGt("speed".into(), WALK_THRESHOLD)],
    )
    .add_transition(
        "walk",
        "run",
        vec![TransitionCond::FloatGt("speed".into(), RUN_THRESHOLD)],
    )
    .add_transition(
        "run",
        "walk",
        vec![TransitionCond::FloatLt("speed".into(), RUN_THRESHOLD)],
    )
    .add_transition(
        "walk",
        "idle",
        vec![TransitionCond::FloatLt("speed".into(), WALK_THRESHOLD)],
    );
    sm
}

/// Crossfade state machine: `add_transition_crossfade` with CROSSFADE_SECS blend.
fn sm_crossfade() -> AnimationStateMachine {
    let mut sm = AnimationStateMachine::new("idle", 0);
    sm.add_state("walk", 1).add_state("run", 2);
    sm.set_float("speed", 0.0);

    sm.add_transition_crossfade(
        "idle",
        "walk",
        vec![TransitionCond::FloatGt("speed".into(), WALK_THRESHOLD)],
        CROSSFADE_SECS,
    )
    .add_transition_crossfade(
        "walk",
        "run",
        vec![TransitionCond::FloatGt("speed".into(), RUN_THRESHOLD)],
        CROSSFADE_SECS,
    )
    .add_transition_crossfade(
        "run",
        "walk",
        vec![TransitionCond::FloatLt("speed".into(), RUN_THRESHOLD)],
        CROSSFADE_SECS,
    )
    .add_transition_crossfade(
        "walk",
        "idle",
        vec![TransitionCond::FloatLt("speed".into(), WALK_THRESHOLD)],
        CROSSFADE_SECS,
    );
    sm
}

// ─── Resources ───────────────────────────────────────────────────────────────

struct Demo {
    hard_char: Entity,
    fade_char: Entity,
    speed: f32,
}

// ─── Systems ─────────────────────────────────────────────────────────────────

struct InputSystem;
impl System for InputSystem {
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
        let go = input.is_pressed(KeyCode::ArrowRight)
            || input.is_pressed(KeyCode::KeyD)
            || input.is_pressed(KeyCode::Space);
        let back = input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA);
        let sprint = input.is_pressed(KeyCode::ShiftLeft) || input.is_pressed(KeyCode::ShiftRight);

        let Some(demo) = world.resource_mut::<Demo>() else {
            return;
        };
        if sprint {
            demo.speed = MAX_SPEED;
        } else if go {
            demo.speed = (demo.speed + ACCEL * dt).min(MAX_SPEED);
        } else if back {
            demo.speed = (demo.speed - DECEL * dt).max(0.0);
        } else {
            // Natural deceleration when no key pressed.
            demo.speed = (demo.speed - DECEL * dt).max(0.0);
        }
        let (hard, fade, speed) = (demo.hard_char, demo.fade_char, demo.speed);

        // Push the same speed into both state machines.
        if let Some(sm) = world.get_mut::<AnimationStateMachine>(hard) {
            sm.set_float("speed", speed);
        }
        if let Some(sm) = world.get_mut::<AnimationStateMachine>(fade) {
            sm.set_float("speed", speed);
        }
    }
}

struct HudSystem;
impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(demo) = world.resource::<Demo>() else {
            return;
        };
        let (hard, fade, speed) = (demo.hard_char, demo.fade_char, demo.speed);

        let hard_clip = world
            .get::<AnimationPlayer>(hard)
            .map(|p| p.current_clip)
            .unwrap_or(0);
        let fade_clip = world
            .get::<AnimationPlayer>(fade)
            .map(|p| p.current_clip)
            .unwrap_or(0);
        let blend_weight = world.get::<BlendWeight>(fade).map(|w| w.0).unwrap_or(1.0);
        let fading = world
            .get::<AnimationPlayer>(fade)
            .map(|p| p.is_crossfading())
            .unwrap_or(false);

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        // Speed bar.
        let filled = (speed / MAX_SPEED * 24.0).round().clamp(0.0, 24.0) as usize;
        let bar: String = format!("{}{}", "#".repeat(filled), "-".repeat(24 - filled));
        tq.push(DrawText::new(
            format!("speed [{bar}] {speed:.2}"),
            Vec2::new(16.0, 14.0),
            18.0,
            [220, 230, 245, 255],
        ));

        // Column labels.
        tq.push(DrawText::new(
            "LEFT: hard switch (add_transition)",
            Vec2::new(30.0, 50.0),
            17.0,
            [255, 160, 100, 255],
        ));
        tq.push(DrawText::new(
            "RIGHT: crossfade 0.2 s (add_transition_crossfade)",
            Vec2::new(WINDOW_W as f32 / 2.0 + 20.0, 50.0),
            17.0,
            [100, 210, 255, 255],
        ));

        // Clip names.
        tq.push(DrawText::new(
            format!("clip: {}", clip_name(hard_clip)),
            Vec2::new(30.0, 74.0),
            17.0,
            [255, 180, 130, 230],
        ));
        tq.push(DrawText::new(
            format!("clip: {}", clip_name(fade_clip)),
            Vec2::new(WINDOW_W as f32 / 2.0 + 20.0, 74.0),
            17.0,
            [130, 220, 255, 230],
        ));

        // Blend weight readout (right character only).
        let bfilled = (blend_weight * 16.0).round().clamp(0.0, 16.0) as usize;
        let bbar: String = format!("{}{}", "#".repeat(bfilled), "-".repeat(16 - bfilled));
        let bcolor = if fading {
            [255, 230, 80, 255]
        } else {
            [130, 150, 170, 200]
        };
        tq.push(DrawText::new(
            format!("blend [{bbar}] {blend_weight:.2}"),
            Vec2::new(WINDOW_W as f32 / 2.0 + 20.0, 98.0),
            17.0,
            bcolor,
        ));

        // Footer controls.
        tq.push(DrawText::new(
            "Right/D/Space accelerate   Left/A decelerate   Shift sprint   Esc quit",
            Vec2::new(16.0, WINDOW_H as f32 - 24.0),
            14.0,
            [160, 175, 200, 200],
        ));
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "sm_crossfade — hard switch vs. crossfade".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.07, 0.09, 0.13, 1.0],
    });

    let sheet = app.load_image(SHEET);

    let cx = WINDOW_W as f32 / 2.0;
    let cy = WINDOW_H as f32 / 2.0 + 40.0;
    let size = Vec2::splat(220.0);

    // LEFT: hard-switch character.
    let hard_char = app.world.spawn();
    app.world.add_component(
        hard_char,
        Transform {
            position: Vec2::new(cx / 2.0, cy),
            scale: size,
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(
        hard_char,
        Sprite::textured_with_handle(SHEET, Some(sheet.clone())),
    );
    app.world
        .add_component(hard_char, AnimationPlayer::new(clips()));
    app.world.add_component(hard_char, sm_hard());

    // RIGHT: crossfade character.
    let fade_char = app.world.spawn();
    app.world.add_component(
        fade_char,
        Transform {
            position: Vec2::new(cx + cx / 2.0, cy),
            scale: size,
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world
        .add_component(fade_char, Sprite::textured_with_handle(SHEET, Some(sheet)));
    app.world
        .add_component(fade_char, AnimationPlayer::new(clips()));
    app.world.add_component(fade_char, sm_crossfade());

    app.world.insert_resource(Demo {
        hard_char,
        fade_char,
        speed: 0.0,
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // System order: input first, then animation (advances frames), then state machine
    // (evaluates transitions on the freshly-advanced state), then HUD.
    app.add_system(InputSystem);
    app.add_system(AnimationSystem::new());
    app.add_system_labeled(
        StateMachineSystem::new(),
        SystemConfig::new()
            .label(StateMachineSystem::LABEL)
            .after(AnimationSystem::LABEL),
    );
    app.add_system(HudSystem);

    app.run();
}
