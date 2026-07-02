//! Game-feel capstone — the juice toolkit + the widget suite composed in one small playable.
//!
//! A tiny training arena that exercises the engine's game-feel helpers **together**, driven live
//! by a real pause/settings menu built from the widget suite:
//!
//! * [`InputBuffer`] — buffered + coyote jumps (jump the gap to the right pad, or walk off it).
//! * [`SpriteTrail`] — a motion trail on the player, preset-switched by a [`Dropdown`].
//! * [`HitFlash`] + [`FloatingText`] + [`Camera::shake`] + a [`TimeScale`] hit-stop — every
//!   landed hit on a training dummy fires the whole juice stack at once.
//! * **Esc** pauses (`TimeScale` 0) and opens a settings panel: a [`TabBar`] switches two pages
//!   of widgets ([`RadioGroup`] shake level, [`Slider`]s for hit-stop / speed / forgiveness
//!   windows, the trail [`Dropdown`]) — and every setting changes the feel the moment you
//!   resume. Set both forgiveness sliders to 0 to feel how unfair a strict jump is.
//!
//! - **← / →** — move · **Space** — jump · **X** — attack the nearest dummy
//! - **Esc** — pause / settings (click, or Tab + arrows) · **Q** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/game_feel.png cargo run --example game_feel`): plays itself —
//! runs in, jumps, lands two hits, then opens the menu, so a capture shows the frozen action
//! (flash + damage numbers + trail) beside the live settings panel. `HEADLESS_FRAMES=N`
//! overrides the default 62.
//!
//! Web (`examples/game_feel/web/build.sh`, then serve that dir): the same arena + settings
//! menu in the browser via `wasm-bindgen` — see the `run_game_feel` entry point.
#[cfg(target_arch = "wasm32")]
use engine::wasm_bindgen;
use engine::{
    spawn_floating_text, App, Camera, Color, DrawRect, DrawText, Dropdown, Entity, Events,
    FloatingText, FloatingTextSystem, HitFlash, HitFlashSystem, InputBuffer, InputState, KeyCode,
    Label, RadioGroup, RealDt, ShouldQuit, Slider, Sprite, SpriteTrail, SpriteTrailSystem, System,
    TabBar, TextQueue, TimeScale, Transform, UiEvent, UiNode, UiQueue, UiSystem, Vec2,
    WindowConfig, World,
};

const WIN_W: u32 = 900;
const WIN_H: u32 = 540;

const PS: f32 = 36.0; // player size
const FLOOR_TOP: f32 = 430.0; // top edge of both pads (world y increases downward)
const PAD_A: (f32, f32) = (40.0, 700.0); // main platform span
const PAD_B: (f32, f32) = (780.0, 880.0); // landing pad across the gap
const SPAWN: Vec2 = Vec2::new(260.0, FLOOR_TOP - PS / 2.0);

const GRAVITY: f32 = 1500.0;
const JUMP_SPEED: f32 = -480.0;
const ATTACK_RANGE: Vec2 = Vec2::new(64.0, 70.0);
const DUMMY_X: [f32; 3] = [480.0, 560.0, 640.0];

/// Widget z for the settings panel — above the game HUD-ish range, far below the dropdown list
/// (90) and tooltips (100).
const MENU_Z: f32 = 6.0;
/// Shake strength per `RadioGroup` option (Off / Low / High).
const SHAKE_LEVELS: [f32; 3] = [0.0, 5.0, 12.0];
const TRAIL_NAMES: [&str; 3] = ["Off", "Subtle", "Heavy"];

/// The settings-menu entities: the tab bar plus one widget group per tab. The demo system
/// toggles their [`UiNode::visible`] each frame (the `ui_tabs` wiring, gated on pause).
struct Menu {
    tab_bar: Entity,
    shake_radio: Entity,
    hitstop_slider: Entity,
    trail_dropdown: Entity,
    speed_slider: Entity,
    buffer_slider: Entity,
    coyote_slider: Entity,
    feel_group: Vec<Entity>,
    move_group: Vec<Entity>,
}

