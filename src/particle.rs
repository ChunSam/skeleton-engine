use glam::Vec2;
use rand::Rng;

use crate::components::{Sprite, Transform};
use crate::ecs::{Entity, System, World};

type ParticleUpdate = (Entity, f32, f32, Vec2, [f32; 4], [f32; 4]);
type EmitterSnapshot = (
    Entity,
    Vec2,
    bool,
    f32,
    f32,
    Vec2,
    Vec2,
    [f32; 4],
    [f32; 4],
    Vec2,
    Option<String>,
);
// (entity, pos, lifetime, velocity_spread, color_start, color_end, size, texture)
type BurstSnapshot = (
    Entity,
    Vec2,
    f32,
    Vec2,
    [f32; 4],
    [f32; 4],
    Vec2,
    Option<String>,
);

// ─── 컴포넌트 ─────────────────────────────────────────────────────────────────

/// 파티클을 방출하는 이미터 컴포넌트.
///
/// 엔티티에 `Transform`과 함께 붙이면 `ParticleSystem`이 파티클을 생성한다.
pub struct ParticleEmitter {
    /// 초당 파티클 생성 수
    pub spawn_rate: f32,
    /// 파티클 생존 시간 (초)
    pub lifetime: f32,
    /// 기본 속도 (픽셀/초)
    pub velocity: Vec2,
    /// 속도에 추가되는 랜덤 범위 (±각 축)
    pub velocity_spread: Vec2,
    /// 생성 시 색상 (RGBA)
    pub color_start: [f32; 4],
    /// 소멸 시 색상 (RGBA) — 생존 시간에 따라 보간
    pub color_end: [f32; 4],
    /// 파티클 크기 (픽셀)
    pub size: Vec2,
    /// 텍스처 경로. None이면 단색 사각형.
    pub texture: Option<String>,
    /// false이면 방출 중단
    pub emit: bool,
    /// 내부 타이머 (직접 수정 불필요)
    pub(crate) timer: f32,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            spawn_rate: 20.0,
            lifetime: 1.0,
            velocity: Vec2::new(0.0, -50.0),
            velocity_spread: Vec2::new(20.0, 10.0),
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
            size: Vec2::splat(8.0),
            texture: None,
            emit: true,
            timer: 0.0,
        }
    }
}

impl ParticleEmitter {
    /// 일회성 버스트(폭발/타격) 전용 이미터 설정.
    ///
    /// 연속 방출을 끄고(`emit = false`, `spawn_rate = 0`) 짧은 수명·방사형
    /// 확산에 어울리는 기본값을 채운다. 반드시 [`ParticleBurst`]와 함께 한
    /// 엔티티에 붙여야 한다 — [`ParticleSystem`]이 다음 틱에 `remaining`개를
    /// 한꺼번에 방출한 뒤 그 엔티티를 despawn한다.
    ///
    /// 연속 이미터의 동작에는 영향을 주지 않는 순수 추가 API다.
    pub fn for_burst() -> Self {
        Self {
            spawn_rate: 0.0,
            lifetime: 0.5,
            velocity: Vec2::ZERO,
            velocity_spread: Vec2::splat(160.0),
            color_start: [1.0, 0.85, 0.35, 1.0],
            color_end: [1.0, 0.25, 0.1, 0.0],
            size: Vec2::splat(6.0),
            texture: None,
            emit: false,
            timer: 0.0,
        }
    }
}

/// 활성 파티클 컴포넌트.
pub struct Particle {
    pub lifetime: f32,
    pub age: f32,
    pub velocity: Vec2,
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
}

/// 일회성 파티클 버스트 마커.
///
/// [`ParticleEmitter`]와 함께 한 엔티티에 붙이면 [`ParticleSystem`]이 다음
/// 틱에 `remaining`개의 파티클을 방사형으로 한꺼번에 방출한 뒤, **그 엔티티를
/// despawn한다.** 폭발·타격 이펙트처럼 전용 단발 엔티티에 사용한다.
///
/// 연속 방출(`ParticleEmitter::emit`)과 독립적이며, 기존 연속 이미터 동작을
/// 전혀 바꾸지 않는 순수 추가 컴포넌트다. 방출 속도(픽셀/초)는 이미터의
/// `velocity_spread` 크기를 반지름으로 사용한다.
pub struct ParticleBurst {
    /// 이번 버스트에서 방출할 파티클 수.
    pub remaining: u32,
}

// ─── 시스템 ──────────────────────────────────────────────────────────────────

pub struct ParticleSystem;

