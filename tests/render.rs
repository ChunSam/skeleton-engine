//! GPU render-path tests — render the **real** render path headlessly (no window, no surface, no
//! display) via [`engine::App::screenshot_headless`], read the frame back, and assert
//! **renderer-tolerant** structural invariants on the RGBA pixels.
//!
//! ## Why this file exists
//! CI is ubuntu-only with no GPU, so before this the engine's GPU passes (sprite / text / lighting /
//! letterbox) were exercised *only* by local macOS shell smokes — a shader, pipeline, or projection
//! regression could pass all of CI and ship. These tests run on a software GPU (Mesa **lavapipe**) in
//! the `render` CI job, and on a real GPU locally, so the render path is finally CI-verified.
//!
//! ## Renderer-tolerant, not pixel-exact
//! A PNG golden rendered on macOS-Metal can never byte-match ubuntu-lavapipe, and a lavapipe-only
//! golden drifts with the runner's Mesa version. So every assertion here is **relative to a sampled
//! background pixel** with generous thresholds — never an absolute RGB value (sRGB store + FP/AA
//! differ ±1 LSB across backends). They still catch real regressions: wrong color, a dropped pass, a
//! blank frame, a broken letterbox.
//!
//! ## Skip vs require
//! [`render_or_skip`] probes for a GPU adapter. With none present it prints a `SKIP` marker and the
//! test no-ops (so the suite is green on a GPU-less box and in the GPU-less `test` job). The `render`
//! CI job sets **`SKELETON_REQUIRE_GPU=1`**, which turns "no adapter" into a panic — so CI can never
//! silently no-op green. On success it prints `[render-test] adapter=…` so the CI guard can confirm a
//! real render happened.

#![cfg(not(target_arch = "wasm32"))]

use engine::renderer::GpuContext;
use engine::{
    AmbientLight, App, Camera, Color, DesignResolution, DrawRect, DrawText, FontData,
    LightingConfig, PointLight, Sprite, System, TextQueue, Transform, UiQueue, Vec2, WindowConfig,
    World,
};

/// Prefix the CI silent-skip guard greps for (`grep -q '\[render-test\] adapter='`).
const MARKER: &str = "[render-test]";

/// Probe for a GPU adapter, then render `frames` headlessly — or skip when there is no GPU.
///
/// Returns `(width, height, rgba)`. When `GpuContext::new_headless` finds no adapter this prints a
/// `SKIP` marker and returns `None`, **unless** `SKELETON_REQUIRE_GPU=1` is set, in which case it
/// panics (the CI `render` job sets it, so a missing/unselected adapter is a hard failure there).
/// On success it prints the `[render-test] adapter=…` marker and drives frames through the public
/// [`App::screenshot_headless`] (its internals are crate-private, so we cannot reimplement it — the
/// probe builds a throwaway context purely to detect adapter availability).
fn render_or_skip(app: &mut App, frames: u32) -> Option<(u32, u32, Vec<u8>)> {
    match pollster::block_on(GpuContext::new_headless(4, 4)) {
        Ok(ctx) => {
            let info = ctx.adapter.get_info();
            println!(
                "{MARKER} adapter={} backend={:?} type={:?}",
                info.name, info.backend, info.device_type
            );
            drop(ctx);
            Some(app.screenshot_headless(frames))
        }
        Err(e) => {
            if std::env::var("SKELETON_REQUIRE_GPU").as_deref() == Ok("1") {
                panic!("{MARKER} SKELETON_REQUIRE_GPU=1 set but no GPU adapter found: {e}");
            }
            println!("{MARKER} SKIP: no GPU adapter ({e})");
            None
        }
    }
}

/// The RGB of the pixel at `(x, y)` in a tightly-packed RGBA8 buffer (row stride `w * 4`).
fn px_rgb(buf: &[u8], w: u32, x: u32, y: u32) -> [u8; 3] {
    let i = ((y * w + x) * 4) as usize;
    [buf[i], buf[i + 1], buf[i + 2]]
}

