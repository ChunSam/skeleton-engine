//! Audio-reactive hooks — drive visuals from what the audio is actually doing.
//!
//! [`Audio::levels`](engine::Audio::levels) reports a named channel's live loudness as
//! [`AudioLevels`](engine::AudioLevels) (`rms` = perceived loudness, `peak` = transient level).
//! This example turns that into the three things games actually build with it: a **pulse** scaled
//! by `rms`, a **kick flash** fired when `peak` crosses a threshold, and **meter bars** showing
//! both. Everything is synthesized in memory, so there is no audio asset and native and web behave
//! identically — the whole [`run`] function carries **zero `cfg` guards**.
//!
//! Two keys exist to demonstrate the design decisions behind the API, and they are the point of
//! the example:
//!
//! * **`M` mutes the master volume — and the visuals keep pulsing.** Levels are measured
//!   **pre-volume** (the sound's own envelope, before channel/bus/master gain), so a beat-reactive
//!   game does not go visually dead when the player turns the sound down. Watch the pulse while
//!   "master vol" reads 0.0.
//! * **`S` cycles the release time.** A meter rises instantly (so a hit is never visually late)
//!   and falls over the release time (so it does not strobe). At `0.00` the bars flicker; at
//!   `0.60` they smear. `0.15` is the default for a reason.
//!
//! Keys (each press also calls [`Audio::resume`], which unlocks audio on web after the first
//! gesture and is a no-op on native):
//! * `Space` — fire one hit on the analyzed channel
//! * `A` — toggle the auto beat (a repeating kick + off-beat blip)
//! * `M` — toggle master mute (**the pre-volume demo**)
//! * `S` — cycle meter release: 0.00 → 0.15 → 0.60
//! * `Esc` — quit (native)
//!
//! Run natively: `cargo run --example audio_reactive`
//!
//! Run in the browser:
//! ```text
//! examples/audio_reactive/web/build.sh
//! python3 -m http.server 8087 --directory examples/audio_reactive/web
//! open http://localhost:8087        # click "Start", then press A
//! ```

use engine::{
    App, Audio, AudioFacadeSystem, DrawRect, DrawText, FontData, InputState, KeyCode, ShouldQuit,
    System, TextQueue, UiQueue, Vec2, WindowConfig, World,
};

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

/// Bundled deterministic Latin font so the demo's text metrics match across platforms.
const FONT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/fonts/DejaVuSans.ttf"
));

const WIN_W: u32 = 820;
const WIN_H: u32 = 470;

const TITLE: [u8; 4] = [235, 235, 245, 255];
const LEGEND: [u8; 4] = [180, 200, 220, 255];
const HEAD: [u8; 4] = [200, 200, 120, 255];
const VALUE: [u8; 4] = [160, 255, 170, 255];
const NOTE: [u8; 4] = [255, 205, 130, 255];

/// The one analyzed channel. A **named** tone channel, because `play_sfx` round-robins anonymous
/// voices on native and so has no stable name to meter (see `Audio::enable_analysis`).
const BEAT_CHANNEL: &str = "ar_beat";
/// Bus the beat rides, so master volume is demonstrably a *separate* stage from the measurement.
const BEAT_BUS: &str = "beat";

/// Low "kick" and higher off-beat "blip" — different peaks, so the meters visibly differ.
const KICK_HZ: f32 = 110.0;
const BLIP_HZ: f32 = 660.0;
const KICK_SECS: f32 = 0.28;
const BLIP_SECS: f32 = 0.14;
/// Seconds between auto-beat steps (8 steps ≈ a 2 s bar).
const STEP_SECS: f32 = 0.25;

/// `peak` above this counts as a transient worth flashing on.
const KICK_THRESHOLD: f32 = 0.35;
/// How long a kick flash stays lit, in seconds.
const FLASH_SECS: f32 = 0.18;

/// Release times cycled by `S`. `0.0` = no smoothing at all.
const RELEASES: [f32; 3] = [0.0, engine::DEFAULT_ANALYSIS_SMOOTHING, 0.6];

// ── Meter layout ──────────────────────────────────────────────────────────────
// Derived from one another rather than being independent magic numbers: the first headless
// capture of this example had the labels drawn *under* the bars because their Y values were
// written separately and drifted apart.
const BAR_X: f32 = 300.0;
const BAR_W: f32 = 460.0;
const BAR_H: f32 = 24.0;
/// Vertical distance between the two bars.
const BAR_GAP: f32 = 46.0;
const RMS_BAR_Y: f32 = 226.0;
const PEAK_BAR_Y: f32 = RMS_BAR_Y + BAR_GAP;
/// A bar's label is drawn this far *above* its bar, so it can never overlap it.
const LABEL_DY: f32 = -19.0;
const LABEL_SIZE: f32 = 13.0;

