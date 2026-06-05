use std::collections::{HashMap, HashSet};

use glam::Vec2;
use rapier2d::prelude::*;

use crate::components::Transform;
use crate::ecs::{Entity, Events, System, World};
use crate::physics::body::PhysicsBody;
use crate::physics::events::{CollisionEvent, TriggerEvent};
use crate::physics::world::PhysicsWorld;

/// 범용 물리 시스템: 매 프레임 step → Transform 동기화.
///
/// `PhysicsWorld`에 바디를 만들 때의 위치/크기는 물리 단위이고, 이 시스템은 Rapier 결과에
/// `pixels_per_unit`을 곱해 `Transform.position` 픽셀 좌표로 반영한다. 예를 들어
/// `pixels_per_unit = 50.0`이면 물리 1 unit이 화면 50px에 해당한다.
///
/// 위치뿐 아니라 바디의 **회전각도 `Transform.rotation`(라디안)에 동기화**한다 — 조인트로
/// 자유 회전하는 바디(힌지 암 등)의 스프라이트가 물리와 함께 돌도록. 회전을 잠근 바디
/// (`lock_rotation: true`)는 각도가 항상 0이라 영향이 없다.
///
/// # Setup
///
/// `PhysicsWorld`는 이 시스템이 아닌 **World 리소스**로 관리된다. 사용 전 반드시
/// `world.insert_resource(physics)` 로 삽입하고, `PhysicsSystem::new(ppu)` 만 시스템에 등록한다.
///
/// ```ignore
/// let physics = PhysicsWorld::new(gravity);
/// app.world.insert_resource(physics);
/// app.add_system(PhysicsSystem::new(50.0));
/// ```
///
/// 다른 시스템이나 게임 코드에서 물리 월드에 접근하려면 `world.resource_mut::<PhysicsWorld>()`
/// 를 사용한다.
pub struct PhysicsSystem {
    /// 화면 픽셀당 물리 단위 비율. 예: 50.0 → 1 unit = 50px
    pub pixels_per_unit: f32,
    active_contacts: HashSet<(ColliderHandle, ColliderHandle)>,
    active_intersections: HashSet<(ColliderHandle, ColliderHandle)>,
}

impl PhysicsSystem {
    pub fn new(pixels_per_unit: f32) -> Self {
        debug_assert!(
            pixels_per_unit > 0.0,
            "PhysicsSystem::new requires pixels_per_unit > 0"
        );
        Self {
            pixels_per_unit: pixels_per_unit.max(f32::EPSILON),
            active_contacts: HashSet::new(),
            active_intersections: HashSet::new(),
        }
    }
}

fn ordered_pair(a: ColliderHandle, b: ColliderHandle) -> (ColliderHandle, ColliderHandle) {
    if a.into_raw_parts() <= b.into_raw_parts() {
        (a, b)
    } else {
        (b, a)
    }
}

