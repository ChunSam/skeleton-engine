//! `Tooltip` — hover popups for game UI widgets.
//!
//! Attach a [`Tooltip`] next to any [`UiNode`] widget (a [`Button`], [`ProgressBar`], [`Label`],
//! …); rest the cursor on the widget and, after [`Tooltip::delay_secs`], the [`UiSystem`] fades a
//! small popup in next to the cursor. Moving off the widget resets the delay; a widget covered by
//! a higher-z panel stays silent. The box auto-sizes to the text (`\n` breaks lines) and clamps to
//! the viewport, and the tooltip text can be rewritten each frame (the health bar's tooltip below
//! tracks its live value).
//!
//! - Hover the **Attack** button, the **health bar**, or the **hint label** — each shows a
//!   different style (multi-line stats / live value / instant custom colors)
//! - **Esc** — quit
//!
//! Headless (`HEADLESS_SHOT=/tmp/ui_tooltip.png cargo run --example ui_tooltip`): a synthetic
//! cursor parks on the button so the capture lands with the tooltip faded in.
//! `HEADLESS_FRAMES=N` overrides the default 45 warm-up frames.
use engine::{
    App, Button, Color, DrawText, Entity, InputState, KeyCode, Label, ProgressBar, ShouldQuit,
    System, TextQueue, Tooltip, UiNode, UiSystem, Vec2, WindowConfig, World,
};

const WIN_W: u32 = 720;
const WIN_H: u32 = 420;

struct Demo {
    health_bar: Entity,
    t: f32,
    /// Headless capture mode: park the cursor on the button so the tooltip shows with no mouse.
    auto_hover: Option<Vec2>,
}

impl System for Demo {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.t += dt;

        if world
            .resource::<InputState>()
            .is_some_and(|i| i.just_pressed(KeyCode::Escape))
        {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }

        // Headless: hold the synthetic cursor on the target widget every frame.
        if let Some(target) = self.auto_hover {
            if let Some(input) = world.resource_mut::<InputState>() {
                input.set_cursor(target);
            }
        }

        // Drive the health bar and keep its tooltip text in sync with the live value.
        let hp = 0.5 + 0.5 * (self.t * 0.9).sin();
        if let Some(bar) = world.get_mut::<ProgressBar>(self.health_bar) {
            bar.value = hp;
        }
        if let Some(tip) = world.get_mut::<Tooltip>(self.health_bar) {
            tip.text = format!("HP {:.0} / 100\nRegen +2.5/s", hp * 100.0);
        }

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Tooltip — hover a widget and wait for the popup",
                Vec2::new(28.0, 24.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.78),
            ));
            tq.push(DrawText::new(
                "Esc: quit",
                Vec2::new(28.0, WIN_H as f32 - 28.0),
                14.0,
                Color::rgb(0.7, 0.72, 0.78),
            ));
        }
    }

    fn name(&self) -> &'static str {
        "ui_tooltip_demo"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "ui_tooltip — hover popups".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.07, 0.08, 0.11, 1.0],
    });

    // 1) A button with a multi-line, bordered stats tooltip (default 0.4s delay).
    let button = app.world.spawn();
    app.world
        .add_component(button, UiNode::new(80.0, 110.0, 180.0, 48.0));
    app.world.add_component(button, Button::new("Attack"));
    app.world.add_component(
        button,
        Tooltip::new("Attack: 12\nCrit: 5%\nCooldown: 1.2s")
            .with_border(1.0, Color::rgba(1.0, 1.0, 1.0, 0.3))
            .with_corner_radius(5.0),
    );

    // 2) A health bar whose tooltip text tracks the live value (see `Demo::run`).
    let health_bar = app.world.spawn();
    app.world
        .add_component(health_bar, UiNode::new(80.0, 210.0, 300.0, 26.0));
    app.world.add_component(
        health_bar,
        ProgressBar::new(1.0)
            .with_colors(
                Color::rgb(0.35, 0.82, 0.42),
                Color::rgba(0.14, 0.15, 0.19, 1.0),
            )
            .with_corner_radius(6.0),
    );
    app.world
        .add_component(health_bar, Tooltip::new("HP").with_delay(0.2));

    // 3) A label with an instant (no delay), custom-colored tooltip.
    let label = app.world.spawn();
    app.world
        .add_component(label, UiNode::new(80.0, 290.0, 260.0, 24.0));
    app.world.add_component(
        label,
        Label::new("hover me: instant tooltip").with_color(Color::rgb(0.75, 0.85, 0.95)),
    );
    app.world.add_component(
        label,
        Tooltip::new("No delay, custom colors.")
            .with_delay(0.0)
            .with_colors(
                Color::rgb(0.1, 0.1, 0.14),
                Color::rgba(0.95, 0.85, 0.4, 0.95),
            ),
    );

    let headless = std::env::var("HEADLESS_SHOT").is_ok();
    app.add_system(UiSystem::new());
    app.add_system(Demo {
        health_bar,
        t: 0.0,
        // Park the cursor mid-button: (80,110)+(180,48)/2 → (170, 134).
        auto_hover: headless.then_some(Vec2::new(170.0, 134.0)),
    });

    if let Ok(out) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(45);
        app.save_screenshot_headless(frames, &out)
            .expect("headless screenshot");
        println!("wrote {out} ({frames} frames)");
        return;
    }
    app.run();
}
