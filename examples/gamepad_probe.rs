//! gamepad_probe — view the engine's gamepad input, and on macOS cross-check it against the
//! GameController framework.
//!
//! The engine's `GamepadState` is fed by **gilrs** (IOKit-HID) on Windows/Linux, and by the
//! **GameController framework** on macOS — where gilrs can't read modern Bluetooth/USB Xbox & PS5
//! pads, because Apple's GameController driver claims them (their HID device shows
//! `IOUserServerName = com.apple.gamecontroller.driver.*`). See `src/input/gamepad_macos.rs`.
//!
//! This probe shows the engine's `GamepadState` live, and on macOS *also* reads the GameController
//! framework **directly** as an independent cross-check. With the macOS backend working the two
//! columns agree — except stick Y, whose sign is mirrored because the engine negates it to its
//! `AxisBinding` convention (up = −Y) while the raw GameController read reports up = +Y.
//!
//! Move the left stick / press A·B·X·Y. On Windows/Linux the GameController column is absent (the
//! gilrs path feeds `GamepadState` there).
//!
//! Run from the repo root:  `cargo run --example gamepad_probe`   (ESC quits)

use engine::{
    App, Color, DrawText, GamepadAxis, GamepadButton, GamepadState, InputState, KeyCode,
    ShouldQuit, System, TextQueue, Vec2, WindowConfig, World,
};

/// What the engine's unified `GamepadState` reports for pad 0 (GC-backed on macOS, gilrs elsewhere).
#[derive(Default)]
struct EngineView {
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

impl EngineView {
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

        let eng = world
            .resource::<GamepadState>()
            .map(EngineView::read)
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
            if eng.active() || g.active() {
                // engine `GamepadState` is GC-backed on macOS, so it should agree with the raw read.
                let tag = match (eng.active(), g.active()) {
                    (true, true) => {
                        "OK — engine GamepadState matches GameController (backend active)"
                    }
                    (false, true) => {
                        "MISMATCH — GameController sees input but engine GamepadState is blind"
                    }
                    (true, false) => "engine-only (raw GameController read empty?)",
                    (false, false) => unreachable!(),
                };
                println!(
                    "[gamepad_probe] {tag}\n    engine GamepadState  L({:+.2},{:+.2}) A{}B{}X{}Y{} LT{:.2} RT{:.2}\n    GameController (raw) L({:+.2},{:+.2}) A{}B{}X{}Y{} LT{:.2} RT{:.2}",
                    eng.lx, eng.ly, eng.a as u8, eng.b as u8, eng.x as u8, eng.y as u8, eng.lt, eng.rt,
                    g.lx, g.ly, g.a as u8, g.b as u8, g.x as u8, g.y as u8, g.lt, g.rt,
                );
            }
            #[cfg(not(target_os = "macos"))]
            if eng.active() {
                println!(
                    "[gamepad_probe] engine GamepadState L({:+.2},{:+.2}) A{}B{}X{}Y{} LT{:.2} RT{:.2}",
                    eng.lx,
                    eng.ly,
                    eng.a as u8,
                    eng.b as u8,
                    eng.x as u8,
                    eng.y as u8,
                    eng.lt,
                    eng.rt,
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
            "gamepad_probe — engine GamepadState vs raw GameController (macOS)",
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

        // ── Left column: the engine's unified GamepadState (GC-backed on macOS, gilrs elsewhere). ──
        let lx0 = 40.0;
        tq.push(DrawText::new(
            "engine GamepadState",
            Vec2::new(lx0, 96.0),
            16.0,
            head,
        ));
        let eng_lines = [
            format!(
                "connected: {}    primary slot: {}",
                eng.connected,
                eng.primary
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".into())
            ),
            format!("L-stick   x {:+.2}   y {:+.2}", eng.lx, eng.ly),
            format!("R-stick   x {:+.2}   y {:+.2}", eng.rx, eng.ry),
            format!(
                "buttons   A {}  B {}  X {}  Y {}",
                line(eng.a),
                line(eng.b),
                line(eng.x),
                line(eng.y)
            ),
            format!("triggers  L {:.2}   R {:.2}", eng.lt, eng.rt),
        ];
        for (i, s) in eng_lines.iter().enumerate() {
            tq.push(DrawText::new(
                s,
                Vec2::new(lx0, 126.0 + i as f32 * 24.0),
                15.0,
                val,
            ));
        }

        // ── Right column: raw GameController read, an independent cross-check (macOS only). ──
        let rx0 = 400.0;
        tq.push(DrawText::new(
            "GameController (raw)",
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

            // Self-explaining verdict: with the macOS backend working the engine's GamepadState
            // mirrors the raw GameController read (stick Y sign flipped by the engine convention).
            let (verdict, vc) = if eng.active() {
                (
                    "→ engine GamepadState is receiving input — the macOS GameController backend is active. \
                     (Stick Y sign is mirrored vs the raw column by design: engine up = −Y.)",
                    Color::rgb(0.6, 0.95, 0.6),
                )
            } else if g.active() {
                (
                    "→ GameController sees input but the engine's GamepadState does not — backend not feeding it?",
                    Color::rgb(1.0, 0.55, 0.45),
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
                "macOS only — on this OS the engine's GamepadState (left) is fed by gilrs.",
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
