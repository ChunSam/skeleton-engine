use serde::{Deserialize, Serialize};

use crate::ecs::{Entity, System, World};
use crate::locale::LocaleResource;
use crate::reflect::{Reflect, ReflectValue};

use super::button::Button;
use super::checkbox::CheckBox;
use super::label::Label;
use super::text_input::TextInput;

/// Binds a translation `key` to a text-bearing UI widget.
///
/// Attach it alongside a [`Label`], [`Button`], or [`CheckBox`] and
/// [`LocalizationSystem`] will keep that widget's text in sync with the current
/// locale every frame — calling [`LocaleResource::set_locale`] is enough to
/// retranslate the whole UI, with no manual per-widget rebuild.
///
/// # Example
/// ```ignore
/// let e = world.spawn();
/// world.add_component(e, UiNode::new(0.0, 0.0, 200.0, 40.0));
/// world.add_component(e, Label::new(""));
/// world.add_component(e, LocalizedText::new("menu.start"));
/// // ...later: world.resource_mut::<LocaleResource>().unwrap().set_locale("ko");
/// // LocalizationSystem now sets the Label text to the Korean "menu.start".
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct LocalizedText {
    /// Translation key looked up via [`LocaleResource::t`].
    pub key: String,
}

impl Reflect for LocalizedText {
    fn fields(&self) -> Vec<(&'static str, ReflectValue)> {
        vec![("key", ReflectValue::String(self.key.clone()))]
    }

    fn set_field(&mut self, name: &str, val: ReflectValue) -> bool {
        match (name, val) {
            ("key", ReflectValue::String(v)) => {
                self.key = v;
                true
            }
            _ => false,
        }
    }

    fn type_name(&self) -> &'static str {
        "LocalizedText"
    }
}

impl LocalizedText {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

/// Resolves every [`LocalizedText`] through the [`LocaleResource`] and writes the
/// result into the entity's [`Label`] / [`Button`] / [`CheckBox`] text field.
///
/// [`LocaleResource`] lives in `src/locale.rs` and is the single source of truth for
/// translation data. Each frame this system calls [`LocaleResource::t`] for every
/// [`LocalizedText`] key, so switching locales via [`LocaleResource::set_locale`]
/// automatically retranslates all bound widgets on the next frame.
///
/// Register it before [`UiSystem`](super::UiSystem) so freshly translated text is
/// rendered the same frame. No-op when no [`LocaleResource`] exists.
pub struct LocalizationSystem;

impl LocalizationSystem {
    /// Schedule label. Recommended order: **before** `UiSystem::LABEL` so resolved
    /// text is rendered the same frame.
    pub const LABEL: crate::ecs::schedule::SystemLabel = "engine::localization";
}

impl System for LocalizationSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // 1+2. Resolve, and buffer ONLY the entities whose text is actually stale.
        //
        // This used to be two passes with two `Vec`s: collect `(Entity, key.clone())`, then map
        // each key through the locale into another `String`. Both allocated unconditionally, so a
        // steady screen paid two `Vec`s plus two `String`s per entity **every frame** just to
        // rediscover that nothing had changed — measured at 401 allocations / 10,400 bytes per
        // frame for 200 labels (`tests/per_frame_alloc.rs`).
        //
        // Two things make one allocation-free pass possible. The key clone was never necessary:
        // the query and the `LocaleResource` lookup are both `&World`, so they coexist and the
        // key can be borrowed straight through `t()`. And staleness is decidable with immutable
        // reads, so `to_string()` happens only for text that is actually about to change — which
        // in steady state is nothing, leaving `pending` empty and `Vec::new()` allocation-free.
        let mut pending: Vec<(Entity, String)> = Vec::new();
        {
            let locale = match world.resource::<LocaleResource>() {
                Some(l) => l,
                None => return,
            };
            for (entity, lt) in world.query::<LocalizedText>() {
                let want = locale.t(&lt.key);
                // Stale if ANY text-bearing widget on this entity disagrees. Mirrors the write
                // below, so the two cannot drift into "buffered but never written".
                let stale = world.get::<Label>(entity).is_some_and(|w| w.text != want)
                    || world.get::<Button>(entity).is_some_and(|w| w.label != want)
                    || world
                        .get::<CheckBox>(entity)
                        .is_some_and(|w| w.label != want)
                    || world
                        .get::<TextInput>(entity)
                        .is_some_and(|w| w.placeholder != want);
                if stale {
                    pending.push((entity, want.to_string()));
                }
            }
        }
        if pending.is_empty() {
            return;
        }

