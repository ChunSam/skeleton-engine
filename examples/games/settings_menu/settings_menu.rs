//! settings_menu_game — UI depth + localization + audio-bus dogfooding example.
//!
//! Every game needs a settings screen and a dialogue box, yet none of `TextInput`,
//! `Slider`, `CheckBox`, `ScrollView`, `Panel`/`LayoutSystem`, `LocaleResource`, or the
//! cross-platform `Audio` facade's low-pass filter had playable-game coverage. This 3-scene
//! slice (Title → Settings → Dialogue) drives all of them in real play — on native **and** web
//! (all audio goes through the `Audio` facade, so the example carries no audio `cfg` guards).
//!
//! It also exercises the engine fix this example surfaced: `LocalizedText` +
//! `LocalizationSystem`. Each static label/button/checkbox carries a translation
//! key; clicking a language button calls `LocaleResource::set_locale` and the whole
//! UI retranslates next frame with no manual rebuild.
//!
//! Cross-scene state (`Settings` and the locale) is kept across the `SceneCmd::Replace`
//! world reset via `App::register_persistent`. `Audio` needs no such call — the engine
//! registers it in `App::new` (v0.141.1).
//!
//! - `SETTINGS_MENU_SELFTEST=1 cargo run --example settings_menu_game` — asserts that the state
//!   above really crosses the world reset, that the rebuilt UI reads it, and that a resource
//!   which was *not* registered is dropped. A screenshot cannot: every read site is
//!   `unwrap_or_default()`, so a dropped resource renders a plausible screen rather than an error.

use engine::{
    Anchor, App, Audio, Button, CheckBox, Color, Entity, Events, GameState, ImeConfig, InputState,
    KeyCode, Label, LayoutDir, LayoutSystem, LocaleResource, LocalizationSystem, LocalizedText,
    Panel, Scene, SceneChange, SceneCmd, ScrollView, ShouldQuit, Slider, System, SystemConfig,
    SystemRegistrar, TextAlign, TextInput, UiEvent, UiNode, UiSystem, WindowConfig, World,
};

const WINDOW_W: u32 = 960;
const WINDOW_H: u32 = 600;

// Background music = a low tone + a high tone played together. The muffle low-pass
// cutoff sits between them, so toggling it clearly removes the bright high tone while
// the low tone stays — a faithful, audible low-pass demo (not a mute).
const BGM_LOW: f32 = 196.0;
const BGM_HIGH: f32 = 1568.0;
const MUFFLE_HZ: u32 = 400;

const LOCALES_RON: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: {
            "app.title": "[color=#ffd27f][b]Skeleton[/b][/color] Settings Demo",
            "app.subtitle": "UI  ·  localization  ·  audio buses",
            "menu.start": "Start",
            "menu.quit": "Quit",
            "menu.back": "Back",
            "menu.continue": "Continue",
            "settings.title": "Settings",
            "settings.name": "Player name",
            "settings.music": "Music volume",
            "settings.sfx": "SFX volume",
            "opt.subtitles": "Subtitles",
            "opt.fullscreen": "Fullscreen (preference)",
            "about.head": "How to play",
            "about.1": "Drag the sliders to set volume",
            "about.2": "Toggle the checkboxes",
            "about.3": "Type a player name",
            "about.4": "Click a language to switch instantly",
            "about.5": "Scroll this list with the wheel",
            "about.6": "Arrow keys / Home / End move the text cursor",
            "about.7": "Continue starts a short dialogue",
            "about.8": "Press M in the dialogue to muffle the music",
            "about.9": "Esc steps back to the title",
            "npc.name": "Guide",
            "npc.greet": "Welcome",
            "npc.line2": "Change the settings anytime from the menu.",
            "dialogue.off": "[ subtitles are off ]",
            "dialogue.help": "Space: next    M: muffle music (low-pass)    Esc: menu",
        } ),
        "ko": ( translations: {
            "app.title": "[color=#ffd27f][b]스켈레톤[/b][/color] 설정 데모",
            "app.subtitle": "UI  ·  현지화  ·  오디오 버스",
            "menu.start": "시작",
            "menu.quit": "종료",
            "menu.back": "뒤로",
            "menu.continue": "계속",
            "settings.title": "설정",
            "settings.name": "플레이어 이름",
            "settings.music": "음악 볼륨",
            "settings.sfx": "효과음 볼륨",
            "opt.subtitles": "자막",
            "opt.fullscreen": "전체 화면 (설정값)",
            "about.head": "사용법",
            "about.1": "슬라이더를 드래그해 볼륨 조절",
            "about.2": "체크박스를 토글",
            "about.3": "플레이어 이름 입력",
            "about.4": "언어를 클릭하면 즉시 전환",
            "about.5": "휠로 이 목록을 스크롤",
            "about.6": "방향키 / Home / End 로 텍스트 커서 이동",
            "about.7": "계속을 누르면 짧은 대화 시작",
            "about.8": "대화에서 M 키로 음악을 먹먹하게 (로우패스)",
            "about.9": "Esc 로 타이틀로 돌아가기",
            "npc.name": "안내자",
            "npc.greet": "환영합니다",
            "npc.line2": "메뉴에서 언제든 설정을 바꿀 수 있어요.",
            "dialogue.off": "[ 자막이 꺼져 있습니다 ]",
            "dialogue.help": "Space: 다음    M: 먹먹하게(로우패스)    Esc: 메뉴",
        } ),
        "es": ( translations: {
            "app.title": "[color=#ffd27f][b]Skeleton[/b][/color] Demo de ajustes",
            "app.subtitle": "UI  ·  localización  ·  buses de audio",
            "menu.start": "Empezar",
            "menu.quit": "Salir",
            "menu.back": "Atrás",
            "menu.continue": "Continuar",
            "settings.title": "Ajustes",
            "settings.name": "Nombre",
            "settings.music": "Volumen de música",
            "settings.sfx": "Volumen de efectos",
            "opt.subtitles": "Subtítulos",
            "opt.fullscreen": "Pantalla completa (preferencia)",
            "about.head": "Cómo jugar",
            "about.1": "Arrastra los deslizadores para el volumen",
            "about.2": "Activa las casillas",
            "about.3": "Escribe un nombre",
            "about.4": "Haz clic en un idioma para cambiar",
            "about.5": "Desplaza esta lista con la rueda",
            "about.6": "Flechas / Inicio / Fin mueven el cursor de texto",
            "about.7": "Continuar inicia un breve diálogo",
            "about.8": "Pulsa M en el diálogo para amortiguar la música",
            "about.9": "Esc vuelve al título",
            "npc.name": "Guía",
            "npc.greet": "Bienvenido",
            "npc.line2": "Cambia los ajustes cuando quieras desde el menú.",
            "dialogue.off": "[ subtítulos desactivados ]",
            "dialogue.help": "Espacio: siguiente    M: amortiguar (paso bajo)    Esc: menú",
        } ),
    },
)
"#;

