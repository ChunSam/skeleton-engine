use crate::ecs::{Entity, System, World};
use crate::locale::LocaleResource;

use super::button::Button;
use super::checkbox::CheckBox;
use super::label::Label;

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
pub struct LocalizedText {
    /// Translation key looked up via [`LocaleResource::t`].
    pub key: String,
}

impl LocalizedText {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

/// Resolves every [`LocalizedText`] through the [`LocaleResource`] and writes the
/// result into the entity's [`Label`] / [`Button`] / [`CheckBox`] text field.
///
/// Register it before [`UiSystem`](super::UiSystem) so freshly translated text is
/// rendered the same frame. No-op when no [`LocaleResource`] exists.
pub struct LocalizationSystem;

impl System for LocalizationSystem {
    fn run(&mut self, world: &mut World, _dt: f32) {
        // 1. Collect (entity, key) — query borrow released before locale lookup.
        let entries: Vec<(Entity, String)> = world
            .query::<LocalizedText>()
            .map(|(entity, lt)| (entity, lt.key.clone()))
            .collect();
        if entries.is_empty() {
            return;
        }

        // 2. Resolve against the current locale.
        let resolved: Vec<(Entity, String)> = {
            let locale = match world.resource::<LocaleResource>() {
                Some(l) => l,
                None => return,
            };
            entries
                .into_iter()
                .map(|(entity, key)| {
                    let text = locale.t(&key).to_string();
                    (entity, text)
                })
                .collect()
        };

        // 3. Apply to whichever text-bearing widget the entity carries.
        for (entity, text) in resolved {
            if let Some(label) = world.get_mut::<Label>(entity) {
                label.text = text.clone();
            }
            if let Some(button) = world.get_mut::<Button>(entity) {
                button.label = text.clone();
            }
            if let Some(checkbox) = world.get_mut::<CheckBox>(entity) {
                checkbox.label = text;
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
}
