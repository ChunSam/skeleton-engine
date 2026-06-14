use crate::animation::blend_tree::BlendTree1D;
use crate::animation::player::AnimationPlayer;
use crate::ecs::{Entity, System, World};

/// Evaluates the `BlendTree1D` param every frame and instructs `AnimationPlayer` to transition clips.
///
/// Must be registered **before** `AnimationSystem` so clip transitions take effect in the same frame.
///
/// The `scratch` buffer is reused across frames to avoid a per-frame allocation.
/// Create with `BlendTreeSystem::new()` or `BlendTreeSystem::default()`.
#[derive(Default)]
pub struct BlendTreeSystem {
    scratch: Vec<Entity>,
}

impl BlendTreeSystem {
    /// Creates a new `BlendTreeSystem` with a pre-allocated scratch buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule label. Recommended order: **before** `AnimationSystem::LABEL`
    /// (`SystemConfig::new().label(BlendTreeSystem::LABEL).before(AnimationSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::blend_tree";
}

impl System for BlendTreeSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        self.scratch.clear();
        self.scratch
            .extend(world.query::<BlendTree1D>().map(|(e, _)| e));

        for &entity in &self.scratch {
            // Extract the target clip and crossfade duration from BlendTree1D
            let (target_clip, crossfade_dur, already_requested) = {
                let Some(tree) = world.get_mut::<BlendTree1D>(entity) else {
                    continue;
                };
                let target = tree.target_clip();
                (target, tree.crossfade_duration, tree.last_clip)
            };

            let Some(clip_index) = target_clip else {
                continue;
            };

            // Skip if this clip was already requested
            if already_requested == Some(clip_index) {
                continue;
            }

            // Instruct AnimationPlayer to transition.
            // If a crossfade to a different clip is already in progress, skip the transition
            // this frame and re-evaluate next frame without updating last_clip — this ensures
            // that even if param jumps across multiple thresholds during a single crossfade,
            // the final target clip is never missed in favor of an intermediate one.
            //
            // The `current_clip == clip_index` branch must also be guarded with
            // `!is_crossfading()`: during a crossfade A→B the FROM is still A, so if the
            // param returns to A's range we'd erroneously update `last_clip = Some(A)` while
            // the crossfade is still running. When the crossfade completes (player now on B)
            // the dedup guard (`already_requested == Some(A)`) would then permanently skip
            // re-requesting A, leaving the player stuck on B forever.
            {
                let Some(player) = world.get_mut::<AnimationPlayer>(entity) else {
                    continue;
                };
                if player.current_clip == clip_index && !player.is_crossfading() {
                    // Truly on the target clip (not just the FROM side of a crossfade) —
                    // fall through to update last_clip below.
                } else if player.is_crossfading() {
                    // A crossfade is in progress (either to a different clip, or `current_clip`
                    // matches but we are still blending) — defer and retry next frame.
                    // Do NOT update last_clip so the target is re-evaluated once the blend settles.
                    continue;
                } else {
                    player.play_with_crossfade(clip_index, crossfade_dur);
                }
            }

            // Update the request record only when a transition was started or already on the target clip
            if let Some(tree) = world.get_mut::<BlendTree1D>(entity) {
                tree.last_clip = Some(clip_index);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::blend_tree::BlendEntry;
    use crate::animation::player::AnimationClip;
    use crate::animation::system::AnimationSystem;
    use crate::renderer::uv::UvRect;

    fn loop_clip() -> AnimationClip {
        AnimationClip {
            frames: vec![UvRect::FULL, UvRect::FULL],
            fps: 10.0,
            looping: true,
        }
    }

    fn locomotion_tree() -> BlendTree1D {
        BlendTree1D::new(
            vec![
                BlendEntry {
                    threshold: 0.0,
                    clip_index: 0,
                }, // idle
                BlendEntry {
                    threshold: 0.5,
                    clip_index: 1,
                }, // walk
                BlendEntry {
                    threshold: 1.5,
                    clip_index: 2,
                }, // run
            ],
            0.2,
        )
    }

    #[test]
    fn target_clip_picks_highest_threshold_at_or_below_param() {
        let mut tree = locomotion_tree();
        tree.set_param(-1.0);
        assert_eq!(tree.target_clip(), Some(0)); // below all thresholds → lowest clip
        tree.set_param(0.4);
        assert_eq!(tree.target_clip(), Some(0));
        tree.set_param(0.5);
        assert_eq!(tree.target_clip(), Some(1));
        tree.set_param(2.0);
        assert_eq!(tree.target_clip(), Some(2));
    }

    /// Regression test: even if param jumps two thresholds (idle→walk→run) during a
    /// single crossfade, the final target clip (run) must be reached. Before the fix,
    /// the system got stuck on walk.
    #[test]
    fn fast_param_jump_during_crossfade_reaches_final_clip() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![loop_clip(), loop_clip(), loop_clip()]),
        );
        world.add_component(e, locomotion_tree());

        let mut bt = BlendTreeSystem::new();
        let mut anim = AnimationSystem::new();

        // 1) param → walk: start an idle→walk crossfade without letting it finish yet.
        world.get_mut::<BlendTree1D>(e).unwrap().set_param(1.0);
        bt.run(&mut world, 0.05);
        anim.run(&mut world, 0.05);
        assert!(
            world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
            "idle→walk crossfade should be in progress"
        );
        assert_eq!(world.get::<AnimationPlayer>(e).unwrap().current_clip, 0);

        // 2) param → run mid-crossfade: two-step jump. Run until both transitions settle.
        world.get_mut::<BlendTree1D>(e).unwrap().set_param(2.0);
        for _ in 0..20 {
            bt.run(&mut world, 0.05);
            anim.run(&mut world, 0.05);
        }

        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            2,
            "should reach the run clip (before the fix, was stuck on walk)"
        );
    }

    /// Fix 4: param reversal during a crossfade must not permanently stick the player
    /// on the destination clip.
    ///
    /// Sequence:
    ///   1. param → walk range (clip 1) → starts idle→walk crossfade, current_clip = idle (0).
    ///   2. param → idle range (clip 0) mid-crossfade → old code updated last_clip=0 so after
    ///      the crossfade completes (player on clip 1) the dedup guard skips forever.
    ///   3. Complete the crossfade (player.current_clip = 1 = walk).
    ///   4. Run blend tree with param still in idle range → must issue a walk→idle transition.
    #[test]
    fn param_reversal_during_crossfade_does_not_stick() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![loop_clip(), loop_clip(), loop_clip()]),
        );
        world.add_component(e, locomotion_tree());

        let mut bt = BlendTreeSystem::new();
        let mut anim = AnimationSystem::new();
        let dt = 0.05_f32; // each tick

        // 1) param → walk: starts idle→walk crossfade.
        world.get_mut::<BlendTree1D>(e).unwrap().set_param(1.0);
        bt.run(&mut world, dt);
        anim.run(&mut world, dt);
        assert!(
            world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
            "should be crossfading idle→walk"
        );
        assert_eq!(world.get::<AnimationPlayer>(e).unwrap().current_clip, 0);

        // 2) param → idle mid-crossfade. Old code: updates last_clip=0, permanently blocking.
        world.get_mut::<BlendTree1D>(e).unwrap().set_param(0.0);
        bt.run(&mut world, dt);
        anim.run(&mut world, dt);
        // crossfade is still in progress (0.2 s duration, only 2 ticks of 0.05 elapsed).
        assert!(
            world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
            "crossfade must still be in progress after param reversal"
        );

        // 3) Advance just enough to complete the original idle→walk crossfade (0.2 s duration).
        // After step 2, elapsed = 0.10 s. We need 0.10 s more = 2 ticks of 0.05 s.
        // We run anim (to complete the crossfade) then check state before the next bt.run
        // issues any new transition — this avoids counting a new crossfade as "still fading".
        for _ in 0..2 {
            bt.run(&mut world, dt);
            anim.run(&mut world, dt);
        }
        // At this point the idle→walk crossfade must have completed (elapsed=0.20 >= 0.20).
        // Check AFTER anim.run but before the next bt.run would issue a new transition.
        assert!(
            !world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
            "idle→walk crossfade should have completed (elapsed was 0.10, needed 0.10 more)"
        );
        // Player is now on clip 1 (walk) because that was the crossfade target.
        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            1,
            "player must be on walk after crossfade completion"
        );

        // 4) param still in idle range → blend tree must now request a walk→idle transition
        // and eventually settle there. Run enough frames to complete the return crossfade.
        for _ in 0..20 {
            bt.run(&mut world, dt);
            anim.run(&mut world, dt);
        }
        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            0,
            "player must return to idle after crossfade completes and param is in idle range (stuck bug)"
        );
    }

    /// After accelerating to run and then decelerating, clips must return run→walk→idle
    /// (B3 regression). Param is ramped directly (bypassing input) to test blend logic only.
    #[test]
    fn decelerating_param_returns_through_clips_to_idle() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![loop_clip(), loop_clip(), loop_clip()]),
        );
        world.add_component(e, locomotion_tree());

        let mut bt = BlendTreeSystem::new();
        let mut anim = AnimationSystem::new();
        let dt = 1.0 / 60.0;

        // Accelerate: speed 0 → 2.4 (ACCEL 4.5), then hold briefly to stabilize on run.
        let mut speed = 0.0f32;
        for _ in 0..90 {
            speed = (speed + 4.5 * dt).min(2.4);
            world.get_mut::<BlendTree1D>(e).unwrap().set_param(speed);
            bt.run(&mut world, dt);
            anim.run(&mut world, dt);
        }
        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            2,
            "should be on run after acceleration"
        );

        // Decelerate: speed 2.4 → 0 (DECEL 3.5), then stabilize on idle.
        for _ in 0..120 {
            speed = (speed - 3.5 * dt).max(0.0);
            world.get_mut::<BlendTree1D>(e).unwrap().set_param(speed);
            bt.run(&mut world, dt);
            anim.run(&mut world, dt);
        }
        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            0,
            "should return to idle after deceleration (B3)"
        );
    }
}
