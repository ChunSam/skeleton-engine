use crate::animation::blend_tree::BlendTree1D;
use crate::animation::player::AnimationPlayer;
use crate::ecs::{Entity, System, World};

/// 매 프레임 `BlendTree1D`의 param을 평가해 `AnimationPlayer`에 클립 전환을 지시한다.
///
/// `AnimationSystem` **이전에** 등록해야 클립 전환이 같은 프레임에 반영된다.
pub struct BlendTreeSystem;

impl BlendTreeSystem {
    /// 스케줄 라벨. 권장 순서: `AnimationSystem::LABEL` **이전**
    /// (`SystemConfig::new().label(BlendTreeSystem::LABEL).before(AnimationSystem::LABEL)`).
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::blend_tree";
}

impl System for BlendTreeSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let entities: Vec<Entity> = world.query::<BlendTree1D>().map(|(e, _)| e).collect();

        for entity in entities {
            // 대상 클립과 크로스페이드 지속 시간을 BlendTree1D에서 추출
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

            // 이미 이 클립을 요청했으면 재요청하지 않는다
            if already_requested == Some(clip_index) {
                continue;
            }

            // AnimationPlayer에 전환 지시.
            // 이미 다른 클립으로 크로스페이드 중이면 이번 프레임엔 전환을 시작하지 않고
            // last_clip도 갱신하지 않은 채 다음 프레임에 다시 평가한다 — 그래야 한 번의
            // 크로스페이드 동안 param이 임계값을 여러 칸 건너뛰어도 마지막 목표 클립을
            // 놓치고 중간 클립에 갇히는 일이 없다.
            {
                let Some(player) = world.get_mut::<AnimationPlayer>(entity) else {
                    continue;
                };
                if player.current_clip == clip_index {
                    // 이미 목표 클립 재생 중 — 아래에서 last_clip만 갱신
                } else if player.is_crossfading() {
                    // 다른 크로스페이드 진행 중 — 보류하고 다음 프레임에 재시도
                    continue;
                } else {
                    player.play_with_crossfade(clip_index, crossfade_dur);
                }
            }

            // 전환을 시작했거나 이미 목표 클립일 때만 요청 기록을 갱신한다
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
    use crate::animation::player::{AnimationClip, UvRect};
    use crate::animation::system::AnimationSystem;

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
        assert_eq!(tree.target_clip(), Some(0)); // 모든 임계값 아래 → 가장 낮은 클립
        tree.set_param(0.4);
        assert_eq!(tree.target_clip(), Some(0));
        tree.set_param(0.5);
        assert_eq!(tree.target_clip(), Some(1));
        tree.set_param(2.0);
        assert_eq!(tree.target_clip(), Some(2));
    }

    /// 회귀 테스트: 한 번의 크로스페이드 동안 param이 임계값을 두 칸(idle→walk→run)
    /// 건너뛰어도 마지막 목표 클립(run)에 도달해야 한다. 수정 전에는 walk에 갇혔다.
    #[test]
    fn fast_param_jump_during_crossfade_reaches_final_clip() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![loop_clip(), loop_clip(), loop_clip()]),
        );
        world.add_component(e, locomotion_tree());

        let mut bt = BlendTreeSystem;
        let mut anim = AnimationSystem;

        // 1) param → walk: idle→walk 크로스페이드를 시작하되 아직 완료되지 않게 한다.
        world.get_mut::<BlendTree1D>(e).unwrap().set_param(1.0);
        bt.run(&mut world, 0.05);
        anim.run(&mut world, 0.05);
        assert!(
            world.get::<AnimationPlayer>(e).unwrap().is_crossfading(),
            "idle→walk 크로스페이드가 진행 중이어야 한다"
        );
        assert_eq!(world.get::<AnimationPlayer>(e).unwrap().current_clip, 0);

        // 2) 크로스페이드 도중 param → run: 두 칸 점프. 이후 두 전환이 모두 안정될 때까지 진행.
        world.get_mut::<BlendTree1D>(e).unwrap().set_param(2.0);
        for _ in 0..20 {
            bt.run(&mut world, 0.05);
            anim.run(&mut world, 0.05);
        }

        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            2,
            "run 클립에 도달해야 한다 (수정 전에는 walk에 갇힘)"
        );
    }

    /// 가속해 run에 도달한 뒤 감속하면 클립이 run→walk→idle로 되돌아와야 한다
    /// (B3 회귀). 입력을 우회해 param을 직접 램프하므로 블렌드 로직만 검증한다.
    #[test]
    fn decelerating_param_returns_through_clips_to_idle() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            AnimationPlayer::new(vec![loop_clip(), loop_clip(), loop_clip()]),
        );
        world.add_component(e, locomotion_tree());

        let mut bt = BlendTreeSystem;
        let mut anim = AnimationSystem;
        let dt = 1.0 / 60.0;

        // 가속: speed 0 → 2.4 (ACCEL 4.5), 이후 잠시 유지해 run 안정화.
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
            "가속 후 run에 있어야 한다"
        );

        // 감속: speed 2.4 → 0 (DECEL 3.5), 이후 idle 안정화.
        for _ in 0..120 {
            speed = (speed - 3.5 * dt).max(0.0);
            world.get_mut::<BlendTree1D>(e).unwrap().set_param(speed);
            bt.run(&mut world, dt);
            anim.run(&mut world, dt);
        }
        assert_eq!(
            world.get::<AnimationPlayer>(e).unwrap().current_clip,
            0,
            "감속 후 idle로 돌아와야 한다 (B3)"
        );
    }
}
