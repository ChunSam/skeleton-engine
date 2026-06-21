//! macOS GameController-framework gamepad backend.
//!
//! On macOS the IOKit-HID `gilrs` backend can't read modern Bluetooth/USB Xbox & PS5 pads — Apple's
//! GameController framework claims them (their HID device shows
//! `IOUserServerName = com.apple.gamecontroller.driver.*`), so gilrs enumerates the pad but gets
//! zero input reports. This module polls the GameController framework each frame and feeds the
//! engine's [`GamepadState`] instead; `gilrs` is left uninitialized on macOS (see `App::new`).
//!
//! The bug and this backend's premise were confirmed empirically with `examples/gamepad_probe`
//! (gilrs/HID reads all-zero while GameController reads full stick/button/trigger input).

// Self-contained OS gate: the `mod` declaration in `input/mod.rs` is already cfg-gated, but
// pinning it here too means the file can never be compiled on Linux/CI (where the objc2
// GameController bindings don't exist) even if an extra include path is added later.
#![cfg(target_os = "macos")]

use crate::input::{GamepadAxis, GamepadButton, GamepadState};
use objc2_game_controller::{GCController, GCExtendedGamepad};
use std::collections::{HashMap, HashSet};

/// Polls the GameController framework and writes each connected pad into `state` (slots 0..4, in
/// controller-array order). Slots past the live controller count are disconnected.
///
/// Stick / D-pad Y is negated to match the engine's `AxisBinding` convention (up = −Y), so games
/// written against the gilrs path behave identically on macOS.
pub(crate) fn poll(state: &mut GamepadState) {
    // SAFETY: `GCController::controllers()` and the element accessors are read on the main thread
    // (this runs from the winit event loop), where the framework services its run loop and updates
    // the element snapshots. Each call only reads a retained snapshot value.
    let controllers = unsafe { GCController::controllers() };
    let count = controllers.len();
    for pad in 0..4 {
        let controller = (pad < count)
            .then(|| controllers.get_retained(pad))
            .flatten();
        let Some(controller) = controller else {
            state.disconnect_macos(pad);
            continue;
        };
        match unsafe { controller.extendedGamepad() } {
            Some(gp) => {
                let (buttons, axes) = unsafe { read_extended(&gp) };
                state.apply_macos_snapshot(pad, buttons, axes);
            }
            // Connected but no extended (dual-stick) profile — keep the slot present but empty.
            None => state.apply_macos_snapshot(pad, HashSet::new(), HashMap::new()),
        }
    }
}

fn push(set: &mut HashSet<GamepadButton>, button: GamepadButton, held: bool) {
    if held {
        set.insert(button);
    }
}

/// Reads the held buttons + axis values from a `GCExtendedGamepad`.
///
/// # Safety
/// Must be called on the main thread (see [`poll`]).
unsafe fn read_extended(
    gp: &GCExtendedGamepad,
) -> (HashSet<GamepadButton>, HashMap<GamepadAxis, f32>) {
    let mut buttons = HashSet::new();

    // Face buttons.
    push(&mut buttons, GamepadButton::South, gp.buttonA().isPressed());
    push(&mut buttons, GamepadButton::East, gp.buttonB().isPressed());
    push(&mut buttons, GamepadButton::West, gp.buttonX().isPressed());
    push(&mut buttons, GamepadButton::North, gp.buttonY().isPressed());
    // Shoulders + triggers (as digital buttons).
    push(
        &mut buttons,
        GamepadButton::LeftBumper,
        gp.leftShoulder().isPressed(),
    );
    push(
        &mut buttons,
        GamepadButton::RightBumper,
        gp.rightShoulder().isPressed(),
    );
    push(
        &mut buttons,
        GamepadButton::LeftTrigger,
        gp.leftTrigger().isPressed(),
    );
    push(
        &mut buttons,
        GamepadButton::RightTrigger,
        gp.rightTrigger().isPressed(),
    );
    // Menu / Options → Start / Select. `buttonOptions` is absent on pads that lack it.
    push(
        &mut buttons,
        GamepadButton::Start,
        gp.buttonMenu().isPressed(),
    );
    if let Some(opt) = gp.buttonOptions() {
        push(&mut buttons, GamepadButton::Select, opt.isPressed());
    }
    // Thumbstick clicks (optional on some pads).
    if let Some(l3) = gp.leftThumbstickButton() {
        push(&mut buttons, GamepadButton::LeftThumb, l3.isPressed());
    }
    if let Some(r3) = gp.rightThumbstickButton() {
        push(&mut buttons, GamepadButton::RightThumb, r3.isPressed());
    }
    // D-pad (buttons).
    let dpad = gp.dpad();
    push(&mut buttons, GamepadButton::DPadUp, dpad.up().isPressed());
    push(
        &mut buttons,
        GamepadButton::DPadDown,
        dpad.down().isPressed(),
    );
    push(
        &mut buttons,
        GamepadButton::DPadLeft,
        dpad.left().isPressed(),
    );
    push(
        &mut buttons,
        GamepadButton::DPadRight,
        dpad.right().isPressed(),
    );

    // Axes — negate Y so up = −Y (engine `AxisBinding` convention; matches the gilrs path).
    let ls = gp.leftThumbstick();
    let rs = gp.rightThumbstick();
    let axes = HashMap::from([
        (GamepadAxis::LeftStickX, ls.xAxis().value()),
        (GamepadAxis::LeftStickY, -ls.yAxis().value()),
        (GamepadAxis::RightStickX, rs.xAxis().value()),
        (GamepadAxis::RightStickY, -rs.yAxis().value()),
        (GamepadAxis::LeftTrigger, gp.leftTrigger().value()),
        (GamepadAxis::RightTrigger, gp.rightTrigger().value()),
        (GamepadAxis::DPadX, dpad.xAxis().value()),
        (GamepadAxis::DPadY, -dpad.yAxis().value()),
    ]);

    (buttons, axes)
}
