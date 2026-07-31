//! Beat Crawler — a dungeon crawler whose turn clock is the music itself.
//!
//! The second capstone: where `roguelike` composed **two** features (procgen + fog-of-war), this
//! composes the engine's newest surface into one playable loop, and does it *structurally* rather
//! than decoratively — remove any one piece and the game stops working:
//!
//! | Engine feature | What it actually does here |
//! |---|---|
//! | [`Audio::bands`] | **the turn clock** — a beat is a low-frequency transient the game *hears* |
//! | [`generate_bsp_dungeon`] / [`generate_cellular_cave`] | the level, alternating by depth |
//! | [`Rng`] | one seed per depth, so a run is reproducible |
//! | [`FovMap`] | fog-of-war; walls that block movement block sight |
//! | [`find_path`] | enemies path to you — but only on the beat |
//! | [`HitFlash`] / [`FloatingText`] / `Camera::shake` | the hit feedback |
//! | [`ProgressBar`] | the health gauge |
//!
//! # Why this needs `bands()` and not `levels()`
//!
//! The soundtrack is a real mix — kick, bass, hats and a lead, all sounding at once — looping from
//! [`TRACK`]. An amplitude meter ([`Audio::levels`]) cannot find the turns in it: everything is
//! loud at once, so `rms` barely moves between a kick and a bar of bass. The game instead sums the
//! bottom two **spectrum bands**, where the kick's 150→48 Hz sweep lives. The rhythm is
//! *discriminated by frequency*, which is exactly the thing `levels()` cannot do.
//!
//! This is a genuinely harder problem than a test tone, and it was made harder on purpose. The
//! first mix put the bass on C2/F2/G2 — right on top of the kick — and measured: every low band
//! sat pinned at full scale, the kick's transient vanished into it, and **no threshold worked at
//! all**. Moving the bass up an octave is the same thing a real mix does, and it leaves the two
//! *separable* without separating them: the bass still overlaps the detector's window through its
//! harmonics. See `assets/soundtrack.py`, which generates the track and documents the numbers.
//!
//! Gameplay still learns the groove by listening: no gameplay code reads [`PATTERN`], which
//! survives only so the [`BEAT_WATCHDOG`] has a schedule to fall back to. The audible groove lives
//! in the `.wav`; `assets/soundtrack.py` renders one from the other, so changing the beat means
//! changing both.
//!
//! # The loop
//!
//! Descend → move on the beat → bump enemies to fight → find the stair → next depth.
//!
//! - **WASD / arrows** — queue a step; it resolves on the next beat
//! - Move into an enemy to attack it. Press **in time with the beat** (within
//!   [`ON_BEAT_WINDOW`]) for **double damage** — the readout says `ON BEAT`.
//! - Enemies only act on the beat too, so the whole dungeon moves to the music.
//! - Reach the **green stair** to descend. Depth 1/3/5… are BSP dungeons, 2/4/6… are caves.
//! - **Esc** — quit
//!
//! # Running it headless
//!
//! - `ENGINE_CAPTURE=90:/tmp/beat.png cargo run --example beat_crawler_game` — engine-level frame
//!   capture (v0.134.0); needs no code in this file and no window.
//! - `BEAT_CRAWLER_SELFTEST=1 cargo run --example beat_crawler_game` — asserts the level is
//!   solvable, enemies path toward the player, and (when an audio device exists) that the detector
//!   finds the kicks **in the real mix**, at the spacing the mix actually has. Exits `0` pass ·
//!   `1` stair unreachable · `2` bad enemy placement · `3` pathing did not approach · `4` no kick
//!   ever detected · `5` detections did not land on the beat grid (wrong count or wrong spacing).
//!   **No audio device is a SKIP, not a failure** — the rhythm cannot be exercised there.
use engine::{
    find_path, generate_bsp_dungeon, generate_cellular_cave, spawn_floating_text, App, Audio,
    AudioFacadeSystem, CaveParams, Color, DrawText, DungeonMap, DungeonParams, Entity,
    FloatingText, FloatingTextSystem, FovMap, HitFlash, HitFlashSystem, InputState, KeyCode,
    PathGrid, ProgressBar, Rng, Scene, ShouldQuit, Sprite, System, SystemRegistrar, TextQueue,
    Transform, UiNode, UiSystem, WindowConfig, World,
};
use glam::{IVec2, Vec2};

// ── Layout ──────────────────────────────────────────────────────────────────────────────────
const COLS: i32 = 32;
const ROWS: i32 = 20;
const CELL: f32 = 26.0;
const GRID_OX: f32 = 16.0;
const GRID_OY: f32 = 16.0;
const HUD_H: f32 = 108.0;
const TORCH_RADIUS: i32 = 7;
/// Top of the HUD band, derived from the grid rather than written out again — the first headless
/// capture of this example had the health bar sitting on top of the depth readout because the two
/// were independent magic numbers. Every HUD row below is an offset from this one value.
const HUD_TOP: f32 = GRID_OY + ROWS as f32 * CELL + 12.0;
const HUD_ROW_BAR: f32 = HUD_TOP + 30.0;
const HUD_ROW_KEYS: f32 = HUD_TOP + 56.0;
const HUD_ROW_MSG: f32 = HUD_TOP + 78.0;
const HP_BAR_W: f32 = 150.0;
const HP_BAR_H: f32 = 14.0;

