//! Shared browser-verdict helper for the `wasm-smokes` CI job.
//!
//! Included by each game that ships a browser smoke:
//!
//! ```ignore
//! #[path = "../shared/web_check.rs"]
//! mod web_check;
//! ```
//!
//! It lives in `examples/shared/` rather than `examples/` because Cargo auto-discovers
//! `examples/*.rs` and `examples/*/main.rs` as example **targets** — a `web_check.rs` one level up
//! would be compiled as a binary with no `main` and break the build. `examples/shared/` is invisible
//! to that discovery, to `scripts/selftests.sh` (which looks for `examples/<name>/<name>.rs`), and to
//! `scripts/build_wasm_examples.sh` (which reads Cargo.toml's `[[example]]` blocks).
//!
//! # Why a page has to verdict itself
//!
//! A native selftest exits with a code. A browser tab has no exit code, so the page has to say what
//! happened somewhere a script outside it can read. The deleted tree settled on
//! **`document.title`**, read live over Chrome's DevTools `/json` endpoint, and that choice is worth
//! keeping for a reason that is not obvious: the title survives a page that is otherwise wedged.
//! A verdict written to the canvas needs the renderer to work, and a verdict `console.log`ged needs
//! the log to be attached before the line is emitted. The title is readable at any time, from
//! outside, without cooperation from the thing under test.
//!
//! ⚠️ **No double quotes in a detail string.** The smoke scripts pull the title out of the DevTools
//! JSON with `grep -oE '<TAG>: [^"]*'`, so a quote truncates the verdict mid-sentence and the
//! script reports a confusing near-miss instead of what the page actually said.
//!
//! # What a check may and may not assume
//!
//! ⚠️ **It runs in real time, never on a frame count.** These pages are driven by the browser's
//! rAF loop against subsystems that live on a wall clock — an `AudioContext` unlocking, a WebSocket
//! handshake, an adapter resolving. That is the same trap `CLAUDE.md` records for `ENGINE_CAPTURE`,
//! one layer out: a check that counted frames would read "no audio" off a device that is playing
//! correctly. Every probe below is handed **elapsed seconds**, not a frame index, and every
//! deadline is in seconds.

#![allow(dead_code)]

use engine::{System, World};

/// What one frame of a browser check decided.
pub enum Step {
    /// Nothing to report yet — run again next frame.
    Pending,
    /// Nothing to report yet, but here is what the probe can currently see.
    ///
    /// The most recent one is folded into the timeout message, which is the difference between
    /// "the check did not finish" and "the check did not finish, and the level was 0.0000 while the
    /// spectrum was flat". Four different sabotages of the audio check all time out; without this
    /// they produce the *same* message, and a sabotage matrix where every row fails identically has
    /// verified that the deadline works, not that the assertions do.
    Waiting(String),
    /// The check finished. `detail` is one line for the smoke script's log, and must contain no
    /// double quotes (see the module docs).
    Done { pass: bool, detail: String },
}

impl Step {
    pub fn pass(detail: impl Into<String>) -> Self {
        Step::Done {
            pass: true,
            detail: detail.into(),
        }
    }

    pub fn fail(detail: impl Into<String>) -> Self {
        Step::Done {
            pass: false,
            detail: detail.into(),
        }
    }
}

/// Runs `probe` every frame until it returns [`Step::Done`] or `deadline` seconds elapse, then
/// publishes `<TAG>: PASS — detail` / `<TAG>: FAIL — detail` where the smoke script can read it.
///
/// A timeout is a **failure**, never a quiet stop: a page that simply never finishes is the exact
/// shape of every browser bug this job exists to catch, and a check that went silent there would
/// report the same "no verdict appeared" as a page that failed to load at all.
pub struct WebCheck<F> {
    tag: &'static str,
    probe: F,
    elapsed: f32,
    deadline: f32,
    finished: bool,
    last_seen: Option<String>,
}

impl<F> WebCheck<F>
where
    F: FnMut(&mut World, f32) -> Step,
{
    /// `tag` is the token the smoke script greps for, e.g. `AUDIO_CHECK`. `deadline` is in seconds
    /// of wall clock.
    pub fn new(tag: &'static str, deadline: f32, probe: F) -> Self {
        Self {
            tag,
            probe,
            elapsed: 0.0,
            deadline,
            finished: false,
            last_seen: None,
        }
    }

    fn publish(&mut self, pass: bool, detail: &str) {
        self.finished = true;
        let verdict = format!(
            "{}: {} — {}",
            self.tag,
            if pass { "PASS" } else { "FAIL" },
            detail.replace('"', "'")
        );
        set_title(&verdict);
        // Also on the console, so `read_console_messages` and a human watching the tab both see it.
        if pass {
            log::info!("{verdict}");
        } else {
            log::error!("{verdict}");
        }
    }
}

impl<F> System for WebCheck<F>
where
    F: FnMut(&mut World, f32) -> Step,
{
    fn run(&mut self, world: &mut World, dt: f32) {
        if self.finished {
            return;
        }
        self.elapsed += dt;
        let step = (self.probe)(world, self.elapsed);
        if let Step::Waiting(note) = &step {
            self.last_seen = Some(note.clone());
        }
        match step {
            Step::Done { pass, detail } => self.publish(pass, &detail),
            Step::Pending | Step::Waiting(_) if self.elapsed >= self.deadline => {
                let elapsed = self.elapsed;
                let detail = match self.last_seen.take() {
                    Some(note) => format!("no verdict within {elapsed:.1} s — last saw {note}"),
                    None => format!("no verdict within {elapsed:.1} s, and the probe never reported what it saw"),
                };
                self.publish(false, &detail);
            }
            _ => {}
        }
    }

    fn name(&self) -> &'static str {
        "WebCheck"
    }
}

/// Writes the verdict where the outside world can read it.
///
/// On wasm that is `document.title`. Natively there is no document, so it goes to stdout — which
/// makes a web check runnable on a desktop while you are writing it, instead of only inside a
/// headless browser behind a shell script.
#[cfg(target_arch = "wasm32")]
fn set_title(verdict: &str) {
    if let Some(doc) = web_sys::window().and_then(|w| w.document()) {
        doc.set_title(verdict);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn set_title(verdict: &str) {
    println!("{verdict}");
}
