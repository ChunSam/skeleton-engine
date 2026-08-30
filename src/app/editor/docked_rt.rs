/// Fallback margins (logical points) for the frames before the docked UI has published a real
/// panel rect — the first frames of a session, and only those: `ui/docked` writes
/// `EditorState::central_rect` from the real egui panel bounds every docked frame thereafter.
///
/// | Side   | Logical points |
/// |--------|---------------|
/// | Left   | 260           |
/// | Right  | 300           |
/// | Top    | 36            |
/// | Bottom | 160           |
///
/// **Deliberately private.** The subtraction is the policy, and it used to live in two places —
/// `App::compute_viewport` re-derived it by hand and disagreed with the renderer on windows too
/// small to hold a central panel (v0.155.6). Reach it through [`docked_viewport`], which is now
/// the only door: nothing outside this module can spell the formula a second time.
const MARGIN_LEFT: f32 = 260.0;
const MARGIN_RIGHT: f32 = 300.0;
const MARGIN_TOP: f32 = 36.0;
const MARGIN_BOTTOM: f32 = 160.0;

/// Number of consecutive stable frames required before the RT is recreated.
const STABLE_FRAMES: u8 = 3;

/// Compute the central viewport rect (in logical points) from the window's logical size
/// minus the fixed placeholder margins.
///
/// Returns `None` when either dimension would be zero or negative.
fn compute_central_rect(window_logical_w: f32, window_logical_h: f32) -> Option<egui::Rect> {
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
fn rect_to_physical(rect: egui::Rect, scale: f32) -> Option<(u32, u32)> {
    let pw = (rect.width() * scale).round() as u32;
    let ph = (rect.height() * scale).round() as u32;
    if pw == 0 || ph == 0 {
        None
    } else {
        Some((pw, ph))
    }
}

/// The docked scene viewport for this frame — the logical rect **and** its physical pixel
/// size — or `None` when the window leaves no room for one.
///
/// **One decision, two consumers.** `Renderer::prepare_docked_scene_view` sizes the offscreen
/// render target from the physical half and skips the scene render when this is `None`;
/// `App::compute_viewport` publishes the logical half as `ViewportSize` and holds the previous
/// value when this is `None`. Both must reach the same verdict from the same inputs: a
/// `ViewportSize` published for a frame that is never rendered describes a viewport no frame
/// matches. A hand-rolled second copy of the margin subtraction did exactly that until v0.155.6
/// — below 561x197 logical points it published a **1x1** viewport while the renderer drew
/// nothing.
///
/// Sources, in order: the real egui central-panel rect once `ui/docked` has published one
/// (`EditorState::central_rect`), else [`compute_central_rect`]'s placeholder margins for the
/// first frames of a session.
pub fn docked_viewport(
    central_rect: Option<egui::Rect>,
    window_logical_w: f32,
    window_logical_h: f32,
    scale: f32,
) -> Option<(egui::Rect, (u32, u32))> {
    let rect = central_rect.or_else(|| compute_central_rect(window_logical_w, window_logical_h))?;
    let physical = rect_to_physical(rect, scale)?;
    Some((rect, physical))
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

    /// Reset debounce state. Called on every non-docked frame through [`docked_teardown`], which
    /// is what makes "3 stable frames" the rule a *re-entered* docked session obeys too.
    pub fn reset(&mut self) {
        self.candidate = None;
        self.stable_count = 0;
    }
}

/// What a non-Docked frame must do with the docked render-target state.
#[derive(Debug, PartialEq, Eq)]
pub enum DockedTeardown {
    /// An offscreen texture exists: free its egui registration and drop it.
    FreeTexture,
    /// Nothing to free — every non-docked frame after the first, and the case this enum exists
    /// for: a mode exit that happened before the debounce had ever fired.
    Nothing,
}

/// What leaving (or being outside) Docked mode does to the render-target state.
///
/// **The debounce restart is unconditional; the texture teardown is not.** That asymmetry is the
/// entire content of this function, and it is why the function mutates rather than returning a
/// plan: the reset cannot be forgotten by a caller that only handles the texture.
///
/// Until v0.155.7 both hung off `docked_scene_texture.is_some()`, so an exit that happened before
/// the debounce ever fired — no RT had been created yet — carried `candidate` and `stable_count`
/// across the mode change. Re-entering at that same size then recreated the RT after **1–2**
/// frames instead of the 3 [`RtDebounce`] documents, because the stale count was still counting.
pub fn docked_teardown(debounce: &mut RtDebounce, has_texture: bool) -> DockedTeardown {
    debounce.reset();
    if has_texture {
        DockedTeardown::FreeTexture
    } else {
        DockedTeardown::Nothing
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
    fn leaving_docked_before_the_debounce_fires_restarts_the_count() {
        // Enter docked and sit two frames at one size. The rule is three, so no RT was created.
        let mut d = RtDebounce::default();
        assert!(d.tick((800, 600), None).is_none());
        assert!(d.tick((800, 600), None).is_none());

        // Leave docked mode. No texture exists — the exact case that used to skip the reset and
        // carry `stable_count = 2` into the next docked session.
        assert_eq!(docked_teardown(&mut d, false), DockedTeardown::Nothing);

        // Re-enter at the same size: the documented three stable frames, counted from the top.
        assert!(
            d.tick((800, 600), None).is_none(),
            "frame 1 after re-entry — before v0.155.7 the stale count made this fire immediately"
        );
        assert!(d.tick((800, 600), None).is_none(), "frame 2 after re-entry");
        assert_eq!(
            d.tick((800, 600), None),
            Some((800, 600)),
            "frame 3 — control: the debounce still fires at all, so the two Nones above are the \
             rule being obeyed rather than the debounce being broken"
        );
    }

    #[test]
    fn a_teardown_with_a_live_texture_restarts_the_debounce_too() {
        // The half that always reset. It must keep doing so — the fix generalises this branch,
        // it does not move the reset from one branch to the other.
        let mut d = RtDebounce::default();
        assert!(d.tick((800, 600), None).is_none());
        assert!(d.tick((800, 600), None).is_none());
        assert_eq!(docked_teardown(&mut d, true), DockedTeardown::FreeTexture);
        assert!(d.tick((800, 600), None).is_none(), "frame 1 after re-entry");
        assert!(d.tick((800, 600), None).is_none(), "frame 2 after re-entry");
        assert_eq!(d.tick((800, 600), None), Some((800, 600)), "frame 3");
    }

    #[test]
    fn only_the_texture_half_of_the_teardown_is_conditional() {
        // Control against the over-correction: an unconditional *reset* must not drag the texture
        // teardown along with it, which would free a registration that does not exist.
        let mut d = RtDebounce::default();
        assert_eq!(docked_teardown(&mut d, true), DockedTeardown::FreeTexture);
        assert_eq!(docked_teardown(&mut d, false), DockedTeardown::Nothing);
    }

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

    // ── docked_viewport: one decision, two consumers ───────────────────────────

    #[test]
    fn docked_viewport_prefers_the_real_panel_rect() {
        // Once `ui/docked` has published a rect, the fallback margins must not be consulted:
        // a panel narrower than the margins would otherwise disagree with what egui drew.
        let panel = egui::Rect::from_min_size(egui::pos2(300.0, 40.0), egui::vec2(400.0, 250.0));
        let (rect, physical) = docked_viewport(Some(panel), 1280.0, 720.0, 1.0).unwrap();
        assert_eq!(rect, panel);
        assert_eq!(physical, (400, 250));
        // Control: the same window with no published rect yields the margin fallback instead,
        // so the assertion above is about precedence and not about the window being ignored.
        let (fallback, _) = docked_viewport(None, 1280.0, 720.0, 1.0).unwrap();
        assert_eq!(fallback.width(), 1280.0 - MARGIN_LEFT - MARGIN_RIGHT);
        assert_ne!(fallback, panel);
    }

    #[test]
    fn a_window_too_small_for_a_central_panel_has_no_viewport_at_all() {
        // The v0.155.6 defect: `App::compute_viewport` re-derived the margin subtraction by hand
        // with a `.max(1.0)` floor and published a **1x1** `ViewportSize` here, while
        // `prepare_docked_scene_view` skipped the scene render from the same inputs. Every system
        // reading the viewport then ran against a size no frame matched.
        // Horizontal margin is 260 + 300 = 560, vertical 36 + 160 = 196; a rect needs 1 pt of each.
        assert!(
            docked_viewport(None, 560.0, 800.0, 1.0).is_none(),
            "one point too narrow for a central panel"
        );
        assert!(
            docked_viewport(None, 1000.0, 196.0, 1.0).is_none(),
            "one point too short for a central panel"
        );
        // Controls — one point wider / taller and a viewport exists, so the two assertions above
        // are not passing merely because the whole neighbourhood is empty.
        assert_eq!(
            docked_viewport(None, 561.0, 800.0, 1.0).map(|(_, p)| p),
            Some((1, 604))
        );
        assert_eq!(
            docked_viewport(None, 1000.0, 197.0, 1.0).map(|(_, p)| p),
            Some((440, 1))
        );
    }

    #[test]
    fn a_panel_rect_that_rounds_to_zero_physical_pixels_has_no_viewport_either() {
        // A squeezed window can leave egui itself handing back a sub-pixel central panel. The
        // renderer has always refused it (no RT can be one-zero-th of a pixel wide); the
        // published viewport used to take it verbatim, because it branched on the rect alone.
        let sliver = egui::Rect::from_min_size(egui::pos2(260.0, 36.0), egui::vec2(0.4, 100.0));
        assert!(docked_viewport(Some(sliver), 1280.0, 720.0, 1.0).is_none());
        // Control: the same sliver on a 2x display rounds up to one real pixel and is renderable,
        // so the refusal above is about the physical size and not about the rect's shape.
        assert_eq!(
            docked_viewport(Some(sliver), 1280.0, 720.0, 2.0).map(|(_, p)| p),
            Some((1, 200))
        );
    }

    #[test]
    fn the_published_size_and_the_render_target_size_are_one_decision() {
        // The invariant that makes the two consumers agree: whenever a viewport exists, the
        // logical rect `App::compute_viewport` publishes is exactly the one
        // `prepare_docked_scene_view` sizes its offscreen target from — and when none exists,
        // neither consumer gets a size to use.
        let mut renderable = 0;
        let mut skipped = 0;
        for &(w, h) in &[
            (200.0, 100.0),   // both axes too small
            (560.0, 800.0),   // too narrow by one point
            (561.0, 800.0),   // the narrowest renderable window
            (1000.0, 196.0),  // too short by one point
            (1000.0, 197.0),  // the shortest renderable window
            (1280.0, 720.0),  // ordinary
            (3840.0, 2160.0), // large
        ] {
            for &scale in &[1.0, 1.5, 2.0] {
                match docked_viewport(None, w, h, scale) {
                    Some((rect, physical)) => {
                        renderable += 1;
                        assert_eq!(
                            rect_to_physical(rect, scale),
                            Some(physical),
                            "{w}x{h} @{scale}: the published rect and the RT size disagree"
                        );
                    }
                    None => skipped += 1,
                }
            }
        }
        // Control against a vacuous sweep: both arms must actually be reached.
        assert!(
            renderable > 0 && skipped > 0,
            "sweep exercised only one arm ({renderable} renderable, {skipped} skipped)"
        );
    }
}
