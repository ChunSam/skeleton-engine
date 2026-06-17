//! save_encrypted — cross-platform **encrypted** player save with `save` / `load` (AEAD).
//!
//! Loads an encrypted launch counter at startup, increments it, and saves it back. Unlike
//! `save_counter` (which uses plain-text `write_ron`), this uses `save` / `load`, which
//! ChaCha20-Poly1305-encrypt the payload — and now work on **both** native (a file) and the web
//! (a hex-encoded blob in `localStorage`), the wasm AEAD parity added in v0.22.0.
//!
//! The saved blob is not human-readable and is tamper-detected (a flipped byte fails to load).
//! The key ships in the binary, so this is obfuscation + integrity, not secrecy against a user
//! inspecting their own local storage — same trust model on every target.
//!
//! Run from the repo root:  `cargo run --example save_encrypted`   (run a few times; ESC quits)

use engine::{
    save, App, Color, DrawText, InputState, KeyCode, ShouldQuit, System, TextQueue, Vec2,
    WindowConfig, World,
};

struct HudSystem {
    count: u32,
}

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(q) = world.resource_mut::<ShouldQuit>() {
                    q.quit();
                }
            }
        }
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Launched {} time(s) — loaded from an encrypted save.",
                    self.count
                ),
                Vec2::new(60.0, 250.0),
                26.0,
                Color::WHITE,
            ));
            tq.push(DrawText::new(
                "save/load = ChaCha20-Poly1305 AEAD: a file natively, hex in localStorage on the web. ESC to quit.",
                Vec2::new(60.0, 300.0),
                16.0,
                Color::rgb(0.6, 0.7, 0.85),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "hud_system"
    }
}

fn main() {
    // Load (decrypt) → increment → save (encrypt). `load_or_default` returns 0 on the first run
    // (no save yet); a corrupt/tampered blob also falls back to 0 here.
    let path = save::save_path("skeleton_save_demo", "launches_enc.sav");
    let count: u32 = save::load_or_default(&path).unwrap_or(0) + 1;
    if let Err(e) = save::save(&path, &count) {
        // On a platform without persistence the count simply won't stick — log and carry on.
        log::warn!("save_encrypted: could not persist launch count: {e}");
    }

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "save_encrypted — encrypted persistent launch count".to_string(),
        width: 820,
        height: 520,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.add_system(HudSystem { count });
    app.run();
}
