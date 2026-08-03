//! Data-driven animation demo.
//!
//! The sprite's animation clips are defined entirely in `assets/clips.ron` — NOT in code.
//! `App::load_animation_clips` loads them into an `AnimationClipRegistry` (hot-reloaded via
//! the asset watcher). Press `1`/`2`/`3` to switch clips by name. **Edit `clips.ron`** (change
//! an `fps` or a `frames` list) while it runs and the animation updates live — a re-sync system
//! rebuilds the `AnimationPlayer` from the registry whenever the loaded clips change.
//!
//! Engine convention: camera position `(0,0)` = viewport TOP-LEFT (Y down); placed at positive coords.
//!
//! **Acceptance test** — `DATA_ANIM_SELFTEST=1 cargo run --example data_anim_game` runs
//! [`self_test`] headlessly instead of opening a window: it edits a clip file on disk and asserts
//! the running sprite picks the change up. See that function for why a screenshot cannot.

use engine::{
    AnimationClip, AnimationClipRegistry, AnimationPlayer, AnimationSystem, App, DrawText, Entity,
    InputState, KeyCode, Sprite, System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

const ANIM: &str = "examples/games/data_anim/assets/anim.png";
const CLIPS: &str = "examples/games/data_anim/assets/clips.ron";
const WIN_W: u32 = 480;
const WIN_H: u32 = 360;

struct DemoState {
    sprite: Entity,
    names: Vec<String>, // clip names (sorted, aligned with clip indices)
    current: usize,
    /// Cheap signature of the loaded clips; when it changes (hot-reload), re-sync the player.
    sig: u64,
}

/// Signature over the registry's clips so we can detect a hot-reload.
fn clips_sig(clips: &[AnimationClip]) -> u64 {
    clips.iter().fold(0u64, |acc, c| {
        acc.wrapping_mul(31)
            .wrapping_add(c.frames.len() as u64)
            .wrapping_mul(31)
            .wrapping_add((c.fps * 100.0) as u64)
            .wrapping_mul(2)
            .wrapping_add(c.looping as u64)
    })
}

fn hero_clips(world: &World) -> Option<(Vec<AnimationClip>, Vec<String>)> {
    let reg = world.resource::<AnimationClipRegistry>()?;
    let set = reg.get("hero")?;
    Some((
        set.clips().to_vec(),
        set.names().map(|s| s.to_string()).collect(),
    ))
}

struct DataAnimSystem;

impl System for DataAnimSystem {
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

        let (sprite, names, current, sig) = match world.resource::<DemoState>() {
            Some(s) => (s.sprite, s.names.clone(), s.current, s.sig),
            None => return,
        };

        // Pull the registry's current clips (owned) before touching components.
        let Some((clips, new_names)) = hero_clips(world) else {
            return;
        };
        let new_sig = clips_sig(&clips);
        let reloaded = new_sig != sig;

        // Pick the target clip index: a key press, else keep current (clamped to the new set).
        let mut idx = current.min(names.len().saturating_sub(1));
        if k1 {
            idx = 0;
        } else if k2 && new_names.len() > 1 {
            idx = 1;
        } else if k3 && new_names.len() > 2 {
            idx = 2;
        }
        idx = idx.min(new_names.len().saturating_sub(1));
        let changed_clip = idx != current;

        // On a hot-reload, rebuild the player from the new clips (preserve the selected clip).
        // On a key press, just switch the clip on the existing player.
        if reloaded {
            if let Some(p) = world.get_mut::<AnimationPlayer>(sprite) {
                *p = AnimationPlayer::new(clips.clone());
                p.play(idx);
            }
        } else if changed_clip {
            if let Some(p) = world.get_mut::<AnimationPlayer>(sprite) {
                p.play(idx);
            }
        }

        if reloaded || changed_clip {
            if let Some(s) = world.resource_mut::<DemoState>() {
                s.current = idx;
                s.names = new_names.clone();
                s.sig = new_sig;
            }
        }

        // HUD.
        let cur_name = new_names.get(idx).cloned().unwrap_or_default();
        let cur_fps = clips.get(idx).map(|c| c.fps).unwrap_or(0.0);
        let cur_frames = clips.get(idx).map(|c| c.frames.len()).unwrap_or(0);
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                "Data-driven animation (clips from RON)",
                Vec2::new(12.0, 10.0),
                18.0,
                [255, 230, 120, 255],
            ));
            tq.push(DrawText::new(
                format!("clip: {cur_name}   fps: {cur_fps}   frames: {cur_frames}"),
                Vec2::new(12.0, 36.0),
                15.0,
                [180, 255, 180, 255],
            ));
            tq.push(DrawText::new(
                "1/2/3: switch clip    edit assets/clips.ron to hot-reload",
                Vec2::new(12.0, WIN_H as f32 - 26.0),
                14.0,
                [180, 200, 230, 230],
            ));
        }
    }

    fn name(&self) -> &'static str {
        "DataAnimSystem"
    }
}

