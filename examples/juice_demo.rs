//! juice_demo — game-feel building blocks in one place.
//!
//! Demonstrates the Phase-2 "juice" features together, and rescues three older features that
//! shipped without an example:
//!   * **TimeScale** hit-stop — SPACE briefly sets `TimeScale(0.06)` so the world freezes on
//!     impact. The freeze is timed with `RealDt` (real, unscaled dt) so it can end itself even
//!     while everything else is frozen.
//!   * **Tween<Vec2>** + new **easing curves** — the player slides in with `EaseOutBack`, and a
//!     row of sprites bobs with Linear / EaseOutBack / EaseOutBounce / EaseOutElastic.
//!   * **Camera shake** (`Camera::shake`), **FadeTransition** (F), and **PostProcessConfig**
//!     (a vignette that pulses on impact) — all previously undemonstrated.
//!
//! Notice the scaled-vs-real split: during hit-stop the bobbing row freezes (it runs on scaled
//! dt), while the camera shake and vignette keep animating (they use real time).
//!
//! Run from the repo root:  `cargo run --example juice_demo`
//! SPACE = impact · F = screen fade · ESC = quit.

use engine::{
    App, Camera, Color, DrawText, Easing, Entity, FadeTransition, InputState, KeyCode,
    PostProcessConfig, RealDt, ShouldQuit, Sprite, System, TextQueue, TimeScale, Transform, Tween,
    Vec2, WindowConfig, World,
};

const ROW_Y: f32 = 180.0;
const ROW_AMP: f32 = 190.0;
const ROW_PERIOD: f32 = 1.8;

struct JuiceSystem {
    player: Entity,
    /// Slide-in animation; dropped once finished.
    intro: Option<Tween<Vec2>>,
    /// (entity, easing curve, fixed x) for the bobbing showcase row.
    row: Vec<(Entity, Easing, f32)>,
    row_t: f32,
    /// Remaining hit-stop freeze, in real seconds.
    hitstop: f32,
    /// Vignette pulse amount (1.0 on impact, decays to 0).
    vignette: f32,
    /// 0 = idle, 1 = fading out, 2 = fading in.
    fade_stage: u8,
    fade_t: f32,
}

