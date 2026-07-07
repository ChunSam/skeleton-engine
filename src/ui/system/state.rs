use glam::Vec2;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::ecs::{Entity, Events, World};
use crate::input::{GamepadAxis, GamepadButton, GamepadState, InputState};
use crate::renderer::{DrawRect, DrawText, TextQueue, UiQueue};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;

use super::UiEvent;
use crate::ui::focus::StickNavConfig;

/// Per-axis edge detector that turns a continuous analog stick into discrete D-pad-style steps.
///
/// Held in [`UiSystem`](crate::ui::UiSystem) across frames (alongside the scratch buffers) and fed
/// the left stick each frame by [`InputSnapshot::from_world`]. Unlike a digital button — whose
/// edge is detected for free by [`GamepadState::just_pressed`] — an analog axis needs its own
/// previous-zone memory so pushing the stick fires exactly **once**, not every frame it is held.
#[derive(Default)]
pub(super) struct StickNav {
    /// Last latched zone for the left stick X axis (`-1` left, `0` neutral, `1` right).
    x: i8,
    /// Last latched zone for the left stick Y axis (`-1` down, `0` neutral, `1` up).
    y: i8,
}

impl StickNav {
    /// Updates the latched zones from this frame's left-stick `(x, y)` and returns the discrete step
    /// fired **this frame** for each axis (`-1`, `0`, or `1`). A non-zero result means the stick just
    /// crossed into that direction; holding it past the threshold yields `0` until it returns to
    /// neutral. Must be called once per frame for the edge detection to stay correct.
    ///
    /// `activate`/`release` are the hysteresis thresholds (see [`StickNavConfig`]); callers pass the
    /// resolved (clamped) pair so `release <= activate` always holds.
    pub(super) fn update(&mut self, x: f32, y: f32, activate: f32, release: f32) -> (i8, i8) {
        (
            step_axis(&mut self.x, x, activate, release),
            step_axis(&mut self.y, y, activate, release),
        )
    }
}

/// Advances one axis's latched zone and reports whether it fired a step this frame. `value` past
/// `±activate` latches that direction; within `±release` it resets to neutral; the band in between
/// holds the previous zone (hysteresis). A step fires only on a transition *into* a non-neutral
/// zone, so a held stick fires once.
fn step_axis(latched: &mut i8, value: f32, activate: f32, release: f32) -> i8 {
    let zone = if value >= activate {
        1
    } else if value <= -activate {
        -1
    } else if value.abs() <= release {
        0
    } else {
        *latched
    };
    let fired = if zone != 0 && zone != *latched {
        zone
    } else {
        0
    };
    *latched = zone;
    fired
}

pub(super) struct InputSnapshot {
    pub(super) cursor: Vec2,
    pub(super) just_pressed: bool,
    pub(super) just_released: bool,
    pub(super) is_held: bool,
    pub(super) scroll_delta: f32,
    pub(super) chars: Vec<char>,
    pub(super) ime_preedit: String,
    pub(super) press_cursor: Vec2,
    pub(super) release_cursor: Vec2,
    pub(super) nav_left: bool,
    pub(super) nav_right: bool,
    /// Up/Down arrows — keyboard only (a gamepad's D-pad/stick Up/Down cycle focus, so a pad steps a
    /// list selection with Left/Right). Currently consumed only by the `ListBox` focus arm.
    pub(super) nav_up: bool,
    pub(super) nav_down: bool,
    pub(super) nav_home: bool,
    pub(super) nav_end: bool,
    pub(super) nav_delete: bool,
    /// Tab pressed this frame (focus advance; combine with `shift` for reverse).
    pub(super) tab: bool,
    /// Either Shift key held this frame.
    pub(super) shift: bool,
    /// Enter or Space pressed this frame (activate the focused widget).
    pub(super) activate: bool,
}

