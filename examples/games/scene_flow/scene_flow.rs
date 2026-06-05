use engine::{
    Anchor, App, Button, ButtonState, Color, Entity, Events, GameState, InputState, KeyCode, Label,
    Scene, SceneChange, SceneCmd, ShouldQuit, System, TextAlign, UiEvent, UiImageQueue, UiNode,
    UiQueue, UiSystem, Vec2, ViewportSize, WindowConfig, World,
};
use engine::{DrawImage, DrawRect};

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 540;
const BG_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/games/scene_flow/assets/flow_bg.png"
);
const BADGE_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/examples/games/scene_flow/assets/flow_badge.png"
);

/// Cross-scene diagnostics. Previously this lived behind `Arc<Mutex<_>>` and was
/// cloned into every scene's constructor, because `SceneCmd::Replace` resets the
/// `World` and would otherwise drop it. It is now a plain resource preserved across
/// resets via `App::register_persistent::<StatsData>()` — no Arc, no Mutex, and
/// scenes no longer carry a handle.
#[derive(Default)]
struct StatsData {
    menu_enters: u32,
    menu_exits: u32,
    play_enters: u32,
    play_exits: u32,
    pause_enters: u32,
    pause_exits: u32,
    result_enters: u32,
    result_exits: u32,
    current_scene: &'static str,
    overlay: &'static str,
}

impl StatsData {
    fn mark_enter(&mut self, scene: &'static str) {
        match scene {
            "Menu" => {
                self.menu_enters += 1;
                self.current_scene = "Menu";
                self.overlay = "None";
            }
            "Play" => {
                self.play_enters += 1;
                self.current_scene = "Play";
                self.overlay = "None";
            }
            "Pause" => {
                self.pause_enters += 1;
                self.overlay = "Pause";
            }
            "Result" => {
                self.result_enters += 1;
                self.overlay = "Result";
            }
            _ => {}
        }
    }

    fn mark_exit(&mut self, scene: &'static str) {
        match scene {
            "Menu" => self.menu_exits += 1,
            "Play" => self.play_exits += 1,
            "Pause" => {
                self.pause_exits += 1;
                if self.overlay == "Pause" {
                    self.overlay = "None";
                }
            }
            "Result" => {
                self.result_exits += 1;
                if self.overlay == "Result" {
                    self.overlay = "None";
                }
            }
            _ => {}
        }
    }

    fn summary(&self) -> String {
        format!(
            "Scene: {} | Overlay: {}\nEnter  M:{} P:{} Pa:{} R:{}\nExit   M:{} P:{} Pa:{} R:{}",
            self.current_scene,
            self.overlay,
            self.menu_enters,
            self.play_enters,
            self.pause_enters,
            self.result_enters,
            self.menu_exits,
            self.play_exits,
            self.pause_exits,
            self.result_exits
        )
    }
}

/// Record a scene enter on the persistent `StatsData` resource.
fn mark_enter(world: &mut World, scene: &'static str) {
    if let Some(stats) = world.resource_mut::<StatsData>() {
        stats.mark_enter(scene);
    }
}

/// Record a scene exit on the persistent `StatsData` resource.
fn mark_exit(world: &mut World, scene: &'static str) {
    if let Some(stats) = world.resource_mut::<StatsData>() {
        stats.mark_exit(scene);
    }
}

