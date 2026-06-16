//! Imperative coroutine sequencer for scripted gameplay.
//!
//! A [`Coroutine`] is an ordered queue of [`CoroutineStep`]s that run against the [`World`]
//! over time. This is distinct from [`crate::timeline`] (data-driven keyframes on entity
//! Transform/Sprite) and pure tween interpolation: a coroutine runs **arbitrary closures**
//! against the world and is well suited for scripted sequences such as wave spawning, boss
//! intros, and cutscene logic.
//!
//! # Example
//!
//! ```rust,no_run
//! use engine::{Coroutine, CoroutineRunner, CoroutineSystem, World};
//!
//! # let mut world = World::new();
//! // Build the sequence:
//! let seq = Coroutine::new()
//!     .wait(1.0)
//!     .run(|_world| { /* spawn wave */ })
//!     .run_for(2.0, |_world, t| { /* animate gate; t goes 0→1 */ })
//!     .wait(0.5)
//!     .run(|_world| { /* spawn boss */ });
//!
//! // Start it:
//! world.resource_mut::<CoroutineRunner>().unwrap().start(seq);
//! ```
//!
//! Register [`CoroutineSystem`] with `app.add_system(CoroutineSystem)` to drive coroutines
//! each frame. Insert [`CoroutineRunner`] into the world before use with
//! `app.world.insert_resource(CoroutineRunner::new())`.
//!
//! ## Borrow safety
//!
//! [`CoroutineRunner`] is a [`World`] resource. Inside [`CoroutineSystem::run`] the runner is
//! **temporarily removed** from the world (via [`World::remove_resource`]) so that coroutine
//! closures can freely receive `&mut World` without aliasing the runner. After all active
//! coroutines are ticked the runner is re-inserted. Closures **must not** re-entrantly access
//! [`CoroutineRunner`] while they are executing — the resource is absent during the tick.

use crate::ecs::{System, World};

// ── Internal callback type aliases ────────────────────────────────────────────

type RunCb = Box<dyn FnMut(&mut World) + Send + Sync>;
type RunForCb = Box<dyn FnMut(&mut World, f32) + Send + Sync>;

// ── Step ─────────────────────────────────────────────────────────────────────

/// A single step in a [`Coroutine`].
///
/// Steps are not part of the public API surface — create them through the builder methods on
/// [`Coroutine`]: [`Coroutine::wait`], [`Coroutine::run`], and [`Coroutine::run_for`].
enum CoroutineStep {
    /// Wait for `remaining` seconds before advancing to the next step.
    Wait(f32),
    /// Run a closure exactly once, then immediately advance.
    Run(RunCb),
    /// Call a closure every frame for `dur` seconds.
    ///
    /// `progress` is passed in the range `0.0..=1.0` (inclusive on both ends). When the
    /// step finishes `f` is called one final time with `progress == 1.0` before advancing.
    RunFor {
        /// Total duration in seconds.
        dur: f32,
        /// Time elapsed so far.
        elapsed: f32,
        /// The per-frame callback; receives `(world, progress)`.
        f: RunForCb,
    },
}

// ── Coroutine ─────────────────────────────────────────────────────────────────

/// An ordered sequence of scripted steps (waits, closures, timed callbacks) that executes
/// against the [`World`] over multiple frames.
///
/// Build a coroutine with the fluent API:
/// ```rust,no_run
/// # use engine::Coroutine;
/// let c = Coroutine::new()
///     .wait(2.0)
///     .run(|_world| { /* one-shot action */ })
///     .run_for(1.5, |_world, t| { /* per-frame with progress 0→1 */ });
/// ```
///
/// Then start it via [`CoroutineRunner::start`]. The coroutine is automatically dropped when
/// all steps complete.
pub struct Coroutine {
    steps: std::collections::VecDeque<CoroutineStep>,
}

impl Coroutine {
    /// Creates an empty coroutine (no steps).
    pub fn new() -> Self {
        Self {
            steps: std::collections::VecDeque::new(),
        }
    }

    /// Appends a pause of `secs` seconds before the next step begins.
    pub fn wait(mut self, secs: f32) -> Self {
        self.steps.push_back(CoroutineStep::Wait(secs.max(0.0)));
        self
    }

