use crate::renderer::uv::UvRect;

// ─── Blend-weight component ───────────────────────────────────────────────────

/// Component indicating crossfade progress. Updated every frame by `AnimationSystem`.
///
/// - `1.0`: no crossfade (or completed)
/// - `0.0 ~ 1.0`: transition in progress (0 = from clip, 1 = to clip)
///
/// Game code can use this for sprite-alpha interpolation and similar effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendWeight(pub f32);

// ─── Crossfade state ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub(crate) struct CrossfadeState {
    pub to_clip: usize,
    pub to_frame: usize,
    pub to_timer: f32,
    pub elapsed: f32,
    pub duration: f32,
}

// ─── Animation data ───────────────────────────────────────────────────────────

/// A single animation clip: list of frames and playback speed.
#[derive(Debug, Clone)]
pub struct AnimationClip {
    pub frames: Vec<UvRect>,
    pub fps: f32,
    pub looping: bool,
}

/// Animation player component attached to an entity.
#[derive(Debug, Clone)]
pub struct AnimationPlayer {
    pub clips: Vec<AnimationClip>,
    pub current_clip: usize,
    pub current_frame: usize,
    /// Accumulated time (seconds) until the next frame.
    pub timer: f32,
    pub(crate) crossfade: Option<CrossfadeState>,
    /// Set by `AnimationSystem` when a non-looping clip reaches its last frame and has
    /// been shown for at least one frame duration. Reset by `play` / `play_with_crossfade`.
    /// This avoids `is_finished()` returning `true` at frame 0 (before any time has elapsed),
    /// which would cause state-machine `AnimationEnd` transitions to fire before the clip is seen.
    pub(crate) finished: bool,
}

impl AnimationPlayer {
    pub fn new(clips: Vec<AnimationClip>) -> Self {
        Self {
            clips,
            current_clip: 0,
            current_frame: 0,
            timer: 0.0,
            crossfade: None,
            finished: false,
        }
    }

    /// Switches to a clip immediately. Does nothing if that clip is already playing.
    pub fn play(&mut self, clip_index: usize) {
        if self.current_clip != clip_index {
            self.current_clip = clip_index;
            self.current_frame = 0;
            self.timer = 0.0;
            self.crossfade = None;
            self.finished = false;
        }
    }

    /// Switches to a clip with a smooth crossfade over `duration` seconds.
    ///
    /// The `BlendWeight` component is updated from 0.0→1.0 during the transition.
    /// If `duration <= 0.0` the switch is immediate (same as `play`).
    ///
    /// Calls that re-fire the **same** `clip_index` while a crossfade to that clip is
    /// already in progress are silently ignored (idempotent). This prevents noisy
    /// threshold parameters (e.g. `FloatGt`/`FloatLt`) from resetting `elapsed` to
    /// zero every frame and preventing the blend from ever completing.
    ///
    /// When a crossfade A→B is interrupted by a call targeting a third clip C, the
    /// in-flight TO side (B) is promoted to FROM before starting the new A→C crossfade.
    /// This prevents a visible one-frame pop back to the original FROM clip (A) and
    /// ensures the first blended frame is `mix(B, C, 0.0)` = B — a smooth continuation.
    pub fn play_with_crossfade(&mut self, clip_index: usize, duration: f32) {
        if self.current_clip == clip_index {
            return;
        }
        // Already crossfading to the same target — do not restart.
        if let Some(cf) = &self.crossfade {
            if cf.to_clip == clip_index {
                return;
            }
        }
        if duration <= 0.0 {
            self.play(clip_index);
            return;
        }
        // If a crossfade A→B is in progress and we now want C, promote B to FROM so
        // the transition continues from B rather than snapping back to A.
        if let Some(cf) = self.crossfade.take() {
            self.current_clip = cf.to_clip;
            self.current_frame = cf.to_frame;
            self.timer = cf.to_timer;
        }
        self.finished = false;
        self.crossfade = Some(CrossfadeState {
            to_clip: clip_index,
            to_frame: 0,
            to_timer: 0.0,
            elapsed: 0.0,
            duration,
        });
    }

