//! Deterministic RNG + weighted random selection.
//!
//! [`Rng`] is a tiny **seedable, deterministic** pseudo-random generator (SplitMix64): the same
//! `seed` always yields the same stream, so a game can store just the seed to reproduce a run,
//! a procedural level, or a loot sequence exactly — independent of `rand`'s thread RNG. It is a
//! plain owned helper (like [`PathGrid`](crate::pathfinding::PathGrid) /
//! [`FovMap`](crate::fov::FovMap)), not an ECS resource: keep one in a system, a struct, or a
//! World resource as you like, and seed it yourself.
//!
//! [`WeightedTable`] draws an item from a weighted set — the loot-drop / spawn-table primitive.
//! Picks are driven by a caller-supplied [`Rng`], so a table + a seed reproduce the same draws.
//!
//! ```
//! use engine::{Rng, WeightedTable};
//!
//! let mut rng = Rng::new(1234);
//! let a = rng.next_u64();
//! let mut rng2 = Rng::new(1234);
//! assert_eq!(a, rng2.next_u64(), "same seed → same stream");
//!
//! // Common drops ~6× as often as rare, but the draw is deterministic from the seed.
//! let loot = WeightedTable::new().with("common", 60.0).with("rare", 10.0);
//! let drop = loot.pick(&mut rng).copied();
//! assert!(drop == Some("common") || drop == Some("rare"));
//! ```

/// A seedable, deterministic pseudo-random generator (SplitMix64).
///
/// Cheap (one `u64` of state), reproducible (seed → stream), and dependency-free. Not
/// cryptographically secure — it is for gameplay reproducibility (seeds, procedural generation,
/// loot), not secrets.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Creates a generator from a 64-bit seed. The same seed always produces the same stream.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// The next raw 64-bit value (SplitMix64). Every other method is built on this, so the whole
    /// stream is fixed by the seed.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// The next raw 32-bit value (the high half of [`next_u64`](Self::next_u64)).
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// A uniform `i32` in `[lo, hi)`. Returns `lo` when the range is empty (`hi <= lo`), never
    /// panics on `% 0`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        lo + (self.next_u64() % (hi - lo) as u64) as i32
    }

    /// A uniform `f32` in `[0.0, 1.0)` (24 bits of mantissa precision).
    pub fn f32_unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// A uniform `f32` in `[lo, hi)` (or `[hi, lo)` — the bounds may be given in either order).
    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.f32_unit() * (hi - lo)
    }

    /// A fair coin flip.
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// `true` with probability `p` (clamped to `0.0..=1.0`).
    pub fn chance(&mut self, p: f32) -> bool {
        self.f32_unit() < p.clamp(0.0, 1.0)
    }

    /// A uniformly-chosen reference into `items`, or `None` if it is empty.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        if items.is_empty() {
            return None;
        }
        Some(&items[self.range(0, items.len() as i32) as usize])
    }

    /// Shuffles `items` in place (Fisher–Yates), deterministically from the current stream.
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.range(0, (i + 1) as i32) as usize;
            items.swap(i, j);
        }
    }
}

/// A set of items with relative weights, from which [`pick`](Self::pick) draws one — the loot-drop
/// / spawn-table primitive. Draws are driven by a caller-supplied [`Rng`], so a table + a seed
/// reproduce the same sequence.
///
/// Weights are relative (they need not sum to 1): an item of weight `2.0` is drawn twice as often
/// as one of weight `1.0`. Non-positive weights are ignored (an item that can never be drawn).
///
/// ```
/// use engine::{Rng, WeightedTable};
///
/// let table = WeightedTable::new()
///     .with('C', 60.0)
///     .with('U', 25.0)
///     .with('R', 12.0)
///     .with('L', 3.0);
/// assert_eq!(table.total_weight(), 100.0);
///
/// let mut rng = Rng::new(7);
/// // Over many draws, 'C' dominates; any single draw is one of the four.
/// assert!(table.pick(&mut rng).is_some());
/// ```
#[derive(Clone, Debug)]
pub struct WeightedTable<T> {
    entries: Vec<(T, f32)>,
    total: f32,
}

impl<T> WeightedTable<T> {
    /// An empty table (no entries; [`pick`](Self::pick) yields `None`).
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            total: 0.0,
        }
    }

    /// Adds an item with the given relative `weight`, returning `self` for chaining. A `weight <=
    /// 0.0` is ignored (the item is stored nowhere and can never be drawn).
    pub fn with(mut self, item: T, weight: f32) -> Self {
        self.add(item, weight);
        self
    }

    /// Adds an item with the given relative `weight`. A `weight <= 0.0` (or non-finite) is a no-op.
    pub fn add(&mut self, item: T, weight: f32) -> &mut Self {
        if weight > 0.0 && weight.is_finite() {
            self.total += weight;
            self.entries.push((item, weight));
        }
        self
    }

    /// The number of drawable entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the table has no drawable entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The sum of all entry weights (`0.0` when empty).
    pub fn total_weight(&self) -> f32 {
        self.total
    }

    /// Draws one item's index, weighted by the entry weights, using `rng`. `None` if the table is
    /// empty. Deterministic for a given `rng` state.
    pub fn pick_index(&self, rng: &mut Rng) -> Option<usize> {
        if self.entries.is_empty() {
            return None;
        }
        let mut r = rng.f32_unit() * self.total;
        for (i, (_, w)) in self.entries.iter().enumerate() {
            r -= w;
            if r < 0.0 {
                return Some(i);
            }
        }
        // Floating-point round-off can leave `r` fractionally >= 0 after the last subtraction;
        // fall back to the final entry rather than returning None.
        Some(self.entries.len() - 1)
    }

    /// Draws one item, weighted by the entry weights, using `rng`. `None` if the table is empty.
    pub fn pick(&self, rng: &mut Rng) -> Option<&T> {
        self.pick_index(rng).map(|i| &self.entries[i].0)
    }

    /// Iterates the items in insertion order (weights dropped).
    pub fn items(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|(t, _)| t)
    }
}

