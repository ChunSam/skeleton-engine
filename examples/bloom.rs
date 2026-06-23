//! Engine **real multi-pass bloom** — dogfoods [`PostProcessConfig`](engine::PostProcessConfig)'s
//! new `bloom` / `bloom_iterations` fields.
//!
//! A dark scene holds several **over-bright** emitters (colour values well above `1.0`) plus a
//! couple of **dim** swatches below the bloom threshold. With HDR on, the `> 1.0` values survive
//! into the `Rgba16Float` intermediate; the bloom pass then extracts the highlights, blurs them
//! with a separable Gaussian (ping-ponged `bloom_iterations` times), and additively composites the
//! soft glow back onto the scene before tone-mapping.
//!
//! Press **B** to toggle between the **real multi-pass bloom** (`bloom: true`) and the cheap
//! **inline 4-tap** bloom (`bloom: false`): the real pass spreads a wide, soft halo around the
//! bright emitters, while the inline version only smears a few pixels. The dim swatches never glow
//! (their luminance is below `bloom_threshold`).
//!
//! Keys: `B` toggle real/inline bloom · `↑`/`↓` intensity · `←`/`→` iterations (glow width) ·
//! `Esc` quit.
//!
//! Run: `cargo run --example bloom`

use engine::{
    App, Camera, Color, DrawText, InputState, KeyCode, PostProcessConfig, ShouldQuit, Sprite,
    System, TextQueue, Tonemap, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 880;
const WIN_H: u32 = 460;

/// (x-fraction, brightness, hue) for each emitter. Brightness > 1.0 = over-bright → only HDR keeps
/// it distinct, and only those above the threshold bloom.
const EMITTERS: [(f32, f32, [f32; 3]); 5] = [
    (0.16, 0.45, [0.9, 0.4, 0.4]), // dim red — below threshold, no glow
    (0.34, 2.6, [1.0, 0.85, 0.4]), // bright warm
    (0.52, 4.5, [0.5, 0.8, 1.0]),  // very bright cyan
    (0.70, 3.2, [0.7, 1.0, 0.6]),  // bright green
    (0.86, 0.5, [0.5, 0.5, 0.95]), // dim blue — below threshold, no glow
];

fn spawn_emitter(app: &mut App, x: f32, brightness: f32, hue: [f32; 3]) {
    let c = [
        hue[0] * brightness,
        hue[1] * brightness,
        hue[2] * brightness,
        1.0,
    ];
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(x, WIN_H as f32 * 0.45),
            scale: Vec2::new(70.0, 70.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(
        e,
        Sprite {
            color: Color::from(c),
            ..Default::default()
        },
    );
}

struct BloomDemo;

impl System for BloomDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (toggle, up, down, left, right, quit) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.just_pressed(KeyCode::KeyB),
                input.just_pressed(KeyCode::ArrowUp),
                input.just_pressed(KeyCode::ArrowDown),
                input.just_pressed(KeyCode::ArrowLeft),
                input.just_pressed(KeyCode::ArrowRight),
                input.just_pressed(KeyCode::Escape),
            )
        };

        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        let (bloom, intensity, iterations) = {
            let Some(pp) = world.resource_mut::<PostProcessConfig>() else {
                return;
            };
            if toggle {
                pp.bloom = !pp.bloom;
            }
            if up {
                pp.bloom_intensity = (pp.bloom_intensity + 0.1).min(3.0);
            }
            if down {
                pp.bloom_intensity = (pp.bloom_intensity - 0.1).max(0.0);
            }
            if right {
                pp.bloom_iterations = (pp.bloom_iterations + 1).min(8);
            }
            if left {
                pp.bloom_iterations = pp.bloom_iterations.saturating_sub(1);
            }
            (pp.bloom, pp.bloom_intensity, pp.bloom_iterations)
        };

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Engine real multi-pass bloom — over-bright emitters glow; dim ones don't",
                Vec2::new(20.0, 16.0),
                16.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                format!(
                    "bloom: {}    intensity: {:.1}    iterations: {}",
                    if bloom {
                        "REAL (multi-pass Gaussian)"
                    } else {
                        "inline 4-tap (cheap)"
                    },
                    intensity,
                    iterations,
                ),
                Vec2::new(20.0, WIN_H as f32 - 52.0),
                15.0,
                if bloom {
                    [160, 255, 170, 255]
                } else {
                    [230, 200, 140, 255]
                },
            ));
            tq.push(DrawText::new(
                "B toggle real/inline   Up/Down intensity   Left/Right iterations   Esc quit",
                Vec2::new(20.0, WIN_H as f32 - 28.0),
                13.0,
                [170, 190, 210, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "BloomDemo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — bloom".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.015, 0.015, 0.025, 1.0],
    });

    for (xf, brightness, hue) in EMITTERS {
        spawn_emitter(&mut app, WIN_W as f32 * xf, brightness, hue);
    }

    // Real bloom + HDR + ACES so the over-bright highlights survive and roll off cleanly. No
    // vignette/chroma — keep the comparison about bloom only.
    app.world.insert_resource(PostProcessConfig {
        enabled: true,
        hdr: true,
        tonemap: Tonemap::AcesFilmic,
        bloom: true,
        bloom_iterations: 4,
        bloom_threshold: 0.75,
        bloom_intensity: 1.0,
        vignette_strength: 0.0,
        chroma_offset: 0.0,
        ..Default::default()
    });

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(BloomDemo);
    app.run();
}
