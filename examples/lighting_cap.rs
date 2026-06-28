//! `LightingConfig` — the configurable point-light cap.
//!
//! The 2D point-light pass renders at most `LightingConfig::max_lights` lights per frame
//! (the nearest to the camera; distant ones are dropped, not arbitrary ones). Before this
//! knob the cap was hardcoded to **16** — a hard limit a fork hit with no way to raise it.
//!
//! This demo lays out a grid of **40 colored point lights** over a dark gray floor:
//!
//! - With the cap **raised to 40** (`LightingConfig { max_lights: 40 }`, the default state
//!   here) every light renders → 40 colored pools.
//! - Press **SPACE** to drop the cap back to the historical **16**: only the 16 lights
//!   nearest the viewport center stay lit; the outer 24 pools go dark. The lighting renderer
//!   rebuilds its shader + uniform on the change.
//!
//! Press **Esc** to quit. Native-only (lighting is a no-op on wasm32).
//!
//! Headless: `HEADLESS_SHOT=/tmp/lights.png cargo run --example lighting_cap` renders the
//! raised-cap state (all 40 lit) to a PNG with no window — works with the monitor off.
//! `LIGHTING_CAP=16` overrides the initial cap (used by the smoke script to A/B 16 vs 40).
use engine::{
    AmbientLight, App, Camera, Color, DrawText, InputState, KeyCode, LightingConfig, PointLight,
    ShouldQuit, Sprite, System, TextQueue, Transform, Vec2, WindowConfig, World,
    DEFAULT_MAX_LIGHTS,
};

const WIN_W: u32 = 960;
const WIN_H: u32 = 640;

// 8 columns × 5 rows = 40 point lights.
const COLS: usize = 8;
const ROWS: usize = 5;
const LIGHT_COUNT: usize = COLS * ROWS;

// Cap states the SPACE key toggles between.
const RAISED_CAP: usize = LIGHT_COUNT; // 40 — all lights
const LOW_CAP: usize = DEFAULT_MAX_LIGHTS; // 16 — the old hardcoded limit

const FLOOR_TILE: f32 = 64.0;
const LIGHT_RADIUS: f32 = 95.0;
const LIGHT_INTENSITY: f32 = 1.35;

/// Demo state: which cap is currently active.
struct Demo {
    cap_raised: bool,
}

/// Evenly-spaced hue → RGB (full saturation/value), so the 40 lights are visually distinct.
fn hue_rgb(t: f32) -> Color {
    let h = (t.fract() * 6.0).clamp(0.0, 6.0);
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (r, g, b) = match i {
        0 => (1.0, f, 0.0),
        1 => (1.0 - f, 1.0, 0.0),
        2 => (0.0, 1.0, f),
        3 => (0.0, 1.0 - f, 1.0),
        4 => (f, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - f),
    };
    Color::rgb(r, g, b)
}

/// SPACE toggles the cap; Esc quits; draws the HUD.
struct CapSystem;

