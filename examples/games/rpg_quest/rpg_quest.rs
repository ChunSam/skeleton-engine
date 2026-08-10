//! rpg_quest — the RPG slice: a dialogue-driven quest that gates world access and persists.
//!
//! `docs/VISION.md` names five genres its success criteria have to cover — platformer, shooter,
//! **RPG**, puzzle, top-down action. Four had a playable game in `examples/games/`; this is the
//! fifth. The dialogue system had five examples before this one and every one of them was a
//! *demo*: `dialogue_demo`, `dialogue_branching`, `dialogue_portrait`, `dialogue_style` and
//! `dialogue_quest` all sit at the top level, open with a box already on screen, and end when the
//! conversation does. None of them walks anywhere, fights anything, or is still there after you
//! quit — so the pieces an RPG needs *together* had never been under gameplay pressure at once.
//!
//! The loop, which is the whole point:
//!
//!   1. **Walk** the village. Walls block (`CollisionGridSystem` + `SpatialGrid`, AABB queries).
//!   2. **Talk** to the merchant (`E` when you are next to him) — a `DialogueTree` loaded from
//!      RON, localized, with conditional choices.
//!   3. **A choice changes the world.** Buying the lantern spends gold and grants
//!      `has_lantern`; asking the gated follow-up records `knows_mine` and lights the mine door.
//!   4. **The lantern gates access.** The mine door carries a wall collider while you have no
//!      lantern; `DoorSystem` removes it once you do. Nothing scripts the door open — it is the
//!      same collider query the walls use, so the gate is real.
//!   5. **Fight**, with stats nobody typed into this file: the slime's hp/strength/bounty come
//!      from `enemies.ron` through `DataTableRegistry`.
//!   6. **Persist.** `F5` saves, `F9` loads, and the game auto-loads at launch.
//!
//! # What this example changed in the engine
//!
//! Step 6 did not work when this example was first written, and the reason is the kind of thing
//! `VISION.md` says examples exist to find. Quest state belongs in `DialogueVars` — that is where
//! choice conditions read from — but `DialogueVars` was **not serializable and had no iterator**.
//! Its getters answer one *already-known* key at a time, so persisting quest state meant hardcoding
//! the name of every flag in the game at the save site and re-`set_*`-ing each one on load: a save
//! path that silently drops any flag someone adds later. `DialogueVars` now derives
//! `Serialize`/`Deserialize` (its values already did) and exposes `iter()`, so the whole bag
//! round-trips in one line — see `SaveData` below.
//!
//! **The second gap it found is now fixed, and this example is what closed it.** A choice's `cond`
//! was a single `DialogueCond`, so `gold >= 10 && !has_lantern` — an entirely ordinary shop gate —
//! could not be written in the tree; the game precomputed it into a `can_buy_lantern` flag it
//! recalculated every frame. That is a designer needing a programmer for a two-term gate.
//! `DialogueChoice` now also carries `cond_all` (conjunction) and `cond_any` (disjunction), the
//! flag and its upkeep are deleted, and `rpg_quest.dlg.ron` states both branches of the shop gate
//! directly. ⚠️ The gate's *price* now lives in the tree next to `LANTERN_COST` here, which is a
//! real coupling. Selftest check 2 pins the two together from **both** sides of the boundary — at
//! `LANTERN_COST - 1` the offer is hidden and "no deal" shows, at exactly `LANTERN_COST` they
//! swap — so a tree written against a different price cannot pass. Check 4's purchase is not
//! enough on its own: it buys at `START_GOLD`, which has ten gold of slack.
//!
//! Run from the repo root:  `cargo run --example rpg_quest_game`
//! WASD/arrows move · `E` talk · SPACE advance · 1/2/3 choose · `F5` save · `F9` load ·
//! `R` reset (deletes the save) · `L` language · ESC quit.
//!
//! - `RPG_QUEST_SELFTEST=1 cargo run --example rpg_quest_game` — asserts the quest gates and the
//!   save round-trip. A screenshot cannot: a save file that dropped `knows_mine` photographs
//!   exactly like one that kept it, and the door's collider has no pixels at all.

use engine::{
    dialogue, save, App, Camera, Collider, CollisionGridSystem, CollisionLayer, Color, DataTable,
    DataTableRegistry, DialogueBox, DialogueEvent, DialogueRegistry, DialogueSystem, DialogueVars,
    DrawText, Entity, Events, InputState, KeyCode, LocaleResource, ShouldQuit, SpatialGrid, Sprite,
    System, TextQueue, Transform, Vec2, WindowConfig, World,
};
use serde::{Deserialize, Serialize};

// ─── Layout ──────────────────────────────────────────────────────────────────

const TILE: f32 = 44.0;

/// `#` wall · `.` floor · `P` player spawn · `M` merchant · `+` mine door · `S` slime.
///
/// The mine (rows 5–7, cols 6–11) is sealed by its own wall ring, and `+` is the only opening —
/// so "can the player reach the slime" is decided entirely by whether the door has a collider.
const MAP: &[&str] = &[
    "################",
    "#..............#",
    "#..P........M..#",
    "#..............#",
    "#....########..#",
    "#....#......#..#",
    "#....+..S...#..#",
    "#....#......#..#",
    "#....########..#",
    "#..............#",
    "################",
];

const MAP_COLS: u32 = 16;
const MAP_ROWS: u32 = 11;
const WINDOW_W: u32 = MAP_COLS * TILE as u32;
const WINDOW_H: u32 = MAP_ROWS * TILE as u32;

const WALL_LAYER: u32 = 1 << 0;
const PLAYER_HALF: f32 = TILE * 0.34;
const PLAYER_SPEED: f32 = 165.0;
const TALK_RANGE: f32 = TILE * 1.4;
const CONTACT_RANGE: f32 = TILE * 0.85;

const START_GOLD: i64 = 20;
const LANTERN_COST: i64 = 10;
const PLAYER_MAX_HP: i32 = 30;
const PLAYER_STRENGTH: i32 = 6;
/// Seconds between blows while you stand on the slime. Measured, not guessed: at 6 damage against
/// the table's 24 hp the fight is 4 exchanges, which at this cadence is a bit under two seconds —
/// long enough to read the hp ticking down, short enough not to be a chore.
const ATTACK_COOLDOWN: f32 = 0.45;

const DLG_PATH: &str = "examples/games/rpg_quest/rpg_quest.dlg.ron";
const ENEMIES_PATH: &str = "examples/games/rpg_quest/enemies.ron";

// ─── Quest variables ─────────────────────────────────────────────────────────
//
// These live in `DialogueVars` rather than in a game struct, because the dialogue tree's `cond`
// fields read from there — putting them anywhere else means the conversation cannot see the world.
// That is also why `DialogueVars` had to become serializable: this list is the save file's payload,
// and it grows every time someone adds a quest step.

const VAR_GOLD: &str = "gold";
const VAR_HAS_LANTERN: &str = "has_lantern";
const VAR_KNOWS_MINE: &str = "knows_mine";
const VAR_SLIME_SLAIN: &str = "slime_slain";