/// The demo system: plays a beat on one analyzed channel and draws everything from its levels.
struct AudioReactive {
    /// Auto-beat on/off.
    auto: bool,
    /// Seconds until the next auto-beat step.
    next_step: f32,
    /// Which of the 8 steps in the bar we are on.
    step: u32,
    /// Master volume, mirrored here because the facade has no getter for it.
    master_vol: f32,
    /// Remaining kick-flash time.
    flash: f32,
    /// Kicks detected so far — proves the threshold actually fires (and is the headless assertion).
    kicks: u32,
    /// Index into [`RELEASES`].
    release_idx: usize,
    /// Highest `rms` seen, so a run leaves evidence it measured something.
    max_rms: f32,
}

impl Default for AudioReactive {
    fn default() -> Self {
        Self {
            auto: true,
            next_step: 0.0,
            step: 0,
            master_vol: 1.0,
            flash: 0.0,
            kicks: 0,
            release_idx: 1, // the default release
            max_rms: 0.0,
        }
    }
}

impl AudioReactive {
    /// Plays one beat step on the analyzed channel. Step 0 and 4 are kicks; 2 and 6 are blips.
    fn play_step(&self, audio: &mut Audio, step: u32) {
        match step % 8 {
            0 | 4 => audio.play_tone_on_channel(BEAT_CHANNEL, KICK_HZ, KICK_SECS, 0.9, BEAT_BUS),
            2 | 6 => audio.play_tone_on_channel(BEAT_CHANNEL, BLIP_HZ, BLIP_SECS, 0.5, BEAT_BUS),
            _ => {}
        }
    }
}

