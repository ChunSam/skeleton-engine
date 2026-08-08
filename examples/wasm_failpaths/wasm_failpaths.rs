//! wasm_failpaths — a page that takes two failure paths **on purpose**, and reports what happened.
//!
//! Every other browser smoke drives a path that is supposed to succeed, and passes when nothing
//! goes wrong. That leaves a blind spot the engine has now been bitten by twice: a *failure*
//! handler can be completely broken and every green check stays green, because nothing ever fails
//! on purpose. Both bugs below were fixed in v0.150.1/v0.150.2 and shipped **compile-verified
//! only** — no automated check could reach either one. This example is what closes that.
//!
//! # The two paths
//!
//! | # | What it does on purpose | What used to happen |
//! |---|---|---|
//! | 1 | `load_image_async` on a URL that 404s | The failure set `AssetLoadState::Failed` but never called `record_failure`, so `asset_failures()` stayed EMPTY and `set_strict_assets` never fired. Both are documented as the way to refuse to start on a missing asset; both were native-only in practice. |
//! | 2 | `send_text` **immediately after `connect`**, while the socket is still `CONNECTING` | The web client handed it to a `CONNECTING` socket, which throws — the message was silently gone. Native queues it in a `sync_channel` and delivers it on open, so the same game lost its join packet on the web and nowhere else. |
//!
//! Check 2 is why the echo server exists: send before open, and the message must come back.
//! Nothing else in the tree sends before `Connected`, which is exactly why the bug survived.
//!
//! # Verdict
//!
//! Stamps `FAILPATH_CHECK: PASS (n/n)` or `FAILPATH_CHECK: FAIL: <step>` into the document title
//! and into `#result`, the same contract `web_audio` uses — `scripts/wasm_failpaths_smoke.sh`
//! reads the title live over Chrome's DevTools endpoint, so the failing step travels with a
//! failure. A deadline turns "never resolved" into a named FAIL rather than a hang.
//!
//! # Running it
//!
//! ```text
//! cargo run --example wasm_failpaths_echo_server      # terminal 1
//! examples/wasm_failpaths/web/build.sh                # terminal 2
//! python3 -m http.server 8080 --directory examples/wasm_failpaths/web
//! ```
//!
//! Native `cargo run --example wasm_failpaths` prints why it is a no-op: both defects are in
//! `#[cfg(target_arch = "wasm32")]` code, so there is nothing to reproduce off the web.

#[cfg(target_arch = "wasm32")]
use engine::{
    App, DrawText, Events, NetworkClient, NetworkEvent, NetworkSystem, System, TextQueue, Vec2,
    WindowConfig, World,
};

/// The URL check 1 asks for. It must not exist — that is the whole point — and the `404-` prefix
/// is a hint to anyone who finds it in a server log wondering what broke.
#[cfg(target_arch = "wasm32")]
const MISSING_ASSET: &str = "assets/404-this-asset-does-not-exist.png";

/// The payload check 2 sends before the socket opens and expects to see echoed back.
#[cfg(target_arch = "wasm32")]
const PREOPEN_SENTINEL: &str = "PREOPEN-SENTINEL";

/// Where the echo server listens. Kept in sync with `echo_server.rs`'s default by hand — the
/// page cannot read the server's env, and hardcoding one string in two files beats inventing a
/// config channel for a smoke.
#[cfg(target_arch = "wasm32")]
const ECHO_URL: &str = "ws://127.0.0.1:9004";

/// How long to wait before calling an unresolved check a failure. Generous: a cold wasm start
/// plus a fetch plus a WebSocket handshake, on a software renderer, on CI.
#[cfg(target_arch = "wasm32")]
const DEADLINE_SECS: f32 = 20.0;

#[cfg(target_arch = "wasm32")]
const TOTAL_CHECKS: u32 = 2;

#[cfg(target_arch = "wasm32")]
struct FailPathSystem {
    elapsed: f32,
    /// `asset_failures()` recorded the 404.
    asset_reported: bool,
    /// The pre-open message came back from the echo server.
    preopen_echoed: bool,
    /// Set once, so the verdict is stamped exactly one time.
    finished: bool,
    status: String,
}

#[cfg(target_arch = "wasm32")]
impl FailPathSystem {
    fn new() -> Self {
        Self {
            elapsed: 0.0,
            asset_reported: false,
            preopen_echoed: false,
            finished: false,
            status: "running…".to_string(),
        }
    }

    fn passed(&self) -> u32 {
        self.asset_reported as u32 + self.preopen_echoed as u32
    }