/// Cross-scene state kept alive across `SceneCmd::Replace` via `register_persistent`.
#[derive(Clone)]
struct Settings {
    name: String,
    music: f32,
    sfx: f32,
    subtitles: bool,
    fullscreen: bool,
    locale: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            name: "Hero".to_string(),
            music: 0.6,
            sfx: 0.6,
            subtitles: true,
            fullscreen: false,
            locale: "en".to_string(),
        }
    }
}

// ── small helpers ────────────────────────────────────────────────────────────

fn ui_events(world: &World) -> Vec<UiEvent> {
    world
        .resource::<Events<UiEvent>>()
        .map(|events| events.read().to_vec())
        .unwrap_or_default()
}

fn key_pressed(world: &World, key: KeyCode) -> bool {
    world
        .resource::<InputState>()
        .map(|input| input.just_pressed(key))
        .unwrap_or(false)
}

fn t(world: &World, key: &str) -> String {
    world
        .resource::<LocaleResource>()
        .map(|l| l.t(key).to_string())
        .unwrap_or_else(|| key.to_string())
}

fn despawn_all(world: &mut World, entities: &mut Vec<Entity>) {
    for entity in entities.drain(..) {
        world.despawn(entity);
    }
}

fn request(world: &mut World, cmd: SceneCmd) {
    if let Some(change) = world.resource_mut::<SceneChange>() {
        change.request(cmd);
    }
}

fn configure_window(world: &mut World) {
    world.insert_resource(WindowConfig {
        title: "skeleton-engine settings menu".to_string(),
        width: WINDOW_W,
        height: WINDOW_H,
        clear_color: [0.02, 0.03, 0.05, 1.0],
    });
    // This example has TextInput + a dialogue box, so it needs IME for CJK composition.
    // IME defaults to off (so game keys aren't swallowed by a CJK input source); opt in here.
    world.insert_resource(ImeConfig { allowed: true });
}

/// Static label whose text is kept in sync with the current locale by
/// `LocalizationSystem` (the engine fix this example validates).
#[allow(clippy::too_many_arguments)]
fn spawn_loc_label(
    world: &mut World,
    entities: &mut Vec<Entity>,
    key: &str,
    node: UiNode,
    font_size: f32,
    color: [u8; 4],
    align: TextAlign,
    rich: bool,
) -> Entity {
    let entity = world.spawn();
    world.add_component(entity, node);
    let mut label = Label::new("")
        .with_font_size(font_size)
        .with_color(color)
        .with_align(align);
    if rich {
        label = label.rich();
    }
    world.add_component(entity, label);
    world.add_component(entity, LocalizedText::new(key));
    entities.push(entity);
    entity
}

fn spawn_loc_button(
    world: &mut World,
    entities: &mut Vec<Entity>,
    key: &str,
    node: UiNode,
) -> Entity {
    let entity = world.spawn();
    world.add_component(entity, node);
    let mut button = Button::new("");
    button.color_normal = Color::rgba(0.06, 0.12, 0.18, 1.0);
    button.color_hovered = Color::rgba(0.10, 0.26, 0.34, 1.0);
    button.color_pressed = Color::rgba(0.90, 0.62, 0.16, 1.0);
    button.text_color = Color::rgba_u8(235, 240, 245, 255);
    button.font_size = 20.0;
    world.add_component(entity, button);
    world.add_component(entity, LocalizedText::new(key));
    entities.push(entity);
    entity
}

fn spawn_plain_button(
    world: &mut World,
    entities: &mut Vec<Entity>,
    label: &str,
    node: UiNode,
) -> Entity {
    let entity = world.spawn();
    world.add_component(entity, node);
    let mut button = Button::new(label);
    button.color_normal = Color::rgba(0.10, 0.10, 0.16, 1.0);
    button.color_hovered = Color::rgba(0.20, 0.22, 0.30, 1.0);
    button.color_pressed = Color::rgba(0.32, 0.40, 0.60, 1.0);
    button.font_size = 18.0;
    world.add_component(entity, button);
    entities.push(entity);
    entity
}

fn about_items(world: &World) -> Vec<String> {
    [
        "about.1", "about.2", "about.3", "about.4", "about.5", "about.6", "about.7", "about.8",
        "about.9",
    ]
    .iter()
    .map(|k| format!("•  {}", t(world, k)))
    .collect()
}

// ── audio (cross-platform via the Audio facade — native + web, no cfg guards) ──

fn set_bus(world: &mut World, bus: &str, vol: f32) {
    if let Some(audio) = world.resource_mut::<Audio>() {
        audio.set_bus_volume(bus, vol);
    }
}

fn blip(world: &mut World, freq: f32) {
    if let Some(audio) = world.resource_mut::<Audio>() {
        // A fire-and-forget UI tone on the "sfx" bus (round-robins an anonymous voice).
        audio.play_tone_on_bus(freq, 0.05, 0.4, "sfx");
    }
}

