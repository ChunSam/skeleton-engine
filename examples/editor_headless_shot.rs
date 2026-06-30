//! Headless editor screenshot — capture the egui editor overlay with **no window/display**.
//!
//! The companion to the headless *game* screenshot (`App::save_screenshot_headless`): that renders
//! only the game scene, this also drives the egui **editor overlay** (EngineStats + Inspector + the
//! keyboard-shortcuts cheatsheet) without a window, so editor UI can be visually verified with the
//! monitor off / asleep / locked, or on a headless CI box.
//!
//! Here it opens the **shortcuts cheatsheet** ([`App::set_editor_shortcuts_visible`]) and captures
//! a frame. Run (native GPU; works headless):
//!
//! ```text
//! HEADLESS_SHOT=/tmp/editor_headless.png cargo run --example editor_headless_shot
//! ```
use engine::{App, Tag, Transform, WindowConfig};

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "editor headless shot".into(),
        width: 900,
        height: 560,
        clear_color: [0.10, 0.12, 0.16, 1.0],
    });

    // A few entities so the editor overlay (EngineStats entity count) has content to show.
    for i in 0..3 {
        let e = app.world.spawn();
        app.world.add_component(e, Tag(format!("Entity {i}")));
        app.world.add_component(e, Transform::default());
    }

    // Open the keyboard-shortcuts cheatsheet (top-left) and push some action-feedback toasts
    // (bottom-right, colour-coded), then capture the editor overlay headlessly.
    app.set_editor_shortcuts_visible(true);
    app.editor_toast("Pasted 2");
    app.editor_toast_success("Scene saved (3)");
    app.editor_toast_error("Load failed: missing.ron");

    let out = std::env::var("HEADLESS_SHOT").unwrap_or_else(|_| "/tmp/editor_headless.png".into());
    app.screenshot_editor_headless(3, &out)
        .expect("headless editor screenshot");
    println!("wrote {out} (editor overlay + cheatsheet + action toasts)");
}