impl System for ParticleSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // 1. 기존 파티클 이동·색상 업데이트, 만료된 것은 수집
        let updates: Vec<ParticleUpdate> = world
            .query::<Particle>()
            .map(|(e, p)| (e, p.age, p.lifetime, p.velocity, p.color_start, p.color_end))
            .collect();

        let mut to_despawn = Vec::new();
        for (entity, age, lifetime, velocity, color_start, color_end) in updates {
            let new_age = age + dt;
            if new_age >= lifetime {
                to_despawn.push(entity);
                continue;
            }
            if let Some(tr) = world.get_mut::<Transform>(entity) {
                tr.position += velocity * dt;
            }
            let t = new_age / lifetime;
            let lerped = [
                color_start[0] + (color_end[0] - color_start[0]) * t,
                color_start[1] + (color_end[1] - color_start[1]) * t,
                color_start[2] + (color_end[2] - color_start[2]) * t,
                color_start[3] + (color_end[3] - color_start[3]) * t,
            ];
            if let Some(sp) = world.get_mut::<Sprite>(entity) {
                sp.color = lerped;
            }
            if let Some(p) = world.get_mut::<Particle>(entity) {
                p.age = new_age;
            }
        }
        for e in to_despawn {
            world.despawn(e);
        }

        // 2. 이미터에서 새 파티클 방출
        let emitter_data: Vec<EmitterSnapshot> = world
            .query2::<Transform, ParticleEmitter>()
            .map(|(e, tr, em)| {
                (
                    e,
                    tr.position,
                    em.emit,
                    em.spawn_rate,
                    em.lifetime,
                    em.velocity,
                    em.velocity_spread,
                    em.color_start,
                    em.color_end,
                    em.size,
                    em.texture.clone(),
                )
            })
            .collect();

        let mut rng = rand::thread_rng();
        for (
            emitter_entity,
            pos,
            emit,
            spawn_rate,
            lifetime,
            velocity,
            spread,
            color_start,
            color_end,
            size,
            texture,
        ) in emitter_data
        {
            if !emit || spawn_rate <= 0.0 {
                continue;
            }
            let should_spawn = {
                let em = world.get_mut::<ParticleEmitter>(emitter_entity).unwrap();
                em.timer += dt;
                let interval = 1.0 / spawn_rate;
                if em.timer >= interval {
                    em.timer -= interval;
                    true
                } else {
                    false
                }
            };
            if !should_spawn {
                continue;
            }

            let actual_velocity = Vec2::new(
                velocity.x + rng.gen_range(-spread.x..=spread.x),
                velocity.y + rng.gen_range(-spread.y..=spread.y),
            );

            spawn_particle(
                world,
                pos,
                size,
                &texture,
                actual_velocity,
                lifetime,
                color_start,
                color_end,
            );
        }

        // 3. 일회성 버스트 방출 (ParticleEmitter + ParticleBurst). 연속 방출과
        //    독립적이며, 방출 후 이미터 엔티티를 despawn한다.
        let burst_emitters: Vec<BurstSnapshot> = world
            .query2::<Transform, ParticleEmitter>()
            .map(|(e, tr, em)| {
                (
                    e,
                    tr.position,
                    em.lifetime,
                    em.velocity_spread,
                    em.color_start,
                    em.color_end,
                    em.size,
                    em.texture.clone(),
                )
            })
            .collect();
        for (entity, pos, lifetime, spread, color_start, color_end, size, texture) in burst_emitters
        {
            let Some(remaining) = world.get::<ParticleBurst>(entity).map(|b| b.remaining) else {
                continue;
            };
            let radius = spread.max_element().max(1.0);
            for _ in 0..remaining {
                let angle = rng.gen_range(0.0..std::f32::consts::TAU);
                let speed = rng.gen_range(0.2..=1.0) * radius;
                let vel = Vec2::new(angle.cos(), angle.sin()) * speed;
                spawn_particle(
                    world,
                    pos,
                    size,
                    &texture,
                    vel,
                    lifetime,
                    color_start,
                    color_end,
                );
            }
            world.despawn(entity);
        }
    }
}

/// 단일 파티클 엔티티를 스폰한다 (연속 방출과 버스트가 공유).
#[allow(clippy::too_many_arguments)]
fn spawn_particle(
    world: &mut World,
    pos: Vec2,
    size: Vec2,
    texture: &Option<String>,
    velocity: Vec2,
    lifetime: f32,
    color_start: [f32; 4],
    color_end: [f32; 4],
) {
    let pe = world.spawn();
    world.add_component(
        pe,
        Transform {
            position: pos,
            scale: size,
            rotation: 0.0,
            z: 0.0,
        },
    );
    let sprite = match texture {
        Some(path) => Sprite::textured(path.as_str()),
        None => Sprite {
            texture: None,
            color: color_start,
            image_handle: None,
        },
    };
    world.add_component(pe, sprite);
    world.add_component(
        pe,
        Particle {
            lifetime,
            age: 0.0,
            velocity,
            color_start,
            color_end,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    #[test]
    fn burst_emits_count_then_retires_emitter() {
        let mut world = World::new();
        let emitter = world.spawn();
        world.add_component(emitter, Transform::default());
        world.add_component(emitter, ParticleEmitter::for_burst());
        world.add_component(emitter, ParticleBurst { remaining: 8 });

        ParticleSystem.run(&mut world, 0.016);

        // 한 틱에 정확히 8개의 파티클이 방출된다.
        assert_eq!(world.query::<Particle>().count(), 8);
        // 버스트 후 이미터 엔티티는 despawn된다 (일회성).
        assert!(!world.is_alive(emitter));
    }

    #[test]
    fn continuous_emitter_unaffected_by_burst_path() {
        let mut world = World::new();
        let emitter = world.spawn();
        world.add_component(emitter, Transform::default());
        // 연속 이미터: emit=true, 높은 spawn_rate. ParticleBurst 없음.
        world.add_component(
            emitter,
            ParticleEmitter {
                spawn_rate: 100.0,
                emit: true,
                ..ParticleEmitter::default()
            },
        );

        ParticleSystem.run(&mut world, 0.05);

        // 연속 이미터는 파티클을 내되, despawn되지 않고 살아남는다.
        assert!(world.query::<Particle>().count() >= 1);
        assert!(world.is_alive(emitter));
    }
}