fn clicked(world: &World) -> Vec<Entity> {
    world
        .resource::<Events<UiEvent>>()
        .map(|events| {
            events
                .read()
                .iter()
                .filter_map(|event| match event {
                    UiEvent::ButtonClicked(entity) => Some(*entity),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn key_pressed(world: &World, key: KeyCode) -> bool {
    world
        .resource::<InputState>()
        .map(|input| input.just_pressed(key))
        .unwrap_or(false)
}

fn add_label(
    world: &mut World,
    entities: &mut Vec<Entity>,
    text: impl Into<String>,
    offset: Vec2,
    size: Vec2,
    font_size: f32,
    color: [u8; 4],
) -> Entity {
    let entity = world.spawn();
    world.add_component(
        entity,
        UiNode::new(offset.x, offset.y, size.x, size.y)
            .with_anchor(Anchor::Center)
            .with_z(0.92),
    );
    world.add_component(
        entity,
        Label::new(text)
            .with_font_size(font_size)
            .with_color(color)
            .with_align(TextAlign::Center),
    );
    entities.push(entity);
    entity
}

fn add_button(
    world: &mut World,
    entities: &mut Vec<Entity>,
    label: impl Into<String>,
    offset: Vec2,
) -> Entity {
    let entity = world.spawn();
    world.add_component(
        entity,
        UiNode::new(offset.x, offset.y, 360.0, 68.0)
            .with_anchor(Anchor::Center)
            .with_z(0.94),
    );
    let mut button = Button::new(label);
    button.color_normal = Color::rgba(0.035, 0.13, 0.16, 0.98);
    button.color_hovered = Color::rgba(0.07, 0.30, 0.34, 1.0);
    button.color_pressed = Color::rgba(0.90, 0.62, 0.16, 1.0);
    button.text_color = Color::rgba_u8(255, 235, 170, 255);
    button.font_size = 28.0;
    world.add_component(entity, button);
    entities.push(entity);
    entity
}

fn set_ui_z(world: &mut World, entity: Entity, z: f32) {
    if let Some(node) = world.get_mut::<UiNode>(entity) {
        node.z = z;
    }
}

fn add_modal_scrim(world: &mut World, entities: &mut Vec<Entity>) {
    let entity = world.spawn();
    world.add_component(
        entity,
        UiNode::new(0.0, 0.0, WINDOW_W as f32, WINDOW_H as f32)
            .with_anchor(Anchor::TopLeft)
            .with_z(0.95),
    );
    let mut scrim = Button::new("");
    scrim.state = ButtonState::Disabled;
    scrim.color_disabled = Color::rgba(0.0, 0.018, 0.026, 0.96);
    world.add_component(entity, scrim);
    entities.push(entity);
}

fn despawn_all(world: &mut World, entities: &mut Vec<Entity>) {
    for entity in entities.drain(..) {
        world.despawn(entity);
    }
}

fn hide_existing_ui(world: &mut World) -> Vec<Entity> {
    let entities: Vec<Entity> = world.query::<UiNode>().map(|(entity, _)| entity).collect();
    for entity in &entities {
        if let Some(node) = world.get_mut::<UiNode>(*entity) {
            node.visible = false;
        }
    }
    entities
}

fn restore_ui(world: &mut World, entities: &mut Vec<Entity>) {
    for entity in entities.drain(..) {
        if let Some(node) = world.get_mut::<UiNode>(entity) {
            node.visible = true;
        }
    }
}

fn configure_window(world: &mut World) {
    world.insert_resource(WindowConfig {
        title: "skeleton-engine scene flow game".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.0, 0.018, 0.026, 1.0],
    });
}

struct MenuScene {
    entities: Vec<Entity>,
}

impl MenuScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Scene for MenuScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        configure_window(world);
        mark_enter(world, "Menu");
        world.insert_resource(GameState::Paused);

        add_label(
            world,
            &mut self.entities,
            "Scene Flow Game",
            Vec2::new(0.0, -100.0),
            Vec2::new(620.0, 52.0),
            42.0,
            [255, 220, 140, 255],
        );
        add_label(
            world,
            &mut self.entities,
            "Replace / Push / Pop validation",
            Vec2::new(0.0, -42.0),
            Vec2::new(660.0, 38.0),
            24.0,
            [190, 230, 220, 255],
        );
        let start = add_button(world, &mut self.entities, "Start", Vec2::new(0.0, 42.0));
        let quit = add_button(world, &mut self.entities, "Quit", Vec2::new(0.0, 124.0));
        let stats_label = add_label(
            world,
            &mut self.entities,
            "",
            Vec2::new(0.0, 220.0),
            Vec2::new(700.0, 90.0),
            18.0,
            [170, 225, 215, 255],
        );

        systems.push(Box::new(BackdropSystem));
        systems.push(Box::new(UiSystem));
        systems.push(Box::new(MenuSystem {
            start,
            quit,
            stats_label,
        }));
    }

    fn on_exit(&mut self, world: &mut World) {
        mark_exit(world, "Menu");
        despawn_all(world, &mut self.entities);
    }
}

struct MenuSystem {
    start: Entity,
    quit: Entity,
    stats_label: Entity,
}

impl System for MenuSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let clicks = clicked(world);
        let start = clicks.contains(&self.start) || key_pressed(world, KeyCode::Enter);
        let quit = clicks.contains(&self.quit) || key_pressed(world, KeyCode::Escape);

        update_stats_label(world, self.stats_label);

        if quit {
            if let Some(should_quit) = world.resource_mut::<ShouldQuit>() {
                should_quit.0 = true;
            }
        } else if start {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Replace(Box::new(PlayScene::new())));
            }
        }
    }
}

