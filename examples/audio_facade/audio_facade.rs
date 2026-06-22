//! Cross-platform audio facade demo — the **same code** plays audio on native **and** web.
//!
//! The engine's two audio backends (`AudioManager` on native, `WebAudio` on wasm) have different
//! shapes, so a dual-target game used to write every audio call twice behind `#[cfg]` guards. The
//! [`Audio`](engine::Audio) facade collapses that: the entire [`run`] function below — window setup,
//! the demo system, every audio call — carries **zero `cfg` guards**. Only the platform entry point
//! (`fn main` vs the wasm `#[wasm_bindgen]` export) is cfg-split, the standard winit boilerplate.
//!
//! All clips are sine tones generated in memory ([`sine_wav`]) so the example needs no audio asset
//! and behaves identically on both platforms.
//!
//! Keys (every press also calls [`Audio::resume`], which unlocks audio on web after the first
//! gesture and is a no-op on native):
//! * `1` `2` `3` — one-shot SFX at three pitches ([`play_sfx`](engine::Audio::play_sfx))
//! * `M` — start looping music · `C` — crossfade to another loop · `X` — stop music
//! * `↑` `↓` — master volume ([`set_master_volume`](engine::Audio::set_master_volume))
//! * `B` — play a 4 s sustained pad on the **"bed"** bus ([`play_sfx_on_bus`](engine::Audio::play_sfx_on_bus))
//! * `[` `]` — "bed" bus volume ([`set_bus_volume`](engine::Audio::set_bus_volume))
//! * `D` — duck the "bed" bus · `F` — release it ([`duck_bus`](engine::Audio::duck_bus) /
//!   [`release_bus`](engine::Audio::release_bus))
//! * `Esc` — quit (native)
//!
//! Run natively: `cargo run --example audio_facade`
//!
//! Run in the browser:
//! ```text
//! examples/audio_facade/web/build.sh
//! python3 -m http.server 8080 --directory examples/audio_facade/web
//! open http://localhost:8080        # click "Start", then press the keys
//! ```

use engine::{
    App, Audio, AudioFacadeSystem, DrawText, FontData, InputState, KeyCode, ShouldQuit, System,
    TextQueue, Vec2, WindowConfig, World,
};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

/// Bundled deterministic Latin font so the demo's text metrics match across platforms.
const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/DejaVuSans.ttf"
));

const TITLE: [u8; 4] = [235, 235, 245, 255];
const LEGEND: [u8; 4] = [180, 200, 220, 255];
const HEAD: [u8; 4] = [200, 200, 120, 255];
const VALUE: [u8; 4] = [160, 255, 170, 255];

/// Builds a 16-bit mono PCM WAV of a `freq`-Hz sine wave `secs` long, in memory — so the example
/// needs no audio asset and both rodio (native) and `decode_audio_data` (web) have real bytes to
/// decode. Amplitude is kept modest so layered sounds don't clip.
fn sine_wav(freq: f32, secs: f32) -> Vec<u8> {
    use core::f32::consts::TAU;
    let sample_rate: u32 = 44_100;
    let bytes_per_sample: u32 = 2;
    let n = (sample_rate as f32 * secs) as u32;
    let data_len = n * bytes_per_sample;

    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    v.extend_from_slice(&1u16.to_le_bytes()); // format = PCM
    v.extend_from_slice(&1u16.to_le_bytes()); // channels = mono
    v.extend_from_slice(&sample_rate.to_le_bytes());
    v.extend_from_slice(&(sample_rate * bytes_per_sample).to_le_bytes()); // byte rate
    v.extend_from_slice(&(bytes_per_sample as u16).to_le_bytes()); // block align
    v.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..n {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * freq * TAU).sin() * 0.25;
        let q = (sample * i16::MAX as f32) as i16;
        v.extend_from_slice(&q.to_le_bytes());
    }
    v
}

/// Demo system: maps key presses to facade calls and draws a live legend + readouts. Holds the
/// generated clips (built once) and the UI source-of-truth volumes.
struct AudioDemo {
    sfx: [Vec<u8>; 3],
    music_a: Vec<u8>,
    music_b: Vec<u8>,
    bed: Vec<u8>,
    master_vol: f32,
    bed_vol: f32,
    bed_duck: f32,
    status: String,
}

impl Default for AudioDemo {
    fn default() -> Self {
        Self {
            // A4, D5, G5 one-shots.
            sfx: [
                sine_wav(440.0, 0.25),
                sine_wav(587.0, 0.25),
                sine_wav(784.0, 0.25),
            ],
            music_a: sine_wav(110.0, 1.0), // low loop bed
            music_b: sine_wav(165.0, 1.0), // crossfade target
            bed: sine_wav(220.0, 4.0),     // sustained pad on the "bed" bus
            master_vol: 1.0,
            bed_vol: 1.0,
            bed_duck: 1.0,
            status: "press a key…".into(),
        }
    }
}

