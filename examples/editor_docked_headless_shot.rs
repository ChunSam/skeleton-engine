//! Headless **docked** editor screenshot — capture the full docked editor layout with **no
//! window/display**.
//!
//! The companion to `editor_headless_shot` (which captures the *overlay* editor): this drives the
//! **docked** layout — top toolbar, left entity list, right inspector, bottom assets panel, and the
//! game scene composited into the central viewport — all with no window, so docked-panel editor UI
//! (the per-entity visibility eye toggle, the inspector, Data Tables, …) can be visually verified
//! with the monitor off / asleep / locked, or on a headless CI box.
//!
//! The central viewport is a debounced offscreen render target, so pass `frames >= 5` to let it warm
//! up and appear (fewer frames still render the side panels, with a placeholder central). Run
//! (native GPU; works headless):
//!
//! ```text
//! HEADLESS_SHOT=/tmp/editor_docked.png cargo run --example editor_docked_headless_shot
//! ```
use engine::{App, Hidden, Sprite, Tag, Transform, Vec2, WindowConfig};

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "docked editor headless shot".into(),
        width: 960,
        height: 600,
        clear_color: [0.10, 0.12, 0.16, 1.0],
    });

    // A small scene: three coloured quads so the central viewport shows real content and the
    // left entity list has named rows to display. In docked mode the scene's viewport is the
    // central-panel rect (window minus the side panels), so the quads are authored within that
    // ~350×520 space, spread horizontally so the Hidden middle quad leaves a visible gap.
    let colors = [(0.9, 0.3, 0.3), (0.3, 0.8, 0.4), (0.4, 0.5, 0.95)];
    let mut entities = Vec::new();
    for (i, (r, g, b)) in colors.iter().enumerate() {
        let e = app.world.spawn();
        app.world.add_component(e, Tag(format!("Quad {i}")));
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(90.0 + i as f32 * 90.0, 250.0),
                scale: Vec2::splat(70.0),
                rotation: 0.0,
                z: 0.0,
            },
        );
        app.world.add_component(e, Sprite::colored(*r, *g, *b));
        entities.push(e);
    }

    // Hide the middle quad: its entity-list row dims and shows the 🙈 (slashed-eye) glyph, and its
    // sprite is suppressed in the central viewport — both visible in the one capture.
    app.world.add_component(entities[1], Hidden);

    // Select the first quad so the right-hand Inspector panel is populated with its components.
    app.editor_select_entity(entities[0]);

    let out = std::env::var("HEADLESS_SHOT").unwrap_or_else(|_| "/tmp/editor_docked.png".into());
    // >= 5 frames so the central scene RT warms up past the 3-frame debounce.
    app.screenshot_editor_docked_headless(6, &out)
        .expect("headless docked editor screenshot");
    println!("wrote {out} (docked editor: toolbar + entity list + inspector + central scene)");
}