struct Arena {
    player: Entity,
    targets: Vec<Entity>,
    menu: Menu,
    vy: f32,
    jump: InputBuffer,
    /// (buffer ms, coyote ms) the current `jump` was built with — rebuilt when a slider moves.
    forgiveness: (u32, u32),
    /// Trail preset currently attached — the dropdown is applied only on change (re-adding a
    /// `SpriteTrail` every frame would reset its emit timer).
    trail_applied: usize,
    hits: u32,
    last_hit: String,
    hitstop: f32,
    paused: bool,
    changes: usize,
    /// Headless "plays itself" mode.
    auto: bool,
    frame: u32,
}

impl Arena {
    fn grounded_at(pos: Vec2, vy: f32) -> bool {
        let feet = pos.y + PS / 2.0;
        let over = (pos.x >= PAD_A.0 && pos.x <= PAD_A.1) || (pos.x >= PAD_B.0 && pos.x <= PAD_B.1);
        over && vy >= 0.0 && feet >= FLOOR_TOP - 0.5
    }

    /// Fire the whole juice stack at the nearest dummy in range: flash + damage number + shake +
    /// hit-stop, each scaled by the live menu settings.
    fn attack(&mut self, world: &mut World, ppos: Vec2, shake_idx: usize, hitstop_ms: f32) {
        let mut best: Option<(Entity, Vec2)> = None;
        for &t in &self.targets {
            if let Some(tr) = world.get::<Transform>(t) {
                let d = tr.position - ppos;
                if d.x.abs() <= ATTACK_RANGE.x && d.y.abs() <= ATTACK_RANGE.y {
                    let closer = best
                        .map(|(_, bp)| d.x.abs() < (bp - ppos).x.abs())
                        .unwrap_or(true);
                    if closer {
                        best = Some((t, tr.position));
                    }
                }
            }
        }
        let Some((target, tpos)) = best else { return };

        self.hits += 1;
        let idx = ((self.hits - 1) % 4) as usize;
        let dmg = [18, 24, 31, 47][idx];
        let crit = idx == 3;

        // Re-adding a HitFlash mid-flash would capture the flashed color as the restore base —
        // skip instead (the dummy is already flashing).
        if world.get::<HitFlash>(target).is_none() {
            world.add_component(target, HitFlash::white(0.22));
        }
        let (size, color, text) = if crit {
            (24.0, Color::rgb(1.0, 0.55, 0.2), format!("{dmg}!"))
        } else {
            (16.0, Color::rgb(1.0, 0.92, 0.6), dmg.to_string())
        };
        // Stagger stacked numbers sideways so rapid hits on one dummy stay readable.
        let dx = ((self.hits % 3) as f32 - 1.0) * 26.0;
        spawn_floating_text(
            world,
            tpos + Vec2::new(dx, -40.0),
            FloatingText::colored(text, color).with_size(size),
        );
        let shake = SHAKE_LEVELS[shake_idx] * if crit { 1.6 } else { 1.0 };
        if shake > 0.0 {
            if let Some(cam) = world.resource_mut::<Camera>() {
                cam.shake(shake, 0.3);
            }
        }
        if hitstop_ms > 0.0 {
            self.hitstop = hitstop_ms / 1000.0;
        }
        self.last_hit = if crit {
            format!("{dmg} CRIT")
        } else {
            dmg.to_string()
        };
    }
}

fn set_visible(world: &mut World, entities: &[Entity], on: bool) {
    for &e in entities {
        if let Some(node) = world.get_mut::<UiNode>(e) {
            node.visible = on;
        }
    }
}

impl System for Arena {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.frame = self.frame.wrapping_add(1);
        let real_dt = world.resource::<RealDt>().map(|r| r.0).unwrap_or(dt);