// ── Rhythm ──────────────────────────────────────────────────────────────────────────────────
/// The soundtrack: one seamless 2.56 s bar, looped by `play_music`. Synthesized from scratch by
/// `assets/soundtrack.py` — sine arithmetic and a seeded PRNG, no sample pack — so it is CC0 and
/// safe to ship in an MIT repository, the same reasoning as `src/audio/fixtures/README.md`.
const TRACK: &[u8] = include_bytes!("assets/soundtrack.wav");
/// Seconds per pattern step. 16 steps = 2.56 s per bar, matching `STEP_SECS` in the generator.
const STEP_SECS: f32 = 0.16;
/// Seconds between kicks — every 4th step. The grid the detector is checked against.
const BEAT_SECS: f32 = STEP_SECS * 4.0;
/// The groove, as 16 steps. `K` = kick (a turn), `b` = blip (musical only, never a turn),
/// `.` = rest. **No gameplay code reads this**, and nothing plays it: the audible groove is in
/// [`TRACK`]. It survives as the schedule [`BEAT_WATCHDOG`] falls back to when nothing can be
/// heard, and as the written description of what the `.wav` contains.
const PATTERN: [Step; 16] = {
    use Step::{Blip, Kick, Rest};
    [
        Kick, Rest, Blip, Rest, // 0-3
        Kick, Rest, Blip, Blip, // 4-7
        Kick, Rest, Blip, Rest, // 8-11
        Kick, Blip, Rest, Blip, // 12-15
    ]
};
/// The meter the turn clock reads. `play_music` publishes under this name on both backends.
const BEAT_METER: &str = Audio::MUSIC_CHANNEL;
/// How many spectrum bands to ask for. `bands()` resamples its internal 32 to whatever length the
/// caller passes — this is our choice, not an engine limit.
const BANDS: usize = 16;
/// Bands `0..LOW_BANDS` are summed into the kick detector.
///
/// **Two, not four.** Measured per layer against the real track: the kick owns bands 0-1 (they
/// share FFT bins at this resolution and move together), while the bass saturates bands 2-6 at
/// full scale for most of the bar. Summing a saturated band adds a constant, not information —
/// including bands 2-3 was what made every threshold fire on bass wobble instead of on kicks.
const LOW_BANDS: usize = 2;
/// Summed low-band energy that counts as a kick.
///
/// Measured, not guessed: over 7 bars of the real track the correct count (28) and the correct
/// spacing (0.640 s, sd 0.03) hold anywhere in **1.45–1.95**, so this sits mid-plateau rather
/// than on an edge. Below ~1.40 the bass starts tripping it; 2.00 is the saturation ceiling.
const KICK_THRESHOLD: f32 = 1.6;
/// A kick cannot fire another turn within this many seconds of the last one.
///
/// **This replaced an arm/re-arm latch, and the replacement is the point.** The latch — re-arm
/// once energy falls back below a floor — is correct for discrete sounds separated by silence,
/// which is what this example used to play. Under a real mix the low band never falls back: the
/// bass holds it up, so the latch either never re-arms or re-arms on bass ripple. Measured over
/// the same 7 bars, arm/re-arm produced 31 fires with gaps of sd 0.16 s; the cooldown produced
/// exactly 28 at sd 0.03 s. This is the second independent confirmation of that finding —
/// `examples/games/survivor` hit it first, on a completely different signal (see module-map
/// row 79).
const KICK_COOLDOWN: f32 = 0.40;
/// Press within this many seconds *before* a beat to land the `ON BEAT` bonus.
const ON_BEAT_WINDOW: f32 = 0.18;
/// If this long passes with kicks scheduled but none *heard*, stop trusting the ears and run off
/// the schedule instead.
///
/// This is not a test affordance — it is the fix for a real way the game breaks. The turn clock is
/// wall-clock audio, so anything that silences the output (the player mutes the OS, the device is
/// busy, the `AudioContext` never unlocks on the web) would otherwise leave the dungeon frozen
/// forever with no indication why. It also makes the game drivable headlessly, where frames run far
/// faster than sound.
const BEAT_WATCHDOG: f32 = 1.2;

// ── Rules ───────────────────────────────────────────────────────────────────────────────────
const PLAYER_MAX_HP: i32 = 6;
const ENEMY_HP: i32 = 2;
const ENEMY_DAMAGE: i32 = 1;
const HIT_DAMAGE: i32 = 1;
const ON_BEAT_DAMAGE: i32 = 2;
/// Enemies are placed at least this far (in BFS steps) from the spawn, so a level never opens with
/// a monster in your face.
const MIN_ENEMY_DIST: i32 = 6;
const ENEMIES_PER_DEPTH: [usize; 4] = [2, 3, 4, 5];