/// The mean RGB over the inclusive pixel rectangle `[x0,x1] × [y0,y1]`.
fn region_mean(buf: &[u8], w: u32, x0: u32, y0: u32, x1: u32, y1: u32) -> [f32; 3] {
    let (mut r, mut g, mut b, mut n) = (0u64, 0u64, 0u64, 0u64);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = px_rgb(buf, w, x, y);
            r += p[0] as u64;
            g += p[1] as u64;
            b += p[2] as u64;
            n += 1;
        }
    }
    let n = n.max(1) as f32;
    [r as f32 / n, g as f32 / n, b as f32 / n]
}

/// Count pixels whose any channel differs from `bg` by more than `thresh` — i.e. "not background".
fn count_far_from(buf: &[u8], bg: [u8; 3], thresh: i32) -> usize {
    buf.chunks_exact(4)
        .filter(|p| {
            (p[0] as i32 - bg[0] as i32).abs() > thresh
                || (p[1] as i32 - bg[1] as i32).abs() > thresh
                || (p[2] as i32 - bg[2] as i32).abs() > thresh
        })
        .count()
}

/// Spawn a colored quad of pixel size `scale`, centered at `pos`.
fn spawn_quad(app: &mut App, pos: Vec2, scale: Vec2, sprite: Sprite) {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: pos,
            scale,
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(e, sprite);
}

// ── Sprite path ──────────────────────────────────────────────────────────────────────────────

/// A full-saturation red quad renders red in the center and leaves the (non-red) background in the
/// corners — proves the sprite pass draws the right color in the right place.
#[test]
fn red_quad_reads_red() {
    let mut app = App::new();
    let (w, h) = (256u32, 256u32);
    app.world.insert_resource(WindowConfig {
        title: "render-test: red quad".into(),
        width: w,
        height: h,
        // Dark, deliberately NOT red, so "red-dominant" is unambiguous.
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    spawn_quad(
        &mut app,
        Vec2::new(w as f32 / 2.0, h as f32 / 2.0),
        Vec2::splat(120.0),
        Sprite::colored(1.0, 0.0, 0.0),
    );

    let Some((rw, rh, px)) = render_or_skip(&mut app, 4) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    // Center should read strongly red (relative — R clearly dominates G and B).
    let c = region_mean(&px, rw, rw / 2 - 20, rh / 2 - 20, rw / 2 + 20, rh / 2 + 20);
    assert!(c[0] > 120.0, "center R too low (quad not drawn?): {c:?}");
    assert!(
        c[0] - c[1] > 60.0 && c[0] - c[2] > 60.0,
        "center is not red-dominant: {c:?}"
    );

    // A corner is the background — must NOT be red-dominant (the quad didn't bleed everywhere, and
    // the clear color survived).
    let bg = px_rgb(&px, rw, 3, 3);
    let red_dominant = bg[0] as i32 - bg[1] as i32 > 60 && bg[0] as i32 - bg[2] as i32 > 60;
    assert!(!red_dominant, "corner unexpectedly red: {bg:?}");
}

// ── Text path ────────────────────────────────────────────────────────────────────────────────

/// Pushes HUD text every frame so the screenshot exercises the glyph pipeline.
struct TextHud;
impl System for TextHud {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "RENDER TEST",
                Vec2::new(20.0, 30.0),
                48.0,
                [240, 240, 240, 255],
            ));
            tq.push(DrawText::new(
                "the quick brown fox 0123456789",
                Vec2::new(20.0, 120.0),
                28.0,
                [200, 220, 255, 255],
            ));
        }
    }
    fn name(&self) -> &'static str {
        "TextHud"
    }
}

/// Text renders to a non-blank frame. Injects the bundled DejaVu Sans so the result does not depend
/// on the CI runner's (sparse) system fonts — on native, an empty `FontData` falls back to system
/// fonts, which a lean runner may lack.
#[test]
fn hud_text_non_blank() {
    let mut app = App::new();
    let (w, h) = (480u32, 240u32);
    app.world.insert_resource(WindowConfig {
        title: "render-test: text".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.05, 0.07, 1.0],
    });
    app.world.insert_resource(FontData(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/DejaVuSans.ttf"
        ))
        .to_vec(),
    ));
    app.add_system(TextHud);

    let Some((rw, rh, px)) = render_or_skip(&mut app, 4) else {
        return;
    };
    // Sample the bottom-left corner for the background — the text sits along the top.
    let bg = px_rgb(&px, rw, 2, rh - 3);
    let non_bg = count_far_from(&px, bg, 40);
    println!("{MARKER} text non_bg={non_bg}");
    assert!(
        non_bg > 200,
        "HUD text appears blank (non_bg={non_bg}); did the glyph pass run?"
    );
}

