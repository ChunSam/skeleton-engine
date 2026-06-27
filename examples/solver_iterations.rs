//! `PhysicsWorld::set_solver_iterations` — the constraint-solver iteration count.
//!
//! Each physics step runs the constraint solver a fixed number of iterations (rapier's
//! default: **4**). More iterations converge contacts and **joints** harder — a long or
//! heavy joint chain (a rope, a crane, a ragdoll) stretches and sags when the solver can't
//! fully resolve it. Before this knob the count was baked into `IntegrationParameters::default()`,
//! so a fork needing stiffer joints had to edit engine source.
//!
//! Two identical chains hang from an anchor, each a row of light links with a **heavy weight**
//! at the bottom (the classic solver stressor). They run in separate `PhysicsWorld`s:
//!
//! - **Left** — `set_solver_iterations(2)`: the solver under-converges, so the heavy weight
//!   visibly stretches the chain (the live `stretch` readout is large).
//! - **Right** — `set_solver_iterations(16)`: the chain holds nearly taut.
//!
//! `stretch` is how far the bottom weight hangs below its rigid (fully-converged) position, in
//! links. The deterministic proof is the engine test `solver_iterations_stiffen_a_joint_chain`.
use engine::{
    App, BodyHandle, DrawText, Entity, PhysicsWorld, Sprite, System, TextQueue, Transform, Vec2,
    WindowConfig, World,
};

const WIN_W: u32 = 760;
const WIN_H: u32 = 640;
/// Pixels per physics unit (render scale). A link is 1 unit tall.
const PPU: f32 = 44.0;
/// Screen y of the anchor (chains hang downward from here).
const TOP_Y: f32 = 60.0;
const N_LINKS: usize = 10;
/// Extra mass on the bottom link — heavy enough to visibly stretch a low-iteration chain.
const HEAVY_MASS: f32 = 50.0;

struct Chain {
    physics: PhysicsWorld,
    /// (body, sprite entity) per link; the last is the heavy weight.
    links: Vec<(BodyHandle, Entity)>,
    iters: usize,
    col_x: f32,
    label: &'static str,
}

/// Builds one chain in its own world and spawns a sprite per link.
fn build_chain(app: &mut App, col_x: f32, iters: usize, label: &'static str) -> Chain {
    let mut physics = PhysicsWorld::new(Vec2::new(0.0, 30.0)); // gravity down (physics units/s²)
    physics.set_solver_iterations(iters);

    let seg = 0.5; // half-size → 1-unit links
    let (anchor, _) = physics.add_static_box(Vec2::new(0.0, 0.0), 0.1, 0.1);
    let mut prev = anchor;
    let mut links: Vec<(BodyHandle, Entity)> = Vec::new();

    for i in 0..N_LINKS {
        let y = (i as f32 + 1.0) * 1.0; // links hang straight down from the anchor
        let (rb, _) = physics.add_dynamic_box(Vec2::new(0.0, y), seg, seg, false);
        // Hinge the new link to the previous body at their shared edge.
        physics.add_revolute_joint(prev, rb, Vec2::new(0.0, seg), Vec2::new(0.0, -seg));
        prev = rb;

        let heavy = i == N_LINKS - 1;
        let sprite = if heavy {
            Sprite::colored(0.95, 0.35, 0.3)
        } else {
            Sprite::colored(0.55, 0.7, 0.95)
        };
        let size = if heavy {
            Vec2::splat(PPU * 1.4)
        } else {
            Vec2::splat(PPU * 0.85)
        };
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: Vec2::new(col_x, y * PPU + TOP_Y),
                scale: size,
                ..Default::default()
            },
        );
        app.world.add_component(e, sprite);
        links.push((rb, e));
    }

    // Make the bottom link heavy — this is what the weak solver fails to hold.
    let bottom = links.last().unwrap().0;
    physics
        .rigid_body_mut(bottom)
        .unwrap()
        .set_additional_mass(HEAVY_MASS, true);

    Chain {
        physics,
        links,
        iters,
        col_x,
        label,
    }
}

/// Steps both worlds, syncs each body into its sprite, and draws the HUD.
struct ChainSystem {
    chains: Vec<Chain>,
}

impl System for ChainSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        for chain in &mut self.chains {
            chain.physics.step(dt);
            for (rb, e) in &chain.links {
                if let Some(body) = chain.physics.rigid_body(*rb) {
                    let t = body.translation();
                    let angle = body.rotation().angle();
                    if let Some(tr) = world.get_mut::<Transform>(*e) {
                        tr.position = Vec2::new(t.x * PPU + chain.col_x, t.y * PPU + TOP_Y);
                        tr.rotation = angle;
                    }
                }
            }
        }

        // Per-chain readout: how far the bottom weight hangs below its rigid position (in links).
        let infos: Vec<(f32, usize, &str, f32)> = self
            .chains
            .iter()
            .map(|c| {
                let bottom = c.links.last().unwrap().0;
                let by = c
                    .physics
                    .rigid_body(bottom)
                    .map(|b| b.translation().y)
                    .unwrap_or(0.0);
                (c.col_x, c.iters, c.label, by - N_LINKS as f32)
            })
            .collect();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "set_solver_iterations — solver convergence stiffens joints (heavy-ended chains)",
                Vec2::new(14.0, 12.0),
                16.0,
                [235, 225, 200, 255],
            ));
            for (col_x, iters, label, stretch) in infos {
                tq.push(DrawText::centered(
                    format!("{label} — {iters} iters"),
                    Vec2::new(col_x, WIN_H as f32 - 56.0),
                    16.0,
                    [200, 215, 235, 255],
                ));
                tq.push(DrawText::centered(
                    format!("stretch: {stretch:.2} links"),
                    Vec2::new(col_x, WIN_H as f32 - 32.0),
                    15.0,
                    [160, 220, 255, 255],
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "ChainSystem"
    }
}

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "PhysicsWorld::set_solver_iterations".into(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.05, 0.06, 0.09, 1.0],
    });

    let left = build_chain(&mut app, 210.0, 2, "low");
    let right = build_chain(&mut app, 550.0, 16, "high");

    app.add_system(ChainSystem {
        chains: vec![left, right],
    });

    // `HEADLESS_SHOT=path` → render to a PNG with no window (the chains settle over the frames)
    // and exit, instead of opening a window. Lets this example be pixel-verified with the
    // monitor off / from CI. `HEADLESS_FRAMES` overrides the settle frame count (default 600).
    if let Ok(path) = std::env::var("HEADLESS_SHOT") {
        let frames = std::env::var("HEADLESS_FRAMES")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(600);
        app.save_screenshot_headless(frames, &path)
            .expect("headless screenshot");
        println!("wrote {path} ({frames} frames)");
        return;
    }
    app.run();
}