const LOCALES_RON: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: {
            "npc.merchant":   "Old Merchant",
            "dlg.intro":      "You look like you're headed east. The old mine swallowed the last three who tried.",
            "dlg.ask":        "A lantern keeps the dark honest. Ten gold.",
            "dlg.buy":        "Here — a steady flame. Mind the wind down there.",
            "dlg.nodeal":     "Perhaps another time, then.",
            "dlg.ask2":       "Anything else I can do for you?",
            "dlg.secret":     "The door's on the west face of the rock. Something wet lives past it — go armed.",
            "dlg.bye":        "Safe travels, friend.",
            "choice.buy":     "Buy the lantern (10 gold)",
            "choice.nodeal":  "I'll pass for now",
            "choice.leave":   "I'll be off",
            "choice.more":    "One more thing...",
            "choice.secret":  "Where is this mine, exactly?",
            "choice.go":      "I'll head out, then",
        } ),
        "ko": ( translations: {
            "npc.merchant":   "노상인",
            "dlg.intro":      "동쪽으로 갈 모양이군. 그 옛 광산은 앞서 간 셋을 그대로 삼켰다네.",
            "dlg.ask":        "등불이 있으면 어둠도 함부로 못 하지. 열 골드일세.",
            "dlg.buy":        "여기 — 꺼지지 않는 불꽃이야. 아래는 바람이 험하니 조심하게.",
            "dlg.nodeal":     "그럼 다음 기회에 하지.",
            "dlg.ask2":       "더 필요한 게 있나?",
            "dlg.secret":     "문은 바위 서쪽 면에 있네. 그 안에 축축한 것이 사니, 무장하고 가게.",
            "dlg.bye":        "부디 무사히 가게, 친구.",
            "choice.buy":     "등불을 산다 (10 골드)",
            "choice.nodeal":  "지금은 넘어가지",
            "choice.leave":   "이만 가보겠네",
            "choice.more":    "하나만 더",
            "choice.secret":  "그 광산이 정확히 어디인가?",
            "choice.go":      "그럼 이만 나서겠네",
        } ),
    },
)
"#;

// ─── Session state ───────────────────────────────────────────────────────────

/// Entity handles + the spawn point, so systems do not re-query for them every frame.
struct Quest {
    player: Entity,
    merchant: Entity,
    slime: Entity,
    door: Entity,
    dialogue_box: Entity,
    player_spawn: Vec2,
    slime_spawn: Vec2,
    /// Where this session's save file lives. A parameter rather than a constant so the selftest
    /// can use its own slot and never read or clobber a real player's save.
    save_path: std::path::PathBuf,
    /// Last thing that happened, shown in the HUD so save/load/purchase are visible in play.
    notice: String,
}

struct PlayerState {
    hp: i32,
    attack_cd: f32,
}

/// The slime, seeded from `enemies.ron` — nothing here is a literal in this file.
struct SlimeState {
    name: String,
    hp: i32,
    max_hp: i32,
    strength: i32,
    bounty: i64,
}

/// Everything a reload has to bring back.
///
/// `vars` is the whole `DialogueVars` bag, not a hand-listed set of flags. That is the difference
/// the engine change bought: a quest step added to `rpg_quest.dlg.ron` tomorrow persists without
/// anyone touching this struct.
#[derive(Serialize, Deserialize)]
struct SaveData {
    player_pos: (f32, f32),
    player_hp: i32,
    slime_hp: i32,
    vars: DialogueVars,
}

// ─── Map parsing ─────────────────────────────────────────────────────────────

fn tile_center(col: usize, row: usize) -> Vec2 {
    Vec2::new(
        col as f32 * TILE + TILE * 0.5,
        row as f32 * TILE + TILE * 0.5,
    )
}

/// Read a `(row, col)` integer cell from the enemies table.
///
/// Returns `None` rather than defaulting, so [`slime_from_table`] can tell "the table is missing"
/// from "the table says 0" — the selftest's seeding check depends on that distinction.
fn table_int(table: &DataTable, row: usize, col: &str) -> Option<i64> {
    match table.get(row, col)? {
        ron::Value::Number(ron::Number::Integer(n)) => Some(*n),
        _ => None,
    }
}