// ── Lighting path ────────────────────────────────────────────────────────────────────────────

/// Build the lighting scene used by both halves of the cap test: a gray floor lit by a grid of
/// white point lights, with the point-light cap set to `max_lights`.
fn build_lit_scene(max_lights: usize) -> App {
    let mut app = App::new();
    let (w, h) = (480u32, 320u32);
    app.world.insert_resource(WindowConfig {
        title: "render-test: lighting".into(),
        width: w,
        height: h,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    });

    // Gray floor tiles so the lights have a surface to reveal (lighting multiplies the scene).
    let tile = 64.0;
    let cols = (w as f32 / tile).ceil() as i32;
    let rows = (h as f32 / tile).ceil() as i32;
    for r in 0..rows {
        for c in 0..cols {
            let e = app.world.spawn();
            app.world.add_component(
                e,
                Transform {
                    position: Vec2::new(c as f32 * tile + tile * 0.5, r as f32 * tile + tile * 0.5),
                    scale: Vec2::splat(tile - 2.0),
                    rotation: 0.0,
                    z: -1.0,
                },
            );
            app.world.add_component(e, Sprite::colored(0.5, 0.5, 0.5));
        }
    }

    // A 6×5 = 30-light grid. With the cap at 8 only the 8 nearest the center stay lit; at 30 all of
    // them do — so the lit area must grow substantially.
    const COLS: usize = 6;
    const ROWS: usize = 5;
    let margin_x = 60.0;
    let margin_y = 50.0;
    let span_x = w as f32 - 2.0 * margin_x;
    let span_y = h as f32 - 2.0 * margin_y;
    for row in 0..ROWS {
        for col in 0..COLS {
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
                    color: Color::rgb(1.0, 1.0, 1.0),
                    radius: 70.0,
                    intensity: 1.35,
                    light_height: 0.5,
                },
            );
        }
    }

    app.world.insert_resource(AmbientLight {
        color: Color::rgb(0.5, 0.55, 0.75),
        intensity: 0.06,
    });
    app.world.insert_resource(LightingConfig { max_lights });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app
}

/// Count pixels brighter than the near-black unlit floor — i.e. lit by a point light.
fn count_lit(px: &[u8]) -> usize {
    px.chunks_exact(4)
        .filter(|p| p[0].max(p[1]).max(p[2]) > 60)
        .count()
}

/// Raising `LightingConfig::max_lights` lights up substantially more of the scene — proves the cap
/// actually drives how many point lights the GPU pass renders (the old hardcoded 16 could not).
#[test]
fn lighting_cap_lights_more_when_raised() {
    let mut low = build_lit_scene(8);
    let Some((_, _, low_px)) = render_or_skip(&mut low, 4) else {
        return;
    };
    // The first render succeeded, so an adapter exists — the second must render too.
    let mut high = build_lit_scene(30);
    let (_, _, high_px) = render_or_skip(&mut high, 4).expect("adapter present after first render");

    let lit_low = count_lit(&low_px);
    let lit_high = count_lit(&high_px);
    println!("{MARKER} lit_low(cap8)={lit_low} lit_high(cap30)={lit_high}");

    assert!(lit_low > 2000, "cap-8 frame looks unlit: {lit_low}");
    // 30 lights must light ≥1.5× the area of 8 (a wide margin absorbs cross-backend FP differences).
    assert!(
        lit_high * 100 >= lit_low * 150,
        "raising the cap did not light substantially more (8: {lit_low}, 30: {lit_high})"
    );
}

// ── Letterbox / DesignResolution path ──────────────────────────────────────────────────────────

/// Fills the whole design canvas with a bright rect each frame (UI primitives are authored in design
/// space and letterboxed, exactly like the `design_resolution` example).
struct DesignFill;
impl System for DesignFill {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            ui.push(DrawRect::new(0.0, 0.0, 800.0, 400.0, [60, 160, 240, 255]));
        }
    }
    fn name(&self) -> &'static str {
        "DesignFill"
    }
}