        // --- input (live or scripted) ---
        let (mut move_dir, mut jump_pressed, mut attack_pressed, pause_toggled, quit) = world
            .resource::<InputState>()
            .map(|i| {
                let dir = (i.is_pressed(KeyCode::ArrowRight) as i32
                    - i.is_pressed(KeyCode::ArrowLeft) as i32) as f32;
                (
                    dir,
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::KeyX),
                    i.just_pressed(KeyCode::Escape),
                    i.just_pressed(KeyCode::KeyQ),
                )
            })
            .unwrap_or((0.0, false, false, false, false));
        if self.auto {
            // Run in, jump, land two hits, then open the menu for the capture.
            move_dir = if self.frame < 42 { 1.0 } else { 0.0 };
            jump_pressed = self.frame == 6;
            attack_pressed = self.frame == 44 || self.frame == 50;
            if self.frame == 54 {
                self.paused = true;
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if pause_toggled {
            self.paused = !self.paused;
        }

        // --- live menu settings (polled; widgets emit UiEvents only on actual change) ---
        let shake_idx = world
            .get::<RadioGroup>(self.menu.shake_radio)
            .map(|r| r.selected_index())
            .unwrap_or(1);
        let hitstop_ms = world
            .get::<Slider>(self.menu.hitstop_slider)
            .map(|s| s.value)
            .unwrap_or(90.0);
        let trail_idx = world
            .get::<Dropdown>(self.menu.trail_dropdown)
            .map(|d| d.selected_index())
            .unwrap_or(1);
        let move_speed = world
            .get::<Slider>(self.menu.speed_slider)
            .map(|s| s.value)
            .unwrap_or(240.0);
        let buffer_ms = world
            .get::<Slider>(self.menu.buffer_slider)
            .map(|s| s.value)
            .unwrap_or(120.0);
        let coyote_ms = world
            .get::<Slider>(self.menu.coyote_slider)
            .map(|s| s.value)
            .unwrap_or(100.0);

        if trail_idx != self.trail_applied {
            self.trail_applied = trail_idx;
            match trail_idx {
                0 => world.remove_component::<SpriteTrail>(self.player),
                1 => world.add_component(
                    self.player,
                    SpriteTrail::new(0.055, 0.30).with_start_alpha(0.35),
                ),
                _ => world.add_component(
                    self.player,
                    SpriteTrail::new(0.03, 0.55).with_start_alpha(0.70),
                ),
            }
        }
        let forgiveness = (buffer_ms.round() as u32, coyote_ms.round() as u32);
        if forgiveness != self.forgiveness {
            self.forgiveness = forgiveness;
            self.jump =
                InputBuffer::new(forgiveness.0 as f32 / 1000.0, forgiveness.1 as f32 / 1000.0);
        }

        // Menu visibility: tab bar whenever paused, each widget group only on its tab.
        let active = world
            .get::<TabBar>(self.menu.tab_bar)
            .map(|t| t.selected_index())
            .unwrap_or(0);
        set_visible(world, &[self.menu.tab_bar], self.paused);
        set_visible(world, &self.menu.feel_group, self.paused && active == 0);
        set_visible(world, &self.menu.move_group, self.paused && active == 1);

        // Count widget events (each fires only on an actual change — the bus must be registered).
        if let Some(events) = world.resource::<Events<UiEvent>>() {
            self.changes += events
                .read()
                .iter()
                .filter(|e| {
                    matches!(
                        e,
                        UiEvent::RadioChanged(..)
                            | UiEvent::SliderChanged(..)
                            | UiEvent::DropdownChanged(..)
                            | UiEvent::TabChanged(..)
                    )
                })
                .count();
        }

        // --- pause / hit-stop drive TimeScale; the countdown runs in REAL time ---
        if self.hitstop > 0.0 {
            self.hitstop -= real_dt;
        }
        let scale = if self.paused {
            0.0
        } else if self.hitstop > 0.0 {
            0.06
        } else {
            1.0
        };
        if let Some(ts) = world.resource_mut::<TimeScale>() {
            ts.set(scale);
        }

        // --- gameplay sim (scaled dt: frozen while paused, crawling during hit-stop) ---
        if !self.paused {
            let dt = dt.min(1.0 / 30.0);
            let mut pos = world
                .get::<Transform>(self.player)
                .map(|t| t.position)
                .unwrap_or(SPAWN);

            pos.x = (pos.x + move_dir * move_speed * dt).clamp(20.0, WIN_W as f32 - 20.0);
            self.vy += GRAVITY * dt;
            pos.y += self.vy * dt;
            let over =
                (pos.x >= PAD_A.0 && pos.x <= PAD_A.1) || (pos.x >= PAD_B.0 && pos.x <= PAD_B.1);
            if over && self.vy >= 0.0 && pos.y + PS / 2.0 >= FLOOR_TOP {
                pos.y = FLOOR_TOP - PS / 2.0;
                self.vy = 0.0;
            }
            if pos.y > WIN_H as f32 + 80.0 {
                pos = SPAWN;
                self.vy = 0.0;
            }

            // InputBuffer contract: set_grounded → press → try_consume → tick LAST.
            let grounded = Arena::grounded_at(pos, self.vy);
            self.jump.set_grounded(grounded);
            if jump_pressed {
                self.jump.press();
            }
            if self.jump.try_consume() {
                self.vy = JUMP_SPEED;
            }
            self.jump.tick(dt);

            if let Some(t) = world.get_mut::<Transform>(self.player) {
                t.position = pos;
            }
            if attack_pressed {
                self.attack(world, pos, shake_idx, hitstop_ms);
            }
        }

        // --- menu backdrop (game-drawn; the widgets render above it) ---
        if self.paused {
            if let Some(uq) = world.resource_mut::<UiQueue>() {
                uq.push(
                    DrawRect::new(
                        32.0,
                        64.0,
                        360.0,
                        420.0,
                        Color::rgba(0.10, 0.11, 0.16, 0.96),
                    )
                    .with_corner_radius(10.0)
                    .with_z(MENU_Z - 0.1),
                );
            }
        }

        // --- HUD (no-z DrawText: the always-on-top pass) ---
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "game_feel — juice toolkit + live settings menu",
                Vec2::new(28.0, 20.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            let status = if self.paused {
                "PAUSED — Esc resumes; settings apply live".to_string()
            } else {
                format!(
                    "hits: {}   last: {}   trail: {}   shake: {}   menu changes: {}",
                    self.hits,
                    self.last_hit,
                    TRAIL_NAMES[self.trail_applied],
                    ["Off", "Low", "High"][shake_idx],
                    self.changes,
                )
            };
            tq.push(DrawText::new(
                status,
                Vec2::new(28.0, 46.0),
                15.0,
                Color::rgb(0.75, 0.9, 0.95),
            ));
            tq.push(DrawText::new(
                "←/→ move   Space jump (buffered + coyote)   X attack   Esc settings   Q quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "game_feel_arena"
    }
}

/// Spawn a hidden menu widget: `UiNode` at panel z, invisible until the menu opens.
fn menu_node(x: f32, y: f32, w: f32, h: f32) -> UiNode {
    UiNode::new(x, y, w, h).with_z(MENU_Z).with_visible(false)
}

fn spawn_menu_label(world: &mut World, x: f32, y: f32, text: &str) -> Entity {
    let e = world.spawn();
    world.add_component(e, menu_node(x, y, 300.0, 18.0));
    world.add_component(
        e,
        Label::new(text)
            .with_font_size(14.0)
            .with_color(Color::rgb(0.72, 0.76, 0.88)),
    );
    e
}

/// Builds the whole example — arena, menu widgets, systems — shared verbatim by the native
/// `main()` (windowed + `HEADLESS_SHOT` capture) and the wasm `run_game_feel()` entry, so the
/// browser runs the exact same setup instead of a drifting hand-copied one.
fn build_app() -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "game_feel — juice toolkit + settings menu".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.register_event::<UiEvent>(); // without the bus, every widget event is silently dropped

    // ── Arena: two pads with a jumpable gap, three training dummies ───────────
    for (l, r) in [PAD_A, PAD_B] {
        let pad = app.world.spawn();
        app.world.add_component(
            pad,
            Transform {
                position: Vec2::new((l + r) / 2.0, FLOOR_TOP + 30.0),
                scale: Vec2::new(r - l, 60.0),
                ..Default::default()
            },
        );
        app.world
            .add_component(pad, Sprite::colored(0.28, 0.32, 0.4));
    }
    let mut targets = Vec::new();
    for x in DUMMY_X {
        let dummy = app.world.spawn();
        app.world.add_component(
            dummy,
            Transform {
                position: Vec2::new(x, FLOOR_TOP - 22.0),
                scale: Vec2::new(28.0, 44.0),
                ..Default::default()
            },
        );
        app.world
            .add_component(dummy, Sprite::colored(0.75, 0.42, 0.38));
        targets.push(dummy);
    }
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: SPAWN,
            scale: Vec2::new(PS, PS),
            ..Default::default()
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.95, 0.75, 0.35));
    app.world.add_component(
        player,
        SpriteTrail::new(0.055, 0.30).with_start_alpha(0.35), // the "Subtle" preset
    );

    // ── Settings menu (hidden until Esc): TabBar + two widget pages ───────────
    let tab_bar = app.world.spawn();
    app.world
        .add_component(tab_bar, menu_node(52.0, 84.0, 320.0, 30.0));
    app.world.add_component(
        tab_bar,
        TabBar::new(["Feel", "Movement"]).with_corner_radius(6.0),
    );

    // Page 0 — Feel: shake level, hit-stop length, trail preset.
    let shake_label = spawn_menu_label(&mut app.world, 52.0, 130.0, "Screen shake");
    let shake_radio = app.world.spawn();
    app.world
        .add_component(shake_radio, menu_node(52.0, 154.0, 240.0, 78.0));
    app.world.add_component(
        shake_radio,
        RadioGroup::new(["Off", "Low", "High"]).with_selected(1),
    );
    let hitstop_label = spawn_menu_label(&mut app.world, 52.0, 250.0, "Hit-stop on hit (ms)");
    let hitstop_slider = app.world.spawn();
    app.world
        .add_component(hitstop_slider, menu_node(52.0, 276.0, 320.0, 20.0));
    app.world
        .add_component(hitstop_slider, Slider::new(0.0, 150.0, 90.0));
    let trail_label = spawn_menu_label(&mut app.world, 52.0, 318.0, "Motion trail");
    let trail_dropdown = app.world.spawn();
    app.world
        .add_component(trail_dropdown, menu_node(52.0, 342.0, 220.0, 30.0));
    app.world.add_component(
        trail_dropdown,
        Dropdown::new(TRAIL_NAMES)
            .with_selected(1)
            .with_corner_radius(4.0),
    );

    // Page 1 — Movement: speed + the two jump-forgiveness windows.
    let speed_label = spawn_menu_label(&mut app.world, 52.0, 130.0, "Move speed");
    let speed_slider = app.world.spawn();
    app.world
        .add_component(speed_slider, menu_node(52.0, 156.0, 320.0, 20.0));
    app.world
        .add_component(speed_slider, Slider::new(120.0, 380.0, 240.0));
    let buffer_label = spawn_menu_label(&mut app.world, 52.0, 200.0, "Jump buffer (ms)");
    let buffer_slider = app.world.spawn();
    app.world
        .add_component(buffer_slider, menu_node(52.0, 226.0, 320.0, 20.0));
    app.world
        .add_component(buffer_slider, Slider::new(0.0, 300.0, 120.0));
    let coyote_label = spawn_menu_label(&mut app.world, 52.0, 270.0, "Coyote time (ms)");
    let coyote_slider = app.world.spawn();
    app.world
        .add_component(coyote_slider, menu_node(52.0, 296.0, 320.0, 20.0));
    app.world
        .add_component(coyote_slider, Slider::new(0.0, 300.0, 100.0));
    let strict_label = spawn_menu_label(
        &mut app.world,
        52.0,
        340.0,
        "Both at 0 = a strict, unfair jump",
    );

    let headless = std::env::var("HEADLESS_SHOT").is_ok();
    app.add_system(UiSystem::new());
    app.add_system(Arena {
        player,
        targets,
        menu: Menu {
            tab_bar,
            shake_radio,
            hitstop_slider,
            trail_dropdown,
            speed_slider,
            buffer_slider,
            coyote_slider,
            feel_group: vec![
                shake_label,
                shake_radio,
                hitstop_label,
                hitstop_slider,
                trail_label,
                trail_dropdown,
            ],
            move_group: vec![
                speed_label,
                speed_slider,
                buffer_label,
                buffer_slider,
                coyote_label,
                coyote_slider,
                strict_label,
            ],
        },
        vy: 0.0,
        jump: InputBuffer::new(0.12, 0.10),
        forgiveness: (120, 100),
        trail_applied: 1,
        hits: 0,
        last_hit: "-".into(),
        hitstop: 0.0,
        paused: false,
        changes: 0,
        auto: headless,
        frame: 0,
    });
    app.add_system(HitFlashSystem);
    app.add_system(FloatingTextSystem);
    app.add_system(SpriteTrailSystem);
    app
}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let mut app = build_app();
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(62);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}

/// WASM entry point — `examples/game_feel/web/index.html` calls this on the "Start" click
/// (winit wants a user gesture before grabbing the canvas). Runs the same arena + live
/// settings menu as native, so the whole juice toolkit is playable in a browser.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn run_game_feel() {
    build_app().run();
}

#[cfg(target_arch = "wasm32")]
fn main() {}