fn table_str(table: &DataTable, row: usize, col: &str) -> Option<String> {
    match table.get(row, col)? {
        ron::Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

/// Seed the slime from row 0 of the `enemies` table.
///
/// The fallbacks are deliberately *wrong-looking* (1 hp, 1 strength, 0 bounty) so a table that
/// failed to load produces an obviously broken slime instead of a plausible one. A default that
/// matched the file would make the load failure invisible — the same trap as a screenshot that
/// flatters itself.
fn slime_from_table(registry: Option<&DataTableRegistry>) -> SlimeState {
    let table = registry.and_then(|r| r.get("enemies"));
    let hp = table.and_then(|t| table_int(t, 0, "hp")).unwrap_or(1) as i32;
    SlimeState {
        name: table
            .and_then(|t| table_str(t, 0, "name"))
            .unwrap_or_else(|| "???".to_string()),
        hp,
        max_hp: hp,
        strength: table.and_then(|t| table_int(t, 0, "strength")).unwrap_or(1) as i32,
        bounty: table.and_then(|t| table_int(t, 0, "bounty")).unwrap_or(0),
    }
}

// ─── Movement ────────────────────────────────────────────────────────────────

fn blocked(grid: &SpatialGrid, center: Vec2, half: f32) -> bool {
    let min = center - Vec2::splat(half);
    let max = center + Vec2::splat(half);
    !grid
        .query_aabb(min, max, CollisionLayer(WALL_LAYER))
        .is_empty()
}

/// Axis-separated slide against `WALL_LAYER` colliders, so brushing a wall does not stop you dead.
///
/// The mine door is one of those colliders while it is locked — the gate is the same query the
/// walls use, not a special case in this function.
fn resolve_walls(world: &World, current: Vec2, proposed: Vec2, half: f32) -> Vec2 {
    let Some(grid) = world.resource::<SpatialGrid>() else {
        return proposed;
    };
    let mut resolved = current;
    if !blocked(grid, Vec2::new(proposed.x, current.y), half) {
        resolved.x = proposed.x;
    }
    if !blocked(grid, Vec2::new(resolved.x, proposed.y), half) {
        resolved.y = proposed.y;
    }
    resolved
}

// ─── Quest helpers (shared by the systems and the selftest) ──────────────────

fn var_bool(world: &World, key: &str) -> bool {
    world
        .resource::<DialogueVars>()
        .and_then(|v| v.get_bool(key))
        .unwrap_or(false)
}

fn gold(world: &World) -> i64 {
    world
        .resource::<DialogueVars>()
        .and_then(|v| v.get_int(VAR_GOLD))
        .unwrap_or(0)
}

/// The starting quest state, used by a fresh game and by `R`.
fn fresh_vars() -> DialogueVars {
    let mut vars = DialogueVars::default();
    vars.set_int(VAR_GOLD, START_GOLD);
    vars.set_bool(VAR_HAS_LANTERN, false);
    vars.set_bool(VAR_KNOWS_MINE, false);
    vars.set_bool(VAR_SLIME_SLAIN, false);
    vars
}

/// Whether a conversation is on screen. Takes the entity rather than the `Quest` resource so it can
/// be called while a resource borrow is already out, and so the selftest can ask the same question.
fn dialogue_open(world: &World, box_entity: Entity) -> bool {
    world
        .get::<DialogueBox>(box_entity)
        .map(|d| !d.is_finished())
        .unwrap_or(false)
}

// ─── Save / load ─────────────────────────────────────────────────────────────

fn write_save(world: &mut World) -> Result<(), save::SaveError> {
    let Some((player, path)) = world
        .resource::<Quest>()
        .map(|q| (q.player, q.save_path.clone()))
    else {
        return Ok(());
    };
    let pos = world
        .get::<Transform>(player)
        .map(|t| t.position)
        .unwrap_or(Vec2::ZERO);
    let data = SaveData {
        player_pos: (pos.x, pos.y),
        player_hp: world.resource::<PlayerState>().map(|p| p.hp).unwrap_or(0),
        slime_hp: world.resource::<SlimeState>().map(|s| s.hp).unwrap_or(0),
        // The one-liner the engine change bought. Every flag, including ones added later.
        vars: world
            .resource::<DialogueVars>()
            .cloned()
            .unwrap_or_default(),
    };
    save::write_ron(&path, &data)
}

/// Apply a `SaveData` to the live world. Split out from [`load_save`] so the selftest can assert on
/// the applied state rather than on the file.
fn apply_save(world: &mut World, data: SaveData) {
    let Some((player, slime)) = world.resource::<Quest>().map(|q| (q.player, q.slime)) else {
        return;
    };
    if let Some(t) = world.get_mut::<Transform>(player) {
        t.position = Vec2::new(data.player_pos.0, data.player_pos.1);
    }
    if let Some(p) = world.resource_mut::<PlayerState>() {
        p.hp = data.player_hp;
        p.attack_cd = 0.0;
    }
    if let Some(s) = world.resource_mut::<SlimeState>() {
        s.hp = data.slime_hp;
    }
    world.insert_resource(data.vars);
    // A slain slime has no sprite; a living one does. Re-derive it rather than trusting whatever
    // the world happened to be showing before the load.
    let slain = var_bool(world, VAR_SLIME_SLAIN);
    let visible = world.get::<Sprite>(slime).is_some();
    if slain && visible {
        world.remove_component::<Sprite>(slime);
    } else if !slain && !visible {
        world.add_component(slime, Sprite::colored(0.45, 0.85, 0.55));
    }
}

fn load_save(world: &mut World) -> bool {
    let Some(path) = world.resource::<Quest>().map(|q| q.save_path.clone()) else {
        return false;
    };
    match save::read_ron::<SaveData>(&path) {
        Ok(data) => {
            apply_save(world, data);
            true
        }
        Err(e) => {
            log::info!("rpg_quest: no save loaded ({e})");
            false
        }
    }
}

fn reset_game(world: &mut World) {
    let Some((player, slime, spawn, slime_spawn, path)) = world.resource::<Quest>().map(|q| {
        (
            q.player,
            q.slime,
            q.player_spawn,
            q.slime_spawn,
            q.save_path.clone(),
        )
    }) else {
        return;
    };
    let _ = save::delete(&path);
    if let Some(t) = world.get_mut::<Transform>(player) {
        t.position = spawn;
    }
    if let Some(t) = world.get_mut::<Transform>(slime) {
        t.position = slime_spawn;
    }
    if let Some(p) = world.resource_mut::<PlayerState>() {
        p.hp = PLAYER_MAX_HP;
        p.attack_cd = 0.0;
    }
    let max = world
        .resource::<SlimeState>()
        .map(|s| s.max_hp)
        .unwrap_or(1);
    if let Some(s) = world.resource_mut::<SlimeState>() {
        s.hp = max;
    }
    if world.get::<Sprite>(slime).is_none() {
        world.add_component(slime, Sprite::colored(0.45, 0.85, 0.55));
    }
    world.insert_resource(fresh_vars());
    let dialogue_box = world.resource::<Quest>().map(|q| q.dialogue_box);
    if let Some(b) = dialogue_box {
        if let Some(d) = world.get_mut::<DialogueBox>(b) {
            d.reset();
        }
    }
}

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Movement, talking, and the meta keys. While a conversation is open, movement is off and the
/// number keys pick choices — the ordinary RPG modality, and the reason this is one system.
struct RpgInputSystem;

impl System for RpgInputSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(keys) = world.resource::<InputState>().map(|i| Keys {
            axis: {
                let mut x = 0.0;
                let mut y = 0.0;
                if i.is_pressed(KeyCode::KeyA) || i.is_pressed(KeyCode::ArrowLeft) {
                    x -= 1.0;
                }
                if i.is_pressed(KeyCode::KeyD) || i.is_pressed(KeyCode::ArrowRight) {
                    x += 1.0;
                }
                if i.is_pressed(KeyCode::KeyW) || i.is_pressed(KeyCode::ArrowUp) {
                    y -= 1.0;
                }
                if i.is_pressed(KeyCode::KeyS) || i.is_pressed(KeyCode::ArrowDown) {
                    y += 1.0;
                }
                Vec2::new(x, y)
            },
            talk: i.just_pressed(KeyCode::KeyE),
            advance: i.just_pressed(KeyCode::Space),
            choose: [
                i.just_pressed(KeyCode::Digit1),
                i.just_pressed(KeyCode::Digit2),
                i.just_pressed(KeyCode::Digit3),
            ],
            save: i.just_pressed(KeyCode::F5),
            load: i.just_pressed(KeyCode::F9),
            reset: i.just_pressed(KeyCode::KeyR),
            language: i.just_pressed(KeyCode::KeyL),
            quit: i.just_pressed(KeyCode::Escape),
        }) else {
            return;
        };

        if keys.quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.quit();
            }
        }
        if keys.language {
            if let Some(locale) = world.resource_mut::<LocaleResource>() {
                let next = if locale.current_locale() == "en" {
                    "ko"
                } else {
                    "en"
                };
                locale.set_locale(next);
            }
        }
        if keys.reset {
            reset_game(world);
            set_notice(world, "reset — save deleted");
            return;
        }
        if keys.save {
            let msg = match write_save(world) {
                Ok(()) => "saved (F9 to load)".to_string(),
                Err(e) => format!("save failed: {e}"),
            };
            set_notice(world, &msg);
        }
        if keys.load {
            let msg = if load_save(world) {
                "loaded"
            } else {
                "no save file yet"
            };
            set_notice(world, msg);
        }

        let Some((player, merchant, dialogue_box)) = world
            .resource::<Quest>()
            .map(|q| (q.player, q.merchant, q.dialogue_box))
        else {
            return;
        };

        // ── In conversation: choices and advance, no walking ──────────────────
        if dialogue_open(world, dialogue_box) {
            for (i, pressed) in keys.choose.iter().enumerate() {
                if *pressed {
                    dialogue::choose(world, dialogue_box, i);
                    return;
                }
            }
            if keys.advance {
                dialogue::advance(world, dialogue_box);
            }
            return;
        }

        // ── Out of conversation: walk, and talk when in range ─────────────────
        if keys.talk {
            let player_pos = world
                .get::<Transform>(player)
                .map(|t| t.position)
                .unwrap_or(Vec2::ZERO);
            let merchant_pos = world
                .get::<Transform>(merchant)
                .map(|t| t.position)
                .unwrap_or(Vec2::ZERO);
            if (merchant_pos - player_pos).length() <= TALK_RANGE {
                if let Some(d) = world.get_mut::<DialogueBox>(dialogue_box) {
                    d.reset();
                }
                return;
            }
            set_notice(world, "nobody close enough to talk to");
        }

        let direction = if keys.axis.length_squared() > 0.0 {
            keys.axis.normalize()
        } else {
            return;
        };
        let Some(current) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        let proposed = current + direction * PLAYER_SPEED * dt;
        let resolved = resolve_walls(world, current, proposed, PLAYER_HALF);
        if let Some(t) = world.get_mut::<Transform>(player) {
            t.position = resolved;
        }
    }

    fn name(&self) -> &'static str {
        "rpg_input"
    }
}

