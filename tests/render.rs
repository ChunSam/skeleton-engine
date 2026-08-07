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
    spawn_floating_text, AmbientLight, App, Camera, Color, DesignResolution, DrawRect, DrawText,
    FloatingText, FloatingTextSystem, FontData, GpuParticleEmitter, LightingConfig, PointLight,
    PostProcessConfig, Sprite, System, Tag, TextQueue, Transform, UiQueue, Vec2, WindowConfig,
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

// ── GPU particle path ────────────────────────────────────────────────────────────────────────

/// GPU particles must **accumulate across frames**.
///
/// The ring write cursor into the particle buffer used to be a frame-local `let mut frame_cursor =
/// 0u32` in the render stage, so every frame restarted the ring at slot 0 and overwrote the
/// particles the previous frame had just spawned. However long a particle's `lifetime` was, the
/// buffer only ever held **one frame's** emission. Nothing caught it: until this test the only
/// GPU-particle test in the tree asserted `size_of::<GpuParticle>()`, so no automated check ever
/// *executed* this renderer — which is also how the format-matched pipeline came to be built only
/// under `if has_emitters` while the pass below it drew unconditionally.
///
/// The assertion is **self-calibrating** — the same scene is rendered for a few frames and for
/// many, and the long run must light up meaningfully more pixels. That needs no absolute pixel
/// constant and so survives the Metal/lavapipe difference. With the cursor reset per frame both
/// runs light the same handful of pixels (a fixed 2 particles, both always one frame old, clustered
/// on the emitter); with it persisted the long run holds ~9× as many, spread over a wider radius.
#[test]
fn gpu_particles_accumulate_across_frames() {
    /// Renders the same emitter scene for `frames` and returns how many pixels are clearly
    /// brighter than the near-black clear color. `None` = no GPU (skip).
    fn lit_pixels(frames: u32) -> Option<u64> {
        let mut app = App::new();
        let (w, h) = (256u32, 256u32);
        app.world.insert_resource(WindowConfig {
            title: "render-test: gpu particles".into(),
            width: w,
            height: h,
            clear_color: [0.02, 0.02, 0.03, 1.0],
        });
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(w as f32 / 2.0, h as f32 / 2.0),
                scale: Vec2::ONE,
                rotation: 0.0,
                z: 0.0,
            },
        );
        // `lifetime` far exceeds the run, so a drop in particle count can only be the cursor
        // overwriting live slots — never expiry. Zero gravity + symmetric spread fans the
        // particles out from the emitter, so older ones occupy pixels newer ones do not.
        app.world.add_component(e, GpuParticleEmitter::default());
        if let Some(em) = app.world.get_mut::<GpuParticleEmitter>(e) {
            em.spawn_rate = 120.0;
            em.lifetime = 30.0;
            em.velocity = Vec2::ZERO;
            em.velocity_spread = Vec2::splat(70.0);
            em.color_start = Color::rgb(1.0, 1.0, 1.0);
            em.color_end = Color::rgb(1.0, 1.0, 1.0);
            em.size = 4.0;
            em.gravity = Vec2::ZERO;
        }

        let (_rw, _rh, px) = render_or_skip(&mut app, frames)?;
        let mut n = 0u64;
        for i in (0..px.len()).step_by(4) {
            if px[i] as u16 + px[i + 1] as u16 + px[i + 2] as u16 > 150 {
                n += 1;
            }
        }
        Some(n)
    }

    let (Some(short), Some(long)) = (lit_pixels(5), lit_pixels(45)) else {
        return;
    };
    assert!(
        short > 0,
        "no GPU particles rendered at all in the 5-frame run — the pass did not draw"
    );
    assert!(
        long as f64 > short as f64 * 2.0,
        "GPU particles are not accumulating across frames: 5 frames lit {short} px, 45 frames lit \
         {long} px. A per-frame reset of the ring cursor makes these two counts equal."
    );
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

/// Number of pixels within `(x0,y0)..(x1,y1)` farther than `thresh` (per channel, max) from `bg`.
#[allow(clippy::too_many_arguments)]
fn region_count_far(
    buf: &[u8],
    w: u32,
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    bg: [u8; 3],
    thresh: i32,
) -> usize {
    let mut n = 0;
    for y in y0..y1 {
        for x in x0..x1 {
            let p = px_rgb(buf, w, x, y);
            let d = (0..3)
                .map(|i| (p[i] as i32 - bg[i] as i32).abs())
                .max()
                .unwrap();
            if d > thresh {
                n += 1;
            }
        }
    }
    n
}

