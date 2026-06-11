//! Async asset loading + progress bar example (Phase 51).
//!
//! - LoadingScene: loads several images via `load_image_async` while drawing a
//!   progress bar with `DrawRect`.
//! - Automatically transitions to GameScene once loading completes.
use engine::{
    App, AssetServer, Color, DrawRect, DrawText, LoadProgress, Scene, SceneChange, SceneCmd,
    System, SystemRegistrar, TextQueue, UiQueue, ViewportSize, WindowConfig, World,
};

// ─── Loading Scene ───────────────────────────────────────────────────────────

struct LoadingScene;

impl Scene for LoadingScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        // Request async loading of several images (missing files fall back to magenta)
        let paths = [
            "assets/bg.png",
            "assets/player.png",
            "assets/enemy.png",
            "assets/tileset.png",
        ];
        let mut count = 0usize;
        if let Some(assets) = world.resource_mut::<AssetServer>() {
            for path in &paths {
                assets.load_image_async(*path);
                count += 1;
            }
        }
        // Initialize LoadProgress
        if let Some(prog) = world.resource_mut::<LoadProgress>() {
            prog.total = count;
            prog.loaded = 0;
        }
        systems.add(LoadingUpdateSystem { done: false });
    }

    fn on_exit(&mut self, _world: &mut World) {}
}

// ─── Loading Update System ───────────────────────────────────────────────────

struct LoadingUpdateSystem {
    done: bool,
}

impl System for LoadingUpdateSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (total, loaded) = world
            .resource::<LoadProgress>()
            .map(|p| (p.total, p.loaded))
            .unwrap_or((0, 0));

        let progress = if total == 0 {
            1.0f32
        } else {
            loaded as f32 / total as f32
        };

        // Render the progress bar — UiQueue/DrawRect uses a screen-pixel coordinate
        // system with origin (0,0) at the top-left. Positive coordinates relative to
        // ViewportSize are needed to center it. (Negative coords drew the bar off-screen.)
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((800.0, 600.0));
        let bar_w = 400.0f32;
        let bar_h = 36.0f32;
        let bar_x = (vw - bar_w) / 2.0;
        let bar_y = (vh - bar_h) / 2.0;

        if let Some(ui) = world.resource_mut::<UiQueue>() {
            // Background bar
            ui.items.push(DrawRect {
                x: bar_x - 2.0,
                y: bar_y - 2.0,
                w: bar_w + 4.0,
                h: bar_h + 4.0,
                color: Color::rgba(0.15, 0.15, 0.15, 1.0),
                z: 0.5,
            });
            // Progress bar
            ui.items.push(DrawRect {
                x: bar_x,
                y: bar_y,
                w: (bar_w * progress).max(0.0),
                h: bar_h,
                color: Color::rgba(0.3, 0.8, 0.3, 1.0),
                z: 0.6,
            });
        }

        // Percentage text
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            // `centered` anchors at the text's center — no manual -width/2 offset.
            tq.push(DrawText::centered(
                format!("Loading... {:.0}%", progress * 100.0),
                glam::Vec2::new(bar_x + bar_w / 2.0, bar_y - 28.0),
                22.0,
                [255, 255, 255, 255],
            ));
        }

        // Transition to the game scene when loading is complete
        if !self.done && loaded >= total && total > 0 {
            self.done = true;
            if let Some(sc) = world.resource_mut::<SceneChange>() {
                sc.request(SceneCmd::Replace(Box::new(GameScene)));
            }
        }
    }
}

// ─── Game Scene ──────────────────────────────────────────────────────────────

struct GameScene;

impl Scene for GameScene {
    fn on_enter(&mut self, _world: &mut World, systems: &mut SystemRegistrar) {
        systems.add(GameUpdateSystem);
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

struct GameUpdateSystem;

impl System for GameUpdateSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((800.0, 600.0));
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::centered(
                "Loading complete! Game ready.",
                glam::Vec2::new(vw / 2.0, vh / 2.0),
                22.0,
                [100, 255, 100, 255],
            ));
        }
    }
}

// ─── Entry Point ─────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "Phase 51 — Async Loading Bar".to_string(),
        width: 800,
        height: 600,
        clear_color: [0.05, 0.05, 0.10, 1.0],
    });

    app.set_scene(Box::new(LoadingScene));
    app.run();
}
