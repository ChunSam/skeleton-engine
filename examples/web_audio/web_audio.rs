//! Web audio demo + self-check — the playable example that exercises [`engine::WebAudio`].
//!
//! [`WebAudio`](engine::WebAudio) is the wasm counterpart to the native rodio
//! [`AudioManager`](engine::AudioManager): a small `AudioContext` wrapper with one-shot SFX, a
//! single looping **music** channel (with track-to-track **crossfade**), a **master volume**,
//! **named mixer buses** (with **ducking**), **2D positional** playback, and **suspend/resume**.
//! It only exists
//! on `wasm32`, so this example is wasm-only — running it natively just prints how to build it.
//!
//! As well as being the demo, this drives the whole `WebAudio` surface in sequence and writes a
//! pass/fail line per step into the page (and the document title), so a headless browser can
//! assert the audio-graph lifecycle automatically — see `scripts/wasm_audio_smoke.sh`. The one
//! thing no script can check is whether you actually *hear* the tone; open it and listen for that.
//!
//! ```text
//! examples/web_audio/web/build.sh                       # build to wasm
//! python3 -m http.server 8080 --directory examples/web_audio/web
//! open http://localhost:8080                            # click "Start audio", then listen
//! ```

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("web_audio is a wasm-only example (engine::WebAudio exists only on wasm32).");
    println!("Build it for the browser and serve it:");
    println!("  examples/web_audio/web/build.sh");
    println!("  python3 -m http.server 8080 --directory examples/web_audio/web");
    println!("  open http://localhost:8080");
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;

/// WASM entry point — `examples/web_audio/web/index.html` calls this after `init()`, and again on
/// the "Start audio" click (a browser user gesture is what unlocks the `AudioContext`).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_web_audio() {
    wasm_bindgen_futures::spawn_local(run_checks());
}

/// WASM entry point for the **paced listening demo** — `index.html` calls this on the "Play paced
/// demo" click. Unlike [`run_web_audio`] (the fast self-check), this plays each audible feature
/// (center / hard-left / hard-right / pan sweep / positional flythrough / music crossfade / smooth
/// bus duck) as a distinct stage with gaps between them, so the stereo + ramp behaviour is clearly
/// distinguishable by ear.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_audio_demo() {
    wasm_bindgen_futures::spawn_local(audio_demo());
}

