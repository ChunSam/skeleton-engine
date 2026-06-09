use glam::{Mat4, Vec2};

/// 2D camera resource.
///
/// # Coordinate convention (top-left anchored)
///
/// `position` points to the **top-left** world coordinate of the viewport (in pixels).
/// Visible region:
///   - X: `[position.x, position.x + width / zoom]`
///   - Y: `[position.y, position.y + height / zoom]`  (Y increases downward)
///
/// To center the player on screen:
///   `camera.position = player_pos - Vec2::new(viewport_w, viewport_h) / (2.0 * zoom)`
///
/// With default `position = Vec2::ZERO, zoom = 1.0`,
/// `view_proj(w, h)` is equivalent to `Mat4::orthographic_rh(0, w, h, 0, -1, 1)`.
///
/// # World-bounds clamping
///
/// Set `bounds = Some((min, max))` to constrain the camera so the visible viewport never
/// scrolls outside the world rectangle. Call [`clamp_to_bounds`](Self::clamp_to_bounds)
/// (or let App call it automatically each frame) after positioning the camera.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// Top-left world coordinate of the viewport (in pixels)
    pub position: Vec2,
    /// Zoom multiplier. 1.0 = normal, 2.0 = 2× in (visible area halved)
    pub zoom: f32,

    // --- World bounds clamping ---
    /// Optional world-space bounding rectangle `(min, max)` that the camera viewport
    /// must stay inside. Set `Some((Vec2::ZERO, Vec2::new(world_w, world_h)))` to keep
    /// the camera from scrolling past the level edges.
    ///
    /// `None` (the default) disables clamping entirely.
    pub bounds: Option<(Vec2, Vec2)>,

    // --- Shake ---
    /// Current shake amplitude (pixels)
    shake_strength: f32,
    /// Remaining shake duration (seconds)
    shake_duration: f32,
    /// Elapsed time for shake sampling
    shake_timer: f32,

    // --- Smooth Follow ---
    /// Entity to follow (`Entity` is Copy, so `Option<Entity>` is also Copy)
    pub follow_entity: Option<crate::ecs::Entity>,
    /// Lerp strength per second. 0.0 = no tracking, 1.0 = instant snap. Default 5.0
    pub lerp_factor: f32,

    // --- Zoom Tween ---
    /// Target zoom value
    zoom_target: f32,
    /// Zoom change per second. 0 = tween inactive
    zoom_tween_speed: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
            bounds: None,
            shake_strength: 0.0,
            shake_duration: 0.0,
            shake_timer: 0.0,
            follow_entity: None,
            lerp_factor: 5.0,
            zoom_target: 1.0,
            zoom_tween_speed: 0.0,
        }
    }
}

impl Camera {
    pub fn new(position: Vec2, zoom: f32) -> Self {
        Self {
            position,
            zoom,
            ..Self::default()
        }
    }

