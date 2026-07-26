//! Scripted input + headless capture — verify a GUI with no window, display or OS permissions.
//!
//! Checking that a screen still looks right normally needs a live, unlocked desktop and OS
//! automation (on macOS: Accessibility + Screen-Recording permissions, `osascript` key codes, a
//! synthetic-mouse helper). None of that runs on CI, so the usual fallback is a boot smoke test
//! that proves the app starts but is blind to a wrong z-order, a missing icon or a misplaced
//! panel.
//!
//! [`InputScript`] replaces the human — it injects keys and clicks into the same [`InputState`]
//! the window feeds — and [`App::capture_frames_headless`] photographs chosen frames. Together a
//! game drives itself through several screens and writes a PNG of each, from a plain `cargo run`.
//!
//! This example is a three-screen toy shop: **Menu** → **Shop** (item grid) → **Detail** (the
//! clicked item). Play it by hand, or hand it the bundled script:
//!
//! ```sh
//! # No code change needed — the engine reads both variables in `App::run`.
//! ENGINE_INPUT=examples/scripted_capture.ron \
//! ENGINE_CAPTURE=8:/tmp/menu.png,28:/tmp/shop.png,52:/tmp/detail.png \
//! cargo run --example scripted_capture
//! ```
//!
//! Three PNGs, three different screens, no window ever opened.
//!
//! - **1 / 2** — menu ↔ shop · **click an item** — open its detail · **Esc** — back / quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/scripted cargo run --example scripted_capture`): plays the same
//! script through [`App::capture_frames_headless`], writes `<prefix>_{menu,shop,detail}.png`, and
//! asserts the script actually drove the transitions — exiting non-zero if the app did not end up
//! on the Detail screen.
use engine::{
    App, Color, DrawRect, DrawText, InputScript, InputState, KeyCode, MouseButton, ShouldQuit,
    System, TextQueue, UiQueue, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 900;
const WIN_H: u32 = 560;

/// Shop stock: the grid the script clicks into.
const ITEMS: [(&str, u32); 6] = [
    ("사슬갑옷", 200),
    ("Iron Sword", 45),
    ("체력 물약", 12),
    ("Elixir of Warding", 1200),
    ("Oak Shield", 80),
    ("Rope", 6),
];

const COLS: usize = 3;
const CELL_W: f32 = 240.0;
const CELL_H: f32 = 110.0;
const GRID_X: f32 = 60.0;
const GRID_Y: f32 = 130.0;
const GAP: f32 = 16.0;

/// Which screen is showing. The script's job is to move this from `Menu` to `Detail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Menu,
    Shop,
    Detail(usize),
}

/// Resource the headless check reads back after the run.
struct Ui {
    screen: Screen,
}

/// Top-left corner of item `i`'s cell.
fn cell_pos(i: usize) -> Vec2 {
    let (col, row) = (i % COLS, i / COLS);
    Vec2::new(
        GRID_X + col as f32 * (CELL_W + GAP),
        GRID_Y + row as f32 * (CELL_H + GAP),
    )
}

/// Item whose cell contains `cursor`, if any — the same hit-test a real click runs.
fn item_at(cursor: Vec2) -> Option<usize> {
    (0..ITEMS.len()).find(|&i| {
        let p = cell_pos(i);
        cursor.x >= p.x && cursor.x < p.x + CELL_W && cursor.y >= p.y && cursor.y < p.y + CELL_H
    })
}

struct Demo;

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // ── Input → screen transitions ───────────────────────────────────────────
        let Some(input) = world.resource::<InputState>() else {
            return;
        };
        let (to_menu, to_shop, back, clicked) = (
            input.just_pressed(KeyCode::Digit1),
            input.just_pressed(KeyCode::Digit2),
            input.just_pressed(KeyCode::Escape),
            input
                .mouse_just_pressed(MouseButton::Left)
                .then(|| item_at(input.mouse_press_cursor(MouseButton::Left)))
                .flatten(),
        );

        let screen = world
            .resource::<Ui>()
            .map(|u| u.screen)
            .unwrap_or(Screen::Menu);
        let next = match screen {
            _ if to_menu => Screen::Menu,
            _ if to_shop => Screen::Shop,
            Screen::Shop => match clicked {
                Some(i) => Screen::Detail(i),
                None => Screen::Shop,
            },
            Screen::Detail(_) if back => Screen::Shop,
            other => other,
        };
        if back && screen == Screen::Menu {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if let Some(ui) = world.resource_mut::<Ui>() {
            ui.screen = next;
        }

        // ── Draw ─────────────────────────────────────────────────────────────────
        match next {
            Screen::Menu => draw_menu(world),
            Screen::Shop => draw_shop(world, None),
            Screen::Detail(i) => {
                draw_shop(world, Some(i));
                draw_detail(world, i);
            }
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "1 menu · 2 shop · click an item · Esc back/quit",
                Vec2::new(GRID_X, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.62, 0.65, 0.72),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "scripted_capture_demo"
    }
}