fn play_bgm(world: &mut World) {
    if let Some(audio) = world.resource_mut::<Audio>() {
        // Sustained, *named* tone channels — so they can be queried (keep_bgm) and filtered
        // (set_muffle). Both ride the "music" bus, so the Music slider scales them live.
        audio.play_tone_on_channel("bgm_low", BGM_LOW, 1.2, 0.4, "music");
        audio.play_tone_on_channel("bgm_high", BGM_HIGH, 1.2, 0.25, "music");
    }
}

/// Keep the sustained two-tone music alive so the Music slider stays audible: replay
/// both tones whenever they drain. Dragging the slider changes the music bus volume,
/// which changes the loudness of these sustained tones in real time.
fn keep_bgm(world: &mut World) {
    let playing = world
        .resource::<Audio>()
        .map(|a| a.is_channel_playing("bgm_low"))
        .unwrap_or(true);
    if !playing {
        play_bgm(world);
    }
}

/// Toggle a low-pass filter on both music channels, then replay so it applies. The cutoff
/// sits between the low and high tones, so muffling removes the bright high tone and keeps
/// the low one — an audible low-pass (not a mute). Validates the facade's `set_low_pass`
/// (native `AudioEffect.low_pass_hz` / web `BiquadFilterNode`).
fn set_muffle(world: &mut World, on: bool) {
    if let Some(audio) = world.resource_mut::<Audio>() {
        for ch in ["bgm_low", "bgm_high"] {
            if on {
                audio.set_low_pass(ch, MUFFLE_HZ);
            } else {
                audio.clear_low_pass(ch);
            }
        }
    }
    play_bgm(world);
}

// ── Animated indicator (window-drag freeze probe) ─────────────────────────────
//
// A spinner that advances every frame from `dt` alone — no input. During a live OS
// window drag on macOS the event loop enters a modal run-loop and frames can stall,
// so this indicator visibly freezes during the drag and resumes after, making the
// engine-side mitigation (inline render on `Resized`) easy to confirm by eye.
const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

struct SpinnerSystem {
    label: Option<Entity>,
    elapsed: f32,
}

impl SpinnerSystem {
    fn new() -> Self {
        Self {
            label: None,
            elapsed: 0.0,
        }
    }
}

impl System for SpinnerSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        self.elapsed += dt;
        // Lazily (re)create the label; the world is reset on each scene change.
        let alive = self.label.map(|e| world.is_alive(e)).unwrap_or(false);
        if !alive {
            let e = world.spawn();
            world.add_component(
                e,
                UiNode::new(12.0, -12.0, 260.0, 26.0)
                    .with_anchor(Anchor::BottomLeft)
                    .with_z(0.99),
            );
            world.add_component(
                e,
                Label::new("")
                    .with_font_size(18.0)
                    .with_color([120, 230, 160, 255]),
            );
            self.label = Some(e);
        }
        if let Some(e) = self.label {
            let frame = SPINNER_FRAMES[((self.elapsed * 8.0) as usize) % SPINNER_FRAMES.len()];
            if let Some(label) = world.get_mut::<Label>(e) {
                label.text = format!("{frame}  live {:.1}s (drag the window)", self.elapsed);
            }
        }
    }
}

// ── Title scene ──────────────────────────────────────────────────────────────

struct TitleScene {
    entities: Vec<Entity>,
}

impl TitleScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Scene for TitleScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        configure_window(world);
        world.insert_resource(GameState::Paused);

        spawn_loc_label(
            world,
            &mut self.entities,
            "app.title",
            UiNode::new(0.0, -150.0, 760.0, 56.0)
                .with_anchor(Anchor::Center)
                .with_z(0.92),
            44.0,
            [240, 240, 240, 255],
            TextAlign::Center,
            true,
        );
        spawn_loc_label(
            world,
            &mut self.entities,
            "app.subtitle",
            UiNode::new(0.0, -92.0, 720.0, 32.0)
                .with_anchor(Anchor::Center)
                .with_z(0.92),
            22.0,
            [170, 210, 220, 255],
            TextAlign::Center,
            false,
        );
        let start = spawn_loc_button(
            world,
            &mut self.entities,
            "menu.start",
            UiNode::new(0.0, 0.0, 320.0, 60.0)
                .with_anchor(Anchor::Center)
                .with_z(0.94),
        );
        let quit = spawn_loc_button(
            world,
            &mut self.entities,
            "menu.quit",
            UiNode::new(0.0, 78.0, 320.0, 60.0)
                .with_anchor(Anchor::Center)
                .with_z(0.94),
        );

        systems.add(LocalizationSystem);
        systems.add(SpinnerSystem::new());
        systems.add(UiSystem::default());
        systems.add(TitleSystem { start, quit });
    }

    fn on_exit(&mut self, world: &mut World) {
        despawn_all(world, &mut self.entities);
    }
}

struct TitleSystem {
    start: Entity,
    quit: Entity,
}

impl System for TitleSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        let mut start = key_pressed(world, KeyCode::Enter);
        let mut quit = key_pressed(world, KeyCode::Escape);
        for ev in ui_events(world) {
            if let UiEvent::ButtonClicked(e) = ev {
                if e == self.start {
                    start = true;
                } else if e == self.quit {
                    quit = true;
                }
            }
        }
        if quit {
            if let Some(should_quit) = world.resource_mut::<ShouldQuit>() {
                should_quit.quit();
            }
        } else if start {
            blip(world, 660.0);
            request(world, SceneCmd::Replace(Box::new(SettingsScene::new())));
        }
    }
}

// ── Settings scene ─────────────────────────────────────────────────────────────

struct SettingsScene {
    entities: Vec<Entity>,
}