// ── Palette ─────────────────────────────────────────────────────────────────────────────────
const UNSEEN: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);
const FLOOR_LIT: Color = Color::rgba(0.30, 0.31, 0.38, 1.0);
const FLOOR_DIM: Color = Color::rgba(0.10, 0.10, 0.14, 1.0);
const WALL_LIT: Color = Color::rgba(0.54, 0.42, 0.30, 1.0);
const WALL_DIM: Color = Color::rgba(0.18, 0.14, 0.11, 1.0);
const PLAYER_COLOR: Color = Color::rgba(1.0, 0.85, 0.22, 1.0);
const ENEMY_COLOR: Color = Color::rgba(0.93, 0.30, 0.34, 1.0);
const STAIR_COLOR: Color = Color::rgba(0.35, 0.92, 0.52, 1.0);
const TEXT_MAIN: Color = Color::rgb(0.92, 0.90, 0.76);
const TEXT_DIM: Color = Color::rgb(0.66, 0.68, 0.75);
const DMG_COLOR: Color = Color::rgba(1.0, 0.86, 0.35, 1.0);
const CRIT_COLOR: Color = Color::rgba(0.45, 1.0, 0.72, 1.0);
const HURT_COLOR: Color = Color::rgba(1.0, 0.42, 0.42, 1.0);

/// One step of the soundtrack. Only [`Step::Kick`] is audible to the low-band detector.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Kick,
    Blip,
    Rest,
}

/// World-space center of grid cell `(x, y)`. The camera sits at the world origin and its
/// `position` is the viewport's **top-left**, so world coordinates read like screen coordinates:
/// `+X` right, `+Y` down — the same orientation as the grid.
fn cell_center(x: i32, y: i32) -> Vec2 {
    Vec2::new(
        GRID_OX + x as f32 * CELL + CELL * 0.5,
        GRID_OY + y as f32 * CELL + CELL * 0.5,
    )
}

fn in_bounds(c: IVec2) -> bool {
    c.x >= 0 && c.y >= 0 && c.x < COLS && c.y < ROWS
}

/// Breadth-first step distance from `start` over the walkable cells of `grid`.
/// `-1` marks unreachable, which is what makes "is this level solvable?" a one-line check.
fn bfs_distances(grid: &PathGrid, start: IVec2) -> Vec<i32> {
    let mut dist = vec![-1; (COLS * ROWS) as usize];
    if !in_bounds(start) || !grid.is_walkable(start.x, start.y) {
        return dist;
    }
    let idx = |c: IVec2| (c.y * COLS + c.x) as usize;
    dist[idx(start)] = 0;
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some(c) = queue.pop_front() {
        let d = dist[idx(c)];
        for step in [
            IVec2::new(1, 0),
            IVec2::new(-1, 0),
            IVec2::new(0, 1),
            IVec2::new(0, -1),
        ] {
            let n = c + step;
            if in_bounds(n) && grid.is_walkable(n.x, n.y) && dist[idx(n)] < 0 {
                dist[idx(n)] = d + 1;
                queue.push_back(n);
            }
        }
    }
    dist
}

// ── Level ───────────────────────────────────────────────────────────────────────────────────

/// A generated depth plus everything derived from it. Both generators produce a [`DungeonMap`],
/// so the only thing that changes between a BSP dungeon and a cave is which function is called.
struct Level {
    map: DungeonMap,
    grid: PathGrid,
    fov: FovMap,
    spawn: IVec2,
    stair: IVec2,
    enemy_cells: Vec<IVec2>,
}

/// Builds a depth. Placement is deliberately **generator-agnostic**: it uses BFS distance from the
/// spawn rather than the room list, because `generate_cellular_cave` records only a single 1×1
/// room and a room-based placement would silently degenerate on even depths.
fn make_level(depth: u32, seed: u64) -> Level {
    let map = if depth % 2 == 1 {
        generate_bsp_dungeon(COLS, ROWS, seed, &DungeonParams::default())
    } else {
        generate_cellular_cave(COLS, ROWS, seed, &CaveParams::default())
    };
    let grid = map.to_path_grid();
    // The one-line bridge, same as `roguelike`: the walls that block movement block sight.
    let fov = FovMap::from_path_grid(&grid);

    let spawn = map.first_room_center().unwrap_or(IVec2::new(1, 1));
    let dist = bfs_distances(&grid, spawn);
    let idx = |c: IVec2| (c.y * COLS + c.x) as usize;

    // The stair goes at the farthest reachable cell — guaranteed to exist and to be reachable,
    // which is what makes every level solvable regardless of which generator produced it.
    let mut stair = spawn;
    let mut best = -1;
    for y in 0..ROWS {
        for x in 0..COLS {
            let c = IVec2::new(x, y);
            if dist[idx(c)] > best {
                best = dist[idx(c)];
                stair = c;
            }
        }
    }

    // Enemies on reachable cells a minimum distance away, drawn from the same seeded stream so a
    // depth is reproducible in full.
    let mut rng = Rng::new(seed ^ 0x5eed_c0de);
    let mut candidates: Vec<IVec2> = (0..ROWS)
        .flat_map(|y| (0..COLS).map(move |x| IVec2::new(x, y)))
        .filter(|&c| dist[idx(c)] >= MIN_ENEMY_DIST && c != stair)
        .collect();
    rng.shuffle(&mut candidates);
    let want =
        ENEMIES_PER_DEPTH[((depth as usize).saturating_sub(1)).min(ENEMIES_PER_DEPTH.len() - 1)];
    let enemy_cells: Vec<IVec2> = candidates.into_iter().take(want).collect();

    Level {
        map,
        grid,
        fov,
        spawn,
        stair,
        enemy_cells,
    }
}

