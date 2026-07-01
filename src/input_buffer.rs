//! Input buffering + coyote time — the two game-feel tricks that make a platformer jump feel fair.
//!
//! Two classic forgiveness mechanics, in one small pure-logic helper (drive it yourself like
//! [`Timer`](crate::Timer) / [`Tween`](crate::Tween) — it is **not** a system or component):
//!
//! - **Input buffering** — a jump pressed a few frames *before* landing still fires the instant the
//!   character touches down, instead of being silently dropped because it was "too early".
//! - **Coyote time** — a jump pressed a few frames *after* walking off a ledge still fires, instead
//!   of being dropped because the character was "already falling".
//!
//! Both are just short countdowns: [`press`](InputBuffer::press) opens the buffer window;
//! [`set_grounded`](InputBuffer::set_grounded) refreshes the coyote window while grounded and lets
//! it decay once airborne. [`try_consume`](InputBuffer::try_consume) fires (and consumes) a jump
//! when a live buffered press overlaps the coyote window. Named for the canonical jump case, but the
//! same helper works for any buffered action gated by a briefly-forgiving condition (a dash off a
//! wall, an attack cancel, …).
//!
//! # Per-frame order
//!
//! Feed the ground state, buffer the press, try to consume, then tick **last**:
//!
//! ```
//! # use engine::InputBuffer;
//! # let (grounded, jump_pressed, dt) = (true, true, 1.0 / 60.0);
//! let mut jump = InputBuffer::default();
//! jump.set_grounded(grounded);      // 1. current ground contact (e.g. CharacterController::grounded)
//! if jump_pressed { jump.press(); } // 2. the player hit the jump button this frame
//! if jump.try_consume() {           // 3. a live press inside the coyote window → jump now
//!     // apply the jump impulse
//! }
//! jump.tick(dt);                    // 4. age both windows (call last)
//! ```
//!
//! Ticking last means the windows are evaluated on the frame a press/landing happens, so even a
//! zero-length window (forgiveness disabled) still lets a grounded press jump on the exact frame.

/// Default buffer window in seconds — a jump pressed within this long *before* landing still fires.
pub const DEFAULT_BUFFER_SECS: f32 = 0.12;
/// Default coyote window in seconds — a jump pressed within this long *after* leaving the ground
/// still fires.
pub const DEFAULT_COYOTE_SECS: f32 = 0.10;

/// A buffered-press + coyote-time gate for a forgiving jump (see the [module docs](crate::input_buffer)).
///
/// Build one with [`InputBuffer::new`] (or [`InputBuffer::default`] for the default windows), then
/// each frame call [`set_grounded`](Self::set_grounded), optionally [`press`](Self::press),
/// [`try_consume`](Self::try_consume), and [`tick`](Self::tick) **in that order** (tick last).
///
/// ```
/// # use engine::InputBuffer;
/// let mut jump = InputBuffer::new(0.1, 0.1);
/// // Walk off a ledge (airborne) then press a hair late — coyote time still allows the jump.
/// jump.set_grounded(true);
/// jump.set_grounded(false); // just left the ground; the coyote window is now counting down
/// jump.tick(0.05);          // 0.05s < 0.1s coyote → still eligible
/// jump.press();
/// assert!(jump.try_consume(), "coyote jump fires");
/// assert!(!jump.try_consume(), "a jump is consumed once");
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct InputBuffer {
    buffer_secs: f32,
    coyote_secs: f32,
    /// Time since the last [`press`](Self::press); a press is live while this is `<= buffer_secs`.
    /// Starts at infinity (no press yet); consuming resets it there.
    since_press: f32,
    /// Remaining coyote time while airborne. Held at [`coyote_secs`](Self::coyote_secs) while
    /// grounded; decays once airborne. (Grounded is always eligible regardless of this.)
    coyote: f32,
    /// Last-fed ground state.
    grounded: bool,
}

impl Default for InputBuffer {
    fn default() -> Self {
        Self::new(DEFAULT_BUFFER_SECS, DEFAULT_COYOTE_SECS)
    }
}

