//! hello_sprite — the smallest "image on screen" example.
//!
//! `basic.rs` draws a solid-color rectangle; this one shows the **asset workflow**:
//! load a PNG from disk, then render it as a textured sprite. It is the recommended
//! starting point for your own game — copy this file to `examples/my_game.rs` (any
//! `examples/*.rs` is auto-discovered) and run `cargo run --example my_game`.
//!
//! Run it from the repository root:  `cargo run --example hello_sprite`
//! Press Esc to quit.

use engine::{
    App, Entity, InputState, KeyCode, ShouldQuit, Sprite, System, Transform, Vec2, WindowConfig,
    World,
};

/// Gently spins the sprite (so you can see the app is live) and quits on Esc.
struct SpinSystem {
    sprite: Entity,
}

impl System for SpinSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // Read input first, then mutate resources — the two borrows can't overlap.
        let mut quit = false;
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
        }
        if quit {
            if let Some(should_quit) = world.resource_mut::<ShouldQuit>() {
                should_quit.quit();
            }
        }

        // Spin the one sprite we kept a handle to (no query needed for a single entity).
        if let Some(transform) = world.get_mut::<Transform>(self.sprite) {
            transform.rotation += dt * 0.8;
        }
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "hello_sprite".to_string(),
        width: 800,
        height: 600,
        clear_color: [0.07, 0.09, 0.13, 1.0],
    });

    // 1. Load the image. Asset paths are relative to the repository (cargo workspace)
    //    root, NOT to this source file. `load_image` registers the texture and returns
    //    a handle immediately; the GPU upload happens when the app starts.
    let path = "examples/assets/player.png";
    let handle = app.load_image(path);

    // 2. Spawn an entity. `Transform` says where/how big; `Sprite` says what to draw.
    //    `textured_with_handle` prefers the loaded handle and keeps the path as a fallback.
    let sprite = app.world.spawn();
    app.world.add_component(
        sprite,
        Transform {
            position: Vec2::new(400.0, 300.0), // window center
            scale: Vec2::splat(96.0),          // draw at 96×96 px
            ..Default::default()
        },
    );
    app.world
        .add_component(sprite, Sprite::textured_with_handle(path, Some(handle)));

    // 3. Add the system and run.
    app.add_system(SpinSystem { sprite });
    app.run();
}
