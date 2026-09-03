//! Pure geometry helpers for the editor transform/resize/rotation gizmos.
//!
//! Extracted verbatim from `gizmo.rs` — these are side-effect-free functions (and the
//! gizmo size/snap constants) shared between the screen-space UI gizmo, the world-space
//! sprite gizmo, and the rotation gizmo in `gizmo.rs`'s `impl App`. Kept separate so the
//! math is unit-testable in isolation (the tests below) and the interaction logic in
//! `gizmo.rs` stays focused on input handling + rendering.
//!
//! Mirrors `gizmo.rs`'s gating: no module-level `cfg`, each item is individually
//! `#[cfg(not(target_arch = "wasm32"))]` (the editor UI is native-only, but `gizmo.rs`
//! itself compiles on wasm with the native bits gated out).

#[cfg(not(target_arch = "wasm32"))]
use crate::app::editor::ResizeHandle;
#[cfg(not(target_arch = "wasm32"))]
use crate::ui::Anchor;

/// The anchor base position — the screen-space point `UiNode::offset` is added to.
///
/// Delegates to [`Anchor::base`], where the formula lives. This was a second copy of that
/// `match` while this very doc claimed the two were "a single authoritative definition"; they
/// agreed, and nothing made them (v0.156.20).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn anchor_base(anchor: Anchor, size: glam::Vec2, vw: f32, vh: f32) -> glam::Vec2 {
    anchor.base(size, glam::Vec2::new(vw, vh))
}

/// Returns the corrected offset after a screen-space drag.
///
/// `start_offset` is the UiNode offset at drag start, `start_cursor` is the
/// cursor position (logical pixels) at drag start, and `cursor` is the current
/// cursor position.  The size and anchor are unchanged, so cursor delta maps
/// 1:1 to an offset delta.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ui_drag_new_offset(
    start_offset: glam::Vec2,
    start_cursor: glam::Vec2,
    cursor: glam::Vec2,
) -> glam::Vec2 {
    start_offset + (cursor - start_cursor)
}

/// Minimum size clamped for both UI nodes and world sprites.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const MIN_UI_SIZE: f32 = 8.0;

/// Minimum scale for world sprites.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const MIN_SPRITE_SCALE: f32 = 2.0;

/// Resize-handle size (square, logical pixels).
#[cfg(not(target_arch = "wasm32"))]
pub(super) const HANDLE_SIZE: f32 = 6.0;

/// Hit-test radius around a handle centre, in **logical pixels**.
///
/// ⚠️ The UI gizmo works in logical pixels and can use this directly; the Transform gizmo works
/// in world units and must divide by the camera zoom. It did not until v0.156.22, so eight
/// WORLD units of handle covered every interior point of anything 16 units or smaller — a
/// 16×16 tile had no move region at any zoom, because zooming in did not shrink the radius.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const HANDLE_HIT_RADIUS: f32 = 8.0;

/// Gap (world units) between the entity's top edge and the rotation handle.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const ROT_HANDLE_GAP: f32 = 16.0;

/// Hit-test radius around the rotation handle, in **logical pixels** — the Transform gizmo
/// divides it by the camera zoom, like [`HANDLE_HIT_RADIUS`]. It was world units until
/// v0.156.22, so the rotation handle's grab area grew and shrank with the zoom.
#[cfg(not(target_arch = "wasm32"))]
pub(super) const ROT_HIT_RADIUS: f32 = 8.0;

/// Rotation snap step when the editor Snap toggle is on (15°).
#[cfg(not(target_arch = "wasm32"))]
pub(super) const ROT_SNAP: f32 = std::f32::consts::PI / 12.0;

/// World position of the rotation handle: centred above the entity's top edge
/// (the engine's Y axis points down, so "above" is a smaller `y`).
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn rotation_handle_pos(position: glam::Vec2, scale: glam::Vec2, gap: f32) -> glam::Vec2 {
    glam::Vec2::new(position.x, position.y - scale.y.abs() * 0.5 - gap)
}

/// Angle (radians) of `cursor` around `center` — `atan2(dy, dx)`.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn cursor_angle(center: glam::Vec2, cursor: glam::Vec2) -> f32 {
    let d = cursor - center;
    d.y.atan2(d.x)
}

/// Round `angle` to the nearest multiple of `step` (identity if `step <= 0`).
#[cfg(not(target_arch = "wasm32"))]
fn snap_angle(angle: f32, step: f32) -> f32 {
    if step <= 0.0 {
        return angle;
    }
    (angle / step).round() * step
}

/// New rotation while dragging the rotation handle: the start rotation plus the cursor-angle
/// delta, optionally snapped to `snap` radians.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn applied_rotation(
    start_rot: f32,
    start_angle: f32,
    cur_angle: f32,
    snap: Option<f32>,
) -> f32 {
    let r = start_rot + (cur_angle - start_angle);
    match snap {
        Some(step) => snap_angle(r, step),
        None => r,
    }
}

