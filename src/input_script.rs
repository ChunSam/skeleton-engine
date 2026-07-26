//! Scripted input playback — drive the app from a `(frame, action)` list instead of a human.
//!
//! Verifying a GUI normally means a live, unlocked desktop plus OS automation (on macOS:
//! Accessibility + Screen-Recording permissions, `osascript` key codes, synthetic mouse events).
//! That is the least portable thing in a game's toolchain and it cannot run on CI at all — so the
//! usual fallback is a boot smoke test that proves the app *starts* but is blind to a wrong
//! z-order, a missing icon, or a mispositioned panel.
//!
//! An [`InputScript`] replaces the human: it injects key and mouse events into the very same
//! [`InputState`] the window feeds, at chosen frame numbers, so the app takes the same code path
//! real input does. Combined with headless capture
//! ([`App::capture_frames_headless`](crate::App::capture_frames_headless), or the `ENGINE_CAPTURE`
//! environment variable) a game can drive itself to a specific screen and photograph it under a
//! plain `cargo run` — no window, no display, no OS permissions.
//!
//! ```no_run
//! # use engine::{App, InputScript};
//! # let mut app = App::new();
//! let script = InputScript::from_ron_str(r#"(
//!     events: [
//!         (frame: 10, action: KeyPress("Digit2")),
//!         (frame: 20, action: MouseMove(420.0, 300.0)),
//!         (frame: 21, action: Click("Left")),
//!     ],
//! )"#).unwrap();
//! app.set_input_script(script);
//! ```
//!
//! # Environment variables
//!
//! Both are read by [`App::run`](crate::App::run) on native, so a game needs **no code change**:
//!
//! - `ENGINE_INPUT=<path.ron>` — load and play this script.
//! - `ENGINE_CAPTURE=<frame>:<path.png>[,<frame>:<path.png>…]` — run headlessly and write a PNG
//!   at each listed frame, then exit instead of opening a window.
//!
//! # Frame numbering
//!
//! Frames are counted by the script itself, starting at 0 for the first frame it sees. An event
//! at frame `n` is applied at the **start** of that frame, before the game's systems run, so
//! `just_pressed` reads `true` inside that same frame — exactly as for a real key press.
//! [`InputAction::KeyPress`] and [`InputAction::Click`] release on the **following** frame, so
//! `just_released` behaves like a real tap.

use glam::Vec2;
use serde::Deserialize;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::ecs::World;
use crate::input::InputState;
use crate::resources::ShouldQuit;

/// One scripted input event, applied at the start of its frame.
///
/// `KeyPress`/`Click` are the convenient forms — they press on their frame and release on the
/// next, so a system sees `just_pressed` then `just_released` exactly as for a real tap. The
/// separate `KeyDown`/`KeyUp` (and `MouseDown`/`MouseUp`) forms exist for held input: walking for
/// 30 frames, or a click-drag.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum InputAction {
    /// Press and hold a key until a matching [`KeyUp`](Self::KeyUp).
    KeyDown(KeyCode),
    /// Release a held key.
    KeyUp(KeyCode),
    /// Press a key, releasing it on the next frame (a tap).
    KeyPress(KeyCode),
    /// Move the cursor to a position, in the same logical coordinates
    /// [`InputState::cursor`] reports.
    MouseMove(Vec2),
    /// Press and hold a mouse button until a matching [`MouseUp`](Self::MouseUp).
    MouseDown(MouseButton),
    /// Release a held mouse button.
    MouseUp(MouseButton),
    /// Press a mouse button, releasing it on the next frame (a click).
    Click(MouseButton),
    /// Add to this frame's scroll delta.
    Scroll(f32),
    /// Request app exit, as if the window had been closed.
    Quit,
}

/// A `(frame, action)` pair.
#[derive(Debug, Clone, PartialEq)]
pub struct ScriptedInput {
    /// Frame at which to apply [`action`](Self::action), counted from the script's first frame.
    pub frame: u32,
    pub action: InputAction,
}

