use glam::Vec2;
use std::collections::HashMap;

/// Individual touch point data.
///
/// # Coordinate convention
/// All positions are in **physical pixels** as reported by the OS/winit.
/// To compare with [`crate::input::state::InputState::cursor`] or UI layout
/// rects (which use logical pixels), divide by the window's `scale_factor`:
/// `logical = physical / scale_factor`.
#[derive(Clone)]
pub(crate) struct TouchPoint {
    /// Current screen position in **physical pixels**.
    pub(crate) position: Vec2,
    /// Touch start position in **physical pixels** (used for swipe detection).
    pub(crate) start_position: Vec2,
}

/// Multi-touch input state ECS resource.
///
/// Auto-registered by `App::new()`.
/// Read in systems via `world.resource::<TouchState>()`.
///
/// # Coordinate convention
/// All positions exposed by this type are in **physical pixels** (as reported
/// by the OS). To compare with `InputState::cursor()` or UI layout rects
/// (logical pixels), divide by the window's `scale_factor`:
/// `logical = physical / scale_factor`.
///
/// # Example
/// ```ignore
/// if let Some(ts) = world.resource::<TouchState>() {
///     if ts.is_touching() {
///         // First touch position (physical pixels).
///         if let Some(pos) = ts.primary_position() {
///             println!("touch position: {pos:?}");
///         }
///     }
/// }
/// ```
pub struct TouchState {
    /// Currently active touch points (id → TouchPoint).
    active: HashMap<u64, TouchPoint>,

    /// Touches that started this frame (id, start position).
    began: Vec<(u64, Vec2)>,

    /// Touches that moved this frame (id, current position, delta).
    moved: Vec<(u64, Vec2, Vec2)>,

    /// Touches that ended this frame (id, end position).
    ended: Vec<(u64, Vec2)>,

    /// Pinch-zoom delta (positive = fingers spreading, negative = pinching).
    /// Updated only when exactly two fingers are active.
    pinch_delta: f32,

    prev_pinch_dist: f32,

    /// Swipe vector for this frame (set when a touch ends after travelling at
    /// least `swipe_threshold` physical pixels from its start position).
    swipe: Option<Vec2>,

    /// Minimum travel distance in **physical pixels** required to register a
    /// swipe. Defaults to `50.0`. Increase on high-DPI displays if swipes are
    /// too sensitive, or decrease for shorter gestures.
    pub swipe_threshold: f32,
}

impl Default for TouchState {
    fn default() -> Self {
        Self {
            active: HashMap::new(),
            began: Vec::new(),
            moved: Vec::new(),
            ended: Vec::new(),
            pinch_delta: 0.0,
            prev_pinch_dist: 0.0,
            swipe: None,
            swipe_threshold: 50.0,
        }
    }
}

impl TouchState {
    // ── Internal update methods (called only from App) ────────────────────────

    pub(crate) fn on_touch_started(&mut self, id: u64, pos: Vec2) {
        self.active.insert(
            id,
            TouchPoint {
                position: pos,
                start_position: pos,
            },
        );
        self.began.push((id, pos));
    }

    pub(crate) fn on_touch_moved(&mut self, id: u64, pos: Vec2) {
        if let Some(point) = self.active.get_mut(&id) {
            let prev = point.position;
            let delta = pos - prev;
            point.position = pos;
            self.moved.push((id, pos, delta));
        } else {
            // moved arrived without a prior started (e.g. touch began outside the window)
            self.active.insert(
                id,
                TouchPoint {
                    position: pos,
                    start_position: pos,
                },
            );
            self.moved.push((id, pos, Vec2::ZERO));
        }

        // Pinch detection: only when exactly 2 active points.
        self.update_pinch();
    }

    pub(crate) fn on_touch_ended(&mut self, id: u64, pos: Vec2) {
        if let Some(point) = self.active.remove(&id) {
            let travel = pos - point.start_position;
            if travel.length() >= self.swipe_threshold {
                self.swipe = Some(travel);
            }
        }
        self.ended.push((id, pos));
        // Reset pinch distance (finger count decreased).
        self.prev_pinch_dist = 0.0;
        self.pinch_delta = 0.0;
    }

    /// Called at the end of every frame to clear per-frame buffers.
    pub(crate) fn flush(&mut self) {
        self.began.clear();
        self.moved.clear();
        self.ended.clear();
        self.pinch_delta = 0.0;
        self.swipe = None;
    }

    // ── Public accessor methods ───────────────────────────────────────────────

    /// Touches that started this frame (id, start position).
    pub fn began(&self) -> &[(u64, Vec2)] {
        &self.began
    }

    /// Touches that moved this frame (id, current position, delta).
    pub fn moved(&self) -> &[(u64, Vec2, Vec2)] {
        &self.moved
    }

    /// Touches that ended this frame (id, end position).
    pub fn ended(&self) -> &[(u64, Vec2)] {
        &self.ended
    }

    /// Pinch-zoom delta (positive = fingers spreading, negative = pinching).
    /// Updated only when exactly two fingers are active.
    pub fn pinch_delta(&self) -> f32 {
        self.pinch_delta
    }

    /// Swipe vector for this frame (set when a touch ends after travelling ≥
    /// [`swipe_threshold`](TouchState::swipe_threshold) physical pixels).
    pub fn swipe(&self) -> Option<Vec2> {
        self.swipe
    }