/// Pushes the text-layering scene every frame: a layered text COVERED by a higher-z bg-colored
/// rect (must vanish), an uncovered layered control text (must render), and an on-top (z-None)
/// text over another high-z rect (must still render — the historical always-on-top path).
struct LayeredTextScene;
impl System for LayeredTextScene {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            // Covered: layered text at z 0.2 under a z 1.0 rect.
            tq.push(
                DrawText::new("COVERED", Vec2::new(40.0, 90.0), 32.0, [255, 255, 255, 255])
                    .with_z(0.2),
            );
            // Control: same z, no cover.
            tq.push(
                DrawText::new(
                    "VISIBLE",
                    Vec2::new(40.0, 170.0),
                    32.0,
                    [255, 255, 255, 255],
                )
                .with_z(0.2),
            );
            // On-top: z-None text over a high-z rect still draws (legacy behavior).
            tq.push(DrawText::new(
                "ONTOP",
                Vec2::new(300.0, 90.0),
                32.0,
                [255, 255, 255, 255],
            ));
        }
        if let Some(uq) = world.resource_mut::<UiQueue>() {
            // Background-colored covers, so any text bleeding through reads as non-bg pixels.
            uq.push(DrawRect::new(20.0, 70.0, 220.0, 70.0, [0.05, 0.05, 0.07, 1.0]).with_z(1.0));
            uq.push(DrawRect::new(280.0, 70.0, 180.0, 70.0, [0.05, 0.05, 0.07, 1.0]).with_z(50.0));
        }
    }
    fn name(&self) -> &'static str {
        "LayeredTextScene"
    }
}

/// A UI rect drawn above a layered (`DrawText::with_z`) text actually covers it, an uncovered
/// layered text still renders, and z-None text keeps drawing on top of every rect.
#[test]
fn layered_text_is_covered_by_higher_z_rect() {
    let mut app = App::new();
    let (w, h) = (480u32, 240u32);
    app.world.insert_resource(WindowConfig {
        title: "render-test: text layering".into(),
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
    app.add_system(LayeredTextScene);

    let Some((rw, _rh, px)) = render_or_skip(&mut app, 4) else {
        return;
    };
    let bg = px_rgb(&px, rw, 2, 2);

    // "COVERED" glyphs sit around (40..200, 90..130); the covering rect spans (20..240, 70..140).
    let covered = region_count_far(&px, rw, 30, 80, 230, 135, bg, 40);
    // "VISIBLE" glyphs around (40..200, 170..210) — uncovered control.
    let visible = region_count_far(&px, rw, 30, 165, 230, 215, bg, 40);
    // "ONTOP" around (300..420, 90..130) — over the z=50 rect.
    let ontop = region_count_far(&px, rw, 290, 80, 450, 135, bg, 40);
    println!("{MARKER} text-layering covered={covered} visible={visible} ontop={ontop}");

    assert!(
        visible > 100,
        "uncovered layered text should render (visible={visible})"
    );
    assert!(
        ontop > 100,
        "z-None text should stay on top of every rect (ontop={ontop})"
    );
    assert!(
        covered < visible / 20,
        "layered text under a higher-z rect must be covered (covered={covered}, visible={visible})"
    );
}

/// Pushes the two covering rects for [`floating_text_with_z_hides_under_a_higher_z_rect`] every
/// frame (the floating texts themselves are long-lived entities driven by `FloatingTextSystem`).
struct FloatingTextCoverScene;
impl System for FloatingTextCoverScene {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(uq) = world.resource_mut::<UiQueue>() {
            // Background-colored covers, so any text bleeding through reads as non-bg pixels.
            uq.push(DrawRect::new(20.0, 70.0, 220.0, 70.0, [0.05, 0.05, 0.07, 1.0]).with_z(1.0));
            uq.push(DrawRect::new(280.0, 70.0, 180.0, 70.0, [0.05, 0.05, 0.07, 1.0]).with_z(50.0));
        }
    }
    fn name(&self) -> &'static str {
        "FloatingTextCoverScene"
    }
}