/// A frame-indexed list of input events, played into [`InputState`] one frame at a time.
///
/// Insert it with [`App::set_input_script`](crate::App::set_input_script) (or let `ENGINE_INPUT`
/// do it). The engine applies one frame's worth at the start of every update; see the
/// [module docs](self) for frame numbering.
#[derive(Debug, Clone, Default)]
pub struct InputScript {
    /// Sorted by frame; applied in order within a frame.
    events: Vec<ScriptedInput>,
    /// Index of the next unapplied event.
    next: usize,
    /// Frame the next [`apply`](Self::apply) will play.
    frame: u32,
    /// Keys/buttons pressed by a `KeyPress`/`Click`, to release at the start of the next frame.
    pending_keys: Vec<KeyCode>,
    pending_buttons: Vec<MouseButton>,
}

/// Why an [`InputScript`] could not be loaded.
#[derive(Debug)]
#[non_exhaustive]
pub enum InputScriptError {
    /// The RON text did not parse, or an action named a key/button the engine does not know.
    Ron(String),
    /// The script file could not be read.
    Io(String),
}

impl std::fmt::Display for InputScriptError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ron(m) => write!(f, "input script: {m}"),
            Self::Io(m) => write!(f, "input script: {m}"),
        }
    }
}

impl std::error::Error for InputScriptError {}

impl InputScript {
    /// Build a script from `(frame, action)` pairs in code.
    ///
    /// Events may be given in any order; they are sorted by frame, and events sharing a frame
    /// keep their relative order.
    pub fn new(events: impl IntoIterator<Item = (u32, InputAction)>) -> Self {
        let mut events: Vec<ScriptedInput> = events
            .into_iter()
            .map(|(frame, action)| ScriptedInput { frame, action })
            .collect();
        // Stable so same-frame events keep the order they were authored in.
        events.sort_by_key(|e| e.frame);
        Self {
            events,
            ..Default::default()
        }
    }

    /// Parse a script from RON.
    ///
    /// ```ron
    /// (
    ///     events: [
    ///         (frame: 10, action: KeyPress("Digit2")),
    ///         (frame: 20, action: MouseMove(420.0, 300.0)),
    ///         (frame: 21, action: Click("Left")),
    ///         (frame: 90, action: Quit),
    ///     ],
    /// )
    /// ```
    ///
    /// Keys are named as in [`winit::keyboard::KeyCode`] (`"KeyA"`, `"Digit1"`, `"Space"`,
    /// `"ArrowUp"`, `"Escape"`, …) and buttons as `"Left"`, `"Right"` or `"Middle"`. An
    /// unknown name is an error rather than a silently dropped event — a typo in a
    /// verification script would otherwise look like a failing feature.
    pub fn from_ron_str(ron_text: &str) -> Result<Self, InputScriptError> {
        let file: ScriptFile =
            ron::from_str(ron_text).map_err(|e| InputScriptError::Ron(e.to_string()))?;
        let mut events = Vec::with_capacity(file.events.len());
        for entry in file.events {
            events.push(ScriptedInput {
                frame: entry.frame,
                action: entry.action.resolve()?,
            });
        }
        events.sort_by_key(|e| e.frame);
        Ok(Self {
            events,
            ..Default::default()
        })
    }

