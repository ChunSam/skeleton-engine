//! Cross-platform **positional** audio via the [`Audio`](engine::Audio) facade — the **same code**
//! on native **and** web. A looping sound source orbits a fixed point; you move the *listener* with
//! the arrow keys and hear (and watch) the sound's volume fall off with distance and pan left/right
//! with the x-offset. The entire demo carries **zero `cfg` guards** except the platform entry point.
//!
//! The facade's positional API addresses a tracked sound by a stable **channel name**:
//! [`play_at_on_channel`](engine::Audio::play_at_on_channel) starts a looping positional sound,
//! [`update_position`](engine::Audio::update_position) repositions it each frame, and
//! [`stop_channel`](engine::Audio::stop_channel) stops it. On native this maps to an `AudioManager`
//! positional channel; on web to a looping `Sfx` with a stereo panner.
//!
//! Keys (the first press also [`resume`](engine::Audio::resume)s audio — browsers gate it behind a
//! user gesture):
//! * `Space` — start / restart the orbiting positional sound
//! * `←` `→` `↑` `↓` — move the listener
//! * `S` — stop the sound ([`stop_channel`](engine::Audio::stop_channel))
//! * `Esc` — quit (native)
//!
//! Run natively: `cargo run --example positional_audio`

use engine::{
    App, Audio, AudioFacadeSystem, DrawText, FontData, InputState, KeyCode, ShouldQuit, System,
    TextQueue, Vec2, WindowConfig, World,
};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/DejaVuSans.ttf"
));

const TITLE: [u8; 4] = [235, 235, 245, 255];
const LEGEND: [u8; 4] = [180, 200, 220, 255];
const HEAD: [u8; 4] = [200, 200, 120, 255];
const VALUE: [u8; 4] = [160, 255, 170, 255];
const DIM: [u8; 4] = [120, 130, 150, 255];

const WIN_W: f32 = 760.0;
const WIN_H: f32 = 360.0;
/// Orbit center (also the listener's starting point).
const CENTER: Vec2 = Vec2::new(WIN_W * 0.5, 150.0);
const ORBIT_RX: f32 = 250.0;
const ORBIT_RY: f32 = 90.0;
/// Silent at/after this source↔listener distance.
const MAX_DIST: f32 = 320.0;
const CHANNEL: &str = "orbit";

/// The same (volume, pan) model the engine's positional audio uses: linear distance falloff (silent
/// at `max_dist`) and pan from the x-offset. Replicated here purely for the on-screen readout.
fn spatial(source: Vec2, listener: Vec2, max_dist: f32) -> (f32, f32) {
    let dist = source.distance(listener);
    let vol = (1.0 - dist / max_dist).clamp(0.0, 1.0);
    let pan = ((source.x - listener.x) / max_dist).clamp(-1.0, 1.0);
    (vol, pan)
}

/// Builds an in-memory looping-friendly mono PCM WAV of a `freq`-Hz sine `secs` long (same helper
/// shape as the `audio_facade` example) so the demo needs no audio asset and both backends decode it.
fn sine_wav(freq: f32, secs: f32) -> Vec<u8> {
    use core::f32::consts::TAU;
    let sample_rate: u32 = 44_100;
    let n = (sample_rate as f32 * secs) as u32;
    let data_len = n * 2;
    let mut v = Vec::with_capacity(44 + data_len as usize);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&(36 + data_len).to_le_bytes());
    v.extend_from_slice(b"WAVE");
    v.extend_from_slice(b"fmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&sample_rate.to_le_bytes());
    v.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&data_len.to_le_bytes());
    for i in 0..n {
        let t = i as f32 / sample_rate as f32;
        let sample = (t * freq * TAU).sin() * 0.3;
        let q = (sample * i16::MAX as f32) as i16;
        v.extend_from_slice(&q.to_le_bytes());
    }
    v
}

/// A 21-cell ASCII meter with the marker at `t` (`0.0..=1.0`).
fn meter(t: f32) -> String {
    let n = 21usize;
    let i = (t.clamp(0.0, 1.0) * (n - 1) as f32).round() as usize;
    let mut s: Vec<char> = vec!['-'; n];
    s[i] = 'o';
    s.into_iter().collect()
}

struct PositionalDemo {
    clip: Vec<u8>,
    angle: f32,
    listener: Vec2,
    playing: bool,
    status: String,
}

