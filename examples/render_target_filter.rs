//! `RenderTarget` sampler filter — `App::create_render_target_with_filter`.
//!
//! The same tiny 40×40 scene is rendered into TWO render targets via two `OffscreenCamera`s,
//! then each is displayed blown up to a 300×300 panel (7.5×). The only difference between the two
//! RTs is the sampler filter used when the RT is shown:
//!
//! - **Left**  — `FilterMode::Nearest` (the default): the upscaled texels stay sharp/blocky.
//! - **Right** — `FilterMode::Linear`: bilinear sampling smooths the upscaled texels.
//!
//! A slowly rotating diamond gives diagonal edges so the filter difference is obvious. Before
//! this knob a scaled/blurred render target was always pixelated.
//!
//! Coordinate note: the default camera is **top-left origin** (world (0,0) = screen top-left), so
//! on-screen panels are placed in screen-pixel coordinates, and the captured scene is authored far
//! off the main view (around world (1000,1000)) — its own `OffscreenCamera` frames it, and the
//! main pass never shows it directly.
use engine::{
    App, Camera, Color, DrawText, OffscreenCamera, RenderLayer, Sprite, System, TextQueue,
    Transform, WindowConfig, World,
};
use glam::Vec2;

const WIN_W: u32 = 760;
const WIN_H: u32 = 460;
/// Source render-target resolution — deliberately tiny so the upscale makes filtering obvious.
const RT_SIZE: u32 = 40;
/// On-screen size of each display panel.
const PANEL: f32 = 300.0;
const LEFT_CX: f32 = 200.0;
const RIGHT_CX: f32 = 560.0;
const PANEL_CY: f32 = 250.0;
/// Where the captured scene lives — far off the main view so it only shows inside the RTs.
const SCENE: Vec2 = Vec2::new(1000.0, 1000.0);

/// Tag for the scene content captured by the offscreen cameras (the rotating diamond).
#[derive(Clone)]
struct Spinner;

/// Spins the captured scene + draws the panel labels.
struct DemoSystem {
    t: f32,
}

impl System for DemoSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.t += dt;
        let rot = self.t * 0.7;
        let spinners: Vec<_> = world.query::<Spinner>().map(|(e, _)| e).collect();
        for e in spinners {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.rotation = rot;
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::centered(
                "Nearest (default) — blocky",
                Vec2::new(LEFT_CX, 36.0),
                18.0,
                [240, 200, 150, 255],
            ));
            tq.push(DrawText::centered(
                "Linear — smooth",
                Vec2::new(RIGHT_CX, 36.0),
                18.0,
                [160, 220, 255, 255],
            ));
            tq.push(DrawText::new(
                "Same 40x40 render target, displayed 7.5x. Only the sampler filter differs.",
                Vec2::new(20.0, WIN_H as f32 - 28.0),
                14.0,
                [180, 195, 215, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "DemoSystem"
    }
}

/// Spawns the rotating diamond + two small squares that make up the captured scene, around `SCENE`.
/// Everything is on `RenderLayer(0)` so only the offscreen cameras (mask `1 << 0`) capture it —
/// the display sprites (layer 20) are excluded, avoiding self-capture.
fn spawn_scene(app: &mut App) {
    let diamond = app.world.spawn();
    app.world.add_component(
        diamond,
        Transform {
            position: SCENE,
            scale: Vec2::new(19.0, 19.0),
            z: 1.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        diamond,
        Sprite {
            color: Color::rgba(0.95, 0.55, 0.2, 1.0),
            ..Default::default()
        },
    );
    app.world.add_component(diamond, Spinner);
    app.world.add_component(diamond, RenderLayer(0));

    for (off, color) in [
        (Vec2::new(-13.0, -13.0), Color::rgba(0.3, 0.85, 0.4, 1.0)),
        (Vec2::new(13.0, 13.0), Color::rgba(0.4, 0.5, 0.95, 1.0)),
    ] {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: SCENE + off,
                scale: Vec2::new(8.0, 8.0),
                z: 0.0,
                ..Default::default()
            },
        );
        app.world.add_component(
            e,
            Sprite {
                color,
                ..Default::default()
            },
        );
        app.world.add_component(e, RenderLayer(0));
    }
}

/// Spawns an `OffscreenCamera` framing `SCENE` into `target`, plus the display sprite that shows
/// that RT scaled up, centered on screen at `cx`.
fn spawn_view(app: &mut App, target: &str, cx: f32) {
    let oc = app.world.spawn();
    app.world.add_component(
        oc,
        OffscreenCamera {
            target: target.to_string(),
            // position is the view's top-left, so subtract half the RT (in world units) to center.
            camera: Camera::new(SCENE - Vec2::splat(RT_SIZE as f32 / 2.0), 1.0),
            layer_mask: 1 << 0, // capture world content only (not the display sprites on layer 20)
        },
    );

    let display = app.world.spawn();
    app.world.add_component(
        display,
        Transform {
            position: Vec2::new(cx, PANEL_CY),
            scale: Vec2::new(PANEL, PANEL),
            z: 100.0,
            ..Default::default()
        },
    );
    app.world.add_component(
        display,
        Sprite {
            texture: Some(target.into()),
            color: Color::WHITE,
            ..Default::default()
        },
    );
    app.world.add_component(display, RenderLayer(20));
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "RenderTarget filter — Nearest vs Linear".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });

    // Two RTs of the SAME scene — the only difference is the display sampler filter.
    app.create_render_target_with_filter("rt_nearest", RT_SIZE, RT_SIZE, wgpu::FilterMode::Nearest);
    app.create_render_target_with_filter("rt_linear", RT_SIZE, RT_SIZE, wgpu::FilterMode::Linear);

    spawn_scene(&mut app);
    spawn_view(&mut app, "rt_nearest", LEFT_CX);
    spawn_view(&mut app, "rt_linear", RIGHT_CX);

    app.add_system(DemoSystem { t: 0.0 });
    app.run();
}
