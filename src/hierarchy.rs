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
    if child == parent {
        log::warn!("hierarchy::attach: cannot attach an entity to itself ({child:?}); ignoring");
        return;
    }
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

/// Re-parents `child` under `new_parent` (or detaches it to a root when `new_parent` is `None`),
/// maintaining both the `Parent` and `Children` lists and **preventing cycles**.
///
/// Unlike the low-level [`attach`] (which only guards against self-attachment), this is safe to
/// drive from arbitrary UI — e.g. the editor's drag-to-reparent — because it returns `false` and
/// makes no change instead of corrupting the graph when the move would:
/// - create a cycle (`new_parent == child`, or `new_parent` is a descendant of `child`), or
/// - change nothing (`new_parent` is already the current parent, or `None` is passed for a root).
///
/// Otherwise the child is detached from its old parent, attached to the new one (or left detached
/// for `None`), and `true` is returned. The child keeps its **local** `Transform`, so its world
/// position shifts to be relative to the new parent — matching [`attach`].
pub fn reparent(world: &mut World, child: Entity, new_parent: Option<Entity>) -> bool {
    let current = world.get::<Parent>(child).map(|p| p.0);
    match new_parent {
        Some(p) => {
            if p == child || current == Some(p) {
                return false;
            }
            if is_ancestor(world, child, p) {
                log::warn!(
                    "hierarchy::reparent: {p:?} is a descendant of {child:?}; reparenting would \
                     create a cycle — ignoring"
                );
                return false;
            }
            detach(world, child);
            attach(world, child, p);
            true
        }
        None => {
            if current.is_none() {
                return false;
            }
            detach(world, child);
            true
        }
    }
}

