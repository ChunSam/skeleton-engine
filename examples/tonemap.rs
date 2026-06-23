//! Engine **HDR tone-mapping** in the post-process pass — dogfoods
//! [`PostProcessConfig`](engine::PostProcessConfig)'s new `hdr` / `tonemap` / `exposure` fields and
//! the [`Tonemap`](engine::Tonemap) operators.
//!
//! A row of squares of the **same warm hue** but increasing brightness (the rightmost are well over
//! `1.0`) is drawn straight to the main camera. With `hdr: true` the scene is rendered into an
//! `Rgba16Float` intermediate (so the `> 1.0` values survive), then the post-process pass applies the
//! chosen tone-map operator + exposure to bring it back to the display:
//!
//! * **None** — the raw colour is clamped: every over-bright square saturates to the *same* flat
//!   white, the highlight detail is gone.
//! * **Reinhard / ACES** — the highlights *roll off* smoothly, so the bright squares stay distinct.
//!
//! Toggle `hdr` off to see why the HDR intermediate matters: the scene is then drawn into the 8-bit
//! surface intermediate, which clamps the `> 1.0` values at store time, so even ACES has nothing left
//! to roll off (it looks like None).
//!
//! Keys: `1` None · `2` Reinhard · `3` ACES · `↑`/`↓` exposure · `H` toggle HDR · `Esc` quit.
//!
//! Run: `cargo run --example tonemap`

use engine::{
    App, Camera, Color, DrawText, InputState, KeyCode, PostProcessConfig, ShouldQuit, Sprite,
    System, TextQueue, Tonemap, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 860;
const WIN_H: u32 = 420;

/// Brightness multipliers for the row of swatches. Values > 1.0 are "over-bright" — only an HDR
/// intermediate keeps them distinct through the post-process pass.
const BRIGHTNESS: [f32; 7] = [0.35, 0.7, 1.2, 2.0, 3.5, 6.0, 12.0];

fn spawn_swatch(app: &mut App, x: f32, brightness: f32) {
    // Same warm hue, scaled — so the only difference between swatches is intensity.
    let c = [brightness, brightness * 0.82, brightness * 0.55, 1.0];
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(x, 200.0),
            scale: Vec2::new(96.0, 200.0),
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

fn tonemap_name(t: Tonemap) -> &'static str {
    match t {
        Tonemap::None => "None (clamp)",
        Tonemap::Reinhard => "Reinhard",
        Tonemap::AcesFilmic => "ACES filmic",
        _ => "?",
    }
}

struct TonemapDemo;

impl System for TonemapDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (none, reinhard, aces, up, down, toggle_hdr, quit) = {
            let Some(input) = world.resource::<InputState>() else {
                return;
            };
            (
                input.just_pressed(KeyCode::Digit1),
                input.just_pressed(KeyCode::Digit2),
                input.just_pressed(KeyCode::Digit3),
                input.just_pressed(KeyCode::ArrowUp),
                input.just_pressed(KeyCode::ArrowDown),
                input.just_pressed(KeyCode::KeyH),
                input.just_pressed(KeyCode::Escape),
            )
        };

        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        let (tonemap, exposure, hdr) = {
            let Some(pp) = world.resource_mut::<PostProcessConfig>() else {
                return;
            };
            if none {
                pp.tonemap = Tonemap::None;
            }
            if reinhard {
                pp.tonemap = Tonemap::Reinhard;
            }
            if aces {
                pp.tonemap = Tonemap::AcesFilmic;
            }
            if up {
                pp.exposure = (pp.exposure + 0.1).min(4.0);
            }
            if down {
                pp.exposure = (pp.exposure - 0.1).max(0.1);
            }
            if toggle_hdr {
                pp.hdr = !pp.hdr;
            }
            (pp.tonemap, pp.exposure, pp.hdr)
        };

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Engine HDR tone-mapping (post-process) — same hue, increasing brightness (rightmost >> 1.0)",
                Vec2::new(20.0, 16.0),
                16.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                format!(
                    "tonemap: {}    exposure: {:.1}    HDR intermediate: {}",
                    tonemap_name(tonemap),
                    exposure,
                    if hdr {
                        "ON (Rgba16Float)"
                    } else {
                        "off (8-bit clamps)"
                    },
                ),
                Vec2::new(20.0, WIN_H as f32 - 52.0),
                15.0,
                [160, 255, 170, 255],
            ));
            tq.push(DrawText::new(
                "1 None  2 Reinhard  3 ACES   Up/Down exposure   H toggle HDR   Esc quit",
                Vec2::new(20.0, WIN_H as f32 - 28.0),
                13.0,
                [170, 190, 210, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "TonemapDemo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — HDR tonemap".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.02, 0.02, 0.03, 1.0],
    });

    // Row of swatches: same warm hue, increasing brightness. Spread across the window width.
    let n = BRIGHTNESS.len();
    let gap = WIN_W as f32 / (n as f32 + 1.0);
    for (i, b) in BRIGHTNESS.iter().enumerate() {
        spawn_swatch(&mut app, gap * (i as f32 + 1.0), *b);
    }

    // Activate the post-process pass with HDR + ACES tone-mapping.
    app.world.insert_resource(PostProcessConfig {
        enabled: true,
        hdr: true,
        tonemap: Tonemap::AcesFilmic,
        exposure: 1.0,
        // No vignette/chroma/grade — keep the comparison about tone-mapping only.
        vignette_strength: 0.0,
        chroma_offset: 0.0,
        bloom_intensity: 0.0,
        ..Default::default()
    });

    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(TonemapDemo);
    app.run();
}
