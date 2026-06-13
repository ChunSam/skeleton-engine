//! ui_layout_editor_game — editor persistence acceptance example.
//!
//! Proves the full vision loop: arrange a menu/HUD in the docked editor (F2),
//! click "Save Scene", restart the example, and the edited layout persists.
//!
//! # Workflow
//!   1. Run the example — you see a default menu (title + 3 buttons + volume slider).
//!   2. Press F2 for the docked editor. Drag/resize widgets in the viewport. Click
//!      "Save Scene" in the editor toolbar (saves to `saved_scene.ron` in the CWD).
//!   3. Quit (Esc) and rerun. The example loads `saved_scene.ron` and restores your layout.
//!
//! The load-or-default pattern: if `saved_scene.ron` doesn't exist (first run), or is
//! empty, the hardcoded default layout is spawned instead.

use std::path::Path;

use engine::{
    spawn_scene_def, Anchor, App, Button, CheckBox, Color, Entity, Events, GameState, Label,
    LayoutDir, LayoutSystem, Panel, Scene, SceneDef, ShouldQuit, Slider, System, SystemConfig,
    SystemRegistrar, TextAlign, UiEvent, UiNode, UiSystem, WindowConfig, World,
};

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 600;
/// Must match `EditorState::editor_save_path` default in `src/app/editor/state.rs`.
const SAVE_FILE: &str = "saved_scene.ron";

fn configure_window(world: &mut World) {
    world.insert_resource(WindowConfig {
        title: "ui_layout_editor_game — editor persistence demo".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.04, 0.05, 0.08, 1.0],
    });
}

fn despawn_all(world: &mut World, entities: &mut Vec<Entity>) {
    for e in entities.drain(..) {
        world.despawn(e);
    }
}

fn ui_events(world: &World) -> Vec<UiEvent> {
    world
        .resource::<Events<UiEvent>>()
        .map(|ev| ev.read().to_vec())
        .unwrap_or_default()
}

// ── Default layout helpers ────────────────────────────────────────────────────

fn spawn_default_layout(world: &mut World, entities: &mut Vec<Entity>) {
    // Title label — centered near the top.
    {
        let e = world.spawn();
        world.add_component(
            e,
            UiNode::new(0.0, -200.0, 600.0, 52.0)
                .with_anchor(Anchor::Center)
                .with_z(0.92),
        );
        world.add_component(
            e,
            Label::new("My Game Menu")
                .with_font_size(40.0)
                .with_color([255, 220, 140, 255])
                .with_align(TextAlign::Center),
        );
        entities.push(e);
    }

    // "Start" button.
    spawn_menu_button(world, entities, "Start", -80.0);
    // "Options" button.
    spawn_menu_button(world, entities, "Options", 0.0);
    // "Quit" button — handled by the interaction system.
    spawn_menu_button(world, entities, "Quit", 80.0);

    // Volume slider inside a small panel.
    let vol_label = {
        let e = world.spawn();
        world.add_component(e, UiNode::new(0.0, 0.0, 260.0, 22.0));
        world.add_component(
            e,
            Label::new("Volume")
                .with_font_size(17.0)
                .with_color([190, 210, 220, 255]),
        );
        entities.push(e);
        e
    };
    let vol_slider = {
        let e = world.spawn();
        world.add_component(e, UiNode::new(0.0, 0.0, 260.0, 20.0));
        world.add_component(e, Slider::new(0.0, 1.0, 0.7));
        entities.push(e);
        e
    };
    // Mute checkbox.
    let mute_cb = {
        let e = world.spawn();
        world.add_component(e, UiNode::new(0.0, 0.0, 260.0, 26.0));
        world.add_component(e, CheckBox::new("Mute"));
        entities.push(e);
        e
    };

    // Panel groups the volume controls, anchored bottom-center.
    let panel = world.spawn();
    world.add_component(
        panel,
        UiNode::new(0.0, -20.0, 300.0, 120.0)
            .with_anchor(Anchor::BottomCenter)
            .with_z(0.80),
    );
    let mut p = Panel::new(LayoutDir::Vertical)
        .with_gap(8.0)
        .with_padding(14.0);
    p.background_color = Color::rgba(0.06, 0.08, 0.14, 0.92);
    p.children = vec![vol_label, vol_slider, mute_cb];
    world.add_component(panel, p);
    entities.push(panel);
}