struct PlayScene {
    entities: Vec<Entity>,
}

impl PlayScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Scene for PlayScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        configure_window(world);
        mark_enter(world, "Play");
        world.insert_resource(GameState::Playing);

        add_label(
            world,
            &mut self.entities,
            "Play Scene",
            Vec2::new(0.0, -152.0),
            Vec2::new(560.0, 52.0),
            40.0,
            [255, 220, 140, 255],
        );
        add_label(
            world,
            &mut self.entities,
            "Complete mission or open pause",
            Vec2::new(0.0, -92.0),
            Vec2::new(660.0, 38.0),
            24.0,
            [190, 230, 220, 255],
        );
        let complete = add_button(
            world,
            &mut self.entities,
            "Complete Mission",
            Vec2::new(0.0, 0.0),
        );
        let pause = add_button(world, &mut self.entities, "Pause", Vec2::new(0.0, 84.0));
        let stats_label = add_label(
            world,
            &mut self.entities,
            "",
            Vec2::new(0.0, 220.0),
            Vec2::new(700.0, 90.0),
            18.0,
            [170, 225, 215, 255],
        );

        systems.push(Box::new(BackdropSystem));
        systems.push(Box::new(UiSystem));
        systems.push(Box::new(PlaySystem {
            complete,
            pause,
            stats_label,
        }));
    }

    fn on_exit(&mut self, world: &mut World) {
        mark_exit(world, "Play");
        despawn_all(world, &mut self.entities);
    }
}

struct PlaySystem {
    complete: Entity,
    pause: Entity,
    stats_label: Entity,
}

impl System for PlaySystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        update_stats_label(world, self.stats_label);

        let is_playing = world
            .resource::<GameState>()
            .map(|state| *state == GameState::Playing)
            .unwrap_or(false);
        if !is_playing {
            return;
        }

        let clicks = clicked(world);
        let complete = clicks.contains(&self.complete) || key_pressed(world, KeyCode::Enter);
        let pause = clicks.contains(&self.pause)
            || key_pressed(world, KeyCode::KeyP)
            || key_pressed(world, KeyCode::Escape);
        let menu = key_pressed(world, KeyCode::KeyM);

        if menu {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Replace(Box::new(MenuScene::new())));
            }
        } else if complete {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Push(Box::new(ResultScene::new())));
            }
        } else if pause {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Push(Box::new(PauseScene::new())));
            }
        }
    }
}

struct PauseScene {
    entities: Vec<Entity>,
    hidden_entities: Vec<Entity>,
}

impl PauseScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            hidden_entities: Vec::new(),
        }
    }
}

impl Scene for PauseScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        configure_window(world);
        mark_enter(world, "Pause");
        world.insert_resource(GameState::Paused);

        self.hidden_entities = hide_existing_ui(world);
        add_modal_scrim(world, &mut self.entities);

        let title = add_label(
            world,
            &mut self.entities,
            "Paused",
            Vec2::new(0.0, -112.0),
            Vec2::new(520.0, 54.0),
            44.0,
            [255, 205, 105, 255],
        );
        let resume = add_button(world, &mut self.entities, "Resume", Vec2::new(0.0, -20.0));
        let menu = add_button(world, &mut self.entities, "Menu", Vec2::new(0.0, 66.0));
        let stats_label = add_label(
            world,
            &mut self.entities,
            "",
            Vec2::new(0.0, 206.0),
            Vec2::new(700.0, 90.0),
            18.0,
            [210, 225, 200, 255],
        );
        set_ui_z(world, title, 0.96);
        set_ui_z(world, resume, 0.97);
        set_ui_z(world, menu, 0.97);
        set_ui_z(world, stats_label, 0.96);

        systems.push(Box::new(PauseSystem {
            resume,
            menu,
            stats_label,
        }));
    }

    fn on_exit(&mut self, world: &mut World) {
        mark_exit(world, "Pause");
        if let Some(state) = world.resource_mut::<GameState>() {
            *state = GameState::Playing;
        }
        restore_ui(world, &mut self.hidden_entities);
        despawn_all(world, &mut self.entities);
    }
}

struct PauseSystem {
    resume: Entity,
    menu: Entity,
    stats_label: Entity,
}