/// Returns `true` if `maybe_ancestor` lies on the `Parent` chain above `entity`
/// (i.e. `entity` is in the subtree rooted at `maybe_ancestor`). Self is not an ancestor.
///
/// Walks up at most once per entity (a `visited` set guards against a pre-existing cycle).
fn is_ancestor(world: &World, maybe_ancestor: Entity, entity: Entity) -> bool {
    let mut visited = std::collections::HashSet::new();
    let mut cur = world.get::<Parent>(entity).map(|p| p.0);
    while let Some(p) = cur {
        if p == maybe_ancestor {
            return true;
        }
        if !visited.insert(p) {
            break; // already-cyclic graph — stop rather than loop forever
        }
        cur = world.get::<Parent>(p).map(|x| x.0);
    }
    false
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
/// `App::new()` registers this system automatically as a permanent built-in — it
/// survives scene transitions and runs **last** among unconstrained systems every
/// frame, ensuring `GlobalTransform` values are always current before rendering.
///
/// # Ordering
///
/// By default `HierarchySystem` has the highest insertion index of any system in the
/// app, so Kahn's topological sort (lowest-index tie-breaker) places it after all
/// unconstrained user systems.  A system that reads `GlobalTransform` values produced
/// this frame should declare:
///
/// ```rust,no_run
/// # use engine::{scene::{Scene, SystemRegistrar}, ecs::{System, World}, SystemConfig, HierarchySystem};
/// # struct ReadGtSystem;
/// # impl System for ReadGtSystem { fn run(&mut self, _: &mut World, _: f32) {} }
/// # struct MyScene;
/// # impl Scene for MyScene {
/// fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
///     systems.add_labeled(
///         ReadGtSystem,
///         SystemConfig::new().after(HierarchySystem::LABEL),
///     );
/// }
/// # fn on_exit(&mut self, _: &mut World) {}
/// # }
/// ```
///
/// # Depth
/// Uses topological sort (root → children order) for a single-pass propagation, supporting
/// **arbitrary-depth** hierarchies (e.g. deep bone chains like hip→torso→upper_arm→forearm→hand).
pub struct HierarchySystem;

impl HierarchySystem {
    /// Schedule label for `HierarchySystem`.
    ///
    /// Use this label to declare ordering relative to hierarchy propagation:
    /// - **`.after(HierarchySystem::LABEL)`** — system runs after propagation and sees
    ///   the current frame's `GlobalTransform` values (typical for render helpers or
    ///   systems that composite world-space positions).
    /// - **`.before(HierarchySystem::LABEL)`** — system runs before propagation; mutated
    ///   `Transform` values will be picked up by `HierarchySystem` in the same frame.
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

    /// Helpers for the reparent tests: spawn a bare entity (Transform only) and read a parent's
    /// `Children` list (empty when absent).
    fn spawn_t(world: &mut World) -> Entity {
        let e = world.spawn();
        world.add_component(e, Transform::default());
        e
    }
    fn children_of(world: &World, e: Entity) -> Vec<Entity> {
        world
            .get::<Children>(e)
            .map(|c| c.0.clone())
            .unwrap_or_default()
    }

    #[test]
    fn reparent_moves_child_between_parents() {
        let mut world = World::new();
        let (r1, r2, c) = (
            spawn_t(&mut world),
            spawn_t(&mut world),
            spawn_t(&mut world),
        );
        attach(&mut world, c, r1);

        assert!(
            reparent(&mut world, c, Some(r2)),
            "a valid move returns true"
        );
        assert_eq!(
            world.get::<Parent>(c).map(|p| p.0),
            Some(r2),
            "Parent updated"
        );
        assert!(
            children_of(&world, r1).is_empty(),
            "old parent's Children pruned"
        );
        assert_eq!(
            children_of(&world, r2),
            vec![c],
            "new parent's Children gains the child"
        );
    }

    #[test]
    fn reparent_to_none_detaches_to_root() {
        let mut world = World::new();
        let (p, c) = (spawn_t(&mut world), spawn_t(&mut world));
        attach(&mut world, c, p);

        assert!(
            reparent(&mut world, c, None),
            "detaching a child returns true"
        );
        assert!(world.get::<Parent>(c).is_none(), "child becomes a root");
        assert!(
            children_of(&world, p).is_empty(),
            "parent's Children pruned"
        );
    }

    #[test]
    fn reparent_rejects_self() {
        let mut world = World::new();
        let e = spawn_t(&mut world);
        assert!(!reparent(&mut world, e, Some(e)), "self-parent rejected");
        assert!(world.get::<Parent>(e).is_none());
    }

    #[test]
    fn reparent_rejects_descendant_cycle() {
        // p → c. Reparenting p UNDER c would make a cycle; must be refused.
        let mut world = World::new();
        let (p, c) = (spawn_t(&mut world), spawn_t(&mut world));
        attach(&mut world, c, p);

        assert!(
            !reparent(&mut world, p, Some(c)),
            "descendant target rejected"
        );
        assert!(world.get::<Parent>(p).is_none(), "p stays a root");
        assert_eq!(
            world.get::<Parent>(c).map(|x| x.0),
            Some(p),
            "c still under p"
        );
    }

    #[test]
    fn reparent_to_same_parent_is_noop_false() {
        let mut world = World::new();
        let (p, c) = (spawn_t(&mut world), spawn_t(&mut world));
        attach(&mut world, c, p);
        assert!(
            !reparent(&mut world, c, Some(p)),
            "no-op move returns false"
        );
        assert_eq!(
            children_of(&world, p),
            vec![c],
            "Children unchanged (no duplicate)"
        );
    }

    #[test]
    fn reparent_root_to_none_is_noop_false() {
        let mut world = World::new();
        let e = spawn_t(&mut world);
        assert!(!reparent(&mut world, e, None), "already a root → false");
    }

    #[test]
    fn self_attach_is_ignored() {
        // Attaching an entity to itself would create a Parent(self)+Children([self]) cycle.
        let mut world = World::new();
        let e = world.spawn();
        attach(&mut world, e, e);
        assert!(
            world.get::<Parent>(e).is_none(),
            "self-attach must not add a Parent"
        );
        assert!(
            world.get::<Children>(e).is_none(),
            "self-attach must not add Children"
        );
    }
}
