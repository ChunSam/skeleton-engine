//! Styled scene transitions — fade / wipe / iris, with automatic scene swapping.
//!
//! [`SceneTransition`] is the styled successor to [`FadeTransition`](crate::FadeTransition): the
//! same full-screen overlay idea, but the coverage can be a uniform fade, a directional wipe, or a
//! circular iris. Its `coverage` runs 0 (nothing covered) → 1 (fully covered) during the
//! [`Out`](TransitionPhase::Out) half, then back to 0 during [`In`](TransitionPhase::In) — so the
//! screen covers, the scene swaps *while hidden*, then the new scene is revealed.
//!
//! For the common case — "transition to a new scene" — call
//! [`App::transition_to_scene`](crate::App::transition_to_scene): it inserts the resource, and
//! `App` automatically swaps the scene at the fully-covered midpoint and drops the transition when
//! it finishes. Insert a bare `SceneTransition` yourself for a self-contained cover/reveal with no
//! swap (an "eyes closing" beat).
//!
//! **Platform note:** the render pass is native-only (like `FadeTransition`). On `wasm32` the state
//! still advances — so [`is_done`](SceneTransition::is_done) fires and any auto-swap happens — but
//! no overlay is drawn.
//!
//! ```
//! use engine::{SceneTransition, TransitionStyle, TransitionPhase};
//!
//! // A 0.3 s iris-in cover, then 0.3 s reveal.
//! let mut t = SceneTransition::new(TransitionStyle::IrisIn, 0.3);
//! assert_eq!(t.phase, TransitionPhase::Out);
//! t.update(0.3); // finishes the Out half
//! assert!(t.just_covered());               // the swap-the-scene moment
//! assert_eq!(t.phase, TransitionPhase::In);
//! t.update(0.3); // finishes the In half
//! assert!(t.is_done());
//! ```

use crate::color::Color;
use crate::ecs::World;
use crate::scene::{Scene, SceneCmd};

/// The visual style of a [`SceneTransition`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransitionStyle {
    /// Uniform full-screen colour fade (coverage = overlay alpha).
    Fade,
    /// A colour edge sweeps left→right.
    WipeLeft,
    /// A colour edge sweeps right→left.
    WipeRight,
    /// A colour edge sweeps top→bottom.
    WipeDown,
    /// A colour edge sweeps bottom→top.
    WipeUp,
    /// The scene closes into a shrinking circular window at the centre.
    IrisIn,
    /// A growing circle from the centre swallows the scene.
    IrisOut,
}

impl TransitionStyle {
    /// The shader's style index (kept in sync with the transition shader). Only the native render
    /// path consumes it, so it is gated out of the wasm build (where no overlay is drawn).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn shader_index(self) -> f32 {
        match self {
            TransitionStyle::Fade => 0.0,
            TransitionStyle::WipeLeft => 1.0,
            TransitionStyle::WipeRight => 2.0,
            TransitionStyle::WipeDown => 3.0,
            TransitionStyle::WipeUp => 4.0,
            TransitionStyle::IrisIn => 5.0,
            TransitionStyle::IrisOut => 6.0,
        }
    }
}

/// Which half of a [`SceneTransition`] is playing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransitionPhase {
    /// Covering the old scene (coverage 0 → 1).
    Out,
    /// Revealing the new scene (coverage 1 → 0).
    In,
    /// Finished (coverage 0); the resource can be removed.
    Done,
}

/// A full-screen styled scene transition. See the [module docs](self).
#[derive(Clone, Copy, Debug)]
pub struct SceneTransition {
    /// The visual style.
    pub style: TransitionStyle,
    /// The overlay colour.
    pub color: Color,
    /// How much of the screen is covered: `0.0` = nothing, `1.0` = fully covered.
    pub coverage: f32,
    /// Which half is playing.
    pub phase: TransitionPhase,
    /// Coverage change per second — each half (cover, reveal) takes `1.0 / speed` seconds.
    pub speed: f32,
    /// Set for the single frame the `Out` half completes — the moment to swap scenes.
    covered_edge: bool,
}