impl SettingsScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Scene for SettingsScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        configure_window(world);
        world.insert_resource(GameState::Paused);

        let settings = world.resource::<Settings>().cloned().unwrap_or_default();

        spawn_loc_label(
            world,
            &mut self.entities,
            "settings.title",
            UiNode::new(60.0, 30.0, 360.0, 44.0).with_z(0.92),
            34.0,
            [255, 220, 140, 255],
            TextAlign::Left,
            false,
        );

        // Left column laid out by Panel + LayoutSystem.
        let name_label = spawn_loc_label(
            world,
            &mut self.entities,
            "settings.name",
            UiNode::new(0.0, 0.0, 340.0, 22.0),
            18.0,
            [200, 210, 220, 255],
            TextAlign::Left,
            false,
        );
        let name_input = {
            let e = world.spawn();
            world.add_component(e, UiNode::new(0.0, 0.0, 340.0, 30.0));
            let mut input = TextInput::new("…").with_max_len(24);
            input.text = settings.name.clone();
            input.cursor = input.text.len(); // start the caret at the end, not in front
            world.add_component(e, input);
            self.entities.push(e);
            e
        };
        let music_label = spawn_loc_label(
            world,
            &mut self.entities,
            "settings.music",
            UiNode::new(0.0, 0.0, 340.0, 22.0),
            18.0,
            [200, 210, 220, 255],
            TextAlign::Left,
            false,
        );
        let music_slider = {
            let e = world.spawn();
            world.add_component(e, UiNode::new(0.0, 0.0, 340.0, 20.0));
            world.add_component(e, Slider::new(0.0, 1.0, settings.music));
            self.entities.push(e);
            e
        };
        let sfx_label = spawn_loc_label(
            world,
            &mut self.entities,
            "settings.sfx",
            UiNode::new(0.0, 0.0, 340.0, 22.0),
            18.0,
            [200, 210, 220, 255],
            TextAlign::Left,
            false,
        );
        let sfx_slider = {
            let e = world.spawn();
            world.add_component(e, UiNode::new(0.0, 0.0, 340.0, 20.0));
            world.add_component(e, Slider::new(0.0, 1.0, settings.sfx));
            self.entities.push(e);
            e
        };
        let subtitles_cb = {
            let e = world.spawn();
            world.add_component(e, UiNode::new(0.0, 0.0, 340.0, 28.0));
            let mut cb = CheckBox::new("");
            cb.checked = settings.subtitles;
            world.add_component(e, cb);
            world.add_component(e, LocalizedText::new("opt.subtitles"));
            self.entities.push(e);
            e
        };
        let fullscreen_cb = {
            let e = world.spawn();
            world.add_component(e, UiNode::new(0.0, 0.0, 340.0, 28.0));
            let mut cb = CheckBox::new("");
            cb.checked = settings.fullscreen;
            world.add_component(e, cb);
            world.add_component(e, LocalizedText::new("opt.fullscreen"));
            self.entities.push(e);
            e
        };

        let panel = world.spawn();
        world.add_component(panel, UiNode::new(60.0, 90.0, 372.0, 320.0).with_z(0.80));
        let mut p = Panel::new(LayoutDir::Vertical)
            .with_gap(10.0)
            .with_padding(16.0);
        p.background_color = Color::rgba(0.06, 0.08, 0.12, 0.95);
        p.children = vec![
            name_label,
            name_input,
            music_label,
            music_slider,
            sfx_label,
            sfx_slider,
            subtitles_cb,
            fullscreen_cb,
        ];
        world.add_component(panel, p);
        self.entities.push(panel);

        // Language switcher (literal labels — they name the language itself).
        let lang_en = spawn_plain_button(
            world,
            &mut self.entities,
            "EN",
            UiNode::new(470.0, 90.0, 130.0, 38.0).with_z(0.92),
        );
        let lang_ko = spawn_plain_button(
            world,
            &mut self.entities,
            "한국어",
            UiNode::new(610.0, 90.0, 130.0, 38.0).with_z(0.92),
        );
        let lang_es = spawn_plain_button(
            world,
            &mut self.entities,
            "ES",
            UiNode::new(750.0, 90.0, 130.0, 38.0).with_z(0.92),
        );

        // About / how-to-play list.
        spawn_loc_label(
            world,
            &mut self.entities,
            "about.head",
            UiNode::new(470.0, 142.0, 410.0, 24.0).with_z(0.92),
            18.0,
            [255, 220, 140, 255],
            TextAlign::Left,
            false,
        );
        let scroll = {
            let e = world.spawn();
            world.add_component(e, UiNode::new(470.0, 172.0, 410.0, 220.0).with_z(0.90));
            world.add_component(
                e,
                ScrollView::new()
                    .with_items(about_items(world))
                    .with_item_height(34.0),
            );
            self.entities.push(e);
            e
        };

        let back = spawn_loc_button(
            world,
            &mut self.entities,
            "menu.back",
            UiNode::new(60.0, 430.0, 200.0, 50.0).with_z(0.92),
        );
        let cont = spawn_loc_button(
            world,
            &mut self.entities,
            "menu.continue",
            UiNode::new(280.0, 430.0, 200.0, 50.0).with_z(0.92),
        );

        // Dedicated long-text field (deliberately narrow, prefilled past its width) to
        // exercise single-line horizontal scroll + caret-follow, with a reachable
        // `max_len` so IME-at-capacity honesty is testable. Standalone (not in the panel).
        {
            let label = world.spawn();
            world.add_component(label, UiNode::new(500.0, 410.0, 380.0, 20.0).with_z(0.92));
            world.add_component(
                label,
                Label::new("Long-text field — type past the edge (scrolls)")
                    .with_font_size(15.0)
                    .with_color([255, 220, 140, 255]),
            );
            self.entities.push(label);

            let field = world.spawn();
            world.add_component(field, UiNode::new(500.0, 434.0, 200.0, 32.0).with_z(0.92));
            let mut input = TextInput::new("type a long line…").with_max_len(48);
            input.text = "The quick brown fox jumps over the lazy dog".to_string();
            input.cursor = input.text.len();
            world.add_component(field, input);
            self.entities.push(field);
        }

        // Re-apply persisted bus volumes (survives the world reset; harmless to repeat),
        // then start the sustained music tone so the Music slider is audible.
        set_bus(world, "music", settings.music);
        set_bus(world, "sfx", settings.sfx);
        play_bgm(world);

        systems.add(LocalizationSystem);
        systems.add(LayoutSystem);
        systems.add(SpinnerSystem::new());
        // UiSystem reads the layout geometry computed by LayoutSystem each frame — must run after it.
        systems.add_labeled(
            UiSystem::default(),
            SystemConfig::new().after(LayoutSystem::LABEL),
        );
        systems.add(SettingsSystem {
            name_input,
            music_slider,
            sfx_slider,
            subtitles_cb,
            fullscreen_cb,
            lang_en,
            lang_ko,
            lang_es,
            scroll,
            back,
            cont,
            last_sfx_step: -1,
        });
    }

    fn on_exit(&mut self, world: &mut World) {
        despawn_all(world, &mut self.entities);
    }
}