fn draw_menu(world: &mut World) {
    if let Some(uq) = world.resource_mut::<UiQueue>() {
        uq.push(
            DrawRect::new(
                GRID_X,
                150.0,
                420.0,
                180.0,
                Color::rgba(0.16, 0.19, 0.29, 1.0),
            )
            .with_corner_radius(10.0)
            .with_z(0.2),
        );
    }
    if let Some(tq) = world.resource_mut::<TextQueue>() {
        tq.push(DrawText::new(
            "MENU",
            Vec2::new(GRID_X + 26.0, 178.0),
            34.0,
            Color::rgb(0.93, 0.9, 0.78),
        ));
        tq.push(DrawText::new(
            "press 2 to enter the shop",
            Vec2::new(GRID_X + 26.0, 232.0),
            18.0,
            Color::rgb(0.78, 0.82, 0.9),
        ));
    }
}

fn draw_shop(world: &mut World, selected: Option<usize>) {
    if let Some(uq) = world.resource_mut::<UiQueue>() {
        for i in 0..ITEMS.len() {
            let p = cell_pos(i);
            let bg = if selected == Some(i) {
                Color::rgba(0.20, 0.36, 0.32, 1.0)
            } else {
                Color::rgba(0.16, 0.19, 0.29, 1.0)
            };
            uq.push(
                DrawRect::new(p.x, p.y, CELL_W, CELL_H, bg)
                    .with_corner_radius(8.0)
                    .with_z(0.2),
            );
        }
    }
    if let Some(tq) = world.resource_mut::<TextQueue>() {
        // These labels carry a z so the detail popup can cover them. Text drawn WITHOUT a z
        // renders in the final on-top pass — right for a HUD, but it would bleed straight
        // through the popup. (The first headless capture of this example caught exactly that:
        // the popup was drawn but the item names still read over it.)
        tq.push(
            DrawText::new(
                "SHOP",
                Vec2::new(GRID_X, 72.0),
                28.0,
                Color::rgb(0.93, 0.9, 0.78),
            )
            .with_z(0.3),
        );
        for (i, (name, price)) in ITEMS.iter().enumerate() {
            let p = cell_pos(i);
            tq.push(
                DrawText::new(
                    *name,
                    Vec2::new(p.x + 16.0, p.y + 22.0),
                    18.0,
                    Color::rgb(0.94, 0.95, 0.98),
                )
                .with_z(0.3),
            );
            tq.push(
                DrawText::new(
                    format!("{price}g"),
                    Vec2::new(p.x + 16.0, p.y + 58.0),
                    16.0,
                    Color::rgb(0.82, 0.86, 0.62),
                )
                .with_z(0.3),
            );
        }
    }
}

fn draw_detail(world: &mut World, i: usize) {
    let (name, price) = ITEMS[i];
    if let Some(uq) = world.resource_mut::<UiQueue>() {
        // Scrim + popup, above the grid and its (z-layered) labels.
        uq.push(
            DrawRect::new(
                0.0,
                0.0,
                WIN_W as f32,
                WIN_H as f32,
                Color::rgba(0.04, 0.05, 0.08, 0.72),
            )
            .with_z(0.5),
        );
        uq.push(
            DrawRect::new(
                200.0,
                170.0,
                500.0,
                220.0,
                Color::rgba(0.22, 0.25, 0.36, 1.0),
            )
            .with_corner_radius(12.0)
            .with_z(0.6),
        );
    }
    if let Some(tq) = world.resource_mut::<TextQueue>() {
        tq.push(
            DrawText::new(
                name,
                Vec2::new(230.0, 205.0),
                30.0,
                Color::rgb(0.96, 0.95, 0.9),
            )
            .with_z(0.7),
        );
        tq.push(
            DrawText::new(
                format!("{price} gold"),
                Vec2::new(230.0, 258.0),
                20.0,
                Color::rgb(0.85, 0.88, 0.64),
            )
            .with_z(0.7),
        );
        tq.push(
            DrawText::new(
                "Esc to go back",
                Vec2::new(230.0, 330.0),
                16.0,
                Color::rgb(0.7, 0.74, 0.82),
            )
            .with_z(0.7),
        );
    }
}

/// Frames the headless pass photographs — one per screen the script visits.
const SHOTS: [(u32, &str); 3] = [(8, "menu"), (28, "shop"), (52, "detail")];

fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "scripted_capture — InputScript + headless capture".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Ui {
        screen: Screen::Menu,
    });
    app.add_system(Demo);
    app
}

fn main() {
    let mut app = build_app();

    // Headless acceptance pass: play the bundled script and photograph each screen, then check
    // that the scripted keys/clicks really drove the transitions.
    if let Ok(prefix) = std::env::var("HEADLESS_SHOT") {
        let script = InputScript::load("examples/scripted_capture.ron").expect("load script");
        assert!(!script.is_empty(), "script should carry events");
        app.set_input_script(script);

        let captures: Vec<(u32, String)> = SHOTS
            .iter()
            .map(|(frame, name)| (*frame, format!("{prefix}_{name}.png")))
            .collect();
        let written = app
            .capture_frames_headless(&captures)
            .expect("headless capture");

        let screen = app.world.resource::<Ui>().expect("ui").screen;
        if !matches!(screen, Screen::Detail(_)) {
            eprintln!(
                "FAIL: after the script the app is on {screen:?}, expected Detail — \
                 scripted input did not drive the transitions"
            );
            std::process::exit(1);
        }
        for path in &written {
            println!("wrote {}", path.display());
        }
        println!("OK: scripted keys + click drove Menu → Shop → {screen:?} with no window");
        return;
    }

    // Windowed. `ENGINE_INPUT` / `ENGINE_CAPTURE` are honoured by `App::run` itself, so the
    // same script can drive this run without any of the code above.
    app.run();
}