impl InputBuffer {
    /// A buffer with a `buffer_secs` press window and a `coyote_secs` ground-grace window. Negative
    /// inputs are clamped to `0` (that forgiveness disabled — a grounded press still jumps on the
    /// exact frame). Starts airborne with no live press.
    pub fn new(buffer_secs: f32, coyote_secs: f32) -> Self {
        Self {
            buffer_secs: buffer_secs.max(0.0),
            coyote_secs: coyote_secs.max(0.0),
            since_press: f32::INFINITY,
            coyote: 0.0,
            grounded: false,
        }
    }

    /// Record that the jump button was pressed this frame — opens the buffer window
    /// ([`buffer_secs`](Self::buffer_secs)). The press stays live (consumable) until the window
    /// elapses or [`try_consume`](Self::try_consume) uses it.
    pub fn press(&mut self) {
        self.since_press = 0.0;
    }

    /// Feed the current ground contact. While grounded the coyote window is held full and the
    /// character is always jump-eligible; the frame it leaves the ground the window begins to decay
    /// in [`tick`](Self::tick). Call this every frame (e.g. with `CharacterController::grounded`),
    /// before [`try_consume`](Self::try_consume).
    pub fn set_grounded(&mut self, grounded: bool) {
        if grounded {
            self.coyote = self.coyote_secs;
        }
        self.grounded = grounded;
    }

    /// If a live buffered press currently overlaps the coyote window (or the character is grounded),
    /// consume it and return `true` (the jump fires this frame). Otherwise return `false`. Consuming
    /// clears the press and the coyote window, so a single press yields exactly one jump — no
    /// double-jump from a held/leftover press while airborne.
    pub fn try_consume(&mut self) -> bool {
        if self.is_buffered() && self.is_coyote_available() {
            self.since_press = f32::INFINITY;
            self.coyote = 0.0;
            true
        } else {
            false
        }
    }

    /// Age both windows by `dt`. The buffer always ages; the coyote window decays only while
    /// airborne (it is kept full by [`set_grounded(true)`](Self::set_grounded)). Call this **last**
    /// each frame, after [`try_consume`](Self::try_consume).
    pub fn tick(&mut self, dt: f32) {
        // Saturate rather than overflow toward infinity across a long idle.
        self.since_press = (self.since_press + dt).min(f32::MAX);
        if !self.grounded {
            self.coyote = (self.coyote - dt).max(0.0);
        }
    }

    /// Remaining buffered-press time in seconds (`0.0` when no live press is waiting).
    pub fn buffered_remaining(&self) -> f32 {
        (self.buffer_secs - self.since_press).max(0.0)
    }

    /// Remaining coyote time in seconds while airborne (`coyote_secs` while grounded).
    pub fn coyote_remaining(&self) -> f32 {
        self.coyote
    }

    /// Whether a live buffered press is waiting (the last [`press`](Self::press) is still within
    /// [`buffer_secs`](Self::buffer_secs) and unconsumed).
    pub fn is_buffered(&self) -> bool {
        self.since_press <= self.buffer_secs
    }

    /// Whether the jump is currently eligible — grounded, or within
    /// [`coyote_secs`](Self::coyote_secs) of having left the ground.
    pub fn is_coyote_available(&self) -> bool {
        self.grounded || self.coyote > 0.0
    }

    /// The last-fed ground state.
    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    /// The configured buffer window in seconds.
    pub fn buffer_secs(&self) -> f32 {
        self.buffer_secs
    }