struct SettingsSystem {
    name_input: Entity,
    music_slider: Entity,
    sfx_slider: Entity,
    subtitles_cb: Entity,
    fullscreen_cb: Entity,
    lang_en: Entity,
    lang_ko: Entity,
    lang_es: Entity,
    scroll: Entity,
    back: Entity,
    cont: Entity,
    /// Last 5%-quantized SFX value we played a blip for, so dragging gives
    /// occasional feedback instead of restarting a tone every frame (which is silent).
    last_sfx_step: i32,
}

impl SettingsSystem {
    fn set_locale(&self, world: &mut World, code: &str) {
        if let Some(locale) = world.resource_mut::<LocaleResource>() {
            locale.set_locale(code);
        }
        if let Some(settings) = world.resource_mut::<Settings>() {
            settings.locale = code.to_string();
        }
        // Rebuild the ScrollView items in the new language (it is data, not a label).
        let items = about_items(world);
        if let Some(sv) = world.get_mut::<ScrollView>(self.scroll) {
            sv.items = items;
        }
        blip(world, 520.0);
    }
}

impl System for SettingsSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        keep_bgm(world);

        let mut go_back = key_pressed(world, KeyCode::Escape);
        let mut go_cont = false;

        for ev in ui_events(world) {
            match ev {
                UiEvent::SliderChanged(e, v) => {
                    if e == self.music_slider {
                        if let Some(s) = world.resource_mut::<Settings>() {
                            s.music = v;
                        }
                        set_bus(world, "music", v);
                    } else if e == self.sfx_slider {
                        if let Some(s) = world.resource_mut::<Settings>() {
                            s.sfx = v;
                        }
                        set_bus(world, "sfx", v);
                        // Blip on the sfx bus only when the 5% step changes, so it is
                        // audible without restarting a 50 ms tone every drag frame.
                        let step = (v * 20.0).round() as i32;
                        if step != self.last_sfx_step {
                            self.last_sfx_step = step;
                            blip(world, 740.0);
                        }
                    }
                }
                UiEvent::CheckBoxToggled(e, checked) => {
                    if e == self.subtitles_cb {
                        if let Some(s) = world.resource_mut::<Settings>() {
                            s.subtitles = checked;
                        }
                    } else if e == self.fullscreen_cb {
                        if let Some(s) = world.resource_mut::<Settings>() {
                            s.fullscreen = checked;
                        }
                    }
                    blip(world, 600.0);
                }
                UiEvent::TextChanged(e, text) | UiEvent::TextSubmitted(e, text)
                    if e == self.name_input =>
                {
                    if let Some(s) = world.resource_mut::<Settings>() {
                        s.name = text;
                    }
                }
                UiEvent::ButtonClicked(e) => {
                    if e == self.lang_en {
                        self.set_locale(world, "en");
                    } else if e == self.lang_ko {
                        self.set_locale(world, "ko");
                    } else if e == self.lang_es {
                        self.set_locale(world, "es");
                    } else if e == self.back {
                        go_back = true;
                    } else if e == self.cont {
                        go_cont = true;
                    }
                }
                _ => {}
            }
        }

        if go_back {
            request(world, SceneCmd::Replace(Box::new(TitleScene::new())));
        } else if go_cont {
            blip(world, 660.0);
            request(world, SceneCmd::Replace(Box::new(DialogueScene::new())));
        }
    }
}

// ── Dialogue scene ─────────────────────────────────────────────────────────────

struct DialogueScene {
    entities: Vec<Entity>,
}

impl DialogueScene {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
        }
    }
}

impl Scene for DialogueScene {
    fn on_enter(&mut self, world: &mut World, systems: &mut SystemRegistrar) {
        configure_window(world);
        world.insert_resource(GameState::Playing);

        // Dialogue box background.
        let panel = world.spawn();
        world.add_component(panel, UiNode::new(80.0, 380.0, 800.0, 170.0).with_z(0.80));
        let mut p = Panel::new(LayoutDir::Vertical)
            .with_padding(20.0)
            .with_gap(12.0);
        p.background_color = Color::rgba(0.05, 0.07, 0.11, 0.96);
        world.add_component(panel, p);
        self.entities.push(panel);

        // Speaker name + body line are filled dynamically by DialogueSystem.
        let speaker = world.spawn();
        world.add_component(speaker, UiNode::new(104.0, 396.0, 760.0, 28.0).with_z(0.92));
        world.add_component(
            speaker,
            Label::new("")
                .with_font_size(22.0)
                .with_color([255, 210, 120, 255]),
        );
        self.entities.push(speaker);

        let body = world.spawn();
        world.add_component(body, UiNode::new(104.0, 430.0, 752.0, 80.0).with_z(0.92));
        world.add_component(
            body,
            Label::new("")
                .with_font_size(20.0)
                .with_color([225, 230, 235, 255])
                .with_align(TextAlign::Left),
        );
        self.entities.push(body);

        let help = spawn_loc_label(
            world,
            &mut self.entities,
            "dialogue.help",
            UiNode::new(0.0, -40.0, 760.0, 24.0)
                .with_anchor(Anchor::BottomCenter)
                .with_z(0.92),
            16.0,
            [150, 170, 185, 255],
            TextAlign::Center,
            false,
        );
        let _ = help;

        play_bgm(world);

        systems.add(LocalizationSystem);
        systems.add(LayoutSystem);
        systems.add(SpinnerSystem::new());
        systems.add(UiSystem::default());
        systems.add(DialogueSystem {
            speaker,
            body,
            line: 0,
            muffled: false,
        });
    }