    /// Load a script from a RON file (resolved against the engine's asset roots).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &str) -> Result<Self, InputScriptError> {
        let text = std::fs::read_to_string(crate::asset_path::resolve(path))
            .map_err(|e| InputScriptError::Io(format!("{path}: {e}")))?;
        Self::from_ron_str(&text)
    }

    /// Frame the next [`apply`](Self::apply) will play (0 before the first).
    pub fn frame(&self) -> u32 {
        self.frame
    }

    /// Number of events in the script.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// `true` when the script holds no events.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// `true` once every event has been applied and nothing is waiting to be released.
    pub fn is_finished(&self) -> bool {
        self.next >= self.events.len()
            && self.pending_keys.is_empty()
            && self.pending_buttons.is_empty()
    }

    /// The frame of the last event, i.e. how long a run must be for the script to finish.
    pub fn last_frame(&self) -> u32 {
        self.events.last().map(|e| e.frame).unwrap_or(0)
    }

    /// Play one frame: release what a previous `KeyPress`/`Click` held, apply this frame's
    /// events, then advance the frame counter.
    ///
    /// Called by the engine at the start of each update; call it directly only when driving
    /// `World` updates yourself.
    pub fn apply(&mut self, world: &mut World) {
        let mut quit = false;
        if let Some(input) = world.resource_mut::<InputState>() {
            // Releases first: a tap pressed on frame n reports `just_released` on frame n+1,
            // matching a real key-up arriving in the next frame's events.
            for key in self.pending_keys.drain(..) {
                input.release(key);
            }
            for btn in self.pending_buttons.drain(..) {
                input.release_mouse(btn);
            }
            while let Some(event) = self.events.get(self.next) {
                if event.frame != self.frame {
                    // Events are sorted, so a later frame ends this frame's batch. An event
                    // whose frame already passed (only possible if `apply` skipped frames) is
                    // applied now rather than dropped.
                    if event.frame > self.frame {
                        break;
                    }
                }
                match event.action.clone() {
                    InputAction::KeyDown(k) => input.press(k),
                    InputAction::KeyUp(k) => input.release(k),
                    InputAction::KeyPress(k) => {
                        input.press(k);
                        self.pending_keys.push(k);
                    }
                    InputAction::MouseMove(p) => input.set_cursor(p),
                    InputAction::MouseDown(b) => input.press_mouse(b),
                    InputAction::MouseUp(b) => input.release_mouse(b),
                    InputAction::Click(b) => {
                        input.press_mouse(b);
                        self.pending_buttons.push(b);
                    }
                    InputAction::Scroll(d) => input.add_scroll(d),
                    InputAction::Quit => quit = true,
                }
                self.next += 1;
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        self.frame = self.frame.saturating_add(1);
    }
}

// ── RON mirror ────────────────────────────────────────────────────────────────
//
// `KeyCode`/`MouseButton` come from winit without serde support, and naming keys by string is
// friendlier in a hand-written script anyway. These private types carry the file shape and are
// resolved into the runtime enum above — the same serde-mirror split the particle/trigger-zone
// config sets use.

#[derive(Deserialize)]
struct ScriptFile {
    events: Vec<ScriptEntry>,
}

#[derive(Deserialize)]
struct ScriptEntry {
    frame: u32,
    action: ActionDef,
}

#[derive(Deserialize)]
enum ActionDef {
    KeyDown(String),
    KeyUp(String),
    KeyPress(String),
    MouseMove(f32, f32),
    MouseDown(String),
    MouseUp(String),
    Click(String),
    Scroll(f32),
    Quit,
}

impl ActionDef {
    fn resolve(self) -> Result<InputAction, InputScriptError> {
        Ok(match self {
            Self::KeyDown(k) => InputAction::KeyDown(key(&k)?),
            Self::KeyUp(k) => InputAction::KeyUp(key(&k)?),
            Self::KeyPress(k) => InputAction::KeyPress(key(&k)?),
            Self::MouseMove(x, y) => InputAction::MouseMove(Vec2::new(x, y)),
            Self::MouseDown(b) => InputAction::MouseDown(button(&b)?),
            Self::MouseUp(b) => InputAction::MouseUp(button(&b)?),
            Self::Click(b) => InputAction::Click(button(&b)?),
            Self::Scroll(d) => InputAction::Scroll(d),
            Self::Quit => InputAction::Quit,
        })
    }
}

fn key(name: &str) -> Result<KeyCode, InputScriptError> {
    key_from_name(name).ok_or_else(|| InputScriptError::Ron(format!("unknown key name '{name}'")))
}

fn button(name: &str) -> Result<MouseButton, InputScriptError> {
    mouse_button_from_name(name)
        .ok_or_else(|| InputScriptError::Ron(format!("unknown mouse button '{name}'")))
}

/// Resolve a [`winit::keyboard::KeyCode`] from its variant name (`"KeyA"`, `"Space"`, `"F5"`, …).
///
/// Covers the letter, digit, function, navigation, modifier, punctuation and numpad keys a game
/// script needs; returns `None` for anything else (`KeyCode` is `#[non_exhaustive]`, so the rarer
/// media/international keys are deliberately not enumerated).
pub fn key_from_name(name: &str) -> Option<KeyCode> {
    Some(match name {
        "KeyA" => KeyCode::KeyA,
        "KeyB" => KeyCode::KeyB,
        "KeyC" => KeyCode::KeyC,
        "KeyD" => KeyCode::KeyD,
        "KeyE" => KeyCode::KeyE,
        "KeyF" => KeyCode::KeyF,
        "KeyG" => KeyCode::KeyG,
        "KeyH" => KeyCode::KeyH,
        "KeyI" => KeyCode::KeyI,
        "KeyJ" => KeyCode::KeyJ,
        "KeyK" => KeyCode::KeyK,
        "KeyL" => KeyCode::KeyL,
        "KeyM" => KeyCode::KeyM,
        "KeyN" => KeyCode::KeyN,
        "KeyO" => KeyCode::KeyO,
        "KeyP" => KeyCode::KeyP,
        "KeyQ" => KeyCode::KeyQ,
        "KeyR" => KeyCode::KeyR,
        "KeyS" => KeyCode::KeyS,
        "KeyT" => KeyCode::KeyT,
        "KeyU" => KeyCode::KeyU,
        "KeyV" => KeyCode::KeyV,
        "KeyW" => KeyCode::KeyW,
        "KeyX" => KeyCode::KeyX,
        "KeyY" => KeyCode::KeyY,
        "KeyZ" => KeyCode::KeyZ,
        "Digit0" => KeyCode::Digit0,
        "Digit1" => KeyCode::Digit1,
        "Digit2" => KeyCode::Digit2,
        "Digit3" => KeyCode::Digit3,
        "Digit4" => KeyCode::Digit4,
        "Digit5" => KeyCode::Digit5,
        "Digit6" => KeyCode::Digit6,
        "Digit7" => KeyCode::Digit7,
        "Digit8" => KeyCode::Digit8,
        "Digit9" => KeyCode::Digit9,
        "F1" => KeyCode::F1,
        "F2" => KeyCode::F2,
        "F3" => KeyCode::F3,
        "F4" => KeyCode::F4,
        "F5" => KeyCode::F5,
        "F6" => KeyCode::F6,
        "F7" => KeyCode::F7,
        "F8" => KeyCode::F8,
        "F9" => KeyCode::F9,
        "F10" => KeyCode::F10,
        "F11" => KeyCode::F11,
        "F12" => KeyCode::F12,
        "Escape" => KeyCode::Escape,
        "Space" => KeyCode::Space,
        "Enter" => KeyCode::Enter,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Delete" => KeyCode::Delete,
        "Insert" => KeyCode::Insert,
        "Home" => KeyCode::Home,
        "End" => KeyCode::End,
        "PageUp" => KeyCode::PageUp,
        "PageDown" => KeyCode::PageDown,
        "ArrowUp" => KeyCode::ArrowUp,
        "ArrowDown" => KeyCode::ArrowDown,
        "ArrowLeft" => KeyCode::ArrowLeft,
        "ArrowRight" => KeyCode::ArrowRight,
        "ShiftLeft" => KeyCode::ShiftLeft,
        "ShiftRight" => KeyCode::ShiftRight,
        "ControlLeft" => KeyCode::ControlLeft,
        "ControlRight" => KeyCode::ControlRight,
        "AltLeft" => KeyCode::AltLeft,
        "AltRight" => KeyCode::AltRight,
        "SuperLeft" => KeyCode::SuperLeft,
        "SuperRight" => KeyCode::SuperRight,
        "Minus" => KeyCode::Minus,
        "Equal" => KeyCode::Equal,
        "BracketLeft" => KeyCode::BracketLeft,
        "BracketRight" => KeyCode::BracketRight,
        "Backslash" => KeyCode::Backslash,
        "Semicolon" => KeyCode::Semicolon,
        "Quote" => KeyCode::Quote,
        "Comma" => KeyCode::Comma,
        "Period" => KeyCode::Period,
        "Slash" => KeyCode::Slash,
        "Backquote" => KeyCode::Backquote,
        "CapsLock" => KeyCode::CapsLock,
        "NumLock" => KeyCode::NumLock,
        "Numpad0" => KeyCode::Numpad0,
        "Numpad1" => KeyCode::Numpad1,
        "Numpad2" => KeyCode::Numpad2,
        "Numpad3" => KeyCode::Numpad3,
        "Numpad4" => KeyCode::Numpad4,
        "Numpad5" => KeyCode::Numpad5,
        "Numpad6" => KeyCode::Numpad6,
        "Numpad7" => KeyCode::Numpad7,
        "Numpad8" => KeyCode::Numpad8,
        "Numpad9" => KeyCode::Numpad9,
        "NumpadEnter" => KeyCode::NumpadEnter,
        "NumpadAdd" => KeyCode::NumpadAdd,
        "NumpadSubtract" => KeyCode::NumpadSubtract,
        "NumpadMultiply" => KeyCode::NumpadMultiply,
        "NumpadDivide" => KeyCode::NumpadDivide,
        "NumpadDecimal" => KeyCode::NumpadDecimal,
        _ => return None,
    })
}

/// Resolve a mouse button from `"Left"`, `"Right"` or `"Middle"` (also `"Back"`/`"Forward"`).
pub fn mouse_button_from_name(name: &str) -> Option<MouseButton> {
    Some(match name {
        "Left" => MouseButton::Left,
        "Right" => MouseButton::Right,
        "Middle" => MouseButton::Middle,
        "Back" => MouseButton::Back,
        "Forward" => MouseButton::Forward,
        _ => return None,
    })
}

/// Every key name [`key_from_name`] accepts, for diagnostics and editor tooling.
pub fn key_names() -> Vec<&'static str> {
    KEY_NAMES.to_vec()
}

