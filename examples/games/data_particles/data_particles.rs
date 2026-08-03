//! Data-driven particles demo.
//!
//! The particle emitters are defined entirely in `assets/particles.ron` — NOT in code.
//! `App::load_particle_configs` loads them into a `ParticleConfigRegistry` (hot-reloaded via
//! the asset watcher). Press `1`/`2`/`3` to switch emitters by name (fire / fountain / smoke).
//! **Edit `particles.ron`** (a `spawn_rate`, a `color_start`, a `velocity`) while it runs and the
//! effect updates live — a re-sync system swaps the `ParticleEmitter` when the loaded config changes.
//!
//! Engine convention: camera position `(0,0)` = viewport TOP-LEFT (Y down); placed at positive coords.
//!
//! **Acceptance test** — `DATA_PARTICLES_SELFTEST=1 cargo run --example data_particles_game` runs
//! [`self_test`] headlessly instead of opening a window: it edits an emitter config on disk and
//! asserts the running effect picks the change up. See that function for why a screenshot cannot.

use engine::{
    App, Camera, DrawText, Entity, InputState, KeyCode, ParticleConfigRegistry, ParticleEmitter,
    ParticleSystem, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

const CONFIGS: &str = "examples/games/data_particles/assets/particles.ron";
const WIN_W: u32 = 520;
const WIN_H: u32 = 460;

struct DemoState {
    emitter_entity: Entity,
    names: Vec<String>,
    current: usize,
    sig: u64,
}

/// Cheap signature over an emitter's config so we can detect a hot-reload.
fn emitter_sig(e: &ParticleEmitter) -> u64 {
    let c = e.color_start;
    [
        e.spawn_rate * 10.0,
        e.lifetime * 100.0,
        e.size.x,
        e.velocity_spread.x,
        (c.r + c.g + c.b + c.a) * 100.0,
    ]
    .iter()
    .fold(0u64, |acc, v| acc.wrapping_mul(131).wrapping_add(*v as u64))
}

fn fx_names(world: &World) -> Vec<String> {
    world
        .resource::<ParticleConfigRegistry>()
        .and_then(|r| r.get("fx"))
        .map(|s| s.names().map(|n| n.to_string()).collect())
        .unwrap_or_default()
}

fn fx_emitter(world: &World, name: &str) -> Option<ParticleEmitter> {
    world
        .resource::<ParticleConfigRegistry>()?
        .get("fx")?
        .emitter(name)
}

struct DataParticleSystem;

impl System for DataParticleSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let (k1, k2, k3) = world
            .resource::<InputState>()
            .map(|i| {
                (
                    i.just_pressed(KeyCode::Digit1),
                    i.just_pressed(KeyCode::Digit2),
                    i.just_pressed(KeyCode::Digit3),
                )
            })
            .unwrap_or((false, false, false));

        let (entity, current, sig) = match world.resource::<DemoState>() {
            Some(s) => (s.emitter_entity, s.current, s.sig),
            None => return,
        };
        let names = fx_names(world);
        if names.is_empty() {
            return;
        }

        // Target emitter index: a key press, else keep the current one.
        let mut idx = current.min(names.len() - 1);
        if k1 {
            idx = 0;
        } else if k2 && names.len() > 1 {
            idx = 1;
        } else if k3 && names.len() > 2 {
            idx = 2;
        }
        let switched = idx != current;

        // Detect a hot-reload of the currently-selected emitter (config values changed).
        let live = fx_emitter(world, &names[idx]);
        let new_sig = live.as_ref().map(emitter_sig).unwrap_or(0);
        let reloaded = !switched && new_sig != sig;

        // Replace the ParticleEmitter ONLY on a switch or hot-reload. Doing it every frame resets
        // the emit timer every frame, which clamps emission to one particle per tick — measured at
        // 60/s against a configured 90/s (a 63-particle population settling at 42). It under-emits
        // rather than stopping, which is why the self-test asserts the *rate* and not just that
        // something spawned.
        if switched || reloaded {
            if let Some(em) = live {
                if let Some(slot) = world.get_mut::<ParticleEmitter>(entity) {
                    *slot = em;
                }
                if let Some(s) = world.resource_mut::<DemoState>() {
                    s.current = idx;
                    s.names = names.clone();
                    s.sig = new_sig;
                }
            }
        }

        // HUD.
        let cur = names.get(idx).cloned().unwrap_or_default();
        let (rate, life) = world
            .get::<ParticleEmitter>(entity)
            .map(|e| (e.spawn_rate, e.lifetime))
            .unwrap_or((0.0, 0.0));
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Data-driven particles (emitters from RON)",
                Vec2::new(12.0, 10.0),
                18.0,
                [255, 230, 120, 255],
            ));
            tq.push(DrawText::new(
                format!("emitter: {cur}   spawn_rate: {rate}   lifetime: {life}"),
                Vec2::new(12.0, 36.0),
                15.0,
                [180, 255, 180, 255],
            ));
            tq.push(DrawText::new(
                "1: fire  2: fountain  3: smoke    edit assets/particles.ron to hot-reload",
                Vec2::new(12.0, WIN_H as f32 - 26.0),
                13.0,
                [180, 200, 230, 230],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "DataParticleSystem"
    }
}

