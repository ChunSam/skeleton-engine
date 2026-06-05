use crate::animation::player::{AnimationPlayer, BlendUv, BlendWeight};
use crate::ecs::{Entity, System, World};

/// 매 프레임 `AnimationPlayer` 타이머를 진행하고 `UvRect`/`BlendWeight`/`BlendUv` 컴포넌트를 동기화한다.
///
/// 크로스페이드 중에는 두 클립(from·to)을 병렬로 진행하고, `UvRect`에 from 프레임을,
/// `BlendUv`에 to 프레임 UV + 진행도(weight)를 출력한다. 스프라이트 셰이더가
/// `mix(from, to, weight)`로 두 프레임을 합성해 부드러운 크로스페이드를 렌더한다.
/// `BlendWeight` 컴포넌트도 항상 갱신되며(전환 없으면 1.0), 게임 코드에서 활용할 수 있다.
pub struct AnimationSystem;

impl System for AnimationSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let entities: Vec<Entity> = world.query::<AnimationPlayer>().map(|(e, _)| e).collect();

        for entity in entities {
            let (uv, weight, blend_uv) = {
                let Some(player) = world.get_mut::<AnimationPlayer>(entity) else {
                    continue;
                };

                // ── 크로스페이드 진행 ───────────────────────────────────────────
                if let Some(cf) = player.crossfade.as_mut() {
                    cf.elapsed += dt;

                    // to_clip 프레임 진행
                    if let Some(to_clip) = player.clips.get(cf.to_clip) {
                        if !to_clip.frames.is_empty() {
                            // fps <= 0 (정지/비정상값)이면 frame_dur = +inf 로 두어
                            // while 루프가 실행되지 않게 한다. fps < 0 일 때 frame_dur 가
                            // 음수가 되어 무한 루프(행)에 빠지던 것을 방지.
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

                    // 전환 완료 여부 확인
                    if cf.elapsed >= cf.duration {
                        let to_clip = cf.to_clip;
                        let to_frame = cf.to_frame;
                        player.current_clip = to_clip;
                        player.current_frame = to_frame;
                        player.timer = 0.0;
                        player.crossfade = None;
                    }
                }

                // ── 현재 클립(from) 프레임 진행 ──────────────────────────────
                let Some(clip) = player.clips.get(player.current_clip) else {
                    continue;
                };
                if clip.frames.is_empty() {
                    continue;
                }
                // fps <= 0 이면 frame_dur = +inf — 프레임을 진행하지 않는다(0 = 정지,
                // 음수 = 무한 루프 방지). clip 빌림은 이 줄에서 끝난다.
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

                // ── 출력 UV 결정 ─────────────────────────────────────────────
                // from 프레임을 항상 UvRect로 출력하고, 크로스페이드 중이면 to 프레임 UV +
                // 진행도(weight)를 BlendUv로 전달한다. 셰이더가 두 프레임을 mix해 블렌딩한다.
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
                    // 전환 중이 아니면 weight 0 — 렌더러는 단일 프레임으로 처리.
                    BlendUv {
                        to: uv,
                        weight: 0.0,
                    }
                };

                (uv, weight, blend_uv)
            };

            // AnimationPlayer 빌림 해제 후 컴포넌트 기록
            world.add_component(entity, uv);
            world.add_component(entity, BlendWeight(weight));
            world.add_component(entity, blend_uv);
        }
    }
}
