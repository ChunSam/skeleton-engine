//! Parallax scrolling: layers that scroll at a fraction of the camera's motion to fake depth.

use crate::camera::Camera;
use crate::components::Transform;
use crate::ecs::{Entity, System, World};
use glam::Vec2;

/// A background or foreground layer that scrolls at a fraction of the camera's motion.
///
/// `factor` is per-axis:
/// - `1.0` = locked to the world (scrolls fully with the camera, like a normal sprite),
/// - `0.0` = locked to the screen (never moves, like a far sky),
/// - `0.0..1.0` = parallax depth (smaller = appears farther / scrolls slower),
/// - `>1.0` = foreground that scrolls *faster* than the world.
///
/// Add it to any entity that also has a [`Transform`]. The first frame [`ParallaxSystem`] runs it
/// captures the entity's current position as the layer's rest anchor (and the camera's position at
/// that moment as the reference) — so just place the sprite where it should sit at the camera's
/// starting position; no base bookkeeping needed.
///
/// Because `ParallaxSystem` is a normal user system, it reads the camera position from the end of
/// the previous frame (the engine finalizes camera follow/clamp after the user system loop), so a
/// layer lags the camera by one frame during motion — sub-perceptual for backgrounds. Add
/// `ParallaxSystem` after the systems that move the camera-followed entity.
#[derive(Clone, Debug)]
pub struct ParallaxLayer {
    /// Per-axis scroll factor (see type docs). Common case: horizontal-only via [`ParallaxLayer::horizontal`].
    pub factor: Vec2,
    /// Rest anchor, lazily captured from the `Transform` on first run. `None` until then.
    base: Option<Vec2>,
    /// Camera position at capture time. `None` until first run.
    cam_ref: Option<Vec2>,
}

impl ParallaxLayer {
    /// New layer with an explicit per-axis factor.
    pub fn new(factor: Vec2) -> Self {
        Self {
            factor,
            base: None,
            cam_ref: None,
        }
    }

    /// Horizontal-only parallax: `fx` on X, `1.0` (world-locked) on Y.
    pub fn horizontal(fx: f32) -> Self {
        Self::new(Vec2::new(fx, 1.0))
    }

    /// Vertical-only parallax: `1.0` (world-locked) on X, `fy` on Y.
    pub fn vertical(fy: f32) -> Self {
        Self::new(Vec2::new(1.0, fy))
    }
}

/// Offsets every [`ParallaxLayer`] entity's [`Transform`] each frame so it scrolls at its `factor`
/// of the camera's motion. Add it with `app.add_system(ParallaxSystem)`.
#[derive(Default)]
pub struct ParallaxSystem;