/// Builds the whole demo onto `app` — window config, the RON emitter set, the emitter entity and
/// `DemoState` — reading its configs from `configs_path`.
///
/// `main` and [`self_test`] both go through this one function, and the self-test passes a temp copy
/// of `particles.ron` rather than the tracked file. The ordering this owns is the part that fails
/// quietly: the configs must be loaded — and their file registered with the watcher — *before* the
/// `ParticleEmitter` is built from them, so a harness that stood the world up its own way would be
/// testing an arrangement the game never runs.
fn setup(app: &mut App, configs_path: &str) {
    app.world.insert_resource(WindowConfig {
        title: "Data-Driven Particles".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.04, 0.04, 0.06, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));

    app.load_particle_configs("fx", configs_path);

    let names = fx_names(&app.world);
    let first = names.first().cloned().unwrap_or_default();
    let emitter = fx_emitter(&app.world, &first).expect("particles.ron failed to load");
    let sig = emitter_sig(&emitter);

    // Emitter entity sits low-centre so particles rise into view.
    let entity = app.world.spawn();
    app.world.add_component(
        entity,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.5, WIN_H as f32 * 0.66),
            ..Default::default()
        },
    );
    app.world.add_component(entity, emitter);

    app.world.insert_resource(DemoState {
        emitter_entity: entity,
        names,
        current: 0,
        sig,
    });
}

