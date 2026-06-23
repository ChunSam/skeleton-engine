//! Engine **fixed design (virtual) resolution + letterbox** — dogfoods
//! [`DesignResolution`](engine::DesignResolution).
//!
//! The whole UI is authored at a fixed **1280×720** design space. With `DesignResolution` set,
//! the engine reports that size as [`ViewportSize`](engine::ViewportSize) to game systems, renders
//! every `DrawRect`/`DrawText`/sprite in design coordinates, then scales the canvas to the real
//! window with a **uniform, centered scale + letterbox bars**. Cursor input is mapped back into
//! design space, so the marker tracks the OS cursor no matter the window size.
//!
//! The window starts at a non-16:9 size, so the letterbox bars (the dark clear color) are visible
//! immediately. **Resize the window**: the 1280×720 canvas stays centered and identically laid
//! out — just scaled. This is the acceptance test: the same UI at 1280×720 and 1920×1080 looks
//! identical, only larger.
//!
//! Keys: `Esc` quit.
//!
//! Run: `cargo run --example design_resolution`

use engine::{
    App, Camera, DesignResolution, DrawRect, DrawText, InputState, KeyCode, ShouldQuit, System,
    TextQueue, UiQueue, Vec2, ViewportSize, WindowConfig, World,
};

/// The fixed design resolution everything is authored in.
const DW: f32 = 1280.0;
const DH: f32 = 720.0;

struct DesignDemo;

impl System for DesignDemo {
    fn run(&mut self, world: &mut World, _dt: f32) {
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                    sq.0 = true;
                }
            }
        }

        // The cursor is reported in DESIGN space (the engine maps the window cursor back through
        // the letterbox), so this lines up with the design-space UI below at any window size.
        let cursor = world
            .resource::<InputState>()
            .map(|i| i.cursor())
            .unwrap_or(Vec2::ZERO);
        let vp = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((DW, DH));

        if let Some(ui) = world.resource_mut::<UiQueue>() {
            // Design-canvas background — makes the letterbox bars (the clear color) distinct.
            ui.push(DrawRect::new(0.0, 0.0, DW, DH, [22, 24, 32, 255]));
            // Border exactly at the design edges; touches the bars under any letterbox.
            ui.push(DrawRect::new(0.0, 0.0, DW, DH, [90, 170, 250, 255]).with_border(4.0));
            // Center crosshair.
            ui.push(DrawRect::new(
                DW * 0.5 - 1.0,
                0.0,
                2.0,
                DH,
                [60, 70, 90, 200],
            ));
            ui.push(DrawRect::new(
                0.0,
                DH * 0.5 - 1.0,
                DW,
                2.0,
                [60, 70, 90, 200],
            ));
            // Corner markers at the four design corners.
            for (x, y) in [
                (0.0, 0.0),
                (DW - 40.0, 0.0),
                (0.0, DH - 40.0),
                (DW - 40.0, DH - 40.0),
            ] {
                ui.push(DrawRect::new(x, y, 40.0, 40.0, [250, 180, 70, 255]));
            }
            // Cursor marker, drawn at the design-space cursor — proves cursor→design mapping.
            ui.push(
                DrawRect::new(
                    cursor.x - 12.0,
                    cursor.y - 12.0,
                    24.0,
                    24.0,
                    [250, 90, 120, 235],
                )
                .with_corner_radius(12.0),
            );
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::centered(
                "DESIGN 1280 × 720 — letterboxed to the window",
                Vec2::new(DW * 0.5, DH * 0.5 - 22.0),
                28.0,
                [235, 235, 245, 255],
            ));
            tq.push(DrawText::new(
                format!(
                    "ViewportSize (design): {:.0} × {:.0}     cursor (design): {:.0}, {:.0}",
                    vp.0, vp.1, cursor.x, cursor.y
                ),
                Vec2::new(24.0, 22.0),
                18.0,
                [160, 255, 170, 255],
            ));
            tq.push(DrawText::new(
                "Resize the window — the canvas stays centered & identical, just scaled.   Esc quit.",
                Vec2::new(24.0, DH - 38.0),
                16.0,
                [170, 190, 210, 255],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "DesignDemo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine — design resolution (letterbox)".to_string(),
        // Start at a NON-16:9 size so the letterbox bars are visible immediately.
        width: 1000,
        height: 760,
        // The letterbox bars are the clear color.
        clear_color: [0.01, 0.01, 0.015, 1.0],
    });
    // Author everything at 1280×720, regardless of the real window size.
    app.world.insert_resource(DesignResolution::new(DW, DH));
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    app.add_system(DesignDemo);
    app.run();
}