/// EW-004 regression: a `FloatingText::with_z` renders under a higher-z UI rect (a pause-scrim
/// overlay covers live combat text), an uncovered layered one still renders, and the default
/// (no z) keeps the historical on-top behavior over every rect.
#[test]
fn floating_text_with_z_hides_under_a_higher_z_rect() {
    let mut app = App::new();
    let (w, h) = (480u32, 240u32);
    app.world.insert_resource(WindowConfig {
        title: "render-test: floating-text layering".into(),
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

    // No Camera resource → FloatingTextSystem uses the Transform position as screen coordinates,
    // so the texts land exactly where the regions below expect. Zero velocity / no fade / long
    // lifetime keep them put and fully opaque across the warm-up frames.
    let still = |text: &str| {
        FloatingText::new(text)
            .with_velocity(Vec2::ZERO)
            .with_fade(false)
            .with_lifetime(100.0)
            .with_size(32.0)
    };
    // Covered: layered at z 0.2 under the z 1.0 rect (rect spans 20..240 × 70..140).
    spawn_floating_text(
        &mut app.world,
        Vec2::new(130.0, 105.0),
        still("COVERED").with_z(0.2),
    );
    // Control: same z, no cover.
    spawn_floating_text(
        &mut app.world,
        Vec2::new(130.0, 190.0),
        still("VISIBLE").with_z(0.2),
    );
    // Default (no z): on-top pass, over the z 50 rect (rect spans 280..460 × 70..140).
    spawn_floating_text(&mut app.world, Vec2::new(370.0, 105.0), still("ONTOP"));

    app.add_system(FloatingTextSystem);
    app.add_system(FloatingTextCoverScene);

    let Some((rw, _rh, px)) = render_or_skip(&mut app, 4) else {
        return;
    };
    let bg = px_rgb(&px, rw, 2, 2);

    let covered = region_count_far(&px, rw, 30, 80, 230, 135, bg, 40);
    let visible = region_count_far(&px, rw, 30, 165, 230, 215, bg, 40);
    let ontop = region_count_far(&px, rw, 290, 80, 450, 135, bg, 40);
    println!("{MARKER} floating-text-layering covered={covered} visible={visible} ontop={ontop}");

    assert!(
        visible > 100,
        "uncovered layered floating text should render (visible={visible})"
    );
    assert!(
        ontop > 100,
        "default floating text should stay on top of every rect (ontop={ontop})"
    );
    assert!(
        covered < visible / 20,
        "floating text with_z under a higher-z rect must be covered (covered={covered}, visible={visible})"
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

/// Like [`render_or_skip`] but drives the **editor** egui path on the GPU-less runner (lavapipe)
/// the same way the game-render tests do. `docked = false` captures the overlay editor via
/// [`App::screenshot_editor_headless_rgba`]; `docked = true` captures the full docked layout via
/// [`App::screenshot_editor_docked_headless_rgba`].
fn editor_render_or_skip(app: &mut App, frames: u32, docked: bool) -> Option<(u32, u32, Vec<u8>)> {
    match pollster::block_on(GpuContext::new_headless(4, 4)) {
        Ok(ctx) => {
            let info = ctx.adapter.get_info();
            println!(
                "{MARKER} adapter={} backend={:?} type={:?}",
                info.name, info.backend, info.device_type
            );
            drop(ctx);
            Some(if docked {
                app.screenshot_editor_docked_headless_rgba(frames)
            } else {
                app.screenshot_editor_headless_rgba(frames)
            })
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

    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 3, false) else {
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

/// The headless editor path also draws **action toasts** (bottom-right colour-coded popups). With a
/// dark clear color, the toast's panel + light text make the bottom-right corner measurably brighter
/// than the bare bottom-left — proving the toast rendered. (The overlay's EngineStats sits top-left,
/// so it doesn't pollute either sampled region.)
#[test]
fn editor_toast_renders_headless() {
    let (w, h) = (520u32, 380u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "editor toast headless".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    app.editor_toast_success("Scene saved (3)");

    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 3, false) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    // The Success toast's green panel is actually darker than the (also-dark) clear color, but its
    // light TEXT is not: the brightest pixel in the bottom-right quadrant (where the toast sits;
    // EngineStats is top-left) far exceeds the uniform clear color. Compare against a bare pixel.
    let clear = px_rgb(&px, rw, rw / 2, rh / 2);
    let clear_luma = clear[0] as u32 + clear[1] as u32 + clear[2] as u32;
    let mut max_br = 0u32;
    for y in (rh / 2)..rh {
        for x in (rw / 2)..rw {
            let p = px_rgb(&px, rw, x, y);
            let l = p[0] as u32 + p[1] as u32 + p[2] as u32;
            if l > max_br {
                max_br = l;
            }
        }
    }
    assert!(
        max_br > clear_luma + 120,
        "action toast text not visible bottom-right: max_br={max_br} clear_luma={clear_luma}"
    );
}

/// The headless **docked** editor path draws the full docked layout (top toolbar, left entity list,
/// right inspector, bottom assets panel) with no window. The left entity-list panel is the part the
/// *overlay* capture can never show — so a bright UI pixel in the left-panel region (where the
/// entity rows' light text sits) proves the docked side panels composited headlessly. In overlay
/// mode (or a bare game frame) that region is just the dark clear color. Position-tolerant: scans
/// the left strip for the brightest pixel, so it doesn't depend on egui's exact text layout.
#[test]
fn editor_docked_renders_headless() {
    // Wide enough that the 260px left + 300px right panels both fit with a central strip between.
    let (w, h) = (700u32, 460u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "docked editor headless".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.06, 0.09, 1.0], // dark, so the lighter docked panels contrast sharply
    });
    // A few named entities so the left entity list has rows of light text to draw.
    let mut ents = Vec::new();
    for i in 0..3 {
        let e = app.world.spawn();
        app.world.add_component(e, Tag(format!("Quad {i}")));
        app.world.add_component(
            e,
            Transform::new(Vec2::new(40.0, 40.0), Vec2::splat(24.0), 0.0),
        );
        app.world.add_component(e, Sprite::colored(0.8, 0.4, 0.4));
        ents.push(e);
    }
    app.world.add_component(ents[1], engine::Hidden);

    // >= 5 frames so the docked layout is fully built (the central RT debounce is irrelevant to the
    // side panels, which render from frame 1, but use the documented count for realism).
    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 6, true) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    // Brightest pixel in the LEFT panel strip (x < 240, below the toolbar): the entity-list text is
    // near-white, far brighter than the dark clear color. If the docked left panel never rendered,
    // this region would stay at the clear color and max_luma would be tiny.
    let mut max_luma = 0u32;
    for y in 40..(rh - 220) {
        for x in 10..240 {
            let p = px_rgb(&px, rw, x, y);
            let l = p[0] as u32 + p[1] as u32 + p[2] as u32;
            if l > max_luma {
                max_luma = l;
            }
        }
    }
    assert!(
        max_luma > 400,
        "no bright docked-panel pixels in the left strip — the docked editor did not render: max_luma={max_luma}"
    );
}

/// The docked editor's game viewport must survive `PostProcessConfig { enabled: true }`.
///
/// While docked, the scene renders into the editor's own offscreen RT, **not** into the
/// post-process intermediate — `render_view` picks the docked view first. But the post, bloom and
/// lighting passes read from that intermediate and composite onto `scene_target`, which is *also*
/// the docked RT. So with post enabled they painted a texture this frame never rendered into over
/// the game viewport: it went blank. (With `hdr: true` it is worse than blank — an `Rgba16Float`
/// intermediate against a surface-format target is a wgpu format-validation error.)
///
/// The check keys on the game's `clear_color` rather than a sprite, so it needs no assumption about
/// where in the central panel a world position lands: the viewport is either showing the game
/// scene (saturated green) or it is not.
#[test]
fn docked_editor_viewport_survives_post_process() {
    // Wide enough that the 260px left + 300px right panels leave a real central strip.
    let (w, h) = (900u32, 520u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "docked editor + post-process".into(),
        width: w,
        height: h,
        // Saturated green — unmistakable against both the grey editor chrome and the black a
        // blanked viewport reads as.
        clear_color: [0.0, 0.85, 0.25, 1.0],
    });
    // The configuration that used to blank the viewport.
    app.world.insert_resource(PostProcessConfig {
        enabled: true,
        ..Default::default()
    });
    let e = app.world.spawn();
    app.world.add_component(e, Tag("Quad".into()));
    app.world.add_component(
        e,
        Transform::new(Vec2::new(40.0, 40.0), Vec2::splat(24.0), 0.0),
    );
    app.world.add_component(e, Sprite::colored(0.9, 0.9, 0.9));

    // >= 5 frames so the central RT debounce has fired and the viewport is composited.
    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 8, true) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    // Central strip only: inside the left panel's 260px and the right panel's 300px, below the
    // toolbar and above the bottom Data Tables panel.
    let mut green = 0u32;
    let mut total = 0u32;
    for y in 60..(rh - 240) {
        for x in 280..(rw - 320) {
            let p = px_rgb(&px, rw, x, y);
            total += 1;
            if p[1] as i32 > p[0] as i32 + 40 && p[1] as i32 > p[2] as i32 + 40 {
                green += 1;
            }
        }
    }
    assert!(
        total > 0,
        "central strip region is empty — bad test geometry"
    );
    assert!(
        green * 4 > total,
        "the docked game viewport is not showing the scene with post-process enabled: only \
         {green}/{total} central pixels are green-dominant. The post chain composited an \
         intermediate the scene never rendered into over the viewport."
    );
}

/// With an inline rename in progress, the docked entity list draws a focused text box (bound to the
/// rename buffer) in place of one row's label. This drives that path headlessly: begin a rename via
/// [`App::editor_begin_rename`], then capture the docked editor. The left-panel strip must still
/// composite bright UI (the text box + the buffer text), proving the rename branch renders without
/// panicking — the prerequisite for golden-testing the in-edit state.
#[test]
fn editor_docked_inline_rename_renders_headless() {
    let (w, h) = (700u32, 460u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "docked rename headless".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    let mut ents = Vec::new();
    for i in 0..3 {
        let e = app.world.spawn();
        app.world.add_component(e, Tag(format!("Quad {i}")));
        app.world.add_component(
            e,
            Transform::new(Vec2::new(40.0, 40.0), Vec2::splat(24.0), 0.0),
        );
        app.world.add_component(e, Sprite::colored(0.8, 0.4, 0.4));
        ents.push(e);
    }
    // Start renaming the middle row: its label is replaced by a text box showing this buffer.
    app.editor_begin_rename(ents[1]);

    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 6, true) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    // Same left-strip brightness probe as the docked test: a renaming row still draws bright UI
    // (the text box frame + the editable name), so the left panel composited the in-edit state.
    let mut max_luma = 0u32;
    for y in 40..(rh - 220) {
        for x in 10..240 {
            let p = px_rgb(&px, rw, x, y);
            let l = p[0] as u32 + p[1] as u32 + p[2] as u32;
            if l > max_luma {
                max_luma = l;
            }
        }
    }
    assert!(
        max_luma > 400,
        "no bright pixels in the left strip while renaming — the inline-rename row did not render: max_luma={max_luma}"
    );
}

/// The docked **Scene** tab renders the parent→children tree with drag-to-reparent wiring. This
/// drives the structural path headlessly: build a small hierarchy, reparent a node via the cycle-safe
/// [`App::editor_reparent`] (the same edit a drop performs), switch to the Scene tab via
/// [`App::editor_show_scene_tree`], then capture. The left-panel strip must composite the (now
/// re-nested) tree's bright UI text, proving the Scene tab + DnD-wrapped nodes render without panic.
#[test]
fn editor_docked_scene_tree_reparent_renders_headless() {
    let (w, h) = (700u32, 460u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "docked scene-tree headless".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    // Three roots; give each a Transform so the hierarchy is well-formed.
    let mut ents = Vec::new();
    for i in 0..3 {
        let e = app.world.spawn();
        app.world.add_component(e, Tag(format!("Node {i}")));
        app.world.add_component(
            e,
            Transform::new(Vec2::new(40.0, 40.0), Vec2::splat(24.0), 0.0),
        );
        ents.push(e);
    }
    // Reparent Node 1 + Node 2 under Node 0 (the same edit a Scene-tree drop performs).
    assert!(
        app.editor_reparent(ents[1], Some(ents[0])),
        "valid reparent"
    );
    assert!(
        app.editor_reparent(ents[2], Some(ents[0])),
        "valid reparent"
    );
    // A cycle attempt must be refused (Node 0 under its own child Node 1).
    assert!(
        !app.editor_reparent(ents[0], Some(ents[1])),
        "descendant target rejected — no cycle"
    );
    app.editor_show_scene_tree();

    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 6, true) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    let mut max_luma = 0u32;
    for y in 40..(rh - 220) {
        for x in 10..240 {
            let p = px_rgb(&px, rw, x, y);
            let l = p[0] as u32 + p[1] as u32 + p[2] as u32;
            if l > max_luma {
                max_luma = l;
            }
        }
    }
    assert!(
        max_luma > 400,
        "no bright pixels in the left strip — the docked Scene tree did not render: max_luma={max_luma}"
    );
}

/// Inline rename also works from the docked **Scene** tab (not just the Entities list). This drives
/// that path headlessly: build a hierarchy, switch to the Scene tab, begin renaming a nested node via
/// the shared [`App::editor_begin_rename`], then capture. In the Scene tree a renaming node draws a
/// focused text box (bound to the rename buffer) instead of the draggable label — the left-panel
/// strip must still composite bright UI, proving the Scene-tab rename branch renders without panic.
#[test]
fn editor_docked_scene_tree_rename_renders_headless() {
    let (w, h) = (700u32, 460u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "docked scene-rename headless".into(),
        width: w,
        height: h,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });
    let mut ents = Vec::new();
    for i in 0..3 {
        let e = app.world.spawn();
        app.world.add_component(e, Tag(format!("Node {i}")));
        app.world.add_component(
            e,
            Transform::new(Vec2::new(40.0, 40.0), Vec2::splat(24.0), 0.0),
        );
        ents.push(e);
    }
    // Nest Node 1 under Node 0, then rename the (now nested) child from the Scene tab.
    assert!(
        app.editor_reparent(ents[1], Some(ents[0])),
        "valid reparent"
    );
    app.editor_show_scene_tree();
    app.editor_begin_rename(ents[1]);

    let Some((rw, rh, px)) = editor_render_or_skip(&mut app, 6, true) else {
        return;
    };
    assert_eq!((rw, rh), (w, h), "read-back size mismatch");

    let mut max_luma = 0u32;
    for y in 40..(rh - 220) {
        for x in 10..240 {
            let p = px_rgb(&px, rw, x, y);
            let l = p[0] as u32 + p[1] as u32 + p[2] as u32;
            if l > max_luma {
                max_luma = l;
            }
        }
    }
    assert!(
        max_luma > 400,
        "no bright pixels in the left strip — the Scene-tab inline-rename node did not render: max_luma={max_luma}"
    );
}

/// A `Hidden` component suppresses an entity's sprite in the render path. Two quads — left red
/// (visible), right green (Hidden). The left reads red; the right region stays at the background
/// (the green quad is gone), proving the sprite pass skips Hidden entities.
#[test]
fn hidden_component_suppresses_sprite() {
    let (w, h) = (256u32, 256u32);
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "render-test: hidden".into(),
        width: w,
        height: h,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    // Left: a plain visible red quad.
    spawn_quad(
        &mut app,
        Vec2::new(w as f32 * 0.25, h as f32 / 2.0),
        Vec2::splat(80.0),
        Sprite::colored(1.0, 0.0, 0.0),
    );
    // Right: a green quad, but Hidden — it must not render.
    let g = app.world.spawn();
    app.world.add_component(
        g,
        Transform {
            position: Vec2::new(w as f32 * 0.75, h as f32 / 2.0),
            scale: Vec2::splat(80.0),
            rotation: 0.0,
            z: 0.0,
        },
    );
    app.world.add_component(g, Sprite::colored(0.0, 1.0, 0.0));
    app.world.add_component(g, engine::Hidden);

    let Some((rw, rh, px)) = render_or_skip(&mut app, 4) else {
        return;
    };
    // Left: the red quad drew (R dominates).
    let left = region_mean(&px, rw, rw / 4 - 15, rh / 2 - 15, rw / 4 + 15, rh / 2 + 15);
    assert!(
        left[0] - left[1] > 60.0 && left[0] - left[2] > 60.0,
        "left red quad not drawn: {left:?}"
    );
    // Right: the Hidden green quad did NOT draw — the region is background, not green-dominant.
    let right = region_mean(
        &px,
        rw,
        (rw * 3) / 4 - 15,
        rh / 2 - 15,
        (rw * 3) / 4 + 15,
        rh / 2 + 15,
    );
    assert!(
        right[1] - right[0] < 30.0 && right[1] - right[2] < 30.0,
        "Hidden green quad still rendered: {right:?}"
    );
}