/// Drives every `WebAudio` method and records a pass/fail line per step.
#[cfg(target_arch = "wasm32")]
async fn run_checks() {
    use engine::WebAudio;

    let mut lines: Vec<String> = Vec::new();
    let mut passed = 0u32;
    let mut total = 0u32;
    let mut first_fail: Option<String> = None;

    macro_rules! check {
        ($cond:expr, $msg:expr) => {{
            let pass = $cond;
            total += 1;
            if pass {
                passed += 1;
            } else if first_fail.is_none() {
                first_fail = Some($msg.to_string());
            }
            lines.push(format!("{} {}", if pass { "✅" } else { "❌" }, $msg));
            render(&lines);
        }};
    }

    let Some(audio) = WebAudio::new() else {
        lines.push("❌ WebAudio::new() returned None — no AudioContext".into());
        render(&lines);
        finish(0, 1, Some("WebAudio::new() returned None".into()));
        return;
    };
    check!(true, "WebAudio::new() created an AudioContext");

    // ── master volume (synchronous) ──────────────────────────────────────────
    check!((audio.volume() - 1.0).abs() < 1e-3, "default volume is 1.0");
    audio.set_volume(0.5);
    check!(
        (audio.volume() - 0.5).abs() < 1e-3,
        "set_volume(0.5) -> volume() == 0.5"
    );
    audio.set_volume(2.0);
    check!(
        (audio.volume() - 1.0).abs() < 1e-3,
        "set_volume(2.0) clamps to 1.0"
    );
    audio.set_volume(-1.0);
    check!(
        audio.volume().abs() < 1e-3,
        "set_volume(-1.0) clamps to 0.0"
    );
    audio.set_volume(0.6);

    // ── resume unlocks the context ───────────────────────────────────────────
    // Poll the conditions instead of sleeping a fixed time: resume() and decode_audio_data are
    // async, and under a headless virtual-time clock a fixed sleep can elapse before the real
    // work settles. Polling is correct under both real and virtual time.
    audio.resume();
    check!(
        wait_until(|| audio.is_running(), 60).await,
        "resume() -> context is running"
    );

    // ── looping music decodes + starts ───────────────────────────────────────
    let wav = sine_wav(440.0, 0.6);
    audio.play_music(&wav);
    check!(
        wait_until(|| audio.is_music_playing(), 60).await,
        "play_music() decoded and started looping music"
    );

    // ── controllable SFX: pan + volume + stop (play_sfx -> Sfx) ───────────────
    let beep = sine_wav(660.0, 0.5);
    let sfx = audio.play_sfx(&beep);
    sfx.set_pan(-0.8); // pan left (works before decode finishes)
    sfx.set_volume(0.5);
    check!(
        wait_until(|| sfx.is_playing(), 60).await,
        "play_sfx() decoded and started a controllable SFX"
    );
    check!(
        sfx.is_playing(),
        "set_pan/set_volume applied without dropping the SFX"
    );
    sfx.stop();
    check!(!sfx.is_playing(), "Sfx::stop() stopped the SFX");

    // ── named mixer buses ─────────────────────────────────────────────────────
    check!(
        (audio.bus_volume("sfx") - 1.0).abs() < 1e-3,
        "unknown bus volume defaults to 1.0"
    );
    audio.set_bus_volume("sfx", 0.5);
    check!(
        (audio.bus_volume("sfx") - 0.5).abs() < 1e-3,
        "set_bus_volume(0.5) -> bus_volume() == 0.5"
    );
    audio.set_bus_volume("sfx", 2.0);
    check!(
        (audio.bus_volume("sfx") - 1.0).abs() < 1e-3,
        "set_bus_volume(2.0) clamps to 1.0"
    );
    audio.set_bus_volume("sfx", -1.0);
    check!(
        audio.bus_volume("sfx").abs() < 1e-3,
        "set_bus_volume(-1.0) clamps to 0.0"
    );
    audio.set_bus_volume("sfx", 0.6);
    check!(
        audio.bus_names() == vec!["sfx".to_string()],
        "bus_names() lists the registered bus"
    );
    // A controllable SFX routed through the bus.
    let bus_sfx = audio.play_sfx_on_bus(&beep, "sfx");
    check!(
        wait_until(|| bus_sfx.is_playing(), 60).await,
        "play_sfx_on_bus() decoded and started SFX on the bus"
    );
    bus_sfx.stop();
    // A fire-and-forget sound on a second bus registers it (sorted in bus_names).
    audio.play_on_bus(&beep, "ui");
    check!(
        audio.bus_names() == vec!["sfx".to_string(), "ui".to_string()],
        "play_on_bus() registers a new bus (names stay sorted)"
    );

    // ── bus ducking ────────────────────────────────────────────────────────────
    // NOTE: a *ramped* duck's live value (AudioParam automation) is computed on the audio render
    // thread, which doesn't advance under headless SwiftShader — so the smoke uses an instant duck
    // (attack/release = 0.0s, written via the directly-readable `value=` setter). A real (dur > 0)
    // duck still ramps smoothly; that ramp, like acoustic output, is verified by ear in a browser.
    check!(
        (audio.bus_duck("sfx") - 1.0).abs() < 1e-3,
        "bus_duck() defaults to 1.0 (no duck)"
    );
    audio.duck_bus("sfx", 0.2, 0.0);
    check!(
        (audio.bus_duck("sfx") - 0.2).abs() < 1e-3,
        "duck_bus(0.2) sets the duck multiplier (independent of bus volume)"
    );
    check!(
        (audio.bus_volume("sfx") - 0.6).abs() < 1e-3,
        "ducking did not touch the bus volume (still 0.6)"
    );
    audio.release_bus("sfx", 0.0);
    check!(
        (audio.bus_duck("sfx") - 1.0).abs() < 1e-3,
        "release_bus() returns the duck to 1.0"
    );
    audio.duck_bus("sfx", -1.0, 0.0);
    check!(
        audio.bus_duck("sfx").abs() < 1e-3,
        "duck_bus(-1.0) clamps the duck to 0.0"
    );
    audio.duck_bus("sfx", 2.0, 0.0);
    check!(
        (audio.bus_duck("sfx") - 1.0).abs() < 1e-3,
        "duck_bus(2.0) clamps the duck to 1.0"
    );

    // ── 2D positional audio ────────────────────────────────────────────────────
    // play_at/update_position set the SFX volume + pan via the directly-readable set_value path, so
    // the spatial result is observable headless (unlike a smooth duck ramp).
    let here = engine::Vec2::ZERO;
    let near = audio.play_at(&beep, here, here, 100.0);
    check!(
        (near.volume() - 1.0).abs() < 1e-3,
        "play_at() at the listener -> full volume"
    );
    check!(
        near.pan().abs() < 1e-3,
        "play_at() at the listener -> centered pan"
    );
    check!(
        wait_until(|| near.is_playing(), 60).await,
        "play_at() decoded and started the SFX"
    );
    near.stop();
    let far = audio.play_at(&beep, engine::Vec2::new(200.0, 0.0), here, 100.0);
    check!(
        far.volume().abs() < 1e-3,
        "play_at() at/beyond max_dist -> silent"
    );
    far.stop();
    let right = audio.play_at(&beep, engine::Vec2::new(50.0, 0.0), here, 100.0);
    check!(right.pan() > 0.4, "play_at() to the right -> positive pan");
    check!(
        (right.volume() - 0.5).abs() < 0.05,
        "play_at() half-way to max_dist -> ~half volume"
    );
    right.update_position(engine::Vec2::new(-50.0, 0.0), here, 100.0);
    check!(
        right.pan() < -0.4,
        "update_position() to the left -> negative pan"
    );
    right.stop();
    // Positional routed through a bus: the per-source volume/pan still carry the spatial result,
    // independent of the bus volume (downstream).
    let bus_pos = audio.play_at_on_bus(&beep, engine::Vec2::new(50.0, 0.0), here, 100.0, "sfx");
    check!(
        wait_until(|| bus_pos.is_playing(), 60).await,
        "play_at_on_bus() decoded and started the SFX"
    );
    check!(
        bus_pos.pan() > 0.4,
        "play_at_on_bus() to the right -> positive pan (spatial through a bus)"
    );
    check!(
        (bus_pos.volume() - 0.5).abs() < 0.05,
        "play_at_on_bus() per-source volume is the spatial value (independent of bus volume)"
    );
    bus_pos.stop();

    // ── music crossfade (track-to-track) ──────────────────────────────────────
    // Music (440 Hz) is still looping from play_music above; cross-fade to a lower tone. The new
    // track fades in on its own gain while the old fades out + stops — observably, the music
    // channel ends up running the new source.
    let wav2 = sine_wav(330.0, 0.6);
    audio.crossfade_music(&wav2, 0.2);
    check!(
        wait_until(|| audio.is_music_playing(), 60).await,
        "crossfade_music() faded in the new track (music channel running)"
    );
    // Crossfade with nothing playing is just a fade-in.
    audio.stop_music();
    check!(
        !audio.is_music_playing(),
        "stop_music() cleared the channel"
    );
    audio.crossfade_music(&wav, 0.2);
    check!(
        wait_until(|| audio.is_music_playing(), 60).await,
        "crossfade_music() with no current track fades in"
    );

    // ── suspend / resume toggles the context ─────────────────────────────────
    audio.suspend();
    check!(
        wait_until(|| !audio.is_running(), 60).await,
        "suspend() -> context not running"
    );
    audio.resume();
    check!(
        wait_until(|| audio.is_running(), 60).await,
        "resume() again -> context running"
    );

    // Leave music playing + volume up so a human listener actually hears the tone.
    audio.set_volume(0.6);
    finish(passed, total, first_fail);
    // Hold the resource alive for the page's lifetime so music keeps looping.
    keep_alive(audio);
}