impl System for AudioReactive {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── Input ─────────────────────────────────────────────────────────────
        let (hit, toggle_auto, toggle_mute, cycle_release, quit) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.just_pressed(KeyCode::Space),
                input.just_pressed(KeyCode::KeyA),
                input.just_pressed(KeyCode::KeyM),
                input.just_pressed(KeyCode::KeyS),
                input.just_pressed(KeyCode::Escape),
            )
        };
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
        }

        if toggle_auto {
            self.auto = !self.auto;
            self.next_step = 0.0;
        }
        if toggle_mute {
            self.master_vol = if self.master_vol > 0.0 { 0.0 } else { 1.0 };
        }
        if cycle_release {
            self.release_idx = (self.release_idx + 1) % RELEASES.len();
        }

        // ── Audio ─────────────────────────────────────────────────────────────
        let levels = {
            let Some(audio) = world.resource_mut::<Audio>() else {
                return;
            };
            if hit || toggle_auto || toggle_mute || cycle_release {
                audio.resume(); // unlocks the web AudioContext after a gesture; no-op on native
            }
            if toggle_mute {
                audio.set_master_volume(self.master_vol);
            }
            if cycle_release {
                audio.set_analysis_smoothing(RELEASES[self.release_idx]);
            }
            if hit {
                audio.play_tone_on_channel(BEAT_CHANNEL, KICK_HZ, KICK_SECS, 0.9, BEAT_BUS);
            }
            if self.auto {
                self.next_step -= dt;
                if self.next_step <= 0.0 {
                    self.next_step += STEP_SECS;
                    let step = self.step;
                    self.step = step.wrapping_add(1);
                    self.play_step(audio, step);
                }
            }
            // Read the meter. `AudioFacadeSystem` has already ticked `update` this frame.
            audio.levels(BEAT_CHANNEL)
        };

        if levels.rms > self.max_rms {
            self.max_rms = levels.rms;
        }

        // Kick detection: a transient crossing the threshold, edge-triggered by the flash timer so
        // one hit counts once rather than every frame it stays loud.
        self.flash = (self.flash - dt).max(0.0);
        if levels.peak >= KICK_THRESHOLD && self.flash <= 0.0 {
            self.flash = FLASH_SECS;
            self.kicks += 1;
        }

        // ── Draw ──────────────────────────────────────────────────────────────
        if let Some(uq) = world.resource_mut::<UiQueue>() {
            // Kick flash: a full-width band that lights on a transient.
            let flash_a = (self.flash / FLASH_SECS).clamp(0.0, 1.0);
            if flash_a > 0.0 {
                uq.push(DrawRect::new(
                    0.0,
                    0.0,
                    WIN_W as f32,
                    6.0,
                    [1.0, 0.75, 0.35, flash_a],
                ));
            }

            // Pulse: a square scaled by rms. This is the "one line of game code" case —
            // `1.0 + rms * k` is usually all a reactive visual needs.
            let cx = 150.0;
            let cy = 250.0;
            let base = 54.0;
            let size = base * (1.0 + levels.rms * 1.9);
            uq.push(
                DrawRect::new(
                    cx - size * 0.5,
                    cy - size * 0.5,
                    size,
                    size,
                    [0.35 + levels.rms * 0.6, 0.45, 0.95, 1.0],
                )
                .with_corner_radius(size * 0.22),
            );

            // Meter bars.
            for (y, value, color) in [
                (RMS_BAR_Y, levels.rms, [0.45, 0.85, 1.0, 1.0]),
                (PEAK_BAR_Y, levels.peak, [1.0, 0.62, 0.42, 1.0]),
            ] {
                // Track.
                uq.push(
                    DrawRect::new(BAR_X, y, BAR_W, BAR_H, [0.16, 0.17, 0.22, 1.0])
                        .with_corner_radius(6.0),
                );
                // Fill.
                let w = (BAR_W * value.clamp(0.0, 1.0)).max(1.0);
                uq.push(DrawRect::new(BAR_X, y, w, BAR_H, color).with_corner_radius(6.0));
            }

            // Kick threshold marker, spanning exactly the peak bar it refers to.
            uq.push(DrawRect::new(
                BAR_X + BAR_W * KICK_THRESHOLD,
                PEAK_BAR_Y,
                2.0,
                BAR_H,
                [1.0, 1.0, 1.0, 0.75],
            ));
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            let x = 24.0;
            tq.push(DrawText::new(
                "skeleton-engine — audio-reactive hooks (Audio::levels)",
                Vec2::new(x, 22.0),
                19.0,
                TITLE,
            ));
            tq.push(DrawText::new(
                "Space : one hit    A : auto beat    M : mute master    S : cycle release    Esc : quit",
                Vec2::new(x, 52.0),
                14.0,
                LEGEND,
            ));
            tq.push(DrawText::new(
                "M mutes the sound and the pulse KEEPS GOING — levels are measured pre-volume,",
                Vec2::new(x, 84.0),
                14.0,
                NOTE,
            ));
            tq.push(DrawText::new(
                "so a beat-reactive visual survives the player turning the volume down.",
                Vec2::new(x, 102.0),
                14.0,
                NOTE,
            ));
            tq.push(DrawText::new(
                "S: release 0.00 flickers · 0.15 reads well · 0.60 smears. Rise is always instant.",
                Vec2::new(x, 126.0),
                14.0,
                NOTE,
            ));

            tq.push(DrawText::new(
                "[ meters ]",
                Vec2::new(BAR_X, RMS_BAR_Y - 46.0),
                15.0,
                HEAD,
            ));
            tq.push(DrawText::new(
                format!("rms  {:.3}", levels.rms),
                Vec2::new(BAR_X, RMS_BAR_Y + LABEL_DY),
                LABEL_SIZE,
                VALUE,
            ));
            tq.push(DrawText::new(
                format!(
                    "peak {:.3}   (kick threshold {KICK_THRESHOLD:.2})",
                    levels.peak
                ),
                Vec2::new(BAR_X, PEAK_BAR_Y + LABEL_DY),
                LABEL_SIZE,
                VALUE,
            ));
            tq.push(DrawText::new(
                format!(
                    "auto beat: {}    master vol: {:.1}    release: {:.2}s    kicks: {}",
                    if self.auto { "on" } else { "off" },
                    self.master_vol,
                    RELEASES[self.release_idx],
                    self.kicks,
                ),
                Vec2::new(x, 340.0),
                15.0,
                VALUE,
            ));
            tq.push(DrawText::new(
                "pulse = rms · top band = a peak transient · white tick = the kick threshold",
                Vec2::new(x, 372.0),
                13.0,
                LEGEND,
            ));
        }
    }

    fn name(&self) -> &'static str {
        "AudioReactive"
    }
}

/// WASM-only acceptance test: watches the meter and stamps a verdict into the document title,
/// where `scripts/audio_reactive_smoke.sh` reads it over the DevTools endpoint.
///
/// This is what makes the cross-platform claim *verified* rather than merely compiled. A wasm
/// build proves the `AnalyserNode` code type-checks; only running it in a browser proves the
/// meter actually moves there. (`examples/embedded_image.rs` was broken for wasm from the day it
/// was added precisely because nothing ran it.)
#[cfg(target_arch = "wasm32")]
#[derive(Default)]
struct WebSelfCheck {
    frames: u32,
    max_rms: f32,
    done: bool,
}

#[cfg(target_arch = "wasm32")]
impl System for WebSelfCheck {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if self.done {
            return;
        }
        self.frames += 1;
        let Some(audio) = world.resource_mut::<Audio>() else {
            stamp("AR_CHECK: FAIL: no Audio resource (AudioContext refused)");
            self.done = true;
            return;
        };
        // The context starts suspended until a gesture; the smoke runs Chrome with
        // --autoplay-policy=no-user-gesture-required, so resuming here is enough.
        if self.frames < 30 {
            audio.resume();
        }
        let rms = audio.levels(BEAT_CHANNEL).rms;
        if rms > self.max_rms {
            self.max_rms = rms;
        }
        // A level well clear of zero means the analyser really is reading the playing tone.
        if self.max_rms > 0.02 {
            stamp(&format!("AR_CHECK: PASS rms={:.3}", self.max_rms));
            self.done = true;
        } else if self.frames > 900 {
            stamp(&format!(
                "AR_CHECK: FAIL: meter never moved (max rms {:.4})",
                self.max_rms
            ));
            self.done = true;
        }
    }

    fn name(&self) -> &'static str {
        "WebSelfCheck"
    }
}

