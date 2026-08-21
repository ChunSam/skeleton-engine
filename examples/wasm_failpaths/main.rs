//! `wasm_failpaths` — a page that takes two failure paths **on purpose**, and reports what happened.
//!
//! # Why this is not a game, and why it lives here anyway
//!
//! Every other browser smoke drives a path that is supposed to succeed, and passes when nothing
//! goes wrong. That leaves a blind spot the engine has been bitten by twice: a *failure* handler
//! can be completely broken while every green check stays green, because nothing ever fails on
//! purpose. Both bugs below were fixed in v0.150.1 / v0.150.2 and shipped **compile-verified
//! only** — no automated check could reach either.
//!
//! ⚠️ **This is deliberately `main.rs`, not `wasm_failpaths.rs`.** `scripts/selftests.sh` defines a
//! *game* as `examples/<name>/<name>.rs` and requires every game to carry a `<NAME>_SELFTEST`. This
//! is a browser harness, not a game: it has no play, and its two defects live entirely in
//! `#[cfg(target_arch = "wasm32")]` code, so a native selftest could only ever print a skip — and
//! "a skip is not a pass" is the rule that layout exists to enforce. Cargo auto-discovers
//! `examples/*/main.rs`, and an explicit `[[example]]` block in `Cargo.toml` puts it in front of
//! `scripts/build_wasm_examples.sh`. It **is** gated — by `scripts/wasm_failpaths_smoke.sh` in the
//! `wasm-smokes` CI job, which is a stronger gate than a native selftest could be here.
//!
//! # The two paths
//!
//! | # | What it does on purpose | What used to happen |
//! |---|---|---|
//! | 1 | `load_image_async` on a URL that 404s | The failure set `AssetLoadState::Failed` but never called `record_failure`, so `asset_failures()` stayed **empty** and `set_strict_assets` never fired. Both are documented as the way to refuse to start on a missing asset; both were native-only in practice. On the web a 404 painted magenta and said nothing either could act on. |
//! | 2 | `send_text` **immediately after `connect`**, while the socket is still `CONNECTING` | The web client handed it to a `CONNECTING` socket, which **throws** — the message was silently gone. Native queues it in a `sync_channel` and delivers it on open, so the same game lost its join packet on the web and nowhere else. |
//!
//! Check 2 is why the echo server exists: send before open, and the message must come back.
//! ⚠️ `netplay_game` also sends before open (its `ClientMsg::Join`), but the server ignores that
//! message's *content*, so its arrival is unobservable — which is exactly how this bug survived.
//! A sentinel that must be echoed is the only way to see it.
//!
//! # Running it
//!
//! ```text
//! cargo run --example wasm_failpaths_echo_server        # terminal 1
//! examples/wasm_failpaths/web/build.sh                  # terminal 2
//! python3 -m http.server 8092 --directory examples/wasm_failpaths/web
//! # then open http://localhost:8092
//! ```
//!
//! or just `scripts/wasm_failpaths_smoke.sh`, which does all of it headlessly.

// Only `web_check_failpaths` uses this; gated so a native build does not compile a module it
// cannot reach.
#[cfg(target_arch = "wasm32")]
#[path = "../shared/web_check.rs"]
mod web_check;

/// The URL check 1 asks for. It **must not exist** — that is the whole point — and the `404-`
/// prefix is a hint to anyone who finds it in a server log wondering what broke.
#[cfg(target_arch = "wasm32")]
const MISSING_ASSET: &str = "assets/404-this-asset-does-not-exist.png";

/// The payload check 2 sends before the socket opens and expects to see echoed back.
#[cfg(target_arch = "wasm32")]
const PREOPEN_SENTINEL: &str = "PREOPEN-SENTINEL";

/// Where the echo server listens. Kept in sync with `echo_server.rs` by hand — a wasm build has no
/// environment to read, so the page can only ever dial a compiled-in address, and hardcoding one
/// string in two files beats inventing a config channel for a smoke. `9007` is clear of
/// `netplay_server`'s `9006` so both can run at once.
#[cfg(target_arch = "wasm32")]
const ECHO_URL: &str = "ws://127.0.0.1:9007";

/// Generous: a cold wasm start, plus a fetch, plus a WebSocket handshake, on a software renderer,
/// on a loaded CI runner. The cost of being generous is paid only when something is already broken.
#[cfg(target_arch = "wasm32")]
const DEADLINE_SECS: f32 = 25.0;

/// Runs both deliberate failures and publishes the verdict to `document.title`, where
/// `scripts/wasm_failpaths_smoke.sh` reads it over Chrome's DevTools endpoint.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn web_check_failpaths() {
    use engine::{App, NetworkClient, NetworkEvent, NetworkSystem, WindowConfig};
    use web_check::{Step, WebCheck};

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "wasm_failpaths".to_string(),
        width: 800,
        height: 600,
        ..Default::default()
    });
    app.register_event::<NetworkEvent>();

    // ── Failure 1 ────────────────────────────────────────────────────────────────────────────
    // Ask for something that is not there. The handle is deliberately dropped: the engine records
    // the failure *globally*, and reading it back through the public `asset_failures()` is
    // precisely the hook under test — not the handle's own load state, which was never the broken
    // half.
    let _ = app.load_image_async(MISSING_ASSET);

    // ── Failure 2 ────────────────────────────────────────────────────────────────────────────
    // Connect, then send IMMEDIATELY. `connect` returns with the socket still `CONNECTING` — a
    // browser cannot complete a handshake without yielding to the event loop, and this is the same
    // synchronous block — so this send is *guaranteed* to take the pre-open path. That is the
    // whole point: it is the branch nothing else in the tree deliberately takes.
    let client = NetworkClient::connect(ECHO_URL);
    client.send_text(PREOPEN_SENTINEL);
    app.world.insert_resource(client);

    app.add_system(NetworkSystem::new());

    let mut asset_reported = false;
    let mut preopen_echoed = false;

    app.add_system(WebCheck::new(
        "FAILPATH_CHECK",
        DEADLINE_SECS,
        move |world, _t| {
            // `record_failure` runs inside the fetch future rather than the frame poll, so this
            // becomes true on its own once the browser resolves the request.
            if !asset_reported {
                asset_reported = engine::asset_path::asset_failures()
                    .iter()
                    .any(|f| f.path.contains("404-this-asset-does-not-exist"));
            }

            // The bus is drained every frame, so the sentinel is visible for exactly one frame —
            // latch it rather than reading the current frame only.
            if !preopen_echoed {
                if let Some(bus) = world.resource::<engine::Events<NetworkEvent>>() {
                    preopen_echoed = bus.read().iter().any(|event| {
                        matches!(event, NetworkEvent::TextMessage(t) if t == PREOPEN_SENTINEL)
                    });
                }
            }

            if asset_reported && preopen_echoed {
                return Step::pass(
                    "a 404 reached asset_failures() and a send issued before the socket opened \
                     came back from the echo server",
                );
            }
            // Report both halves every frame so a timeout names WHICH failure handler is broken.
            // Without this both sabotages produce the same 'no verdict' message, and a matrix
            // where every row fails identically has verified the deadline, not the assertions.
            Step::Waiting(format!(
                "asset_failures saw the 404: {asset_reported} · the pre-open send came back: \
                 {preopen_echoed}"
            ))
        },
    ));

    app.run();
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
           scripts/wasm_failpaths_smoke.sh\n\
         \n\
         or by hand:\n  \
           cargo run --example wasm_failpaths_echo_server\n  \
           examples/wasm_failpaths/web/build.sh\n  \
           python3 -m http.server 8092 --directory examples/wasm_failpaths/web"
    );
}