    /// Returns `true` when the player is actively playing `clip_index` (not crossfading
    /// away from it) and that clip has reached its last frame.
    ///
    /// This is the correct predicate for the `AnimationEnd` transition condition: it
    /// avoids false positives during a crossfade where `current_clip` is still the FROM
    /// clip even though the state machine has already committed to a new state.
    pub fn is_clip_finished(&self, clip_index: usize) -> bool {
        // During a crossfade, current_clip is the FROM clip — don't report it as finished
        // for the destination state that is still blending in.
        if self.crossfade.is_some() {
            return false;
        }
        if self.current_clip != clip_index {
            return false;
        }
        self.is_finished()
    }

    /// Crossfade progress [0.0..=1.0]. Returns `1.0` when no transition is active.
    pub fn blend_weight(&self) -> f32 {
        match &self.crossfade {
            None => 1.0,
            Some(cf) => (cf.elapsed / cf.duration).clamp(0.0, 1.0),
        }
    }

    /// Returns `true` while a crossfade transition is in progress.
    pub fn is_crossfading(&self) -> bool {
        self.crossfade.is_some()
    }

    /// Returns the UV of the current frame. Falls back to the full texture if there are no clips or frames.
    pub fn current_uv(&self) -> UvRect {
        self.clips
            .get(self.current_clip)
            .and_then(|c| c.frames.get(self.current_frame))
            .copied()
            .unwrap_or(UvRect::FULL)
    }

    /// Returns whether the current non-looping clip has finished.
    ///
    /// Returns `false` for looping clips and for clips with no frames. For non-looping
    /// clips, returns `true` only after `AnimationSystem` has advanced the clip to its
    /// last frame **and** held it there for at least one full frame duration — i.e. the
    /// flag is set by the system, not inferred from frame index alone. This prevents a
    /// 1-frame clip from reporting finished at frame 0 before any time has elapsed.
    pub fn is_finished(&self) -> bool {
        let Some(clip) = self.clips.get(self.current_clip) else {
            return true;
        };
        if clip.looping || clip.frames.is_empty() {
            return false;
        }
        self.finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::System;

    fn make_clip(n_frames: usize, fps: f32, looping: bool) -> AnimationClip {
        AnimationClip {
            frames: vec![UvRect::FULL; n_frames],
            fps,
            looping,
        }
    }

    fn make_player() -> AnimationPlayer {
        // Three clips: A (0), B (1), C (2)
        AnimationPlayer::new(vec![
            make_clip(4, 10.0, true),
            make_clip(4, 10.0, true),
            make_clip(4, 10.0, true),
        ])
    }

    // ── Bug #1: third-clip interrupt should promote B to FROM, not revert to A ──

    /// When A→B crossfade is interrupted at weight 0.7 by C, the new FROM must be
    /// B (current_clip == 1) — no pop back to A.
    #[test]
    fn interrupt_promotes_to_clip_to_from() {
        let mut player = make_player();
        // Start A→B crossfade, advance elapsed to 70% of duration
        player.play_with_crossfade(1, 1.0);
        {
            let cf = player.crossfade.as_mut().unwrap();
            cf.elapsed = 0.7;
            cf.to_frame = 2;
            cf.to_timer = 0.05;
        }

        // Interrupt with C
        player.play_with_crossfade(2, 0.5);

        // FROM side must be B (clip index 1), not A (clip index 0)
        assert_eq!(
            player.current_clip, 1,
            "interrupt should promote B to FROM clip (was A before fix)"
        );
        assert_eq!(
            player.current_frame, 2,
            "promoted FROM frame must carry B's to_frame"
        );
        assert!(
            (player.timer - 0.05).abs() < 1e-6,
            "promoted FROM timer must carry B's to_timer"
        );

        // A new crossfade toward C must be in place
        let cf = player
            .crossfade
            .as_ref()
            .expect("crossfade to C must exist");
        assert_eq!(cf.to_clip, 2);
        assert_eq!(cf.elapsed, 0.0, "new crossfade starts fresh");
    }

    /// Idempotent re-fire guard must still hold after the interrupt promotion:
    /// calling play_with_crossfade(C, _) again while already crossfading to C is a no-op.
    #[test]
    fn same_target_guard_survives_after_interrupt() {
        let mut player = make_player();
        // A→B in progress
        player.play_with_crossfade(1, 1.0);
        {
            let cf = player.crossfade.as_mut().unwrap();
            cf.elapsed = 0.3;
        }
        // Interrupt to C (promotes B→FROM, starts B→C)
        player.play_with_crossfade(2, 0.5);
        let elapsed_before = player.crossfade.as_ref().unwrap().elapsed;

        // Re-fire the same target C — must be a no-op
        player.play_with_crossfade(2, 0.5);
        let elapsed_after = player.crossfade.as_ref().unwrap().elapsed;
        assert_eq!(
            elapsed_before, elapsed_after,
            "re-firing the same crossfade target must not reset elapsed"
        );
        assert_eq!(
            player.current_clip, 1,
            "FROM clip must remain B after no-op re-fire"
        );
    }

    /// A 1-frame non-looping clip must not report `is_finished()` at frame 0 (before any dt).
    /// It must become finished only after at least one frame-duration of accumulated dt.
    /// This test drives the system directly rather than going through AnimationSystem.
    #[test]
    fn one_frame_nonlooping_clip_not_finished_at_entry() {
        use crate::animation::system::AnimationSystem;
        use crate::ecs::World;
        let fps = 10.0_f32; // frame_dur = 100 ms
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![AnimationClip {
                frames: vec![UvRect::FULL],
                fps,
                looping: false,
            }]),
        );
        let mut sys = AnimationSystem::new();

