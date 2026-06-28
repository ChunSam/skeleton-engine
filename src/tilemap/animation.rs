//! Per-tile-value frame animation for [`super::Tilemap`].
//!
//! # Overview
//!
//! Attach a [`TileAnimationSet`] to the **same entity** as a [`super::Tilemap`] to make
//! certain tile values cycle through atlas frames at runtime. Only values listed in
//! the set are animated; all others remain static and incur zero per-frame cost.
//!
//! # How it works (decoupled design)
//!
//! - [`super::system::TilemapSystem`] reads the `TileAnimationSet` while spawning or
//!   updating tile entities.  For animated tile values it also attaches an
//!   [`AnimatedTileCell`] marker to the tile entity, carrying pre-computed [`UvRect`]s
//!   for every frame so no atlas lookup is needed per frame.
//! - [`AnimatedTileSystem`] runs independently every frame and advances the animation
//!   clock for every entity that has both `AnimatedTileCell` and [`UvRect`], overwriting
//!   `UvRect` with the current frame.  It never touches `Tilemap::generation`, so the
//!   generation-cache fast-path in `TilemapSystem` is fully preserved for non-animated
//!   maps.
//!
//! # Example
//! ```rust,no_run
//! use engine::{
//!     Tilemap, TilemapAtlas,
//!     TileAnimation, TileAnimationSet, AnimatedTileSystem,
//!     Vec2, App,
//! };
//!
//! let atlas = TilemapAtlas::new("water.png", 4, 1);
//! let tilemap = Tilemap::new(atlas, vec![vec![1, 2, 1]], 32.0, Vec2::ZERO);
//!
//! let mut anim_set = TileAnimationSet::new();
//! // Tile value 1 (atlas IDs 0-3, 200 ms per frame)
//! anim_set.insert(1, TileAnimation::new(vec![0, 1, 2, 3], 0.2));
//!
//! let mut app = App::new();
//! let map_entity = app.world.spawn();
//! app.world.add_component(map_entity, tilemap);
//! app.world.add_component(map_entity, anim_set);
//! app.add_system(AnimatedTileSystem::new());
//! ```

use std::collections::HashMap;

use crate::ecs::{System, World};
use crate::renderer::uv::UvRect;

// ─── Public types ──────────────────────────────────────────────────────────────

/// A looping frame animation for a single tile value.
///
/// `frames` are **atlas tile IDs** (0-based, as [`TilemapAtlas::uv_for`](super::TilemapAtlas::uv_for) accepts).
/// Each frame is shown for `frame_time` seconds, then the sequence loops.
#[derive(Debug, Clone)]
pub struct TileAnimation {
    /// Ordered list of atlas tile IDs to display, looping.
    pub frames: Vec<u32>,
    /// Duration each frame is shown (seconds).
    pub frame_time: f32,
}

impl TileAnimation {
    /// Creates a new `TileAnimation` from a list of atlas tile IDs and a per-frame duration.
    ///
    /// `frames` must not be empty; if it is, the animation will always show the first frame
    /// of the atlas.  A `frame_time` ≤ 0 is clamped to a small positive epsilon at runtime.
    pub fn new(frames: Vec<u32>, frame_time: f32) -> Self {
        Self { frames, frame_time }
    }

    /// Total duration of one full cycle (seconds).
    pub fn total_time(&self) -> f32 {
        self.frame_time * self.frames.len() as f32
    }

    /// Returns the frame index (into `self.frames`) at the given elapsed time, wrapping.
    ///
    /// # Examples
    /// ```rust
    /// # use engine::TileAnimation;
    /// // Use 0.25 s/frame (exact in f32) to avoid floating-point rounding in the example.
    /// let anim = TileAnimation::new(vec![0, 1, 2, 3], 0.25);
    /// assert_eq!(anim.frame_at(0.0), 0);
    /// assert_eq!(anim.frame_at(0.25), 1);
    /// assert_eq!(anim.frame_at(0.75), 3);
    /// assert_eq!(anim.frame_at(1.0), 0); // loops (total = 1.0 s)
    /// ```
    pub fn frame_at(&self, elapsed: f32) -> usize {
        if self.frames.is_empty() {
            return 0;
        }
        let ft = self.frame_time.max(f32::EPSILON);
        let total = ft * self.frames.len() as f32;
        let t = elapsed % total;
        ((t / ft) as usize).min(self.frames.len() - 1)
    }
}

