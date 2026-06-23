//! Offscreen **`ShaderMaterial`** into non-surface render targets — dogfoods the format-matched
//! material pipeline (material pipelines are now keyed by `(frag-source hash, target format)`).
//!
//! A single animated-plasma `ShaderMaterial` quad lives off-screen (layer 0) and is captured by two
//! `OffscreenCamera`s into two render targets framing it:
//! * **HDR** — `Rgba16Float` (a non-surface format), and
//! * **LDR** — the default surface format.
//!
//! Both monitors show the animated material. Before this feature the material pipeline was compiled
//! for the surface format only, so a material drawn into the HDR target was **skipped** (the HDR
//! monitor would be blank) — now each target format gets its own material pipeline, so both render.
//!
//! Keys: `Esc` quit. Native-only (render targets are native).
//!
//! Run: `cargo run --example offscreen_material`

use engine::{
    App, Camera, DrawText, Entity, InputState, KeyCode, OffscreenCamera, RenderLayer,
    ShaderMaterial, ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 440;
const RT_W: u32 = 320;
const RT_H: u32 = 300;
/// Off-screen scene center (only ever seen through the render targets).
const SCENE: Vec2 = Vec2::new(4000.0, 1500.0);

/// Animated plasma — a custom fragment shader (same interface as the `shader_material` example).
const PLASMA: &str = r#"
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;
@group(2) @binding(0) var<uniform> params: vec4<f32>; // params[0] = elapsed time

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = params[0];
    let u = in.uv.x;
    let v = in.uv.y;
    let a = sin(u * 10.0 + t * 2.0);
    let b = sin(v * 10.0 + t * 1.5);
    let c = sin((u + v) * 8.0 + t * 1.8);
    let d = sin(sqrt((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)) * 18.0 - t * 2.5);
    let s = (a + b + c + d) * 0.25;
    let rgb = vec3<f32>(0.5 + 0.5 * sin(s * 3.14159 + 0.0),
                        0.5 + 0.5 * sin(s * 3.14159 + 2.094),
                        0.5 + 0.5 * sin(s * 3.14159 + 4.188));
    return vec4<f32>(rgb, 1.0) * in.color;
}
"#;

fn spawn_monitor(app: &mut App, pos: Vec2, size: Vec2, rt: &str) {
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
            texture: Some(rt.into()),
            ..Default::default()
        },
    );
    app.world.add_component(e, RenderLayer(1)); // layer 1 = not captured by the OffscreenCameras
}

struct Demo {
    material: Entity,
    elapsed: f32,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.elapsed += dt;
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                    sq.0 = true;
                }
            }
        }
        if let Some(mat) = world.get_mut::<ShaderMaterial>(self.material) {
            mat.params[0] = self.elapsed;
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Offscreen ShaderMaterial → format-matched render targets (same material, two formats)",
                Vec2::new(20.0, 16.0),
                15.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                "HDR  (Rgba16Float)",
                Vec2::new(110.0, 52.0),
                14.0,
                [150, 230, 255, 255],
            ));
            tq.push(DrawText::new(
                "LDR  (surface)",
                Vec2::new(480.0, 52.0),
                14.0,
                [255, 200, 150, 255],
            ));
            tq.push(DrawText::new(
                "Both monitors render the material — its pipeline is built per target format.  Esc quit",
                Vec2::new(20.0, WIN_H as f32 - 28.0),
                13.0,
                [170, 190, 210, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "OffscreenMaterialDemo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — offscreen material format-matching".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.02, 0.02, 0.03, 1.0],
    });

    app.create_render_target_with_format("hdr", RT_W, RT_H, wgpu::TextureFormat::Rgba16Float);
    app.create_render_target("ldr", RT_W, RT_H); // surface format

    // The off-screen material quad (layer 0), centered on SCENE.
    let material = app.world.spawn();
    app.world.add_component(
        material,
        Transform {
            position: SCENE,
            scale: Vec2::new(300.0, 280.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world
        .add_component(material, Sprite::colored(1.0, 1.0, 1.0));
    app.world
        .add_component(material, ShaderMaterial::new(PLASMA, [0.0; 4]));
    app.world.add_component(material, RenderLayer(0));

    // An OffscreenCamera per target, both framing the scene (layer 0 only).
    for target in ["hdr", "ldr"] {
        let oc = app.world.spawn();
        app.world.add_component(
            oc,
            OffscreenCamera {
                target: target.to_string(),
                // Camera position is the view's TOP-LEFT corner — offset by half the RT.
                camera: Camera::new(SCENE - Vec2::new(RT_W as f32 / 2.0, RT_H as f32 / 2.0), 1.0),
                layer_mask: 1 << 0,
            },
        );
    }

    // Two monitors (layer 1) showing the targets side by side.
    spawn_monitor(
        &mut app,
        Vec2::new(195.0, 230.0),
        Vec2::new(340.0, 300.0),
        "hdr",
    );
    spawn_monitor(
        &mut app,
        Vec2::new(565.0, 230.0),
        Vec2::new(340.0, 300.0),
        "ldr",
    );

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(Demo {
        material,
        elapsed: 0.0,
    });
    app.run();
}
