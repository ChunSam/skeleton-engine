//! gamepad_probe — a side-by-side diagnostic of the two macOS gamepad input paths.
//!
//! The engine reads gamepads through **gilrs** (an IOKit-HID client). On macOS, modern
//! Bluetooth/USB Xbox & PS5 controllers are claimed by Apple's **GameController framework**
//! (their HID device shows `IOUserServerName = com.apple.gamecontroller.driver.*`), so the
//! IOKit-HID client *enumerates* the pad but receives **zero input reports** — sticks and buttons
//! read as 0 even while you're moving them. (See `docs/HANDOFF.md` / the engine memory note.)
//!
//! This probe shows, every frame and next to each other:
//!   - **gilrs (HID)** — exactly what the engine's `GamepadState` resource sees.
//!   - **GameController** — what Apple's framework sees for the same pad (macOS only).
//!
//! Run it on a Mac with a pad connected and move the left stick / press A·B·X·Y. If the
//! **GameController** column moves but the **gilrs (HID)** column stays at 0, that empirically
//! confirms the input path and shows that a GameController-framework backend is the real fix.
//! On Windows/Linux the GameController column is absent (the gilrs/HID path is the live one there).
//!
//! Note on stick signs: the engine's `AxisBinding` convention is up = −Y, down = +Y; the
//! GameController framework reports up = +Y. The raw values are shown unmodified on each side.
//!
//! Run from the repo root:  `cargo run --example gamepad_probe`   (ESC quits)

use engine::{
    App, Color, DrawText, GamepadAxis, GamepadButton, GamepadState, InputState, KeyCode,
    ShouldQuit, System, TextQueue, Vec2, WindowConfig, World,
};

/// What the engine's gilrs (IOKit-HID) backend reports for pad 0.
#[derive(Default)]
struct HidView {
    connected: bool,
    primary: Option<usize>,
    lx: f32,
    ly: f32,
    rx: f32,
    ry: f32,
    a: bool,
    b: bool,
    x: bool,
    y: bool,
    lt: f32,
    rt: f32,
}

impl HidView {
    fn read(gs: &GamepadState) -> Self {
        Self {
            connected: gs.any_connected(),
            primary: gs.primary(),
            lx: gs.axis(0, GamepadAxis::LeftStickX),
            ly: gs.axis(0, GamepadAxis::LeftStickY),
            rx: gs.axis(0, GamepadAxis::RightStickX),
            ry: gs.axis(0, GamepadAxis::RightStickY),
            a: gs.is_pressed(0, GamepadButton::South),
            b: gs.is_pressed(0, GamepadButton::East),
            x: gs.is_pressed(0, GamepadButton::West),
            y: gs.is_pressed(0, GamepadButton::North),
            lt: gs.axis(0, GamepadAxis::LeftTrigger),
            rt: gs.axis(0, GamepadAxis::RightTrigger),
        }
    }

    fn active(&self) -> bool {
        self.lx.abs() > 0.2
            || self.ly.abs() > 0.2
            || self.rx.abs() > 0.2
            || self.ry.abs() > 0.2
            || self.a
            || self.b
            || self.x
            || self.y
            || self.lt > 0.2
            || self.rt > 0.2
    }
}

/// What Apple's GameController framework reports for the first connected pad (macOS only).
#[cfg(target_os = "macos")]
mod gc {
    use objc2_game_controller::GCController;

    #[derive(Default)]
    pub struct GcView {
        pub present: bool,
        pub extended: bool,
        pub lx: f32,
        pub ly: f32,
        pub a: bool,
        pub b: bool,
        pub x: bool,
        pub y: bool,
        pub lt: f32,
        pub rt: f32,
    }

    impl GcView {
        pub fn active(&self) -> bool {
            self.lx.abs() > 0.2
                || self.ly.abs() > 0.2
                || self.a
                || self.b
                || self.x
                || self.y
                || self.lt > 0.2
                || self.rt > 0.2
        }
    }

