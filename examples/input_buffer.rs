//! `InputBuffer` — input buffering + coyote time, the two tricks that make a jump feel fair.
//!
//! A jump pressed a few frames *before* landing should still fire the instant you touch down
//! (**input buffering**); a jump pressed a few frames *after* walking off a ledge should still fire
//! (**coyote time**). [`InputBuffer`] is a tiny pure-logic helper you drive yourself — feed it the
//! ground state + button press each frame and ask it whether to jump.
//!
//! This demo is a single kinematic box on a platform with a gap on the right. Walk off the edge and
//! you can still jump for a split second (coyote); tap jump just before landing and it fires on
//! touchdown (buffered). The HUD shows both windows counting down and labels each jump.
//!
//! - **← / →** — walk (walk off the right edge to try coyote time)
//! - **Space** — jump (tap it early/late to feel the buffer + coyote)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/input_buffer.png cargo run --example input_buffer`): the box
//! auto-walks and jumps on a timer so a capture catches it mid-hop with the HUD live.
//! `HEADLESS_FRAMES=N` overrides the default 40 warm-up frames.
use engine::{
    App, Camera, Color, DrawText, Entity, InputBuffer, InputState, KeyCode, ShouldQuit, Sprite,
    System, TextQueue, Transform, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 460;

const PS: f32 = 40.0; // player size
const FLOOR_TOP: f32 = 360.0; // y of the platform's top edge (world y increases downward)
const LEDGE_L: f32 = 120.0; // platform left/right edges — beyond the right is a gap to fall through
const LEDGE_R: f32 = 520.0;

const GRAVITY: f32 = 1500.0; // world units / s²  (pulls +y = down)
const JUMP_SPEED: f32 = -600.0; // instantaneous upward velocity (−y = up)
const MOVE_SPEED: f32 = 240.0;
const SPAWN: Vec2 = Vec2::new(240.0, FLOOR_TOP - PS / 2.0);

struct Player {
    entity: Entity,
    vy: f32,
    jump: InputBuffer,
    jumps: u32,
    last_kind: &'static str,
    /// Headless "plays itself" mode.
    auto: bool,
    frame: u32,
}

impl Player {
    /// Is the box currently resting on the platform (feet at the floor top, within the ledge span)?
    fn grounded_at(pos: Vec2, vy: f32) -> bool {
        let feet = pos.y + PS / 2.0;
        let over_ledge = pos.x >= LEDGE_L && pos.x <= LEDGE_R;
        over_ledge && vy >= 0.0 && feet >= FLOOR_TOP - 0.5
    }
}

impl System for Player {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.frame = self.frame.wrapping_add(1);
        let dt = dt.min(1.0 / 30.0); // clamp so a stall can't tunnel the box through the floor

        // --- input (live or scripted) ---
        let (mut move_dir, mut jump_pressed, quit) = world
            .resource::<InputState>()
            .map(|i| {
                let dir = (i.is_pressed(KeyCode::ArrowRight) as i32
                    - i.is_pressed(KeyCode::ArrowLeft) as i32) as f32;
                (
                    dir,
                    i.just_pressed(KeyCode::Space),
                    i.just_pressed(KeyCode::Escape),
                )
            })
            .unwrap_or((0.0, false, false));

        if self.auto {
            // Walk right off the ledge, jump a few frames after leaving it (coyote), and periodically
            // reset — so a capture always shows motion + a live HUD.
            move_dir = 0.7;
            jump_pressed = self.frame.is_multiple_of(34);
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // --- kinematic integrate ---
        let mut pos = world
            .get::<Transform>(self.entity)
            .map(|t| t.position)
            .unwrap_or(SPAWN);

        pos.x = (pos.x + move_dir * MOVE_SPEED * dt).clamp(-40.0, WIN_W as f32 + 40.0);
        self.vy += GRAVITY * dt;
        pos.y += self.vy * dt;

        // Land on the platform if descending onto it.
        let over_ledge = pos.x >= LEDGE_L && pos.x <= LEDGE_R;
        if over_ledge && self.vy >= 0.0 && pos.y + PS / 2.0 >= FLOOR_TOP {
            pos.y = FLOOR_TOP - PS / 2.0;
            self.vy = 0.0;
        }
        // Fell through the gap → respawn on the platform.
        if pos.y > WIN_H as f32 + 80.0 {
            pos = SPAWN;
            self.vy = 0.0;
        }

        let grounded = Player::grounded_at(pos, self.vy);

        // --- InputBuffer: set_grounded → press → try_consume → tick (tick last) ---
        self.jump.set_grounded(grounded);
        if jump_pressed {
            self.jump.press();
        }
        if self.jump.try_consume() {
            self.last_kind = if grounded {
                if jump_pressed {
                    "ground"
                } else {
                    "buffered"
                }
            } else {
                "coyote"
            };
            self.vy = JUMP_SPEED;
            self.jumps += 1;
        }
        self.jump.tick(dt);

        if let Some(t) = world.get_mut::<Transform>(self.entity) {
            t.position = pos;
        }

        // --- HUD ---
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "InputBuffer — buffered jump + coyote time",
                Vec2::new(28.0, 22.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            let ground_c = if grounded {
                Color::rgb(0.55, 0.9, 0.55)
            } else {
                Color::rgb(0.95, 0.6, 0.5)
            };
            tq.push(DrawText::new(
                format!("grounded: {}", if grounded { "yes" } else { "no" }),
                Vec2::new(28.0, 52.0),
                15.0,
                ground_c,
            ));
            tq.push(DrawText::new(
                format!(
                    "coyote: {:>4.0} ms    buffer: {:>4.0} ms",
                    self.jump.coyote_remaining() * 1000.0,
                    self.jump.buffered_remaining() * 1000.0
                ),
                Vec2::new(28.0, 74.0),
                15.0,
                Color::rgb(0.75, 0.85, 0.95),
            ));
            tq.push(DrawText::new(
                format!("jumps: {}   last: {}", self.jumps, self.last_kind),
                Vec2::new(28.0, 96.0),
                15.0,
                Color::rgb(0.95, 0.9, 0.6),
            ));
            tq.push(DrawText::new(
                "←/→ walk (off the right edge for coyote)   Space: jump   Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 26.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "input_buffer_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "input_buffer — buffered jump + coyote time".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // Platform (with a gap to the right of LEDGE_R).
    let floor = app.world.spawn();
    app.world.add_component(
        floor,
        Transform {
            position: Vec2::new((LEDGE_L + LEDGE_R) / 2.0, FLOOR_TOP + 40.0),
            scale: Vec2::new(LEDGE_R - LEDGE_L, 80.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(floor, Sprite::colored(0.28, 0.32, 0.4));

    // Player box.
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

    let headless = std::env::var("HEADLESS_SHOT").is_ok();
    app.add_system(Player {
        entity: player,
        vy: 0.0,
        jump: InputBuffer::default(),
        jumps: 0,
        last_kind: "-",
        auto: headless,
        frame: 0,
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(40);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