/// Builds the whole demo onto `app` — window config, atlas, the RON clip set, the animated sprite
/// and `DemoState` — reading its clips from `clips_path`.
///
/// `main` and [`self_test`] both go through this one function, and the self-test passes a temp copy
/// of `clips.ron` rather than the tracked file. The ordering this owns is the part that fails
/// quietly: the clips must be loaded — and their file registered with the watcher — *before* the
/// `AnimationPlayer` is built from them, so a harness that stood the world up its own way would be
/// testing an arrangement the game never runs.
fn setup(app: &mut App, clips_path: &str) {
    app.world.insert_resource(WindowConfig {
        title: "Data-Driven Animation".to_string(),
        width: WIN_W,
        height: WIN_H,
        clear_color: [0.06, 0.07, 0.10, 1.0],
    });
    app.world
        .insert_resource(engine::Camera::new(Vec2::ZERO, 1.0));

    app.load_atlas(ANIM, 4, 4);
    app.load_animation_clips("hero", clips_path);

    // Build the player from the RON-loaded clips (idle = index 0; clips sort alphabetically).
    let (clips, names) = hero_clips(&app.world).expect("clips.ron failed to load");
    let sig = clips_sig(&clips);
    let mut player = AnimationPlayer::new(clips);
    player.play(0);

    let sprite = app.world.spawn();
    app.world.add_component(
        sprite,
        Transform {
            position: Vec2::new(WIN_W as f32 * 0.5, WIN_H as f32 * 0.5 + 16.0),
            scale: Vec2::splat(112.0),
            z: 0.0,
            ..Default::default()
        },
    );
    app.world.add_component(sprite, Sprite::textured(ANIM));
    app.world.add_component(sprite, player);

    app.world.insert_resource(DemoState {
        sprite,
        names,
        current: 0,
        sig,
    });
}