/// A live monster: its cell, its health, and the sprite entity that shows it.
struct Enemy {
    cell: IVec2,
    hp: i32,
    entity: Entity,
}

// ── The game ────────────────────────────────────────────────────────────────────────────────

struct Crawler {
    level: Level,
    depth: u32,
    seed: u64,
    rng: Rng,

    /// One sprite entity per grid cell, indexed `y * COLS + x`. Fog is expressed by rewriting
    /// these colors, so there is no second rendering path for "remembered" terrain.
    cells: Vec<Entity>,
    player_e: Option<Entity>,
    stair_e: Option<Entity>,
    enemies: Vec<Enemy>,
    hp_bar: Option<Entity>,

    player: IVec2,
    hp: i32,
    /// Queued step, resolved on the next beat, plus how long it has been queued (for `ON BEAT`).
    pending: Option<IVec2>,
    pending_age: f32,

    // Soundtrack scheduling (what we *play*).
    step: usize,
    step_timer: f32,
    /// Set when the scheduler places a kick; consumed by `detect_beat` as the no-device fallback.
    scheduled_kick: bool,
    /// Whether the last beat came from the ears (`true`) or the watchdog/schedule (`false`).
    heard: bool,
    /// Seconds since the last beat fired, for the watchdog.
    since_beat: f32,

    // Beat detection (what we *hear*).
    bands: [f32; BANDS],
    low_energy: f32,
    beats: u32,
    last_on_beat: bool,
    flash: f32,

    needs_spawn: bool,
    dirty: bool,
    message: String,
    message_timer: f32,
}

impl Crawler {
    fn new(seed: u64) -> Self {
        Self {
            level: make_level(1, seed),
            depth: 1,
            seed,
            rng: Rng::new(seed),
            cells: Vec::new(),
            player_e: None,
            stair_e: None,
            enemies: Vec::new(),
            hp_bar: None,
            player: IVec2::ZERO,
            hp: PLAYER_MAX_HP,
            pending: None,
            pending_age: 0.0,
            step: 0,
            step_timer: 0.0,
            scheduled_kick: false,
            heard: false,
            since_beat: 0.0,
            bands: [0.0; BANDS],
            low_energy: 0.0,
            beats: 0,
            last_on_beat: false,
            flash: 0.0,
            needs_spawn: true,
            dirty: true,
            message: String::new(),
            message_timer: 0.0,
        }
    }

    fn cell_index(c: IVec2) -> usize {
        (c.y * COLS + c.x) as usize
    }

    fn enemy_at(&self, c: IVec2) -> Option<usize> {
        self.enemies.iter().position(|e| e.cell == c)
    }

    fn say(&mut self, text: impl Into<String>) {
        self.message = text.into();
        self.message_timer = 2.0;
    }

    /// Spawns the sprite entities for the current level. The per-cell grid is created once and
    /// then only recolored; only the actors are despawned and respawned on descent.
    fn spawn_entities(&mut self, world: &mut World) {
        if self.cells.is_empty() {
            for y in 0..ROWS {
                for x in 0..COLS {
                    let e = world.spawn();
                    let mut t = Transform::new(cell_center(x, y), Vec2::splat(CELL - 2.0), 0.0);
                    t.z = 0.1;
                    world.add_component(e, t);
                    world.add_component(e, Sprite::colored(0.0, 0.0, 0.0));
                    self.cells.push(e);
                }
            }
        }

        for old in self.enemies.drain(..) {
            world.despawn(old.entity);
        }
        if let Some(e) = self.stair_e.take() {
            world.despawn(e);
        }

        self.player = self.level.spawn;
        if self.player_e.is_none() {
            let e = world.spawn();
            let mut t = Transform::new(
                cell_center(self.player.x, self.player.y),
                Vec2::splat(CELL - 8.0),
                0.0,
            );
            t.z = 0.5;
            world.add_component(e, t);
            // Set once, never per frame: `HitFlash` owns `Sprite.color` while it is running, and a
            // game that also wrote the color every frame would fight it.
            world.add_component(
                e,
                Sprite::colored(PLAYER_COLOR.r, PLAYER_COLOR.g, PLAYER_COLOR.b),
            );
            self.player_e = Some(e);
        } else if let Some(pe) = self.player_e {
            if let Some(t) = world.get_mut::<Transform>(pe) {
                t.position = cell_center(self.player.x, self.player.y);
            }
        }

        let stair = self.level.stair;
        let e = world.spawn();
        let mut t = Transform::new(cell_center(stair.x, stair.y), Vec2::splat(CELL - 10.0), 0.0);
        t.z = 0.2;
        world.add_component(e, t);
        world.add_component(
            e,
            Sprite::colored(STAIR_COLOR.r, STAIR_COLOR.g, STAIR_COLOR.b),
        );
        self.stair_e = Some(e);

        for &c in &self.level.enemy_cells.clone() {
            let e = world.spawn();
            let mut t = Transform::new(cell_center(c.x, c.y), Vec2::splat(CELL - 9.0), 0.0);
            t.z = 0.4;
            world.add_component(e, t);
            world.add_component(
                e,
                Sprite::colored(ENEMY_COLOR.r, ENEMY_COLOR.g, ENEMY_COLOR.b),
            );
            self.enemies.push(Enemy {
                cell: c,
                hp: ENEMY_HP,
                entity: e,
            });
        }

        self.dirty = true;
    }