    /// Names the first unmet check, for a FAIL that says which half broke.
    fn first_unmet(&self) -> Option<String> {
        if !self.asset_reported {
            return Some(format!(
                "a 404 asset fetch did not reach asset_failures() within {DEADLINE_SECS:.0}s"
            ));
        }
        if !self.preopen_echoed {
            return Some(format!(
                "a send issued before the socket opened never came back within {DEADLINE_SECS:.0}s"
            ));
        }
        None
    }
}

#[cfg(target_arch = "wasm32")]
impl System for FailPathSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        if self.finished {
            draw_status(world, &self.status);
            return;
        }
        self.elapsed += dt;

        // ── Check 1: the 404 must be visible through the documented hook ──────────────────
        // `record_failure` runs inside the fetch future, not in the frame poll, so this
        // becomes true on its own once the browser resolves the request.
        if !self.asset_reported {
            self.asset_reported = engine::asset_path::asset_failures()
                .iter()
                .any(|f| f.path.contains("404-this-asset-does-not-exist"));
        }

        // ── Check 2: the pre-open send must have survived the CONNECTING window ───────────
        if !self.preopen_echoed {
            if let Some(bus) = world.resource::<Events<NetworkEvent>>() {
                self.preopen_echoed = bus.read().iter().any(|event| {
                    matches!(event, NetworkEvent::TextMessage(text) if text == PREOPEN_SENTINEL)
                });
            }
        }

        let done = self.asset_reported && self.preopen_echoed;
        let timed_out = self.elapsed >= DEADLINE_SECS;
        if done || timed_out {
            self.finished = true;
            let first_fail = self.first_unmet();
            self.status = match &first_fail {
                None => format!("PASS ({}/{})", self.passed(), TOTAL_CHECKS),
                Some(step) => format!("FAIL: {step}"),
            };
            finish(self.passed(), TOTAL_CHECKS, first_fail);
        } else {
            self.status = format!(
                "running… asset_failures={} preopen_echo={} ({:.1}s)",
                self.asset_reported, self.preopen_echoed, self.elapsed
            );
        }

        draw_status(world, &self.status);
    }
}

/// Draws the live state so a human opening the page sees the same thing the smoke reads.
#[cfg(target_arch = "wasm32")]
fn draw_status(world: &mut World, status: &str) {
    let Some(tq) = world.resource_mut::<TextQueue>() else {
        return;
    };
    tq.push(DrawText::new(
        "wasm_failpaths — two deliberate failures",
        Vec2::new(16.0, 16.0),
        20.0,
        [255, 255, 255, 230],
    ));
    tq.push(DrawText::new(
        status,
        Vec2::new(16.0, 48.0),
        16.0,
        [180, 220, 255, 230],
    ));
}

/// Stamps the verdict where both a human and the smoke script can see it: `#result` and the
/// **document title**. Same contract as `web_audio`'s `AUDIO_CHECK`.
#[cfg(target_arch = "wasm32")]
fn finish(passed: u32, total: u32, first_fail: Option<String>) {
    let verdict = match first_fail {
        None => format!("FAILPATH_CHECK: PASS ({passed}/{total})"),
        Some(step) => format!("FAILPATH_CHECK: FAIL: {step}"),
    };
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        doc.set_title(&verdict);
        if let Some(el) = doc.get_element_by_id("result") {
            el.set_inner_html(&verdict);
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn run() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "wasm_failpaths".to_string(),
        width: 800,
        height: 600,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.register_event::<NetworkEvent>();

    // Check 1: ask for something that is not there. The handle is deliberately dropped — the
    // engine records the failure globally, and reading it back through the public
    // `asset_failures()` is precisely the hook under test.
    let _ = app.load_image_async(MISSING_ASSET);

    // Check 2: connect, then send IMMEDIATELY. `connect` returns with the socket still in
    // `CONNECTING` — the browser cannot complete a handshake without yielding to the event
    // loop, and this is the same synchronous block — so this send is guaranteed to take the
    // pre-open path. That is the whole point: it is the branch nothing else in the tree takes.
    let client = NetworkClient::connect(ECHO_URL);
    client.send_text(PREOPEN_SENTINEL);
    app.world.insert_resource(client);

    app.add_system(NetworkSystem::new());
    app.add_system(FailPathSystem::new());
    app.run();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_wasm_failpaths() {
    run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // Not a silent no-op: someone runs this natively sooner or later and deserves to know why
    // nothing happened.
    println!(
        "wasm_failpaths is a BROWSER check — both defects it exercises live in \
         #[cfg(target_arch = \"wasm32\")] code, so there is nothing to reproduce natively.\n\
         \n\
         Run it with:\n  \
           cargo run --example wasm_failpaths_echo_server\n  \
           scripts/wasm_failpaths_smoke.sh"
    );
}