static KEY_NAMES: &[&str] = &[
    "KeyA",
    "KeyB",
    "KeyC",
    "KeyD",
    "KeyE",
    "KeyF",
    "KeyG",
    "KeyH",
    "KeyI",
    "KeyJ",
    "KeyK",
    "KeyL",
    "KeyM",
    "KeyN",
    "KeyO",
    "KeyP",
    "KeyQ",
    "KeyR",
    "KeyS",
    "KeyT",
    "KeyU",
    "KeyV",
    "KeyW",
    "KeyX",
    "KeyY",
    "KeyZ",
    "Digit0",
    "Digit1",
    "Digit2",
    "Digit3",
    "Digit4",
    "Digit5",
    "Digit6",
    "Digit7",
    "Digit8",
    "Digit9",
    "F1",
    "F2",
    "F3",
    "F4",
    "F5",
    "F6",
    "F7",
    "F8",
    "F9",
    "F10",
    "F11",
    "F12",
    "Escape",
    "Space",
    "Enter",
    "Tab",
    "Backspace",
    "Delete",
    "Insert",
    "Home",
    "End",
    "PageUp",
    "PageDown",
    "ArrowUp",
    "ArrowDown",
    "ArrowLeft",
    "ArrowRight",
    "ShiftLeft",
    "ShiftRight",
    "ControlLeft",
    "ControlRight",
    "AltLeft",
    "AltRight",
    "SuperLeft",
    "SuperRight",
    "Minus",
    "Equal",
    "BracketLeft",
    "BracketRight",
    "Backslash",
    "Semicolon",
    "Quote",
    "Comma",
    "Period",
    "Slash",
    "Backquote",
    "CapsLock",
    "NumLock",
    "Numpad0",
    "Numpad1",
    "Numpad2",
    "Numpad3",
    "Numpad4",
    "Numpad5",
    "Numpad6",
    "Numpad7",
    "Numpad8",
    "Numpad9",
    "NumpadEnter",
    "NumpadAdd",
    "NumpadSubtract",
    "NumpadMultiply",
    "NumpadDivide",
    "NumpadDecimal",
];

