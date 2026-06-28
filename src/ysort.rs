//! Y-sorting — depth-order sprites by their world Y for top-down scenes.
//!
//! In a top-down view an entity lower on the screen should overlap one higher up (it is "nearer" the
//! camera). [`YSortSystem`] gives each [`YSort`] entity that ordering by writing its `Transform.z`
//! from `Transform.position.y` every frame, so the sprite renderer (which sorts by `(layer, z)`,
//! higher `z` in front) draws lower entities on top.

use serde::{Deserialize, Serialize};

use crate::components::Transform;
use crate::ecs::{System, World};

/// Marks an entity to be depth-sorted by its Y position (see the [module docs](crate::ysort)).
///
/// Add it alongside a [`Transform`] and let [`YSortSystem`] drive `Transform.z`. Sorting is **within
/// a [`RenderLayer`](crate::RenderLayer)** (layer is the renderer's primary sort key), so keep a
/// static background and the y-sorted actors on separate layers rather than relying on Y alone.
///
/// `bias` shifts the sort point along Y. Since Y increases downward, a **positive** bias sorts the
/// entity as if it were lower on screen (more in front); set it to about +half the sprite height to
/// sort by the sprite's "feet" (bottom edge) instead of its center.
///
/// ```
/// # use engine::YSort;
/// // Sort a 64-px-tall sprite by its feet rather than its center:
/// let feet = YSort::new(32.0);
/// assert_eq!(feet.bias, 32.0);
/// assert_eq!(YSort::default().bias, 0.0);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct YSort {
    /// Y offset added before deriving `z` (e.g. +half the sprite height to sort by the feet).
    pub bias: f32,
}

impl YSort {
    /// A `YSort` with the given Y `bias`.
    pub const fn new(bias: f32) -> Self {
        Self { bias }
    }
}

/// Sets `Transform.z = Transform.position.y + YSort::bias` for every [`YSort`] entity each frame.
///
/// User-added (like [`ParallaxSystem`](crate::ParallaxSystem)). Add it once with
/// `App::add_system(YSortSystem)`; it should run before rendering and, if the entities are part of a
/// hierarchy, before the hierarchy system that composes `GlobalTransform`.
#[derive(Default)]
pub struct YSortSystem;

impl System for YSortSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        for (_e, ysort, transform) in world.query2_mut::<YSort, Transform>() {
            transform.z = transform.position.y + ysort.bias;
        }
    }

    fn name(&self) -> &'static str {
        "YSortSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec2;

    fn spawn_at(world: &mut World, y: f32, bias: f32) -> crate::ecs::Entity {
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: Vec2::new(0.0, y),
                ..Default::default()
            },
        );
        world.add_component(e, YSort::new(bias));
        e
    }

    #[test]
    fn z_is_y_plus_bias() {
        let mut world = World::new();
        let a = spawn_at(&mut world, 100.0, 0.0);
        let b = spawn_at(&mut world, 50.0, 32.0);
        // An entity without YSort must be left untouched.
        let c = world.spawn();
        world.add_component(
            c,
            Transform {
                position: Vec2::new(0.0, 999.0),
                z: 7.0,
                ..Default::default()
            },
        );

        YSortSystem.run(&mut world, 0.016);

        assert_eq!(world.get::<Transform>(a).unwrap().z, 100.0);
        assert_eq!(world.get::<Transform>(b).unwrap().z, 82.0);
        assert_eq!(
            world.get::<Transform>(c).unwrap().z,
            7.0,
            "non-YSort untouched"
        );
    }

    #[test]
    fn lower_entity_gets_higher_z_so_it_draws_in_front() {
        // Y increases downward, higher z renders in front → the lower (larger-Y) entity must end up
        // with the larger z.
        let mut world = World::new();
        let upper = spawn_at(&mut world, 80.0, 0.0);
        let lower = spawn_at(&mut world, 240.0, 0.0);

        YSortSystem.run(&mut world, 0.016);

        let zu = world.get::<Transform>(upper).unwrap().z;
        let zl = world.get::<Transform>(lower).unwrap().z;
        assert!(
            zl > zu,
            "lower entity (larger Y) must sort in front (larger z)"
        );
    }
}