/// A 2:1 design canvas in a 1:1 window letterboxes to a centered band with top/bottom bars in the
/// clear color — proves the design-resolution scale+center projection runs end-to-end.
#[test]
fn design_resolution_letterboxes() {
    let mut app = App::new();
    let (w, h) = (600u32, 600u32);
    app.world.insert_resource(WindowConfig {
        title: "render-test: letterbox".into(),
        width: w,
        height: h,
        // The letterbox bars are the clear color — dark.
        clear_color: [0.01, 0.01, 0.02, 1.0],
    });
    // Design 800×400 (2:1) into a 600×600 (1:1) window → scale 0.75 → content 600×300, centered →
    // 150px bars top and bottom.
    app.world
        .insert_resource(DesignResolution::new(800.0, 400.0));
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.add_system(DesignFill);

    let Some((rw, rh, px)) = render_or_skip(&mut app, 3) else {
        return;
    };

    let brightness = |c: [u8; 3]| c[0] as i32 + c[1] as i32 + c[2] as i32;
    let top = px_rgb(&px, rw, rw / 2, 4); // top bar (clear color)
    let mid = px_rgb(&px, rw, rw / 2, rh / 2); // center (bright content)
    let bot = px_rgb(&px, rw, rw / 2, rh - 5); // bottom bar (clear color)

    // The content band is far brighter than the bars.
    assert!(
        brightness(mid) > brightness(top) + 150,
        "center is not brighter than the top bar (letterbox/projection broken?): mid={mid:?} top={top:?}"
    );
    // The top region is the dark clear-color bar, not content that filled the whole frame.
    assert!(
        brightness(top) < 120,
        "top region is not a dark letterbox bar: {top:?}"
    );
    // Top and bottom bars match — a symmetric, centered letterbox.
    assert!(
        (brightness(top) - brightness(bot)).abs() < 60,
        "top/bottom bars differ (letterbox not centered): top={top:?} bot={bot:?}"
    );
}

/// Like [`render_or_skip`] but drives the **editor overlay** via
/// [`App::screenshot_editor_headless_rgba`] — verifies the headless egui-editor path on the
/// GPU-less runner (lavapipe) the same way the game-render tests do.
fn editor_render_or_skip(app: &mut App, frames: u32) -> Option<(u32, u32, Vec<u8>)> {
    match pollster::block_on(GpuContext::new_headless(4, 4)) {
        Ok(ctx) => {
            let info = ctx.adapter.get_info();
            println!(
                "{MARKER} adapter={} backend={:?} type={:?}",
                info.name, info.backend, info.device_type
            );
            drop(ctx);
            Some(app.screenshot_editor_headless_rgba(frames))
        }
        Err(e) => {
            if std::env::var("SKELETON_REQUIRE_GPU").as_deref() == Ok("1") {
                panic!("{MARKER} SKELETON_REQUIRE_GPU=1 set but no GPU adapter found: {e}");
            }
            println!("{MARKER} SKIP: no GPU adapter ({e})");
            None
        }
    }
}

/// The headless **editor** screenshot path draws the egui editor overlay (here the keyboard-shortcuts
/// cheatsheet) onto the offscreen texture with no window. Over a dark clear color the cheatsheet's
/// light text/panel must produce pixels far brighter than the background — if the editor egui never
/// rendered, the whole frame stays at the dark clear color. Position-independent (scans for the
/// brightest pixel) so it doesn't depend on egui's exact window placement.
#[test]
fn editor_overlay_renders_headless() {
    let (w, h) = (520u32, 380u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "editor headless".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.06, 0.09, 1.0], // dark, so the lighter editor UI contrasts sharply
    });
    app.set_editor_shortcuts_visible(true);

    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 3) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    // Brightest pixel in the frame: with only a dark clear color in the scene, anything bright is
    // editor UI (the cheatsheet's light text/title). Clear-color luma ≈ 0.05+0.06+0.09 in 8-bit
    // terms is tiny; the editor text pushes some pixel's R+G+B well past 400/765.
    let mut max_luma = 0u32;
    for y in 0..rh {
        for x in 0..rw {
            let p = px_rgb(&px, rw, x, y);
            let l = p[0] as u32 + p[1] as u32 + p[2] as u32;
            if l > max_luma {
                max_luma = l;
            }
        }
    }
    assert!(
        max_luma > 400,
        "no bright editor-UI pixels — the headless editor overlay did not render: max_luma={max_luma}"
    );
}