impl InputSnapshot {
    pub(super) fn from_world(world: &World, stick: &mut StickNav) -> Option<Self> {
        let input = world.resource::<InputState>()?;
        let cursor = input.cursor();

        // Hit-test clicks against the cursor at the press/release moment, not the
        // live cursor. Hover/drag still use `cursor`.
        let mut snap = Self {
            cursor,
            just_pressed: input.mouse_just_pressed(MouseButton::Left),
            just_released: input.mouse_just_released(MouseButton::Left),
            is_held: input.is_mouse_pressed(MouseButton::Left),
            scroll_delta: input.scroll(),
            chars: input.text_chars().to_vec(),
            ime_preedit: input.ime_preedit().to_string(),
            press_cursor: input.mouse_press_cursor(MouseButton::Left),
            release_cursor: input.mouse_release_cursor(MouseButton::Left),
            nav_left: input.just_pressed(KeyCode::ArrowLeft),
            nav_right: input.just_pressed(KeyCode::ArrowRight),
            nav_up: input.just_pressed(KeyCode::ArrowUp),
            nav_down: input.just_pressed(KeyCode::ArrowDown),
            nav_home: input.just_pressed(KeyCode::Home),
            nav_end: input.just_pressed(KeyCode::End),
            nav_delete: input.just_pressed(KeyCode::Delete),
            tab: input.just_pressed(KeyCode::Tab),
            shift: input.is_pressed(KeyCode::ShiftLeft) || input.is_pressed(KeyCode::ShiftRight),
            activate: input.just_pressed(KeyCode::Enter) || input.just_pressed(KeyCode::Space),
        };

        // Fold in gamepad focus navigation from the first connected pad, mirroring the keyboard:
        // D-pad **or left analog stick** Down/Up cycle focus (Up = reverse, like Shift+Tab),
        // Left/Right nudge a focused slider, A (South) activates. The analog stick is edge-detected
        // via `stick` (one step per push, no auto-repeat). Axis signs follow the engine's existing
        // convention (see the `survivor` example's `AxisBinding`s): up = -Y, down = +Y, right = +X.
        // Optional resource — no pad / no GamepadState = no-op.
        if let Some(gp) = world.resource::<GamepadState>() {
            if let Some(p) = gp.primary() {
                let (activate, release) = world
                    .resource::<StickNavConfig>()
                    .copied()
                    .unwrap_or_default()
                    .resolved();
                let (sx, sy) = stick.update(
                    gp.axis(p, GamepadAxis::LeftStickX),
                    gp.axis(p, GamepadAxis::LeftStickY),
                    activate,
                    release,
                );
                let up = gp.just_pressed(p, GamepadButton::DPadUp) || sy < 0;
                let down = gp.just_pressed(p, GamepadButton::DPadDown) || sy > 0;
                if down || up {
                    snap.tab = true;
                    snap.shift |= up;
                }
                snap.nav_left |= gp.just_pressed(p, GamepadButton::DPadLeft) || sx < 0;
                snap.nav_right |= gp.just_pressed(p, GamepadButton::DPadRight) || sx > 0;
                snap.activate |= gp.just_pressed(p, GamepadButton::South);
            }
        }

        Some(snap)
    }
}

#[derive(Default)]
pub(super) struct UiOutput {
    pub(super) rects: Vec<DrawRect>,
    pub(super) texts: Vec<DrawText>,
    pub(super) events: Vec<UiEvent>,
}

pub(super) fn viewport_from_world(world: &World) -> Option<ViewportSize> {
    world.resource::<ViewportSize>().copied()
}

pub(super) fn submit_output(world: &mut World, output: UiOutput) {
    if let Some(ui_queue) = world.resource_mut::<UiQueue>() {
        for rect in output.rects {
            ui_queue.push(rect);
        }
    }
    if let Some(text_queue) = world.resource_mut::<TextQueue>() {
        for text in output.texts {
            text_queue.push(text);
        }
    }

    if !output.events.is_empty() {
        if let Some(events) = world.resource_mut::<Events<UiEvent>>() {
            for ev in output.events {
                events.send(ev);
            }
        }
    }
}

pub(super) fn in_bounds(cursor: Vec2, pos: Vec2, size: Vec2) -> bool {
    cursor.x >= pos.x
        && cursor.x <= pos.x + size.x
        && cursor.y >= pos.y
        && cursor.y <= pos.y + size.y
}

