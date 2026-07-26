//! Text measurement — size a panel from the text that will actually be drawn in it.
//!
//! A tooltip/chip/bubble has to be as wide as its string, but the string's width is only known
//! after shaping. The usual stand-in is a `chars × px` guess, which breaks as soon as scripts
//! mix: at 15 px a Hangul glyph advances ≈ 15 px while `4` or `g` advances ≈ 8 px, so
//! `"사슬갑옷"` and `"200g"` — both 4 characters — are nearly 2× apart. The guess gets padded
//! until it stops clipping, and every panel is loose forever after.
//!
//! [`measure_text`] shapes through the renderer's own path (same metrics, same font stack incl.
//! the Hangul fallback loaded below) and reports the real width.
//!
//! Each row is a shop tooltip drawn twice: **left** sized by a `chars × 15 + 22` heuristic,
//! **right** sized by `measure_text`. A vertical tick marks where the text truly ends, so the
//! heuristic panel's error is visible — amber when it is loose, red when it clips. Raise the
//! font size and the heuristic, tuned at 15, turns red across the board.
//!
//! - **Space** — draw the heuristic panels or hide them (measured only)
//! - **↑ / ↓** — font size (the heuristic is only ever tuned for ONE size)
//! - **← / →** — wrap width of the paragraph panel ([`measure_text_wrapped`])
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/text_measure.png cargo run --example text_measure`): asserts the
//! measurement is script-aware, that it disagrees with the heuristic, and that a wrapped
//! measurement stays inside its bound — exits non-zero if any check fails, then captures a shot.
use engine::resources::{ExtraFonts, FontData};
use engine::{
    measure_text, measure_text_wrapped, App, Color, DrawRect, DrawText, InputState, KeyCode,
    ShouldQuit, System, TextMeasurer, TextQueue, UiQueue, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 900;
const WIN_H: u32 = 600;

/// Shop rows: the name/value pairs a merchant tooltip shows. Deliberately mixed —
/// Hangul, Latin, wide caps, digits — because that is where a per-character guess fails.
const ROWS: [(&str, &str); 5] = [
    ("사슬갑옷", "200g"),
    ("Iron Sword", "45g"),
    ("체력 물약", "12g"),
    ("Elixir of Warding", "1200g"),
    ("MMMM WWWW", "8g"),
];

/// The paragraph measured with a wrap bound.
const PARAGRAPH: &str =
    "이 갑옷은 오래된 광산에서 발굴되었다. Sturdy, well-oiled, and still worth a haggle.";

/// Inner padding a measured panel adds around its text.
const PAD: f32 = 11.0;

/// Font size the heuristic below was hand-tuned at — a constant baked into the formula,
/// which is what makes the guess collapse as soon as the UI is drawn at another size.
const HEURISTIC_TUNED_SIZE: f32 = 15.0;

/// Largest size ↑ reaches. Heuristic panels start clipping here (the headless check asserts
/// it), so the size-dependence is reachable in real play, not just in theory.
const MAX_FONT_SIZE: f32 = 28.0;

/// The heuristic this API replaces: the widest line's CHARACTER count times the size it was
/// tuned at, plus a fixed pad. Roughly right for full-width glyphs at that one size, and
/// wrong everywhere else — too loose for Latin, too narrow once the text grows.
fn heuristic_width(name: &str, value: &str) -> f32 {
    let chars = name.chars().count().max(value.chars().count()) as f32;
    chars * HEURISTIC_TUNED_SIZE + 22.0
}

struct Demo {
    font_size: f32,
    wrap_width: f32,
    show_heuristic: bool,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input ────────────────────────────────────────────────────────────────
        let (quit, toggle, bigger, smaller, wider, narrower) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Escape),
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::ArrowUp),
                    i.just_pressed(KeyCode::ArrowDown),
                    i.just_pressed(KeyCode::ArrowRight),
                    i.just_pressed(KeyCode::ArrowLeft),
                )
            })
            .unwrap_or((false, false, false, false, false, false));
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if toggle {
            self.show_heuristic = !self.show_heuristic;
        }
        if bigger {
            self.font_size = (self.font_size + 2.0).min(MAX_FONT_SIZE);
        }
        if smaller {
            self.font_size = (self.font_size - 2.0).max(10.0);
        }
        if wider {
            self.wrap_width = (self.wrap_width + 20.0).min(380.0);
        }
        if narrower {
            self.wrap_width = (self.wrap_width - 20.0).max(120.0);
        }

        let size = self.font_size;
        let line_h = TextMeasurer::line_height(size);
        let row_h = line_h * 2.0 + PAD * 2.0;

        // ── Measure everything up front (each call borrows the world briefly) ─────
        let mut rows = Vec::with_capacity(ROWS.len());
        for (name, value) in ROWS {
            let name_w = measure_text(world, name, size).x;
            let value_w = measure_text(world, value, size).x;
            rows.push((name, value, name_w.max(value_w)));
        }
        let para = measure_text_wrapped(world, PARAGRAPH, size, self.wrap_width);

        // ── Draw ─────────────────────────────────────────────────────────────────
        let left_x = 40.0;
        let right_x = 480.0;
        let top_y = 96.0;

        if let Some(uq) = world.resource_mut::<UiQueue>() {
            for (i, (name, value, text_w)) in rows.iter().enumerate() {
                let y = top_y + i as f32 * (row_h + 14.0);
                let measured_w = text_w + PAD * 2.0;
                let guess_w = heuristic_width(name, value);

                if self.show_heuristic {
                    // Red when the guess is narrower than the text (clips), amber when it
                    // overshoots by more than a pad's worth (loose), else neutral.
                    let color = if guess_w < measured_w {
                        Color::rgba(0.45, 0.15, 0.17, 1.0)
                    } else if guess_w > measured_w + PAD {
                        Color::rgba(0.40, 0.31, 0.13, 1.0)
                    } else {
                        Color::rgba(0.20, 0.22, 0.28, 1.0)
                    };
                    uq.push(
                        DrawRect::new(left_x, y, guess_w, row_h, color)
                            .with_corner_radius(6.0)
                            .with_z(0.2),
                    );
                    // Tick at the real text edge: the gap to the panel edge IS the error.
                    uq.push(
                        DrawRect::new(
                            left_x + PAD + text_w,
                            y + 4.0,
                            2.0,
                            row_h - 8.0,
                            Color::rgba(0.95, 0.85, 0.35, 1.0),
                        )
                        .with_z(0.3),
                    );
                }

                uq.push(
                    DrawRect::new(
                        right_x,
                        y,
                        measured_w,
                        row_h,
                        Color::rgba(0.14, 0.27, 0.24, 1.0),
                    )
                    .with_corner_radius(6.0)
                    .with_z(0.2),
                );
                uq.push(
                    DrawRect::new(
                        right_x + PAD + text_w,
                        y + 4.0,
                        2.0,
                        row_h - 8.0,
                        Color::rgba(0.45, 0.85, 0.6, 1.0),
                    )
                    .with_z(0.3),
                );
            }

            // Paragraph panel — sized to the LONGEST WRAPPED LINE, not to the wrap bound,
            // so it is tight even when the text does not fill the last line.
            let para_y = top_y + ROWS.len() as f32 * (row_h + 14.0) + 10.0;
            uq.push(
                DrawRect::new(
                    left_x,
                    para_y,
                    para.x + PAD * 2.0,
                    para.y + PAD * 2.0,
                    Color::rgba(0.17, 0.19, 0.30, 1.0),
                )
                .with_corner_radius(6.0)
                .with_z(0.2),
            );
            // The wrap bound the text was measured against — the panel sits inside it.
            uq.push(
                DrawRect::new(
                    left_x,
                    para_y,
                    self.wrap_width + PAD * 2.0,
                    para.y + PAD * 2.0,
                    Color::rgba(0.30, 0.33, 0.45, 1.0),
                )
                .with_border(1.0)
                .with_corner_radius(6.0)
                .with_z(0.15),
            );
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for (i, (name, value, text_w)) in rows.iter().enumerate() {
                let y = top_y + i as f32 * (row_h + 14.0);
                let guess_w = heuristic_width(name, value);
                let measured_w = text_w + PAD * 2.0;

                if self.show_heuristic {
                    push_row(tq, name, value, left_x + PAD, y + PAD, size, line_h);
                }
                push_row(tq, name, value, right_x + PAD, y + PAD, size, line_h);

                tq.push(DrawText::new(
                    format!(
                        "{:+.0}px ({:.0} vs {:.0})",
                        guess_w - measured_w,
                        guess_w,
                        measured_w
                    ),
                    Vec2::new(right_x + measured_w + 14.0, y + PAD + 2.0),
                    13.0,
                    if guess_w < measured_w {
                        Color::rgb(0.95, 0.55, 0.5)
                    } else {
                        Color::rgb(0.65, 0.68, 0.75)
                    },
                ));
            }

            let para_y = top_y + ROWS.len() as f32 * (row_h + 14.0) + 10.0;
            tq.push(
                DrawText::new(
                    PARAGRAPH,
                    Vec2::new(left_x + PAD, para_y + PAD),
                    size,
                    Color::rgb(0.86, 0.89, 0.95),
                )
                .with_bounds(Vec2::new(self.wrap_width, para.y + 4.0)),
            );
            tq.push(DrawText::new(
                format!(
                    "measure_text_wrapped → {:.0} × {:.0} inside a {:.0}px bound",
                    para.x, para.y, self.wrap_width
                ),
                Vec2::new(left_x + self.wrap_width + PAD * 2.0 + 16.0, para_y + PAD),
                13.0,
                Color::rgb(0.65, 0.68, 0.75),
            ));

            // Headers + HUD (no z — always on top).
            tq.push(DrawText::new(
                "text measurement — panel width from the shaped string, not a chars×px guess",
                Vec2::new(left_x, 28.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                "heuristic: chars × 15 + 22 (tuned at size 15)",
                Vec2::new(left_x, 66.0),
                14.0,
                Color::rgb(0.9, 0.75, 0.45),
            ));
            tq.push(DrawText::new(
                "measure_text",
                Vec2::new(right_x, 66.0),
                14.0,
                Color::rgb(0.5, 0.88, 0.68),
            ));
            tq.push(DrawText::new(
                format!(
                    "Space: heuristic {}   ↑↓: size {size:.0}   ←→: wrap {:.0}   Esc: quit",
                    if self.show_heuristic {
                        "shown"
                    } else {
                        "hidden"
                    },
                    self.wrap_width
                ),
                Vec2::new(left_x, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "text_measure_demo"
    }
}

/// Two-line tooltip body: name over value.
fn push_row(tq: &mut TextQueue, name: &str, value: &str, x: f32, y: f32, size: f32, line_h: f32) {
    tq.push(DrawText::new(name, Vec2::new(x, y), size, Color::rgb(0.94, 0.95, 0.98)).with_z(0.4));
    tq.push(
        DrawText::new(
            value,
            Vec2::new(x, y + line_h),
            size,
            Color::rgb(0.82, 0.86, 0.62),
        )
        .with_z(0.4),
    );
}

/// Headless acceptance checks — the properties a chars×px guess cannot have.
/// Returns the failures found.
fn self_check(app: &mut App) -> Vec<String> {
    let mut fails = Vec::new();
    let size = 15.0;

    // 1. The Hangul fallback font is actually consulted: "사슬갑옷" must measure like real
    //    glyphs, not collapse to zero (missing font) or to notdef boxes.
    let hangul = app.measure_text("사슬갑옷", size).x;
    if hangul < size * 3.0 {
        fails.push(format!(
            "Hangul measured {hangul:.1}px — too narrow for 4 full-width glyphs (fallback font missing?)"
        ));
    }

    // 2. Script awareness: 4 Hangul glyphs must be clearly wider than 4 digits/letters,
    //    which is exactly the distinction a per-character guess cannot make.
    let latin = app.measure_text("200g", size).x;
    if hangul <= latin * 1.3 {
        fails.push(format!(
            "same-char-count strings measured alike (한글 {hangul:.1} vs latin {latin:.1}) — measurement is not script-aware"
        ));
    }

    // 3. The heuristic really is wrong: at least one row must disagree by >10%.
    let worst = ROWS
        .iter()
        .map(|(name, value)| {
            let text_w = app
                .measure_text(name, size)
                .x
                .max(app.measure_text(value, size).x)
                + PAD * 2.0;
            ((heuristic_width(name, value) - text_w) / text_w).abs()
        })
        .fold(0.0f32, f32::max);
    if worst < 0.1 {
        fails.push(format!(
            "heuristic matched every row within {:.0}% — the example proves nothing",
            worst * 100.0
        ));
    }

    // 4. The heuristic is tuned for ONE size: draw the same rows larger and its fixed
    //    per-character constant now under-measures, i.e. the panel clips its own text.
    let big = MAX_FONT_SIZE;
    let clips = ROWS.iter().any(|(name, value)| {
        let text_w = app
            .measure_text(name, big)
            .x
            .max(app.measure_text(value, big).x);
        heuristic_width(name, value) < text_w + PAD * 2.0
    });
    if !clips {
        fails.push(format!(
            "no row clipped at size {big} — the size-dependence of the heuristic is not demonstrated"
        ));
    }

    // 5. A wrapped measurement stays inside its bound and is taller than one line.
    let bound = 240.0;
    let para = app.measure_text_wrapped(PARAGRAPH, size, bound);
    if para.x > bound + 0.5 {
        fails.push(format!(
            "wrapped width {:.1} exceeded bound {bound}",
            para.x
        ));
    }
    if para.y <= TextMeasurer::line_height(size) {
        fails.push(format!(
            "wrapped paragraph measured {:.1}px tall — expected multiple lines",
            para.y
        ));
    }

    println!(
        "measure: 사슬갑옷={hangul:.1}px  200g={latin:.1}px  worst heuristic error={:.0}%  wrapped={:.0}×{:.0}",
        worst * 100.0,
        para.x,
        para.y
    );
    fails
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "text_measure — measure_text".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    // Latin base font + a Hangul fallback: measuring a mixed string is only correct if the
    // measurer walks the same fallback chain the renderer draws with.
    app.world.insert_resource(FontData(
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/DejaVuSans.ttf"
        ))
        .to_vec(),
    ));
    app.world
        .insert_resource(ExtraFonts(vec![include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/fonts/NotoSansKR-Regular-subset.ttf"
        ))
        .to_vec()]));
    app.add_system(Demo {
        font_size: 15.0,
        wrap_width: 240.0,
        show_heuristic: true,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let fails = self_check(&mut app);
        if !fails.is_empty() {
            for f in &fails {
                eprintln!("FAIL: {f}");
            }
            std::process::exit(1);
        }
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(10);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("OK: measurement is script-aware and beats the heuristic; wrote {out}");
        return;
    }
    app.run();
}
