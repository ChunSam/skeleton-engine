//! Audio fade + release-envelope demo.
//!
//! The built-in `AudioSystem` drives `AudioManager::update(dt)` every frame so
//! scheduled fades (including release envelopes) actually progress.
//!
//! `cargo run --example audio_fades`
//!
//! **Fade controls**
//! - Space   : play a sustained tone
//! - F       : fade out over 1.5 s, then stop
//! - 1 / 2 / 3 : fade channel volume to 0.2 / 0.6 / 1.0 over 0.8 s
//!
//! **Release-envelope controls**
//! - R       : play tone with a 1.5 s release envelope configured via `AudioEffect`
//! - S       : stop the release-tone channel — hear the 1.5 s fade-out before silence
//! - I       : stop the release-tone channel *immediately* (bypasses release by
//!   calling `stop()` a second time while the release fade is still active)
//!
//! `fade_out` / `fade_volume` and the release envelope only *schedule* a fade — it
//! advances when `AudioManager::update(dt)` is ticked. `AudioSystem` does that tick;
//! remove it and the fades would freeze at their start volume.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use engine::{
        App, AudioEffect, AudioManager, AudioSystem, DrawText, InputState, KeyCode, System,
        TextQueue, WindowConfig, World,
    };
    use glam::Vec2;

    struct FadeDemoSystem {
        status: String,
        release_status: String,
    }

    impl System for FadeDemoSystem {
        fn run(&mut self, world: &mut World, _dt: f32) {
            let (play, fade_out, v1, v2, v3, play_rel, stop_rel, stop_imm) = {
                let Some(input) = world.resource::<InputState>() else {
                    return;
                };
                (
                    input.just_pressed(KeyCode::Space),
                    input.just_pressed(KeyCode::KeyF),
                    input.just_pressed(KeyCode::Digit1),
                    input.just_pressed(KeyCode::Digit2),
                    input.just_pressed(KeyCode::Digit3),
                    input.just_pressed(KeyCode::KeyR),
                    input.just_pressed(KeyCode::KeyS),
                    input.just_pressed(KeyCode::KeyI),
                )
            };

            if let Some(audio) = world.resource_mut::<AudioManager>() {
                // ── Fade-out / volume fades ───────────────────────────────────
                if play {
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

                // ── Release envelope demo ─────────────────────────────────────
                if play_rel {
                    // Configure a 1.5 s release envelope on the "release_tone" channel.
                    audio.set_effect(
                        "release_tone",
                        AudioEffect {
                            release_secs: 1.5,
                            ..AudioEffect::default()
                        },
                    );
                    // A long tone so it sustains until we stop it.
                    audio.play_tone("release_tone", 520.0, 120.0, 0.5);
                    self.release_status = "playing 520 Hz (release_secs=1.5)".into();
                }
                if stop_rel {
                    // stop() honors release_secs: fades out over 1.5 s then tears down.
                    audio.stop("release_tone");
                    self.release_status = "releasing — fading out over 1.5 s…".into();
                }
                if stop_imm {
                    // A second stop() (or any stop() while already releasing) cuts immediately.
                    audio.stop("release_tone");
                    self.release_status = "immediate stop (overrides release)".into();
                }
            }

            if let Some(tq) = world.resource_mut::<TextQueue>() {
                tq.push(DrawText::new(
                    "Audio fades + release envelope",
                    Vec2::new(20.0, 20.0),
                    22.0,
                    [255, 255, 255, 255],
                ));

                tq.push(DrawText::new(
                    "[ Fade controls ]",
                    Vec2::new(20.0, 58.0),
                    15.0,
                    [200, 200, 120, 255],
                ));
                tq.push(DrawText::new(
                    "Space: play 330 Hz   F: fade out 1.5s   1/2/3: set volume 0.2/0.6/1.0",
                    Vec2::new(20.0, 78.0),
                    14.0,
                    [180, 200, 220, 255],
                ));
                tq.push(DrawText::new(
                    format!("fade status: {}", self.status),
                    Vec2::new(20.0, 98.0),
                    14.0,
                    [160, 255, 160, 255],
                ));

                tq.push(DrawText::new(
                    "[ Release-envelope controls ]",
                    Vec2::new(20.0, 128.0),
                    15.0,
                    [200, 120, 200, 255],
                ));
                tq.push(DrawText::new(
                    "R: play 520 Hz with release_secs=1.5   S: stop (fade 1.5s)   I: immediate stop",
                    Vec2::new(20.0, 148.0),
                    14.0,
                    [180, 200, 220, 255],
                ));
                tq.push(DrawText::new(
                    format!("release status: {}", self.release_status),
                    Vec2::new(20.0, 168.0),
                    14.0,
                    [255, 160, 255, 255],
                ));

                tq.push(DrawText::new(
                    "Press R to start, S to stop with fade, I to cut immediately",
                    Vec2::new(20.0, 200.0),
                    12.0,
                    [140, 140, 160, 255],
                ));
            }
        }

        fn name(&self) -> &'static str {
            "FadeDemoSystem"
        }
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Audio Fades + Release Envelope".into(),
        width: 820,
        height: 240,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });

    match AudioManager::new() {
        Some(audio) => app.world.insert_resource(audio),
        None => eprintln!("No audio device — running silently (fades won't be audible)."),
    }

    // Without this system the scheduled fades and release envelopes never advance.
    app.add_system(AudioSystem);
    app.add_system(FadeDemoSystem {
        status: "press Space to play".into(),
        release_status: "press R to play".into(),
    });
    app.run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // AudioManager / AudioSystem are native-only; nothing to run on wasm.
}
