use crate::tween::Lerp;
use std::collections::VecDeque;

/// A timestamped value sample for [`SnapshotBuffer`].
#[derive(Clone)]
struct Snapshot<T> {
    t: f64,
    value: T,
}

/// Default number of snapshots retained per [`SnapshotBuffer`].
const DEFAULT_SNAPSHOT_CAPACITY: usize = 8;

/// Buffers timestamped snapshots of one remote value and returns an interpolated value at a render
/// time *in the past* (`now - interp_delay`), so server-owned motion stays smooth despite a low
/// snapshot rate. Clamps at the ends when the render time is outside the buffered range.
///
/// Remote state arrives at the network tick rate (often 10–30 Hz) but renders at 60 Hz. Stamp each
/// snapshot with the client's monotonic clock as it arrives ([`push`](Self::push)), then each frame
/// [`sample`](Self::sample) the value at a slightly delayed render time so playback always has two
/// real samples to interpolate between.
///
/// It is generic over any [`Lerp`] value — `f32` (e.g. a rotation angle), [`glam::Vec2`] (a
/// position), [`Color`](crate::color::Color), etc. — and is **orthogonal** to
/// [`RemoteEntities`](crate::RemoteEntities): that owns the `id → Entity` lifecycle, this owns the per-entity value
/// history the renderer reads. A game keeps them as parallel maps (see
/// `examples/games/orbital_dodger` and `examples/games/predict_shooter`).
///
/// ```
/// # use engine::SnapshotBuffer;
/// let mut buf: SnapshotBuffer<f32> = SnapshotBuffer::new();
/// buf.push(0.0, 0.0);
/// buf.push(1.0, 10.0);
/// assert_eq!(buf.sample(0.5), Some(5.0)); // halfway between the two samples
/// assert_eq!(buf.sample(-1.0), Some(0.0)); // before range → clamps to the first
/// assert_eq!(buf.sample(2.0), Some(10.0)); // after range → clamps to the last
/// ```
pub struct SnapshotBuffer<T: Lerp> {
    samples: VecDeque<Snapshot<T>>,
    /// `pub(super)` so the sibling `tests` module can assert on the clamped floor (it lived in
    /// the same module before the network.rs → network/ split; this keeps that access).
    pub(super) capacity: usize,
}

impl<T: Lerp> Default for SnapshotBuffer<T> {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_SNAPSHOT_CAPACITY)
    }
}

impl<T: Lerp> SnapshotBuffer<T> {
    /// Creates an empty buffer retaining the 8 most recent snapshots (use
    /// [`with_capacity`](Self::with_capacity) to choose a different bound).
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty buffer retaining the most recent `capacity` snapshots (clamped to ≥ 2 so
    /// there is always a pair to interpolate between).
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            samples: VecDeque::new(),
            capacity: capacity.max(2),
        }
    }

    /// Records a snapshot stamped at client time `t` (seconds, monotonic). Out-of-order or duplicate
    /// stamps (`t` not strictly greater than the latest) are ignored so the buffer stays monotonic;
    /// the oldest snapshot is dropped once `capacity` is exceeded.
    pub fn push(&mut self, t: f64, value: T) {
        if let Some(back) = self.samples.back() {
            if t <= back.t {
                return;
            }
        }
        self.samples.push_back(Snapshot { t, value });
        while self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
    }

    /// Interpolated value at render time `rt`. Returns `None` only when the buffer is empty; clamps
    /// to the first/last snapshot when `rt` is before/after the buffered range.
    pub fn sample(&self, rt: f64) -> Option<T> {
        let front = self.samples.front()?;
        if rt <= front.t {
            return Some(front.value.clone());
        }
        let back = self.samples.back()?;
        if rt >= back.t {
            return Some(back.value.clone());
        }
        for i in 0..self.samples.len() - 1 {
            let a = &self.samples[i];
            let b = &self.samples[i + 1];
            if a.t <= rt && rt <= b.t {
                let span = b.t - a.t;
                let f = if span > 0.0 {
                    ((rt - a.t) / span) as f32
                } else {
                    0.0
                };
                return Some(T::lerp(&a.value, &b.value, f));
            }
        }
        Some(back.value.clone())
    }

    /// Whether no snapshots are buffered (so [`sample`](Self::sample) returns `None`).
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// Number of buffered snapshots.
    pub fn len(&self) -> usize {
        self.samples.len()
    }
}