impl System for CapSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (quit, toggle) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Escape),
                    i.just_pressed(KeyCode::Space),
                )
            })
            .unwrap_or((false, false));

        if quit {
            if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                sq.0 = true;
            }
        }

        if toggle {
            let raised = {
                let d = world.resource_mut::<Demo>().unwrap();
                d.cap_raised = !d.cap_raised;
                d.cap_raised
            };
            let cap = if raised { RAISED_CAP } else { LOW_CAP };
            if let Some(cfg) = world.resource_mut::<LightingConfig>() {
                cfg.max_lights = cap;
            }
        }

        let cap = if world
            .resource::<Demo>()
            .map(|d| d.cap_raised)
            .unwrap_or(true)
        {
            RAISED_CAP
        } else {
            LOW_CAP
        };

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "LightingConfig::max_lights — configurable point-light cap",
                Vec2::new(14.0, 12.0),
                17.0,
                [235, 230, 210, 255],
            ));
            let lit = cap.min(LIGHT_COUNT);
            tq.push(DrawText::new(
                format!(
                    "{LIGHT_COUNT} point lights · cap {cap} → {lit} lit   (SPACE: toggle {RAISED_CAP} ↔ {LOW_CAP},  Esc: quit)"
                ),
                Vec2::new(14.0, WIN_H as f32 - 28.0),
                15.0,
                [200, 215, 235, 255],
            ));
        }
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "LightingConfig — point-light cap".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    });

    // Dark gray floor tiles fill the viewport so the lights have a surface to reveal
    // (lighting multiplies the scene; pure-black areas would stay black).
    let cols = (WIN_W as f32 / FLOOR_TILE).ceil() as i32;
    let rows = (WIN_H as f32 / FLOOR_TILE).ceil() as i32;
    for r in 0..rows {
        for c in 0..cols {
            let e = app.world.spawn();
            app.world.add_component(
                e,
                Transform {
                    position: Vec2::new(
                        c as f32 * FLOOR_TILE + FLOOR_TILE * 0.5,
                        r as f32 * FLOOR_TILE + FLOOR_TILE * 0.5,
                    ),
                    scale: Vec2::splat(FLOOR_TILE - 2.0),
                    rotation: 0.0,
                    z: -1.0,
                },
            );
            // Subtle checker so the floor reads as tiles under the colored pools.
            let shade = if (r + c) % 2 == 0 { 0.55 } else { 0.48 };
            app.world
                .add_component(e, Sprite::colored(shade, shade, shade));
        }
    }

    // 40 colored point lights laid out on a centered grid.
    let margin_x = 110.0;
    let margin_y = 95.0;
    let span_x = WIN_W as f32 - 2.0 * margin_x;
    let span_y = WIN_H as f32 - 2.0 * margin_y;
    for row in 0..ROWS {
        for col in 0..COLS {
            let idx = row * COLS + col;
            let x = margin_x + span_x * col as f32 / (COLS as f32 - 1.0);
            let y = margin_y + span_y * row as f32 / (ROWS as f32 - 1.0);
            let e = app.world.spawn();
            app.world.add_component(
                e,
                Transform {
                    position: Vec2::new(x, y),
                    scale: Vec2::splat(1.0),
                    rotation: 0.0,
                    z: 0.0,
                },
            );
            app.world.add_component(
                e,
                PointLight {
                    color: hue_rgb(idx as f32 / LIGHT_COUNT as f32),
                    radius: LIGHT_RADIUS,
                    intensity: LIGHT_INTENSITY,
                    light_height: 0.5,
                },
            );
        }
    }

    // Low ambient so unlit areas read as dark; presence of AmbientLight enables the pass.
    app.world.insert_resource(AmbientLight {
        color: Color::rgb(0.5, 0.55, 0.75),
        intensity: 0.06,
    });

    // Raise the cap so all 40 lights render. Without this resource the engine default (16)
    // applies and only the 16 nearest the camera would light up. `LIGHTING_CAP` overrides
    // the starting cap so the headless smoke can capture both the 16- and 40-light states.
    let initial_cap = std::env::var("LIGHTING_CAP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(RAISED_CAP);
    app.world.insert_resource(LightingConfig {
        max_lights: initial_cap,
    });

    app.world.insert_resource(Demo {
        cap_raised: initial_cap >= RAISED_CAP,
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(CapSystem);

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit. Self-reports the count
    // of *lit* pixels (brighter than the dark floor) so the smoke script can assert that the
    // 40-light cap lights up substantially more of the frame than the 16-light cap.
    if let Ok(path) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        let (w, h, pixels) = app.screenshot_headless(frames);
        // The unlit floor sits near black (floor shade × 0.06 ambient ≈ 8/255); a lit pool is
        // far brighter. Count pixels whose brightest channel clears that floor by a wide margin.
        let lit = pixels
            .chunks_exact(4)
            .filter(|p| p[0].max(p[1]).max(p[2]) > 60)
            .count();
        image::RgbaImage::from_raw(w, h, pixels)
            .expect("readback byte count")
            .save(&path)
            .expect("save png");
        println!("LIGHTING_CAP: cap={initial_cap} wrote {w}x{h} PNG to {path}");
        println!("LIGHTING_CAP: lit_pixels={lit}");
        return;
    }
    app.run();
}
