//! Top-down "lit dungeon" playable example — dogfoods 2D lighting + PostProcess.
//!
//! Validates `PointLight` / `AmbientLight` and `PostProcessConfig` together in real
//! play for the first time (both were shipped but never exercised by a game; see
//! `docs/NEXT_WORK.md`). The dungeon is almost pitch black: the player carries a
//! torch (`PointLight`) whose radius/intensity **decay over time** (fuel). Lighting
//! a scattered brazier spawns a persistent `PointLight` and refills the torch. Light
//! every brazier to open the exit, then reach it. Run out of fuel in the dark and you
//! lose.
//!
//! The level holds **more than 16 light sources** at full clear (torch + 16 braziers +
//! exit glow = 18) to pressure the engine's 16-light hard cap; the camera follows the
//! player, so the nearest-16 culling (added alongside this example in
//! `src/renderer/lighting.rs`) is visible as distant braziers fading out.
//!
//! Known limitations this example deliberately lives within (documented, not bugs):
//!   * No occlusion / shadows — light passes through walls (radial attenuation only).
//!   * Lighting is native-only; on wasm the App render path disables it, so the wasm
//!     build renders unlit. PostProcess still works on wasm.
//!
//! Controls: WASD / Arrows move · E light a nearby brazier · P toggle post-process ·
//! R restart · Esc quit.

use engine::{
    AmbientLight, App, Camera, Collider, CollisionGridSystem, CollisionLayer, Color, DrawText,
    Entity, InputState, KeyCode, PointLight, PostProcessConfig, ShouldQuit, SpatialGrid, Sprite,
    System, TextQueue, Transform, WindowConfig, World,
};
use glam::Vec2;

// ─── Layout / constants ──────────────────────────────────────────────────────

const TILE: f32 = 64.0;

// 15 cols × 12 rows. '#' wall, '.' floor, 'P' player spawn, 'B' brazier, 'X' exit.
// Interior is mostly open with only isolated pillars, so every floor cell is
// reachable (no hand-verified maze needed). 16 braziers.
const MAP: &[&str] = &[
    "###############",
    "#P............#",
    "#..B...B...B..#",
    "#....##....##.#",
    "#.B....B....B.#",
    "#......B......#",
    "#.##....B..##.#",
    "#..B......B...#",
    "#....B...B....#",
    "#.B.......B.B.#",
    "#.....B......X#",
    "###############",
];

const COLS: usize = 15;
const ROWS: usize = 12;
const LEVEL_W: f32 = COLS as f32 * TILE;
const LEVEL_H: f32 = ROWS as f32 * TILE;

const WINDOW_W: u32 = 800;
const WINDOW_H: u32 = 600;

const PLAYER_HALF: f32 = TILE * 0.3;
const PLAYER_SPEED: f32 = 235.0;
const WALL_LAYER: u32 = 1 << 0;
const INTERACT_RADIUS: f32 = TILE * 0.95;
const EXIT_RADIUS: f32 = TILE * 0.6;

// Torch fuel: full burn lasts MAX_FUEL seconds; lighting a brazier refills it.
const MAX_FUEL: f32 = 13.0;
const TORCH_MIN_RADIUS: f32 = 36.0;
const TORCH_MAX_RADIUS: f32 = 240.0;
const TORCH_MAX_INTENSITY: f32 = 1.7;

const TILE_FLOOR: u32 = 1;
const TILE_WALL: u32 = 2;

// Warm/cool light colors.
const TORCH_COLOR: Color = Color::rgb(1.0, 0.82, 0.5);
const BRAZIER_COLOR: Color = Color::rgb(1.0, 0.64, 0.32);
const EXIT_COLOR: Color = Color::rgb(0.5, 1.0, 0.75);

// ─── Session ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Playing,
    Won,
    Lost,
}

struct Brazier {
    entity: Entity,
    pos: Vec2,
    lit: bool,
}

struct DungeonSession {
    player: Entity,
    player_spawn: Vec2,
    braziers: Vec<Brazier>,
    exit_entity: Entity,
    exit_pos: Vec2,
    exit_lit: bool,
    fuel: f32,
    status: Status,
    post_enabled: bool,
}

impl DungeonSession {
    fn lit_count(&self) -> usize {
        self.braziers.iter().filter(|b| b.lit).count()
    }
}

