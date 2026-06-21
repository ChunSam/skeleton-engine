mod gamepad;
/// macOS GameController backend (gilrs can't read modern pads there). Polled from the app loop.
#[cfg(target_os = "macos")]
pub(crate) mod gamepad_macos;
mod map;
mod state;
mod touch;

pub use gamepad::{GamepadAxis, GamepadButton, GamepadState};
pub use map::{AxisBinding, InputMap};
pub use state::InputState;
pub use touch::TouchState;
