//! `YSort` — depth-sort sprites by world Y for a top-down scene.
//!
//! Overlapping top-down sprites need the one **lower on screen** to draw in front (it is nearer the
//! camera). Add a [`YSort`] component and [`YSortSystem`] writes each entity's `Transform.z` from its
//! `Transform.position.y` every frame, so the sprite renderer (which sorts by `(layer, z)`) overlaps
//! them correctly.
//!
//! This demo stacks three green "trees" down a diagonal and a red "player" you move through them:
//! - **↑ / ↓** — move the player up / down; it weaves in front of lower trees and behind higher ones
//! - **Space** — toggle Y-sort. Off, the sprites draw in spawn order (the player, spawned last,
//!   always sits on top — the overlap bug Y-sort fixes).
//! - **Esc** — quit
//!
//! Headless: `HEADLESS_SHOT=/tmp/ysort.png cargo run --example ysort`.
use engine::{
    App, Camera, Color, DrawText, Entity, InputState, KeyCode, ShouldQuit, Sprite, System,
    TextQueue, Transform, Vec2, WindowConfig, World, YSort, YSortSystem,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 460;
const PLAYER_SPEED: f32 = 170.0;
const PLAYER_MIN_Y: f32 = 70.0;
const PLAYER_MAX_Y: f32 = 390.0;

struct Demo {
    /// All y-sortable entities in spawn order (used to fake an unsorted draw order when toggled off).
    entities: Vec<Entity>,
    player: Entity,
    sorted: bool,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        let (up, down, toggle, quit) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.is_pressed(KeyCode::ArrowUp),
                    i.is_pressed(KeyCode::ArrowDown),
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

        // Move the player along Y (Y increases downward, so Up = -Y).
        let dy = (down as i32 - up as i32) as f32 * PLAYER_SPEED * dt;
        if dy != 0.0 {
            if let Some(t) = world.get_mut::<Transform>(self.player) {
                t.position.y = (t.position.y + dy).clamp(PLAYER_MIN_Y, PLAYER_MAX_Y);
            }
        }

        // Toggle Y-sort by adding / removing the component. With it gone, YSortSystem skips the
        // entity and its z stays at the spawn-order value set below — the player (spawned last) then
        // always draws on top, regardless of Y.
        if toggle {
            self.sorted = !self.sorted;
            for (i, &e) in self.entities.iter().enumerate() {
                if self.sorted {
                    world.add_component(e, YSort::default());
                } else {
                    world.remove_component::<YSort>(e);
                    if let Some(t) = world.get_mut::<Transform>(e) {
                        t.z = i as f32; // spawn order → player (last) on top
                    }
                }
            }
        }

        let py = world
            .get::<Transform>(self.player)
            .map(|t| t.position.y)
            .unwrap_or(0.0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "YSort — depth-sort sprites by world Y (top-down overlap)",
                Vec2::new(40.0, 26.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                format!(
                    "y-sort: {}    player y = {py:.0}",
                    if self.sorted {
                        "ON"
                    } else {
                        "OFF (spawn order)"
                    }
                ),
                Vec2::new(40.0, 52.0),
                15.0,
                if self.sorted {
                    Color::rgb(0.7, 0.95, 0.78)
                } else {
                    Color::rgb(0.95, 0.6, 0.5)
                },
            ));
            tq.push(DrawText::new(
                "Up/Down: move player    Space: toggle y-sort    Esc: quit",
                Vec2::new(40.0, WIN_H as f32 - 28.0),
                15.0,
                Color::rgb(0.95, 0.85, 0.5),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ysort_demo"
    }
}

fn spawn_quad(app: &mut App, pos: Vec2, size: Vec2, color: Color) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: pos,
            scale: size,
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::colored(color.r, color.g, color.b));
    app.world.add_component(e, YSort::default());
    e
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ysort — top-down depth sorting".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    // Three "trees" staggered down a diagonal and overlapping in X, so the lower one must occlude the
    // upper ones once y-sorted (x-step 45 < width 90 → real overlap to order).
    let tree = Color::rgb(0.3, 0.65, 0.35);
    let mut entities = vec![
        spawn_quad(
            &mut app,
            Vec2::new(330.0, 165.0),
            Vec2::new(90.0, 132.0),
            tree,
        ),
        spawn_quad(
            &mut app,
            Vec2::new(375.0, 230.0),
            Vec2::new(90.0, 132.0),
            tree,
        ),
        spawn_quad(
            &mut app,
            Vec2::new(420.0, 295.0),
            Vec2::new(90.0, 132.0),
            tree,
        ),
    ];
    // Player spawns LAST (so it sits on top when y-sort is off), starting between the lower two trees
    // and overlapping both — y-sort draws it over the higher tree and behind the lower one.
    let player = spawn_quad(
        &mut app,
        Vec2::new(398.0, 262.0),
        Vec2::new(72.0, 72.0),
        Color::rgb(0.9, 0.35, 0.3),
    );
    entities.push(player);

    app.add_system(Demo {
        entities,
        player,
        sorted: true,
    });
    app.add_system(YSortSystem);

    // `HEADLESS_SHOT=path` → render to a PNG with no window and exit.
    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(4);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