/// Maps tile **values** (the `1+` values stored in `Tilemap::tiles`) to a [`TileAnimation`].
///
/// Attach this component to the **same entity** as a [`super::Tilemap`].  Any tile value that
/// appears in the map is animated; values absent from this set remain static.
///
/// # Example
/// ```rust,no_run
/// # use engine::{TileAnimation, TileAnimationSet};
/// let mut set = TileAnimationSet::new();
/// // Value 1 cycles atlas IDs 0-3 at 200 ms each
/// set.insert(1, TileAnimation::new(vec![0, 1, 2, 3], 0.2));
/// ```
/// Default per-cell animation **phase stagger** factor (see [`TileAnimationSet::stagger`]).
///
/// Was a hardcoded literal in [`super::system::TilemapSystem`]; `0.37` keeps the historical
/// look (neighbouring animated cells start slightly out of phase so they don't sync-flash).
pub const DEFAULT_TILE_ANIM_STAGGER: f32 = 0.37;

#[derive(Debug, Clone)]
pub struct TileAnimationSet {
    anims: HashMap<u32, TileAnimation>,
    /// Per-cell phase-stagger factor: each animated cell starts at phase
    /// `(row + col) × frame_time × stagger`, so a higher value spreads neighbouring cells
    /// further apart in their cycle (a rippling, out-of-sync look). `0.0` makes **every**
    /// cell of a tile value animate in lockstep (a synchronized pulse). Default
    /// [`DEFAULT_TILE_ANIM_STAGGER`] (`0.37`) reproduces the previous hardcoded behaviour.
    pub stagger: f32,
}

impl Default for TileAnimationSet {
    fn default() -> Self {
        Self {
            anims: HashMap::new(),
            stagger: DEFAULT_TILE_ANIM_STAGGER,
        }
    }
}

/// Per-cell animation start phase (seconds): `(row + col) × frame_time × stagger`, wrapped
/// into `[0, total_time)`. `stagger = 0` → every cell starts at phase 0 (lockstep). Extracted
/// from `TilemapSystem` so the stagger formula has one home and is unit-testable.
pub(crate) fn stagger_phase(
    row: usize,
    col: usize,
    frame_time: f32,
    total_time: f32,
    stagger: f32,
) -> f32 {
    ((row + col) as f32 * frame_time * stagger) % total_time.max(f32::EPSILON)
}

impl TileAnimationSet {
    /// Creates an empty `TileAnimationSet` with the default [`stagger`](Self::stagger).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the per-cell phase [`stagger`](Self::stagger) factor. Builder form: `0.0`
    /// synchronizes every cell of a value, larger values spread them further out of phase.
    pub fn with_stagger(mut self, stagger: f32) -> Self {
        self.stagger = stagger;
        self
    }

    /// Registers a [`TileAnimation`] for the given tile value.
    ///
    /// `value` is the raw value stored in `Tilemap::tiles` (i.e. `1+`; the still-frame
    /// atlas ID is `value - 1`). Calling `insert` with the same value replaces the
    /// existing animation.
    pub fn insert(&mut self, value: u32, anim: TileAnimation) {
        self.anims.insert(value, anim);
    }

    /// Returns the animation for the given tile value, if registered.
    pub fn get(&self, value: u32) -> Option<&TileAnimation> {
        self.anims.get(&value)
    }

    /// Returns `true` if `value` has a registered animation.
    pub fn contains(&self, value: u32) -> bool {
        self.anims.contains_key(&value)
    }

    /// Removes the animation for `value`, if present.
    pub fn remove(&mut self, value: u32) -> Option<TileAnimation> {
        self.anims.remove(&value)
    }

    /// Iterator over all registered `(tile_value, animation)` pairs.
    pub fn iter(&self) -> impl Iterator<Item = (u32, &TileAnimation)> {
        self.anims.iter().map(|(&v, a)| (v, a))
    }
}

/// Marker component attached to a tile entity by [`super::system::TilemapSystem`] when its tile value
/// has a registered [`TileAnimation`].
///
/// Carries pre-computed [`UvRect`]s for every frame (one per atlas tile ID in the
/// animation), so [`AnimatedTileSystem`] can advance the clock without touching the atlas.
///
/// Removed from the tile entity when the underlying tile value changes to a non-animated
/// value; added when a static cell changes to an animated value.
#[derive(Debug, Clone)]
pub struct AnimatedTileCell {
    /// Pre-computed UV for each frame (indexed by frame position in the animation).
    pub frame_uvs: Vec<UvRect>,
    /// Duration each frame is shown (seconds).
    pub frame_time: f32,
    /// Phase offset (seconds) so neighbouring animated tiles don't all start on frame 0.
    pub phase: f32,
    /// Accumulated elapsed time (seconds), updated every frame by [`AnimatedTileSystem`].
    pub elapsed: f32,
}