/// Reads the layout fields of the [`UiNode`] attached to `entity`.
///
/// Returns `Some((screen_pos, size, z, visible))` if the node exists,
/// or `None` if the entity has no `UiNode` component.
pub(super) fn node_layout(
    world: &World,
    entity: Entity,
    viewport: &ViewportSize,
) -> Option<(Vec2, Vec2, f32, bool)> {
    let node = world.get::<UiNode>(entity)?;
    Some((node.screen_pos(viewport), node.size, node.z, node.visible))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::focus::{DEFAULT_STICK_ACTIVATE, DEFAULT_STICK_RELEASE};

    /// Drives one frame with the default (historical) thresholds — keeps the legacy assertions
    /// readable now that `update` takes the hysteresis thresholds explicitly.
    fn upd(s: &mut StickNav, x: f32, y: f32) -> (i8, i8) {
        s.update(x, y, DEFAULT_STICK_ACTIVATE, DEFAULT_STICK_RELEASE)
    }

    /// A push past the activation threshold fires once; holding it (even relaxed into the
    /// hysteresis band) does not repeat; only after returning to neutral does it fire again.
    #[test]
    fn stick_fires_once_per_push_then_requires_neutral() {
        let mut s = StickNav::default();
        assert_eq!(upd(&mut s, 0.0, -0.8), (0, -1), "first push fires");
        assert_eq!(
            upd(&mut s, 0.0, -0.8),
            (0, 0),
            "held past threshold does not repeat"
        );
        assert_eq!(
            upd(&mut s, 0.0, -0.5),
            (0, 0),
            "relaxed into the hysteresis band (RELEASE..ACTIVATE) still does not fire"
        );
        assert_eq!(
            upd(&mut s, 0.0, 0.0),
            (0, 0),
            "returning to neutral does not fire, just resets"
        );
        assert_eq!(
            upd(&mut s, 0.0, -0.8),
            (0, -1),
            "pushing again after neutral fires"
        );
    }

    /// Slamming the stick straight to the opposite direction (without passing through neutral)
    /// fires the new direction.
    #[test]
    fn stick_slam_to_opposite_direction_fires() {
        let mut s = StickNav::default();
        assert_eq!(upd(&mut s, 0.9, 0.0), (1, 0), "push right fires +1");
        assert_eq!(
            upd(&mut s, -0.9, 0.0),
            (-1, 0),
            "slam left fires -1 without a neutral frame"
        );
    }

    /// Small resting drift below the release threshold must never latch or fire.
    #[test]
    fn stick_below_release_never_fires() {
        let mut s = StickNav::default();
        assert_eq!(upd(&mut s, 0.3, 0.2), (0, 0));
        assert_eq!(upd(&mut s, -0.3, -0.2), (0, 0));
    }

    /// A latched zone carries the sign of the stick value (positive → `1`, negative → `-1`); the
    /// semantic up/down/left/right mapping is applied by the caller, not here.
    #[test]
    fn stick_zone_carries_value_sign() {
        let mut s = StickNav::default();
        assert_eq!(upd(&mut s, 0.8, 0.8), (1, 1));
        let mut s2 = StickNav::default();
        assert_eq!(upd(&mut s2, -0.8, -0.8), (-1, -1));
    }

    /// A tighter custom deadzone fires at a push the default thresholds would still treat as the
    /// neutral/hold band — the knob actually changes where the step triggers.
    #[test]
    fn custom_tight_deadzone_fires_earlier() {
        // Default (0.6 / 0.35): a 0.45 push sits in the hold band → no step.
        let mut def = StickNav::default();
        assert_eq!(
            def.update(0.0, -0.45, DEFAULT_STICK_ACTIVATE, DEFAULT_STICK_RELEASE),
            (0, 0),
            "0.45 is below the default activate (0.6) → no step"
        );
        // Tighter (0.4 / 0.2): the same 0.45 push is past activate → fires.
        let mut tight = StickNav::default();
        assert_eq!(
            tight.update(0.0, -0.45, 0.4, 0.2),
            (0, -1),
            "0.45 is past the tight activate (0.4) → fires"
        );
        // ...and re-arming now requires falling back inside the tighter 0.2 release band: a relax to
        // 0.3 stays latched (still in the 0.2..0.4 hold band).
        assert_eq!(
            tight.update(0.0, -0.3, 0.4, 0.2),
            (0, 0),
            "held in hold band"
        );
        assert_eq!(
            tight.update(0.0, -0.15, 0.4, 0.2),
            (0, 0),
            "reset to neutral"
        );
        assert_eq!(
            tight.update(0.0, -0.45, 0.4, 0.2),
            (0, -1),
            "fires again after neutral"
        );
    }

    /// `resolved()` clamps so a misconfigured `release >= activate` can never invert the band.
    #[test]
    fn config_resolved_clamps_invalid_thresholds() {
        let inverted = StickNavConfig {
            activate: 0.5,
            release: 0.9,
        };
        assert_eq!(
            inverted.resolved(),
            (0.5, 0.5),
            "release is clamped down to activate"
        );
        let over = StickNavConfig {
            activate: 2.0,
            release: -1.0,
        };
        assert_eq!(over.resolved(), (1.0, 0.0), "both clamped into range");
        assert_eq!(
            StickNavConfig::default().resolved(),
            (DEFAULT_STICK_ACTIVATE, DEFAULT_STICK_RELEASE),
            "default resolves to the historical thresholds"
        );
    }
}