    fn descend(&mut self, world: &mut World) {
        self.depth += 1;
        self.seed = self.rng.next_u64();
        self.level = make_level(self.depth, self.seed);
        self.needs_spawn = true;
        self.say(format!("Depth {} — {}", self.depth, self.generator_name()));
        let _ = world;
    }

    fn generator_name(&self) -> &'static str {
        if self.depth % 2 == 1 {
            "BSP dungeon"
        } else {
            "cellular cave"
        }
    }

    fn restart(&mut self) {
        self.depth = 1;
        self.seed = self.rng.next_u64();
        self.level = make_level(1, self.seed);
        self.hp = PLAYER_MAX_HP;
        self.needs_spawn = true;
        self.say("You fell. Back to depth 1.");
    }

    /// Advances the written schedule alongside the track. This is the **only** place the pattern
    /// is consulted; the turn logic never sees it.
    ///
    /// Nothing is played here — [`TRACK`] loops on its own. All this produces is
    /// `scheduled_kick`, which exists so [`BEAT_WATCHDOG`] can tell "the music is playing and I
    /// am failing to hear it" apart from "there is nothing to hear yet".
    fn advance_schedule(&mut self, dt: f32) {
        self.step_timer += dt;
        if self.step_timer < STEP_SECS {
            return;
        }
        self.step_timer -= STEP_SECS;
        let step = PATTERN[self.step % PATTERN.len()];
        self.step = (self.step + 1) % PATTERN.len();
        if step == Step::Kick {
            self.scheduled_kick = true;
        }
    }

    /// Listens for a kick and reports whether this frame is a beat.
    ///
    /// With an audio device this is a genuine spectrum read: the bottom bands spike on the kick's
    /// sweep and sit lower between kicks, even though the bass never lets them fall silent. With
    /// **no** device there is nothing to hear, so it falls back to the schedule — the game stays
    /// playable and honest about which clock it is on.
    fn detect_beat(&mut self, world: &mut World, dt: f32) -> bool {
        let scheduled = std::mem::take(&mut self.scheduled_kick);
        self.since_beat += dt;

        let Some(audio) = world.resource::<Audio>() else {
            // No device at all: nothing to hear, so run off the schedule from the start.
            self.heard = false;
            if scheduled {
                self.since_beat = 0.0;
                return true;
            }
            return false;
        };

        audio.bands(BEAT_METER, &mut self.bands);
        self.low_energy = self.bands[..LOW_BANDS].iter().sum();
        // Retrigger guard: a cooldown, not an arm/re-arm latch. See `KICK_COOLDOWN` — under a
        // sustained mix the energy never falls back far enough for a latch to re-arm honestly.
        // `since_beat` is exactly "seconds since the last turn", so it is the guard already.
        if self.low_energy >= KICK_THRESHOLD && self.since_beat >= KICK_COOLDOWN {
            self.heard = true;
            self.since_beat = 0.0;
            return true;
        }
        // Heard nothing for too long — fall back so the dungeon cannot freeze.
        if scheduled && self.since_beat > BEAT_WATCHDOG {
            self.heard = false;
            self.since_beat = 0.0;
            return true;
        }
        false
    }

    /// One turn of the world: the player's queued step, then every enemy.
    fn tick_beat(&mut self, world: &mut World) {
        self.beats += 1;
        self.flash = 0.12;

        let on_beat = self.pending.is_some() && self.pending_age <= ON_BEAT_WINDOW;
        self.last_on_beat = on_beat;
        if let Some(dir) = self.pending.take() {
            self.player_step(world, dir, on_beat);
        }
        self.enemy_turn(world);
        self.dirty = true;
    }

    fn player_step(&mut self, world: &mut World, dir: IVec2, on_beat: bool) {
        let dest = self.player + dir;
        if !in_bounds(dest) || !self.level.map.is_floor(dest.x, dest.y) {
            return;
        }
        // Bump attack: moving into an enemy hits it instead of moving.
        if let Some(i) = self.enemy_at(dest) {
            let dmg = if on_beat { ON_BEAT_DAMAGE } else { HIT_DAMAGE };
            self.enemies[i].hp -= dmg;
            let target = self.enemies[i].entity;
            let dead = self.enemies[i].hp <= 0;

            world.add_component(target, HitFlash::white(0.18));
            let color = if on_beat { CRIT_COLOR } else { DMG_COLOR };
            let label = if on_beat {
                format!("{dmg}  ON BEAT!")
            } else {
                format!("{dmg}")
            };
            spawn_floating_text(
                world,
                cell_center(dest.x, dest.y),
                FloatingText::colored(label, color)
                    .with_size(17.0)
                    .with_lifetime(0.8),
            );
            if let Some(cam) = world.resource_mut::<engine::Camera>() {
                cam.shake(if on_beat { 5.0 } else { 2.5 }, 0.18);
            }
            if dead {
                world.despawn(target);
                self.enemies.remove(i);
            }
            return;
        }
        self.player = dest;
        if let Some(pe) = self.player_e {
            if let Some(t) = world.get_mut::<Transform>(pe) {
                t.position = cell_center(dest.x, dest.y);
            }
        }
        if dest == self.level.stair {
            self.descend(world);
        }
    }

    fn enemy_turn(&mut self, world: &mut World) {
        let player = self.player;
        let occupied: Vec<IVec2> = self.enemies.iter().map(|e| e.cell).collect();
        let mut damage_taken = 0;

        for i in 0..self.enemies.len() {
            let from = self.enemies[i].cell;
            // Only enemies you can see act — the same FOV that draws the fog gates the AI, so a
            // monster never chases you through a wall you cannot see it through.
            if !self.level.fov.is_visible(from.x, from.y) {
                continue;
            }
            let adjacent = (from - player).abs().element_sum() == 1;
            if adjacent {
                damage_taken += ENEMY_DAMAGE;
                continue;
            }
            let Some(path) = find_path(&self.level.grid, from, player) else {
                continue;
            };
            let Some(&next) = path.get(1) else { continue };
            // Do not step onto the player or onto another monster.
            if next == player || occupied.iter().any(|&c| c == next && c != from) {
                continue;
            }
            self.enemies[i].cell = next;
            let e = self.enemies[i].entity;
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position = cell_center(next.x, next.y);
            }
        }

        if damage_taken > 0 {
            self.hp -= damage_taken;
            if let Some(pe) = self.player_e {
                world.add_component(pe, HitFlash::new(HURT_COLOR, 0.22));
            }
            spawn_floating_text(
                world,
                cell_center(player.x, player.y),
                FloatingText::colored(format!("-{damage_taken}"), HURT_COLOR)
                    .with_size(18.0)
                    .with_lifetime(0.9),
            );
            if let Some(cam) = world.resource_mut::<engine::Camera>() {
                cam.shake(7.0, 0.25);
            }
            if self.hp <= 0 {
                self.restart();
            }
        }
    }

    /// Rewrites every cell sprite's color from the field of view: lit in sight, dim if remembered,
    /// fully transparent if never seen.
    fn repaint_fog(&mut self, world: &mut World) {
        self.level.fov.compute(self.player, TORCH_RADIUS);
        for y in 0..ROWS {
            for x in 0..COLS {
                let c = IVec2::new(x, y);
                let e = self.cells[Self::cell_index(c)];
                let color = if !self.level.fov.is_revealed(x, y) {
                    UNSEEN
                } else {
                    let visible = self.level.fov.is_visible(x, y);
                    match (self.level.map.is_wall(x, y), visible) {
                        (true, true) => WALL_LIT,
                        (true, false) => WALL_DIM,
                        (false, true) => FLOOR_LIT,
                        (false, false) => FLOOR_DIM,
                    }
                };
                if let Some(s) = world.get_mut::<Sprite>(e) {
                    s.color = color;
                }
            }
        }
        // The stair and the monsters are only drawn where you can actually see them.
        let stair_visible = self
            .level
            .fov
            .is_visible(self.level.stair.x, self.level.stair.y);
        if let Some(se) = self.stair_e {
            if let Some(s) = world.get_mut::<Sprite>(se) {
                s.color.a = if stair_visible { 1.0 } else { 0.0 };
            }
        }
        let visible: Vec<(Entity, bool)> = self
            .enemies
            .iter()
            .map(|e| (e.entity, self.level.fov.is_visible(e.cell.x, e.cell.y)))
            .collect();
        for (e, vis) in visible {
            // Skip while a HitFlash is running so the two never fight over the same field.
            if world.get::<HitFlash>(e).is_some() {
                continue;
            }
            if let Some(s) = world.get_mut::<Sprite>(e) {
                s.color.a = if vis { 1.0 } else { 0.0 };
            }
        }
    }
}