impl AnimatedTileCell {
    /// Creates a new `AnimatedTileCell` with the given pre-computed UVs, per-frame duration,
    /// and phase offset (seconds).
    pub fn new(frame_uvs: Vec<UvRect>, frame_time: f32, phase: f32) -> Self {
        Self {
            frame_uvs,
            frame_time,
            phase,
            elapsed: phase,
        }
    }

    /// Returns the current frame index based on the accumulated elapsed time.
    pub fn current_frame(&self) -> usize {
        if self.frame_uvs.is_empty() {
            return 0;
        }
        let ft = self.frame_time.max(f32::EPSILON);
        let total = ft * self.frame_uvs.len() as f32;
        let t = self.elapsed % total;
        ((t / ft) as usize).min(self.frame_uvs.len() - 1)
    }
}

// ─── AnimatedTileSystem ───────────────────────────────────────────────────────

/// System that advances per-tile animation clocks and writes the current frame's
/// [`UvRect`] to each animated tile entity.
///
/// Add this system **after** [`TilemapSystem`](super::system::TilemapSystem) in your
/// system list so the tile entities exist before the first animation tick.
///
/// This system does **not** touch `Tilemap::generation`; the generation-cache
/// fast-path for non-animated tilemaps is fully preserved.
///
/// # Example
/// ```rust,no_run
/// use engine::{App, AnimatedTileSystem};
/// let mut app = App::new();
/// // ... (add TilemapSystem, set up Tilemap + TileAnimationSet) ...
/// app.add_system(AnimatedTileSystem::new());
/// ```
pub struct AnimatedTileSystem;

impl AnimatedTileSystem {
    /// Creates a new `AnimatedTileSystem`.
    pub fn new() -> Self {
        Self
    }
}

