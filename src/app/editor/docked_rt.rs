/// Placeholder margins (logical points) used to compute the central rect
/// until package 2 replaces them with real egui panel sizes.
///
/// | Side   | Logical points |
/// |--------|---------------|
/// | Left   | 260           |
/// | Right  | 300           |
/// | Top    | 36            |
/// | Bottom | 160           |
pub const MARGIN_LEFT: f32 = 260.0;
pub const MARGIN_RIGHT: f32 = 300.0;
pub const MARGIN_TOP: f32 = 36.0;
pub const MARGIN_BOTTOM: f32 = 160.0;

/// Number of consecutive stable frames required before the RT is recreated.
const STABLE_FRAMES: u8 = 3;

/// Compute the central viewport rect (in logical points) from the window's logical size
/// minus the fixed placeholder margins.
///
/// Returns `None` when either dimension would be zero or negative.
pub fn compute_central_rect(window_logical_w: f32, window_logical_h: f32) -> Option<egui::Rect> {
    let x = MARGIN_LEFT;
    let y = MARGIN_TOP;
    let w = window_logical_w - MARGIN_LEFT - MARGIN_RIGHT;
    let h = window_logical_h - MARGIN_TOP - MARGIN_BOTTOM;
    if w < 1.0 || h < 1.0 {
        return None;
    }
    Some(egui::Rect::from_min_size(
        egui::pos2(x, y),
        egui::vec2(w, h),
    ))
}

/// Convert a logical-point rect to its physical-pixel size using the display
/// scale factor.  Returns `None` when either dimension rounds to zero.
pub fn rect_to_physical(rect: egui::Rect, scale: f32) -> Option<(u32, u32)> {
    let pw = (rect.width() * scale).round() as u32;
    let ph = (rect.height() * scale).round() as u32;
    if pw == 0 || ph == 0 {
        None
    } else {
        Some((pw, ph))
    }
}

/// Translate a window-space logical mouse position into game-viewport coordinates.
///
/// Subtracts `central_rect.min` from the window position.  Returns `None` when
/// the position lies outside the rect — callers should suppress mouse input for
/// the game when this returns `None`.
///
/// # Arguments
/// - `window_pos` — cursor in logical pixels from the top-left of the OS window.
/// - `central_rect` — the egui central panel rect in logical points.
///
/// # Example
/// ```rust,ignore
/// if let Some(game_pos) = viewport_to_game(cursor_logical, central_rect) {
///     input.set_cursor(game_pos);
/// }
/// ```
pub fn viewport_to_game(window_pos: egui::Pos2, central_rect: egui::Rect) -> Option<egui::Pos2> {
    if central_rect.contains(window_pos) {
        let delta = window_pos - central_rect.min;
        Some(egui::pos2(delta.x, delta.y))
    } else {
        None
    }
}

/// Decide whether a pointer event (button / wheel / gizmo drag) may reach the
/// game while docked.
///
/// `Context::egui_wants_pointer_input()` cannot be used here: the docked game
/// viewport lives inside an egui `CentralPanel`, so egui reports the pointer as
/// "over egui" across the entire viewport, which would swallow every click.
/// Instead the rule is:
///
/// 1. the pointer must be physically inside `central_rect` (window space), and
/// 2. egui must not be actively using the pointer (dragging a slider, a panel
///    resize handle, …), and
/// 3. the topmost egui layer under the pointer must be the `Background` order —
///    panels live on `Background`, while menus / popups / tooltips / floating
///    windows that overlap the viewport are higher orders and keep the click.
pub fn docked_game_pointer_allowed(
    window_cursor: Option<egui::Pos2>,
    central_rect: Option<egui::Rect>,
    ctx: Option<&egui::Context>,
) -> bool {
    let (Some(pos), Some(rect)) = (window_cursor, central_rect) else {
        return false;
    };
    if !rect.contains(pos) {
        return false;
    }
    let Some(ctx) = ctx else {
        return true;
    };
    if ctx.egui_is_using_pointer() {
        return false;
    }
    ctx.layer_id_at(pos)
        .is_none_or(|layer| layer.order == egui::Order::Background)
}