    /// The configured coyote window in seconds.
    pub fn coyote_secs(&self) -> f32 {
        self.coyote_secs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grounded + pressed → an immediate jump (order: set_grounded, press, try_consume, tick).
    #[test]
    fn grounded_press_jumps_immediately() {
        let mut b = InputBuffer::new(0.1, 0.1);
        b.set_grounded(true);
        b.press();
        assert!(b.try_consume());
        assert!(!b.try_consume(), "only one jump per press");
    }

    /// Press just AFTER leaving a ledge (within coyote) still jumps.
    #[test]
    fn coyote_allows_a_late_press() {
        let mut b = InputBuffer::new(0.1, 0.1);
        b.set_grounded(true);
        b.set_grounded(false); // walked off the ledge (coyote still full from the grounded frame)
        b.tick(0.05); // 0.05 < 0.1 coyote → still eligible
        b.set_grounded(false);
        b.press();
        assert!(b.try_consume(), "coyote jump");
    }

    /// Past the coyote window, a late press is dropped.
    #[test]
    fn coyote_expires() {
        let mut b = InputBuffer::new(0.1, 0.1);
        b.set_grounded(true);
        b.set_grounded(false);
        b.tick(0.2); // 0.2 > 0.1 coyote → expired
        b.set_grounded(false);
        b.press();
        assert!(!b.try_consume(), "too late to coyote-jump");
    }

    /// Press just BEFORE landing (within the buffer) fires the instant the character lands.
    #[test]
    fn buffered_press_fires_on_landing() {
        let mut b = InputBuffer::new(0.12, 0.1);
        // Airborne, coyote already expired.
        b.set_grounded(false);
        b.tick(0.2);
        // Press while still falling — buffered.
        b.set_grounded(false);
        b.press();
        assert!(!b.try_consume(), "cannot jump yet — still airborne");
        b.tick(0.05); // still airborne; since_press 0.05 < 0.12 buffer → still live
                      // Land: grounded → eligible, the buffered press now fires.
        b.set_grounded(true);
        assert!(b.try_consume(), "buffered jump fires on landing");
    }

    /// A press whose buffer window elapses before landing is dropped.
    #[test]
    fn buffer_expires_if_landing_is_too_late() {
        let mut b = InputBuffer::new(0.1, 0.1);
        b.set_grounded(false);
        b.tick(0.2);
        b.set_grounded(false);
        b.press();
        b.tick(0.15); // 0.15 > 0.1 buffer → press expired before landing
        b.set_grounded(true);
        assert!(!b.try_consume(), "buffered press expired");
    }

    /// Grounded holds the coyote window full across ticks (you can always jump while grounded).
    #[test]
    fn grounded_keeps_coyote_full() {
        let mut b = InputBuffer::new(0.1, 0.1);
        b.set_grounded(true);
        b.tick(1.0); // a long frame while grounded must NOT drain coyote eligibility
        b.set_grounded(true);
        assert!((b.coyote_remaining() - 0.1).abs() < 1e-6);
        b.press();
        assert!(b.try_consume());
    }

    /// No mid-air re-jump: after consuming, an airborne re-press cannot fire until landing.
    #[test]
    fn no_double_jump_in_the_air() {
        let mut b = InputBuffer::new(0.2, 0.1);
        b.set_grounded(true);
        b.press();
        assert!(b.try_consume(), "first jump");
        // Now airborne; mash again.
        b.tick(0.016);
        b.set_grounded(false);
        b.press();
        assert!(!b.try_consume(), "coyote was consumed — no air jump");
    }

    /// Zero windows disable forgiveness but a grounded same-frame press still jumps (no footgun).
    #[test]
    fn zero_windows_still_allow_a_grounded_jump() {
        let mut b = InputBuffer::new(0.0, 0.0);
        assert_eq!(b.buffer_secs(), 0.0);
        assert_eq!(b.coyote_secs(), 0.0);
        b.set_grounded(true);
        b.press(); // since_press = 0 == buffer_secs(0) → still live this frame
        assert!(
            b.try_consume(),
            "a grounded same-frame press jumps even with zero windows"
        );
        // But airborne with zero coyote → no forgiveness.
        b.tick(0.016);
        b.set_grounded(false);
        b.press();
        assert!(!b.try_consume(), "no coyote grace with a zero window");
    }

    /// Negative windows are clamped to zero.
    #[test]
    fn negative_windows_clamp_to_zero() {
        let b = InputBuffer::new(-1.0, -2.0);
        assert_eq!(b.buffer_secs(), 0.0);
        assert_eq!(b.coyote_secs(), 0.0);
    }
}