impl SceneTransition {
    /// A transition of `style` whose cover half (and, symmetrically, reveal half) each take
    /// `half_duration` seconds. Starts in [`Out`](TransitionPhase::Out) at zero coverage, black.
    pub fn new(style: TransitionStyle, half_duration: f32) -> Self {
        Self {
            style,
            color: Color::BLACK,
            coverage: 0.0,
            phase: TransitionPhase::Out,
            speed: 1.0 / half_duration.max(0.001),
            covered_edge: false,
        }
    }

    /// Sets the overlay colour (builder).
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Advances the transition by `dt` seconds (called automatically by `App`). Covers during
    /// [`Out`](TransitionPhase::Out), flips to [`In`](TransitionPhase::In) at full coverage (and
    /// flags [`just_covered`](Self::just_covered) that frame), then reveals and reaches
    /// [`Done`](TransitionPhase::Done).
    pub fn update(&mut self, dt: f32) {
        self.covered_edge = false;
        let step = self.speed * dt.max(0.0);
        match self.phase {
            TransitionPhase::Out => {
                self.coverage += step;
                if self.coverage >= 1.0 {
                    self.coverage = 1.0;
                    self.phase = TransitionPhase::In;
                    self.covered_edge = true;
                }
            }
            TransitionPhase::In => {
                self.coverage -= step;
                if self.coverage <= 0.0 {
                    self.coverage = 0.0;
                    self.phase = TransitionPhase::Done;
                }
            }
            TransitionPhase::Done => {}
        }
    }

    /// `true` only on the frame the cover half completed — the moment `App` swaps the scene.
    pub fn just_covered(&self) -> bool {
        self.covered_edge
    }

    /// Whether the transition has fully finished (revealed).
    pub fn is_done(&self) -> bool {
        self.phase == TransitionPhase::Done
    }

    /// Whether the point `(x, y)` in normalized screen space (`0..1`, origin top-left) is covered
    /// at the current `coverage`, for the given `aspect` (width / height). The GPU shader mirrors
    /// this (with a soft edge); exposed for logic/tests that need coverage without a GPU.
    ///
    /// For [`Fade`](TransitionStyle::Fade) — which has no spatial edge — this reports the whole
    /// screen covered once `coverage >= 0.5`.
    pub fn covered_at(&self, x: f32, y: f32, aspect: f32) -> bool {
        let c = self.coverage;
        match self.style {
            TransitionStyle::Fade => c >= 0.5,
            TransitionStyle::WipeLeft => x <= c,
            TransitionStyle::WipeRight => x >= 1.0 - c,
            TransitionStyle::WipeDown => y <= c,
            TransitionStyle::WipeUp => y >= 1.0 - c,
            TransitionStyle::IrisIn => normalized_radius(x, y, aspect) >= 1.0 - c,
            TransitionStyle::IrisOut => normalized_radius(x, y, aspect) <= c,
        }
    }
}

/// Distance from the screen centre to `(x, y)`, normalized so the farthest corner is `1.0`
/// (aspect-corrected so an iris reads as a circle, not an ellipse).
fn normalized_radius(x: f32, y: f32, aspect: f32) -> f32 {
    let dx = (x - 0.5) * aspect;
    let dy = y - 0.5;
    let max_r = ((0.5 * aspect).powi(2) + 0.25).sqrt();
    (dx * dx + dy * dy).sqrt() / max_r.max(1e-6)
}

/// Internal resource carrying the scene to swap to at the covered midpoint of a
/// [`SceneTransition`]. Set by [`start_scene_transition`] / [`App::transition_to_scene`], consumed
/// by `App` when the cover half completes. Not persistent — it is taken before the scene reset.
pub(crate) struct PendingSceneTransition(pub(crate) Option<SceneCmd>);