// ─── Layout parsing ──────────────────────────────────────────────────────────

struct ParsedMap {
    tiles: Vec<Vec<u32>>,
    player: Vec2,
    exit: Vec2,
    braziers: Vec<Vec2>,
}

fn tile_center(col: usize, row: usize) -> Vec2 {
    Vec2::new(
        col as f32 * TILE + TILE * 0.5,
        row as f32 * TILE + TILE * 0.5,
    )
}

fn parse_map() -> ParsedMap {
    let mut tiles = Vec::with_capacity(ROWS);
    let mut player = Vec2::ZERO;
    let mut exit = Vec2::ZERO;
    let mut braziers = Vec::new();

    for (row, line) in MAP.iter().enumerate() {
        let mut row_tiles = Vec::with_capacity(COLS);
        for (col, ch) in line.chars().enumerate() {
            row_tiles.push(if ch == '#' { TILE_WALL } else { TILE_FLOOR });
            let center = tile_center(col, row);
            match ch {
                'P' => player = center,
                'X' => exit = center,
                'B' => braziers.push(center),
                _ => {}
            }
        }
        tiles.push(row_tiles);
    }

    ParsedMap {
        tiles,
        player,
        exit,
        braziers,
    }
}

// ─── Wall collision (SpatialGrid resource, per-axis slide) ─────────────────────

fn resolve_walls(world: &World, current: Vec2, proposed: Vec2, half: f32) -> Vec2 {
    let Some(grid) = world.resource::<SpatialGrid>() else {
        return proposed;
    };
    let mask = CollisionLayer(WALL_LAYER);
    let mut resolved = current;

    if !overlaps_wall(grid, Vec2::new(proposed.x, current.y), half, mask) {
        resolved.x = proposed.x;
    }
    if !overlaps_wall(grid, Vec2::new(resolved.x, proposed.y), half, mask) {
        resolved.y = proposed.y;
    }
    resolved
}

fn overlaps_wall(grid: &SpatialGrid, center: Vec2, half: f32, mask: CollisionLayer) -> bool {
    let min = center - Vec2::splat(half);
    let max = center + Vec2::splat(half);
    !grid.query_aabb(min, max, mask).is_empty()
}

fn torch_from_fuel(fuel: f32) -> (f32, f32) {
    let frac = (fuel / MAX_FUEL).clamp(0.0, 1.0);
    let radius = TORCH_MIN_RADIUS + (TORCH_MAX_RADIUS - TORCH_MIN_RADIUS) * frac;
    let intensity = TORCH_MAX_INTENSITY * frac;
    (radius, intensity)
}

// ─── Systems ─────────────────────────────────────────────────────────────────

/// Movement + brazier interaction + global keys (P/R/Esc).
struct PlayerSystem;
impl System for PlayerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some(input) = world.resource::<InputState>() else {
            return;
        };
        let mut axis = Vec2::ZERO;
        if input.is_pressed(KeyCode::KeyA) || input.is_pressed(KeyCode::ArrowLeft) {
            axis.x -= 1.0;
        }
        if input.is_pressed(KeyCode::KeyD) || input.is_pressed(KeyCode::ArrowRight) {
            axis.x += 1.0;
        }
        if input.is_pressed(KeyCode::KeyW) || input.is_pressed(KeyCode::ArrowUp) {
            axis.y -= 1.0;
        }
        if input.is_pressed(KeyCode::KeyS) || input.is_pressed(KeyCode::ArrowDown) {
            axis.y += 1.0;
        }
        let interact = input.just_pressed(KeyCode::KeyE);
        let toggle_post = input.just_pressed(KeyCode::KeyP);
        let restart = input.just_pressed(KeyCode::KeyR);
        let quit = input.just_pressed(KeyCode::Escape);

        if quit {
            if let Some(q) = world.resource_mut::<ShouldQuit>() {
                q.0 = true;
            }
        }
        if toggle_post {
            let enabled = world
                .resource_mut::<DungeonSession>()
                .map(|s| {
                    s.post_enabled = !s.post_enabled;
                    s.post_enabled
                })
                .unwrap_or(true);
            if let Some(pp) = world.resource_mut::<PostProcessConfig>() {
                pp.enabled = enabled;
            }
        }
        if restart {
            reset(world);
            return;
        }

        let Some((player, status)) = world
            .resource::<DungeonSession>()
            .map(|s| (s.player, s.status))
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }

        // Move.
        let direction = if axis.length_squared() > 0.0 {
            axis.normalize()
        } else {
            Vec2::ZERO
        };
        let Some(current) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        let proposed = current + direction * PLAYER_SPEED * dt;
        let resolved = resolve_walls(world, current, proposed, PLAYER_HALF);
        if let Some(t) = world.get_mut::<Transform>(player) {
            t.position = resolved;
        }

        // Light the nearest unlit brazier in range.
        if interact {
            try_light_brazier(world, resolved);
        }
    }

    fn name(&self) -> &'static str {
        "PlayerSystem"
    }
}