impl System for ParallaxSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(cam_pos) = world.resource::<Camera>().map(|c| c.position) else {
            return;
        };
        let entities: Vec<Entity> = world.query::<ParallaxLayer>().map(|(e, _)| e).collect();
        for e in entities {
            // Read factor + capture base/cam_ref lazily from the current Transform/camera.
            let (factor, base, cam_ref) = {
                let Some(layer) = world.get::<ParallaxLayer>(e) else {
                    continue;
                };
                (layer.factor, layer.base, layer.cam_ref)
            };
            let (base, cam_ref) = match (base, cam_ref) {
                (Some(b), Some(c)) => (b, c),
                _ => {
                    let Some(t) = world.get::<Transform>(e) else {
                        continue;
                    };
                    let b = t.position;
                    if let Some(layer) = world.get_mut::<ParallaxLayer>(e) {
                        layer.base = Some(b);
                        layer.cam_ref = Some(cam_pos);
                    }
                    (b, cam_pos)
                }
            };
            // pos = base + (cam - cam_ref) * (1 - factor)
            let offset = (cam_pos - cam_ref) * (Vec2::ONE - factor);
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position = base + offset;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;
    use crate::components::Transform;
    use crate::ecs::World;
    use glam::Vec2;

    fn make_world_with_layer(cam_start: Vec2, entity_pos: Vec2, factor: Vec2) -> (World, Entity) {
        let mut world = World::new();
        world.insert_resource(Camera::new(cam_start, 1.0));
        let e = world.spawn();
        world.add_component(
            e,
            Transform {
                position: entity_pos,
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        world.add_component(e, ParallaxLayer::new(factor));
        (world, e)
    }

    fn move_camera(world: &mut World, new_pos: Vec2) {
        if let Some(cam) = world.resource_mut::<Camera>() {
            cam.position = new_pos;
        }
    }

    fn get_pos(world: &World, e: Entity) -> Vec2 {
        world
            .get::<Transform>(e)
            .map(|t| t.position)
            .unwrap_or(Vec2::ZERO)
    }

    /// factor = 1.0 (world-locked): moving the camera does not change the entity's position.
    #[test]
    fn factor_one_world_locked() {
        let cam_start = Vec2::ZERO;
        let entity_pos = Vec2::new(200.0, 100.0);
        let (mut world, e) = make_world_with_layer(cam_start, entity_pos, Vec2::ONE);

        // First run: captures base and cam_ref
        let mut sys = ParallaxSystem;
        sys.run(&mut world, 0.016);

        // Move camera right by 100 px
        move_camera(&mut world, Vec2::new(100.0, 0.0));
        sys.run(&mut world, 0.016);

        let pos = get_pos(&world, e);
        // offset = (100,0) * (1 - 1) = (0,0) => position unchanged
        assert!(
            (pos.x - entity_pos.x).abs() < 1e-4,
            "x should be {}, got {}",
            entity_pos.x,
            pos.x
        );
        assert!(
            (pos.y - entity_pos.y).abs() < 1e-4,
            "y should be {}, got {}",
            entity_pos.y,
            pos.y
        );
    }

    /// factor = 0.0 (screen-locked): entity follows the camera exactly (stays fixed on screen).
    #[test]
    fn factor_zero_screen_locked() {
        let cam_start = Vec2::ZERO;
        let entity_pos = Vec2::new(50.0, 50.0);
        let delta = Vec2::new(100.0, 0.0);
        let (mut world, e) = make_world_with_layer(cam_start, entity_pos, Vec2::ZERO);

        let mut sys = ParallaxSystem;
        sys.run(&mut world, 0.016);

        move_camera(&mut world, cam_start + delta);
        sys.run(&mut world, 0.016);

        let pos = get_pos(&world, e);
        // offset = delta * (1 - 0) = delta => pos = base + delta
        let expected = entity_pos + delta;
        assert!(
            (pos.x - expected.x).abs() < 1e-4,
            "x should be {}, got {}",
            expected.x,
            pos.x
        );
        assert!(
            (pos.y - expected.y).abs() < 1e-4,
            "y should be {}, got {}",
            expected.y,
            pos.y
        );
    }

    /// factor = 0.5: entity moves at half the camera speed.
    #[test]
    fn factor_half_midground() {
        let cam_start = Vec2::ZERO;
        let entity_pos = Vec2::new(300.0, 200.0);
        let delta = Vec2::new(100.0, 0.0);
        let (mut world, e) = make_world_with_layer(cam_start, entity_pos, Vec2::new(0.5, 1.0));

        let mut sys = ParallaxSystem;
        sys.run(&mut world, 0.016);

        move_camera(&mut world, cam_start + delta);
        sys.run(&mut world, 0.016);

        let pos = get_pos(&world, e);
        // offset = (100,0) * (1 - 0.5, 1 - 1.0) = (50, 0) => pos = (350, 200)
        let expected_x = entity_pos.x + delta.x * 0.5;
        let expected_y = entity_pos.y;
        assert!(
            (pos.x - expected_x).abs() < 1e-4,
            "x should be {}, got {}",
            expected_x,
            pos.x
        );
        assert!(
            (pos.y - expected_y).abs() < 1e-4,
            "y should be {}, got {}",
            expected_y,
            pos.y
        );
    }
}