/// Begins an animated scene change from anywhere with a `&mut World` — a **system** or setup code.
///
/// Inserts a [`SceneTransition`] (the visual cover→reveal) plus the pending swap; `App` covers the
/// screen over `half_duration` seconds in `style`, swaps to `scene` while fully hidden, then reveals
/// it over another `half_duration`. This is the world-level twin of
/// [`App::transition_to_scene`](crate::App::transition_to_scene) (which a system can't call). The
/// overlay is native-only, but the swap happens on every platform.
///
/// ```no_run
/// # use engine::{start_scene_transition, TransitionStyle, ecs::World, scene::{Scene, SystemRegistrar}};
/// # struct NextLevel;
/// # impl Scene for NextLevel { fn on_enter(&mut self, _: &mut World, _: &mut SystemRegistrar) {} fn on_exit(&mut self, _: &mut World) {} }
/// # fn in_a_system(world: &mut World) {
/// start_scene_transition(world, Box::new(NextLevel), TransitionStyle::IrisOut, 0.4);
/// # }
/// ```
pub fn start_scene_transition(
    world: &mut World,
    scene: Box<dyn Scene>,
    style: TransitionStyle,
    half_duration: f32,
) {
    world.insert_resource(SceneTransition::new(style, half_duration));
    world.insert_resource(PendingSceneTransition(Some(SceneCmd::Replace(scene))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phases_progress_out_then_in_then_done() {
        let mut t = SceneTransition::new(TransitionStyle::Fade, 0.5); // speed = 2/s
        assert_eq!(t.phase, TransitionPhase::Out);
        assert!(!t.just_covered());

        t.update(0.25); // coverage 0.5, still Out
        assert!((t.coverage - 0.5).abs() < 1e-5);
        assert_eq!(t.phase, TransitionPhase::Out);

        t.update(0.5); // overshoots the cover → clamps to 1, flips to In
        assert_eq!(t.coverage, 1.0);
        assert_eq!(t.phase, TransitionPhase::In);
        assert!(t.just_covered(), "the swap moment fires once");

        t.update(0.001); // next frame no longer flags just_covered
        assert!(!t.just_covered());

        t.update(1.0); // reveals fully
        assert_eq!(t.coverage, 0.0);
        assert!(t.is_done());
    }

    #[test]
    fn done_transition_is_idempotent() {
        let mut t = SceneTransition::new(TransitionStyle::Fade, 0.1);
        t.update(10.0); // covers → In
        t.update(10.0); // reveals → Done
        assert!(t.is_done());
        t.update(10.0); // no panic, stays done
        assert_eq!(t.coverage, 0.0);
        assert!(t.is_done());
    }

    #[test]
    fn coverage_endpoints_cover_nothing_then_everything() {
        // Sample a grid of points; at coverage 0 none are covered, at coverage 1 all are.
        for style in [
            TransitionStyle::WipeLeft,
            TransitionStyle::WipeRight,
            TransitionStyle::WipeUp,
            TransitionStyle::WipeDown,
            TransitionStyle::IrisIn,
            TransitionStyle::IrisOut,
        ] {
            let mut t = SceneTransition::new(style, 1.0);
            t.coverage = 0.0;
            let none = grid_points().all(|(x, y)| !t.covered_at(x, y, 16.0 / 9.0));
            assert!(none, "{style:?} at coverage 0 should cover ~nothing");
            t.coverage = 1.0;
            let all = grid_points().all(|(x, y)| t.covered_at(x, y, 16.0 / 9.0));
            assert!(all, "{style:?} at coverage 1 should cover everything");
        }
    }

    #[test]
    fn wipe_left_covers_the_left_side_at_half() {
        let mut t = SceneTransition::new(TransitionStyle::WipeLeft, 1.0);
        t.coverage = 0.5;
        assert!(t.covered_at(0.25, 0.5, 1.0), "left of the edge is covered");
        assert!(!t.covered_at(0.75, 0.5, 1.0), "right of the edge is clear");
    }

    #[test]
    fn iris_out_covers_the_centre_first() {
        let mut t = SceneTransition::new(TransitionStyle::IrisOut, 1.0);
        t.coverage = 0.3;
        assert!(t.covered_at(0.5, 0.5, 1.0), "the centre is covered early");
        assert!(!t.covered_at(0.02, 0.02, 1.0), "a corner is still clear");
    }

    // Cell-centre samples (0.05, 0.15, … 0.95) — never the exact centre (0.5) or the corners
    // (0/1), the measure-zero boundary points where a coverage of exactly 0 or 1 is ambiguous
    // (and which the shader's soft edge covers anyway).
    fn grid_points() -> impl Iterator<Item = (f32, f32)> {
        (0..10)
            .flat_map(|i| (0..10).map(move |j| ((i as f32 + 0.5) / 10.0, (j as f32 + 0.5) / 10.0)))
    }
}