fn main() {
    // `DATA_PARTICLES_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("DATA_PARTICLES_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }

    let mut app = App::new();
    setup(&mut app, CONFIGS);

    // The self-test ticks these two by hand, in this order.
    app.add_system(ParticleSystem);
    app.add_system(DataParticleSystem);
    app.run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// Simulated frame time. Emission is driven entirely by `dt`, so a fixed-step loop reproduces real
/// playback exactly; the one real-time part is `notify`'s delivery latency, polled against an
/// `Instant` deadline.
#[cfg(not(target_arch = "wasm32"))]
const DT: f32 = 1.0 / 60.0;

/// `DATA_PARTICLES_SELFTEST=1 cargo run --example data_particles_game` — asserts what this example
/// exists to show and a screenshot cannot.
///
/// The headline feature is hot-reload: edit `particles.ron` while it runs and the effect changes.
/// Every link in that chain fails *silently*. A watch that was never registered, a registry that
/// reloaded nothing, or a re-sync that stopped swapping the `ParticleEmitter` all leave a perfectly
/// good effect running on the config it was born with — which is what a screenshot photographs
/// either way, and what the person editing the RON sees when their change "did nothing" and they go
/// looking at their own file instead of at the engine.
///
/// Check 1 also guards the failure this example's own comment warns about: replacing the
/// `ParticleEmitter` every frame resets its emit timer, so nothing ever spawns.
///
/// It needs no GPU, window or display: `App::new` builds neither, so the game's own [`setup`] and
/// its own systems run against a plain `World`.
///
/// Exit codes: `0` pass · `1` the emitter does not spawn at the rate the RON declares ·
/// `2` the `1`/`2`/`3` switch does not bring the other emitter's config · `3` an edit on disk never
/// reached the registry · `4` the registry reloaded but the live emitter kept its old config (the
/// re-sync is dead) · `5` the reload dropped the emitter that was selected · `6` the reloaded
/// config never reached the render layer.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{AssetServer, Particle};
    use std::path::Path;
    use std::time::{Duration, Instant};

    /// Tick the game's real system chain in `main`'s order for `frames` frames, then report the
    /// live particle population and the distinct sizes those particles were *drawn* at.
    ///
    /// The sizes come off `Transform::scale` — what the renderer reads — rather than off the
    /// emitter config, so a config that changed without reaching a particle still fails. They are
    /// returned as data: the test never computes an expected size of its own.
    fn settle(world: &mut World, frames: usize) -> (usize, Vec<f32>) {
        let mut particles = ParticleSystem;
        let mut demo = DataParticleSystem;
        for _ in 0..frames {
            particles.run(world, DT);
            demo.run(world, DT);
        }
        let live: Vec<Entity> = world.query::<Particle>().map(|(e, _)| e).collect();
        let mut sizes: Vec<f32> = Vec::new();
        for e in &live {
            if let Some(t) = world.get::<Transform>(*e) {
                if !sizes.contains(&t.scale.x) {
                    sizes.push(t.scale.x);
                }
            }
        }
        (live.len(), sizes)
    }

    /// The `ParticleEmitter` actually on the entity — the one the engine emits from.
    fn live_emitter(world: &World) -> Option<(f32, f32)> {
        let entity = world.resource::<DemoState>()?.emitter_entity;
        world
            .get::<ParticleEmitter>(entity)
            .map(|e| (e.spawn_rate, e.lifetime))
    }

    fn registry_rate(world: &World, name: &str) -> Option<f32> {
        fx_emitter(world, name).map(|e| e.spawn_rate)
    }

    fn bail(dir: &Path, code: i32, msg: String) -> i32 {
        std::fs::remove_dir_all(dir).ok();
        eprintln!("FAIL: {msg}");
        code
    }

    // Longer than the longest lifetime in the file, so the population has reached steady state and
    // — after a reload — every particle from the old config has aged out.
    const SECONDS: f32 = 2.0;
    let frames = (SECONDS / DT).round() as usize;
    // A steady-state population is `spawn_rate * lifetime`, give or take the particles spawned in
    // the sampling frame itself but not yet retired — so the slack is *one frame of spawning*, not
    // a constant. Measured against the product: fire (90/s) 63 vs 63, fountain (70/s) 79 vs 77,
    // edited fountain (200/s) 224 vs 220. The overshoot tracks `rate * DT` exactly, which is why a
    // flat tolerance failed the fastest emitter first; +2 on top absorbs rounding.
    //
    // Loose enough to pass, nowhere near loose enough to hide a failure: a dead re-sync leaves the
    // edited emitter at 79 against a wanted 220.
    fn slack(rate: f32) -> f32 {
        rate * DT + 2.0
    }

    // ── 1. The emitter spawns at the rate the RON declares ────────────────────────────────────
    //
    // The floor case, and the one this example's own code comment warns about: an emitter replaced
    // every frame has its emit timer reset every frame and never spawns at all. An empty screen is
    // also what a correct emitter looks like in the instant before the first particle.
    {
        let mut app = App::new();
        setup(&mut app, CONFIGS);
        let Some((rate, lifetime)) = live_emitter(&app.world) else {
            eprintln!("FAIL: no ParticleEmitter on the entity — particles.ron did not reach it");
            return 1;
        };
        let (live, _) = settle(&mut app.world, frames);
        let want = rate * lifetime;
        if rate <= 0.0 || (live as f32 - want).abs() > slack(rate) {
            eprintln!(
                "FAIL: the emitter does not spawn at the rate particles.ron declares — it reads \
                 {rate}/s over a {lifetime} s life, so the population should settle near \
                 {want:.0}; measured {live}"
            );
            return 1;
        }
        println!("emitter 0 ok: {live} live particles at {rate}/s x {lifetime} s");
    }

    // ── 2. Switching brings the other emitter's own config ────────────────────────────────────
    //
    // Pressing `2` selects "fountain", which the RON gives a different rate *and* lifetime from
    // "fire". If the switch moved the name but not the numbers — a plausible way for the data half
    // to be lost — check 1 would still pass. It is also the precondition for check 5: the reload
    // needs a non-default selection to preserve.
    let (fountain_rate, fountain_life) = {
        let mut app = App::new();
        setup(&mut app, CONFIGS);
        let before = live_emitter(&app.world).unwrap_or((0.0, 0.0));
        select_fountain(&mut app.world);
        let Some((rate, lifetime)) = live_emitter(&app.world) else {
            eprintln!("FAIL: the emitter entity lost its ParticleEmitter on a switch");
            return 2;
        };
        let (live, _) = settle(&mut app.world, frames);
        let want = rate * lifetime;
        if (rate, lifetime) == before || rate <= 0.0 || (live as f32 - want).abs() > slack(rate) {
            eprintln!(
                "FAIL: pressing 2 did not bring the second emitter's config — it reads {rate}/s \
                 over {lifetime} s (emitter 0 was {before:?}), so the population should settle \
                 near {want:.0}; measured {live}"
            );
            return 2;
        }
        println!("emitter 1 ok: {live} live particles at {rate}/s x {lifetime} s");
        (rate, lifetime)
    };

    // ── 3-6. An edit on disk reaches the running effect ────────────────────────────────────────
    //
    // The headline. Everything above passes just as happily with hot-reload entirely dead.
    let dir = std::env::temp_dir().join(format!("data-particles-selftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let configs_file = dir.join("particles.ron");

    // A temp copy, never the tracked `assets/particles.ron`: a self-test that edited the real file
    // would leave the repo dirty if it died mid-run. The path is absolute, so `resolve` is an
    // identity and the watch lands on this file whatever the working directory is.
    let original = std::fs::read_to_string(CONFIGS).expect("read particles.ron");
    std::fs::write(&configs_file, &original).expect("seed the temp config file");
    let configs_path = configs_file.to_string_lossy().to_string();

    // The edit rewrites the selected emitter's *size as well as its spawn rate*. Both are
    // load-bearing: the rate is what checks 3-4 read out of the registry and the component, and the
    // size is what makes check 6 falsifiable — a re-sync that took the new rate but kept the old
    // config would satisfy every other check while drawing the wrong particles.
    const OLD_RATE: &str = "spawn_rate: 70.0";
    const NEW_RATE: &str = "spawn_rate: 200.0";
    const OLD_SIZE: &str = "size: (6.0, 6.0)";
    const NEW_SIZE: &str = "size: (20.0, 20.0)";
    const NEW_RATE_VALUE: f32 = 200.0;
    let edited = original
        .replace(OLD_RATE, NEW_RATE)
        .replace(OLD_SIZE, NEW_SIZE);
    // A no-op edit would make everything below a plausible measurement of a thing that never
    // happened — the trap that once made a `beat_crawler` check pass on a burst that never fired.
    assert!(
        original.contains(OLD_RATE) && original.contains(OLD_SIZE),
        "self-test bug: particles.ron no longer contains '{OLD_RATE}' and '{OLD_SIZE}', so the \
         edit would change nothing"
    );

    let mut app = App::new();
    setup(&mut app, &configs_path);
    select_fountain(&mut app.world);
    // Clear the key press. Left set, `just_pressed(Digit2)` would re-select emitter 1 on every tick
    // below and check 5 would pass without the reload preserving anything.
    app.world.insert_resource(InputState::default());

    // The sizes this emitter draws *before* the edit, so check 6 can ask whether the new ones
    // replaced them without the test computing a single value of its own.
    let (_, sizes_before) = settle(&mut app.world, frames);

    let mut particles = ParticleSystem;
    let mut demo = DataParticleSystem;

    // `notify` has real latency and coalesces writes; let the watch settle before editing, then
    // poll against a deadline rather than exactly once.
    std::thread::sleep(Duration::from_millis(300));
    std::fs::write(&configs_file, &edited).expect("edit the temp config file");

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_event = false;
    while Instant::now() < deadline {
        let reloaded: Vec<String> = app
            .world
            .resource_mut::<AssetServer>()
            .map(|a| a.poll_reloads())
            .unwrap_or_default();
        if !reloaded.is_empty() {
            saw_event = true;
            // The `HotReloadable` forwarder in `src/app/schedule.rs`, inlined — the one step of the
            // App frame an example cannot reach (`App::update` is crate-private). Everything it
            // feeds, and everything downstream of it, is the real thing.
            if let Some(reg) = app.world.resource_mut::<ParticleConfigRegistry>() {
                for path in &reloaded {
                    reg.reload_path(path);
                }
            }
        }
        // Keep ticking the game while waiting: the re-sync lives in `DataParticleSystem`, so it
        // only ever runs here.
        particles.run(&mut app.world, DT);
        demo.run(&mut app.world, DT);
        if live_emitter(&app.world).map(|(r, _)| r) == Some(NEW_RATE_VALUE) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // ── 3. …as far as the registry ────────────────────────────────────────────────────────────
    let reg_rate = registry_rate(&app.world, "fountain");
    if reg_rate != Some(NEW_RATE_VALUE) {
        return bail(
            &dir,
            3,
            format!(
                "an edit on disk never reached the registry — 'fountain' still reads {reg_rate:?} \
                 particles/s, wanted {NEW_RATE_VALUE}; the watcher {} report the change",
                if saw_event { "did" } else { "did NOT" }
            ),
        );
    }

    // ── 4. …and as far as the emitter that is already running ─────────────────────────────────
    let live = live_emitter(&app.world);
    if live.map(|(r, _)| r) != Some(NEW_RATE_VALUE) {
        return bail(
            &dir,
            4,
            format!(
                "the registry reloaded but the running emitter did not — 'fountain' reads \
                 {NEW_RATE_VALUE}/s in the registry and {live:?} on the entity. The re-sync in \
                 DataParticleSystem is dead: the effect plays on with the config it was born with."
            ),
        );
    }

    // ── 5. …without losing which emitter was selected ─────────────────────────────────────────
    //
    // The lifetime is the fingerprint: the edit left it alone, so an emitter still carrying
    // "fountain"'s lifetime is still "fountain" and not the default the demo starts on.
    let selected = app.world.resource::<DemoState>().map(|s| s.current);
    if live.map(|(_, l)| l) != Some(fountain_life) || selected != Some(1) {
        return bail(
            &dir,
            5,
            format!(
                "the reload swapped the emitter but dropped the selection — running {live:?} at \
                 index {selected:?}, wanted index 1 with a {fountain_life} s life. An edit would \
                 yank the effect back to the first emitter mid-play."
            ),
        );
    }

    // ── 6. …and the new config is what actually gets drawn ────────────────────────────────────
    //
    // Checks 3-5 all read struct fields. This one asks the render layer: the new population, out of
    // particles the new size. A partial re-sync that took the rate and left the rest is the failure
    // it exists for.
    let (population, sizes_after) = settle(&mut app.world, frames);
    let want = NEW_RATE_VALUE * fountain_life;
    let stale: Vec<&f32> = sizes_after
        .iter()
        .filter(|s| sizes_before.contains(s))
        .collect();
    if (population as f32 - want).abs() > slack(NEW_RATE_VALUE) || !stale.is_empty() {
        return bail(
            &dir,
            6,
            format!(
                "the reloaded config never reached the render layer — the emitter carries \
                 {NEW_RATE_VALUE}/s over {fountain_life} s, so the population should settle near \
                 {want:.0}; measured {population}. Drawn sizes {sizes_after:?} against \
                 {sizes_before:?} before the edit ({} still stale)",
                stale.len()
            ),
        );
    }

    std::fs::remove_dir_all(&dir).ok();
    println!(
        "hot-reload ok: 'fountain' went {fountain_rate} -> {NEW_RATE_VALUE} particles/s on an \
         emitter already running it ({population} live, drawn at {sizes_after:?} where it was \
         {sizes_before:?}), selection preserved"
    );
    println!("PASS: data_particles");
    0
}

/// Press `2` the way a player does, then tick once so `DataParticleSystem` acts on it.
///
/// `InputState::press` is crate-private on purpose, so this goes through `InputScript` — the same
/// injection path `ENGINE_INPUT` uses.
#[cfg(not(target_arch = "wasm32"))]
fn select_fountain(world: &mut World) {
    use engine::{InputAction, InputScript};

    world.insert_resource(InputState::default());
    let mut script = InputScript::new([(0, InputAction::KeyPress(KeyCode::Digit2))]);
    script.apply(world);
    ParticleSystem.run(world, DT);
    DataParticleSystem.run(world, DT);
}