fn try_light_brazier(world: &mut World, player_pos: Vec2) {
    // Snapshot brazier state to avoid holding the session borrow while mutating.
    let candidate = world.resource::<DungeonSession>().and_then(|s| {
        s.braziers
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.lit)
            .map(|(i, b)| (i, b.entity, (b.pos - player_pos).length()))
            .filter(|(_, _, d)| *d <= INTERACT_RADIUS)
            .min_by(|a, b| a.2.total_cmp(&b.2))
    });
    let Some((idx, entity, _)) = candidate else {
        return;
    };

    // Persistent brazier light + a bright sprite so bloom catches the glow.
    world.add_component(
        entity,
        PointLight {
            color: BRAZIER_COLOR,
            radius: 190.0,
            intensity: 1.45,
            light_height: 0.3,
        },
    );
    world.add_component(entity, Sprite::colored(1.0, 0.78, 0.45));

    if let Some(s) = world.resource_mut::<DungeonSession>() {
        s.braziers[idx].lit = true;
        s.fuel = MAX_FUEL;
    }
}

/// Drain torch fuel and drive the player `PointLight` from it. Empty = you lose.
struct TorchSystem;
impl System for TorchSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        let Some((player, status)) = world
            .resource::<DungeonSession>()
            .map(|s| (s.player, s.status))
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }

        let fuel = {
            let Some(s) = world.resource_mut::<DungeonSession>() else {
                return;
            };
            s.fuel = (s.fuel - dt).max(0.0);
            if s.fuel <= 0.0 {
                s.status = Status::Lost;
            }
            s.fuel
        };

        let (radius, intensity) = torch_from_fuel(fuel);
        if let Some(light) = world.get_mut::<PointLight>(player) {
            light.radius = radius;
            light.intensity = intensity;
        }
    }

    fn name(&self) -> &'static str {
        "TorchSystem"
    }
}

/// Center the camera on the player. Clamping to the level bounds is handled
/// automatically by the engine each frame via `Camera::bounds`.
struct CameraFollowSystem;
impl System for CameraFollowSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some(player) = world.resource::<DungeonSession>().map(|s| s.player) else {
            return;
        };
        let Some(pos) = world.get::<Transform>(player).map(|t| t.position) else {
            return;
        };
        if let Some(cam) = world.resource_mut::<Camera>() {
            let half = Vec2::new(WINDOW_W as f32, WINDOW_H as f32) / (2.0 * cam.zoom);
            // Set position to center on the player; Camera::bounds handles edge clamping.
            cam.position = pos - half;
        }
    }

    fn name(&self) -> &'static str {
        "CameraFollowSystem"
    }
}