    /// Reads the first GameController-framework pad's live state. Returns `present = false` when
    /// the framework sees no controller.
    pub fn read() -> GcView {
        // SAFETY: every GameController accessor below is called on the main thread (engine systems
        // run there), which is where the framework services its run loop and updates element
        // state. Each call only reads a retained snapshot value — no mutation, no escaping pointer.
        unsafe {
            let controllers = GCController::controllers();
            let Some(controller) = controllers.first_retained() else {
                return GcView::default();
            };
            let Some(gp) = controller.extendedGamepad() else {
                // A controller exists but exposes no extended (dual-stick) profile.
                return GcView {
                    present: true,
                    ..GcView::default()
                };
            };
            let ls = gp.leftThumbstick();
            GcView {
                present: true,
                extended: true,
                lx: ls.xAxis().value(),
                ly: ls.yAxis().value(),
                a: gp.buttonA().isPressed(),
                b: gp.buttonB().isPressed(),
                x: gp.buttonX().isPressed(),
                y: gp.buttonY().isPressed(),
                lt: gp.leftTrigger().value(),
                rt: gp.rightTrigger().value(),
            }
        }
    }
}

#[derive(Default)]
struct GamepadProbe {
    /// Seconds since the last stdout log line (throttles the per-frame log to ~0.3 s).
    log_accum: f32,
}

