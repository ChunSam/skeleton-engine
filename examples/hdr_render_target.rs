//! HDR (`Rgba16Float`) offscreen render target vs an ordinary 8-bit one — dogfoods
//! [`App::create_render_target_with_format`](engine::App::create_render_target_with_format) and the
//! format-matched sprite pipeline.
//!
//! An off-screen "scene" of **over-bright** sprites (colour values `> 1.0` — a dim background, a
//! mid-bright square, and a very bright core) is rendered by **two** `OffscreenCamera`s into two
//! render targets framing the same scene:
//! * **HDR** — `Rgba16Float`, which *stores* the `> 1.0` values, and
//! * **LDR** — the default 8-bit `Rgba8UnormSrgb`, which **clamps** them to `1.0` at store time.
//!
//! Both targets are shown on-screen as "monitors", each displayed with an **exposure** multiply (the
//! display sprite's colour). Lower the exposure with `↓`: on the HDR monitor the bright core and the
//! mid square stay distinct (their stored values differ), while on the LDR monitor they collapse to
//! the **same** brightness — the headroom the 8-bit target threw away. Tone-mapping for final display
//! is the game's job; this demo uses the simplest possible one (a constant exposure scale).
//!
//! Keys: `↑`/`↓` exposure · `Esc` quit. Native-only (render targets are native).
//!
//! Run: `cargo run --example hdr_render_target`

use engine::{
    App, Camera, Color, DrawText, Entity, InputState, KeyCode, OffscreenCamera, RenderLayer,
    ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 820;
const WIN_H: u32 = 460;
const RT_W: u32 = 340;
const RT_H: u32 = 300;
/// World position of the off-screen bright scene's **center** (far from the main camera so it is
/// only ever seen through the render targets, never directly).
const SCENE: Vec2 = Vec2::new(5000.0, 2000.0);

fn spawn_quad(app: &mut App, offset: Vec2, size: Vec2, color: [f32; 4], z: f32, layer: i32) {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: SCENE + offset,
            scale: size,
            rotation: 0.0,
            z,
        },
    );
    app.world.add_component(
        e,
        Sprite {
            color: Color::from(color),
            ..Default::default()
        },
    );
    app.world.add_component(e, RenderLayer(layer));
}

fn spawn_monitor(app: &mut App, pos: Vec2, size: Vec2, rt_name: &str) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: pos,
            scale: size,
            rotation: 0.0,
            z: 50.0,
        },
    );
    app.world.add_component(
        e,
        Sprite {
            texture: Some(rt_name.into()),
            color: Color::WHITE,
            ..Default::default()
        },
    );
    app.world.add_component(e, RenderLayer(1)); // layer 1 = NOT captured by the OffscreenCameras
    e
}

struct HdrDemo {
    hdr_monitor: Entity,
    ldr_monitor: Entity,
    exposure: f32,
}

impl System for HdrDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (up, down, quit) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.just_pressed(KeyCode::ArrowUp),
                input.just_pressed(KeyCode::ArrowDown),
                input.just_pressed(KeyCode::Escape),
            )
        };
        if up {
            self.exposure = (self.exposure + 0.05).min(1.0);
        }
        if down {
            self.exposure = (self.exposure - 0.05).max(0.05);
        }
        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        // The exposure is the display sprite's colour multiply: displayed = sampled_rt * exposure.
        let e = self.exposure;
        let tint = Color::from([e, e, e, 1.0]);
        for m in [self.hdr_monitor, self.ldr_monitor] {
            if let Some(s) = world.get_mut::<Sprite>(m) {
                s.color = tint;
            }
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "HDR render target (Rgba16Float)  vs  LDR (Rgba8 sRGB)  —  same over-bright scene",
                Vec2::new(20.0, 18.0),
                17.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                "HDR  (keeps > 1.0)",
                Vec2::new(120.0, 56.0),
                15.0,
                [150, 230, 255, 255],
            ));
            tq.push(DrawText::new(
                "LDR  (clamps to 1.0)",
                Vec2::new(530.0, 56.0),
                15.0,
                [255, 200, 150, 255],
            ));
            tq.push(DrawText::new(
                format!(
                    "exposure: {:.2}   (Up/Down)   — lower it: HDR keeps core vs mid distinct, LDR collapses them",
                    self.exposure
                ),
                Vec2::new(20.0, WIN_H as f32 - 34.0),
                14.0,
                [160, 255, 170, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "HdrDemo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — HDR render target".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.02, 0.02, 0.03, 1.0],
    });

    // Two render targets framing the same scene: one HDR float, one default 8-bit.
    app.create_render_target_with_format("hdr", RT_W, RT_H, wgpu::TextureFormat::Rgba16Float);
    app.create_render_target("ldr", RT_W, RT_H); // surface format (Rgba8UnormSrgb)

    // Over-bright off-screen scene (layer 0): dim bg, mid-bright square, very bright core.
    spawn_quad(
        &mut app,
        Vec2::ZERO,
        Vec2::new(320.0, 280.0),
        [0.5, 0.5, 0.6, 1.0],
        0.0,
        0,
    );
    spawn_quad(
        &mut app,
        Vec2::ZERO,
        Vec2::new(150.0, 150.0),
        [2.5, 2.4, 1.0, 1.0],
        1.0,
        0,
    );
    spawn_quad(
        &mut app,
        Vec2::ZERO,
        Vec2::new(64.0, 64.0),
        [6.0, 5.8, 5.0, 1.0],
        2.0,
        0,
    );

    // An OffscreenCamera per target, both framing the scene (layer 0 only).
    for target in ["hdr", "ldr"] {
        let oc = app.world.spawn();
        app.world.add_component(
            oc,
            OffscreenCamera {
                target: target.to_string(),
                // Camera `position` is the view's TOP-LEFT corner — offset by half the RT size so the
                // scene's center sits in the middle of the captured frame.
                camera: Camera::new(SCENE - Vec2::new(RT_W as f32 / 2.0, RT_H as f32 / 2.0), 1.0),
                layer_mask: 1 << 0,
            },
        );
    }

    // Two on-screen monitors (layer 1) displaying the targets, side by side.
    // Main camera position = view top-left = world origin, so the view is [0,WIN_W] x [0,WIN_H].
    // Monitors are centered at these world points (sprites are centered at their Transform position).
    let hdr_monitor = spawn_monitor(
        &mut app,
        Vec2::new(210.0, 215.0),
        Vec2::new(370.0, 300.0),
        "hdr",
    );
    let ldr_monitor = spawn_monitor(
        &mut app,
        Vec2::new(610.0, 215.0),
        Vec2::new(370.0, 300.0),
        "ldr",
    );

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(HdrDemo {
        hdr_monitor,
        ldr_monitor,
        exposure: 0.5,
    });
    app.run();
}