impl Default for AnimatedTileSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl System for AnimatedTileSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Collect entities that need updating to avoid borrow conflicts.
        let animated: Vec<crate::ecs::Entity> =
            world.query::<AnimatedTileCell>().map(|(e, _)| e).collect();

        for entity in animated {
            // Advance the clock.
            if let Some(cell) = world.get_mut::<AnimatedTileCell>(entity) {
                cell.elapsed += dt;
                let frame = cell.current_frame();
                let uv = cell.frame_uvs.get(frame).copied();
                if let (Some(uv_rect), Some(uv_comp)) = (uv, world.get_mut::<UvRect>(entity)) {
                    *uv_comp = uv_rect;
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "engine::animated_tile"
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── TileAnimation::frame_at ───────────────────────────────────────────────

    #[test]
    fn frame_at_selects_correct_frame() {
        let anim = TileAnimation::new(vec![10, 20, 30], 0.2);
        assert_eq!(anim.frame_at(0.0), 0, "t=0 → frame 0");
        assert_eq!(anim.frame_at(0.19), 0, "t<0.2 → frame 0");
        assert_eq!(anim.frame_at(0.2), 1, "t=0.2 → frame 1");
        assert_eq!(anim.frame_at(0.39), 1, "t<0.4 → frame 1");
        assert_eq!(anim.frame_at(0.4), 2, "t=0.4 → frame 2");
        assert_eq!(anim.frame_at(0.59), 2, "t<0.6 → frame 2");
    }

    #[test]
    fn frame_at_wraps_around() {
        // Use frame_time = 0.25 s (exact in f32) with 4 frames → total = 1.0 s.
        let anim = TileAnimation::new(vec![0, 1, 2, 3], 0.25);
        assert_eq!(anim.frame_at(1.0), 0, "t=total → wraps to frame 0");
        assert_eq!(anim.frame_at(1.25), 1, "t=total+0.25 → frame 1");
        assert_eq!(anim.frame_at(2.0), 0, "t=2*total → frame 0 again");
        assert_eq!(anim.frame_at(1.5), 2, "t=total+0.5 → frame 2");
    }

    #[test]
    fn frame_at_single_frame_always_zero() {
        let anim = TileAnimation::new(vec![5], 0.5);
        for t in [0.0f32, 0.5, 1.0, 10.0] {
            assert_eq!(anim.frame_at(t), 0, "single-frame anim always returns 0");
        }
    }

    #[test]
    fn frame_at_empty_frames_returns_zero() {
        let anim = TileAnimation::new(vec![], 0.2);
        assert_eq!(anim.frame_at(0.5), 0, "empty frames → 0 (safe fallback)");
    }

    #[test]
    fn frame_at_zero_frame_time_does_not_panic() {
        let anim = TileAnimation::new(vec![0, 1, 2], 0.0);
        // Should not panic; clamps ft to EPSILON.
        let _ = anim.frame_at(0.5);
    }

    // ── TileAnimation::total_time ─────────────────────────────────────────────

    #[test]
    fn total_time_is_frame_time_times_frame_count() {
        let anim = TileAnimation::new(vec![0, 1, 2, 3], 0.25);
        assert!((anim.total_time() - 1.0).abs() < 1e-5);
    }

    // ── TileAnimationSet ──────────────────────────────────────────────────────

    #[test]
    fn animation_set_insert_and_lookup() {
        let mut set = TileAnimationSet::new();
        set.insert(1, TileAnimation::new(vec![0, 1, 2], 0.1));
        assert!(set.contains(1), "value 1 should be registered");
        assert!(!set.contains(2), "value 2 should NOT be registered");

        let anim = set.get(1).expect("should find animation for value 1");
        assert_eq!(anim.frames, vec![0, 1, 2]);
        assert!((anim.frame_time - 0.1).abs() < 1e-6);
    }

    #[test]
    fn animation_set_remove() {
        let mut set = TileAnimationSet::new();
        set.insert(3, TileAnimation::new(vec![0, 1], 0.3));
        assert!(set.remove(3).is_some());
        assert!(!set.contains(3));
        assert!(set.remove(3).is_none(), "double-remove returns None");
    }

    #[test]
    fn animation_set_overwrite() {
        let mut set = TileAnimationSet::new();
        set.insert(5, TileAnimation::new(vec![0], 0.1));
        set.insert(5, TileAnimation::new(vec![0, 1, 2], 0.5));
        let anim = set.get(5).unwrap();
        assert_eq!(anim.frames.len(), 3, "second insert should overwrite");
    }

    #[test]
    fn stagger_defaults_to_historical_value_and_builder_overrides() {
        assert!((TileAnimationSet::new().stagger - DEFAULT_TILE_ANIM_STAGGER).abs() < 1e-6);
        assert!((DEFAULT_TILE_ANIM_STAGGER - 0.37).abs() < 1e-6);
        let synced = TileAnimationSet::new().with_stagger(0.0);
        assert_eq!(synced.stagger, 0.0);
    }

    #[test]
    fn stagger_phase_synchronizes_at_zero_and_spreads_otherwise() {
        let (frame_time, total_time) = (0.2_f32, 0.8_f32);

        // stagger 0 → every cell starts at phase 0 (lockstep).
        for (r, c) in [(0, 0), (1, 0), (3, 5), (9, 9)] {
            assert_eq!(stagger_phase(r, c, frame_time, total_time, 0.0), 0.0);
        }

        // stagger 0.37 → neighbouring cells differ; (0,0) is still 0.
        assert_eq!(stagger_phase(0, 0, frame_time, total_time, 0.37), 0.0);
        let p10 = stagger_phase(1, 0, frame_time, total_time, 0.37);
        assert!(p10 > 0.0, "a staggered neighbour must shift off phase 0");
        // (1,0) and (0,1) share row+col so land on the same phase.
        assert!((p10 - stagger_phase(0, 1, frame_time, total_time, 0.37)).abs() < 1e-6);

        // The phase always wraps into [0, total_time) even for far cells.
        let far = stagger_phase(100, 100, frame_time, total_time, 0.37);
        assert!((0.0..total_time).contains(&far), "phase must wrap: {far}");
    }

    // ── AnimatedTileCell::current_frame ──────────────────────────────────────

    #[test]
    fn animated_tile_cell_current_frame_advances() {
        let uvs = vec![
            UvRect::new(0.0, 0.0, 0.25, 1.0),
            UvRect::new(0.25, 0.0, 0.25, 1.0),
            UvRect::new(0.5, 0.0, 0.25, 1.0),
        ];
        let mut cell = AnimatedTileCell::new(uvs, 0.2, 0.0);
        assert_eq!(cell.current_frame(), 0);
        cell.elapsed = 0.2;
        assert_eq!(cell.current_frame(), 1);
        cell.elapsed = 0.4;
        assert_eq!(cell.current_frame(), 2);
        cell.elapsed = 0.6;
        assert_eq!(cell.current_frame(), 0, "should wrap to frame 0");
    }

    #[test]
    fn animated_tile_cell_phase_offset_shifts_start_frame() {
        let uvs = vec![
            UvRect::new(0.0, 0.0, 0.5, 1.0),
            UvRect::new(0.5, 0.0, 0.5, 1.0),
        ];
        // Phase = 0.15 s (past the 0.1 boundary → starts on frame 1)
        let cell = AnimatedTileCell::new(uvs, 0.1, 0.15);
        assert_eq!(cell.current_frame(), 1, "phase should shift starting frame");
    }
}
