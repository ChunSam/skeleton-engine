use glam::{EulerRot, Mat4, Quat, Vec2, Vec3};

use crate::components::Transform;
use crate::ecs::{Entity, System, World};

/// Component pointing to the parent entity.
///
/// Each frame, `HierarchySystem` computes a `GlobalTransform` for entities
/// that have this component by composing the parent's world transform.
#[derive(Debug, Clone, Copy)]
pub struct Parent(pub Entity);

/// Component holding the list of child entities.
///
/// Managed automatically alongside `Parent` via the `attach()` helper.
#[derive(Debug, Clone)]
pub struct Children(pub Vec<Entity>);

/// World-space transform after hierarchy propagation.
///
/// Computed and overwritten every frame by `HierarchySystem`.
/// The renderer prefers this component over `Transform`.
/// Root entities (without a `Parent`) are also populated with a copy of their `Transform`.
#[derive(Debug, Clone, Copy)]
pub struct GlobalTransform {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation: f32,
    pub z: f32,
}

impl GlobalTransform {
    pub fn from_transform(t: &Transform) -> Self {
        Self {
            position: t.position,
            scale: t.scale,
            rotation: t.rotation,
            z: t.z,
        }
    }

    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            Vec3::new(self.scale.x, self.scale.y, 1.0),
            Quat::from_rotation_z(self.rotation),
            Vec3::new(self.position.x, self.position.y, 0.0),
        )
    }
}

/// Attaches `child` to `parent`.
///
/// Manages the `Parent` component and the `Children` list simultaneously.
pub fn attach(world: &mut World, child: Entity, parent: Entity) {
    world.add_component(child, Parent(parent));
    let mut children = world
        .get::<Children>(parent)
        .map(|c| c.0.clone())
        .unwrap_or_default();
    if !children.contains(&child) {
        children.push(child);
    }
    world.add_component(parent, Children(children));
}

/// Detaches `child` from its parent.
///
/// Removes the `Parent` component and also removes the child from the parent's `Children` list.
pub fn detach(world: &mut World, child: Entity) {
    let parent = match world.get::<Parent>(child).copied() {
        Some(p) => p.0,
        None => return,
    };
    world.remove_component::<Parent>(child);

    let children: Vec<Entity> = world
        .get::<Children>(parent)
        .map(|c| c.0.iter().copied().filter(|&e| e != child).collect())
        .unwrap_or_default();
    world.add_component(parent, Children(children));
}

/// Topologically sorts the entity list so roots come before their children.
///
/// Uses BFS from root entities (those with no `Parent` in the provided slice) down to
/// their children. Entities whose parent is not in `entities` are treated as roots.
///
/// This is a pure hierarchy utility: it only inspects the `Parent` component and does not
/// depend on any serialization logic.
pub fn topological_sort_entities(entities: &[Entity], world: &World) -> Vec<Entity> {
    use std::collections::{HashMap, HashSet, VecDeque};

    // Parent → children adjacency map
    let mut children_map: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let entity_set: HashSet<Entity> = entities.iter().copied().collect();
    let mut roots: Vec<Entity> = Vec::new();

    for &e in entities {
        match world.get::<Parent>(e) {
            Some(p) if entity_set.contains(&p.0) => {
                children_map.entry(p.0).or_default().push(e);
            }
            _ => roots.push(e),
        }
    }

    // BFS: collect from roots down to children
    let mut result = Vec::with_capacity(entities.len());
    let mut queue: VecDeque<Entity> = roots.into_iter().collect();
    while let Some(e) = queue.pop_front() {
        result.push(e);
        if let Some(kids) = children_map.get(&e) {
            for &kid in kids {
                queue.push_back(kid);
            }
        }
    }
    result
}