/// Open the exit once every brazier is lit, and detect the win/standby states.
struct WinSystem;
impl System for WinSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let Some((player, exit_entity, exit_pos, all_lit, exit_lit, status)) =
            world.resource::<DungeonSession>().map(|s| {
                (
                    s.player,
                    s.exit_entity,
                    s.exit_pos,
                    s.lit_count() == s.braziers.len(),
                    s.exit_lit,
                    s.status,
                )
            })
        else {
            return;
        };
        if status != Status::Playing {
            return;
        }

        // Light the exit the moment the last brazier is lit (the 18th light source).
        if all_lit && !exit_lit {
            world.add_component(
                exit_entity,
                PointLight {
                    color: EXIT_COLOR,
                    radius: 210.0,
                    intensity: 1.6,
                    light_height: 0.35,
                },
            );
            world.add_component(exit_entity, Sprite::colored(0.6, 1.0, 0.8));
            if let Some(s) = world.resource_mut::<DungeonSession>() {
                s.exit_lit = true;
            }
        }

        if all_lit {
            if let Some(ppos) = world.get::<Transform>(player).map(|t| t.position) {
                if (ppos - exit_pos).length() <= EXIT_RADIUS + PLAYER_HALF {
                    if let Some(s) = world.resource_mut::<DungeonSession>() {
                        s.status = Status::Won;
                    }
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "WinSystem"
    }
}

/// HUD via `TextQueue` (drawn after the lighting pass, so it stays readable).
struct HudSystem {
    fps: f32,
}
impl System for HudSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        if dt > 0.0 {
            let inst = 1.0 / dt;
            self.fps = if self.fps <= 0.0 {
                inst
            } else {
                self.fps * 0.9 + inst * 0.1
            };
        }
        let Some((fuel, lit, total, status)) = world
            .resource::<DungeonSession>()
            .map(|s| (s.fuel, s.lit_count(), s.braziers.len(), s.status))
        else {
            return;
        };
        let Some(tq) = world.resource_mut::<TextQueue>() else {
            return;
        };

        let pct = (fuel / MAX_FUEL * 100.0).round() as i32;
        let filled = (fuel / MAX_FUEL * 16.0).round().clamp(0.0, 16.0) as usize;
        let bar: String = format!("{}{}", "#".repeat(filled), "-".repeat(16 - filled));
        let fuel_color = if pct <= 25 {
            [255, 130, 110, 255]
        } else {
            [255, 220, 140, 255]
        };
        tq.push(DrawText::new(
            format!("Torch [{bar}] {pct}%"),
            Vec2::new(10.0, 10.0),
            20.0,
            fuel_color,
        ));
        tq.push(DrawText::new(
            format!("Braziers lit: {lit} / {total}"),
            Vec2::new(10.0, 34.0),
            20.0,
            [200, 230, 255, 235],
        ));
        tq.push(DrawText::new(
            format!("FPS: {:.0}", self.fps),
            Vec2::new(WINDOW_W as f32 - 96.0, 10.0),
            18.0,
            [160, 220, 170, 220],
        ));
        tq.push(DrawText::new(
            "WASD move   E light brazier   P post-fx   R restart   Esc quit",
            Vec2::new(10.0, WINDOW_H as f32 - 26.0),
            16.0,
            [180, 200, 230, 200],
        ));

        let mid = Vec2::new(WINDOW_W as f32 * 0.5 - 180.0, WINDOW_H as f32 * 0.5 - 16.0);
        match status {
            Status::Playing if lit == total => tq.push(DrawText::new(
                "All braziers lit — reach the green exit!",
                mid,
                24.0,
                [150, 255, 200, 255],
            )),
            Status::Playing => {}
            Status::Won => tq.push(DrawText::new(
                "Escaped the dark! Press R to play again.",
                mid,
                26.0,
                [150, 255, 190, 255],
            )),
            Status::Lost => tq.push(DrawText::new(
                "Your torch went out. Press R to retry.",
                mid,
                26.0,
                [255, 140, 140, 255],
            )),
        }
    }

    fn name(&self) -> &'static str {
        "HudSystem"
    }
}

// ─── Reset ───────────────────────────────────────────────────────────────────