/// Publishes the verdict to the document title (polled by the smoke) and the `#status` element
/// (visible to a human).
#[cfg(target_arch = "wasm32")]
fn stamp(verdict: &str) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        doc.set_title(verdict);
        if let Some(el) = doc.get_element_by_id("status") {
            el.set_inner_html(verdict);
        }
    }
}

fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — audio reactive".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.05, 0.05, 0.08, 1.0],
    });
    app.world.insert_resource(FontData(FONT.to_vec()));

    if let Some(mut audio) = Audio::new() {
        // Opt in BEFORE the first play on this channel — the meter is wired into the sound when it
        // starts, so enabling it mid-sound would not take effect until the next play.
        audio.enable_analysis(BEAT_CHANNEL);
        app.world.insert_resource(audio);
    }
    app.add_system(AudioFacadeSystem); // ticks the meters (and native fades/ducks)
    app.add_system(AudioReactive::default());
    #[cfg(target_arch = "wasm32")]
    app.add_system(WebSelfCheck::default());
    app
}

fn run() {
    build_app().run();
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // `AUDIO_REACTIVE_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    if std::env::var("AUDIO_REACTIVE_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }
    run();
}

/// Headless acceptance test: drives a real tone through a real device and asserts the meter both
/// **rises** while it plays and **decays** after it stops.
///
/// Deliberately does not go through `App`: a headless frame loop runs as fast as it can, while the
/// audio device advances on the wall clock, so a frame-count-based assertion would be timing
/// noise. Sleeping on the wall clock and ticking `update` with the real elapsed time is what
/// actually reproduces a game's frame cadence here.
///
/// Exit codes: `0` = pass (or skipped, no device) · `1` = the meter never moved · `2` = it never
/// decayed.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use std::time::{Duration, Instant};

    let Some(mut audio) = Audio::new() else {
        // No audio device (CI box, no sound card). The feature cannot be exercised; do not fail.
        println!("SKIP: no audio device available — nothing to measure");
        return 0;
    };
    audio.enable_analysis(BEAT_CHANNEL);
    assert!(audio.is_analysis_enabled(BEAT_CHANNEL));
    assert!(
        audio.levels(BEAT_CHANNEL).is_silent(),
        "a channel that has never played must read silent"
    );

    // A long, loud tone so the meter has something unambiguous to see.
    audio.play_tone_on_channel(BEAT_CHANNEL, KICK_HZ, 2.0, 0.9, BEAT_BUS);

    let tick = Duration::from_millis(16);
    let mut peak_seen: f32 = 0.0;
    let rise_deadline = Instant::now() + Duration::from_millis(1500);
    while Instant::now() < rise_deadline {
        std::thread::sleep(tick);
        audio.update(tick.as_secs_f32());
        let l = audio.levels(BEAT_CHANNEL);
        peak_seen = peak_seen.max(l.rms);
        if peak_seen > 0.05 {
            break;
        }
    }
    if peak_seen <= 0.05 {
        eprintln!("FAIL: the meter never rose while a tone was playing (max rms {peak_seen:.4})");
        return 1;
    }
    println!("OK: meter rose to rms {peak_seen:.3} while the tone played");

    // Let the tone finish, then confirm the meter falls back to silence instead of freezing at its
    // last value — the failure mode a stopped channel would otherwise show as a stuck bar.
    let decay_deadline = Instant::now() + Duration::from_millis(4000);
    let mut final_rms = f32::MAX;
    while Instant::now() < decay_deadline {
        std::thread::sleep(tick);
        audio.update(tick.as_secs_f32());
        final_rms = audio.levels(BEAT_CHANNEL).rms;
        if final_rms < 0.01 {
            break;
        }
    }
    if final_rms >= 0.01 {
        eprintln!("FAIL: the meter never decayed after the tone ended (rms {final_rms:.4})");
        return 2;
    }
    println!("OK: meter decayed to rms {final_rms:.4} after the tone ended");
    println!("OK: audio-reactive levels rise with the sound and fall back to silence");
    0
}

/// WASM entry point — `examples/audio_reactive/web/index.html` calls this after `init()` (and on
/// the "Start" click; winit wants a user gesture before grabbing the canvas, which also lets the
/// first keypress unlock the `AudioContext`). Runs the same code as native.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_audio_reactive() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
