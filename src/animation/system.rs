use crate::animation::player::{AnimationPlayer, BlendUv, BlendWeight};
use crate::ecs::{Entity, System, World};

/// Advances the `AnimationPlayer` timer every frame and synchronizes
/// `UvRect` / `BlendWeight` / `BlendUv` components.
///
/// During a crossfade both clips (from and to) advance in parallel.
/// The from-frame is written to `UvRect` and the to-frame UV plus progress (weight)
/// to `BlendUv`. The sprite shader blends them with `mix(from, to, weight)` for a
/// smooth crossfade. `BlendWeight` is always updated (1.0 when not transitioning)
/// and can be read by game code.
pub struct AnimationSystem;

impl AnimationSystem {
    /// Schedule label. Other systems can request execution after this one via
    /// `SystemConfig::new().after(AnimationSystem::LABEL)`
    /// (e.g. StateMachineSystem must run after; BlendTreeSystem must run before).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::animation";
}

impl System for AnimationSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let entities: Vec<Entity> = world.query::<AnimationPlayer>().map(|(e, _)| e).collect();

        for entity in entities {
            let (uv, weight, blend_uv) = {
                let Some(player) = world.get_mut::<AnimationPlayer>(entity) else {
                    continue;
                };

                // ── Advance crossfade ────────────────────────────────────────────
                if let Some(cf) = player.crossfade.as_mut() {
                    cf.elapsed += dt;

                    // Advance to_clip frames
                    if let Some(to_clip) = player.clips.get(cf.to_clip) {
                        if !to_clip.frames.is_empty() {
                            // If fps <= 0 (stopped / invalid value), set frame_dur = +inf so
                            // the while loop never executes. Prevents an infinite loop (hang)
                            // when fps < 0 would make frame_dur negative.
                            let frame_dur = if to_clip.fps > 0.0 {
                                1.0 / to_clip.fps
                            } else {
                                f32::INFINITY
                            };
                            cf.to_timer += dt;
                            while cf.to_timer >= frame_dur {
                                cf.to_timer -= frame_dur;
                                let n = player.clips[cf.to_clip].frames.len();
                                if player.clips[cf.to_clip].looping {
                                    cf.to_frame = (cf.to_frame + 1) % n;
                                } else {
                                    cf.to_frame = (cf.to_frame + 1).min(n - 1);
                                }
                            }
                        }
                    }

                    // Check whether the transition has finished
                    if cf.elapsed >= cf.duration {
                        let to_clip = cf.to_clip;
                        let to_frame = cf.to_frame;
                        player.current_clip = to_clip;
                        player.current_frame = to_frame;
                        player.timer = 0.0;
                        player.crossfade = None;
                    }
                }

                // ── Advance current clip (from) frames ───────────────────────
                let Some(clip) = player.clips.get(player.current_clip) else {
                    continue;
                };
                if clip.frames.is_empty() {
                    continue;
                }
                // fps <= 0 → frame_dur = +inf — do not advance frames (0 = paused,
                // negative = prevent infinite loop). The clip borrow ends at this line.
                let frame_dur = if clip.fps > 0.0 {
                    1.0 / clip.fps
                } else {
                    f32::INFINITY
                };
                player.timer += dt;
                if player.timer >= frame_dur {
                    player.timer -= frame_dur;
                    let n = player.clips[player.current_clip].frames.len();
                    if player.clips[player.current_clip].looping {
                        player.current_frame = (player.current_frame + 1) % n;
                    } else {
                        player.current_frame = (player.current_frame + 1).min(n - 1);
                    }
                }

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
                        .unwrap_or(crate::animation::player::UvRect::FULL);
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