impl<T> Default for WeightedTable<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(0xDEAD_BEEF);
        let mut b = Rng::new(0xDEAD_BEEF);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = Rng::new(1);
        let mut b = Rng::new(2);
        // The very first outputs already differ (SplitMix64 mixes the seed hard).
        assert_ne!(a.next_u64(), b.next_u64());
    }

    /// Golden values lock the stream: if the SplitMix64 constants ever change, every seed-derived
    /// artifact (procedural dungeons, loot sequences) would silently change — this catches it.
    #[test]
    fn stream_is_stable_golden() {
        let mut r = Rng::new(0);
        assert_eq!(r.next_u64(), 16294208416658607535);
        assert_eq!(r.next_u64(), 7960286522194355700);
        assert_eq!(r.next_u64(), 487617019471545679);
    }

    #[test]
    fn range_is_bounded_and_handles_empty() {
        let mut r = Rng::new(9);
        for _ in 0..10_000 {
            let v = r.range(-3, 7);
            assert!((-3..7).contains(&v));
        }
        // Empty / inverted ranges return the low bound instead of panicking on `% 0`.
        assert_eq!(Rng::new(1).range(5, 5), 5);
        assert_eq!(Rng::new(1).range(5, 2), 5);
    }

    #[test]
    fn f32_unit_is_in_range() {
        let mut r = Rng::new(42);
        for _ in 0..10_000 {
            let u = r.f32_unit();
            assert!((0.0..1.0).contains(&u));
        }
    }

    #[test]
    fn chance_extremes_and_rough_rate() {
        let mut r = Rng::new(3);
        // p=0 never fires, p=1 always fires (clamped).
        for _ in 0..100 {
            assert!(!r.chance(0.0));
            assert!(r.chance(1.0));
            assert!(!r.chance(-5.0));
            assert!(r.chance(5.0));
        }
        // p=0.5 lands near half over many trials.
        let hits = (0..10_000).filter(|_| r.chance(0.5)).count();
        assert!((4500..5500).contains(&hits), "chance(0.5) hits = {hits}");
    }

    #[test]
    fn pick_and_shuffle() {
        let mut r = Rng::new(11);
        let items = [10, 20, 30];
        for _ in 0..100 {
            assert!(items.contains(r.pick(&items).unwrap()));
        }
        assert_eq!(r.pick::<i32>(&[]), None);

        // Shuffle is a permutation (same multiset), and deterministic for a seed.
        let mut v: Vec<i32> = (0..20).collect();
        let mut v2 = v.clone();
        Rng::new(5).shuffle(&mut v);
        Rng::new(5).shuffle(&mut v2);
        assert_eq!(v, v2, "same seed → same shuffle");
        let mut sorted = v.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            (0..20).collect::<Vec<_>>(),
            "shuffle preserves the multiset"
        );
    }

    #[test]
    fn weighted_table_empty_and_basics() {
        let mut r = Rng::new(1);
        let empty: WeightedTable<i32> = WeightedTable::new();
        assert!(empty.is_empty());
        assert_eq!(empty.total_weight(), 0.0);
        assert_eq!(empty.pick(&mut r), None);

        let one = WeightedTable::new().with("only", 3.0);
        assert_eq!(one.len(), 1);
        assert_eq!(one.total_weight(), 3.0);
        assert_eq!(one.pick(&mut r), Some(&"only"));
    }

    #[test]
    fn weighted_table_ignores_nonpositive_weights() {
        let t = WeightedTable::new()
            .with("a", 1.0)
            .with("b", 0.0)
            .with("c", -2.0);
        assert_eq!(t.len(), 1);
        assert_eq!(t.total_weight(), 1.0);
    }

    #[test]
    fn weighted_table_respects_weights() {
        // 'common' weight 90, 'rare' weight 10 → ~9:1 over many draws.
        let table = WeightedTable::new().with("common", 90.0).with("rare", 10.0);
        let mut r = Rng::new(2024);
        let mut common = 0;
        let n = 20_000;
        for _ in 0..n {
            if table.pick(&mut r) == Some(&"common") {
                common += 1;
            }
        }
        let frac = common as f32 / n as f32;
        assert!((0.86..0.94).contains(&frac), "common fraction = {frac}");
    }

    #[test]
    fn weighted_table_pick_is_deterministic() {
        let table = WeightedTable::new()
            .with('a', 1.0)
            .with('b', 1.0)
            .with('c', 1.0);
        let mut r1 = Rng::new(77);
        let mut r2 = Rng::new(77);
        let s1: Vec<_> = (0..50).map(|_| table.pick(&mut r1).copied()).collect();
        let s2: Vec<_> = (0..50).map(|_| table.pick(&mut r2).copied()).collect();
        assert_eq!(s1, s2);
    }
}