    fn on_exit(&mut self, world: &mut World) {
        despawn_all(world, &mut self.entities);
    }
}

struct DialogueSystem {
    speaker: Entity,
    body: Entity,
    line: usize,
    muffled: bool,
}

const DIALOGUE_LINES: usize = 2;

impl System for DialogueSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        keep_bgm(world);

        if key_pressed(world, KeyCode::Escape) {
            request(world, SceneCmd::Replace(Box::new(TitleScene::new())));
            return;
        }
        if key_pressed(world, KeyCode::KeyM) {
            self.muffled = !self.muffled;
            set_muffle(world, self.muffled);
        }
        if key_pressed(world, KeyCode::Space) {
            self.line += 1;
            if self.line >= DIALOGUE_LINES {
                request(world, SceneCmd::Replace(Box::new(TitleScene::new())));
                return;
            }
        }

        let (name, subtitles) = world
            .resource::<Settings>()
            .map(|s| (s.name.clone(), s.subtitles))
            .unwrap_or_else(|| ("Hero".to_string(), true));

        let speaker_text = t(world, "npc.name");
        let body_text = if !subtitles {
            t(world, "dialogue.off")
        } else if self.line == 0 {
            format!("{}, {}!", t(world, "npc.greet"), name)
        } else {
            t(world, "npc.line2")
        };

        if let Some(label) = world.get_mut::<Label>(self.speaker) {
            label.text = speaker_text;
        }
        if let Some(label) = world.get_mut::<Label>(self.body) {
            label.text = body_text;
        }
    }
}

// ── entry point ────────────────────────────────────────────────────────────────

/// The whole game, ready to `run()`. The acceptance test below calls **this** rather than
/// assembling its own app: the persistent-resource registrations are precisely what it verifies,
/// so a harness that repeated them would be grading its own copy and would stay green after a
/// `register_persistent` call was dropped from the real setup.
fn build_app() -> App {
    let mut app = App::new();
    app.register_event::<UiEvent>();

    app.world.insert_resource(Settings::default());
    app.world
        .insert_resource(LocaleResource::from_ron_str(LOCALES_RON).expect("valid locale bundle"));
    app.register_persistent::<Settings>();
    app.register_persistent::<LocaleResource>();

    // Cross-platform audio via the Audio facade: two buses (music / sfx), preserved across scene
    // resets by the engine itself (`App::new` registers `Audio` as persistent). The facade assigns
    // each tone to its bus per call (play_tone_on_channel / _on_bus), so only the bus volumes are
    // set here. Works the same on native and web — no cfg guards.
    if let Some(mut audio) = Audio::new() {
        audio.set_bus_volume("music", 0.6);
        audio.set_bus_volume("sfx", 0.6);
        app.world.insert_resource(audio);
    }

    app.set_scene(Box::new(TitleScene::new()));
    configure_window(&mut app.world);
    app
}

fn main() {
    // `SETTINGS_MENU_SELFTEST=1` runs the headless acceptance test instead of opening a window.
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("SETTINGS_MENU_SELFTEST").is_ok() {
        std::process::exit(self_test());
    }

    // Native log backend so `log::debug!` (incl. the engine's `frame gap …ms`
    // drag-stall instrumentation) is visible. Control with e.g. RUST_LOG=engine=debug.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = env_logger::try_init();

    build_app().run();
}

// ── Acceptance test ───────────────────────────────────────────────────────────────────────────

