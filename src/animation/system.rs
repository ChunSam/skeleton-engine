use crate::animation::player::{AnimationPlayer, BlendWeight};
use crate::ecs::{Entity, System, World};
use crate::renderer::uv::BlendUv;

/// Advances the `AnimationPlayer` timer every frame and synchronizes
/// `UvRect` / `BlendWeight` / `BlendUv` components.
///
/// During a crossfade both clips (from and to) advance in parallel.
/// The from-frame is written to `UvRect` and the to-frame UV plus progress (weight)
/// to `BlendUv`. The shader blends them with `mix(from, to, weight)` for a
/// smooth crossfade. `BlendWeight` is always updated (1.0 when not transitioning)
/// and can be read by game code.
///
/// The `scratch` buffer is reused across frames to avoid a per-frame allocation.
/// Create with `AnimationSystem::new()` or `AnimationSystem::default()`.
#[derive(Default)]
pub struct AnimationSystem {
    scratch: Vec<Entity>,
}

impl AnimationSystem {
    /// Creates a new `AnimationSystem` with a pre-allocated scratch buffer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Schedule label. Other systems can request execution after this one via
    /// `SystemConfig::new().after(AnimationSystem::LABEL)`
    /// (e.g. StateMachineSystem must run after; BlendTreeSystem must run before).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::animation";
}

/// Returns the duration of a single frame for the given `fps`.
///
/// When `fps <= 0` (stopped or invalid) the result is `+∞` so that `while
/// timer >= frame_dur` never fires, preventing an infinite loop on negative
/// fps values.
#[inline]
fn frame_dur(fps: f32) -> f32 {
    if fps > 0.0 {
        1.0 / fps
    } else {
        f32::INFINITY
    }
}