fn spawn_menu_button(world: &mut World, entities: &mut Vec<Entity>, label: &str, y_offset: f32) {
    let e = world.spawn();
    world.add_component(
        e,
        UiNode::new(0.0, y_offset, 280.0, 58.0)
            .with_anchor(Anchor::Center)
            .with_z(0.94),
    );
    let mut btn = Button::new(label);
    btn.color_normal = Color::rgba(0.06, 0.12, 0.20, 0.98);
    btn.color_hovered = Color::rgba(0.12, 0.28, 0.40, 1.0);
    btn.color_pressed = Color::rgba(0.88, 0.60, 0.14, 1.0);
    btn.text_color = Color::rgba_u8(240, 235, 220, 255);
    btn.font_size = 24.0;
    world.add_component(e, btn);
    entities.push(e);
}

// ── Scene ─────────────────────────────────────────────────────────────────────

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
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        configure_window(world);
        world.insert_resource(GameState::Paused);

        // Load-or-default: try the editor's saved scene; fall back to hardcoded layout.
        let loaded = SceneDef::load(Path::new(SAVE_FILE))
            .ok()
            .filter(|s| !s.entities.is_empty());

        if let Some(scene) = loaded {
            log::info!("[ui_layout_editor] Loaded saved layout from '{SAVE_FILE}'.");
            let spawned = spawn_scene_def(world, &scene);
            self.entities.extend(spawned);
        } else {
            log::info!("[ui_layout_editor] No saved layout found — using default.");
            spawn_default_layout(world, &mut self.entities);
        }

        // LayoutSystem BEFORE UiSystem (required — Panel geometry drives UiSystem hit-test).
        systems.add(LayoutSystem);
        systems.add_labeled(UiSystem, SystemConfig::new().after(LayoutSystem::LABEL));
        systems.add(InteractionSystem);
    }

    fn on_exit(&mut self, world: &mut World) {
        despawn_all(world, &mut self.entities);
    }
}

// ── Interaction system ────────────────────────────────────────────────────────

struct InteractionSystem;

impl System for InteractionSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        for ev in ui_events(world) {
            match ev {
                UiEvent::ButtonClicked(e) => {
                    // Resolve the button label for logging — no named routing needed.
                    let label = world
                        .get_mut::<Button>(e)
                        .map(|b| b.label.clone())
                        .unwrap_or_else(|| format!("entity {:?}", e));
                    log::info!("[ui_layout_editor] Button clicked: '{label}'");
                    println!("[ui_layout_editor] Button clicked: '{label}'");

                    if label == "Quit" {
                        if let Some(sq) = world.resource_mut::<ShouldQuit>() {
                            sq.quit();
                        }
                    }
                }
                UiEvent::SliderChanged(e, v) => {
                    log::info!("[ui_layout_editor] Slider {e:?} changed: {v:.2}");
                }
                UiEvent::CheckBoxToggled(e, checked) => {
                    log::info!("[ui_layout_editor] CheckBox {e:?} toggled: {checked}");
                }
                _ => {}
            }
        }
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = env_logger::try_init();

    println!();
    println!("=== ui_layout_editor_game ===");
    println!("Persistence demo for the skeleton-engine docked editor.");
    println!();
    println!("  F2            — open docked editor (drag/resize widgets)");
    println!("  Editor toolbar → 'Save Scene' — saves layout to '{SAVE_FILE}' in CWD");
    println!("  Restart       — the saved layout is automatically restored");
    println!("  F1            — overlay inspector (Inspector panel)");
    println!("  Esc / 'Quit'  — close the window");
    println!();

    let mut app = App::new();
    app.register_event::<UiEvent>();
    app.set_scene(Box::new(MenuScene::new()));
    configure_window(&mut app.world);
    app.run();
}
