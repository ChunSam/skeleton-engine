//! Minimap example (Phase 46): offscreen `RenderTarget`.
//!
//! Renders the world normally with the main camera, and an `OffscreenCamera`
//! builds a zoomed-out 256x256 minimap shown in the top-right corner.
use engine::{
    App, Camera, Color, DrawText, Entity, KeyCode, OffscreenCamera, RenderLayer, Sprite, System,
    TextQueue, Transform, ViewportSize, WindowConfig, World,
};
use glam::Vec2;

// ─── 시스템: 플레이어 이동 ────────────────────────────────────────────────────
struct MoveSystem;

impl System for MoveSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let speed = 150.0;
        let mut dx = 0.0f32;
        let mut dy = 0.0f32;

        {
            if let Some(input) = world.resource::<engine::InputState>() {
                if input.is_pressed(KeyCode::ArrowLeft) || input.is_pressed(KeyCode::KeyA) {
                    dx -= 1.0;
                }
                if input.is_pressed(KeyCode::ArrowRight) || input.is_pressed(KeyCode::KeyD) {
                    dx += 1.0;
                }
                if input.is_pressed(KeyCode::ArrowUp) || input.is_pressed(KeyCode::KeyW) {
                    dy += 1.0;
                }
                if input.is_pressed(KeyCode::ArrowDown) || input.is_pressed(KeyCode::KeyS) {
                    dy -= 1.0;
                }
            }
        }

        let entities: Vec<Entity> = world.query::<PlayerTag>().map(|(e, _)| e).collect();
        let dt = _dt;
        for e in &entities {
            if let Some(t) = world.get_mut::<Transform>(*e) {
                t.position.x += dx * speed * dt;
                t.position.y += dy * speed * dt;
            }
        }

        // 메인 카메라를 플레이어 위치에 추적
        let player_pos = entities
            .first()
            .and_then(|&e| world.get::<Transform>(e))
            .map(|t| t.position);
        if let (Some(pos), Some(cam)) = (player_pos, world.resource_mut::<Camera>()) {
            cam.position = pos;
        }
    }

    fn name(&self) -> &'static str {
        "MoveSystem"
    }
}

// ─── 태그 컴포넌트 ───────────────────────────────────────────────────────────
#[derive(Clone)]
struct PlayerTag;

/// 적 위에 떠 있는 월드 고정 라벨 마커.
#[derive(Clone)]
struct EnemyTag;

// ─── 시스템: 월드 엔티티 위에 화면 라벨 띄우기 ───────────────────────────────
/// Demonstrates [`Camera::world_to_screen`] + [`DrawText::centered`]: a nameplate
/// is anchored to each enemy's *world* position, projected to screen pixels each
/// frame so it tracks the enemy as the camera follows the player.
struct WorldLabelSystem;

impl System for WorldLabelSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // 카메라 스냅샷 (불변 차용을 끊고 TextQueue 를 가변 차용하기 위해 먼저 수집).
        let Some(camera) = world.resource::<Camera>().copied() else {
            return;
        };
        // 각 적의 월드 위치에서 살짝 위(스크린 기준)로 라벨을 띄운다 (Y 아래가 +).
        let enemies: Vec<Entity> = world.query::<EnemyTag>().map(|(e, _)| e).collect();
        let labels: Vec<Vec2> = enemies
            .iter()
            .filter_map(|&e| world.get::<Transform>(e))
            .map(|t| camera.world_to_screen(t.position + Vec2::new(0.0, -28.0)))
            .collect();

        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for screen_pos in labels {
                tq.push(DrawText::centered(
                    "ENEMY",
                    screen_pos,
                    14.0,
                    [255, 180, 180, 230],
                ));
            }
        }
    }

    fn name(&self) -> &'static str {
        "WorldLabelSystem"
    }
}

/// 미니맵 표시 스프라이트 마커 — HUD 시스템이 매 프레임 위치를 갱신한다.
#[derive(Clone)]
struct MinimapTag;

// ─── 시스템: 미니맵 HUD 위치 고정 ────────────────────────────────────────────
struct MinimapHudSystem;

impl System for MinimapHudSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // 메인 카메라는 top-left 앵커이며 매 프레임 플레이어를 따라간다. 미니맵
        // 스프라이트를 카메라 가시 영역의 우상단 코너에 고정한다(월드 고정 좌표면
        // 카메라가 움직일 때 화면 밖으로 흘러가던 기존 버그를 해결).
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((800.0, 600.0));
        let Some((min, max)) = world
            .resource::<Camera>()
            .map(|cam| cam.visible_rect(vw, vh))
        else {
            return;
        };
        let inset = 90.0 + 16.0; // 스프라이트 반 크기 + 여백 (zoom=1 → 월드=픽셀)
        let target = Vec2::new(max.x - inset, min.y + inset);
        let entities: Vec<Entity> = world.query::<MinimapTag>().map(|(e, _)| e).collect();
        for e in entities {
            if let Some(t) = world.get_mut::<Transform>(e) {
                t.position = target;
            }
        }
    }

    fn name(&self) -> &'static str {
        "MinimapHudSystem"
    }
}

