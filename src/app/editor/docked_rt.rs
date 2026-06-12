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
}