// ── Environment-driven entry points ───────────────────────────────────────────

/// Parse `ENGINE_CAPTURE` into `(frame, path)` pairs, or `None` when it is unset/empty.
///
/// Format: `<frame>:<path>` entries separated by commas, e.g.
/// `ENGINE_CAPTURE=60:/tmp/menu.png,150:/tmp/shop.png`. A malformed entry is reported and
/// skipped rather than silently ignored — a typo in a verification command would otherwise look
/// like a feature that failed to render.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn capture_plan_from_env() -> Option<Vec<(u32, String)>> {
    let spec = std::env::var("ENGINE_CAPTURE").ok()?;
    let plan = parse_capture_plan(&spec);
    if plan.is_empty() {
        log::error!("ENGINE_CAPTURE='{spec}' contained no usable <frame>:<path> entry");
        return None;
    }
    Some(plan)
}

/// Pure parser behind [`capture_plan_from_env`], split out so it is testable without env vars.
#[cfg(not(target_arch = "wasm32"))]
fn parse_capture_plan(spec: &str) -> Vec<(u32, String)> {
    let mut plan = Vec::new();
    for entry in spec.split(',').map(str::trim).filter(|e| !e.is_empty()) {
        // Split on the FIRST colon only: a Windows path ("C:\shots\a.png") contains one too.
        match entry.split_once(':') {
            Some((frame, path)) if !path.is_empty() => match frame.trim().parse::<u32>() {
                Ok(frame) => plan.push((frame, path.trim().to_string())),
                Err(_) => {
                    log::error!("ENGINE_CAPTURE entry '{entry}': '{frame}' is not a frame number")
                }
            },
            _ => log::error!("ENGINE_CAPTURE entry '{entry}': expected <frame>:<path>"),
        }
    }
    plan
}