/// Tracks the "stable for 3 frames" debounce rule for the docked RT.
///
/// The RT is only recreated when the target physical size has been **identical**
/// for `STABLE_FRAMES` consecutive frames AND differs from the current RT size.
#[derive(Debug, Default)]
pub struct RtDebounce {
    /// The target size seen during the current stable run.
    candidate: Option<(u32, u32)>,
    /// How many consecutive frames the candidate has been stable.
    stable_count: u8,
}

impl RtDebounce {
    /// Feed the target physical size for this frame.
    ///
    /// Returns `Some((w, h))` when the RT should be recreated to that size;
    /// returns `None` when the target is still changing or is already current.
    ///
    /// `current_size` is `Some((w, h))` when an RT already exists, `None` on
    /// the first docked frame.
    pub fn tick(
        &mut self,
        target: (u32, u32),
        current_size: Option<(u32, u32)>,
    ) -> Option<(u32, u32)> {
        // No-op when the RT already matches the target.
        if current_size == Some(target) {
            self.candidate = None;
            self.stable_count = 0;
            return None;
        }

        if self.candidate == Some(target) {
            self.stable_count = self.stable_count.saturating_add(1);
        } else {
            self.candidate = Some(target);
            self.stable_count = 1;
        }

        if self.stable_count >= STABLE_FRAMES {
            self.candidate = None;
            self.stable_count = 0;
            Some(target)
        } else {
            None
        }
    }