struct Keys {
    axis: Vec2,
    talk: bool,
    advance: bool,
    choose: [bool; 3],
    save: bool,
    load: bool,
    reset: bool,
    language: bool,
    quit: bool,
}

fn set_notice(world: &mut World, msg: &str) {
    if let Some(q) = world.resource_mut::<Quest>() {
        q.notice = msg.to_string();
    }
}

/// Turns dialogue events into world consequences.
///
/// It used to do a second job — recompute a derived `can_buy_lantern` flag every frame, because
/// `gold >= LANTERN_COST && !has_lantern` could not be written in the tree. `cond_all` /
/// `cond_any` (v0.152.0) took that job back into the data, and the flag is gone.
struct QuestEventSystem;

impl System for QuestEventSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let events: Vec<String> = world
            .resource::<Events<DialogueEvent>>()
            .map(|ev| ev.read().iter().map(|e| e.name.clone()).collect())
            .unwrap_or_default();

        for name in events {
            if name == "buy_lantern" {
                // One event, two consequences — a choice carries one effect, so the game splits it.
                let paid = gold(world) - LANTERN_COST;
                if let Some(v) = world.resource_mut::<DialogueVars>() {
                    v.set_int(VAR_GOLD, paid);
                    v.set_bool(VAR_HAS_LANTERN, true);
                }
                set_notice(world, "bought the lantern — the mine door will open");
            }
        }
    }

    fn name(&self) -> &'static str {
        "quest_events"
    }
}

/// Adds or removes the mine door's wall collider to match `has_lantern`.
///
/// Idempotent on purpose: it is also what makes a loaded save consistent, since `F9` can drop the
/// player into either state.
struct DoorSystem;

impl System for DoorSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(door) = world.resource::<Quest>().map(|q| q.door) else {
            return;
        };
        let open = var_bool(world, VAR_HAS_LANTERN);
        let locked = world.get::<Collider>(door).is_some();
        if open && locked {
            world.remove_component::<Collider>(door);
        } else if !open && !locked {
            world.add_component(
                door,
                Collider::Aabb {
                    half_extents: Vec2::splat(TILE * 0.5),
                },
            );
        }
        // Colour tracks the gate so the state is readable in a screenshot: locked = dull stone,
        // unlocked = warm lantern light, and lit gold once the merchant has told you where it is.
        let knows = var_bool(world, VAR_KNOWS_MINE);
        let sprite = match (open, knows) {
            (true, _) => Sprite::colored(0.85, 0.68, 0.30),
            (false, true) => Sprite::colored(0.55, 0.45, 0.25),
            (false, false) => Sprite::colored(0.26, 0.29, 0.38),
        };
        world.add_component(door, sprite);
    }

    fn name(&self) -> &'static str {
        "door"
    }
}

/// Bump-to-attack: stand on the slime and you trade blows on a cooldown.
struct CombatSystem;

impl System for CombatSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some((player, slime)) = world.resource::<Quest>().map(|q| (q.player, q.slime)) else {
            return;
        };
        if let Some(p) = world.resource_mut::<PlayerState>() {
            p.attack_cd = (p.attack_cd - dt).max(0.0);
        }
        if var_bool(world, VAR_SLIME_SLAIN) {
            return;
        }
        let (Some(ppos), Some(spos)) = (
            world.get::<Transform>(player).map(|t| t.position),
            world.get::<Transform>(slime).map(|t| t.position),
        ) else {
            return;
        };
        if (spos - ppos).length() > CONTACT_RANGE {
            return;
        }
        if world.resource::<PlayerState>().map(|p| p.attack_cd) != Some(0.0) {
            return;
        }

        let slime_strength = world
            .resource::<SlimeState>()
            .map(|s| s.strength)
            .unwrap_or(0);
        if let Some(s) = world.resource_mut::<SlimeState>() {
            s.hp -= PLAYER_STRENGTH;
        }
        if let Some(p) = world.resource_mut::<PlayerState>() {
            p.hp -= slime_strength;
            p.attack_cd = ATTACK_COOLDOWN;
        }

        let slime_dead = world.resource::<SlimeState>().map(|s| s.hp <= 0) == Some(true);
        if slime_dead {
            let bounty = world
                .resource::<SlimeState>()
                .map(|s| s.bounty)
                .unwrap_or(0);
            let purse = gold(world) + bounty;
            if let Some(v) = world.resource_mut::<DialogueVars>() {
                v.set_int(VAR_GOLD, purse);
                v.set_bool(VAR_SLIME_SLAIN, true);
            }
            world.remove_component::<Sprite>(slime);
            set_notice(world, "the slime bursts — quest complete (F5 to save)");
            return;
        }

        let player_dead = world.resource::<PlayerState>().map(|p| p.hp <= 0) == Some(true);
        if player_dead {
            // A fair retry: back to the square with full hp, the slime healed, quest flags kept.
            let (spawn, max) = world
                .resource::<Quest>()
                .map(|q| q.player_spawn)
                .zip(world.resource::<SlimeState>().map(|s| s.max_hp))
                .unwrap_or((Vec2::ZERO, 1));
            if let Some(t) = world.get_mut::<Transform>(player) {
                t.position = spawn;
            }
            if let Some(p) = world.resource_mut::<PlayerState>() {
                p.hp = PLAYER_MAX_HP;
                p.attack_cd = 0.0;
            }
            if let Some(s) = world.resource_mut::<SlimeState>() {
                s.hp = max;
            }
            set_notice(world, "you black out and wake in the square");
        }
    }

    fn name(&self) -> &'static str {
        "combat"
    }
}

struct HudSystem;