        // 3. Apply to whichever text-bearing widget the entity carries.
        // An entity carries at most one text-bearing widget in practice, but every branch
        // cloned unconditionally — so the common single-widget case paid for four `String`
        // allocations per entity per frame and threw three away. Write into whichever widget
        // matches and MOVE the string into the last one; also skip the write when the text is
        // already correct, which is every frame between locale switches.
        for (entity, text) in pending {
            if let Some(label) = world.get_mut::<Label>(entity) {
                if label.text != text {
                    label.text = text.clone();
                }
            }
            if let Some(button) = world.get_mut::<Button>(entity) {
                if button.label != text {
                    button.label = text.clone();
                }
            }
            if let Some(checkbox) = world.get_mut::<CheckBox>(entity) {
                if checkbox.label != text {
                    checkbox.label = text.clone();
                }
            }
            if let Some(ti) = world.get_mut::<TextInput>(entity) {
                if ti.placeholder != text {
                    ti.placeholder = text;
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "LocalizationSystem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
(
    default_locale: "en",
    locales: {
        "en": ( translations: { "menu.start": "Start", "opt.subtitles": "Subtitles" } ),
        "ko": ( translations: { "menu.start": "시작", "opt.subtitles": "자막" } ),
    },
)
"#;

    fn world_with_locale() -> World {
        let mut world = World::new();
        world.insert_resource(LocaleResource::from_ron_str(SAMPLE).unwrap());
        world
    }

    #[test]
    fn resolves_label_text_for_current_locale() {
        let mut world = world_with_locale();
        let e = world.spawn();
        world.add_component(e, Label::new(""));
        world.add_component(e, LocalizedText::new("menu.start"));

        LocalizationSystem.run(&mut world, 0.0);
        assert_eq!(world.get::<Label>(e).unwrap().text, "Start");
    }

    #[test]
    fn locale_switch_updates_every_widget_kind() {
        let mut world = world_with_locale();

        let label = world.spawn();
        world.add_component(label, Label::new(""));
        world.add_component(label, LocalizedText::new("menu.start"));

        let button = world.spawn();
        world.add_component(button, Button::new(""));
        world.add_component(button, LocalizedText::new("menu.start"));

        let checkbox = world.spawn();
        world.add_component(checkbox, CheckBox::new(""));
        world.add_component(checkbox, LocalizedText::new("opt.subtitles"));

        LocalizationSystem.run(&mut world, 0.0);
        assert_eq!(world.get::<Label>(label).unwrap().text, "Start");
        assert_eq!(world.get::<Button>(button).unwrap().label, "Start");
        assert_eq!(world.get::<CheckBox>(checkbox).unwrap().label, "Subtitles");

        world
            .resource_mut::<LocaleResource>()
            .unwrap()
            .set_locale("ko");
        LocalizationSystem.run(&mut world, 0.0);
        assert_eq!(world.get::<Label>(label).unwrap().text, "시작");
        assert_eq!(world.get::<Button>(button).unwrap().label, "시작");
        assert_eq!(world.get::<CheckBox>(checkbox).unwrap().label, "자막");
    }

    #[test]
    fn missing_key_falls_back_to_key() {
        let mut world = world_with_locale();
        let e = world.spawn();
        world.add_component(e, Label::new(""));
        world.add_component(e, LocalizedText::new("does.not.exist"));

        LocalizationSystem.run(&mut world, 0.0);
        assert_eq!(world.get::<Label>(e).unwrap().text, "does.not.exist");
    }

    #[test]
    fn locale_switch_updates_text_input_placeholder() {
        let mut world = world_with_locale();

        let ti_entity = world.spawn();
        world.add_component(ti_entity, TextInput::new(""));
        world.add_component(ti_entity, LocalizedText::new("menu.start"));

        // First run: English locale.
        LocalizationSystem.run(&mut world, 0.0);
        assert_eq!(
            world.get::<TextInput>(ti_entity).unwrap().placeholder,
            "Start",
            "TextInput placeholder should be resolved for the current locale"
        );

        // Switch to Korean and re-run.
        world
            .resource_mut::<LocaleResource>()
            .unwrap()
            .set_locale("ko");
        LocalizationSystem.run(&mut world, 0.0);
        assert_eq!(
            world.get::<TextInput>(ti_entity).unwrap().placeholder,
            "시작",
            "TextInput placeholder should update after set_locale"
        );
    }
}