/// The paced listening demo: plays each audible `WebAudio` feature as its own stage, with a gap
/// (and an on-screen "now playing" line) between stages, so stereo pan and the smooth ramps are
/// clearly distinguishable by ear — unlike the fire-everything-at-once self-check.
#[cfg(target_arch = "wasm32")]
async fn audio_demo() {
    use engine::WebAudio;

    let Some(audio) = WebAudio::new() else {
        set_status("❌ no AudioContext — your browser refused Web Audio");
        return;
    };
    audio.set_volume(0.7);
    audio.resume();
    wait_until(|| audio.is_running(), 60).await;
    let here = engine::Vec2::ZERO;

    // 1 — center reference, so the ear has a baseline before the L/R stages.
    set_status("1/7 ▸ CENTER reference tone (pan 0)");
    let s = audio.play_sfx(&sine_wav(440.0, 1.2));
    s.set_pan(0.0);
    sleep_ms(1300).await;
    s.stop();
    sleep_ms(600).await;

    // 2 — hard left.
    set_status("2/7 ◀◀ LEFT  (pan −1.0) — should come only from the left");
    let s = audio.play_sfx(&sine_wav(440.0, 1.4));
    s.set_pan(-1.0);
    sleep_ms(1500).await;
    s.stop();
    sleep_ms(600).await;

    // 3 — hard right.
    set_status("3/7 RIGHT ▶▶  (pan +1.0) — should come only from the right");
    let s = audio.play_sfx(&sine_wav(440.0, 1.4));
    s.set_pan(1.0);
    sleep_ms(1500).await;
    s.stop();
    sleep_ms(800).await;

    // 4 — continuous pan sweep left → right on one sustained tone.
    set_status("4/7 ↔ pan SWEEP  left → right (one tone, panner animated)");
    let s = audio.play_sfx(&sine_wav(440.0, 2.8));
    for i in 0..=24 {
        s.set_pan(-1.0 + (i as f32 / 24.0) * 2.0);
        sleep_ms(95).await;
    }
    s.stop();
    sleep_ms(800).await;

    // 5 — positional flythrough: source crosses left→right at a fixed distance, so it pans AND
    //     swells loudest as it passes the listener (linear distance falloff).
    set_status("5/7 ✈ POSITIONAL flythrough  (play_at: pans L→R, loudest at center)");
    let s = audio.play_at(
        &sine_wav(523.0, 3.4),
        engine::Vec2::new(-100.0, 0.0),
        here,
        100.0,
    );
    for i in 0..=32 {
        let x = -100.0 + (i as f32 / 32.0) * 200.0;
        s.update_position(engine::Vec2::new(x, 0.0), here, 100.0);
        sleep_ms(95).await;
    }
    s.stop();
    sleep_ms(800).await;

    // 6 — music crossfade with a long (2s) blend so the two tones audibly overlap.
    set_status("6/7 ⤬ music CROSSFADE  440 Hz → 330 Hz over 2.0s (tones overlap mid-fade)");
    audio.play_music(&sine_wav(440.0, 1.0));
    sleep_ms(1700).await;
    audio.crossfade_music(&sine_wav(330.0, 1.0), 2.0);
    sleep_ms(2700).await;
    audio.stop_music();
    sleep_ms(700).await;

    // 7 — smooth bus duck (dur > 0, the ramp the headless self-check can't show): a steady bed on a
    //     bus dips under a "voice" beep over 0.4s, then rises back over 0.4s.
    set_status("7/7 ⤵ DUCKING  steady bed dips under a beep (0.4s ramp down, 0.4s back up)");
    let bed = audio.play_sfx_on_bus(&sine_wav(220.0, 6.0), "bed");
    sleep_ms(800).await;
    audio.duck_bus("bed", 0.15, 0.4); // smooth ramp DOWN
    sleep_ms(250).await;
    let _voice = audio.play_sfx(&sine_wav(880.0, 0.7)); // the cue the bed ducks for
    sleep_ms(1200).await;
    audio.release_bus("bed", 0.4); // smooth ramp UP
    sleep_ms(1300).await;
    bed.stop();

    set_status("✅ Paced demo complete. Refresh the page, then click again to replay.");
}