impl crate::app::App {
    /// Play `script` into this app's input, starting from the next update.
    ///
    /// The script is registered persistent, so a scene change does not cancel a run in progress.
    pub fn set_input_script(&mut self, script: InputScript) -> &mut Self {
        self.register_persistent::<InputScript>();
        self.world.insert_resource(script);
        self
    }

    /// Load the script named by `ENGINE_INPUT`, if set. A load failure is logged and ignored so a
    /// bad path cannot stop the game from running normally.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn apply_input_script_env(&mut self) {
        let Ok(path) = std::env::var("ENGINE_INPUT") else {
            return;
        };
        match InputScript::load(&path) {
            Ok(script) => {
                log::info!(
                    "ENGINE_INPUT: playing {} event(s) from {path} (last frame {})",
                    script.len(),
                    script.last_frame()
                );
                self.set_input_script(script);
            }
            Err(err) => log::error!("ENGINE_INPUT: {err}"),
        }
    }

    /// Advance the input script by one frame, if one is installed. Called at the start of every
    /// update, before the game's systems run, so an injected press reads as `just_pressed`
    /// within its own frame.
    pub(crate) fn apply_input_script_frame(&mut self) {
        let Some(mut script) = self.world.remove_resource::<InputScript>() else {
            return;
        };
        script.apply(&mut self.world);
        self.world.insert_resource(script);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One frame of the engine loop as far as input is concerned: play the script, let a
    /// "system" observe, then flush — which is what `App::update` does around the systems.
    fn world_with_input() -> World {
        let mut world = World::new();
        world.insert_resource(InputState::default());
        world.insert_resource(ShouldQuit(false));
        world
    }

    fn flush(world: &mut World) {
        world.resource_mut::<InputState>().expect("input").flush();
    }

    fn input(world: &World) -> &InputState {
        world.resource::<InputState>().expect("input")
    }

    #[test]
    fn a_scripted_press_reads_as_just_pressed_in_its_own_frame() {
        let mut world = world_with_input();
        let mut script = InputScript::new([(2, InputAction::KeyPress(KeyCode::Space))]);

        for frame in 0..2 {
            script.apply(&mut world);
            assert!(
                !input(&world).just_pressed(KeyCode::Space),
                "nothing scripted for frame {frame}"
            );
            flush(&mut world);
        }

        // Frame 2 — the scripted frame.
        script.apply(&mut world);
        assert!(input(&world).just_pressed(KeyCode::Space));
        assert!(input(&world).is_pressed(KeyCode::Space));
        flush(&mut world);

        // Frame 3 — a tap releases on the following frame, like a real key-up event.
        script.apply(&mut world);
        assert!(input(&world).just_released(KeyCode::Space));
        assert!(!input(&world).is_pressed(KeyCode::Space));
        assert!(script.is_finished());
    }

    #[test]
    fn key_down_is_held_until_key_up() {
        let mut world = world_with_input();
        let mut script = InputScript::new([
            (0, InputAction::KeyDown(KeyCode::KeyD)),
            (3, InputAction::KeyUp(KeyCode::KeyD)),
        ]);

        for frame in 0..3 {
            script.apply(&mut world);
            assert!(
                input(&world).is_pressed(KeyCode::KeyD),
                "still held on frame {frame}"
            );
            flush(&mut world);
        }
        script.apply(&mut world);
        assert!(!input(&world).is_pressed(KeyCode::KeyD));
        assert!(input(&world).just_released(KeyCode::KeyD));
    }

    #[test]
    fn a_scripted_click_moves_the_cursor_and_clicks_there() {
        let mut world = world_with_input();
        let mut script = InputScript::new([
            (0, InputAction::MouseMove(Vec2::new(120.0, 240.0))),
            (0, InputAction::Click(MouseButton::Left)),
        ]);

        script.apply(&mut world);
        assert_eq!(input(&world).cursor(), Vec2::new(120.0, 240.0));
        assert!(input(&world).mouse_just_pressed(MouseButton::Left));
        // The press position is what a widget hit-tests against.
        assert_eq!(
            input(&world).mouse_press_cursor(MouseButton::Left),
            Vec2::new(120.0, 240.0)
        );
        flush(&mut world);

        script.apply(&mut world);
        assert!(input(&world).mouse_just_released(MouseButton::Left));
    }

    #[test]
    fn scroll_and_quit_reach_their_resources() {
        let mut world = world_with_input();
        let mut script = InputScript::new([(0, InputAction::Scroll(-2.5)), (1, InputAction::Quit)]);

        script.apply(&mut world);
        assert_eq!(input(&world).scroll(), -2.5);
        assert!(!world.resource::<ShouldQuit>().expect("quit").0);
        flush(&mut world);

        script.apply(&mut world);
        assert!(world.resource::<ShouldQuit>().expect("quit").0);
    }

    /// Events authored out of order still play in frame order, and two events sharing a frame
    /// keep the order they were written in (move-then-click must not become click-then-move).
    #[test]
    fn events_are_sorted_by_frame_and_stable_within_one() {
        let script = InputScript::new([
            (5, InputAction::KeyPress(KeyCode::KeyB)),
            (1, InputAction::MouseMove(Vec2::ZERO)),
            (1, InputAction::Click(MouseButton::Left)),
        ]);
        let frames: Vec<u32> = script.events.iter().map(|e| e.frame).collect();
        assert_eq!(frames, vec![1, 1, 5]);
        assert!(matches!(script.events[0].action, InputAction::MouseMove(_)));
        assert!(matches!(script.events[1].action, InputAction::Click(_)));
        assert_eq!(script.last_frame(), 5);
    }

    /// A run shorter than the script must not drop events: catching up applies what is due.
    #[test]
    fn an_event_whose_frame_passed_is_not_dropped() {
        let mut world = world_with_input();
        let mut script = InputScript::new([(0, InputAction::KeyDown(KeyCode::KeyA))]);
        // Skip ahead as if the caller had advanced its own counter.
        script.frame = 4;
        script.apply(&mut world);
        assert!(input(&world).is_pressed(KeyCode::KeyA));
    }

    #[test]
    fn parses_a_ron_script() {
        let script = InputScript::from_ron_str(
            r#"(
                events: [
                    (frame: 10, action: KeyPress("Digit2")),
                    (frame: 20, action: MouseMove(420.0, 300.0)),
                    (frame: 21, action: Click("Left")),
                    (frame: 90, action: Quit),
                ],
            )"#,
        )
        .expect("parse");
        assert_eq!(script.len(), 4);
        assert_eq!(script.last_frame(), 90);
        assert_eq!(
            script.events[0].action,
            InputAction::KeyPress(KeyCode::Digit2)
        );
        assert_eq!(
            script.events[1].action,
            InputAction::MouseMove(Vec2::new(420.0, 300.0))
        );
        assert_eq!(
            script.events[2].action,
            InputAction::Click(MouseButton::Left)
        );
    }

    /// A typo must fail loudly: a silently dropped event looks exactly like a broken feature.
    #[test]
    fn an_unknown_key_name_is_an_error() {
        let err = InputScript::from_ron_str(r#"(events: [(frame: 0, action: KeyPress("Nope"))])"#)
            .expect_err("should reject");
        let msg = err.to_string();
        assert!(msg.contains("Nope"), "error should name the typo: {msg}");
    }

    #[test]
    fn key_and_button_names_resolve() {
        assert_eq!(key_from_name("KeyA"), Some(KeyCode::KeyA));
        assert_eq!(key_from_name("Space"), Some(KeyCode::Space));
        assert_eq!(key_from_name("ArrowUp"), Some(KeyCode::ArrowUp));
        assert_eq!(key_from_name("F5"), Some(KeyCode::F5));
        assert_eq!(key_from_name("Numpad7"), Some(KeyCode::Numpad7));
        assert_eq!(key_from_name("keya"), None, "names are case-sensitive");
        assert_eq!(mouse_button_from_name("Middle"), Some(MouseButton::Middle));
        assert_eq!(mouse_button_from_name("Fourth"), None);
        // Every advertised name must actually resolve.
        for name in key_names() {
            assert!(
                key_from_name(name).is_some(),
                "{name} is advertised but unknown"
            );
        }
    }

    #[test]
    fn an_empty_script_is_finished_immediately() {
        let script = InputScript::default();
        assert!(script.is_empty());
        assert!(script.is_finished());
        assert_eq!(script.last_frame(), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn capture_plan_parses_frames_and_paths() {
        let plan = parse_capture_plan("60:/tmp/menu.png, 150:/tmp/shop.png");
        assert_eq!(
            plan,
            vec![
                (60, "/tmp/menu.png".to_string()),
                (150, "/tmp/shop.png".to_string())
            ]
        );
    }

    /// A Windows path carries its own colon, so only the FIRST one separates frame from path.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn capture_plan_keeps_a_drive_letter_in_the_path() {
        let plan = parse_capture_plan(r"12:C:\shots\a.png");
        assert_eq!(plan, vec![(12, r"C:\shots\a.png".to_string())]);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn capture_plan_skips_malformed_entries() {
        let plan = parse_capture_plan("nope:/tmp/a.png,,7:/tmp/b.png,8:");
        assert_eq!(
            plan,
            vec![(7, "/tmp/b.png".to_string())],
            "only the well-formed entry survives"
        );
    }
}