        // Before any tick: must NOT be finished.
        assert!(
            !world.get::<AnimationPlayer>(e).unwrap().is_finished(),
            "1-frame clip must not be finished before any time has elapsed"
        );

        // After less than one frame duration: still not finished.
        sys.run(&mut world, 0.05);
        assert!(
            !world.get::<AnimationPlayer>(e).unwrap().is_finished(),
            "1-frame clip must not be finished after 50ms (< 100ms frame_dur)"
        );

        // After one full frame duration: now finished.
        sys.run(&mut world, 0.06); // cumulative 110ms > 100ms
        assert!(
            world.get::<AnimationPlayer>(e).unwrap().is_finished(),
            "1-frame clip must be finished after one full frame duration"
        );
    }

    /// Multi-frame non-looping clip: `is_finished()` becomes true the frame the system
    /// advances the playhead onto the last frame (same semantics as before the fix; the
    /// fix only corrects the 1-frame false-positive at construction time).
    #[test]
    fn multi_frame_nonlooping_clip_finished_after_last_frame_shown() {
        use crate::animation::system::AnimationSystem;
        use crate::ecs::World;
        let fps = 10.0_f32; // frame_dur = 100 ms, 3 frames → last frame reached at 200 ms
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![AnimationClip {
                frames: vec![UvRect::FULL; 3],
                fps,
                looping: false,
            }]),
        );
        let mut sys = AnimationSystem::new();

        // Not finished before the last frame is reached (just over 1 frame in).
        sys.run(&mut world, 0.15); // 150ms: frame 0→1 at 100ms, still on frame 1
        assert!(
            !world.get::<AnimationPlayer>(e).unwrap().is_finished(),
            "3-frame clip must not be finished at 150ms (still on frame 1)"
        );

        // Advance past the 200ms mark so the system steps to frame 2 (last).
        sys.run(&mut world, 0.11); // 260ms total: frame 1→2 (last) → finished
        assert!(
            world.get::<AnimationPlayer>(e).unwrap().is_finished(),
            "3-frame clip must be finished after advancing to the last frame"
        );
    }

    /// Interrupting when no crossfade is active starts a normal A→C crossfade.
    #[test]
    fn interrupt_with_no_active_crossfade_starts_fresh() {
        let mut player = make_player();
        // No crossfade yet — directly play C
        player.play_with_crossfade(2, 0.5);
        assert_eq!(
            player.current_clip, 0,
            "FROM must remain A (no promotion occurred)"
        );
        let cf = player.crossfade.as_ref().expect("crossfade must exist");
        assert_eq!(cf.to_clip, 2);
        assert_eq!(cf.elapsed, 0.0);
    }
}