    /// Appends a one-shot closure that is called exactly once, then advances.
    pub fn run<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut World) + Send + Sync + 'static,
    {
        self.steps.push_back(CoroutineStep::Run(Box::new(f)));
        self
    }

    /// Appends a timed callback that is called every frame for `dur` seconds.
    ///
    /// `f(world, progress)` receives a `progress` value in `0.0..=1.0`.
    /// The callback is invoked one final time with `progress == 1.0` when the step ends.
    pub fn run_for<F>(mut self, dur: f32, f: F) -> Self
    where
        F: FnMut(&mut World, f32) + Send + Sync + 'static,
    {
        self.steps.push_back(CoroutineStep::RunFor {
            dur: dur.max(0.0),
            elapsed: 0.0,
            f: Box::new(f),
        });
        self
    }

    /// Returns `true` when the coroutine has no remaining steps.
    fn is_done(&self) -> bool {
        self.steps.is_empty()
    }

    /// Advances the coroutine by `dt` seconds.
    ///
    /// Any leftover time after a step completes is carried into the next step within the
    /// same call, so a large `dt` can advance through multiple steps in one tick.
    fn tick(&mut self, world: &mut World, mut dt: f32) {
        while dt > 0.0 && !self.steps.is_empty() {
            match self.steps.front_mut().expect("checked non-empty") {
                CoroutineStep::Wait(remaining) => {
                    if dt >= *remaining {
                        dt -= *remaining;
                        self.steps.pop_front();
                    } else {
                        *remaining -= dt;
                        dt = 0.0;
                    }
                }
                CoroutineStep::Run(_) => {
                    // Remove the step, call the closure, carry remaining dt.
                    if let Some(CoroutineStep::Run(mut f)) = self.steps.pop_front() {
                        f(world);
                    }
                    // dt is consumed by the Run step (instant), carried forward as-is.
                }
                CoroutineStep::RunFor { dur, elapsed, .. } => {
                    let step_dur = *dur;
                    let new_elapsed = *elapsed + dt;
                    if new_elapsed >= step_dur {
                        let leftover = new_elapsed - step_dur;
                        // Call at progress = 1.0 to signal completion, then advance.
                        if let Some(CoroutineStep::RunFor { mut f, .. }) = self.steps.pop_front() {
                            f(world, 1.0);
                        }
                        dt = leftover;
                    } else {
                        *elapsed = new_elapsed;
                        let progress = if step_dur > 0.0 {
                            new_elapsed / step_dur
                        } else {
                            1.0
                        };
                        if let Some(CoroutineStep::RunFor { f, .. }) = self.steps.front_mut() {
                            f(world, progress);
                        }
                        dt = 0.0;
                    }
                }
            }
        }
    }
}

impl Default for Coroutine {
    fn default() -> Self {
        Self::new()
    }
}

// ── Runner ────────────────────────────────────────────────────────────────────

/// World resource that holds all active [`Coroutine`]s and drives them each frame.
///
/// Insert into the world before use:
/// ```rust,no_run
/// # use engine::{CoroutineRunner, World};
/// # let mut world = World::new();
/// world.insert_resource(CoroutineRunner::new());
/// ```
///
/// Then register [`CoroutineSystem`] to tick it each frame.
pub struct CoroutineRunner {
    active: Vec<Coroutine>,
}

impl CoroutineRunner {
    /// Creates an empty runner (no active coroutines).
    pub fn new() -> Self {
        Self { active: Vec::new() }
    }

    /// Enqueues a coroutine to start running immediately.
    pub fn start(&mut self, coroutine: Coroutine) {
        self.active.push(coroutine);
    }

    /// Returns the number of currently active (unfinished) coroutines.
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

impl Default for CoroutineRunner {
    fn default() -> Self {
        Self::new()
    }
}

// ── System ────────────────────────────────────────────────────────────────────

/// Advances all active [`Coroutine`]s in [`CoroutineRunner`] by `dt` each frame.
///
/// Add it with `app.add_system(CoroutineSystem)`.
///
/// ## Borrow safety
///
/// [`CoroutineRunner`] is **removed** from the world at the start of each tick and
/// **re-inserted** after all coroutines have been advanced. This means closures inside
/// coroutines receive unrestricted `&mut World` access without aliasing the runner.
/// Do **not** access `CoroutineRunner` from within a coroutine closure — it will be absent.
#[derive(Default)]
pub struct CoroutineSystem;

impl System for CoroutineSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Remove the runner so closures may freely access `world`.
        let Some(mut runner) = world.remove_resource::<CoroutineRunner>() else {
            return;
        };

        // Tick every coroutine, carrying leftover dt across steps.
        for coroutine in &mut runner.active {
            coroutine.tick(world, dt);
        }

        // Drop finished coroutines.
        runner.active.retain(|c| !c.is_done());

