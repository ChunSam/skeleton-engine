use glam::{Mat4, Vec2};

/// 2D 카메라 리소스.
///
/// # 좌표 규약 (top-left anchored)
///
/// `position` 은 뷰포트의 **좌상단** 월드 좌표(픽셀 단위)를 가리킨다.
/// 보이는 영역:
///   - X: `[position.x, position.x + width / zoom]`
///   - Y: `[position.y, position.y + height / zoom]`  (Y 아래가 +)
///
/// 플레이어를 화면 중앙에 놓으려면:
///   `camera.position = player_pos - Vec2::new(viewport_w, viewport_h) / (2.0 * zoom)`
///
/// 기본값 `position = Vec2::ZERO, zoom = 1.0` 일 때
/// `view_proj(w, h)` 는 기존 `Mat4::orthographic_rh(0, w, h, 0, -1, 1)` 과 동일하다.
#[derive(Debug, Clone, Copy)]
pub struct Camera {
    /// 뷰포트 좌상단 월드 좌표 (픽셀 단위)
    pub position: Vec2,
    /// 줌 배율. 1.0 = 정상, 2.0 = 2배 확대 (보이는 영역 절반)
    pub zoom: f32,

    // --- Shake ---
    /// 현재 shake 진폭 (픽셀 단위)
    shake_strength: f32,
    /// 남은 shake 지속 시간 (초)
    shake_duration: f32,
    /// shake 샘플링용 경과 시간
    shake_timer: f32,

    // --- Smooth Follow ---
    /// 따라갈 엔티티 (`Entity` 타입은 Copy이므로 `Option<Entity>` 도 Copy)
    pub follow_entity: Option<crate::ecs::Entity>,
    /// 초당 lerp 강도. 0.0 = 추적 없음, 1.0 = 즉시 스냅. 기본값 5.0
    pub lerp_factor: f32,

    // --- Zoom Tween ---
    /// 목표 줌 값
    zoom_target: f32,
    /// 초당 zoom 변화량. 0 = 트윈 비활성
    zoom_tween_speed: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            zoom: 1.0,
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

    /// 화면(픽셀) 좌표를 월드 좌표로 변환한다.
    ///
    /// `screen_pos`: `InputState::cursor()` 가 반환하는 좌상단 기준 픽셀 좌표.
    /// `viewport_w/h`: `ViewportSize` 리소스의 width/height.
    ///
    /// 역연산: world = position + screen / zoom
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        let zoom = self.safe_zoom();
        Vec2::new(
            screen_pos.x / zoom + self.position.x,
            screen_pos.y / zoom + self.position.y,
        )
    }

    /// 월드 좌표를 화면(픽셀) 좌표로 변환한다 — [`screen_to_world`](Self::screen_to_world)
    /// 의 역연산.
    ///
    /// 월드 공간 엔티티 위치에 화면 텍스트/UI 를 배치할 때 쓴다. 반환값은 좌상단
    /// 기준 픽셀 좌표라 `DrawText`/`TextQueue` 에 그대로 넘길 수 있다 (이들은
    /// 카메라의 영향을 받지 않는 스크린 공간이다).
    ///
    /// 연산: screen = (world - position) * zoom
    pub fn world_to_screen(&self, world_pos: Vec2) -> Vec2 {
        (world_pos - self.position) * self.zoom
    }

    /// 0 나눗셈/NaN을 막기 위한 안전 줌 배율. `zoom` 이 0(또는 비정상적으로 작은 값)
    /// 으로 설정돼도 `screen_to_world`/`visible_rect`/`view_proj` 가 NaN 좌표를
    /// 내보내지 않도록 한다.
    #[inline]
    fn safe_zoom(&self) -> f32 {
        if self.zoom.abs() < f32::EPSILON {
            f32::EPSILON
        } else {
            self.zoom
        }
    }

    /// 현재 카메라의 월드 공간 가시 AABB를 `(min, max)` 로 반환한다.
    ///
    /// 이 직사각형 밖에 있는 스프라이트는 렌더링해도 화면에 보이지 않으므로 컬링 가능.
    pub fn visible_rect(&self, viewport_w: f32, viewport_h: f32) -> (Vec2, Vec2) {
        let zoom = self.safe_zoom();
        let min = self.position;
        let max = self.position + Vec2::new(viewport_w / zoom, viewport_h / zoom);
        (min, max)
    }

    /// 뷰포트 크기 `(width, height)` 를 받아 MVP 용 직교 투영 행렬을 반환한다.
    ///
    /// shake_offset 이 활성화된 경우 position에 더해 화면을 흔든다.
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

    /// 카메라 흔들기.
    ///
    /// - `strength`: 최대 진폭 (픽셀 단위)
    /// - `duration`: 지속 시간 (초)
    pub fn shake(&mut self, strength: f32, duration: f32) {
        self.shake_strength = strength;
        self.shake_duration = duration;
        self.shake_timer = 0.0;
    }

    /// target_zoom으로 부드럽게 줌.
    ///
    /// - `target_zoom`: 목표 줌 배율
    /// - `speed`: 초당 zoom 변화량 (양수)
    pub fn zoom_to(&mut self, target_zoom: f32, speed: f32) {
        self.zoom_target = target_zoom;
        self.zoom_tween_speed = speed;
    }

    /// 현재 프레임의 shake 오프셋을 반환한다 (view_proj 내부에서 자동 적용됨).
    pub fn shake_offset(&self) -> Vec2 {
        if self.shake_duration <= 0.0 || self.shake_strength <= 0.0 {
            return Vec2::ZERO;
        }
        // 결정론적 의사 난수 오프셋 — 서로 다른 주파수의 sin/cos로 자연스러운 흔들림 연출
        let t = self.shake_timer * 30.0; // ~30 Hz shake frequency
        let ox = (t * 1.7).sin() * self.shake_strength;
        let oy = (t * 2.3).cos() * self.shake_strength;
        Vec2::new(ox, oy)
    }

    /// 카메라 이펙트를 dt 초 진행한다.
    ///
    /// `follow_pos`: 이번 프레임에 따라갈 엔티티의 월드 좌표 (없으면 `None`).
    /// App이 매 프레임 자동 호출한다.
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
        // 카메라가 (100, 50) 이동하면 월드 원점(0,0)은 화면 밖으로 나간다.
        // 즉, view_proj 는 default 와 달라야 한다.
        let default_m = Camera::default().view_proj(W, H);
        assert!(
            !m.abs_diff_eq(default_m, 1e-6),
            "camera position had no effect on view_proj"
        );
        // 직접 검증: 이동한 카메라의 left/right/top/bottom 확인
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
        // 화면 좌상단(0,0)은 카메라 position과 같아야 한다
        assert_eq!(world, Vec2::new(50.0, 80.0));
    }

    #[test]
    fn screen_to_world_with_zoom() {
        let cam = Camera::new(Vec2::ZERO, 2.0);
        // zoom=2 → 화면 픽셀 1개 = 월드 0.5 단위
        let world = cam.screen_to_world(Vec2::new(100.0, 60.0));
        assert_eq!(world, Vec2::new(50.0, 30.0));
    }

    #[test]
    fn zoom_scales_visible_region() {
        let cam = Camera::new(Vec2::ZERO, 2.0);
        let m = cam.view_proj(W, H);
        // zoom=2 → 보이는 영역이 절반: right = W/2, bottom = H/2
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
        // zoom == 0 이어도 좌표 변환/투영이 NaN 을 내보내지 않아야 한다.
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
}
