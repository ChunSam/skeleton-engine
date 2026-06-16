//! `shader_material` — demonstrates custom per-entity fragment shaders via [`ShaderMaterial`].
//!
//! Three sprites are placed side by side, each with a distinct WGSL fragment shader:
//!
//! 1. **Hue cycle** — animates through the full hue wheel based on UV.x + elapsed time.
//! 2. **Plasma ripple** — classic sin-wave procedural plasma driven by UV and time.
//! 3. **Dissolve** — procedural threshold dissolve with a glowing edge; intensity
//!    controlled by ↑/↓ arrow keys.
//!
//! Controls: ↑/↓ adjust dissolve threshold | Esc quit

use engine::{
    App, DrawText, Entity, InputState, KeyCode, ShaderMaterial, ShouldQuit, Sprite, System,
    TextQueue, Transform, Vec2, WindowConfig, World,
};

// ─── WGSL shader sources ──────────────────────────────────────────────────────

/// Shader 1: Hue cycle — cycles through the hue wheel across UV.x + time.
const SHADER_HUE_CYCLE: &str = r#"
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;
@group(2) @binding(0) var<uniform> params: vec4<f32>; // params[0] = elapsed time

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn hue_to_rgb(h: f32) -> vec3<f32> {
    let h6 = fract(h) * 6.0;
    let r = clamp(abs(h6 - 3.0) - 1.0, 0.0, 1.0);
    let g = clamp(2.0 - abs(h6 - 2.0), 0.0, 1.0);
    let b = clamp(2.0 - abs(h6 - 4.0), 0.0, 1.0);
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_sprite, s_sprite, in.uv);
    let hue = fract(in.uv.x + params[0] * 0.3);
    let rgb = hue_to_rgb(hue);
    return vec4<f32>(rgb, 1.0) * in.color * tex;
}
"#;

/// Shader 2: Plasma ripple — sin-wave interference pattern animated by time.
const SHADER_PLASMA: &str = r#"
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
    let tex = textureSample(t_sprite, s_sprite, in.uv);
    let t = params[0];
    let u = in.uv.x;
    let v = in.uv.y;
    let v1 = sin(u * 10.0 + t * 2.0);
    let v2 = sin(v * 10.0 + t * 1.5);
    let v3 = sin((u + v) * 8.0 + t * 1.8);
    let v4 = sin(sqrt((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)) * 18.0 - t * 2.5);
    let plasma = (v1 + v2 + v3 + v4) * 0.25 + 0.5;
    let r = sin(plasma * 3.14159) * 0.5 + 0.5;
    let g = sin(plasma * 3.14159 + 2.094) * 0.5 + 0.5;
    let b = sin(plasma * 3.14159 + 4.189) * 0.5 + 0.5;
    return vec4<f32>(r, g, b, 1.0) * in.color * tex;
}
"#;

/// Shader 3: Dissolve — procedural noise threshold with a glowing edge.
/// params[0] = elapsed time (unused; kept for uniform layout consistency)
/// params[1] = dissolve threshold 0..1 (controlled by ↑/↓)
const SHADER_DISSOLVE: &str = r#"
@group(1) @binding(0) var t_sprite: texture_2d<f32>;
@group(1) @binding(1) var s_sprite: sampler;
@group(2) @binding(0) var<uniform> params: vec4<f32>; // params[0]=time, params[1]=threshold

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv:    vec2<f32>,
    @location(1) color: vec4<f32>,
};