        // Re-insert runner.
        world.insert_resource(runner);
    }

    fn name(&self) -> &'static str {
        "CoroutineSystem"
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Helper: shared mutable log collected by closures.
    fn log() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    // ── Wait ──────────────────────────────────────────────────────────────────

    #[test]
    fn wait_countdown_not_done_before_time() {
        let mut world = World::new();
        let mut c = Coroutine::new().wait(1.0);
        c.tick(&mut world, 0.5);
        assert!(!c.is_done(), "should still have remaining wait time");
    }

    #[test]
    fn wait_countdown_done_after_full_time() {
        let mut world = World::new();
        let mut c = Coroutine::new().wait(1.0);
        c.tick(&mut world, 1.0);
        assert!(c.is_done());
    }

    #[test]
    fn wait_done_when_overshot() {
        let mut world = World::new();
        let mut c = Coroutine::new().wait(0.3);
        c.tick(&mut world, 1.0);
        assert!(c.is_done(), "a large dt should finish the wait");
    }

    // ── Run (one-shot) ────────────────────────────────────────────────────────

    #[test]
    fn run_step_executes_once_then_done() {
        let mut world = World::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter2 = Arc::clone(&counter);
        let mut c = Coroutine::new().run(move |_w| {
            *counter2.lock().unwrap() += 1;
        });
        c.tick(&mut world, 0.016);
        assert_eq!(
            *counter.lock().unwrap(),
            1,
            "closure should run exactly once"
        );
        assert!(c.is_done());
    }

    // ── Run ordering ──────────────────────────────────────────────────────────

    #[test]
    fn run_steps_execute_in_order() {
        let mut world = World::new();
        let events = log();
        let ev1 = Arc::clone(&events);
        let ev2 = Arc::clone(&events);
        let ev3 = Arc::clone(&events);
        let mut c = Coroutine::new()
            .run(move |_w| ev1.lock().unwrap().push("a".to_string()))
            .run(move |_w| ev2.lock().unwrap().push("b".to_string()))
            .run(move |_w| ev3.lock().unwrap().push("c".to_string()));
        c.tick(&mut world, 0.016);
        let got = events.lock().unwrap().clone();
        assert_eq!(got, vec!["a", "b", "c"]);
    }

    // ── RunFor progress ───────────────────────────────────────────────────────

    #[test]
    fn run_for_first_call_not_at_one() {
        let mut world = World::new();
        let calls = Arc::new(Mutex::new(Vec::<f32>::new()));
        let calls2 = Arc::clone(&calls);
        let mut c = Coroutine::new().run_for(1.0, move |_w, t| {
            calls2.lock().unwrap().push(t);
        });
        c.tick(&mut world, 0.5); // half-way
        let got = calls.lock().unwrap().clone();
        assert_eq!(got.len(), 1);
        assert!(
            (got[0] - 0.5).abs() < 1e-5,
            "expected progress 0.5 got {}",
            got[0]
        );
        assert!(!c.is_done(), "still half remaining");
    }

    #[test]
    fn run_for_final_call_at_one() {
        let mut world = World::new();
        let last = Arc::new(Mutex::new(0.0f32));
        let last2 = Arc::clone(&last);
        let mut c = Coroutine::new().run_for(1.0, move |_w, t| {
            *last2.lock().unwrap() = t;
        });
        c.tick(&mut world, 0.5);
        c.tick(&mut world, 0.6); // overshoots
        assert!(
            (*last.lock().unwrap() - 1.0).abs() < 1e-5,
            "final call must be progress 1.0"
        );
        assert!(c.is_done());
    }

    // ── Multi-step carryover ──────────────────────────────────────────────────

    #[test]
    fn carryover_dt_spans_multiple_steps_in_one_tick() {
        let mut world = World::new();
        let events = log();
        let ev1 = Arc::clone(&events);
        let ev2 = Arc::clone(&events);
        // wait 0.1s, then run two closures — all in a single tick of 5s.
        let mut c = Coroutine::new()
            .wait(0.1)
            .run(move |_w| ev1.lock().unwrap().push("step1".to_string()))
            .run(move |_w| ev2.lock().unwrap().push("step2".to_string()));
        c.tick(&mut world, 5.0);
        let got = events.lock().unwrap().clone();
        assert_eq!(got, vec!["step1", "step2"], "all steps should complete");
        assert!(c.is_done());
    }

    // ── Completion / drop (via CoroutineRunner) ───────────────────────────────

    #[test]
    fn runner_drops_finished_coroutines() {
        let mut world = World::new();
        world.insert_resource(CoroutineRunner::new());
        let counter = Arc::new(Mutex::new(0u32));
        let counter2 = Arc::clone(&counter);
        if let Some(runner) = world.resource_mut::<CoroutineRunner>() {
            runner.start(Coroutine::new().run(move |_w| {
                *counter2.lock().unwrap() += 1;
            }));
        }
        let mut sys = CoroutineSystem;
        sys.run(&mut world, 0.016);
        assert_eq!(*counter.lock().unwrap(), 1);
        assert_eq!(
            world.resource::<CoroutineRunner>().unwrap().active_count(),
            0,
            "finished coroutine should be dropped from the runner"
        );
    }

    #[test]
    fn runner_reinserts_resource_after_tick() {
        let mut world = World::new();
        world.insert_resource(CoroutineRunner::new());
        let mut sys = CoroutineSystem;
        sys.run(&mut world, 0.016);
        assert!(
            world.resource::<CoroutineRunner>().is_some(),
            "runner must be re-inserted after tick"
        );
    }

    // ── Zero-duration RunFor ──────────────────────────────────────────────────

    #[test]
    fn run_for_zero_duration_calls_at_one() {
        let mut world = World::new();
        let last = Arc::new(Mutex::new(-1.0f32));
        let last2 = Arc::clone(&last);
        let mut c = Coroutine::new().run_for(0.0, move |_w, t| {
            *last2.lock().unwrap() = t;
        });
        c.tick(&mut world, 0.016);
        assert!(
            (*last.lock().unwrap() - 1.0).abs() < 1e-5,
            "zero-dur run_for must fire at progress 1.0"
        );
        assert!(c.is_done());
    }
}