/// `SETTINGS_MENU_SELFTEST=1 cargo run --example settings_menu_game` — asserts what this example
/// exists to show and a screenshot cannot.
///
/// The subject is the engine's **documented reset footgun**: `SceneCmd::Replace` rebuilds the
/// `World`, and every resource in it is dropped unless `register_persistent` named the type. This
/// example is the tree's cross-scene state case — `Settings` and `LocaleResource` are registered,
/// `Audio` is registered by `App::new` itself — so it is where the mechanism gets driven.
///
/// A capture can photograph the settings screen in Korean after a scripted round trip, so the
/// locale alone is not the argument. Three things here have no pixels at all:
///
/// * **The failure is silent and flatters itself.** Every read site in this example is
///   `unwrap_or_default()` / `unwrap_or_else` — a game must not crash on a missing resource — so a
///   dropped `Settings` does not error, it renders a *complete, plausible settings screen* built
///   from `Settings::default()`. "Hero", 0.6, 0.6, subtitles on is exactly as convincing a
///   photograph as whatever the player actually chose. Nothing on screen distinguishes state that
///   survived from state that was silently re-defaulted.
/// * **Selectivity has no visual.** Check 3 is the one that gives the others meaning: a resource
///   that was *never* registered must be **gone**. Without it an engine that simply never reset the
///   `World` would pass every positive check here.
/// * **`Audio` cannot be photographed at all** — a headless capture advances at a fixed `1/60` dt
///   with no wall clock. Whether the device handle and its bus volumes crossed the reset is the
///   "session state, must persist" half of the v0.139.1 audit, and no still frame reaches it.
///
/// It is also the first check in the tree that crosses a scene boundary. It could not exist before
/// `App::step_headless`: `SceneCmd` is consumed *after* the schedule, so a test that ticks systems
/// by hand — which is how every other selftest here drives a scene — never reaches the transition
/// and never sees the reset.
///
/// Exit codes: `0` pass (the audio check may have skipped) · `1` the title→settings transition does
/// not happen · `2` the language button does not change the locale · `3` a resource that was never
/// registered persistent survives the reset · `4` registered cross-scene state does not survive it
/// · `5` the rebuilt UI does not read the state that survived · `6` the engine-registered `Audio`
/// handle or its bus volumes do not survive.
#[cfg(not(target_arch = "wasm32"))]
fn self_test() -> i32 {
    use engine::{InputAction, InputScript, MouseButton, Vec2, ViewportSize};

    /// Fixed step. Everything asserted below is driven by scripted input and by `dt`; nothing
    /// waits on a wall clock, so this reproduces real play exactly. (The audio check reads stored
    /// bus state, not a live meter — a *meter* is what would need an `Instant`.)
    const DT: f32 = 1.0 / 60.0;

    /// A full app frame each: scripted input, the schedule, then the end-of-frame work where
    /// `SceneCmd` is consumed. The transition under test lives in that last part.
    fn steps(app: &mut App, n: u32) {
        for _ in 0..n {
            app.step_headless(DT);
        }
    }

    /// The Settings screen is the only scene with sliders, and it has exactly two.
    fn slider_count(world: &World) -> usize {
        world.query::<Slider>().count()
    }

    // A resource the game never registers — the negative control for check 3.
    struct SceneLocal;

    // ── Arrange ───────────────────────────────────────────────────────────────────────────────
    //
    // Does this machine have an output device at all? Asked **before the app exists**, because
    // check 6 needs to tell "no sound card" apart from "the reset dropped the handle" and those
    // are the same observable afterwards — a check that reads only the far side takes the SKIP
    // path when it should fail, and reports success.
    //
    // Sampling `app.world.resource::<Audio>()` after `build_app` does not work either, and the
    // reason is worth keeping: **`set_scene` is itself a `SceneCmd::Replace`**, so `build_app`
    // has already performed one world reset by the time it returns. A sample taken there is a
    // sample taken downstream of the mechanism under test. (Both of these were caught by
    // sabotaging the engine's own `register_persistent::<Audio>` call and watching this check
    // pass twice.)
    let device_available = Audio::new().is_some();

    // `build_app` is the game's own setup, `register_persistent` calls included: those calls are
    // the subject, so the harness must not repeat them.
    let mut app = build_app();

    // Headless has no surface, and `compute_viewport` — the only writer of ViewportSize — is
    // skipped without a GPU, so the UI would have no coordinate space to hit-test the click in.
    // Supply the window's own size, and keep it across the resets the windowed app would simply
    // recompute it after. This is the one thing the harness stands in for.
    app.world
        .insert_resource(ViewportSize::new(WINDOW_W, WINDOW_H));
    app.register_persistent::<ViewportSize>();

    // ── 1. The real transition happens ────────────────────────────────────────────────────────
    //
    // Everything below is about what crosses a scene boundary, so a run that never crossed one
    // would pass the rest vacuously.
    app.set_input_script(InputScript::new([(
        1,
        InputAction::KeyPress(KeyCode::Enter),
    )]));
    steps(&mut app, 8);

    if slider_count(&app.world) != 2 {
        eprintln!(
            "FAIL: Enter at the title did not reach the Settings scene — {} slider(s) in the \
             world after 8 frames, want 2. Nothing below crosses a scene boundary without this.",
            slider_count(&app.world)
        );
        return 1;
    }
    println!("transition ok: title → settings, 2 sliders on screen");

    // ── 2. Change the language through the real UI ────────────────────────────────────────────
    //
    // A click on the 한국어 button, not a resource poke: it is `SettingsSystem`'s own handler that
    // writes both `LocaleResource` and `Settings.locale`, and a harness that wrote them itself
    // would keep passing after that handler was deleted. The language buttons are absolutely
    // positioned (`UiNode::new(610.0, 90.0, 130.0, 38.0)`), not laid out by `LayoutSystem`, so
    // their centre is known without reproducing the layout pass.
    app.set_input_script(InputScript::new([
        (0, InputAction::MouseMove(Vec2::new(675.0, 109.0))),
        (1, InputAction::Click(MouseButton::Left)),
    ]));
    steps(&mut app, 6);

    let locale_now = app
        .world
        .resource::<LocaleResource>()
        .map(|l| l.current_locale().to_string())
        .unwrap_or_default();
    if locale_now != "ko" {
        eprintln!(
            "FAIL: clicking the 한국어 button did not switch the locale — current locale {locale_now:?}, \
             want \"ko\". The state the reset checks are about to carry was never actually set."
        );
        return 2;
    }
    println!("ui ok: 한국어 click switched the locale to ko");

    // The remaining values a player sets by typing and dragging inside the laid-out panel.
    // Set directly: reproducing `LayoutSystem`'s geometry to aim a drag would test the layout
    // pass, not the reset. What is under test is that these values *cross the boundary*, and
    // they are distinguishable from `Settings::default()` ("Hero", 0.6) on purpose — a silent
    // re-default has to look different from a survival, or the check proves nothing.
    if let Some(s) = app.world.resource_mut::<Settings>() {
        s.name = "Ada".to_string();
        s.music = 0.9;
    }
    if let Some(audio) = app.world.resource_mut::<Audio>() {
        audio.set_bus_volume("music", 0.9);
    }
    // The negative control, inserted on the far side of the boundary from its check.
    app.world.insert_resource(SceneLocal);

    // ── 3. An unregistered resource does NOT survive ──────────────────────────────────────────
    //
    // The check that gives checks 4-6 their meaning. `SceneLocal` differs from `Settings` in
    // exactly one way — nothing called `register_persistent` for it — so if it is still here the
    // World was never reset, and "persisted" below would mean only "never dropped anything".
    app.set_input_script(InputScript::new([(
        1,
        InputAction::KeyPress(KeyCode::Escape),
    )]));
    steps(&mut app, 8);

    let back_at_title = slider_count(&app.world) == 0;
    let leaked = app.world.resource::<SceneLocal>().is_some();
    if !back_at_title || leaked {
        eprintln!(
            "FAIL: the world reset did not happen as a reset — back at the title: {back_at_title} \
             (want true; {} slider(s) still alive), unregistered SceneLocal still present: {leaked} \
             (want false). If it survived, `register_persistent` is not what decides, and every \
             check below would pass on an engine that never resets the World at all.",
            slider_count(&app.world)
        );
        return 3;
    }
    println!("reset ok: settings → title, scene entities gone, unregistered resource dropped");

    // ── 4. Registered cross-scene state DOES survive ──────────────────────────────────────────
    app.set_input_script(InputScript::new([(
        1,
        InputAction::KeyPress(KeyCode::Enter),
    )]));
    steps(&mut app, 8);

    if slider_count(&app.world) != 2 {
        eprintln!(
            "FAIL: could not return to the Settings scene — {} slider(s), want 2.",
            slider_count(&app.world)
        );
        return 1;
    }

    let settings = app
        .world
        .resource::<Settings>()
        .cloned()
        .unwrap_or_default();
    let locale = app
        .world
        .resource::<LocaleResource>()
        .map(|l| l.current_locale().to_string())
        .unwrap_or_default();
    if settings.name != "Ada" || (settings.music - 0.9).abs() > 1e-6 || locale != "ko" {
        eprintln!(
            "FAIL: cross-scene state did not survive the reset — name {:?} (want \"Ada\"), music \
             {:.3} (want 0.900), locale {locale:?} (want \"ko\"). Defaults are \"Hero\" / 0.600 / \
             \"en\": reading those back means the resources were dropped and re-created, which \
             renders a perfectly plausible settings screen and reports no error anywhere.",
            settings.name, settings.music
        );
        return 4;
    }
    println!(
        "persist ok: name {:?}, music {:.2}, locale {locale:?} survived a Replace round trip",
        settings.name, settings.music
    );

    // ── 5. The rebuilt UI reads the state that survived ───────────────────────────────────────
    //
    // Checks 4 and 5 are not the same check. A `Settings` that survives but is never read back
    // leaves the player looking at default widgets, which is the same bug from the player's side.
    // `on_enter` seeds the widgets from the resource; `LocalizationSystem` retranslates the
    // labels — so the Korean title proves the *rebuilt* tree consumed the persisted locale, not
    // merely that a resource somewhere still holds "ko".
    let name_shown = app
        .world
        .query::<TextInput>()
        .any(|(_, input)| input.text == "Ada");
    let music_shown = app
        .world
        .query::<Slider>()
        .any(|(_, s)| (s.value - 0.9).abs() < 1e-6);
    let korean_title = app.world.query::<Label>().any(|(_, l)| l.text == "설정");
    if !name_shown || !music_shown || !korean_title {
        eprintln!(
            "FAIL: the rebuilt Settings screen did not read the state that survived — name field \
             shows \"Ada\": {name_shown}, a slider sits at 0.9: {music_shown}, the title label \
             retranslated to \"설정\": {korean_title} (all want true). The resource surviving and \
             the screen showing it are two different failures with the same symptom for a player."
        );
        return 5;
    }
    println!("rebuild ok: the new scene's widgets show Ada / 0.90 / 설정");

    // ── 6. The engine-registered Audio handle survives ────────────────────────────────────────
    //
    // `Audio` is session state: a device handle and its bus mix, registered persistent by
    // `App::new` (v0.141.1) rather than by this example. Dropping it on a scene change would
    // silence the game from the second scene onward and raise nothing — and no capture can
    // photograph it, because a headless run has no wall clock for audio to be measured on.
    //
    // The inline test `audio_is_registered_to_survive_a_scene_reset` can only assert on the
    // *registration*, because CI has no device and `Audio::new()` returns `None` there. This runs
    // on a machine that has one, so it asserts on the surviving **instance** and its bus mix —
    // the thing the registration exists to produce.
    //
    // SKIPs with exit 0 only when the machine has no device, the same rule
    // `BEAT_CRAWLER_SELFTEST` uses — decided by `device_available`, probed before the app existed,
    // so a handle the reset actually dropped fails here instead of skipping.
    if !device_available {
        println!("SKIP: no audio device on this machine — the Audio persistence check did not run");
    } else {
        let Some(audio) = app.world.resource::<Audio>() else {
            eprintln!(
                "FAIL: the Audio resource is gone after the reset, though this machine has a \
                 working output device. `App::new` registers Audio persistent (v0.141.1); \
                 without that the game \
                 falls silent from the second scene onward and every `resource_mut::<Audio>()` \
                 call site quietly does nothing — no error, no log, no pixels."
            );
            return 6;
        };
        let music = audio.bus_volume("music");
        let sfx = audio.bus_volume("sfx");
        if (music - 0.9).abs() > 1e-6 || (sfx - 0.6).abs() > 1e-6 {
            eprintln!(
                "FAIL: the Audio handle did not carry its bus mix across the reset — music bus \
                 {music:.3} (want 0.900, set before the transition), sfx bus {sfx:.3} (want \
                 0.600, set at startup). Audio is registered persistent by `App::new`, not by \
                 this example, so this is the engine's own registration under test."
            );
            return 6;
        }
        println!("audio ok: the Audio handle survived with music {music:.2} / sfx {sfx:.2}");
    }

    println!("SETTINGS_MENU_SELFTEST: all checks passed");
    0
}