impl System for GamepadProbe {
    fn run(&mut self, world: &mut World, dt: f32) {
        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        let hid = world
            .resource::<GamepadState>()
            .map(HidView::read)
            .unwrap_or_default();

        // Read the GameController view once (macOS) — reused by both the stdout log and the overlay.
        #[cfg(target_os = "macos")]
        let g = gc::read();

        // Throttled stdout log so the result is *capturable in the terminal* (the on-screen overlay
        // is for live viewing; this prints which backend actually receives input, ~0.3 s apart and
        // only while a pad is being moved/pressed — so an idle run stays quiet).
        self.log_accum += dt;
        if self.log_accum >= 0.3 {
            self.log_accum = 0.0;
            #[cfg(target_os = "macos")]
            if hid.active() || g.active() {
                let tag = match (hid.active(), g.active()) {
                    (false, true) => "GC-only (HID blind — GameController backend is the fix)",
                    (true, true) => "both",
                    (true, false) => "HID-only",
                    (false, false) => unreachable!(),
                };
                println!(
                    "[gamepad_probe] {tag}\n    gilrs/HID    L({:+.2},{:+.2}) A{}B{}X{}Y{} LT{:.2} RT{:.2}\n    GameController L({:+.2},{:+.2}) A{}B{}X{}Y{} LT{:.2} RT{:.2}",
                    hid.lx, hid.ly, hid.a as u8, hid.b as u8, hid.x as u8, hid.y as u8, hid.lt, hid.rt,
                    g.lx, g.ly, g.a as u8, g.b as u8, g.x as u8, g.y as u8, g.lt, g.rt,
                );
            }
            #[cfg(not(target_os = "macos"))]
            if hid.active() {
                println!(
                    "[gamepad_probe] gilrs/HID L({:+.2},{:+.2}) A{}B{}X{}Y{} LT{:.2} RT{:.2}",
                    hid.lx,
                    hid.ly,
                    hid.a as u8,
                    hid.b as u8,
                    hid.x as u8,
                    hid.y as u8,
                    hid.lt,
                    hid.rt,
                );
            }
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        let title = Color::rgb(0.95, 0.95, 0.98);
        let head = Color::rgb(0.6, 0.85, 1.0);
        let val = Color::rgb(0.85, 0.9, 0.78);
        let dim = Color::rgb(0.55, 0.58, 0.66);
        let line = |s: bool| if s { "■" } else { "□" };

        tq.push(DrawText::new(
            "gamepad_probe — gilrs (IOKit-HID) vs macOS GameController, side by side",
            Vec2::new(30.0, 22.0),
            18.0,
            title,
        ));
        tq.push(DrawText::new(
            "Move the LEFT STICK / press A·B·X·Y.  ESC quits.",
            Vec2::new(30.0, 48.0),
            14.0,
            dim,
        ));

        // ── Left column: gilrs (HID), as the engine's GamepadState sees it. ──
        let lx0 = 40.0;
        tq.push(DrawText::new(
            "gilrs (IOKit-HID)",
            Vec2::new(lx0, 96.0),
            16.0,
            head,
        ));
        let hid_lines = [
            format!(
                "connected: {}    primary slot: {}",
                hid.connected,
                hid.primary
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".into())
            ),
            format!("L-stick   x {:+.2}   y {:+.2}", hid.lx, hid.ly),
            format!("R-stick   x {:+.2}   y {:+.2}", hid.rx, hid.ry),
            format!(
                "buttons   A {}  B {}  X {}  Y {}",
                line(hid.a),
                line(hid.b),
                line(hid.x),
                line(hid.y)
            ),
            format!("triggers  L {:.2}   R {:.2}", hid.lt, hid.rt),
        ];
        for (i, s) in hid_lines.iter().enumerate() {
            tq.push(DrawText::new(
                s,
                Vec2::new(lx0, 126.0 + i as f32 * 24.0),
                15.0,
                val,
            ));
        }

        // ── Right column: GameController framework (macOS only). ──
        let rx0 = 400.0;
        tq.push(DrawText::new(
            "GameController (Apple)",
            Vec2::new(rx0, 96.0),
            16.0,
            head,
        ));

        #[cfg(target_os = "macos")]
        {
            let gc_lines = [
                format!("controller present: {}", g.present),
                format!(
                    "extended profile:   {}",
                    if g.extended { "yes" } else { "no" }
                ),
                format!("L-stick   x {:+.2}   y {:+.2}", g.lx, g.ly),
                format!(
                    "buttons   A {}  B {}  X {}  Y {}",
                    line(g.a),
                    line(g.b),
                    line(g.x),
                    line(g.y)
                ),
                format!("triggers  L {:.2}   R {:.2}", g.lt, g.rt),
            ];
            for (i, s) in gc_lines.iter().enumerate() {
                tq.push(DrawText::new(
                    s,
                    Vec2::new(rx0, 126.0 + i as f32 * 24.0),
                    15.0,
                    val,
                ));
            }

            // Self-explaining verdict.
            let (verdict, vc) = if g.active() && !hid.active() {
                (
                    "→ GameController receives input but gilrs/HID does not: the macOS \
                     GameController backend is the fix (matches the known issue).",
                    Color::rgb(1.0, 0.78, 0.4),
                )
            } else if hid.active() {
                (
                    "→ gilrs/HID receives input on this setup — no backend change needed here.",
                    Color::rgb(0.6, 0.95, 0.6),
                )
            } else {
                (
                    "→ no input yet — move the left stick or press a face button.",
                    dim,
                )
            };
            tq.push(DrawText::new(verdict, Vec2::new(40.0, 300.0), 15.0, vc));
        }
        #[cfg(not(target_os = "macos"))]
        {
            tq.push(DrawText::new(
                "macOS only — on this OS the gilrs/HID path on the left is the live one.",
                Vec2::new(rx0, 126.0),
                14.0,
                dim,
            ));
        }
    }

    fn name(&self) -> &'static str {
        "gamepad_probe"
    }
}

fn main() {
    println!(
        "gamepad_probe: move the LEFT STICK / press A·B·X·Y. While a pad is active this logs \
         gilrs(HID) vs GameController each ~0.3s (and shows both live on-screen). ESC quits."
    );
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "gamepad_probe — gilrs vs GameController".to_string(),
        width: 760,
        height: 360,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.add_system(GamepadProbe::default());
    app.run();
}
