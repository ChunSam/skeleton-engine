//! Web audio demo + self-check — the playable example that exercises [`engine::WebAudio`].
//!
//! [`WebAudio`](engine::WebAudio) is the wasm counterpart to the native rodio
//! [`AudioManager`](engine::AudioManager): a small `AudioContext` wrapper with one-shot SFX, a
//! single looping **music** channel, a **master volume**, and **suspend/resume**. It only exists
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
