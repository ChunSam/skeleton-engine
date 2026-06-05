//! Audio fade demo — the built-in `AudioSystem` drives `AudioManager::update(dt)`
//! every frame so scheduled fades actually progress (issue #20).
//!
//! `cargo run --example audio_fades`
//!
//! - Space   : play a sustained tone
//! - F       : fade out over 1.5 s, then stop
//! - 1 / 2 / 3 : fade channel volume to 0.2 / 0.6 / 1.0 over 0.8 s
//!
//! `fade_out` / `fade_volume` only *schedule* a fade — it advances when
//! `AudioManager::update(dt)` is ticked. `AudioSystem` does that tick; remove it
//! and the fades would freeze at their start volume.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use engine::{
        App, AudioManager, AudioSystem, DrawText, InputState, KeyCode, System, TextQueue,
        WindowConfig, World,
    };
    use glam::Vec2;

    struct FadeDemoSystem {
        status: String,
    }

    impl System for FadeDemoSystem {
        fn run(&mut self, world: &mut World, _dt: f32) {
            let (play, fade_out, v1, v2, v3) = {
                let Some(input) = world.resource::<InputState>() else {
                    return;
                };
                (
                    input.just_pressed(KeyCode::Space),
                    input.just_pressed(KeyCode::KeyF),
                    input.just_pressed(KeyCode::Digit1),
                    input.just_pressed(KeyCode::Digit2),
                    input.just_pressed(KeyCode::Digit3),
                )
            };

            if let Some(audio) = world.resource_mut::<AudioManager>() {
                if play {
                    // A long tone so it sustains for the whole demo.
                    audio.play_tone("tone", 330.0, 120.0, 0.5);
                    self.status = "playing 330 Hz tone".into();
                }
                if fade_out {
                    audio.fade_out("tone", 1.5);
                    self.status = "fading out (1.5 s) then stop".into();
                }
                if v1 {
                    audio.fade_volume("tone", 0.2, 0.8);
                    self.status = "fading volume -> 0.2".into();
                }
                if v2 {
                    audio.fade_volume("tone", 0.6, 0.8);
                    self.status = "fading volume -> 0.6".into();
                }
                if v3 {
                    audio.fade_volume("tone", 1.0, 0.8);
                    self.status = "fading volume -> 1.0".into();
                }
            }

            if let Some(tq) = world.resource_mut::<TextQueue>() {
                tq.push(DrawText::new(
                    "Audio fades - AudioSystem drives update(dt)",
                    Vec2::new(20.0, 24.0),
                    22.0,
                    [255, 255, 255, 255],
                ));
                tq.push(DrawText::new(
                    "Space: play   F: fade out   1/2/3: fade volume 0.2/0.6/1.0",
                    Vec2::new(20.0, 60.0),
                    16.0,
                    [180, 200, 220, 255],
                ));
                tq.push(DrawText::new(
                    format!("status: {}", self.status),
                    Vec2::new(20.0, 92.0),
                    16.0,
                    [160, 255, 160, 255],
                ));
            }
        }

        fn name(&self) -> &'static str {
            "FadeDemoSystem"
        }
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Audio Fades - AudioSystem".into(),
        width: 720,
        height: 240,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });

    match AudioManager::new() {
        Some(audio) => app.world.insert_resource(audio),
        None => eprintln!("No audio device — running silently (fades won't be audible)."),
    }

    // Without this system the scheduled fades never advance.
    app.add_system(AudioSystem);
    app.add_system(FadeDemoSystem {
        status: "press Space to play".into(),
    });
    app.run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // AudioManager / AudioSystem are native-only; nothing to run on wasm.
}
