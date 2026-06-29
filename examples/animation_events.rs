//! `AnimationEvents` — fire a tagged event when an animation playhead enters a frame.
//!
//! Add an [`AnimationEvents`] component next to an [`AnimationPlayer`], list `(clip, frame, tag)`
//! triples, and register the bus with `App::register_event::<AnimationEvent>()`. Each frame
//! [`AnimationSystem`] advances the playhead onto a listed frame it sends an [`AnimationEvent`] you
//! read the same frame — the idiomatic way to drive footsteps / attack hit-frames / VFX off an
//! animation without hand-polling `current_frame`.
//!
//! This demo plays a 4-frame walk cycle whose two **contact** frames (1 and 3) carry a `"footstep"`
//! event. A reader system counts the steps, flashes "STEP!", and logs each one to the console.
//!
//! - **Esc** — quit
//!
//! Headless: `HEADLESS_SHOT=/tmp/anim_events.png cargo run --example animation_events`
//! (defaults to 60 warmup frames so the cycle has time to advance; `HEADLESS_FRAMES=N` overrides).
use engine::{
    AnimationEvent, AnimationEvents, AnimationPlayer, AnimationSystem, App, Camera, Color,
    DrawText, Events, InputState, KeyCode, ShouldQuit, Sprite, System, TextQueue, Transform,
    UvRect, Vec2, WindowConfig, World,
};
use std::path::PathBuf;

const WIN_W: u32 = 640;
const WIN_H: u32 = 420;
const FRAMES: u32 = 4;
const FPS: f32 = 4.0; // 0.25 s/frame — footsteps land ~twice per second
const FLASH_SECS: f32 = 0.5;

/// Writes a 4×1 walk sheet. Frames 1 and 3 are "contact" poses: the body bobs down and a bright
/// ground band appears — exactly the frames the `"footstep"` events fire on.
fn generate_walk_sheet() -> PathBuf {
    use image::{Rgba, RgbaImage};
    const CELL: u32 = 64;
    let mut img = RgbaImage::from_pixel(CELL * FRAMES, CELL, Rgba([20, 22, 30, 255]));
    for f in 0..FRAMES {
        let contact = f % 2 == 1; // frames 1 and 3
        let ox = f * CELL;
        let body_top = if contact { 20 } else { 10 };
        let body = if contact {
            Rgba([235, 180, 70, 255]) // warm = planted
        } else {
            Rgba([120, 200, 235, 255]) // cool = passing / airborne
        };
        for y in body_top..(body_top + 28) {
            for x in 22..42 {
                img.put_pixel(ox + x, y, body);
            }
        }
        if contact {
            for y in 54..60 {
                for x in 8..56 {
                    img.put_pixel(ox + x, y, Rgba([250, 240, 120, 255])); // ground-contact band
                }
            }
        }
    }
    let path = std::env::temp_dir().join("skeleton_animation_events_walk.png");
    img.save(&path).expect("write walk sheet");
    path
}

/// Step counter + flash timer, updated by the reader system.
#[derive(Default)]
struct Steps {
    count: u32,
    flash: f32,
    last_frame: usize,
}

/// Reads `Events<AnimationEvent>` each frame and reacts to `"footstep"` tags.
struct FootstepSystem {
    player: engine::Entity,
}

impl System for FootstepSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        if world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false)
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Collect this frame's footstep events (clone out so the Events borrow ends before we mutate).
        let steps: Vec<usize> = world
            .resource::<Events<AnimationEvent>>()
            .map(|ev| {
                ev.read()
                    .iter()
                    .filter(|e| e.entity == self.player && e.tag == "footstep")
                    .map(|e| e.frame)
                    .collect()
            })
            .unwrap_or_default();

        if let Some(s) = world.resource_mut::<Steps>() {
            if s.flash > 0.0 {
                s.flash = (s.flash - dt).max(0.0);
            }
            for frame in &steps {
                s.count += 1;
                s.flash = FLASH_SECS;
                s.last_frame = *frame;
                println!("footstep! (frame {frame}) total = {}", s.count);
            }
        }

        let cur = world
            .get::<AnimationPlayer>(self.player)
            .map(|p| p.current_frame)
            .unwrap_or(0);
        let (count, flash, last) = world
            .resource::<Steps>()
            .map(|s| (s.count, s.flash, s.last_frame))
            .unwrap_or((0, 0.0, 0));

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "AnimationEvents — fire a tagged event on a frame (footsteps on the contact frames)",
                Vec2::new(20.0, 22.0),
                16.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("current frame: {cur}    footsteps: {count}"),
                Vec2::new(20.0, WIN_H as f32 - 60.0),
                18.0,
                Color::rgb(0.78, 0.86, 0.96),
            ));
            if flash > 0.0 {
                let a = (flash / FLASH_SECS).clamp(0.0, 1.0);
                tq.push(DrawText::new(
                    format!("STEP!  (frame {last})"),
                    Vec2::new(WIN_W as f32 * 0.5 - 70.0, WIN_H as f32 * 0.5 + 120.0),
                    26.0,
                    Color::rgba(0.98, 0.92, 0.45, a),
                ));
            }
            tq.push(DrawText::new(
                "Esc: quit",
                Vec2::new(20.0, WIN_H as f32 - 28.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "footstep_reader"
    }
}

fn main() {
    let sheet_path = generate_walk_sheet();
    let sheet = sheet_path.to_string_lossy().to_string();

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "animation_events — footsteps on the contact frames".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    // Top-left anchored camera so world coords match window pixels.
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<AnimationEvent>();

    let handle = app.load_image(&sheet);
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.5, WIN_H as f32 * 0.5),
            scale: Vec2::splat(220.0),
            ..Default::default()
        },
    );
    app.world.add_component(
        player,
        Sprite::textured_with_handle(sheet.as_str(), Some(handle)),
    );
    app.world.add_component(
        player,
        AnimationPlayer::new(vec![engine::AnimationClip {
            frames: (0..FRAMES)
                .map(|c| UvRect::from_grid(c, 0, FRAMES, 1))
                .collect(),
            fps: FPS,
            looping: true,
        }]),
    );
    // Footsteps fire when the playhead enters the two contact frames (1 and 3).
    app.world.add_component(
        player,
        AnimationEvents::new()
            .on(0, 1, "footstep")
            .on(0, 3, "footstep"),
    );

    app.world.insert_resource(Steps::default());

    // Order matters: the player must advance (and emit) before the reader reads the same frame.
    app.add_system(AnimationSystem::new());
    app.add_system(FootstepSystem { player });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(60); // ≈1 s at 60 fps — enough for the cycle to fire a couple of footsteps
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