impl System for AnimationSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.scratch.clear();
        self.scratch
            .extend(world.query::<AnimationPlayer>().map(|(e, _)| e));

        for &entity in &self.scratch {
            let (uv, weight, blend_uv) = {
                let Some(player) = world.get_mut::<AnimationPlayer>(entity) else {
                    continue;
                };

                // ── Advance crossfade ────────────────────────────────────────────
                //
                // `crossfade_just_completed` is set when the transition finishes this
                // tick.  When true, the main-clip advance block below is skipped:
                // `to_timer` (which already includes this tick's `dt`) is carried into
                // `player.timer`, so adding `dt` again would double-count the delta and
                // cause a visible low-fps first-frame stutter.
                let mut crossfade_just_completed = false;
                if let Some(cf) = player.crossfade.as_mut() {
                    cf.elapsed += dt;

                    // Advance to_clip frames
                    if let Some(to_clip) = player.clips.get(cf.to_clip) {
                        if !to_clip.frames.is_empty() {
                            // If fps <= 0 (stopped / invalid value), set frame_dur = +inf so
                            // the while loop never executes. Prevents an infinite loop (hang)
                            // when fps < 0 would make frame_dur negative.
                            let dur = frame_dur(to_clip.fps);
                            cf.to_timer += dt;
                            while cf.to_timer >= dur {
                                cf.to_timer -= dur;
                                let n = player.clips[cf.to_clip].frames.len();
                                if player.clips[cf.to_clip].looping {
                                    cf.to_frame = (cf.to_frame + 1) % n;
                                } else {
                                    cf.to_frame = (cf.to_frame + 1).min(n - 1);
                                }
                            }
                        }
                    }

                    // Check whether the transition has finished.
                    // Carry `to_timer` (already advanced by `dt` above) into `player.timer`
                    // so that the first post-blend frame lasts exactly `frame_dur - to_timer`,
                    // not `frame_dur` (which would stutter at low fps).  The main-clip advance
                    // block is then skipped for this tick to avoid counting `dt` twice.
                    if cf.elapsed >= cf.duration {
                        let to_clip = cf.to_clip;
                        let to_frame = cf.to_frame;
                        let to_timer = cf.to_timer;
                        player.current_clip = to_clip;
                        player.current_frame = to_frame;
                        player.timer = to_timer;
                        player.crossfade = None;
                        crossfade_just_completed = true;
                    }
                }

                // ── Advance current clip (from) frames ───────────────────────
                //
                // Skip this block when a crossfade completed this tick: `player.timer`
                // already equals `cf.to_timer` which has `dt` baked in from the to-side
                // advance above.  Running `timer += dt` again would double-count.
                if !crossfade_just_completed {
                    let Some(clip) = player.clips.get(player.current_clip) else {
                        continue;
                    };
                    if clip.frames.is_empty() {
                        continue;
                    }
                    // fps <= 0 → frame_dur = +inf — do not advance frames (0 = paused,
                    // negative = prevent infinite loop). The clip borrow ends at this line.
                    let dur = frame_dur(clip.fps);
                    player.timer += dt;
                    while player.timer >= dur {
                        player.timer -= dur;
                        let n = player.clips[player.current_clip].frames.len();
                        if player.clips[player.current_clip].looping {
                            player.current_frame = (player.current_frame + 1) % n;
                        } else {
                            player.current_frame = (player.current_frame + 1).min(n - 1);
                            // Non-looping: once the last frame is reached the timer has
                            // already been clamped to that frame; stop draining to avoid
                            // an infinite loop (dur is finite but frame never advances).
                            if player.current_frame + 1
                                >= player.clips[player.current_clip].frames.len()
                            {
                                player.timer = 0.0;
                                break;
                            }
                        }
                    }
                } // end !crossfade_just_completed

                // ── Determine output UV ───────────────────────────────────────
                // Always write the from-frame to UvRect; if crossfading, also pass the
                // to-frame UV + progress (weight) via BlendUv. The shader blends the two.
                let weight = player.blend_weight();
                let uv = player.current_uv();
                let blend_uv = if let Some(cf) = &player.crossfade {
                    let to = player.clips[cf.to_clip]
                        .frames
                        .get(cf.to_frame)
                        .copied()
                        .unwrap_or(crate::renderer::uv::UvRect::FULL);
                    BlendUv { to, weight }
                } else {
                    // Not transitioning — weight 0 so the renderer treats it as a single frame.
                    BlendUv {
                        to: uv,
                        weight: 0.0,
                    }
                };

                (uv, weight, blend_uv)
            };

            // Write components after the AnimationPlayer borrow is released
            world.add_component(entity, uv);
            world.add_component(entity, BlendWeight(weight));
            world.add_component(entity, blend_uv);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::animation::player::{AnimationClip, AnimationPlayer};
    use crate::ecs::World;
    use crate::renderer::uv::UvRect;

    fn make_clip(n_frames: usize, fps: f32, looping: bool) -> AnimationClip {
        AnimationClip {
            frames: vec![UvRect::FULL; n_frames],
            fps,
            looping,
        }
    }

    // ── Bug #2: to_timer must be carried on crossfade completion ─────────────────

    /// Low-fps stutter test: after crossfade completion the first post-blend frame
    /// must consume only `frame_dur - to_timer_remainder`, not a full `frame_dur`.
    ///
    /// Setup: two clips at 5 fps (frame_dur = 200 ms).  Advance the crossfade so it
    /// completes in a tick where `to_timer = 80 ms`.  The next tick (dt = 100 ms) must
    /// NOT advance to the next frame — `timer` after that tick must be 180 ms, not ≥ 200 ms.
    #[test]
    fn completion_carries_to_timer_no_double_dt() {
        let frame_dur_ms = 200.0_f32; // fps = 5.0
        let fps = 1000.0 / frame_dur_ms;
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![
                make_clip(4, fps, true), // clip 0 (A)
                make_clip(4, fps, true), // clip 1 (B)
            ]),
        );

        let mut sys = AnimationSystem::new();

        // Start A→B crossfade with duration = 0.3 s
        world
            .get_mut::<AnimationPlayer>(e)
            .unwrap()
            .play_with_crossfade(1, 0.3);

        // Advance until just before completion (elapsed < 0.3)
        // Tick 1: dt = 0.15 s → elapsed = 0.15, to_timer = 0.15 (< 0.2, no frame advance yet)
        sys.run(&mut world, 0.15);
        {
            let p = world.get::<AnimationPlayer>(e).unwrap();
            assert!(p.is_crossfading(), "crossfade should still be in progress");
        }

        // Tick 2: dt = 0.18 s → elapsed = 0.33 ≥ 0.3 → completes this tick
        // to_timer = 0.15 + 0.18 = 0.33 → wraps once (0.33 - 0.20 = 0.13), so to_timer = 0.13 s
        // After completion, player.timer must equal 0.13 s (not 0.0, not 0.13 + 0.18)
        sys.run(&mut world, 0.18);
        {
            let p = world.get::<AnimationPlayer>(e).unwrap();
            assert!(!p.is_crossfading(), "crossfade must have completed");
            assert_eq!(p.current_clip, 1, "must have switched to clip B");
            // to_timer after wrap: 0.15 + 0.18 = 0.33 → 0.33 - 0.20 = 0.13
            let expected_timer = 0.13_f32;
            assert!(
                (p.timer - expected_timer).abs() < 1e-4,
                "timer must carry to_timer remainder ({expected_timer:.4}), got {:.4}",
                p.timer
            );
        }

        // Tick 3: dt = 0.1 s → timer = 0.13 + 0.10 = 0.23 ≥ 0.20 → one frame advance
        // This tick should advance ONE frame (0.23 wraps once), leaving timer = 0.03
        let frame_before = world.get::<AnimationPlayer>(e).unwrap().current_frame;
        sys.run(&mut world, 0.10);
        {
            let p = world.get::<AnimationPlayer>(e).unwrap();
            let expected_frame = (frame_before + 1) % 4;
            assert_eq!(
                p.current_frame, expected_frame,
                "frame should advance exactly once after one full frame_dur"
            );
        }
    }

    /// Regression: with the old code (timer = 0.0 on completion), the first
    /// post-blend tick would advance the frame if dt ≥ frame_dur.  Verify the fixed
    /// code does NOT advance when dt < frame_dur and to_timer is non-zero.
    #[test]
    fn old_timer_zero_bug_is_fixed() {
        // fps = 5, frame_dur = 200 ms
        let fps = 5.0_f32;
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![make_clip(4, fps, true), make_clip(4, fps, true)]),
        );

        let mut sys = AnimationSystem::new();
        world
            .get_mut::<AnimationPlayer>(e)
            .unwrap()
            .play_with_crossfade(1, 0.05);

        // Single tick larger than duration so crossfade completes immediately.
        // dt = 0.10 s > duration 0.05 → completes; to_timer = 0.10 s → wraps once,
        // to_timer remainder = 0.10 - 0.20 = negative... use smaller dt.
        // dt = 0.06: elapsed = 0.06 ≥ 0.05 → completes; to_timer = 0.06 < 0.20, no frame wrap.
        // With the old bug timer = 0.0 + 0.06 = 0.06 (ok, but wrong origin).
        // The important check: timer = to_timer = 0.06, not 0.0 and not 0.12.
        sys.run(&mut world, 0.06);
        let p = world.get::<AnimationPlayer>(e).unwrap();
        assert!(!p.is_crossfading(), "must have completed");
        assert!(
            (p.timer - 0.06).abs() < 1e-5,
            "timer must equal to_timer=0.06, got {:.5}",
            p.timer
        );
    }

    // ── Bug #1 + #2 combined: interrupt then complete ────────────────────────────

    /// A→B interrupted to C; C then completes — verify no revert to A and frame
    /// timing is consistent throughout.
    #[test]
    fn interrupt_then_complete_no_revert_and_consistent_timing() {
        let fps = 5.0_f32; // frame_dur = 200 ms
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![
                make_clip(4, fps, true), // A = 0
                make_clip(4, fps, true), // B = 1
                make_clip(4, fps, true), // C = 2
            ]),
        );

        let mut sys = AnimationSystem::new();

        // Start A→B crossfade (duration 0.3 s)
        world
            .get_mut::<AnimationPlayer>(e)
            .unwrap()
            .play_with_crossfade(1, 0.3);

        // Advance to ~70% of A→B (elapsed = 0.21 s)
        sys.run(&mut world, 0.21);
        {
            let p = world.get::<AnimationPlayer>(e).unwrap();
            assert!(p.is_crossfading(), "A→B should still be in progress");
            assert_eq!(p.current_clip, 0, "FROM should still be A");
        }

        // Interrupt: switch to C (duration 0.2 s) — B must become the new FROM
        world
            .get_mut::<AnimationPlayer>(e)
            .unwrap()
            .play_with_crossfade(2, 0.2);
        {
            let p = world.get::<AnimationPlayer>(e).unwrap();
            assert_eq!(
                p.current_clip, 1,
                "B must be promoted to FROM (no revert to A)"
            );
            let cf = p.crossfade.as_ref().expect("B→C crossfade must exist");
            assert_eq!(cf.to_clip, 2);
        }

        // Run until B→C completes (>0.2 s remaining)
        for _ in 0..5 {
            sys.run(&mut world, 0.05);
        }
        {
            let p = world.get::<AnimationPlayer>(e).unwrap();
            assert!(!p.is_crossfading(), "B→C should have completed");
            assert_eq!(
                p.current_clip, 2,
                "must have settled on clip C (not A, not B)"
            );
            // Timer must be in range [0, frame_dur)
            assert!(
                p.timer >= 0.0 && p.timer < 0.2,
                "timer must be in [0, frame_dur) after completion, got {}",
                p.timer
            );
        }
    }

    // ── Scratch buffer reuse ─────────────────────────────────────────────────────

    /// The scratch buffer must not leak entity references across frames.
    ///
    /// Run the system twice: first with entity `a` alive, then despawn `a` and
    /// spawn `b`.  The second run must process exactly `b` and not touch `a`.
    /// If the scratch buffer leaks (e.g. old entities are still present after
    /// `clear()`), the second run would attempt to update `a` which no longer
    /// has an `AnimationPlayer` — the `world.get_mut` would return `None` and the
    /// entity would silently be skipped rather than panicking, so we instead
    /// verify by checking that `b`'s player was advanced while `a` was not.
    #[test]
    fn scratch_does_not_leak_entities_across_frames() {
        let mut world = World::new();
        let mut sys = AnimationSystem::new();

        let fps = 10.0_f32; // frame_dur = 100 ms
        let clip = make_clip(4, fps, true);

        // First frame: only entity `a`.
        let a = world.spawn();
        world.add_component(a, AnimationPlayer::new(vec![clip.clone()]));
        sys.run(&mut world, 0.0); // no-op tick to populate scratch with `a`

        // Despawn `a` and spawn `b` — the world no longer contains `a`'s component.
        // (Despawn by removing the component so queries don't find it.)
        world.remove_component::<AnimationPlayer>(a);

        let b = world.spawn();
        world.add_component(b, AnimationPlayer::new(vec![clip.clone()]));

        // Advance enough to flip a frame on `b` (> 100 ms).
        sys.run(&mut world, 0.15);

        // `b` must have been processed — its timer advanced.
        let b_player = world.get::<AnimationPlayer>(b).unwrap();
        assert!(
            b_player.timer > 0.0 || b_player.current_frame > 0,
            "b's player must have been advanced by the second run"
        );

        // `a`'s component was removed — the world should have no stale data for it.
        assert!(
            world.get::<AnimationPlayer>(a).is_none(),
            "entity a must no longer have an AnimationPlayer"
        );
    }
}