fn main() {
    let mut app = App::new();

    // 윈도우 설정
    app.world.insert_resource(WindowConfig {
        title: "Phase 46 — Minimap".into(),
        width: 800,
        height: 600,
        clear_color: [0.08, 0.10, 0.15, 1.0],
    });

    // ─── 오프스크린 렌더 타겟 등록 (미니맵 256×256) ─────────────────────────
    app.create_render_target("minimap", 256, 256);

    // ─── 플레이어 (녹색 박스) ────────────────────────────────────────────────
    let player = app.world.spawn();
    app.world.add_component(
        player,
        Transform {
            position: Vec2::new(0.0, 0.0),
            scale: Vec2::new(40.0, 40.0),
            ..Default::default()
        },
    );
    app.world.add_component(
        player,
        Sprite {
            color: Color::rgba(0.2, 0.9, 0.3, 1.0),
            ..Default::default()
        },
    );
    app.world.add_component(player, PlayerTag);

    // ─── 적들 (빨간 박스) ────────────────────────────────────────────────────
    let enemy_positions = [
        Vec2::new(200.0, 100.0),
        Vec2::new(-150.0, 200.0),
        Vec2::new(300.0, -100.0),
        Vec2::new(-250.0, -180.0),
        Vec2::new(120.0, 280.0),
    ];
    for pos in enemy_positions {
        let e = app.world.spawn();
        app.world.add_component(
            e,
            Transform {
                position: pos,
                scale: Vec2::new(30.0, 30.0),
                ..Default::default()
            },
        );
        app.world.add_component(
            e,
            Sprite {
                color: Color::rgba(0.9, 0.2, 0.2, 1.0),
                ..Default::default()
            },
        );
        app.world.add_component(e, EnemyTag);
    }

    // ─── 배경 타일들 (회색 박스들) ────────────────────────────────────────────
    for i in -5..=5 {
        for j in -5..=5 {
            let bg = app.world.spawn();
            app.world.add_component(
                bg,
                Transform {
                    position: Vec2::new(i as f32 * 80.0, j as f32 * 80.0),
                    scale: Vec2::new(70.0, 70.0),
                    z: -1.0,
                    ..Default::default()
                },
            );
            let shade = if (i + j) % 2 == 0 { 0.15 } else { 0.20 };
            app.world.add_component(
                bg,
                Sprite {
                    color: Color::rgba(shade, shade, shade, 1.0),
                    ..Default::default()
                },
            );
            app.world.add_component(bg, RenderLayer(-1));
        }
    }

    // ─── OffscreenCamera 엔티티 (미니맵용, 줌 아웃) ──────────────────────────
    let oc_entity = app.world.spawn();
    app.world.add_component(
        oc_entity,
        OffscreenCamera {
            target: "minimap".to_string(),
            camera: Camera::new(Vec2::ZERO, 0.15),
            layer_mask: 1 << 0, // 게임 월드(layer 0)만 — 미니맵 표시용 스프라이트(layer 1) 제외
        },
    );

    // ─── 미니맵 표시용 스프라이트 (화면 우상단 고정) ──────────────────────────
    // 메인 카메라가 플레이어를 따라 움직이므로 월드 고정 좌표로 두면 미니맵이
    // 화면 밖으로 흘러간다(기존 버그). MinimapHudSystem 이 매 프레임 카메라
    // 가시 영역의 우상단으로 위치를 갱신해 항상 코너에 고정한다.
    let minimap_sprite = app.world.spawn();
    app.world.add_component(
        minimap_sprite,
        Transform {
            position: Vec2::new(694.0, 106.0), // 1프레임째 HUD 시스템이 갱신
            scale: Vec2::new(180.0, 180.0),
            z: 100.0, // 최상위 레이어
            ..Default::default()
        },
    );
    app.world.add_component(
        minimap_sprite,
        Sprite {
            texture: Some("minimap".to_string()), // RT 키
            color: Color::rgba(1.0, 1.0, 1.0, 0.9),
            ..Default::default()
        },
    );
    app.world.add_component(minimap_sprite, RenderLayer(10));
    app.world.add_component(minimap_sprite, MinimapTag);

    app.add_system(MoveSystem);
    app.add_system(MinimapHudSystem);
    app.add_system(WorldLabelSystem);
    app.run();
}
