//! Data-driven animation demo.
//!
//! The sprite's animation clips are defined entirely in `assets/clips.ron` — NOT in code.
//! `App::load_animation_clips` loads them into an `AnimationClipRegistry` (hot-reloaded via
//! the asset watcher). Press `1`/`2`/`3` to switch clips by name. **Edit `clips.ron`** (change
//! an `fps` or a `frames` list) while it runs and the animation updates live — a re-sync system
//! rebuilds the `AnimationPlayer` from the registry whenever the loaded clips change.
//!
//! Engine convention: camera position `(0,0)` = viewport TOP-LEFT (Y down); placed at positive coords.

use engine::{
    AnimationClip, AnimationClipRegistry, AnimationPlayer, AnimationSystem, App, DrawText, Entity,
    InputState, KeyCode, Sprite, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

const ANIM: &str = "examples/games/data_anim/assets/anim.png";
const CLIPS: &str = "examples/games/data_anim/assets/clips.ron";
const WIN_W: u32 = 480;
const WIN_H: u32 = 360;

struct DemoState {
    sprite: Entity,
    names: Vec<String>, // clip names (sorted, aligned with clip indices)
    current: usize,
    /// Cheap signature of the loaded clips; when it changes (hot-reload), re-sync the player.
    sig: u64,
}

/// Signature over the registry's clips so we can detect a hot-reload.
fn clips_sig(clips: &[AnimationClip]) -> u64 {
    clips.iter().fold(0u64, |acc, c| {
        acc.wrapping_mul(31)
            .wrapping_add(c.frames.len() as u64)
            .wrapping_mul(31)
            .wrapping_add((c.fps * 100.0) as u64)
            .wrapping_mul(2)
            .wrapping_add(c.looping as u64)
    })
}

fn hero_clips(world: &World) -> Option<(Vec<AnimationClip>, Vec<String>)> {
    let reg = world.resource::<AnimationClipRegistry>()?;
    let set = reg.get("hero")?;
    Some((
        set.clips().to_vec(),
        set.names().map(|s| s.to_string()).collect(),
    ))
}

struct DataAnimSystem;

impl System for DataAnimSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (k1, k2, k3) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Digit1),
                    i.just_pressed(KeyCode::Digit2),
                    i.just_pressed(KeyCode::Digit3),
                )
            })
            .unwrap_or((false, false, false));

        let (sprite, names, current, sig) = match world.resource::<DemoState>() {
            Some(s) => (s.sprite, s.names.clone(), s.current, s.sig),
            None => return,
        };

        // Pull the registry's current clips (owned) before touching components.
        let Some((clips, new_names)) = hero_clips(world) else {
            return;
        };
        let new_sig = clips_sig(&clips);
        let reloaded = new_sig != sig;

        // Pick the target clip index: a key press, else keep current (clamped to the new set).
        let mut idx = current.min(names.len().saturating_sub(1));
        if k1 {
            idx = 0;
        } else if k2 && new_names.len() > 1 {
            idx = 1;
        } else if k3 && new_names.len() > 2 {
            idx = 2;
        }
        idx = idx.min(new_names.len().saturating_sub(1));
        let changed_clip = idx != current;

        // On a hot-reload, rebuild the player from the new clips (preserve the selected clip).
        // On a key press, just switch the clip on the existing player.
        if reloaded {
            if let Some(p) = world.get_mut::<AnimationPlayer>(sprite) {
                *p = AnimationPlayer::new(clips.clone());
                p.play(idx);
            }
        } else if changed_clip {
            if let Some(p) = world.get_mut::<AnimationPlayer>(sprite) {
                p.play(idx);
            }
        }

        if reloaded || changed_clip {
            if let Some(s) = world.resource_mut::<DemoState>() {
                s.current = idx;
                s.names = new_names.clone();
                s.sig = new_sig;
            }
        }

        // HUD.
        let cur_name = new_names.get(idx).cloned().unwrap_or_default();
        let cur_fps = clips.get(idx).map(|c| c.fps).unwrap_or(0.0);
        let cur_frames = clips.get(idx).map(|c| c.frames.len()).unwrap_or(0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Data-driven animation (clips from RON)",
                Vec2::new(12.0, 10.0),
                18.0,
                [255, 230, 120, 255],
            ));
            tq.push(DrawText::new(
                format!("clip: {cur_name}   fps: {cur_fps}   frames: {cur_frames}"),
                Vec2::new(12.0, 36.0),
                15.0,
                [180, 255, 180, 255],
            ));
            tq.push(DrawText::new(
                "1/2/3: switch clip    edit assets/clips.ron to hot-reload",
                Vec2::new(12.0, WIN_H as f32 - 26.0),
                14.0,
                [180, 200, 230, 230],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "DataAnimSystem"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Data-Driven Animation".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.world
        .insert_resource(engine::Camera::new(Vec2::ZERO, 1.0));

    app.load_atlas(ANIM, 4, 4);
    app.load_animation_clips("hero", CLIPS);

    // Build the player from the RON-loaded clips (idle = index 0; clips sort alphabetically).
    let (clips, names) = hero_clips(&app.world).expect("clips.ron failed to load");
    let sig = clips_sig(&clips);
    let mut player = AnimationPlayer::new(clips);
    player.play(0);

    let sprite = app.world.spawn();
    app.world.add_component(
        sprite,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.5, WIN_H as f32 * 0.5 + 16.0),
            scale: Vec2::splat(112.0),
            z: 0.0,
            ..Default::default()
        },
    );
    app.world.add_component(sprite, Sprite::textured(ANIM));
    app.world.add_component(sprite, player);

    app.world.insert_resource(DemoState {
        sprite,
        names,
        current: 0,
        sig,
    });

    app.add_system(AnimationSystem::new());
    app.add_system(DataAnimSystem);
    app.run();
}