impl System for PauseSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        update_stats_label(world, self.stats_label);
        let clicks = clicked(world);
        let resume = clicks.contains(&self.resume)
            || key_pressed(world, KeyCode::KeyP)
            || key_pressed(world, KeyCode::Escape);
        let menu = clicks.contains(&self.menu) || key_pressed(world, KeyCode::KeyM);

        if menu {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Replace(Box::new(MenuScene::new())));
            }
        } else if resume {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Pop);
            }
        }
    }
}

struct ResultScene {
    entities: Vec<Entity>,
    hidden_entities: Vec<Entity>,
}

impl ResultScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            hidden_entities: Vec::new(),
        }
    }
}

impl Scene for ResultScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut Vec<Box<dyn System>>) {
        configure_window(world);
        mark_enter(world, "Result");
        world.insert_resource(GameState::GameOver);

        self.hidden_entities = hide_existing_ui(world);
        add_modal_scrim(world, &mut self.entities);
        let title = add_label(
            world,
            &mut self.entities,
            "Mission Complete",
            Vec2::new(0.0, -108.0),
            Vec2::new(620.0, 54.0),
            44.0,
            [255, 205, 105, 255],
        );
        let retry = add_button(world, &mut self.entities, "Retry", Vec2::new(0.0, -12.0));
        let menu = add_button(world, &mut self.entities, "Menu", Vec2::new(0.0, 74.0));
        let stats_label = add_label(
            world,
            &mut self.entities,
            "",
            Vec2::new(0.0, 216.0),
            Vec2::new(700.0, 90.0),
            18.0,
            [210, 225, 200, 255],
        );
        set_ui_z(world, title, 0.96);
        set_ui_z(world, retry, 0.97);
        set_ui_z(world, menu, 0.97);
        set_ui_z(world, stats_label, 0.96);

        systems.push(Box::new(ResultSystem {
            retry,
            menu,
            stats_label,
        }));
    }

    fn on_exit(&mut self, world: &mut World) {
        mark_exit(world, "Result");
        restore_ui(world, &mut self.hidden_entities);
        despawn_all(world, &mut self.entities);
    }
}

struct ResultSystem {
    retry: Entity,
    menu: Entity,
    stats_label: Entity,
}

impl System for ResultSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        update_stats_label(world, self.stats_label);
        let clicks = clicked(world);
        let retry = clicks.contains(&self.retry) || key_pressed(world, KeyCode::KeyR);
        let menu = clicks.contains(&self.menu)
            || key_pressed(world, KeyCode::Escape)
            || key_pressed(world, KeyCode::KeyM);

        if retry {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Replace(Box::new(PlayScene::new())));
            }
        } else if menu {
            if let Some(scene_change) = world.resource_mut::<SceneChange>() {
                scene_change.request(SceneCmd::Replace(Box::new(MenuScene::new())));
            }
        }
    }
}

fn update_stats_label(world: &mut World, entity: Entity) {
    let text = world
        .resource::<StatsData>()
        .map(StatsData::summary)
        .unwrap_or_else(|| "Stats unavailable".to_string());

    if let Some(label) = world.get_mut::<Label>(entity) {
        label.text = text;
    }
}

struct BackdropSystem;

impl System for BackdropSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let viewport = world
            .resource::<ViewportSize>()
            .copied()
            .unwrap_or_else(|| ViewportSize::new(WINDOW_W, WINDOW_H));
        let w = viewport.width.max(WINDOW_W as f32);
        let h = viewport.height.max(WINDOW_H as f32);

        if let Some(images) = world.resource_mut::<UiImageQueue>() {
            images.push(DrawImage::textured(0.0, 0.0, w, h, BG_PATH).with_z(0.01));
            images.push(DrawImage::textured(28.0, 28.0, 112.0, 112.0, BADGE_PATH).with_z(0.03));
        }

        if let Some(rects) = world.resource_mut::<UiQueue>() {
            rects.push(DrawRect::new(0.0, 0.0, w, h, [0.0, 0.025, 0.035, 0.72]).with_z(0.50));
        }
    }
}

fn main() {
    let mut app = App::new();
    app.register_event::<UiEvent>();

    // Cross-scene diagnostics live as a plain resource preserved across the World
    // resets that `SceneCmd::Replace` triggers — no Arc<Mutex>, no per-scene handle.
    app.world.insert_resource(StatsData::default());
    app.register_persistent::<StatsData>();

    app.set_scene(Box::new(MenuScene::new()));
    configure_window(&mut app.world);
    app.load_image(BG_PATH);
    app.load_image(BADGE_PATH);
    app.run();
}