/// Sets the page's `#status` line to a single "now playing" message for the paced demo.
#[cfg(target_arch = "wasm32")]
fn set_status(msg: &str) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("status"))
    {
        el.set_inner_html(msg);
    }
}

/// Writes the running list of step lines into the page's `#status` element.
#[cfg(target_arch = "wasm32")]
fn render(lines: &[String]) {
    if let Some(el) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("status"))
    {
        el.set_inner_html(&lines.join("<br>"));
    }
}

/// Stamps the final verdict where both a human and the smoke script can see it: the `#result`
/// element and the **document title** both become `AUDIO_CHECK: PASS (n/n)` or
/// `AUDIO_CHECK: FAIL: <step>`. The smoke script polls the title over the DevTools endpoint, so
/// the failing step name travels with the verdict.
#[cfg(target_arch = "wasm32")]
fn finish(passed: u32, total: u32, first_fail: Option<String>) {
    let verdict = match first_fail {
        None => format!("AUDIO_CHECK: PASS ({passed}/{total})"),
        Some(step) => format!("AUDIO_CHECK: FAIL: {step}"),
    };
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        doc.set_title(&verdict);
        if let Some(el) = doc.get_element_by_id("result") {
            el.set_inner_html(&verdict);
        }
    }
}

/// Keeps the cloned `WebAudio` (and thus its `AudioContext` + looping music) alive for the
/// lifetime of the page by leaking it — fine for a demo that runs until the tab closes.
#[cfg(target_arch = "wasm32")]
fn keep_alive(audio: engine::WebAudio) {
    Box::leak(Box::new(audio));
}

/// Polls `cond` every 50 ms up to `max_iters` times, returning `true` as soon as it holds (or its
/// final value). Used to await async audio state without racing a fixed sleep.
#[cfg(target_arch = "wasm32")]
async fn wait_until(mut cond: impl FnMut() -> bool, max_iters: usize) -> bool {
    for _ in 0..max_iters {
        if cond() {
            return true;
        }
        sleep_ms(50).await;
    }
    cond()
}

/// Async sleep via `setTimeout` wrapped in a `Promise`.
#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        let _ = web_sys::window()
            .expect("no window")
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Builds a 16-bit mono PCM WAV of a `freq`-Hz sine wave `secs` long, in memory — so the example
/// needs no audio asset and the browser's `decode_audio_data` has something real to decode.
#[cfg(target_arch = "wasm32")]
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
        let sample = (t * freq * TAU).sin() * 0.25; // 25% amplitude — not too loud
        let q = (sample * i16::MAX as f32) as i16;
        v.extend_from_slice(&q.to_le_bytes());
    }
    v
}