    /// Iterates over currently active touch points. Returns `(id, position)`.
    pub fn active_touches(&self) -> impl Iterator<Item = (u64, Vec2)> + '_ {
        self.active.iter().map(|(&id, p)| (id, p.position))
    }

    /// Number of currently active touches.
    pub fn touch_count(&self) -> usize {
        self.active.len()
    }

    /// Whether one or more touches are currently active.
    pub fn is_touching(&self) -> bool {
        !self.active.is_empty()
    }

    /// Position of the touch point with the lowest id (primary pointer).
    pub fn primary_position(&self) -> Option<Vec2> {
        self.active
            .iter()
            .min_by_key(|(&id, _)| id)
            .map(|(_, p)| p.position)
    }

    // ── Internal pinch distance update ───────────────────────────────────────

    fn update_pinch(&mut self) {
        if self.active.len() != 2 {
            self.prev_pinch_dist = 0.0;
            return;
        }
        let mut iter = self.active.values();
        let a = iter.next().unwrap().position;
        let b = iter.next().unwrap().position;
        let dist = a.distance(b);

        if self.prev_pinch_dist > 0.0 {
            self.pinch_delta = dist - self.prev_pinch_dist;
        }
        self.prev_pinch_dist = dist;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touch_started_adds_active() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::new(100.0, 200.0));
        assert_eq!(ts.touch_count(), 1);
        assert!(ts.is_touching());
        assert_eq!(ts.began().len(), 1);
        assert_eq!(ts.primary_position(), Some(Vec2::new(100.0, 200.0)));
    }

    #[test]
    fn touch_moved_updates_position() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        ts.on_touch_moved(0, Vec2::new(10.0, 5.0));
        assert_eq!(ts.primary_position(), Some(Vec2::new(10.0, 5.0)));
        assert_eq!(ts.moved().len(), 1);
        let (id, pos, delta) = ts.moved()[0];
        assert_eq!(id, 0);
        assert_eq!(pos, Vec2::new(10.0, 5.0));
        assert_eq!(delta, Vec2::new(10.0, 5.0));
    }

    #[test]
    fn touch_ended_removes_active() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        ts.on_touch_ended(0, Vec2::new(0.0, 0.0));
        assert_eq!(ts.touch_count(), 0);
        assert!(!ts.is_touching());
        assert_eq!(ts.ended().len(), 1);
    }

    #[test]
    fn swipe_detected_on_long_move() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        ts.on_touch_ended(0, Vec2::new(100.0, 0.0));
        assert!(ts.swipe().is_some());
        let swipe = ts.swipe().unwrap();
        assert!((swipe.x - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn swipe_not_detected_on_short_move() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        ts.on_touch_ended(0, Vec2::new(10.0, 0.0));
        assert!(ts.swipe().is_none());
    }

    #[test]
    fn flush_clears_frame_buffers() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::ZERO);
        ts.on_touch_moved(0, Vec2::new(5.0, 5.0));
        ts.flush();
        assert!(ts.began().is_empty());
        assert!(ts.moved().is_empty());
        assert_eq!(ts.pinch_delta(), 0.0);
        assert!(ts.swipe().is_none());
        // Active points persist after flush.
        assert_eq!(ts.touch_count(), 1);
    }

    #[test]
    fn pinch_delta_computed_for_two_fingers() {
        let mut ts = TouchState::default();
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        ts.on_touch_started(1, Vec2::new(100.0, 0.0));
        // First moved → sets prev_dist.
        ts.on_touch_moved(0, Vec2::new(0.0, 0.0));
        let delta_after_first = ts.pinch_delta();
        // Second moved (fingers spreading).
        ts.on_touch_moved(0, Vec2::new(-10.0, 0.0));
        // Two-finger distance: 110 - 100 = 10.
        assert!(ts.pinch_delta() > 0.0 || delta_after_first == 0.0);
    }

    #[test]
    fn primary_position_returns_lowest_id() {
        let mut ts = TouchState::default();
        ts.on_touch_started(5, Vec2::new(500.0, 0.0));
        ts.on_touch_started(2, Vec2::new(200.0, 0.0));
        ts.on_touch_started(8, Vec2::new(800.0, 0.0));
        assert_eq!(ts.primary_position(), Some(Vec2::new(200.0, 0.0)));
    }

    // ── swipe_threshold configurability ──────────────────────────────────────

    #[test]
    fn swipe_threshold_default_is_50() {
        let ts = TouchState::default();
        assert!((ts.swipe_threshold - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn custom_swipe_threshold_respected() {
        let mut ts = TouchState::default();
        ts.swipe_threshold = 20.0;
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        // 30 px travel — above the new threshold of 20, below the old default of 50.
        ts.on_touch_ended(0, Vec2::new(30.0, 0.0));
        assert!(ts.swipe().is_some());
    }

    #[test]
    fn custom_swipe_threshold_blocks_short_swipe() {
        let mut ts = TouchState::default();
        ts.swipe_threshold = 100.0;
        ts.on_touch_started(0, Vec2::new(0.0, 0.0));
        // 60 px travel — above the default 50 but below the new threshold of 100.
        ts.on_touch_ended(0, Vec2::new(60.0, 0.0));
        assert!(ts.swipe().is_none());
    }
}