impl System for JuiceSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // 1. Read input first (immutable borrow), then mutate resources.
        let (mut impact, mut fade, mut quit) = (false, false, false);
        if let Some(input) = world.resource::<InputState>() {
            impact = input.just_pressed(KeyCode::Space);
            fade = input.just_pressed(KeyCode::KeyF);
            quit = input.just_pressed(KeyCode::Escape);
        }
        // Real (unscaled) dt — used by anything that must keep running during hit-stop.
        let real_dt = world.resource::<RealDt>().map(|r| r.0).unwrap_or(dt);

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // 2. Impact → start hit-stop, kick the camera, pulse the vignette.
        if impact {
            self.hitstop = 0.18;
            self.vignette = 1.0;
            if let Some(cam) = world.resource_mut::<Camera>() {
                cam.shake(16.0, 0.5);
            }
        }

        // Count the freeze down in REAL time, then drive TimeScale.
        if self.hitstop > 0.0 {
            self.hitstop -= real_dt;
        }
        let ts = if self.hitstop > 0.0 { 0.06 } else { 1.0 };
        if let Some(time_scale) = world.resource_mut::<TimeScale>() {
            time_scale.set(ts);
        }

        // 3. Vignette pulse decays in real time and feeds the post-process pass.
        self.vignette = (self.vignette - real_dt * 2.5).max(0.0);
        if let Some(pp) = world.resource_mut::<PostProcessConfig>() {
            pp.enabled = true;
            pp.vignette_radius = 0.7;
            pp.vignette_strength = 0.22 + self.vignette * 0.6;
        }

        // 4. Fade cycle: F starts fade-out → fade-in. FadeTransition is auto-updated by App;
        //    we only time when to flip stages (with real dt, so a frozen frame still fades).
        if fade && self.fade_stage == 0 {
            world.insert_resource(FadeTransition::fade_out(0.35));
            self.fade_stage = 1;
            self.fade_t = 0.35;
        }
        if self.fade_stage != 0 {
            self.fade_t -= real_dt;
            if self.fade_t <= 0.0 {
                if self.fade_stage == 1 {
                    world.insert_resource(FadeTransition::fade_in(0.4));
                    self.fade_stage = 2;
                    self.fade_t = 0.4;
                } else {
                    self.fade_stage = 0;
                }
            }
        }

        // 5. Player slide-in (Tween<Vec2>) — gameplay, so it uses the scaled dt.
        if let Some(tween) = self.intro.as_mut() {
            let pos = tween.tick(dt);
            let done = tween.finished();
            if let Some(tr) = world.get_mut::<Transform>(self.player) {
                tr.position = pos;
            }
            if done {
                self.intro = None;
            }
        }

        // 6. Easing showcase row — gameplay (scaled dt), so it FREEZES during hit-stop.
        self.row_t += dt;
        let frac = (self.row_t % ROW_PERIOD) / ROW_PERIOD;
        let ping = if frac < 0.5 {
            frac * 2.0
        } else {
            (1.0 - frac) * 2.0
        };
        for (entity, easing, base_x) in &self.row {
            let y = ROW_Y + easing.apply(ping) * ROW_AMP;
            if let Some(tr) = world.get_mut::<Transform>(*entity) {
                tr.position = Vec2::new(*base_x, y);
            }
        }

        // 7. HUD.
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "SPACE: impact (hit-stop + shake)     F: screen fade     ESC: quit",
                Vec2::new(20.0, 18.0),
                20.0,
                Color::WHITE,
            ));
            tq.push(DrawText::new(
                "row easings (L->R): Linear · EaseOutBack · EaseOutBounce · EaseOutElastic",
                Vec2::new(20.0, 44.0),
                16.0,
                Color::rgb(0.7, 0.8, 0.9),
            ));
            if self.hitstop > 0.0 {
                tq.push(DrawText::new(
                    "HIT-STOP",
                    Vec2::new(20.0, 74.0),
                    26.0,
                    Color::rgb(1.0, 0.85, 0.2),
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "juice_system"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "juice_demo — hit-stop · shake · easing · fade".to_string(),
        width: 900,
        height: 600,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    // Enable the post-process pass so the vignette pulse is visible.
    app.world.insert_resource(PostProcessConfig {
        enabled: true,
        ..Default::default()
    });

    // The bobbing easing row.
    let easings = [
        (Easing::Linear, Color::rgb(0.55, 0.6, 0.7)),
        (Easing::EaseOutBack, Color::rgb(0.95, 0.6, 0.3)),
        (Easing::EaseOutBounce, Color::rgb(0.4, 0.85, 0.5)),
        (Easing::EaseOutElastic, Color::rgb(0.8, 0.45, 0.9)),
    ];
    let mut row = Vec::new();
    for (i, (easing, color)) in easings.into_iter().enumerate() {
        let base_x = 150.0 + i as f32 * 200.0;
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(base_x, ROW_Y),
                scale: Vec2::splat(56.0),
                ..Default::default()
            },
        );
        app.world.add_component(
            e,
            Sprite {
                color,
                ..Sprite::colored(1.0, 1.0, 1.0)
            },
        );
        row.push((e, easing, base_x));
    }

    // The player — slides in from the left with an EaseOutBack overshoot.
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(-120.0, 480.0),
            scale: Vec2::splat(80.0),
            ..Default::default()
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.3, 0.9, 0.8));

    app.add_system(JuiceSystem {
        player,
        intro: Some(
            Tween::new(Vec2::new(-120.0, 480.0), Vec2::new(450.0, 480.0), 1.2)
                .with_easing(Easing::EaseOutBack),
        ),
        row,
        row_t: 0.0,
        hitstop: 0.0,
        vignette: 0.0,
        fade_stage: 0,
        fade_t: 0.0,
    });
    app.run();
}