impl System for HudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let hp = world.resource::<PlayerState>().map(|p| p.hp).unwrap_or(0);
        let purse = gold(world);
        let lantern = var_bool(world, VAR_HAS_LANTERN);
        let knows = var_bool(world, VAR_KNOWS_MINE);
        let slain = var_bool(world, VAR_SLIME_SLAIN);
        let slime = world
            .resource::<SlimeState>()
            .map(|s| (s.name.clone(), s.hp, s.max_hp))
            .unwrap_or_else(|| ("???".to_string(), 0, 0));
        let lang = world
            .resource::<LocaleResource>()
            .map(|l| l.current_locale().to_string())
            .unwrap_or_default();
        let notice = world
            .resource::<Quest>()
            .map(|q| q.notice.clone())
            .unwrap_or_default();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            tq.push(DrawText::new(
                format!("HP {hp}/{PLAYER_MAX_HP}    {purse} gold"),
                Vec2::new(14.0, 20.0),
                18.0,
                Color::rgb(0.92, 0.9, 0.82),
            ));

            let quest_line = if slain {
                "[done] the mine is quiet".to_string()
            } else if lantern {
                format!("[!] {} — {}/{} hp", slime.0, slime.1.max(0), slime.2)
            } else if knows {
                "[?] the door is on the west face".to_string()
            } else {
                "[ ] talk to the merchant (E)".to_string()
            };
            let quest_color = if slain {
                Color::rgb(0.55, 0.9, 0.6)
            } else if lantern {
                Color::rgb(1.0, 0.82, 0.35)
            } else {
                Color::rgb(0.6, 0.64, 0.72)
            };
            tq.push(DrawText::new(
                quest_line,
                Vec2::new(14.0, 42.0),
                18.0,
                quest_color,
            ));

            tq.push(DrawText::new(
                format!(
                    "WASD move · E talk · SPACE/1-3 dialogue · F5 save · F9 load · R reset · L [{lang}] · ESC"
                ),
                Vec2::new(14.0, WINDOW_H as f32 - 18.0),
                14.0,
                Color::rgb(0.5, 0.55, 0.66),
            ));
            // Top-left, not above the controls line: `DialogueSystem` owns the bottom of the screen
            // and its choice list reaches y ≈ H-48, which the first capture showed this line
            // colliding with mid-conversation.
            if !notice.is_empty() {
                tq.push(DrawText::new(
                    notice,
                    Vec2::new(14.0, 64.0),
                    16.0,
                    Color::rgb(0.75, 0.8, 0.95),
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "hud"
    }
}

// ─── Setup ───────────────────────────────────────────────────────────────────

/// The game's own setup, used by `main` **and** by the selftest — a harness that rebuilt this would
/// never catch a bug in it.
///
/// `save_slot` names the save file's directory so the selftest cannot read or overwrite a real
/// player's progress.
fn build_app(save_slot: &str) -> App {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "rpg_quest — dialogue-gated quest with a save file".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.05, 0.06, 0.08, 1.0],
    });
    app.world.insert_resource(Camera::new(Vec2::ZERO, 1.0));
    app.world
        .insert_resource(LocaleResource::from_ron_str(LOCALES_RON).expect("valid locale bundle"));
    app.world.insert_resource(fresh_vars());
    app.register_event::<DialogueEvent>();

    app.load_data_table("enemies", ENEMIES_PATH);
    let slime_state = slime_from_table(app.world.resource::<DataTableRegistry>());

    // ── Map ──────────────────────────────────────────────────────────────────
    let mut player_spawn = tile_center(1, 1);
    let mut merchant_spawn = tile_center(1, 1);
    let mut slime_spawn = tile_center(1, 1);
    let mut door_cell = (0usize, 0usize);

    for (row, line) in MAP.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            let center = tile_center(col, row);
            match ch {
                'P' => player_spawn = center,
                'M' => merchant_spawn = center,
                'S' => slime_spawn = center,
                '+' => door_cell = (row, col),
                _ => {}
            }
            let e = app.world.spawn();
            if ch == '#' {
                app.world.add_component(
                    e,
                    Transform {
                        position: center,
                        scale: Vec2::splat(TILE),
                        rotation: 0.0,
                        z: 0.0,
                    },
                );
                app.world
                    // Wall vs floor needs a wide gap in *linear* space: the renderer writes to an
                    // sRGB surface, so 0.16 and 0.10 — a sane-looking pair in a colour picker —
                    // both land near mid-grey on screen and the map reads as one flat slab. First
                    // capture of this example showed exactly that.
                    .add_component(e, Sprite::colored(0.30, 0.33, 0.42));
                app.world.add_component(
                    e,
                    Collider::Aabb {
                        half_extents: Vec2::splat(TILE * 0.5),
                    },
                );
                app.world.add_component(e, CollisionLayer(WALL_LAYER));
            } else {
                app.world.add_component(
                    e,
                    Transform {
                        position: center,
                        scale: Vec2::splat(TILE),
                        rotation: 0.0,
                        z: -1.0,
                    },
                );
                app.world
                    .add_component(e, Sprite::colored(0.055, 0.065, 0.09));
            }
        }
    }

    // ── The door: a wall that `DoorSystem` unlocks ────────────────────────────
    let door = app.world.spawn();
    app.world.add_component(
        door,
        Transform {
            position: tile_center(door_cell.1, door_cell.0),
            scale: Vec2::splat(TILE),
            rotation: 0.0,
            z: 0.1,
        },
    );
    app.world.add_component(door, CollisionLayer(WALL_LAYER));
    // No `Collider` and no `Sprite` here — `DoorSystem` owns both, so the locked/unlocked state has
    // exactly one writer and a loaded save cannot disagree with it.

    let merchant = app.world.spawn();
    app.world.add_component(
        merchant,
        Transform {
            position: merchant_spawn,
            scale: Vec2::splat(TILE * 0.72),
            rotation: 0.0,
            z: 1.0,
        },
    );
    app.world
        .add_component(merchant, Sprite::colored(0.55, 0.65, 0.95));

    let slime = app.world.spawn();
    app.world.add_component(
        slime,
        Transform {
            position: slime_spawn,
            scale: Vec2::splat(TILE * 0.66),
            rotation: 0.0,
            z: 1.0,
        },
    );
    app.world
        .add_component(slime, Sprite::colored(0.45, 0.85, 0.55));

    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: player_spawn,
            scale: Vec2::splat(TILE * 0.68),
            rotation: 0.0,
            z: 2.0,
        },
    );
    app.world
        .add_component(player, Sprite::colored(0.95, 0.86, 0.38));

    // ── Dialogue ─────────────────────────────────────────────────────────────
    app.load_dialogue("merchant", DLG_PATH);
    let mut merchant_box = app
        .world
        .resource::<DialogueRegistry>()
        .and_then(|r| r.box_of("merchant"))
        .unwrap_or_else(|| panic!("failed to load {DLG_PATH} (run from the repo root)"));
    // Start finished: the conversation opens when the player presses `E`, not at launch.
    merchant_box.current = merchant_box.lines.len().max(merchant_box.line_keys.len());
    let dialogue_box = app.world.spawn();
    app.world.add_component(dialogue_box, merchant_box);

    app.world.insert_resource(PlayerState {
        hp: PLAYER_MAX_HP,
        attack_cd: 0.0,
    });
    app.world.insert_resource(slime_state);
    app.world.insert_resource(Quest {
        player,
        merchant,
        slime,
        door,
        dialogue_box,
        player_spawn,
        slime_spawn,
        save_path: save::save_path(save_slot, "quest.ron"),
        notice: String::new(),
    });

    // Order: input (may emit a dialogue event) → events (grant the lantern) → door (react to the
    // flag) → combat → hud → the engine's dialogue tick/render.
    app.add_system(CollisionGridSystem::new(TILE * 2.0));
    app.add_system(RpgInputSystem);
    app.add_system(QuestEventSystem);
    app.add_system(DoorSystem);
    app.add_system(CombatSystem);
    app.add_system(HudSystem);
    app.add_system(DialogueSystem);
    app
}

