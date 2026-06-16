//! `tween_sequence` — demonstrates [`TweenSequence`] chained tweens.
//!
//! A colored square traces a looping rectangular path using two independent
//! [`TweenSequence`]s — one for x and one for y — with four different easings
//! on each leg so the differences in motion feel are clearly visible:
//!
//! ```text
//!   Right (EaseOut) → Down (EaseInOut) → Left (EaseInBack) → Up (EaseOutBack)
//! ```
//!
//! On-screen text shows the current segment index and overall fraction for both axes.
//!
//! Press **Esc** to quit.

use engine::{
    App, DrawText, Easing, Entity, InputState, KeyCode, ShouldQuit, Sprite, System, TextQueue,
    Transform, TweenSequence, Vec2, WindowConfig, World,
};

const WIN_W: f32 = 800.0;
const WIN_H: f32 = 600.0;

// Margins defining the rectangular path.
const MARGIN_X: f32 = 150.0;
const MARGIN_Y: f32 = 130.0;

// Leg duration (seconds).
const LEG: f32 = 1.2;

// Derived path corners.
const LEFT: f32 = MARGIN_X;
const RIGHT: f32 = WIN_W - MARGIN_X;
const TOP: f32 = MARGIN_Y;
const BOTTOM: f32 = WIN_H - MARGIN_Y;

// ─── Marker component ─────────────────────────────────────────────────────────

/// Marker for the animated square.
#[derive(Clone)]
struct Square;

// ─── System ───────────────────────────────────────────────────────────────────

/// Drives two [`TweenSequence`]s (x and y) that move the square around a rect.
struct SequenceSystem {
    entity: Entity,
    seq_x: TweenSequence,
    seq_y: TweenSequence,
}

impl SequenceSystem {
    fn new(entity: Entity) -> Self {
        // X axis: Left → Right → Right → Left → (loop)
        //   leg 0: right leg (L→R, EaseOut)
        //   leg 1: right side stationary (R→R, Linear)
        //   leg 2: left leg  (R→L, EaseInBack)
        //   leg 3: left side stationary  (L→L, Linear)
        let seq_x = TweenSequence::new()
            .then(LEFT, RIGHT, LEG, Easing::EaseOut)
            .then(RIGHT, RIGHT, LEG, Easing::Linear)
            .then(RIGHT, LEFT, LEG, Easing::EaseInBack)
            .then(LEFT, LEFT, LEG, Easing::Linear)
            .looping(true);

        // Y axis: offset by two legs so x and y are 90° out of phase.
        //   leg 0: top side stationary (T→T, Linear)
        //   leg 1: down leg (T→B, EaseInOut)
        //   leg 2: bottom stationary (B→B, Linear)
        //   leg 3: up leg   (B→T, EaseOutBack)
        let seq_y = TweenSequence::new()
            .then(TOP, TOP, LEG, Easing::Linear)
            .then(TOP, BOTTOM, LEG, Easing::EaseInOut)
            .then(BOTTOM, BOTTOM, LEG, Easing::Linear)
            .then(BOTTOM, TOP, LEG, Easing::EaseOutBack)
            .looping(true);

        Self {
            entity,
            seq_x,
            seq_y,
        }
    }
}

impl System for SequenceSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // ── Input ────────────────────────────────────────────────────────────
        let mut should_quit = false;
        if let Some(input) = world.resource::<InputState>() {
            if input.just_pressed(KeyCode::Escape) {
                should_quit = true;
            }
        }
        if should_quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
            return;
        }

        // ── Advance sequences ─────────────────────────────────────────────
        let x = self.seq_x.tick(dt);
        let y = self.seq_y.tick(dt);

        // ── Update transform ──────────────────────────────────────────────
        // Collect entity into a Vec, then call get_mut (borrow-checker workaround).
        let entities: Vec<Entity> = vec![self.entity];
        for e in entities {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position.x = x;
                t.position.y = y;
            }
        }

        // ── HUD ───────────────────────────────────────────────────────────
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        let seg_names = [
            "Right/EaseOut",
            "Hold/Linear",
            "Left/EaseInBack",
            "Hold/Linear",
        ];
        let seg_y_names = [
            "Hold/Linear",
            "Down/EaseInOut",
            "Hold/Linear",
            "Up/EaseOutBack",
        ];

        let xi = self.seq_x.current_segment().min(seg_names.len() - 1);
        let yi = self.seq_y.current_segment().min(seg_y_names.len() - 1);

        tq.push(DrawText::new(
            "TweenSequence demo  —  Esc to quit",
            Vec2::new(16.0, 12.0),
            18.0,
            [230, 230, 255, 240],
        ));
        tq.push(DrawText::new(
            format!(
                "X  seg {xi}/3  [{:12}]  fraction {:.2}",
                seg_names[xi],
                self.seq_x.fraction()
            ),
            Vec2::new(16.0, 40.0),
            15.0,
            [180, 220, 255, 220],
        ));
        tq.push(DrawText::new(
            format!(
                "Y  seg {yi}/3  [{:12}]  fraction {:.2}",
                seg_y_names[yi],
                self.seq_y.fraction()
            ),
            Vec2::new(16.0, 60.0),
            15.0,
            [180, 255, 200, 220],
        ));
        tq.push(DrawText::new(
            format!("position  ({:.0}, {:.0})", x, y),
            Vec2::new(16.0, 80.0),
            15.0,
            [220, 200, 255, 210],
        ));

        // Corner labels to make the path visible.
        tq.push(DrawText::new(
            "TL",
            Vec2::new(LEFT - 20.0, TOP - 20.0),
            13.0,
            [100, 150, 200, 180],
        ));
        tq.push(DrawText::new(
            "TR",
            Vec2::new(RIGHT + 4.0, TOP - 20.0),
            13.0,
            [100, 150, 200, 180],
        ));
        tq.push(DrawText::new(
            "BR",
            Vec2::new(RIGHT + 4.0, BOTTOM + 4.0),
            13.0,
            [100, 150, 200, 180],
        ));
        tq.push(DrawText::new(
            "BL",
            Vec2::new(LEFT - 20.0, BOTTOM + 4.0),
            13.0,
            [100, 150, 200, 180],
        ));
    }
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();

    app.world.insert_resource(WindowConfig {
        title: "tween_sequence — chained tweens demo".to_string(),
        width: WIN_W as u32,
        height: WIN_H as u32,
        clear_color: [0.06, 0.06, 0.12, 1.0],
    });

    // The animated square.
    let square = app.world.spawn();
    app.world.add_component(
        square,
        Transform {
            position: Vec2::new(LEFT, TOP),
            scale: Vec2::splat(40.0),
            rotation: 0.0,
            z: 0.5,
        },
    );
    app.world
        .add_component(square, Sprite::colored(0.4, 0.8, 1.0));
    app.world.add_component(square, Square);

    app.add_system(SequenceSystem::new(square));

    app.run();
}