impl System for Crawler {
    fn run(&mut self, world: &mut World, dt: f32) {
        if self.needs_spawn {
            self.spawn_entities(world);
            self.needs_spawn = false;
        }

        // ── Input: queue a step; it resolves on the beat ────────────────────────
        let (mut mv, mut quit) = (IVec2::ZERO, false);
        if let Some(input) = world.resource::<InputState>() {
            quit = input.just_pressed(KeyCode::Escape);
            if input.just_pressed(KeyCode::KeyW) || input.just_pressed(KeyCode::ArrowUp) {
                mv.y -= 1;
            }
            if input.just_pressed(KeyCode::KeyS) || input.just_pressed(KeyCode::ArrowDown) {
                mv.y += 1;
            }
            if input.just_pressed(KeyCode::KeyA) || input.just_pressed(KeyCode::ArrowLeft) {
                mv.x -= 1;
            }
            if input.just_pressed(KeyCode::KeyD) || input.just_pressed(KeyCode::ArrowRight) {
                mv.x += 1;
            }
        }
        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if mv != IVec2::ZERO {
            self.pending = Some(mv);
            self.pending_age = 0.0;
        } else if self.pending.is_some() {
            self.pending_age += dt;
        }

        // ── Track the written schedule, then listen to the track itself ────────
        self.advance_schedule(dt);
        if self.detect_beat(world, dt) {
            self.tick_beat(world);
        }
        self.flash = (self.flash - dt).max(0.0);
        self.message_timer = (self.message_timer - dt).max(0.0);

        if self.needs_spawn {
            // A descent queued inside the beat: rebuild before drawing anything.
            self.spawn_entities(world);
            self.needs_spawn = false;
        }
        if self.dirty {
            self.repaint_fog(world);
            self.dirty = false;
        }

        // ── HUD ─────────────────────────────────────────────────────────────────
        if let Some(bar) = self.hp_bar {
            if let Some(pb) = world.get_mut::<ProgressBar>(bar) {
                pb.value = (self.hp as f32 / PLAYER_MAX_HP as f32).clamp(0.0, 1.0);
            }
        }
        let clock = if self.heard {
            "bands() — listening"
        } else {
            "schedule (nothing heard)"
        };
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!(
                    "Depth {} ({})   ·   seed {}   ·   monsters {}   ·   beats {}",
                    self.depth,
                    self.generator_name(),
                    self.seed,
                    self.enemies.len(),
                    self.beats,
                ),
                Vec2::new(GRID_OX, HUD_TOP),
                17.0,
                TEXT_MAIN,
            ));
            let beat_mark = if self.flash > 0.0 { "◆ BEAT" } else { "◇" };
            tq.push(DrawText::new(
                format!(
                    "{beat_mark}  turn clock: {clock}   low-band {:.2} / {KICK_THRESHOLD:.1}   {}",
                    self.low_energy,
                    if self.last_on_beat { "ON BEAT x2" } else { "" }
                ),
                // Starts clear of the health bar, which occupies GRID_OX..GRID_OX+HP_BAR_W.
                Vec2::new(GRID_OX + HP_BAR_W + 18.0, HUD_ROW_BAR - 2.0),
                15.0,
                if self.flash > 0.0 {
                    CRIT_COLOR
                } else {
                    TEXT_DIM
                },
            ));
            tq.push(DrawText::new(
                "WASD / arrows: queue a step (resolves on the beat)    bump a monster to attack    green = stair    Esc: quit",
                Vec2::new(GRID_OX, HUD_ROW_KEYS),
                13.0,
                TEXT_DIM,
            ));
            if self.message_timer > 0.0 {
                tq.push(DrawText::new(
                    self.message.clone(),
                    Vec2::new(GRID_OX, HUD_ROW_MSG),
                    15.0,
                    CRIT_COLOR,
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "beat_crawler"
    }
}

struct CrawlScene;

impl Scene for CrawlScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        let mut game = Crawler::new(0xB3A7_C7A2_1234_5678);

        // Health gauge — the widget suite doing a real job rather than a demo one.
        let bar = world.spawn();
        world.add_component(bar, UiNode::new(GRID_OX, HUD_ROW_BAR, HP_BAR_W, HP_BAR_H));
        world.add_component(bar, ProgressBar::new(1.0));
        game.hp_bar = Some(bar);

        // ⚠️ `AudioFacadeSystem` belongs to the SCENE, not to `main`.
        //
        // `SceneCmd::Replace` swaps out the entire systems list, so a system registered on the
        // `App` before `set_scene` is silently dropped. This one ticks `Audio::update`, and
        // `bands()` only ever reports what that tick published — registered one line too early,
        // every band read back `0.000` forever, this game ran permanently on `BEAT_WATCHDOG`'s
        // schedule, and the only symptom was the HUD reading "schedule (nothing heard)" instead
        // of "listening". Registering it here makes the ordering unforgeable: the system that
        // feeds the turn clock is owned by the scene whose turn clock it is.
        systems.add(AudioFacadeSystem);
        systems.add(game);
    }
    fn on_exit(&mut self, _world: &mut World) {}
}