impl Default for PositionalDemo {
    fn default() -> Self {
        Self {
            clip: sine_wav(330.0, 0.5),
            angle: 0.0,
            listener: CENTER,
            playing: false,
            status: "press Space to start".into(),
        }
    }
}

impl System for PositionalDemo {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (start, stop, left, right, up, down, quit) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.just_pressed(KeyCode::Space),
                input.just_pressed(KeyCode::KeyS),
                input.is_pressed(KeyCode::ArrowLeft),
                input.is_pressed(KeyCode::ArrowRight),
                input.is_pressed(KeyCode::ArrowUp),
                input.is_pressed(KeyCode::ArrowDown),
                input.just_pressed(KeyCode::Escape),
            )
        };

        // Move the listener (held arrows).
        let speed = 220.0 * dt;
        if left {
            self.listener.x -= speed;
        }
        if right {
            self.listener.x += speed;
        }
        if up {
            self.listener.y -= speed;
        }
        if down {
            self.listener.y += speed;
        }
        self.listener.x = self.listener.x.clamp(0.0, WIN_W);
        self.listener.y = self.listener.y.clamp(0.0, WIN_H);

        // Advance the orbit and compute the current source position.
        if self.playing {
            self.angle += dt * 0.9;
        }
        let source = CENTER + Vec2::new(self.angle.cos() * ORBIT_RX, self.angle.sin() * ORBIT_RY);
        let (vol, pan) = spatial(source, self.listener, MAX_DIST);

        if let Some(audio) = world.resource_mut::<Audio>() {
            if start || stop {
                audio.resume(); // unlock on web (no-op native)
            }
            if start {
                audio.play_at_on_channel(
                    CHANNEL,
                    &self.clip,
                    source,
                    self.listener,
                    MAX_DIST,
                    "sfx",
                );
                self.playing = true;
                self.status = "play_at_on_channel (looping, orbiting)".into();
            }
            if stop {
                audio.stop_channel(CHANNEL);
                self.playing = false;
                self.status = "stop_channel".into();
            }
            if self.playing {
                audio.update_position(CHANNEL, source, self.listener, MAX_DIST);
            }
        }

        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let x = 24.0;
            tq.push(DrawText::new(
                "Cross-platform positional audio facade — same code, native + web",
                Vec2::new(x, 22.0),
                21.0,
                TITLE,
            ));
            tq.push(DrawText::new(
                "Space : start/restart    Arrows : move listener    S : stop    (the sound orbits; volume falls with distance, pan follows x)",
                Vec2::new(x, 54.0),
                13.0,
                LEGEND,
            ));
            tq.push(DrawText::new(
                "[ geometry ]",
                Vec2::new(x, 92.0),
                15.0,
                HEAD,
            ));
            tq.push(DrawText::new(
                format!(
                    "source: ({:>4.0}, {:>4.0})    listener: ({:>4.0}, {:>4.0})    distance: {:>3.0} / {:.0}",
                    source.x, source.y, self.listener.x, self.listener.y,
                    source.distance(self.listener), MAX_DIST
                ),
                Vec2::new(x, 114.0),
                14.0,
                VALUE,
            ));
            tq.push(DrawText::new(
                "[ spatial result ]",
                Vec2::new(x, 150.0),
                15.0,
                HEAD,
            ));
            tq.push(DrawText::new(
                format!("volume  {}  {:.2}", meter(vol), vol),
                Vec2::new(x, 172.0),
                14.0,
                VALUE,
            ));
            tq.push(DrawText::new(
                format!("pan  L {} R  {:+.2}", meter((pan + 1.0) * 0.5), pan),
                Vec2::new(x, 194.0),
                14.0,
                VALUE,
            ));
            tq.push(DrawText::new(
                format!("playing: {}", if self.playing { "yes" } else { "no" }),
                Vec2::new(x, 224.0),
                14.0,
                DIM,
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
        "PositionalDemo"
    }
}

fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — positional audio facade".to_string(),
        width: WIN_W as u32,
        height: WIN_H as u32,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });
    app.world.insert_resource(FontData(FONT.to_vec()));

    if let Some(audio) = Audio::new() {
        app.world.insert_resource(audio);
    }
    app.add_system(AudioFacadeSystem); // ticks native fades/ducks; no-op on web
    app.add_system(PositionalDemo::default());
    app.run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    run();
}

/// WASM entry point — a `web/` harness would call this after `init()` + a Start click (same shape as
/// the `audio_facade` example). Runs the identical `Audio` positional demo as native.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_positional_audio() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