fn rand(co: vec2<f32>) -> f32 {
    return fract(sin(dot(co, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

fn noise(uv: vec2<f32>) -> f32 {
    let i = floor(uv);
    let f = fract(uv);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(rand(i),                rand(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(rand(i + vec2<f32>(0.0, 1.0)), rand(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex = textureSample(t_sprite, s_sprite, in.uv);
    let threshold = params[1];
    let n = noise(in.uv * 6.0);
    if n < threshold {
        discard;
    }
    // Glowing edge near the dissolve boundary.
    let edge_width = 0.05;
    let edge = 1.0 - smoothstep(threshold, threshold + edge_width, n);
    let glow = vec4<f32>(1.0, 0.6, 0.1, 1.0) * edge;
    return (tex * in.color + glow);
}
"#;

// ─── Tags ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct HueCycleTag;

#[derive(Clone)]
struct PlasmaTag;

#[derive(Clone)]
struct DissolveTag;

// ─── System ───────────────────────────────────────────────────────────────────

struct ShaderDemoSystem {
    elapsed: f32,
    dissolve: f32,
}

impl ShaderDemoSystem {
    fn new() -> Self {
        Self {
            elapsed: 0.0,
            dissolve: 0.5,
        }
    }
}

impl System for ShaderDemoSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.elapsed += dt;

        // ── Input ────────────────────────────────────────────────────────────
        let mut should_quit = false;
        let mut dissolve_delta = 0.0_f32;

        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                should_quit = true;
            }
            if input.is_pressed(KeyCode::ArrowUp) {
                dissolve_delta += dt;
            }
            if input.is_pressed(KeyCode::ArrowDown) {
                dissolve_delta -= dt;
            }
        }

        if should_quit {
            if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                quit.quit();
            }
            return;
        }

        self.dissolve = (self.dissolve + dissolve_delta).clamp(0.0, 1.0);

        // ── Update hue-cycle material ─────────────────────────────────────
        let hue_entities: Vec<Entity> = world.query::<HueCycleTag>().map(|(e, _)| e).collect();
        for e in hue_entities {
            if let Some(mat) = world.get_mut::<ShaderMaterial>(e) {
                mat.params[0] = self.elapsed;
            }
        }

        // ── Update plasma material ────────────────────────────────────────
        let plasma_entities: Vec<Entity> = world.query::<PlasmaTag>().map(|(e, _)| e).collect();
        for e in plasma_entities {
            if let Some(mat) = world.get_mut::<ShaderMaterial>(e) {
                mat.params[0] = self.elapsed;
            }
        }

        // ── Update dissolve material ──────────────────────────────────────
        let dissolve_entities: Vec<Entity> = world.query::<DissolveTag>().map(|(e, _)| e).collect();
        for e in dissolve_entities {
            if let Some(mat) = world.get_mut::<ShaderMaterial>(e) {
                mat.params[0] = self.elapsed;
                mat.params[1] = self.dissolve;
            }
        }

        // ── Labels ────────────────────────────────────────────────────────
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        // Sprite labels (screen-space, below the sprites; window is 1100x500,
        // sprites are centered at y=220 with scale 140 so bottom edge ≈ y=290)
        tq.push(DrawText::new(
            "Hue Cycle",
            Vec2::new(183.0, 310.0),
            18.0,
            [220, 220, 255, 230],
        ));
        tq.push(DrawText::new(
            "Plasma Ripple",
            Vec2::new(490.0, 310.0),
            18.0,
            [220, 255, 220, 230],
        ));
        tq.push(DrawText::new(
            format!("Dissolve  {:.2}", self.dissolve),
            Vec2::new(800.0, 310.0),
            18.0,
            [255, 220, 180, 230],
        ));

        // Hint line
        tq.push(DrawText::new(
            "Up/Down: dissolve threshold    Esc: quit",
            Vec2::new(12.0, 470.0),
            16.0,
            [160, 170, 185, 200],
        ));
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "shader_material — custom fragment shaders".to_string(),
        width: 1100,
        height: 500,
        clear_color: [0.04, 0.04, 0.08, 1.0],
    });

    // Sprite 1: Hue cycle — left
    let e1 = app.world.spawn();
    app.world.add_component(
        e1,
        Transform {
            position: Vec2::new(230.0, 220.0),
            scale: Vec2::splat(140.0),
            ..Default::default()
        },
    );
    app.world.add_component(e1, Sprite::colored(1.0, 1.0, 1.0));
    app.world
        .add_component(e1, ShaderMaterial::new(SHADER_HUE_CYCLE, [0.0; 4]));
    app.world.add_component(e1, HueCycleTag);

    // Sprite 2: Plasma ripple — centre
    let e2 = app.world.spawn();
    app.world.add_component(
        e2,
        Transform {
            position: Vec2::new(550.0, 220.0),
            scale: Vec2::splat(140.0),
            ..Default::default()
        },
    );
    app.world.add_component(e2, Sprite::colored(1.0, 1.0, 1.0));
    app.world
        .add_component(e2, ShaderMaterial::new(SHADER_PLASMA, [0.0; 4]));
    app.world.add_component(e2, PlasmaTag);

    // Sprite 3: Dissolve — right (params[1] = initial threshold 0.5)
    let e3 = app.world.spawn();
    app.world.add_component(
        e3,
        Transform {
            position: Vec2::new(870.0, 220.0),
            scale: Vec2::splat(140.0),
            ..Default::default()
        },
    );
    app.world.add_component(e3, Sprite::colored(1.0, 1.0, 1.0));
    app.world.add_component(
        e3,
        ShaderMaterial::new(SHADER_DISSOLVE, [0.0, 0.5, 0.0, 0.0]),
    );
    app.world.add_component(e3, DissolveTag);

    app.add_system(ShaderDemoSystem::new());
    app.run();
}
