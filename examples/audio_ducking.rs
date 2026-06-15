//! Bus ducking / sidechain mixing demo.
//!
//! Demonstrates automatic bus ducking via [`AudioManager::set_sidechain`]: a music
//! channel on bus "music" ducks down while a voice/blip on bus "voice" is playing,
//! then releases back to full volume once the blip finishes.
//!
//! The live `bus_duck("music")` multiplier is displayed on screen so the ducking
//! is visually verifiable even without audio output.
//!
//! `cargo run --example audio_ducking`
//!
//! **Controls**
//! - Space : play a voice blip → music bus ducks to 0.25, then releases (0.6 s)
//! - M     : toggle music playback (press once to start, once to stop)
//! - D     : manually duck the music bus to 0.1 immediately (0.3 s attack)
//! - R     : release the music duck manually (0.8 s release)

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use engine::{
        App, AudioManager, AudioSystem, DrawText, InputState, KeyCode, System, TextQueue,
        WindowConfig, World,
    };
    use glam::Vec2;

    struct DuckDemoSystem {
        music_playing: bool,
        status: String,
        duck_status: String,
    }

    impl System for DuckDemoSystem {
        fn run(&mut self, world: &mut World, _dt: f32) {
            let (space, key_m, key_d, key_r) = {
                let Some(input) = world.resource::<InputState>() else {
                    return;
                };
                (
                    input.just_pressed(KeyCode::Space),
                    input.just_pressed(KeyCode::KeyM),
                    input.just_pressed(KeyCode::KeyD),
                    input.just_pressed(KeyCode::KeyR),
                )
            };

            let (duck_val, sfx_duck_val);
            if let Some(audio) = world.resource_mut::<AudioManager>() {
                // ── Music toggle ──────────────────────────────────────────────
                if key_m {
                    if self.music_playing {
                        audio.stop("music");
                        self.music_playing = false;
                        self.status = "music stopped".into();
                    } else {
                        // Assign channel to "music" bus and play a looping tone.
                        audio.assign_bus("music", "music");
                        audio.play_tone("music", 220.0, 3600.0, 0.5);
                        self.music_playing = true;
                        self.status = "music playing (220 Hz loop)".into();
                    }
                }

                // ── Voice blip → triggers sidechain duck ──────────────────────
                if space {
                    // Assign the blip channel to the "voice" bus (sidechain trigger).
                    audio.assign_bus("blip", "voice");
                    // A short 0.3 s tone to simulate a voice/SFX blip.
                    audio.play_tone("blip", 660.0, 1.2, 0.6);
                    self.duck_status = "voice blip playing → music ducking…".into();
                }

                // ── Manual duck / release ─────────────────────────────────────
                // Manual duck targets a SEPARATE "sfx" bus — a sidechained bus
                // ("music") is owned by its sidechain and would override a manual duck.
                if key_d {
                    audio.duck_bus("sfx", 0.1, 0.3);
                    self.duck_status = "manual duck → sfx: 0.1 (0.3 s attack)".into();
                }
                if key_r {
                    audio.release_bus("sfx", 0.8);
                    self.duck_status = "manual release → sfx: 1.0 (0.8 s)".into();
                }

                duck_val = audio.bus_duck("music");
                sfx_duck_val = audio.bus_duck("sfx");

                // Update duck status when the blip finishes and duck is releasing.
                if !audio.is_playing("blip") && (duck_val - 1.0).abs() > 0.05 {
                    self.duck_status = format!("releasing… music duck: {duck_val:.3}");
                } else if !audio.is_playing("blip")
                    && (duck_val - 1.0).abs() <= 0.05
                    && (self.duck_status.starts_with("voice blip")
                        || self.duck_status.starts_with("releasing"))
                {
                    self.duck_status = "duck released — music at full volume".into();
                }
            } else {
                duck_val = 1.0;
                sfx_duck_val = 1.0;
            }

            if let Some(tq) = world.resource_mut::<TextQueue>() {
                tq.push(DrawText::new(
                    "Audio Ducking / Sidechain Demo",
                    Vec2::new(20.0, 20.0),
                    22.0,
                    [255, 255, 255, 255],
                ));

                tq.push(DrawText::new(
                    "[ Controls ]",
                    Vec2::new(20.0, 58.0),
                    15.0,
                    [200, 200, 120, 255],
                ));
                tq.push(DrawText::new(
                    "M: toggle music   Space: voice blip (triggers sidechain duck)",
                    Vec2::new(20.0, 78.0),
                    14.0,
                    [180, 200, 220, 255],
                ));
                tq.push(DrawText::new(
                    "D: manual duck sfx bus   R: release sfx bus",
                    Vec2::new(20.0, 98.0),
                    14.0,
                    [180, 200, 220, 255],
                ));

                tq.push(DrawText::new(
                    format!("music status: {}", self.status),
                    Vec2::new(20.0, 130.0),
                    14.0,
                    [160, 255, 160, 255],
                ));
                tq.push(DrawText::new(
                    format!("duck status: {}", self.duck_status),
                    Vec2::new(20.0, 150.0),
                    14.0,
                    [255, 220, 100, 255],
                ));

                // Live duck readout — the key visual indicator.
                let duck_color = if duck_val < 0.5 {
                    [255, 100, 100, 255]
                } else if duck_val < 0.9 {
                    [255, 200, 80, 255]
                } else {
                    [100, 255, 100, 255]
                };
                tq.push(DrawText::new(
                    format!("music duck: {duck_val:.3}"),
                    Vec2::new(20.0, 180.0),
                    20.0,
                    duck_color,
                ));

                let sfx_color = if sfx_duck_val < 0.5 {
                    [255, 100, 100, 255]
                } else if sfx_duck_val < 0.9 {
                    [255, 200, 80, 255]
                } else {
                    [100, 255, 100, 255]
                };
                tq.push(DrawText::new(
                    format!("sfx duck (manual): {sfx_duck_val:.3}"),
                    Vec2::new(20.0, 206.0),
                    16.0,
                    sfx_color,
                ));

                tq.push(DrawText::new(
                    "Sidechain: voice bus active → music ducks to 0.25 (0.15 s attack, 0.6 s release)",
                    Vec2::new(20.0, 236.0),
                    12.0,
                    [140, 140, 160, 255],
                ));
            }
        }

        fn name(&self) -> &'static str {
            "DuckDemoSystem"
        }
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Audio Ducking / Sidechain Demo".into(),
        width: 860,
        height: 300,
        clear_color: [0.05, 0.07, 0.11, 1.0],
    });

    match AudioManager::new() {
        Some(mut audio) => {
            // Register the sidechain: whenever "voice" bus has audio playing,
            // duck the "music" bus to 0.25 (attack 0.15 s, release 0.6 s).
            audio.set_sidechain("voice", "music", 0.25, 0.15, 0.6);
            app.world.insert_resource(audio);
        }
        None => eprintln!("No audio device — running silently (duck value will stay at 1.0)."),
    }

    // AudioSystem ticks AudioManager::update(dt) each frame, which drives both
    // fade progression and sidechain/duck animation.
    app.add_system(AudioSystem);
    app.add_system(DuckDemoSystem {
        music_playing: false,
        status: "press M to start".into(),
        duck_status: "no duck active".into(),
    });
    app.run();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // AudioManager / AudioSystem are native-only; nothing to run on wasm.
}