// ── Headless acceptance test ────────────────────────────────────────────────────────────────

/// Asserts the things a screenshot cannot: that every level is solvable, that enemies actually
/// close distance, and that the low-band detector separates a kick from a blip.
fn self_test() -> i32 {
    // 1. Solvability across a spread of depths — both generators, several seeds.
    for depth in 1..=6u32 {
        let level = make_level(depth, 1000 + depth as u64);
        let dist = bfs_distances(&level.grid, level.spawn);
        let d = dist[(level.stair.y * COLS + level.stair.x) as usize];
        if d < 0 {
            eprintln!("FAIL: depth {depth} stair unreachable from spawn");
            return 1;
        }
        println!("depth {depth} ({} cells to the stair) ok", d);
        for &c in &level.enemy_cells {
            let ed = dist[(c.y * COLS + c.x) as usize];
            if ed < MIN_ENEMY_DIST {
                eprintln!("FAIL: depth {depth} enemy at {c:?} only {ed} steps from spawn");
                return 2;
            }
        }
    }

    // 2. Enemies close distance: one `find_path` step must reduce the BFS distance to the player.
    let level = make_level(1, 4242);
    let dist = bfs_distances(&level.grid, level.spawn);
    let far = level.enemy_cells.first().copied().unwrap_or(level.stair);
    if let Some(path) = find_path(&level.grid, far, level.spawn) {
        if let Some(&next) = path.get(1) {
            let before = dist[(far.y * COLS + far.x) as usize];
            let after = dist[(next.y * COLS + next.x) as usize];
            if after >= before {
                eprintln!("FAIL: pathing did not approach ({before} -> {after})");
                return 3;
            }
            println!("pathing approaches: {before} -> {after} ok");
        }
    }

    // 3. The rhythm itself — needs a real device, so a box without one SKIPS.
    let Some(mut audio) = Audio::new() else {
        println!("SKIP: no audio device available (level checks passed)");
        return 0;
    };
    audio.enable_spectrum(BEAT_METER);
    audio.play_music(TRACK);

    // Run the game's own detector against the real track for a few bars.
    //
    // The previous version of this check played the game's two tones and asserted they were far
    // apart in the low band. That could not fail: kick 110 Hz and blip 880 Hz measured 4.00 vs
    // 0.61, and nothing in between was ever played. With a mix there is no such gap to assert on,
    // so the question becomes the real one — does the detector find the kicks, at the spacing the
    // music actually has?
    //
    // Timing comes off `Instant`, not an accumulator: `sleep(1/60)` sleeps *at least* 1/60, so a
    // `t += 1.0/60.0` clock runs slower than the music and every measured gap comes out short.
    // That mistake made a correct detector look like it was firing 40% too often.
    const BARS: f32 = 4.0;
    let mut bands = [0.0f32; BANDS];
    let mut fires: Vec<f32> = Vec::new();
    let start = std::time::Instant::now();
    let (mut prev, mut last_fire, mut frame) = (start, -99.0f32, 0u32);
    loop {
        frame += 1;
        let target = start + std::time::Duration::from_secs_f32(frame as f32 / 60.0);
        let now = std::time::Instant::now();
        if target > now {
            std::thread::sleep(target - now);
        }
        let now = std::time::Instant::now();
        let t = (now - start).as_secs_f32();
        audio.update((now - prev).as_secs_f32());
        prev = now;
        audio.bands(BEAT_METER, &mut bands);
        let low: f32 = bands[..LOW_BANDS].iter().sum();
        // Skip the first bar: the analyser is still filling its first FFT windows, and playback
        // starts at an arbitrary offset into ours.
        let bar_secs = STEP_SECS * PATTERN.len() as f32;
        if t > bar_secs && low >= KICK_THRESHOLD && t - last_fire >= KICK_COOLDOWN {
            last_fire = t;
            fires.push(t);
        }
        if t >= bar_secs * (BARS + 1.0) {
            break;
        }
    }
    audio.stop_music();

    if fires.is_empty() {
        eprintln!("FAIL: no kick was ever detected in the soundtrack");
        return 4;
    }

    // 4 kicks per bar is the ground truth; allow one either side for where the window lands.
    let expected = (BARS * 4.0) as usize;
    let gaps: Vec<f32> = fires.windows(2).map(|p| p[1] - p[0]).collect();
    let on_grid = gaps
        .iter()
        .filter(|g| (**g - BEAT_SECS).abs() <= 0.12)
        .count();
    let mean = gaps.iter().sum::<f32>() / gaps.len().max(1) as f32;
    println!(
        "kicks heard {} (expected ~{expected}), mean gap {mean:.3}s (grid {BEAT_SECS:.3}s), \
         on-grid {on_grid}/{}",
        fires.len(),
        gaps.len()
    );
    if fires.len().abs_diff(expected) > 1 || on_grid * 10 < gaps.len() * 8 {
        eprintln!(
            "FAIL: detections did not land on the beat grid — the turn clock is hearing the \
             bass, or missing kicks"
        );
        return 5;
    }

    println!(
        "PASS: levels solvable, enemies approach, {} kicks found in a real mix at {mean:.3}s \
         spacing with threshold {KICK_THRESHOLD:.2}",
        fires.len()
    );
    0
}

