//! `Camera::lookahead` — bias the camera ahead of a moving follow target.
//!
//! With a plain smooth-follow the camera centers on the target, so a fast-moving player sees as much
//! behind them as ahead. **Lookahead** leads the view in the target's direction of motion: set
//! [`Camera::lookahead`](engine::Camera::lookahead) (world units) and the camera derives the
//! target's velocity from its movement, eases an offset toward `direction * lookahead` (at
//! [`Camera::lookahead_speed`](engine::Camera::lookahead_speed)), and aims there — so the player
//! sees more of where they are going, and the view recenters when they stop.
//!
//! This demo follows a player across a field of posts via a **camera anchor** kept at
//! `player - viewport/2` (the engine's top-left-anchored follow convention). A screen-space line
//! marks the viewport center: with lookahead **on**, a player moving right sits left of center
//! (the camera leads); toggle it **off** and the player re-centers.
//!
//! - **← / →** — move the player; **Space** — toggle lookahead; **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/camera_lookahead.png cargo run --example camera_lookahead`): the
//! player auto-drifts right so the capture shows the camera leading (player left of the center line).
//! `HEADLESS_FRAMES=N` overrides the default 120 warm-up frames.
use engine::{
    App, Camera, Color, DrawRect, DrawText, Entity, InputState, KeyCode, ShouldQuit, Sprite,
    System, TextQueue, Transform, UiQueue, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 470;
const WORLD_W: f32 = 3200.0;
const PLAYER_SPEED: f32 = 200.0;
const PLAYER_MIN_X: f32 = 300.0;
const PLAYER_MAX_X: f32 = WORLD_W - 300.0;
const BAND_Y: f32 = 300.0;
const LOOKAHEAD_DIST: f32 = 200.0;

struct Demo {
    player: Entity,
    anchor: Entity,
    /// Headless / "plays itself" mode: drift the player regardless of input.
    auto: bool,
    /// Auto-drift direction (+1 right, -1 left); ping-pongs at the world edges.
    dir: f32,
    /// Whether lookahead is currently enabled (Space toggles).
    lookahead_on: bool,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        // --- input ---
        let (left, right, toggle, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.is_pressed(KeyCode::ArrowLeft),
                    i.is_pressed(KeyCode::ArrowRight),
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((false, false, false, false));

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if toggle {
            self.lookahead_on = !self.lookahead_on;
        }

        // --- move the player ---
        let dx = if self.auto {
            self.dir * PLAYER_SPEED * dt
        } else {
            (right as i32 - left as i32) as f32 * PLAYER_SPEED * dt
        };
        let player_pos = if let Some(t) = world.get_mut::<Transform>(self.player) {
            t.position.x += dx;
            if t.position.x <= PLAYER_MIN_X {
                t.position.x = PLAYER_MIN_X;
                self.dir = 1.0;
            } else if t.position.x >= PLAYER_MAX_X {
                t.position.x = PLAYER_MAX_X;
                self.dir = -1.0;
            }
            t.position
        } else {
            Vec2::new(PLAYER_MIN_X, BAND_Y)
        };

        // --- keep the camera anchor at player - viewport/2 (top-left-anchored follow) ---
        let half = Vec2::new(WIN_W as f32, WIN_H as f32) * 0.5;
        if let Some(t) = world.get_mut::<Transform>(self.anchor) {
            t.position = player_pos - half;
        }

        // --- drive the camera's lookahead from the toggle ---
        let (offset_x, cam_pos) = if let Some(cam) = world.resource_mut::<Camera>() {
            cam.lookahead = if self.lookahead_on {
                LOOKAHEAD_DIST
            } else {
                0.0
            };
            (cam.lookahead_offset().x, cam.position)
        } else {
            (0.0, Vec2::ZERO)
        };
        // Player x in screen space (zoom 1 → screen = world - camera.position).
        let player_screen_x = player_pos.x - cam_pos.x;
        let center_x = WIN_W as f32 * 0.5;

        // --- center reference line (screen-space UI) ---
        if let Some(ui) = world.resource_mut::<UiQueue>() {
            ui.push(DrawRect::new(
                center_x - 1.0,
                0.0,
                2.0,
                WIN_H as f32,
                Color::rgba(0.95, 0.95, 0.55, 0.5),
            ));
        }

        // --- HUD ---
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Camera lookahead — lead the view ahead of motion",
                Vec2::new(28.0, 22.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            let state = if self.lookahead_on {
                format!("lookahead: ON ({LOOKAHEAD_DIST:.0})   offset.x: {offset_x:+.0}")
            } else {
                "lookahead: OFF".to_string()
            };
            tq.push(DrawText::new(
                state,
                Vec2::new(28.0, 48.0),
                15.0,
                Color::rgb(0.75, 0.92, 0.95),
            ));
            tq.push(DrawText::new(
                format!(
                    "player screen-x: {player_screen_x:.0}   (center {center_x:.0})  — leading when left of center",
                ),
                Vec2::new(28.0, 70.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
            tq.push(DrawText::new(
                "Arrows: move    Space: toggle lookahead    Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "camera_lookahead_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "camera_lookahead — lead the view ahead of motion".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    // A field of posts so the camera scroll (and the lead) is visible.
    let mut x = 200.0;
    let mut i = 0;
    while x < WORLD_W {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(x, BAND_Y),
                scale: Vec2::new(26.0, 320.0),
                z: -1.0,
                ..Default::default()
            },
        );
        let shade = if i % 2 == 0 { 0.34 } else { 0.24 };
        app.world
            .add_component(e, Sprite::colored(shade, shade + 0.06, shade + 0.12));
        x += 200.0;
        i += 1;
    }

    // Player.
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(PLAYER_MIN_X + 100.0, BAND_Y),
            scale: Vec2::new(40.0, 40.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.96, 0.84, 0.3));

    // Camera follows an anchor kept at player - viewport/2; lookahead leads the aim.
    let anchor = app.world.spawn();
    app.world.add_component(
        anchor,
        Transform {
            position: Vec2::new(PLAYER_MIN_X + 100.0, BAND_Y)
                - Vec2::new(WIN_W as f32, WIN_H as f32) * 0.5,
            ..Default::default()
        },
    );
    let mut camera = Camera::new(Vec2::ZERO, 1.0);
    camera.follow_entity = Some(anchor);
    camera.lerp_factor = 8.0;
    camera.lookahead = LOOKAHEAD_DIST;
    app.world.insert_resource(camera);

    let headless = std::env::var("HEADLESS_SHOT").is_ok();
    app.add_system(Demo {
        player,
        anchor,
        auto: headless,
        dir: 1.0,
        lookahead_on: true,
    });

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(120);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