/// Returns the 8 handle centre positions (in the same space as `pos`) in
/// `ResizeHandle` declaration order: TL, T, TR, L, R, BL, B, BR.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn handle_centers(pos: glam::Vec2, size: glam::Vec2) -> [glam::Vec2; 8] {
    let cx = pos.x + size.x * 0.5;
    let cy = pos.y + size.y * 0.5;
    let r = pos.x + size.x;
    let b = pos.y + size.y;
    [
        glam::Vec2::new(pos.x, pos.y), // TopLeft
        glam::Vec2::new(cx, pos.y),    // Top
        glam::Vec2::new(r, pos.y),     // TopRight
        glam::Vec2::new(pos.x, cy),    // Left
        glam::Vec2::new(r, cy),        // Right
        glam::Vec2::new(pos.x, b),     // BottomLeft
        glam::Vec2::new(cx, b),        // Bottom
        glam::Vec2::new(r, b),         // BottomRight
    ]
}

/// Hit-test the 8 resize handles; returns the first one whose centre is within `radius` of
/// `cursor`, or `None`. Everything is in one space — logical pixels for the UI gizmo, world
/// units for the Transform one, which passes [`HANDLE_HIT_RADIUS`] divided by the camera zoom
/// so the grab area is the same size on screen at any zoom (v0.156.22).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn hit_test_handles(
    pos: glam::Vec2,
    size: glam::Vec2,
    cursor: glam::Vec2,
    radius: f32,
) -> Option<ResizeHandle> {
    use ResizeHandle::*;
    let centres = handle_centers(pos, size);
    let variants = [
        TopLeft,
        Top,
        TopRight,
        Left,
        Right,
        BottomLeft,
        Bottom,
        BottomRight,
    ];
    for (c, v) in centres.iter().zip(variants.iter()) {
        if (cursor - *c).length() <= radius {
            return Some(*v);
        }
    }
    None
}