impl System for AudioDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Read all key edges in one immutable input borrow ──────────────────
        let (s1, s2, s3, music, xfade, stop, vol_up, vol_dn, bed, bus_dn, bus_up, duck, rel, quit) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.just_pressed(KeyCode::Digit1),
                input.just_pressed(KeyCode::Digit2),
                input.just_pressed(KeyCode::Digit3),
                input.just_pressed(KeyCode::KeyM),
                input.just_pressed(KeyCode::KeyC),
                input.just_pressed(KeyCode::KeyX),
                input.just_pressed(KeyCode::ArrowUp),
                input.just_pressed(KeyCode::ArrowDown),
                input.just_pressed(KeyCode::KeyB),
                input.just_pressed(KeyCode::BracketLeft),
                input.just_pressed(KeyCode::BracketRight),
                input.just_pressed(KeyCode::KeyD),
                input.just_pressed(KeyCode::KeyF),
                input.just_pressed(KeyCode::Escape),
            )
        };
        let any = s1
            || s2
            || s3
            || music
            || xfade
            || stop
            || vol_up
            || vol_dn
            || bed
            || bus_dn
            || bus_up
            || duck
            || rel;

        // ── Drive the facade (one mutable borrow) ─────────────────────────────
        if let Some(audio) = world.resource_mut::<Audio>() {
            // First key on web unlocks the AudioContext; no-op on native.
            if any {
                audio.resume();
            }
            if s1 {
                audio.play_sfx(&self.sfx[0]);
                self.status = "play_sfx (440 Hz)".into();
            }
            if s2 {
                audio.play_sfx(&self.sfx[1]);
                self.status = "play_sfx (587 Hz)".into();
            }
            if s3 {
                audio.play_sfx(&self.sfx[2]);
                self.status = "play_sfx (784 Hz)".into();
            }
            if music {
                audio.play_music(&self.music_a);
                self.status = "play_music (110 Hz loop)".into();
            }
            if xfade {
                audio.crossfade_music(&self.music_b, 1.5);
                self.status = "crossfade_music -> 165 Hz (1.5 s)".into();
            }
            if stop {
                audio.stop_music();
                self.status = "stop_music".into();
            }
            if vol_up || vol_dn {
                self.master_vol =
                    (self.master_vol + if vol_up { 0.1 } else { -0.1 }).clamp(0.0, 1.0);
                audio.set_master_volume(self.master_vol);
                self.status = format!("set_master_volume {:.1}", self.master_vol);
            }
            if bed {
                audio.play_sfx_on_bus(&self.bed, "bed");
                self.status = "play_sfx_on_bus 220 Hz pad -> \"bed\"".into();
            }
            if bus_dn || bus_up {
                self.bed_vol = (self.bed_vol + if bus_up { 0.1 } else { -0.1 }).clamp(0.0, 1.0);
                audio.set_bus_volume("bed", self.bed_vol);
                self.status = format!("set_bus_volume(\"bed\", {:.1})", self.bed_vol);
            }
            if duck {
                audio.duck_bus("bed", 0.2, 0.15);
                self.status = "duck_bus(\"bed\", 0.2)".into();
            }
            if rel {
                audio.release_bus("bed", 0.4);
                self.status = "release_bus(\"bed\")".into();
            }
            // Live readouts (note: bus_volume/bus_duck reflect the backend, not just our copy).
            self.bed_vol = audio.bus_volume("bed");
            self.bed_duck = audio.bus_duck("bed");
        }

        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        // ── Draw legend + readouts ────────────────────────────────────────────
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let x = 24.0;
            tq.push(DrawText::new(
                "Cross-platform audio facade — same code, native + web",
                Vec2::new(x, 22.0),
                22.0,
                TITLE,
            ));
            tq.push(DrawText::new(
                "[ one-shot SFX ]",
                Vec2::new(x, 64.0),
                15.0,
                HEAD,
            ));
            tq.push(DrawText::new(
                "1 / 2 / 3 : play_sfx (440 / 587 / 784 Hz)",
                Vec2::new(x, 84.0),
                14.0,
                LEGEND,
            ));
            tq.push(DrawText::new("[ music ]", Vec2::new(x, 116.0), 15.0, HEAD));
            tq.push(DrawText::new(
                "M : play loop    C : crossfade    X : stop",
                Vec2::new(x, 136.0),
                14.0,
                LEGEND,
            ));
            tq.push(DrawText::new("[ mixer ]", Vec2::new(x, 168.0), 15.0, HEAD));
            tq.push(DrawText::new(
                "Up/Down : master vol    B : pad on \"bed\" bus    [ / ] : bed vol    D/F : duck/release",
                Vec2::new(x, 188.0),
                14.0,
                LEGEND,
            ));
            tq.push(DrawText::new(
                format!(
                    "master vol: {:.1}    bed bus vol: {:.1}    bed duck: {:.2}",
                    self.master_vol, self.bed_vol, self.bed_duck
                ),
                Vec2::new(x, 224.0),
                15.0,
                VALUE,
            ));
            tq.push(DrawText::new(
                format!("last: {}", self.status),
                Vec2::new(x, 248.0),
                14.0,
                VALUE,
            ));
        }
    }

    fn name(&self) -> &'static str {
        "AudioDemo"
    }
}

fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — audio facade".to_string(),
        width: 820,
        height: 300,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });
    app.world.insert_resource(FontData(FONT.to_vec()));

    if let Some(audio) = Audio::new() {
        app.world.insert_resource(audio);
    }
    app.add_system(AudioFacadeSystem); // ticks native fades/ducks; no-op on web
    app.add_system(AudioDemo::default());
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run();
}

/// WASM entry point — `examples/audio_facade/web/index.html` calls this after `init()` (and on the
/// "Start" click; winit wants a user gesture before grabbing the canvas, which also lets the first
/// keypress unlock the `AudioContext`). Runs the same `Audio` facade demo as native.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_audio_facade() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
