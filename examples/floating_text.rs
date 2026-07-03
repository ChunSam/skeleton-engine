//! `FloatingText` — pop rising, fading damage numbers at a world position, then despawn.
//!
//! The genre-agnostic game-feel staple: hit something and a number floats up off it and fades out.
//! Spawn a dedicated entity carrying a [`FloatingText`] (here via [`spawn_floating_text`]) and
//! [`FloatingTextSystem`] drives it — drifting it up, fading its alpha, and despawning it when it
//! expires. It pairs naturally with a damage event; combine it with a
//! [`HitFlash`](engine::HitFlash) on the thing that was hit for extra juice.
//!
//! This demo shows three targets. Hitting them pops a colored number above each; clicking anywhere
//! pops one at the cursor. Numbers are spawned with [`FloatingText::with_z`] (UI-layer depth), so
//! the toggleable overlay scrim — a higher-z `DrawRect`, the pause-menu pattern — actually covers
//! them: they rise hidden behind it and emerge above its top edge. Press **Z** to switch new pops
//! back to the default on-top text pass and watch them bleed straight through the scrim instead.
//!
//! - **Space** — hit the targets (pop a damage/heal number over each)
//! - **Left-click** — pop a number at the cursor
//! - **P** — toggle the overlay scrim (z 100, covers layered floats)
//! - **Z** — toggle new pops between layered (`with_z(50)`) and default on-top
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/floating_text.png cargo run --example floating_text`): with no
//! input it auto-hits the targets on a timer, so a capture always catches several numbers mid-rise
//! at different heights; the scrim turns on partway through, so the capture also shows numbers
//! cut off at its top edge. `HEADLESS_FRAMES=N` overrides the default 70 warm-up frames.
use engine::{
    spawn_floating_text, App, Camera, Color, DrawRect, DrawText, Entity, FloatingText,
    FloatingTextSystem, InputState, KeyCode, MouseButton, ShouldQuit, Sprite, System, TextQueue,
    Transform, UiQueue, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 460;
const TARGET_Y: f32 = 250.0;
/// Headless: hit the targets every this many frames so several numbers are always in flight.
const AUTO_INTERVAL: u32 = 14;
/// Headless: turn the overlay scrim on after this many frames so the capture shows the cover.
const AUTO_SCRIM_FRAME: u32 = 30;
/// UI-layer depth for the popped numbers (below the scrim — they hide behind it).
const FLOAT_Z: f32 = 50.0;
/// UI-layer depth of the overlay scrim (the pause-menu pattern: above the floats).
const SCRIM_Z: f32 = 100.0;
/// The scrim band (x, y, w, h): full-width, covering the spawn area — rising numbers emerge above
/// its top edge at y = 150.
const SCRIM_RECT: (f32, f32, f32, f32) = (40.0, 150.0, WIN_W as f32 - 80.0, 190.0);

/// One hit's number + color + starting horizontal drift — cycled so successive pops vary without a
/// random-number dependency. Reds are damage, the green "+" is a heal, the big yellow is a crit.
const POPS: &[(&str, Color, f32, f32)] = &[
    ("12", Color::rgb(1.0, 0.55, 0.30), -14.0, 22.0), // damage, drift left
    ("8", Color::rgb(1.0, 0.72, 0.28), 12.0, 22.0),   // damage, drift right
    ("CRIT 27!", Color::rgb(1.0, 0.92, 0.35), 0.0, 30.0), // crit, bigger, straight up
    ("+15", Color::rgb(0.45, 0.90, 0.45), 8.0, 22.0), // heal, drift right
    ("6", Color::rgb(1.0, 0.5, 0.35), -10.0, 22.0),   // damage, drift left
    ("MISS", Color::rgb(0.72, 0.75, 0.82), 0.0, 20.0), // miss, gray
];

struct Target {
    x: f32,
    name: &'static str,
}

struct Demo {
    targets: Vec<Target>,
    /// Headless / "plays itself" mode: hit the targets on a timer instead of waiting for input.
    auto: bool,
    /// Frame counter for the auto timer.
    frame: u32,
    /// Which [`POPS`] entry the next pop uses (advances so successive numbers differ).
    next: usize,
    /// Total numbers spawned (shown in the HUD).
    spawned: u32,
    /// Whether the overlay scrim is up (toggled with **P**; auto mode raises it partway through).
    overlay: bool,
    /// Whether new pops get `with_z(FLOAT_Z)` (default) or the legacy on-top pass (toggle **Z**).
    layered: bool,
}

impl Demo {
    /// Pop the next cycling number from [`POPS`] at `world_pos`.
    fn pop(&mut self, world: &mut World, world_pos: Vec2) {
        let (text, color, drift_x, size) = POPS[self.next % POPS.len()];
        self.next += 1;
        self.spawned += 1;
        let mut ft = FloatingText::colored(text, color)
            .with_velocity(Vec2::new(drift_x, -52.0))
            .with_size(size)
            .with_lifetime(1.1);
        // Layered mode (default): the number composites among the UI rects at FLOAT_Z, so the
        // higher-z scrim covers it. On-top mode: the historical always-on-top text pass.
        if self.layered {
            ft = ft.with_z(FLOAT_Z);
        }
        spawn_floating_text(world, world_pos, ft);
    }
}

impl System for Demo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        self.frame = self.frame.wrapping_add(1);

        // --- input ---
        let (hit, click, click_pos, toggle_overlay, toggle_layered, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Space),
                    i.mouse_just_pressed(MouseButton::Left),
                    i.cursor(),
                    i.just_pressed(KeyCode::KeyP),
                    i.just_pressed(KeyCode::KeyZ),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((false, false, Vec2::ZERO, false, false, false));

        if toggle_overlay {
            self.overlay = !self.overlay;
        }
        if toggle_layered {
            self.layered = !self.layered;
        }
        // Headless: raise the scrim partway through so the capture shows numbers cut off by it.
        if self.auto && self.frame == AUTO_SCRIM_FRAME {
            self.overlay = true;
        }

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Camera at origin / zoom 1 → screen == world, but map through the camera anyway so the demo
        // stays correct if the view is moved/zoomed.
        let cursor_world = world
            .resource::<Camera>()
            .map(|c| c.screen_to_world(click_pos))
            .unwrap_or(click_pos);

        // Space (or the headless auto-timer): hit every target — a number pops above each.
        let auto_fire = self.auto && self.frame.is_multiple_of(AUTO_INTERVAL);
        if hit || auto_fire {
            let xs: Vec<f32> = self.targets.iter().map(|t| t.x).collect();
            for x in xs {
                self.pop(world, Vec2::new(x, TARGET_Y - 46.0));
            }
        }
        // Left-click: pop a single number at the cursor.
        if click {
            self.pop(world, cursor_world);
        }

        // --- overlay scrim (the pause-menu pattern: a high-z rect over the float layer) ---
        if self.overlay {
            if let Some(uq) = world.resource_mut::<UiQueue>() {
                let (x, y, w, h) = SCRIM_RECT;
                uq.push(DrawRect::new(x, y, w, h, [0.10, 0.10, 0.14, 0.94]).with_z(SCRIM_Z));
            }
        }

        // --- HUD ---
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "FloatingText — pop rising, fading numbers, then despawn",
                Vec2::new(32.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!(
                    "numbers popped: {}    new pops: {}    overlay: {}",
                    self.spawned,
                    if self.layered {
                        "layered (with_z 50)"
                    } else {
                        "on-top (no z)"
                    },
                    if self.overlay { "ON (z 100)" } else { "off" },
                ),
                Vec2::new(32.0, 50.0),
                15.0,
                Color::rgb(0.75, 0.92, 0.95),
            ));
            if self.overlay {
                let (x, y, ..) = SCRIM_RECT;
                tq.push(DrawText::new(
                    "scrim (z 100) — layered floats hide behind it, emerging above",
                    Vec2::new(x + 14.0, y + 12.0),
                    14.0,
                    Color::rgb(0.62, 0.65, 0.75),
                ));
            }
            tq.push(DrawText::new(
                "Space: hit    Click: pop at cursor    P: overlay    Z: layered/on-top    Esc: quit",
                Vec2::new(32.0, WIN_H as f32 - 28.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
            for t in &self.targets {
                tq.push(DrawText::centered(
                    t.name,
                    Vec2::new(t.x, TARGET_Y + 60.0),
                    14.0,
                    Color::rgb(0.7, 0.72, 0.78),
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "floating_text_demo"
    }
}

fn spawn_target(app: &mut App, name: &'static str, x: f32, rest: Color) -> Target {
    let e: Entity = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(x, TARGET_Y),
            scale: Vec2::new(80.0, 80.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::colored(rest.r, rest.g, rest.b));
    Target { x, name }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "floating_text — rising damage numbers".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    let targets = vec![
        spawn_target(&mut app, "goblin", 200.0, Color::rgb(0.40, 0.55, 0.30)),
        spawn_target(&mut app, "slime", 380.0, Color::rgb(0.30, 0.55, 0.62)),
        spawn_target(&mut app, "wisp", 560.0, Color::rgb(0.52, 0.38, 0.70)),
    ];

    let headless = std::env::var("HEADLESS_SHOT").is_ok();

    // Order: FloatingTextSystem (ages/drifts/despawns) → Demo (spawns new pops). Either order works
    // since a freshly spawned text is only advanced next frame, but driving the system first keeps
    // the mental model "existing numbers update, then new ones appear".
    app.add_system(FloatingTextSystem);
    app.add_system(Demo {
        targets,
        auto: headless,
        frame: 0,
        next: 0,
        spawned: 0,
        overlay: false,
        layered: true,
    });

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(70);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