fn main() {
    if std::env::var("BEAT_CRAWLER_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }

    let mut app = App::new();
    let win_w = (COLS as f32 * CELL + GRID_OX * 2.0) as u32;
    let win_h = (ROWS as f32 * CELL + GRID_OY * 2.0 + HUD_H) as u32;
    app.world.insert_resource(WindowConfig {
        title: "Beat Crawler — the music is the turn clock".to_string(),
        width: win_w,
        height: win_h,
        clear_color: [0.03, 0.03, 0.05, 1.0],
    });
    if let Some(cam) = app.world.resource_mut::<engine::Camera>() {
        cam.position = Vec2::ZERO;
    }

    if let Some(mut audio) = Audio::new() {
        // Opt in BEFORE the first play: the analyser is wired into a sound as it starts, so
        // enabling it later would not take effect until the next play on this channel.
        audio.enable_spectrum(BEAT_METER);
        audio.play_music(TRACK);
        app.world.insert_resource(audio);
    }
    // No `register_persistent::<Audio>()` here: the engine registers it in `App::new` (v0.141.1).
    // This example is why that had to stop being the game's job — its turn clock *is* the audio
    // device, so a dropped `Audio` would not merely mute it, the world would stop taking turns.

    // `AudioFacadeSystem` is deliberately NOT registered here — `CrawlScene::on_enter` owns it,
    // because a system added before `set_scene` is dropped by the scene swap. See the comment
    // there; that ordering silently disabled this game's turn clock.
    app.set_scene(Box::new(CrawlScene));
    app.add_system(HitFlashSystem);
    app.add_system(FloatingTextSystem);
    app.add_system(UiSystem::new());

    app.run();
}
