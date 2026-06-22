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
//! Cross-scene state (`Settings`, the locale, and the cross-platform `Audio`) is kept
//! across the `SceneCmd::Replace` world reset via `App::register_persistent`.

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

fn main() {
    // Native log backend so `log::debug!` (incl. the engine's `frame gap …ms`
    // drag-stall instrumentation) is visible. Control with e.g. RUST_LOG=engine=debug.
    #[cfg(not(target_arch = "wasm32"))]
    let _ = env_logger::try_init();

    let mut app = App::new();
    app.register_event::<UiEvent>();

    app.world.insert_resource(Settings::default());
    app.world
        .insert_resource(LocaleResource::from_ron_str(LOCALES_RON).expect("valid locale bundle"));
    app.register_persistent::<Settings>();
    app.register_persistent::<LocaleResource>();

    // Cross-platform audio via the Audio facade: two buses (music / sfx) preserved across scene
    // resets. The facade assigns each tone to its bus per call (play_tone_on_channel / _on_bus),
    // so only the bus volumes are set here. Works the same on native and web — no cfg guards.
    if let Some(mut audio) = Audio::new() {
        audio.set_bus_volume("music", 0.6);
        audio.set_bus_volume("sfx", 0.6);
        app.world.insert_resource(audio);
        app.register_persistent::<Audio>();
    }

    app.set_scene(Box::new(TitleScene::new()));
    configure_window(&mut app.world);
    app.run();
}