fn main() {
    // `RPG_QUEST_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("RPG_QUEST_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }

    #[cfg(not(target_arch = "wasm32"))]
    let _ = env_logger::try_init();

    let mut app = build_app("skeleton_rpg_quest");
    // Auto-load, the way an RPG does. A first run finds nothing and starts fresh.
    if load_save(&mut app.world) {
        set_notice(&mut app.world, "save loaded");
    }
    app.run();
}

// ─── Acceptance test ─────────────────────────────────────────────────────────

/// `RPG_QUEST_SELFTEST=1 cargo run --example rpg_quest_game` — asserts what this example exists to
/// show and a screenshot cannot.
///
/// Three of these have no pixels at all:
///
/// * **A collider is invisible.** The mine door's gate is a `Collider` that `DoorSystem` adds and
///   removes. A locked door and an unlocked one photograph identically apart from a colour this
///   example chose to tint them — and that tint is drawn by the same system, so a screenshot of a
///   "locked" door proves the *tint* ran, not that anything blocks. Only walking into it decides.
/// * **A save file that dropped a flag looks exactly like one that kept it.** Reloading and
///   finding `knows_mine` gone renders a perfectly plausible game: the door is simply un-lit. The
///   only way to see the difference is to write the state, destroy it in memory, read it back, and
///   compare.
/// * **The reason `DialogueVars` had to become serializable** is check 7, and it is deliberately a
///   flag this file has no constant for. Hand-listing flags at the save site passes checks 1–6
///   forever; it fails the moment someone adds a quest step. That is the regression this example
///   was written to make impossible, so it gets its own check.
///
/// A hidden dialogue choice (checks 2 and 4) has no pixels either — the whole point of a gate is
/// that the ungated screen looks complete.
///
/// The harness stands in for exactly one thing: it teleports the player instead of walking him
/// across the village, because the walk is not the subject and checks 3 and 5 do drive real held
/// input through `RpgInputSystem` over the distance that *is*. Everything else — buying, gating,
/// saving — goes through the game's own systems in `build_app`'s order.
///
/// Exit codes: `0` pass · `1` the slime was not seeded from `enemies.ron` · `2` a gate that should
/// hide a choice does not · `3` the locked door does not block · `4` buying the lantern through the
/// real conversation does not grant it (or the gated follow-up stays hidden) · `5` the lantern does
/// not open the door · `6` the save round-trip loses state · `7` the round-trip only carries the
/// flags this file happens to name.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{InputAction, InputScript};

    /// Fixed step: every claim below is driven by scripted input and `dt`. Nothing waits on a wall
    /// clock, so this reproduces real play exactly.
    const DT: f32 = 1.0 / 60.0;
    /// Its own save slot, so a run can never read or clobber a real player's progress.
    const SLOT: &str = "skeleton_rpg_quest_selftest";

    fn steps(app: &mut App, n: u32) {
        for _ in 0..n {
            app.step_headless(DT);
        }
    }

    /// Hold `key` down for `frames` frames of real ticking, then release — the `InputScript` path,
    /// because `InputState` has no public press setter and a harness that wrote it directly would
    /// stop testing the game's own input read.
    fn hold(app: &mut App, key: KeyCode, frames: u32) {
        app.set_input_script(InputScript::new([
            (0, InputAction::KeyDown(key)),
            (frames, InputAction::KeyUp(key)),
        ]));
        steps(app, frames + 2);
    }

    fn tap(app: &mut App, key: KeyCode) {
        app.set_input_script(InputScript::new([(0, InputAction::KeyPress(key))]));
        steps(app, 3);
    }

    fn teleport(app: &mut App, to: Vec2) {
        let player = app.world.resource::<Quest>().map(|q| q.player);
        if let Some(p) = player {
            if let Some(t) = app.world.get_mut::<Transform>(p) {
                t.position = to;
            }
        }
    }

    fn player_x(app: &App) -> f32 {
        app.world
            .resource::<Quest>()
            .and_then(|q| app.world.get::<Transform>(q.player))
            .map(|t| t.position.x)
            .unwrap_or(f32::NAN)
    }

    fn dialogue_entity(app: &App) -> Entity {
        app.world
            .resource::<Quest>()
            .map(|q| q.dialogue_box)
            .expect("Quest resource")
    }

    /// The localization keys of the choices currently offered, gates applied.
    fn visible_keys(app: &App) -> Vec<String> {
        let Some(d) = app.world.get::<DialogueBox>(dialogue_entity(app)) else {
            return Vec::new();
        };
        let fallback = DialogueVars::default();
        let vars = app.world.resource::<DialogueVars>().unwrap_or(&fallback);
        d.visible_choices(vars)
            .iter()
            .filter_map(|c| c.key.clone())
            .collect()
    }

    fn is_choosing(app: &App) -> bool {
        let Some(d) = app.world.get::<DialogueBox>(dialogue_entity(app)) else {
            return false;
        };
        let fallback = DialogueVars::default();
        let vars = app.world.resource::<DialogueVars>().unwrap_or(&fallback);
        d.is_choosing(vars)
    }

    /// Open the merchant's conversation and press SPACE until it is offering choices.
    ///
    /// A press count rather than a frame count, because the typewriter means the first SPACE
    /// completes the line and the second moves on — asserting on the *state* instead of guessing
    /// how many frames 40 chars/sec needs.
    fn talk_until_choices(app: &mut App) -> bool {
        tap(app, KeyCode::KeyE);
        for _ in 0..12 {
            if is_choosing(app) {
                return true;
            }
            tap(app, KeyCode::Space);
        }
        is_choosing(app)
    }

    fn set_gold(app: &mut App, amount: i64) {
        if let Some(v) = app.world.resource_mut::<DialogueVars>() {
            v.set_int(VAR_GOLD, amount);
        }
    }

    /// Play the conversation out to its end, taking the first offered choice at every prompt.
    ///
    /// ⚠️ **Required before any walking check.** `RpgInputSystem` disables movement while a box is
    /// open — correctly, that is the RPG modality — so a walk attempted mid-conversation does not
    /// move at all. The first draft of check 3 passed on exactly that: the player "was blocked by
    /// the locked door" while actually being blocked by an open dialogue box, and it took check 5
    /// failing to expose it. The control in check 3 is the other half of the fix.
    fn close_dialogue(app: &mut App) -> bool {
        for _ in 0..24 {
            if !dialogue_open(&app.world, dialogue_entity(app)) {
                return true;
            }
            if is_choosing(app) {
                tap(app, KeyCode::Digit1);
            } else {
                tap(app, KeyCode::Space);
            }
        }
        !dialogue_open(&app.world, dialogue_entity(app))
    }

    // Cell centres the checks aim at, derived from MAP rather than retyped.
    let door_x = tile_center(5, 6).x;
    let outside_door = tile_center(4, 6);
    let mine_inside_x = 6.0 * TILE;
    let beside_merchant = tile_center(11, 2);

    // ── Arrange ──────────────────────────────────────────────────────────────
    //
    // Delete first: a save left by an earlier run would start this one mid-quest and every gate
    // below would be asked about the wrong state. (`build_app` does not auto-load — `main` does —
    // but the file has to be gone before check 6 writes its own.)
    let _ = save::delete(&save::save_path(SLOT, "quest.ron"));
    let mut app = build_app(SLOT);
    steps(&mut app, 2); // let CollisionGridSystem build the grid and the derived vars settle

    // ── 1. The slime came from enemies.ron ───────────────────────────────────
    //
    // First, because every check after this one assumes a real slime. The fallbacks in
    // `slime_from_table` are 1 hp / 1 strength / 0 bounty, so a table that failed to load is
    // obvious here instead of quietly making check 6 assert on nothing.
    let table_hp = app
        .world
        .resource::<DataTableRegistry>()
        .and_then(|r| r.get("enemies"))
        .and_then(|t| table_int(t, 0, "hp"));
    let seeded = app
        .world
        .resource::<SlimeState>()
        .map(|s| (s.name.clone(), s.hp, s.strength, s.bounty));
    match (table_hp, &seeded) {
        (Some(hp), Some((name, live_hp, strength, bounty)))
            if *live_hp as i64 == hp && name != "???" && *strength > 1 && *bounty > 0 =>
        {
            println!(
                "table ok: '{name}' seeded from enemies.ron — {live_hp} hp, {strength} strength, \
                 {bounty} bounty"
            );
        }
        _ => {
            eprintln!(
                "FAIL: the slime was not seeded from {ENEMIES_PATH} — table hp {table_hp:?}, live \
                 slime {seeded:?}. Everything below assumes a real enemy."
            );
            return 1;
        }
    }

    // ── 2. The gates hide what they should, before anything is earned ────────
    //
    // The negative half, and it is the half that gives checks 4 and 5 meaning: a build that simply
    // ignored every gate field would pass all of the positive checks below. It runs both gate
    // shapes against each other — `choice.buy`'s `cond_all` and `choice.nodeal`'s `cond_any` are
    // exact negations, so at every gold value exactly one of them is offered.
    set_gold(&mut app, LANTERN_COST - 1);
    steps(&mut app, 2);
    teleport(&mut app, beside_merchant);
    if !talk_until_choices(&mut app) {
        eprintln!("FAIL: pressing E next to the merchant never reached a choice prompt.");
        return 2;
    }
    let broke_keys = visible_keys(&app);
    if broke_keys.iter().any(|k| k == "choice.buy") {
        eprintln!(
            "FAIL: the lantern is on offer at {} gold with a cost of {LANTERN_COST} — the \
             cond_all gate on choice.buy is not being read. Visible: {broke_keys:?}",
            LANTERN_COST - 1
        );
        return 2;
    }
    // The `cond_any` half — the exact negation — must be offering the other branch, or the
    // merchant has nothing to say and "no buy offer" above would also be true of a tree that
    // failed to load at all.
    if !broke_keys.iter().any(|k| k == "choice.nodeal") {
        eprintln!(
            "FAIL: broke at {} gold and the merchant offers neither branch — the cond_any gate \
             on choice.nodeal is not being read. Visible: {broke_keys:?}",
            LANTERN_COST - 1
        );
        return 2;
    }
    println!(
        "gate ok: broke at {} gold, no buy offer, 'no deal' instead — {broke_keys:?}",
        LANTERN_COST - 1
    );

    // One more gold, nothing else touched: the offer must flip. This pins the gate's *boundary* to
    // `LANTERN_COST` exactly, which matters because the price now lives in the tree too. Check 4 is
    // not enough on its own — it buys at `START_GOLD`, ten gold of slack, so a tree written against
    // the wrong price would sail through it. It also shows the gate is read live out of the tree:
    // the old build needed a `QuestEventSystem` frame to recompute a derived flag first, and this
    // changes no flag at all.
    set_gold(&mut app, LANTERN_COST);
    let afford_keys = visible_keys(&app);
    if !afford_keys.iter().any(|k| k == "choice.buy") {
        eprintln!(
            "FAIL: exactly {LANTERN_COST} gold and no buy offer — the cond_all gate disagrees \
             with LANTERN_COST. Visible: {afford_keys:?}"
        );
        return 2;
    }
    if afford_keys.iter().any(|k| k == "choice.nodeal") {
        eprintln!(
            "FAIL: both branches offered at {LANTERN_COST} gold — cond_all and cond_any are \
             supposed to be exact negations. Visible: {afford_keys:?}"
        );
        return 2;
    }
    println!(
        "boundary ok: at exactly {LANTERN_COST} gold the offer flips to buy — {afford_keys:?}"
    );
    // Required, not tidiness: check 3 closes the conversation by taking the first offered choice
    // at every prompt, and at this gold that first choice is `buy` — which would grant the lantern,
    // open the mine door, and make the locked-door check assert against an unlocked door.
    set_gold(&mut app, LANTERN_COST - 1);

    // ── 3. The locked door blocks ────────────────────────────────────────────
    //
    // Real held input over a real distance: one second at 165 px/s is ~165 px, and the door is 44 px
    // away, so a door that does not block puts the player well inside the mine.
    if !close_dialogue(&mut app) {
        eprintln!("FAIL: the conversation would not close, so no walking check can run.");
        return 3;
    }
    // The control, and it is not optional: "the player did not move" is the same reading for a
    // working door, a stuck input path, and an open dialogue box. Walk the other way first, over
    // open floor, and require actual movement — otherwise the assertion below proves nothing.
    teleport(&mut app, outside_door);
    hold(&mut app, KeyCode::KeyA, 20);
    let moved_x = player_x(&app);
    if moved_x >= outside_door.x - 1.0 {
        eprintln!(
            "FAIL: the player did not move west over open floor — x {moved_x:.1}, started at \
             {:.1}. Held input is not reaching movement, so the door check below cannot mean \
             anything.",
            outside_door.x
        );
        return 3;
    }
    println!(
        "input ok: held A moved the player {:.1} px west over open floor",
        outside_door.x - moved_x
    );

    teleport(&mut app, outside_door);
    hold(&mut app, KeyCode::KeyD, 60);
    let blocked_x = player_x(&app);
    if blocked_x >= door_x {
        eprintln!(
            "FAIL: the player walked through a locked mine door — x {blocked_x:.1}, door at \
             {door_x:.1}. has_lantern is {}, so DoorSystem should be holding a collider there.",
            var_bool(&app.world, VAR_HAS_LANTERN)
        );
        return 3;
    }
    println!("door ok: stopped at x {blocked_x:.1}, west of the door at {door_x:.1}");

    // ── 4. Buy it through the real conversation ──────────────────────────────
    set_gold(&mut app, START_GOLD);
    steps(&mut app, 2);
    teleport(&mut app, beside_merchant);
    if !talk_until_choices(&mut app) {
        eprintln!("FAIL: could not reopen the conversation with enough gold.");
        return 4;
    }
    let rich_keys = visible_keys(&app);
    let Some(buy_index) = rich_keys.iter().position(|k| k == "choice.buy") else {
        eprintln!(
            "FAIL: no buy offer at {START_GOLD} gold — the gate is stuck shut. Visible: \
             {rich_keys:?}"
        );
        return 4;
    };
    // Press the number key for that visible slot: 1/2/3 → indices 0/1/2, the same mapping a player
    // uses, routed through RpgInputSystem rather than calling `dialogue::choose` directly.
    let digit = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3][buy_index];
    tap(&mut app, digit);
    steps(&mut app, 2); // QuestEventSystem turns the event into gold + lantern

    if !var_bool(&app.world, VAR_HAS_LANTERN) || gold(&app.world) != START_GOLD - LANTERN_COST {
        eprintln!(
            "FAIL: buying the lantern did not take. has_lantern {}, gold {} (want {}).",
            var_bool(&app.world, VAR_HAS_LANTERN),
            gold(&app.world),
            START_GOLD - LANTERN_COST
        );
        return 4;
    }
    println!(
        "purchase ok: lantern granted, gold {} → {}",
        START_GOLD,
        gold(&app.world)
    );

    // The gated follow-up must now appear — the positive half of check 2.
    if !talk_until_choices(&mut app) {
        eprintln!("FAIL: the conversation did not continue past the purchase.");
        return 4;
    }
    let mut after_keys = visible_keys(&app);
    for _ in 0..4 {
        if after_keys.iter().any(|k| k == "choice.secret") {
            break;
        }
        // Walk on through the tree (choice slot 0) looking for the ask2 node.
        tap(&mut app, KeyCode::Digit1);
        if !talk_until_choices(&mut app) {
            break;
        }
        after_keys = visible_keys(&app);
    }
    if !after_keys.iter().any(|k| k == "choice.secret") {
        eprintln!(
            "FAIL: the has_lantern-gated 'secret' choice never appeared with the lantern in hand. \
             Visible at the last prompt: {after_keys:?}"
        );
        return 4;
    }
    let secret_index = after_keys
        .iter()
        .position(|k| k == "choice.secret")
        .expect("checked above");
    tap(
        &mut app,
        [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3][secret_index],
    );
    steps(&mut app, 2);
    if !var_bool(&app.world, VAR_KNOWS_MINE) {
        eprintln!("FAIL: taking the gated choice did not apply its SetVar effect (knows_mine).");
        return 4;
    }
    println!("gated choice ok: appeared with the lantern, and its SetVar set knows_mine");

    // ── 5. The lantern opens the door ────────────────────────────────────────
    //
    // Same walk as check 3, same input, only the flag differs — so a pass here is about the
    // collider and nothing else.
    if !close_dialogue(&mut app) {
        eprintln!("FAIL: the conversation would not close after the purchase.");
        return 5;
    }
    teleport(&mut app, outside_door);
    hold(&mut app, KeyCode::KeyD, 60);
    let open_x = player_x(&app);
    if open_x < mine_inside_x {
        eprintln!(
            "FAIL: the lantern did not open the mine door — x {open_x:.1}, mine interior starts at \
             {mine_inside_x:.1}. DoorSystem should have removed the collider."
        );
        return 5;
    }
    println!("lantern ok: same walk now reaches x {open_x:.1}, inside the mine");

    // ── 6. Save, destroy, load ───────────────────────────────────────────────
    let saved_pos = Vec2::new(open_x, tile_center(4, 6).y);
    if let Some(p) = app.world.resource_mut::<PlayerState>() {
        p.hp = 17; // a value neither the default nor the max, so a reset is not mistaken for a load
    }
    if let Some(s) = app.world.resource_mut::<SlimeState>() {
        s.hp = 9;
    }
    if let Err(e) = write_save(&mut app.world) {
        eprintln!("FAIL: could not write the save file: {e}");
        return 6;
    }

    // Wreck everything the save is supposed to carry.
    teleport(&mut app, tile_center(1, 1));
    if let Some(p) = app.world.resource_mut::<PlayerState>() {
        p.hp = 1;
    }
    if let Some(s) = app.world.resource_mut::<SlimeState>() {
        s.hp = 1;
    }
    app.world.insert_resource(DialogueVars::default());

    if !load_save(&mut app.world) {
        eprintln!("FAIL: the save written a moment ago did not load back.");
        return 6;
    }
    let restored_pos = app
        .world
        .resource::<Quest>()
        .and_then(|q| app.world.get::<Transform>(q.player))
        .map(|t| t.position)
        .unwrap_or(Vec2::ZERO);
    let restored_hp = app.world.resource::<PlayerState>().map(|p| p.hp);
    let restored_slime = app.world.resource::<SlimeState>().map(|s| s.hp);
    let ok = (restored_pos - saved_pos).length() < 0.5
        && restored_hp == Some(17)
        && restored_slime == Some(9)
        && var_bool(&app.world, VAR_HAS_LANTERN)
        && var_bool(&app.world, VAR_KNOWS_MINE)
        && gold(&app.world) == START_GOLD - LANTERN_COST;
    if !ok {
        eprintln!(
            "FAIL: the save round-trip lost state. pos {restored_pos:?} (want {saved_pos:?}), hp \
             {restored_hp:?} (want 17), slime hp {restored_slime:?} (want 9), has_lantern {}, \
             knows_mine {}, gold {} (want {}).",
            var_bool(&app.world, VAR_HAS_LANTERN),
            var_bool(&app.world, VAR_KNOWS_MINE),
            gold(&app.world),
            START_GOLD - LANTERN_COST
        );
        return 6;
    }
    println!(
        "save ok: position, hp {}, slime hp {}, lantern, knows_mine and {} gold all came back",
        restored_hp.unwrap_or(0),
        restored_slime.unwrap_or(0),
        gold(&app.world)
    );

    // ── 7. A flag this file never names round-trips too ──────────────────────
    //
    // The check that makes `DialogueVars: Serialize` worth having. Nothing in this example declares
    // "side_quest_started" — no constant, no getter, no field in `SaveData`. A save path that
    // hand-listed the flags it knows about (which is what the API forced before) passes checks 1–6
    // and fails here, which is precisely the regression that would otherwise ship silently the next
    // time somebody adds a quest step to rpg_quest.dlg.ron.
    if let Some(v) = app.world.resource_mut::<DialogueVars>() {
        v.set_bool("side_quest_started", true);
        v.set_int("bandits_defeated", 3);
    }
    if let Err(e) = write_save(&mut app.world) {
        eprintln!("FAIL: could not write the save file for the unnamed-flag check: {e}");
        return 7;
    }
    app.world.insert_resource(DialogueVars::default());
    if !load_save(&mut app.world) {
        eprintln!("FAIL: the second save did not load back.");
        return 7;
    }
    let extra_bool = var_bool(&app.world, "side_quest_started");
    let extra_int = app
        .world
        .resource::<DialogueVars>()
        .and_then(|v| v.get_int("bandits_defeated"));
    if !extra_bool || extra_int != Some(3) {
        eprintln!(
            "FAIL: flags this file does not name were dropped by the save — side_quest_started \
             {extra_bool}, bandits_defeated {extra_int:?} (want true / Some(3)). The save is \
             carrying a hand-listed subset of DialogueVars, not the bag."
        );
        return 7;
    }
    let total = app.world.resource::<DialogueVars>().map(|v| v.len());
    println!(
        "unnamed flags ok: side_quest_started and bandits_defeated survived; {total:?} vars in the \
         restored bag"
    );

    // Leave nothing behind for the next run to trip over.
    let _ = save::delete(&save::save_path(SLOT, "quest.ron"));
    println!("PASS: rpg_quest");
    0
}
