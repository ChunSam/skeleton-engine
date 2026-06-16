//! `nine_slice` — demonstrates scalable 9-patch panels via [`NineSlice`].
//!
//! A single 64×64 bordered texture is generated at startup, then drawn at many
//! different sizes:
//!
//! - Four [`NineSlice`] panels (wide bar, tall panel, small square, large panel)
//!   whose orange corners stay a fixed 16px while the teal edges and dark center
//!   stretch to fill.
//! - One **naive** sprite using the same texture without `NineSlice`, so you can
//!   see the corners smear when the whole quad is stretched.
//! - One rotating `NineSlice` panel, showing the nine sub-quads rotate rigidly.
//!
//! Controls: Esc quit.

use engine::{
    App, DrawText, Entity, InputState, KeyCode, NineSlice, ShouldQuit, Sprite, System, TextQueue,
    Transform, Vec2, WindowConfig, World,
};
use image::{Rgba, RgbaImage};

const TEX: u32 = 64;
const BORDER: u32 = 16;

/// Writes the bordered panel texture to a temp PNG and returns its path.
///
/// Layout (64×64, 16px border): orange corner blocks, teal edge blocks, a dark
/// translucent center, and a 2px white outer outline — chosen so a naive stretch
/// visibly distorts the corners while a 9-slice keeps them crisp.
fn generate_panel_png() -> String {
    let img = RgbaImage::from_fn(TEX, TEX, |x, y| {
        let in_border_x = !(BORDER..TEX - BORDER).contains(&x);
        let in_border_y = !(BORDER..TEX - BORDER).contains(&y);
        let corner = in_border_x && in_border_y;
        let edge = (in_border_x || in_border_y) && !corner;
        let on_outline = x < 2 || y < 2 || x >= TEX - 2 || y >= TEX - 2;
        if corner {
            if on_outline {
                Rgba([255, 255, 255, 255])
            } else {
                Rgba([240, 140, 40, 255])
            }
        } else if edge {
            if on_outline {
                Rgba([255, 255, 255, 255])
            } else {
                Rgba([50, 180, 200, 255])
            }
        } else {
            Rgba([25, 28, 45, 210])
        }
    });
    let path = std::env::temp_dir().join("skeleton_nine_slice_panel.png");
    img.save(&path).expect("write nine-slice panel png");
    path.to_string_lossy().into_owned()
}

/// Drives the rotating panel, pushes labels, and handles Esc.
struct DemoSystem {
    rotating: Entity,
    elapsed: f32,
}

impl System for DemoSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.elapsed += dt;

        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                if let Some(quit) = world.resource_mut::<ShouldQuit>() {
                    quit.quit();
                }
                return;
            }
        }

        if let Some(tr) = world.get_mut::<Transform>(self.rotating) {
            tr.rotation = self.elapsed * 0.6;
        }

        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };
        let label = |tq: &mut TextQueue, text: &str, x: f32, y: f32| {
            tq.push(DrawText::new(
                text,
                Vec2::new(x, y),
                15.0,
                [225, 230, 240, 230],
            ));
        };
        tq.push(DrawText::new(
            "NineSlice — one 64px texture, many sizes; corners stay 16px",
            Vec2::new(16.0, 16.0),
            18.0,
            [255, 245, 220, 240],
        ));
        label(tq, "wide bar (9-slice)", 70.0, 168.0);
        label(
            tq,
            "same texture, naive stretch -> corners smear",
            70.0,
            262.0,
        );
        label(tq, "large panel (9-slice)", 70.0, 540.0);
        label(tq, "tall (9-slice)", 512.0, 556.0);
        label(tq, "small", 700.0, 192.0);
        label(tq, "rotating (9-slice)", 720.0, 470.0);
        tq.push(DrawText::new(
            "Esc: quit",
            Vec2::new(16.0, 596.0),
            15.0,
            [160, 170, 185, 200],
        ));
    }
}

/// Spawns a textured panel centered at `(cx, cy)` with size `(w, h)`. When
/// `slice` is true the entity gets a [`NineSlice`]; otherwise it stretches naively.
fn spawn_panel(
    app: &mut App,
    handle: &engine::Handle<engine::ImageAsset>,
    cx: f32,
    cy: f32,
    w: f32,
    h: f32,
    slice: bool,
) -> Entity {
    let e = app.world.spawn();
    app.world.add_component(
        e,
        Transform {
            position: Vec2::new(cx, cy),
            scale: Vec2::new(w, h),
            ..Default::default()
        },
    );
    app.world
        .add_component(e, Sprite::with_handle(handle.clone()));
    if slice {
        // 16px corners; the source border is 16/64 = 0.25 of the texture per edge.
        app.world.add_component(
            e,
            NineSlice::uniform(BORDER as f32, BORDER as f32 / TEX as f32),
        );
    }
    e
}

fn main() {
    let path = generate_panel_png();

    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "nine_slice — scalable 9-patch panels".to_string(),
        width: 980,
        height: 620,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let handle = app.load_image(path);

    // Wide bar + the naive-stretch comparison directly below it.
    spawn_panel(&mut app, &handle, 280.0, 130.0, 400.0, 70.0, true);
    spawn_panel(&mut app, &handle, 280.0, 224.0, 400.0, 70.0, false);
    // Large panel, tall panel, small square.
    spawn_panel(&mut app, &handle, 280.0, 430.0, 380.0, 200.0, true);
    spawn_panel(&mut app, &handle, 560.0, 380.0, 110.0, 340.0, true);
    spawn_panel(&mut app, &handle, 740.0, 150.0, 96.0, 96.0, true);
    // Rotating panel.
    let rotating = spawn_panel(&mut app, &handle, 800.0, 380.0, 150.0, 150.0, true);

    app.add_system(DemoSystem {
        rotating,
        elapsed: 0.0,
    });
    app.run();
}
