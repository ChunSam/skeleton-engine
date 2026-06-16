//! Music crossfade demo.
//!
//! Demonstrates [`AudioManager::crossfade`]: the old track fades out while the
//! new one fades in simultaneously, producing a smooth overlap.
//!
//! `cargo run --example music_crossfade`
//!
//! **Controls**
//! - Space : start Track A (looping)
//! - X     : crossfade to the alternate track (A ↔ B) over 2 s
//! - S     : stop all playback immediately
//!
//! **How it works:**
//! `audio.crossfade("bgm", path, true, 2.0)` moves the current sink to an
//! internal `bgm__xfade` temporary channel (fade-out) and starts the new track
//! on `bgm` (fade-in). `AudioSystem` drives both fades forward each frame and
//! tears down `bgm__xfade` when the fade-out completes.
//!
//! **Note:** no audio files ship with this project, so two tiny sine-wave WAV
//! files are written to the system temp directory at startup and removed on exit.
//! They produce an audible tone difference (220 Hz vs. 330 Hz) so the crossfade
//! is perceptible through headphones.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use engine::{
        App, AudioManager, AudioSystem, DrawText, InputState, KeyCode, System, TextQueue,
        WindowConfig, World,
    };
    use glam::Vec2;
    use std::path::PathBuf;

    // ── Generate a minimal PCM WAV file at `path` ─────────────────────────────

    /// Write a 1-second mono 44 100 Hz 16-bit sine-wave WAV to `path`.
    ///
    /// Returns `false` if the file could not be written (demo will run silently).
    fn write_sine_wav(path: &PathBuf, freq_hz: f32) -> bool {
        let sample_rate: u32 = 44_100;
        let duration_secs: u32 = 2; // long enough to hear the crossfade
        let num_samples = sample_rate * duration_secs;

        // Build 16-bit PCM samples.
        let mut pcm: Vec<u8> = Vec::with_capacity((num_samples * 2) as usize);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (0.4 * (2.0 * std::f32::consts::PI * freq_hz * t).sin()) as f64;
            let s16 = (sample * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16;
            pcm.extend_from_slice(&s16.to_le_bytes());
        }

        let data_len = pcm.len() as u32;
        let file_len = 36 + data_len;
        let byte_rate = sample_rate * 2; // mono × 2 bytes/sample
        let mut wav: Vec<u8> = Vec::with_capacity(44 + pcm.len());
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&file_len.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM = 1
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&pcm);

        std::fs::write(path, wav).is_ok()
    }

    // ── Temp audio assets ──────────────────────────────────────────────────────
    let tmp_dir = std::env::temp_dir().join(format!(
        "skeleton-engine-music-crossfade-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp_dir).expect("could not create temp dir for audio assets");

    let track_a = tmp_dir.join("track_a.wav");
    let track_b = tmp_dir.join("track_b.wav");
    let a_ok = write_sine_wav(&track_a, 220.0); // Track A — 220 Hz
    let b_ok = write_sine_wav(&track_b, 330.0); // Track B — 330 Hz
    if !a_ok || !b_ok {
        eprintln!("Could not write temp audio files — demo will run silently.");
    }

    let track_a_str = track_a.to_str().unwrap_or("").to_string();
    let track_b_str = track_b.to_str().unwrap_or("").to_string();
    let tmp_dir_cleanup = tmp_dir.clone();

    // ── Demo system ────────────────────────────────────────────────────────────

    #[derive(Clone, Copy, PartialEq)]
    enum Track {
        A,
        B,
        None,
    }

    struct CrossfadeDemoSystem {
        current: Track,
        track_a: String,
        track_b: String,
        status: String,
        xfade_timer: f32,
    }

    impl CrossfadeDemoSystem {
        const DUR: f32 = 2.0;
    }

    impl System for CrossfadeDemoSystem {
        fn run(&mut self, world: &mut World, dt: f32) {
            if self.xfade_timer > 0.0 {
                self.xfade_timer = (self.xfade_timer - dt).max(0.0);
            }

            let (play, crossfade, stop) = {
                let Some(input) = world.resource::<InputState>() else {
                    return;
                };
                (
                    input.just_pressed(KeyCode::Space),
                    input.just_pressed(KeyCode::KeyX),
                    input.just_pressed(KeyCode::KeyS),
                )
            };

            let xfade_sink_exists;
            if let Some(audio) = world.resource_mut::<AudioManager>() {
                if play && self.current == Track::None {
                    audio.play_fade_in("bgm", &self.track_a, true, 0.5);
                    self.current = Track::A;
                    self.status = "Playing Track A (220 Hz)".into();
                }

                if crossfade && self.current != Track::None {
                    let (next_track, next_path, label) = match self.current {
                        Track::A => (Track::B, self.track_b.clone(), "Track B (330 Hz)"),
                        _ => (Track::A, self.track_a.clone(), "Track A (220 Hz)"),
                    };

                    // ── THE FEATURE: one-call track-to-track crossfade ──────
                    audio.crossfade("bgm", &next_path, true, Self::DUR);

                    self.current = next_track;
                    self.xfade_timer = Self::DUR;
                    self.status = format!("Crossfading → {label} over {:.0} s…", Self::DUR);
                }

                if stop {
                    audio.stop("bgm");
                    audio.stop("bgm__xfade");
                    self.current = Track::None;
                    self.xfade_timer = 0.0;
                    self.status = "Stopped".into();
                }

                xfade_sink_exists = audio.is_playing("bgm__xfade");
            } else {
                xfade_sink_exists = false;
            }

            if let Some(tq) = world.resource_mut::<TextQueue>() {
                tq.push(DrawText::new(
                    "Music Crossfade Demo",
                    Vec2::new(20.0, 20.0),
                    24.0,
                    [255, 255, 255, 255],
                ));
                tq.push(DrawText::new(
                    "Space: play Track A    X: crossfade A<->B    S: stop",
                    Vec2::new(20.0, 58.0),
                    14.0,
                    [180, 200, 220, 255],
                ));
                tq.push(DrawText::new(
                    format!("Status: {}", self.status),
                    Vec2::new(20.0, 88.0),
                    16.0,
                    [160, 255, 160, 255],
                ));
                if self.xfade_timer > 0.0 {
                    tq.push(DrawText::new(
                        format!("Crossfade: {:.1} s remaining", self.xfade_timer),
                        Vec2::new(20.0, 114.0),
                        14.0,
                        [255, 220, 100, 255],
                    ));
                }
                if xfade_sink_exists {
                    tq.push(DrawText::new(
                        "bgm__xfade channel active — old track fading out",
                        Vec2::new(20.0, 134.0),
                        13.0,
                        [200, 140, 255, 255],
                    ));
                }
                tq.push(DrawText::new(
                    "audio.crossfade(\"bgm\", path, true, 2.0)",
                    Vec2::new(20.0, 192.0),
                    12.0,
                    [120, 160, 220, 255],
                ));
            }
        }

        fn name(&self) -> &'static str {
            "CrossfadeDemoSystem"
        }
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Music Crossfade".into(),
        width: 720,
        height: 240,
        clear_color: [0.05, 0.06, 0.10, 1.0],
    });

    match AudioManager::new() {
        Some(audio) => app.world.insert_resource(audio),
        None => eprintln!("No audio device — running silently."),
    }

    app.add_system(AudioSystem);
    app.add_system(CrossfadeDemoSystem {
        current: Track::None,
        track_a: track_a_str,
        track_b: track_b_str,
        status: "Press Space to start Track A".into(),
        xfade_timer: 0.0,
    });
    app.run();

    // Clean up temp audio files.
    std::fs::remove_dir_all(&tmp_dir_cleanup).ok();
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // AudioManager / AudioSystem are native-only; nothing to run on wasm.
}