impl System for PhysicsSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(mut physics) = world.remove_resource::<PhysicsWorld>() else {
            return;
        };

        physics.step(dt);

        // ── 충돌 이벤트 diff ──────────────────────────────────────────────────
        let col_map: HashMap<ColliderHandle, Entity> = world
            .query::<PhysicsBody>()
            .map(|(e, b)| (b.collider_handle, e))
            .collect();

        // Rapier는 동일 쌍의 collider1/collider2 순서를 프레임 간 유지한다
        let current: HashSet<(ColliderHandle, ColliderHandle)> = physics
            .narrow_phase
            .contact_pairs()
            .filter(|p| p.has_any_active_contact)
            .filter_map(|p| {
                col_map.get(&p.collider1)?;
                col_map.get(&p.collider2)?;
                Some((p.collider1, p.collider2))
            })
            .collect();

        let mut collision_events: Vec<CollisionEvent> = Vec::new();
        for &(c1, c2) in &current {
            if !self.active_contacts.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    collision_events.push(CollisionEvent::Started(e1, e2));
                }
            }
        }
        for &(c1, c2) in &self.active_contacts {
            if !current.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    collision_events.push(CollisionEvent::Stopped(e1, e2));
                }
            }
        }
        self.active_contacts = current;

        if !collision_events.is_empty() {
            if let Some(bus) = world.resource_mut::<Events<CollisionEvent>>() {
                for ev in collision_events {
                    bus.send(ev);
                }
            }
        }
        // ── end 충돌 이벤트 diff ─────────────────────────────────────────────

        // ── 센서 이벤트 diff ──────────────────────────────────────────────────
        let current_intersections: HashSet<(ColliderHandle, ColliderHandle)> = physics
            .narrow_phase
            .intersection_pairs()
            .filter(|(_, _, intersecting)| *intersecting)
            .filter_map(|(c1, c2, _)| {
                col_map.get(&c1)?;
                col_map.get(&c2)?;
                Some(ordered_pair(c1, c2))
            })
            .collect();

        let mut trigger_events: Vec<TriggerEvent> = Vec::new();
        for &(c1, c2) in &current_intersections {
            if !self.active_intersections.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    trigger_events.push(TriggerEvent::Entered(e1, e2));
                }
            }
        }
        for &(c1, c2) in &self.active_intersections {
            if !current_intersections.contains(&(c1, c2)) {
                if let (Some(&e1), Some(&e2)) = (col_map.get(&c1), col_map.get(&c2)) {
                    trigger_events.push(TriggerEvent::Exited(e1, e2));
                }
            }
        }
        self.active_intersections = current_intersections;

        if !trigger_events.is_empty() {
            if let Some(bus) = world.resource_mut::<Events<TriggerEvent>>() {
                for ev in trigger_events {
                    bus.send(ev);
                }
            }
        }
        // ── end 센서 이벤트 diff ─────────────────────────────────────────────

        // borrow checker: (entity, handle) 를 먼저 수집해야 world를 다시 빌릴 수 있다
        let pairs: Vec<(Entity, RigidBodyHandle)> = world
            .query::<PhysicsBody>()
            .map(|(e, b)| (e, b.rigid_body_handle))
            .collect();

        let scale = self.pixels_per_unit.max(f32::EPSILON);
        for (entity, handle) in pairs {
            if let Some(body) = physics.rigid_body(handle) {
                let t = *body.translation();
                let angle = body.rotation().angle();
                if let Some(tr) = world.get_mut::<Transform>(entity) {
                    tr.position = Vec2::new(t.x * scale, t.y * scale);
                    tr.rotation = angle;
                }
            }
        }

        world.insert_resource(physics);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Transform;
    use crate::physics::events::TriggerEvent;

    #[test]
    fn sensor_intersection_emits_trigger_entered() {
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (sensor_body, sensor_col) = physics.add_sensor_box(Vec2::ZERO, 1.0, 1.0);
        let (actor_body, actor_col) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);

        let mut world = World::new();
        world.insert_resource(physics);
        world.insert_resource(Events::<TriggerEvent>::default());
        let mut system = PhysicsSystem::new(1.0);

        let sensor = world.spawn();
        world.add_component(
            sensor,
            PhysicsBody {
                rigid_body_handle: sensor_body,
                collider_handle: sensor_col,
            },
        );
        world.add_component(sensor, Transform::default());

        let actor = world.spawn();
        world.add_component(
            actor,
            PhysicsBody {
                rigid_body_handle: actor_body,
                collider_handle: actor_col,
            },
        );
        world.add_component(actor, Transform::default());

        system.run(&mut world, 1.0 / 60.0);

        let events = world.resource::<Events<TriggerEvent>>().unwrap().read();
        assert_eq!(events, &[TriggerEvent::Entered(sensor, actor)]);
    }

    #[test]
    #[should_panic(expected = "PhysicsSystem::new requires pixels_per_unit > 0")]
    fn non_positive_pixels_per_unit_is_clamped() {
        let _ = PhysicsSystem::new(0.0);
    }

    #[test]
    fn syncs_body_rotation_to_transform() {
        // 회전하는 동적 바디의 각도가 Transform.rotation으로 동기화되어야 한다.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb, col) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, false);
        physics.rigid_body_mut(rb).unwrap().set_angvel(3.0, true);

        let mut world = World::new();
        world.insert_resource(physics);
        let mut system = PhysicsSystem::new(1.0);
        let e = world.spawn();
        world.add_component(
            e,
            PhysicsBody {
                rigid_body_handle: rb,
                collider_handle: col,
            },
        );
        world.add_component(e, Transform::default());

        let dt = 1.0 / 60.0;
        system.run(&mut world, dt);

        let rotation = world.get_mut::<Transform>(e).unwrap().rotation;
        // angvel 3.0 rad/s 가 dt만큼 적분 → 약 0.05 rad. 동기화 안 하면 0으로 남는다.
        assert!(
            (rotation - 3.0 * dt).abs() < 1e-2,
            "rotation 은 body 각도(≈angvel*dt)와 일치해야 함: {rotation}"
        );
    }

    #[test]
    fn locked_rotation_body_keeps_zero_rotation() {
        // 회전 잠금 바디는 토크를 줘도 각도 0 — 동기화가 무해함을 보장.
        let mut physics = PhysicsWorld::new(Vec2::ZERO);
        let (rb, col) = physics.add_dynamic_box(Vec2::ZERO, 0.5, 0.5, true);
        physics
            .rigid_body_mut(rb)
            .unwrap()
            .apply_torque_impulse(5.0, true);

        let mut world = World::new();
        world.insert_resource(physics);
        let mut system = PhysicsSystem::new(1.0);
        let e = world.spawn();
        world.add_component(
            e,
            PhysicsBody {
                rigid_body_handle: rb,
                collider_handle: col,
            },
        );
        world.add_component(e, Transform::default());

        system.run(&mut world, 1.0 / 60.0);

        let rotation = world.get_mut::<Transform>(e).unwrap().rotation;
        assert!(
            rotation.abs() < 1e-6,
            "회전 잠금 바디는 rotation 0을 유지해야 함: {rotation}"
        );
    }
}
