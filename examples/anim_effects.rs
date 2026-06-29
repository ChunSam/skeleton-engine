//! Data-driven Animation→Effect bindings — fire RON-authored effects on tagged animation frames.
//!
//! The companion to `examples/zone_effects.rs`: there a zone overlap fires effects, here an
//! animation frame does. This plays the same 4-frame walk cycle as `examples/animation_events.rs`,
//! whose two **contact** frames (1 and 3) carry a `"footstep"` [`AnimationEvent`]. Instead of the
//! game wiring the reaction in Rust, an [`AnimEffectSystem`](engine::AnimEffectSystem) reads each
//! event and runs the [`anim_effects.ron`](./anim_effects.ron) bindings for its tag — a dust puff
//! (visuals from [`anim_effects_particles.ron`](./anim_effects_particles.ron)), a soft click tone,
//! and a quick warm flash on the walker. The animation, the particles, and the reactions are all
//! authored in RON, composed by the frame tag, with **zero reaction glue in code**.
//!
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/anim_effects.png cargo run --example anim_effects`): runs 70 warmup
//! frames so the cycle fires a couple of footsteps; `HEADLESS_FRAMES=N` overrides.
use engine::{
    AnimEffectSystem, AnimationEvent, AnimationEvents, AnimationPlayer, AnimationSystem, App,
    Audio, AudioFacadeSystem, Camera, Color, DrawText, Entity, Events, HitFlashSystem, InputState,
    KeyCode, ParticleSystem, ShouldQuit, Sprite, System, TextQueue, Transform, UvRect, Vec2,
    WindowConfig, World,
};
use std::path::PathBuf;

const WIN_W: u32 = 640;
const WIN_H: u32 = 420;
const FRAMES: u32 = 4;
const FPS: f32 = 4.0; // 0.25 s/frame — footsteps land ~twice per second

/// Writes a 4×1 walk sheet (same as `animation_events`). Frames 1 and 3 are "contact" poses — the
/// body bobs down and a ground band appears — exactly the frames the `"footstep"` events fire on.
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
            Rgba([120, 200, 235, 255]) // cool = passing
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
    let path = std::env::temp_dir().join("skeleton_anim_effects_walk.png");
    img.save(&path).expect("write walk sheet");
    path
}

/// HUD + step counter + Esc handling. The effects themselves are applied by `AnimEffectSystem`
/// before this runs — this only reports.
struct Hud {
    player: Entity,
    steps: u32,
}

impl System for Hud {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if world
            .resource::<InputState>()
            .map(|i| i.just_pressed(KeyCode::Escape))
            .unwrap_or(false)
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Count this frame's footsteps for the readout (AnimEffectSystem already fired their effects).
        let new_steps = world
            .resource::<Events<AnimationEvent>>()
            .map(|ev| {
                ev.read()
                    .iter()
                    .filter(|e| e.entity == self.player && e.tag == "footstep")
                    .count()
            })
            .unwrap_or(0);
        self.steps += new_steps as u32;
        for _ in 0..new_steps {
            println!("footstep → fired its effects  (total: {})", self.steps);
        }

        let cur = world
            .get::<AnimationPlayer>(self.player)
            .map(|p| p.current_frame)
            .unwrap_or(0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Animation → Effect bindings — dust + click + flash on the contact frames, all in RON",
                Vec2::new(20.0, 22.0),
                15.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!("current frame: {cur}    footsteps: {}", self.steps),
                Vec2::new(20.0, WIN_H as f32 - 52.0),
                17.0,
                Color::rgb(0.78, 0.86, 0.96),
            ));
            tq.push(DrawText::new(
                "Esc: quit    (edit anim_effects.ron to retune the footstep reaction)",
                Vec2::new(20.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "anim_effects_hud"
    }
}

fn main() {
    let sheet_path = generate_walk_sheet();
    let sheet = sheet_path.to_string_lossy().to_string();

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "anim_effects — RON effects on tagged animation frames".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<AnimationEvent>();

    // Data-driven: the particle emitter the footstep references, and the tag→effects bindings.
    app.load_particle_configs("fx", "examples/anim_effects_particles.ron");
    app.load_anim_effects("effects", "examples/anim_effects.ron");

    let handle = app.load_image(&sheet);
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.5, WIN_H as f32 * 0.5),
            scale: Vec2::splat(180.0),
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

    // Audio is optional: skip it cleanly when there's no device (headless / locked session). The
    // PlayTone effect no-ops without an `Audio` resource.
    if let Some(audio) = Audio::new() {
        app.world.insert_resource(audio);
        app.add_system(AudioFacadeSystem);
    }

    // Order: animation advances + emits → anim-effects apply → HUD reports → particle/flash systems
    // advance the spawned bursts + the flash.
    app.add_system(AnimationSystem::new());
    app.add_system(AnimEffectSystem::new("effects"));
    app.add_system(Hud { player, steps: 0 });
    app.add_system(ParticleSystem);
    app.add_system(HitFlashSystem);

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(70);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