fn main() {
    // `DATA_ANIM_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("DATA_ANIM_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }

    let mut app = App::new();
    setup(&mut app, CLIPS);

    // The self-test ticks these two by hand, in this order.
    app.add_system(AnimationSystem::new());
    app.add_system(DataAnimSystem);
    app.run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// Simulated frame time. The animation is driven entirely by `dt`, so a fixed-step loop reproduces
/// real playback exactly — unlike the audio examples, nothing here is a wall-clock signal. The one
/// real-time part is `notify`'s delivery latency, which is polled against an `Instant` deadline.
#[cfg(not(target_arch = "wasm32"))]
const DT: f32 = 1.0 / 60.0;

/// `DATA_ANIM_SELFTEST=1 cargo run --example data_anim_game` — asserts what this example exists to
/// show and a screenshot cannot.
///
/// The headline feature is hot-reload: edit `clips.ron` while it runs and the animation changes.
/// Every link in that chain fails *silently*. A watch that was never registered, a registry that
/// reloaded nothing, or a re-sync system that stopped rebuilding the `AnimationPlayer` all leave a
/// sprite happily animating the clips it was born with — which is exactly what a screenshot
/// photographs either way, and what the person editing the RON sees when their change "did
/// nothing" and they go looking at their own file instead of at the engine. `beat_crawler` shipped
/// a dead headline feature for several releases on precisely this shape.
///
/// It needs no GPU, window or display: `App::new` builds neither, so the game's own [`setup`] and
/// its own systems run against a plain `World`.
///
/// Exit codes: `0` pass · `1` the animation does not advance at the fps the RON declares ·
/// `2` the `1`/`2`/`3` clip switch does not carry that clip's own fps to the render layer ·
/// `3` an edit on disk never reached the registry · `4` the registry reloaded but the running
/// player kept its old clips (the re-sync is dead) · `5` the rebuilt player lost the clip that was
/// selected · `6` the reloaded fps never reached the render layer.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{AssetServer, UvRect};
    use std::path::Path;
    use std::time::{Duration, Instant};

    /// Tick the game's real system chain in `main`'s order for `frames` frames, returning how many
    /// times the drawn `UvRect` changed and which distinct rects were drawn.
    ///
    /// Asserting on `UvRect` rather than on `AnimationPlayer::current_frame` is deliberate: it is
    /// the component the renderer actually reads, so a player advancing into a UV nobody writes
    /// still fails here. The rects come back as data, not as expected values computed from the
    /// atlas grid — the test never re-derives what the engine is being asked to produce.
    fn uv_trace(world: &mut World, frames: usize) -> (usize, Vec<UvRect>) {
        let mut anim = AnimationSystem::new();
        let mut demo = DataAnimSystem;
        let sprite = match world.resource::<DemoState>() {
            Some(s) => s.sprite,
            None => return (0, Vec::new()),
        };
        // Warm-up: `UvRect` is written by the first tick, so counting from "absent" would score a
        // free change that a frozen animation would also earn.
        anim.run(world, DT);
        demo.run(world, DT);
        let mut last = world.get::<UvRect>(sprite).copied();
        let mut seen: Vec<UvRect> = last.into_iter().collect();
        let mut changes = 0;
        for _ in 0..frames {
            anim.run(world, DT);
            demo.run(world, DT);
            let now = world.get::<UvRect>(sprite).copied();
            if now != last {
                changes += 1;
                last = now;
                if let Some(uv) = now {
                    if !seen.contains(&uv) {
                        seen.push(uv);
                    }
                }
            }
        }
        (changes, seen)
    }

    fn uv_changes(world: &mut World, frames: usize) -> usize {
        uv_trace(world, frames).0
    }

    fn player_fps(world: &World, clip: usize) -> Option<f32> {
        let sprite = world.resource::<DemoState>()?.sprite;
        world
            .get::<AnimationPlayer>(sprite)
            .and_then(|p| p.clips.get(clip))
            .map(|c| c.fps)
    }

    fn player_current_clip(world: &World) -> Option<usize> {
        let sprite = world.resource::<DemoState>()?.sprite;
        world.get::<AnimationPlayer>(sprite).map(|p| p.current_clip)
    }

    fn registry_fps(world: &World, clip: usize) -> Option<f32> {
        world
            .resource::<AnimationClipRegistry>()?
            .get("hero")?
            .clips()
            .get(clip)
            .map(|c| c.fps)
    }

    fn bail(dir: &Path, code: i32, msg: String) -> i32 {
        std::fs::remove_dir_all(dir).ok();
        eprintln!("FAIL: {msg}");
        code
    }

    // Two seconds is long enough that a clip's period is small change against it, so the count is
    // dominated by the fps rather than by where the tick boundaries happen to land.
    const SECONDS: f32 = 2.0;
    let frames = (SECONDS / DT).round() as usize;

    // ── 1. The animation advances at the fps the RON declares ─────────────────────────────────
    //
    // The floor case: clips that loaded as an empty/garbage set, an `AnimationSystem` that stopped
    // writing `UvRect`, or an fps of 0 all render as a sprite that simply sits there — and a still
    // sprite is what a single screenshot of a *working* animation looks like too.
    {
        let mut app = App::new();
        setup(&mut app, CLIPS);
        let idle_fps = match player_fps(&app.world, 0) {
            Some(f) => f,
            None => {
                eprintln!("FAIL: no AnimationPlayer on the sprite — clips.ron did not reach it");
                return 1;
            }
        };
        let changes = uv_changes(&mut app.world, frames);
        let want = idle_fps * SECONDS;
        if idle_fps <= 0.0 || (changes as f32 - want).abs() > 1.0 {
            eprintln!(
                "FAIL: the animation does not advance at the fps clips.ron declares — clip 0 says \
                 {idle_fps} fps, so {SECONDS} s should draw {want:.0} frame changes; measured \
                 {changes}"
            );
            return 1;
        }
        println!("clip 0 ok: {changes} UvRect changes in {SECONDS} s at {idle_fps} fps");
    }

    // ── 2. Each clip's own fps reaches the render layer ────────────────────────────────────────
    //
    // Pressing `2` selects "run", which the RON gives a different fps from "idle". If the player
    // were animating at one rate for every clip — a plausible way for the data half to be lost —
    // check 1 would still pass. This is also the precondition for check 5: the reload has to have a
    // non-default selection to preserve.
    let run_fps = {
        let mut app = App::new();
        setup(&mut app, CLIPS);
        select_run_clip(&mut app.world);
        let idle_fps = player_fps(&app.world, 0).unwrap_or(0.0);
        let run_fps = player_fps(&app.world, 1).unwrap_or(0.0);
        let changes = uv_changes(&mut app.world, frames);
        let want = run_fps * SECONDS;
        if player_current_clip(&app.world) != Some(1)
            || run_fps <= 0.0
            || (run_fps - idle_fps).abs() < f32::EPSILON
            || (changes as f32 - want).abs() > 1.0
        {
            eprintln!(
                "FAIL: pressing 2 did not carry clip 1's own fps to the render layer — selected \
                 clip {:?} (want 1), clip 1 says {run_fps} fps against clip 0's {idle_fps}, so \
                 {SECONDS} s should draw {want:.0} frame changes; measured {changes}",
                player_current_clip(&app.world)
            );
            return 2;
        }
        println!("clip 1 ok: {changes} UvRect changes in {SECONDS} s at {run_fps} fps");
        run_fps
    };

    // ── 3-6. An edit on disk reaches the running player ────────────────────────────────────────
    //
    // The headline. Everything above passes just as happily with hot-reload entirely dead.
    let dir = std::env::temp_dir().join(format!("data-anim-selftest-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let clips_file = dir.join("clips.ron");

    // A temp copy, never the tracked `assets/clips.ron`: a self-test that edited the real file
    // would leave the repo dirty if it died mid-run. The path is absolute, so `resolve` is an
    // identity and the watch lands on this file whatever the working directory is.
    let original = std::fs::read_to_string(CLIPS).expect("read clips.ron");
    std::fs::write(&clips_file, &original).expect("seed the temp clip file");
    let clips_path = clips_file.to_string_lossy().to_string();

    // The edit rewrites the "run" clip's *frames as well as its fps*. Both halves are load-bearing:
    // the fps is what checks 3-4 read out of the registry and the player, and the frame swap is what
    // makes check 6 falsifiable — a re-sync that copied the new rate but kept the old frame list
    // would satisfy every other check while drawing the wrong tiles.
    const OLD_RUN: &str = "frames: [4, 5, 6, 7], fps: 12.0";
    const NEW_RUN: &str = "frames: [8, 9, 10, 11], fps: 30.0";
    const NEW_FPS: f32 = 30.0;
    let edited = original.replace(OLD_RUN, NEW_RUN);
    // A no-op edit would make everything below a plausible measurement of a thing that never
    // happened — the trap that once made a `beat_crawler` check pass on a burst that never fired.
    assert_ne!(
        edited, original,
        "self-test bug: clips.ron no longer contains '{OLD_RUN}', so the edit changed nothing"
    );

    let mut app = App::new();
    setup(&mut app, &clips_path);
    select_run_clip(&mut app.world);
    // Clear the key press. Left set, `just_pressed(Digit2)` would re-select clip 1 on every tick
    // below and check 5 would pass without the reload preserving anything.
    app.world.insert_resource(InputState::default());

    // The tiles this clip draws *before* the edit, so check 6 can ask whether the new ones replaced
    // them without the test computing a single UV of its own.
    let (_, uvs_before) = uv_trace(&mut app.world, frames);

    let mut anim = AnimationSystem::new();
    let mut demo = DataAnimSystem;

    // `notify` has real latency and coalesces writes; let the watch settle before editing, then
    // poll against a deadline rather than exactly once.
    std::thread::sleep(Duration::from_millis(300));
    std::fs::write(&clips_file, &edited).expect("edit the temp clip file");

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
            if let Some(reg) = app.world.resource_mut::<AnimationClipRegistry>() {
                for path in &reloaded {
                    reg.reload_path(path);
                }
            }
        }
        // Keep ticking the game while waiting: the re-sync lives in `DataAnimSystem`, so it only
        // ever runs here.
        anim.run(&mut app.world, DT);
        demo.run(&mut app.world, DT);
        if player_fps(&app.world, 1) == Some(NEW_FPS) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // ── 3. …as far as the registry ────────────────────────────────────────────────────────────
    let reg_fps = registry_fps(&app.world, 1);
    if reg_fps != Some(NEW_FPS) {
        return bail(
            &dir,
            3,
            format!(
                "an edit on disk never reached the registry — clip 1 still reads {reg_fps:?} fps, \
                 wanted {NEW_FPS}; the watcher {} report the change",
                if saw_event { "did" } else { "did NOT" }
            ),
        );
    }

    // ── 4. …and as far as the sprite that is already playing ──────────────────────────────────
    let live_fps = player_fps(&app.world, 1);
    if live_fps != Some(NEW_FPS) {
        return bail(
            &dir,
            4,
            format!(
                "the registry reloaded but the running player did not — clip 1 reads {NEW_FPS} fps \
                 in the registry and {live_fps:?} on the sprite. The re-sync in DataAnimSystem is \
                 dead: the game plays on, animating the clips it was born with."
            ),
        );
    }

    // ── 5. …without losing what the player had selected ───────────────────────────────────────
    let selected = player_current_clip(&app.world);
    if selected != Some(1) {
        return bail(
            &dir,
            5,
            format!(
                "the reload rebuilt the player but dropped the selected clip — playing {selected:?}, \
                 wanted 1. An edit would yank the animation back to idle mid-play."
            ),
        );
    }

    // ── 6. …and the new clip is what actually gets drawn ──────────────────────────────────────
    //
    // Checks 3-5 all read struct fields. This one asks the render layer: at the new rate, and out
    // of the new frames. A partial re-sync that took the fps and left the frame list is the failure
    // it exists for — and the only one of these that a still screenshot could not even be *staged*
    // to show, since the difference is which tiles cycle over time.
    let (changes, uvs_after) = uv_trace(&mut app.world, frames);
    let want = NEW_FPS * SECONDS;
    let stale: Vec<&UvRect> = uvs_after
        .iter()
        .filter(|u| uvs_before.contains(u))
        .collect();
    if (changes as f32 - want).abs() > 1.0 || !stale.is_empty() {
        return bail(
            &dir,
            6,
            format!(
                "the reloaded clip never reached the render layer — the player carries {NEW_FPS} \
                 fps, so {SECONDS} s should draw {want:.0} frame changes; measured {changes}. \
                 {} of the {} rects drawn were still the pre-edit tiles",
                stale.len(),
                uvs_after.len()
            ),
        );
    }

    std::fs::remove_dir_all(&dir).ok();
    println!(
        "hot-reload ok: clip 1 went {run_fps} -> {NEW_FPS} fps on a sprite already playing it \
         ({changes} UvRect changes in {SECONDS} s, {} new tiles, none stale), selection preserved",
        uvs_after.len()
    );
    println!("PASS: data_anim");
    0
}

/// Press `2` the way a player does, then tick once so `DataAnimSystem` acts on it.
///
/// `InputState::press` is crate-private on purpose, so this goes through [`InputScript`] — the same
/// injection path `ENGINE_INPUT` uses.
#[cfg(not(target_arch = "wasm32"))]
fn select_run_clip(world: &mut World) {
    use engine::{InputAction, InputScript};

    world.insert_resource(InputState::default());
    let mut script = InputScript::new([(0, InputAction::KeyPress(KeyCode::Digit2))]);
    script.apply(world);
    AnimationSystem::new().run(world, DT);
    DataAnimSystem.run(world, DT);
}