/// Compute the new `(offset, size)` after dragging `handle` by `delta` pixels,
/// preserving the screen position of the corner that should stay fixed for ANY
/// `anchor` type.
///
/// For `Anchor::TopLeft` the anchor base is constant (zero), so the compensation
/// term is zero and behaviour is identical to the previous implementation.
/// For non-TopLeft anchors `base(size)` depends on the current size, so resizing
/// would shift the widget without the correction below.
///
/// The fix: after computing `(offset, new_size)` with TopLeft-style math,
/// add the difference `base(start_size) − base(new_size)` to offset so the
/// fixed corner's screen position (`base + offset`) remains unchanged.
///
/// Corners adjust both offset and size; edges adjust one axis.
/// Size is clamped to `MIN_UI_SIZE × MIN_UI_SIZE`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn ui_resize_new_layout(
    start_offset: glam::Vec2,
    start_size: glam::Vec2,
    delta: glam::Vec2,
    handle: ResizeHandle,
    anchor: Anchor,
    viewport_wh: (f32, f32),
) -> (glam::Vec2, glam::Vec2) {
    use ResizeHandle::*;
    let mut offset = start_offset;
    let mut size = start_size;
    match handle {
        TopLeft => {
            offset += delta;
            size -= delta;
        }
        Top => {
            offset.y += delta.y;
            size.y -= delta.y;
        }
        TopRight => {
            offset.y += delta.y;
            size.x += delta.x;
            size.y -= delta.y;
        }
        Left => {
            offset.x += delta.x;
            size.x -= delta.x;
        }
        Right => {
            size.x += delta.x;
        }
        BottomLeft => {
            offset.x += delta.x;
            size.x -= delta.x;
            size.y += delta.y;
        }
        Bottom => {
            size.y += delta.y;
        }
        BottomRight => {
            size += delta;
        }
    }
    // Clamp size.  When a top/left edge would push size below min, lock it.
    if size.x < MIN_UI_SIZE {
        let excess = MIN_UI_SIZE - size.x;
        size.x = MIN_UI_SIZE;
        match handle {
            TopLeft | Left | BottomLeft => {
                offset.x -= excess;
            }
            _ => {}
        }
    }
    if size.y < MIN_UI_SIZE {
        let excess = MIN_UI_SIZE - size.y;
        size.y = MIN_UI_SIZE;
        match handle {
            TopLeft | Top | TopRight => {
                offset.y -= excess;
            }
            _ => {}
        }
    }
    // Anchor-base compensation: for non-TopLeft anchors the base position
    // depends on `size`, so a size change shifts `screen_pos = base + offset`
    // unless we compensate by adjusting `offset` by the change in base.
    // For TopLeft: base == Vec2::ZERO always → compensation is zero (no change).
    let (vw, vh) = viewport_wh;
    let base_delta = anchor_base(anchor, start_size, vw, vh) - anchor_base(anchor, size, vw, vh);
    offset += base_delta;
    (offset, size)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── handle_centers / hit_test_handles ───────────────────────────────────

    /// A 16-unit entity's every interior point sits within eight units of some handle centre, and
    /// `hit_test_handles` returns the first match — so at zoom 1 there is no move region at all,
    /// and before v0.156.22 zooming in did not help because the radius was world units. Passing
    /// the radius in the caller's space fixes both: at zoom 4 the same press finds the body.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn a_small_entity_gets_a_move_region_when_zoomed_in() {
        // A 16×16 entity centred on the origin: top-left (-8, -8).
        let (pos, size) = (glam::Vec2::splat(-8.0), glam::Vec2::splat(16.0));
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::ZERO, HANDLE_HIT_RADIUS),
            Some(ResizeHandle::Top),
            "precondition: at eight units of radius the centre of a 16×16 entity is a handle"
        );
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::ZERO, HANDLE_HIT_RADIUS / 4.0),
            None,
            "at zoom 4 the radius is two world units and the centre is free"
        );
        // Control: the corner is still a handle at that zoom, so the radius shrank rather than
        // the hit test breaking.
        assert_eq!(
            hit_test_handles(pos, size, pos, HANDLE_HIT_RADIUS / 4.0),
            Some(ResizeHandle::TopLeft),
            "control: the corner still grabs"
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_handle_hit_corners() {
        // Box at (50, 50) with size (200, 100).
        let pos = glam::Vec2::new(50.0, 50.0);
        let size = glam::Vec2::new(200.0, 100.0);

        // Top-left corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(50.0, 50.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::TopLeft)
        );
        // Top-right corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(250.0, 50.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::TopRight)
        );
        // Bottom-left corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(50.0, 150.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::BottomLeft)
        );
        // Bottom-right corner.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(250.0, 150.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::BottomRight)
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_handle_hit_middle_returns_none() {
        let pos = glam::Vec2::new(50.0, 50.0);
        let size = glam::Vec2::new(200.0, 100.0);

        // Centre of the box — far from any handle.
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(150.0, 100.0), HANDLE_HIT_RADIUS),
            None
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_handle_hit_edge_midpoints() {
        let pos = glam::Vec2::new(50.0, 50.0);
        let size = glam::Vec2::new(200.0, 100.0);

        // Top edge midpoint: (150, 50).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(150.0, 50.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::Top)
        );
        // Right edge midpoint: (250, 100).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(250.0, 100.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::Right)
        );
        // Bottom edge midpoint: (150, 150).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(150.0, 150.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::Bottom)
        );
        // Left edge midpoint: (50, 100).
        assert_eq!(
            hit_test_handles(pos, size, glam::Vec2::new(50.0, 100.0), HANDLE_HIT_RADIUS),
            Some(ResizeHandle::Left)
        );
    }

    // ─── ui_drag_new_offset ───────────────────────────────────────────────────

    #[test]
    fn move_offset_invariant_simple() {
        let start_offset = glam::Vec2::new(100.0, 200.0);
        let start_cursor = glam::Vec2::new(150.0, 220.0);
        let cursor = glam::Vec2::new(170.0, 230.0); // moved by (20, 10)
        let result = ui_drag_new_offset(start_offset, start_cursor, cursor);
        let expected = glam::Vec2::new(120.0, 210.0);
        assert!(
            (result - expected).length() < 0.001,
            "got {result:?} expected {expected:?}"
        );
    }

    #[test]
    fn move_offset_invariant_zero_delta() {
        let start_offset = glam::Vec2::new(50.0, 80.0);
        let start_cursor = glam::Vec2::new(100.0, 100.0);
        let result = ui_drag_new_offset(start_offset, start_cursor, start_cursor);
        assert!((result - start_offset).length() < 0.001);
    }

    // ─── ui_resize_new_layout ────────────────────────────────────────────────
    // TopLeft anchor: base is always zero → compensation is zero → same assertions as before.

    #[test]
    fn resize_topleft_basic() {
        let start_offset = glam::Vec2::new(50.0, 50.0);
        let start_size = glam::Vec2::new(200.0, 100.0);
        let delta = glam::Vec2::new(20.0, 10.0); // drag TopLeft down-right → shrinks
        let (new_offset, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::TopLeft,
            Anchor::TopLeft,
            (800.0, 600.0),
        );
        assert!((new_offset.x - 70.0).abs() < 0.001, "offset.x");
        assert!((new_offset.y - 60.0).abs() < 0.001, "offset.y");
        assert!((new_size.x - 180.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 90.0).abs() < 0.001, "size.y");
    }

    #[test]
    fn resize_topleft_min_clamp() {
        let start_offset = glam::Vec2::new(50.0, 50.0);
        let start_size = glam::Vec2::new(20.0, 20.0);
        // Delta so large it would go negative.
        let delta = glam::Vec2::new(100.0, 100.0);
        let (_, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::TopLeft,
            Anchor::TopLeft,
            (800.0, 600.0),
        );
        assert!(
            new_size.x >= MIN_UI_SIZE,
            "size.x below min: {}",
            new_size.x
        );
        assert!(
            new_size.y >= MIN_UI_SIZE,
            "size.y below min: {}",
            new_size.y
        );
    }

    #[test]
    fn resize_bottomright_grows() {
        let start_offset = glam::Vec2::new(10.0, 10.0);
        let start_size = glam::Vec2::new(100.0, 50.0);
        let delta = glam::Vec2::new(30.0, 20.0);
        let (new_offset, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::BottomRight,
            Anchor::TopLeft,
            (800.0, 600.0),
        );
        // offset unchanged for BottomRight with TopLeft anchor.
        assert!(
            (new_offset - start_offset).length() < 0.001,
            "offset changed"
        );
        assert!((new_size.x - 130.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 70.0).abs() < 0.001, "size.y");
    }

    /// For a Center-anchored node, dragging the BottomRight handle must grow
    /// the size WITHOUT shifting the widget's top-left screen position.
    ///
    /// screen_pos = base(anchor, size, vw, vh) + offset
    ///
    /// Before: size=(100,50), offset=(0,0), vp=(800,600)
    ///   base = ((800-100)/2, (600-50)/2) = (350, 275)  → screen_pos = (350, 275)
    ///
    /// After drag BottomRight by (30, 20): size=(130,70)
    ///   For screen_pos to stay at (350, 275) we need:
    ///   base_new = ((800-130)/2, (600-70)/2) = (335, 265)
    ///   new_offset = screen_pos - base_new = (15, 10)
    ///
    /// The fix applies: offset += base(start_size) - base(new_size)
    ///   = (350,275) - (335,265) = (15,10)  ✓
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resize_center_anchor_fixed_corner_preserved() {
        let vp = (800.0_f32, 600.0_f32);
        let start_size = glam::Vec2::new(100.0, 50.0);
        let start_offset = glam::Vec2::ZERO;
        let anchor = Anchor::Center;

        // Compute screen_pos before resize.
        let base_before = anchor_base(anchor, start_size, vp.0, vp.1);
        let screen_pos_before = base_before + start_offset;

        let delta = glam::Vec2::new(30.0, 20.0);
        let (new_offset, new_size) = ui_resize_new_layout(
            start_offset,
            start_size,
            delta,
            ResizeHandle::BottomRight,
            anchor,
            vp,
        );

        // Compute screen_pos after resize.
        let base_after = anchor_base(anchor, new_size, vp.0, vp.1);
        let screen_pos_after = base_after + new_offset;

        assert!(
            (screen_pos_after - screen_pos_before).length() < 0.001,
            "top-left screen position shifted: before={screen_pos_before:?} after={screen_pos_after:?}"
        );
        // Size did grow as expected.
        assert!((new_size.x - 130.0).abs() < 0.001, "size.x");
        assert!((new_size.y - 70.0).abs() < 0.001, "size.y");
    }

    // ─── rotation gizmo ──────────────────────────────────────────────────────

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rotation_helpers_math() {
        use std::f32::consts::PI;
        // cursor_angle cardinal directions (Y down: +y is "down").
        assert!(cursor_angle(glam::Vec2::ZERO, glam::Vec2::new(10.0, 0.0)).abs() < 1e-5);
        assert!(
            (cursor_angle(glam::Vec2::ZERO, glam::Vec2::new(0.0, 10.0)) - PI / 2.0).abs() < 1e-5
        );
        // Rotation handle sits above the top edge (smaller y).
        assert_eq!(
            rotation_handle_pos(
                glam::Vec2::new(50.0, 50.0),
                glam::Vec2::new(20.0, 20.0),
                16.0
            ),
            glam::Vec2::new(50.0, 24.0)
        );
        // Snap to nearest multiple; step<=0 is identity.
        assert!(snap_angle(0.4, 1.0).abs() < 1e-6);
        assert!((snap_angle(0.6, 1.0) - 1.0).abs() < 1e-6);
        assert_eq!(snap_angle(0.7, 0.0), 0.7);
        // applied_rotation = start + (cur - start_angle).
        assert!((applied_rotation(0.0, -PI / 2.0, 0.0, None) - PI / 2.0).abs() < 1e-5);
        assert!((applied_rotation(1.0, 0.5, 0.5, None) - 1.0).abs() < 1e-6); // no delta
    }
}