    /// Converts screen (pixel) coordinates to world coordinates.
    ///
    /// `screen_pos`: pixel coordinate relative to the top-left corner, as returned by `InputState::cursor()`.
    /// `viewport_w/h`: width/height from the `ViewportSize` resource.
    ///
    /// Inverse: world = position + screen / zoom
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let zoom = self.safe_zoom();
        Vec2::new(
            screen_pos.x / zoom + self.position.x,
            screen_pos.y / zoom + self.position.y,
        )
    }

    /// Converts world coordinates to screen (pixel) coordinates — the inverse of
    /// [`screen_to_world`](Self::screen_to_world).
    ///
    /// Useful for placing screen-space text/UI at a world-space entity position.
    /// The return value is a top-left-relative pixel coordinate that can be passed
    /// directly to `DrawText`/`TextQueue` (which operate in screen space, unaffected
    /// by the camera).
    ///
    /// Formula: screen = (world - position) * zoom
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        (world_pos - self.position) * self.zoom
    }

    /// Safe zoom multiplier to prevent division-by-zero/NaN. Even if `zoom` is set to
    /// 0 (or an abnormally small value), `screen_to_world`/`visible_rect`/`view_proj`
    /// will not emit NaN coordinates.
    #[inline]
    fn safe_zoom(&self) -> f32 {
        if self.zoom.abs() < f32::EPSILON {
            f32::EPSILON
        } else {
            self.zoom
        }
    }

    /// Returns the world-space visible AABB of the current camera as `(min, max)`.
    ///
    /// Sprites outside this rectangle are not visible on screen and can be culled.
    pub fn visible_rect(&self, viewport_w: f32, viewport_h: f32) -> (Vec2, Vec2) {
        let zoom = self.safe_zoom();
        let min = self.position;
        let max = self.position + Vec2::new(viewport_w / zoom, viewport_h / zoom);
        (min, max)
    }

    /// Returns an orthographic projection matrix for MVP, given viewport `(width, height)`.
    ///
    /// When shake_offset is active it is added to position to shake the screen.
    ///
    /// left = position.x,  right = position.x + width/zoom
    /// top  = position.y,  bottom = position.y + height/zoom
    pub fn view_proj(&self, width: f32, height: f32) -> Mat4 {
        let zoom = self.safe_zoom();
        let pos = self.position + self.shake_offset();
        let left = pos.x;
        let right = pos.x + width / zoom;
        let top = pos.y;
        let bottom = pos.y + height / zoom;
        Mat4::orthographic_rh(left, right, bottom, top, -1.0, 1.0)
    }

    // ── Camera Effects ────────────────────────────────────────────────────────

    /// Shake the camera.
    ///
    /// - `strength`: maximum amplitude (pixels)
    /// - `duration`: duration (seconds)
    pub fn shake(&mut self, strength: f32, duration: f32) {
        self.shake_strength = strength;
        self.shake_duration = duration;
        self.shake_timer = 0.0;
    }

    /// Smoothly zoom toward target_zoom.
    ///
    /// - `target_zoom`: target zoom multiplier
    /// - `speed`: zoom change per second (positive)
    pub fn zoom_to(&mut self, target_zoom: f32, speed: f32) {
        self.zoom_target = target_zoom;
        self.zoom_tween_speed = speed;
    }

    /// Returns the shake offset for the current frame (automatically applied inside `view_proj`).
    pub fn shake_offset(&self) -> Vec2 {
        if self.shake_duration <= 0.0 || self.shake_strength <= 0.0 {
            return Vec2::ZERO;
        }
        // Deterministic pseudo-random offset — sin/cos at different frequencies for natural-feeling shake
        let t = self.shake_timer * 30.0; // ~30 Hz shake frequency
        let ox = (t * 1.7).sin() * self.shake_strength;
        let oy = (t * 2.3).cos() * self.shake_strength;
        Vec2::new(ox, oy)
    }

    /// Advances all camera effects by `dt` seconds.
    ///
    /// `follow_pos`: world position of the entity to follow this frame (`None` if none).
    /// Called automatically every frame by App.
    pub fn update(&mut self, dt: f32, follow_pos: Option<Vec2>) {
        // 1. Smooth follow
        if let Some(pos) = follow_pos {
            let factor = (self.lerp_factor * dt).min(1.0);
            self.position = self.position + (pos - self.position) * factor;
        }

        // 2. Zoom tween
        if self.zoom_tween_speed > 0.0 {
            let diff = self.zoom_target - self.zoom;
            let step = self.zoom_tween_speed * dt;
            if diff.abs() <= step {
                self.zoom = self.zoom_target;
                self.zoom_tween_speed = 0.0;
            } else {
                self.zoom += diff.signum() * step;
            }
        }

        // 3. Shake decay
        if self.shake_duration > 0.0 {
            self.shake_duration -= dt;
            self.shake_timer += dt;
            if self.shake_duration < 0.0 {
                self.shake_duration = 0.0;
                self.shake_strength = 0.0;
            }
        }
    }

    /// Clamps `position` so the visible viewport stays inside `bounds`, if set.
    ///
    /// Call this after any code that repositions the camera (including smooth-follow).
    /// App calls it automatically each frame right after [`update`](Self::update), so
    /// callers that rely on the built-in follow mechanism do not need to call it manually.
    ///
    /// # Edge case — world smaller than viewport
    ///
    /// When the world is narrower (or shorter) than the visible area on an axis
    /// (`bounds.max - bounds.min < viewport / zoom`), the clamp range would invert
    /// (min > max). In that case `position` on that axis is pinned to `bounds.min`,
    /// centering the world content at the top-left corner of the viewport rather than
    /// producing an undefined result.
    pub fn clamp_to_bounds(&mut self, viewport_w: f32, viewport_h: f32) {
        let Some((bmin, bmax)) = self.bounds else {
            return;
        };
        let zoom = self.safe_zoom();
        let visible_w = viewport_w / zoom;
        let visible_h = viewport_h / zoom;

        // Allowed range for position.x: [bmin.x, bmax.x - visible_w]
        // If world < viewport the upper bound goes below the lower bound; pin to bmin.
        let lo_x = bmin.x;
        let hi_x = (bmax.x - visible_w).max(bmin.x);
        let lo_y = bmin.y;
        let hi_y = (bmax.y - visible_h).max(bmin.y);

        self.position.x = self.position.x.clamp(lo_x, hi_x);
        self.position.y = self.position.y.clamp(lo_y, hi_y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: f32 = 800.0;
    const H: f32 = 600.0;

    #[test]
    fn world_to_screen_is_inverse_of_screen_to_world() {
        let cam = Camera::new(Vec2::new(120.0, -40.0), 2.0);
        // world → screen → world round-trips.
        let world = Vec2::new(300.0, 150.0);
        let screen = cam.world_to_screen(world);
        assert_eq!(screen, (world - cam.position) * cam.zoom);
        let back = cam.screen_to_world(screen);
        assert!(
            (back - world).length() < 1e-3,
            "round-trip drifted: {back:?}"
        );

        // The camera's own position maps to screen origin (top-left).
        assert_eq!(cam.world_to_screen(cam.position), Vec2::ZERO);
    }

    #[test]
    fn default_matches_legacy_ortho() {
        let got = Camera::default().view_proj(W, H);
        let expected = Mat4::orthographic_rh(0.0, W, H, 0.0, -1.0, 1.0);
        assert!(
            got.abs_diff_eq(expected, 1e-6),
            "default view_proj differs from legacy ortho\ngot:      {got:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn camera_position_translates_view() {
        let cam = Camera::new(Vec2::new(100.0, 50.0), 1.0);
        let m = cam.view_proj(W, H);
        // When the camera moves to (100, 50) the world origin (0,0) is off-screen.
        // Therefore view_proj must differ from the default.
        let default_m = Camera::default().view_proj(W, H);
        assert!(
            !m.abs_diff_eq(default_m, 1e-6),
            "camera position had no effect on view_proj"
        );
        // Direct verification: check left/right/top/bottom for the translated camera
        let expected = Mat4::orthographic_rh(100.0, 100.0 + W, 50.0 + H, 50.0, -1.0, 1.0);
        assert!(
            m.abs_diff_eq(expected, 1e-6),
            "translated view_proj mismatch\ngot:      {m:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn screen_to_world_no_offset_no_zoom() {
        let cam = Camera::default(); // position=(0,0), zoom=1
        let world = cam.screen_to_world(Vec2::new(100.0, 200.0));
        assert_eq!(world, Vec2::new(100.0, 200.0));
    }

    #[test]
    fn screen_to_world_with_camera_offset() {
        let cam = Camera::new(Vec2::new(50.0, 80.0), 1.0);
        let world = cam.screen_to_world(Vec2::new(0.0, 0.0));
        // The screen top-left (0,0) must equal the camera position
        assert_eq!(world, Vec2::new(50.0, 80.0));
    }

    #[test]
    fn screen_to_world_with_zoom() {
        let cam = Camera::new(Vec2::ZERO, 2.0);
        // zoom=2 → 1 screen pixel = 0.5 world units
        let world = cam.screen_to_world(Vec2::new(100.0, 60.0));
        assert_eq!(world, Vec2::new(50.0, 30.0));
    }

    #[test]
    fn zoom_scales_visible_region() {
        let cam = Camera::new(Vec2::ZERO, 2.0);
        let m = cam.view_proj(W, H);
        // zoom=2 → visible area is halved: right = W/2, bottom = H/2
        let expected = Mat4::orthographic_rh(0.0, W / 2.0, H / 2.0, 0.0, -1.0, 1.0);
        assert!(
            m.abs_diff_eq(expected, 1e-6),
            "zoom=2 view_proj mismatch\ngot:      {m:?}\nexpected: {expected:?}"
        );
    }

    #[test]
    fn visible_rect_no_zoom() {
        let cam = Camera::new(Vec2::new(100.0, 50.0), 1.0);
        let (min, max) = cam.visible_rect(W, H);
        assert_eq!(min, Vec2::new(100.0, 50.0));
        assert_eq!(max, Vec2::new(100.0 + W, 50.0 + H));
    }

    #[test]
    fn visible_rect_with_zoom() {
        let cam = Camera::new(Vec2::ZERO, 2.0);
        let (min, max) = cam.visible_rect(W, H);
        assert_eq!(min, Vec2::ZERO);
        assert_eq!(max, Vec2::new(W / 2.0, H / 2.0));
    }

    // ── Camera Effects tests ──────────────────────────────────────────────────

    #[test]
    fn shake_offset_zero_when_inactive() {
        let cam = Camera::default();
        assert_eq!(cam.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn shake_decays_over_time() {
        let mut cam = Camera::default();
        cam.shake(10.0, 0.1);
        assert!(cam.shake_duration > 0.0);
        cam.update(0.2, None); // dt > duration → shake ends
        assert_eq!(cam.shake_duration, 0.0);
        assert_eq!(cam.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn zoom_tween_reaches_target() {
        let mut cam = Camera::default();
        cam.zoom_to(2.0, 10.0); // speed=10/sec, gap=1.0 → needs 0.1s
        cam.update(0.5, None); // 0.5s well exceeds needed time
        assert_eq!(cam.zoom, 2.0);
        assert_eq!(cam.zoom_tween_speed, 0.0); // tween ended
    }

    #[test]
    fn zoom_tween_partial_progress() {
        let mut cam = Camera {
            zoom: 1.0,
            ..Default::default()
        };
        cam.zoom_to(3.0, 4.0); // speed=4/sec, gap=2.0 → needs 0.5s
        cam.update(0.25, None); // half the time → zoom = 1.0 + 4.0*0.25 = 2.0
        assert!((cam.zoom - 2.0).abs() < 1e-5);
        assert!(cam.zoom_tween_speed > 0.0); // still tweening
    }

    #[test]
    fn smooth_follow_lerps_toward_target() {
        let mut cam = Camera {
            position: Vec2::ZERO,
            lerp_factor: 10.0,
            ..Default::default()
        };
        // follow_pos = (100, 0), dt = 0.1s → factor = min(10*0.1, 1.0) = 1.0 → snap
        cam.update(0.1, Some(Vec2::new(100.0, 0.0)));
        assert!((cam.position.x - 100.0).abs() < 1e-5);
    }

    #[test]
    fn smooth_follow_no_pos_does_not_move() {
        let mut cam = Camera {
            position: Vec2::new(50.0, 50.0),
            ..Default::default()
        };
        cam.update(0.016, None);
        assert_eq!(cam.position, Vec2::new(50.0, 50.0));
    }

    #[test]
    fn shake_active_produces_nonzero_offset() {
        let mut cam = Camera::default();
        cam.shake(20.0, 1.0);
        cam.update(0.016, None); // advance timer
                                 // After some time shake_timer > 0, offset should be non-zero
        let offset = cam.shake_offset();
        // At least one component should be non-zero (sin/cos won't both be 0 at 0.016*30~0.48)
        assert!(offset.x != 0.0 || offset.y != 0.0);
    }

    #[test]
    fn zoom_zero_does_not_produce_nan() {
        // Even when zoom == 0, coordinate transforms/projection must not produce NaN.
        let cam = Camera::new(Vec2::new(10.0, 20.0), 0.0);
        let w = cam.screen_to_world(Vec2::new(100.0, 50.0));
        assert!(
            w.x.is_finite() && w.y.is_finite(),
            "screen_to_world NaN: {w:?}"
        );
        let (min, max) = cam.visible_rect(W, H);
        assert!(
            min.x.is_finite() && max.x.is_finite() && max.y.is_finite(),
            "visible_rect NaN: {min:?}..{max:?}"
        );
        let m = cam.view_proj(W, H);
        assert!(
            m.to_cols_array().iter().all(|v| v.is_finite()),
            "view_proj NaN: {m:?}"
        );
    }

    // ── clamp_to_bounds tests ─────────────────────────────────────────────────

    #[test]
    fn clamp_to_bounds_none_is_noop() {
        // With no bounds set, position must be unchanged.
        let mut cam = Camera::new(Vec2::new(500.0, 500.0), 1.0);
        cam.bounds = None;
        cam.clamp_to_bounds(W, H);
        assert_eq!(cam.position, Vec2::new(500.0, 500.0));
    }

    #[test]
    fn clamp_to_bounds_clamps_to_min() {
        // Camera scrolled left/above the world origin: must be pinned at (0, 0).
        let mut cam = Camera::new(Vec2::new(-100.0, -50.0), 1.0);
        cam.bounds = Some((Vec2::ZERO, Vec2::new(2000.0, 1500.0)));
        cam.clamp_to_bounds(W, H);
        assert_eq!(cam.position.x, 0.0);
        assert_eq!(cam.position.y, 0.0);
    }

    #[test]
    fn clamp_to_bounds_clamps_to_max() {
        // Camera scrolled past the right/bottom edge.
        // World 2000×1500, viewport 800×600 → max position = (1200, 900).
        let mut cam = Camera::new(Vec2::new(1500.0, 1200.0), 1.0);
        cam.bounds = Some((Vec2::ZERO, Vec2::new(2000.0, 1500.0)));
        cam.clamp_to_bounds(W, H);
        assert!(
            (cam.position.x - 1200.0).abs() < 1e-3,
            "x={}",
            cam.position.x
        );
        assert!(
            (cam.position.y - 900.0).abs() < 1e-3,
            "y={}",
            cam.position.y
        );
    }

    #[test]
    fn clamp_to_bounds_inside_range_unchanged() {
        // Position already inside the valid range must not move.
        let mut cam = Camera::new(Vec2::new(400.0, 300.0), 1.0);
        cam.bounds = Some((Vec2::ZERO, Vec2::new(2000.0, 1500.0)));
        cam.clamp_to_bounds(W, H);
        assert_eq!(cam.position, Vec2::new(400.0, 300.0));
    }

    #[test]
    fn clamp_to_bounds_zoom_is_accounted_for() {
        // zoom=2 halves the visible area (400×300). World 800×600 → max position (400, 300).
        let mut cam = Camera::new(Vec2::new(600.0, 500.0), 2.0);
        cam.bounds = Some((Vec2::ZERO, Vec2::new(800.0, 600.0)));
        cam.clamp_to_bounds(W, H); // viewport W=800, H=600 → visible 400×300
        assert!(
            (cam.position.x - 400.0).abs() < 1e-3,
            "x={}",
            cam.position.x
        );
        assert!(
            (cam.position.y - 300.0).abs() < 1e-3,
            "y={}",
            cam.position.y
        );
    }

    #[test]
    fn clamp_to_bounds_world_smaller_than_viewport_pins_to_min() {
        // World 400×300 is smaller than the 800×600 viewport at zoom=1.
        // The clamp hi values go below lo, so position must be pinned to bounds.min.
        let mut cam = Camera::new(Vec2::new(50.0, 50.0), 1.0);
        cam.bounds = Some((Vec2::new(10.0, 20.0), Vec2::new(410.0, 320.0)));
        cam.clamp_to_bounds(W, H); // visible area 800×600, world 400×300 → smaller
        assert!((cam.position.x - 10.0).abs() < 1e-3, "x={}", cam.position.x);
        assert!((cam.position.y - 20.0).abs() < 1e-3, "y={}", cam.position.y);
    }
}