/// System that propagates `GlobalTransform` through the `Transform` hierarchy.
///
/// Run automatically by `App` immediately after user systems; no manual registration needed.
///
/// # Depth
/// Uses topological sort (root → children order) for a single-pass propagation, supporting
/// **arbitrary-depth** hierarchies (e.g. deep bone chains like hip→torso→upper_arm→forearm→hand).
pub struct HierarchySystem;

impl HierarchySystem {
    /// Schedule label. Run **after** systems that mutate `Transform` so
    /// `GlobalTransform` reflects the current frame's state.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::hierarchy";
}

impl System for HierarchySystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // Topological sort of all entities with a Transform (root → children order).
        // Parents are always processed before children, so arbitrary depth propagates in a single pass.
        let all: Vec<Entity> = world.query::<Transform>().map(|(e, _)| e).collect();
        let ordered = topological_sort_entities(&all, world);

        for entity in ordered {
            let gt = match world.get::<Parent>(entity).map(|p| p.0) {
                // Has a parent: compose with the parent's GlobalTransform (already computed).
                Some(parent) => {
                    match (
                        world.get::<GlobalTransform>(parent).copied(),
                        world.get::<Transform>(entity),
                    ) {
                        (Some(pgt), Some(lt)) => compose(&pgt, lt),
                        // If the parent lacks Transform/GlobalTransform, fall back to local.
                        _ => match world.get::<Transform>(entity) {
                            Some(lt) => GlobalTransform::from_transform(lt),
                            None => continue,
                        },
                    }
                }
                // Root: copy local Transform
                None => match world.get::<Transform>(entity) {
                    Some(lt) => GlobalTransform::from_transform(lt),
                    None => continue,
                },
            };
            world.add_component(entity, gt);
        }
    }
}

/// Composes a parent world transform with a child local transform.
fn compose(parent: &GlobalTransform, local: &Transform) -> GlobalTransform {
    let world_mat = parent.to_matrix() * local.to_matrix();
    let (scale, rot_quat, translation) = world_mat.to_scale_rotation_translation();
    GlobalTransform {
        position: Vec2::new(translation.x, translation.y),
        scale: Vec2::new(scale.x, scale.y),
        rotation: rot_quat.to_euler(EulerRot::ZYX).0,
        z: parent.z + local.z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::World;

    fn pos(world: &World, e: Entity) -> Vec2 {
        world.get::<GlobalTransform>(e).unwrap().position
    }

    #[test]
    fn propagates_arbitrary_depth_chain() {
        // hip→torso→upper_arm→forearm→hand : depth 5 (impossible with the old 2-pass approach)
        let mut world = World::new();
        let names = ["hip", "torso", "upper_arm", "forearm", "hand"];
        let mut prev: Option<Entity> = None;
        let mut bones = Vec::new();
        for _ in names {
            let e = world.spawn();
            // Each bone is offset +10 to the right of its parent, no rotation or scale
            world.add_component(
                e,
                Transform {
                    position: Vec2::new(10.0, 0.0),
                    scale: Vec2::ONE,
                    rotation: 0.0,
                    z: 0.0,
                },
            );
            if let Some(p) = prev {
                attach(&mut world, e, p);
            }
            prev = Some(e);
            bones.push(e);
        }

        HierarchySystem.run(&mut world, 0.0);

        // Cumulative position: bone i is at (i+1)*10
        for (i, &e) in bones.iter().enumerate() {
            assert!(
                (pos(&world, e).x - (i as f32 + 1.0) * 10.0).abs() < 1e-3,
                "bone {i} expected x={}, got {}",
                (i as f32 + 1.0) * 10.0,
                pos(&world, e).x
            );
        }
    }

    #[test]
    fn root_without_parent_copies_local() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(5.0, 7.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 2.0,
            },
        );
        HierarchySystem.run(&mut world, 0.0);
        let gt = world.get::<GlobalTransform>(e).unwrap();
        assert_eq!(gt.position, Vec2::new(5.0, 7.0));
        assert_eq!(gt.z, 2.0);
    }
}