    /// Reset debounce state (called when exiting docked mode).
    pub fn reset(&mut self) {
        self.candidate = None;
        self.stable_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── debounce logic ────────────────────────────────────────────────────────

    #[test]
    fn fires_after_three_stable_frames() {
        let mut d = RtDebounce::default();
        let target = (800, 600);
        assert!(d.tick(target, None).is_none(), "frame 1: not yet stable");
        assert!(d.tick(target, None).is_none(), "frame 2: not yet stable");
        let result = d.tick(target, None);
        assert_eq!(result, Some(target), "frame 3: should fire");
    }

    #[test]
    fn size_change_resets_counter() {
        let mut d = RtDebounce::default();
        assert!(d.tick((800, 600), None).is_none());
        assert!(d.tick((800, 600), None).is_none());
        // size changes → reset
        assert!(d.tick((900, 600), None).is_none(), "reset on size change");
        assert!(d.tick((900, 600), None).is_none());
        let result = d.tick((900, 600), None);
        assert_eq!(result, Some((900, 600)));
    }

    #[test]
    fn no_fire_when_already_current_size() {
        let mut d = RtDebounce::default();
        // RT already exists at target size — should never fire.
        for _ in 0..10 {
            assert!(d.tick((800, 600), Some((800, 600))).is_none());
        }
    }

    #[test]
    fn fires_on_first_creation_after_three_frames() {
        // current_size = None (no RT yet)
        let mut d = RtDebounce::default();
        assert!(d.tick((640, 480), None).is_none());
        assert!(d.tick((640, 480), None).is_none());
        assert_eq!(d.tick((640, 480), None), Some((640, 480)));
    }

    // ── EditorMode transitions ─────────────────────────────────────────────

    #[test]
    fn f1_transitions() {
        use crate::app::editor::state::{apply_f1, EditorMode};
        assert_eq!(apply_f1(EditorMode::Off), EditorMode::Overlay);
        assert_eq!(apply_f1(EditorMode::Overlay), EditorMode::Off);
        assert_eq!(apply_f1(EditorMode::Docked), EditorMode::Overlay);
    }

    #[test]
    fn f2_transitions() {
        use crate::app::editor::state::{apply_f2, EditorMode};
        assert_eq!(apply_f2(EditorMode::Off), EditorMode::Docked);
        assert_eq!(apply_f2(EditorMode::Overlay), EditorMode::Docked);
        assert_eq!(apply_f2(EditorMode::Docked), EditorMode::Off);
    }

    // ── physical size calc ────────────────────────────────────────────────

    #[test]
    fn rect_to_physical_rounds_correctly() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.5, 599.4));
        let (pw, ph) = rect_to_physical(r, 1.0).unwrap();
        assert_eq!(pw, 801);
        assert_eq!(ph, 599);
    }

    #[test]
    fn rect_to_physical_with_scale_factor() {
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 360.0));
        let (pw, ph) = rect_to_physical(r, 2.0).unwrap();
        assert_eq!(pw, 1280);
        assert_eq!(ph, 720);
    }

    #[test]
    fn rect_to_physical_zero_guard() {
        // Margins wider than window → degenerate rect.
        let r = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(0.1, 0.1));
        let result = rect_to_physical(r, 1.0);
        // 0.1 * 1.0 rounds to 0, so None
        assert!(result.is_none());
    }

    #[test]
    fn compute_central_rect_basic() {
        let rect = compute_central_rect(1280.0, 720.0).unwrap();
        assert_eq!(rect.min.x, MARGIN_LEFT);
        assert_eq!(rect.min.y, MARGIN_TOP);
        assert!((rect.width() - (1280.0 - MARGIN_LEFT - MARGIN_RIGHT)).abs() < 0.01);
        assert!((rect.height() - (720.0 - MARGIN_TOP - MARGIN_BOTTOM)).abs() < 0.01);
    }

    #[test]
    fn compute_central_rect_zero_guard() {
        // Window too small for the margins.
        assert!(compute_central_rect(10.0, 10.0).is_none());
    }

    // ── viewport_to_game ─────────────────────────────────────────────────────

    #[test]
    fn viewport_to_game_inside_translates() {
        let rect = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(720.0, 534.0));
        let result = viewport_to_game(egui::pos2(360.0, 136.0), rect);
        assert!(result.is_some());
        let g = result.unwrap();
        assert!((g.x - 100.0).abs() < 0.01);
        assert!((g.y - 100.0).abs() < 0.01);
    }

    #[test]
    fn viewport_to_game_outside_returns_none() {
        let rect = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(720.0, 534.0));
        // Cursor is to the left of the panel
        assert!(viewport_to_game(egui::pos2(10.0, 100.0), rect).is_none());
        // Cursor is above the panel
        assert!(viewport_to_game(egui::pos2(400.0, 10.0), rect).is_none());
    }

    #[test]
    fn viewport_to_game_on_edge_is_inside() {
        let rect = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(720.0, 534.0));
        // egui Rect::contains treats the boundary as included on min, excluded on max
        let result = viewport_to_game(egui::pos2(260.0, 36.0), rect);
        assert!(result.is_some());
    }

    // ── docked_game_pointer_allowed ──────────────────────────────────────────

    #[test]
    fn pointer_allowed_requires_cursor_and_rect() {
        let rect = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(720.0, 534.0));
        assert!(!docked_game_pointer_allowed(None, Some(rect), None));
        assert!(!docked_game_pointer_allowed(
            Some(egui::pos2(300.0, 100.0)),
            None,
            None
        ));
    }

    #[test]
    fn pointer_outside_rect_is_blocked() {
        let rect = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(720.0, 534.0));
        // Over the left side panel (x < 260): the inspector keeps the click.
        assert!(!docked_game_pointer_allowed(
            Some(egui::pos2(100.0, 300.0)),
            Some(rect),
            None
        ));
    }

    #[test]
    fn pointer_inside_rect_with_idle_ctx_is_allowed() {
        let rect = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(720.0, 534.0));
        let pos = egui::pos2(400.0, 200.0);
        assert!(docked_game_pointer_allowed(Some(pos), Some(rect), None));
        // A fresh Context has no layers under the pointer and is not using it.
        let ctx = egui::Context::default();
        assert!(docked_game_pointer_allowed(
            Some(pos),
            Some(rect),
            Some(&ctx)
        ));
    }
}