fn reset(world: &mut World) {
    let Some((player, spawn, brazier_entities)) = world.resource::<DungeonSession>().map(|s| {
        (
            s.player,
            s.player_spawn,
            s.braziers.iter().map(|b| b.entity).collect::<Vec<_>>(),
        )
    }) else {
        return;
    };

    if let Some(t) = world.get_mut::<Transform>(player) {
        t.position = spawn;
    }
    // Extinguish every brazier: drop its PointLight, dim the sprite.
    for entity in brazier_entities {
        world.remove_component::<PointLight>(entity);
        world.add_component(entity, Sprite::colored(0.55, 0.40, 0.28));
    }
    if let Some(s) = world.resource_mut::<DungeonSession>() {
        let exit_entity = s.exit_entity;
        for b in &mut s.braziers {
            b.lit = false;
        }
        s.exit_lit = false;
        s.fuel = MAX_FUEL;
        s.status = Status::Playing;
        world.remove_component::<PointLight>(exit_entity);
        world.add_component(exit_entity, Sprite::colored(0.30, 0.58, 0.42));
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

fn main() {
    let mut app = App::new();
    app.world.insert_resource(WindowConfig {
        title: "skeleton-engine lit dungeon".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.0, 0.0, 0.0, 1.0],
    });

    let map = parse_map();

    // Floor + wall sprites (walls also get a collider on WALL_LAYER).
    for (row, row_tiles) in map.tiles.iter().enumerate() {
        for (col, &tile) in row_tiles.iter().enumerate() {
            let center = tile_center(col, row);
            let entity = app.world.spawn();
            if tile == TILE_WALL {
                app.world.add_component(
                    entity,
                    Transform {
                        position: center,
                        scale: Vec2::splat(TILE),
                        rotation: 0.0,
                        z: 0.0,
                    },
                );
                app.world
                    .add_component(entity, Sprite::colored(0.60, 0.58, 0.70));
                app.world.add_component(
                    entity,
                    Collider::Aabb {
                        half_extents: Vec2::splat(TILE * 0.5),
                    },
                );
                app.world.add_component(entity, CollisionLayer(WALL_LAYER));
            } else {
                app.world.add_component(
                    entity,
                    Transform {
                        position: center,
                        scale: Vec2::splat(TILE),
                        rotation: 0.0,
                        z: -1.0,
                    },
                );
                app.world
                    .add_component(entity, Sprite::colored(0.42, 0.40, 0.50));
            }
        }
    }

    // Braziers (unlit).
    let mut braziers = Vec::new();
    for pos in &map.braziers {
        let entity = app.world.spawn();
        app.world.add_component(
            entity,
            Transform {
                position: *pos,
                scale: Vec2::splat(TILE * 0.4),
                rotation: 0.0,
                z: 0.5,
            },
        );
        app.world
            .add_component(entity, Sprite::colored(0.55, 0.40, 0.28));
        braziers.push(Brazier {
            entity,
            pos: *pos,
            lit: false,
        });
    }

    // Exit (dim until all braziers are lit).
    let exit_entity = app.world.spawn();
    app.world.add_component(
        exit_entity,
        Transform {
            position: map.exit,
            scale: Vec2::splat(TILE * 0.55),
            rotation: 0.0,
            z: 0.4,
        },
    );
    app.world
        .add_component(exit_entity, Sprite::colored(0.30, 0.58, 0.42));

    // Player + torch PointLight.
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: map.player,
            scale: Vec2::splat(TILE * 0.42),
            rotation: 0.0,
            z: 1.0,
        },
    );
    app.world
        .add_component(player, Sprite::colored(1.0, 0.92, 0.65));
    let (r0, i0) = torch_from_fuel(MAX_FUEL);
    app.world.add_component(
        player,
        PointLight {
            color: TORCH_COLOR,
            radius: r0,
            intensity: i0,
            light_height: 0.4,
        },
    );

    app.world.insert_resource(DungeonSession {
        player,
        player_spawn: map.player,
        braziers,
        exit_entity,
        exit_pos: map.exit,
        exit_lit: false,
        fuel: MAX_FUEL,
        status: Status::Playing,
        post_enabled: true,
    });

    // Dark blue ambient — presence of this resource enables the lighting pass.
    app.world.insert_resource(AmbientLight {
        color: Color::rgb(0.55, 0.6, 0.95),
        intensity: 0.08,
    });

    // Post-process preset: bloom on the torch/brazier glow + vignette tunnel-vision.
    app.world.insert_resource(PostProcessConfig {
        enabled: true,
        vignette_strength: 0.55,
        vignette_radius: 0.62,
        chroma_offset: 0.0015,
        bloom_threshold: 0.55,
        bloom_intensity: 0.8,
        brightness: 0.0,
        contrast: 1.05,
        saturation: 1.1,
        ..Default::default()
    });

    // Clamp the camera to the level rectangle so it never scrolls past the edges.
    let mut camera = Camera::new(Vec2::ZERO, 1.0);
    camera.bounds = Some((Vec2::ZERO, Vec2::new(LEVEL_W, LEVEL_H)));
    app.world.insert_resource(camera);

    // Order: rebuild grid → move/interact → drain torch → camera → win → HUD.
    app.add_system(CollisionGridSystem::new(TILE * 2.0));
    app.add_system(PlayerSystem);
    app.add_system(TorchSystem);
    app.add_system(CameraFollowSystem);
    app.add_system(WinSystem);
    app.add_system(HudSystem { fps: 0.0 });

    app.run();
}
