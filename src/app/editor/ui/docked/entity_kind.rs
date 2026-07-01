//! Entity "kind" classification — shared type-icon + sort-by-kind logic.

use super::*;

/// An entity's editor "kind", derived from its most salient component. Backs both the per-row
/// type-icon (via [`EntityKind::icon`]) and the "sort by kind" order (the variant order = the group
/// order). The classification is a single priority ladder ([`entity_kind`]) so icon and sort never
/// drift apart.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum EntityKind {
    Light,
    Tilemap,
    Particles,
    Camera,
    Animation,
    Ui,
    Sprite,
    Transform,
    Bare,
}

#[cfg(not(target_arch = "wasm32"))]
impl EntityKind {
    /// The one-glyph editor icon for this kind. Glyphs are chosen from egui's bundled emoji set
    /// (verified to render, not □ tofu, in the headless docked capture).
    fn icon(self) -> &'static str {
        match self {
            EntityKind::Light => "💡",
            EntityKind::Tilemap => "🗺",
            EntityKind::Particles => "✨",
            EntityKind::Camera => "🎥",
            EntityKind::Animation => "🎬",
            EntityKind::Ui => "🔘",
            EntityKind::Sprite => "🖼",
            EntityKind::Transform => "🔹",
            EntityKind::Bare => "·",
        }
    }
}

/// Classify an entity by its most salient component, checked in priority order (first match wins) —
/// so a light that also has a sprite still classifies as [`EntityKind::Light`]. A transform-only
/// entity is [`EntityKind::Transform`]; a bare / marker-only one is [`EntityKind::Bare`]. Native-only
/// (the whole docked UI is); a pure `world.get` scan, so it never mutates.
#[cfg(not(target_arch = "wasm32"))]
fn entity_kind(world: &crate::World, e: Entity) -> EntityKind {
    use crate::{
        AnimationPlayer, AnimationStateMachine, AtlasSprite, Button, CameraTarget, CheckBox, Label,
        NineSlice, Panel, ParticleEmitter, PointLight, ShaderMaterial, Slider, Sprite, TextInput,
        Tilemap, Transform, UiNode,
    };
    if world.get::<PointLight>(e).is_some() {
        EntityKind::Light
    } else if world.get::<Tilemap>(e).is_some() {
        EntityKind::Tilemap
    } else if world.get::<ParticleEmitter>(e).is_some() {
        EntityKind::Particles
    } else if world.get::<CameraTarget>(e).is_some() {
        EntityKind::Camera
    } else if world.get::<AnimationPlayer>(e).is_some()
        || world.get::<AnimationStateMachine>(e).is_some()
    {
        EntityKind::Animation
    } else if world.get::<UiNode>(e).is_some()
        || world.get::<Button>(e).is_some()
        || world.get::<Label>(e).is_some()
        || world.get::<TextInput>(e).is_some()
        || world.get::<Slider>(e).is_some()
        || world.get::<CheckBox>(e).is_some()
        || world.get::<Panel>(e).is_some()
    {
        EntityKind::Ui
    } else if world.get::<Sprite>(e).is_some()
        || world.get::<AtlasSprite>(e).is_some()
        || world.get::<NineSlice>(e).is_some()
        || world.get::<ShaderMaterial>(e).is_some()
    {
        EntityKind::Sprite
    } else if world.get::<Transform>(e).is_some() {
        EntityKind::Transform
    } else {
        EntityKind::Bare
    }
}

/// A small per-row glyph hinting at an entity's kind — a light 💡 vs a sprite 🖼 vs a tilemap 🗺 vs a
/// UI widget 🔘 — drawn before the label in the Entities list and the Scene tree so an entity's type
/// is legible at a glance. Thin wrapper over [`entity_kind`] + [`EntityKind::icon`].
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn entity_type_icon(world: &crate::World, e: Entity) -> &'static str {
    entity_kind(world, e).icon()
}

/// A display-only ordering of `entity_list` for the Entities tab. Sorts a **copy** — the world's
/// entity order and the scene-save order are untouched. `Insertion` returns the raw order unchanged;
/// `Name` sorts case-insensitively by label; `Kind` groups by [`EntityKind`] (variant order) then by
/// name. The sort is stable, so equal keys keep their insertion order.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn sorted_entity_list(
    entity_list: &[Entity],
    mode: EntitySortMode,
    world: &crate::World,
    tag_map: &HashMap<Entity, String>,
) -> Vec<Entity> {
    let mut v = entity_list.to_vec();
    let name_key = |e: Entity| entity_label(e, tag_map).to_lowercase();
    // `sort_by_key` is stable, so within a sort group equal keys keep their insertion order.
    match mode {
        EntitySortMode::Insertion => {}
        EntitySortMode::Name => v.sort_by_key(|&e| name_key(e)),
        EntitySortMode::Kind => v.sort_by_key(|&e| (entity_kind(world, e), name_key(e))),
    }
    v
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod icon_tests {
    use super::{entity_type_icon, sorted_entity_list};
    use crate::app::editor::EntitySortMode;
    use crate::prefab::Tag;
    use crate::{App, CameraTarget, Entity, PointLight, Sprite, Transform};
    use std::collections::HashMap;

    /// Build the `entity -> label` map the editor derives from `Tag`s (what `entity_label` reads).
    fn tag_map(app: &App, ents: &[Entity]) -> HashMap<Entity, String> {
        ents.iter()
            .filter_map(|&e| app.world.get::<Tag>(e).map(|t| (e, t.0.clone())))
            .collect()
    }

    #[test]
    fn bare_entity_gets_the_generic_dot() {
        let mut app = App::new();
        let e = app.world.spawn();
        assert_eq!(entity_type_icon(&app.world, e), "·");
    }

    #[test]
    fn transform_only_entity_gets_the_diamond() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        assert_eq!(entity_type_icon(&app.world, e), "🔹");
    }

    #[test]
    fn a_sprite_entity_gets_the_picture_glyph() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        app.world.add_component(e, Sprite::colored(0.5, 0.5, 0.5));
        assert_eq!(entity_type_icon(&app.world, e), "🖼");
    }

    #[test]
    fn a_light_entity_gets_the_bulb_glyph() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        app.world.add_component(e, PointLight::default());
        assert_eq!(entity_type_icon(&app.world, e), "💡");
    }

    #[test]
    fn a_camera_rig_gets_the_camera_glyph() {
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, CameraTarget);
        assert_eq!(entity_type_icon(&app.world, e), "🎥");
    }

    #[test]
    fn priority_a_light_that_also_has_a_sprite_still_reads_as_a_light() {
        // Priority order: PointLight is checked before Sprite, so the more specific "kind" wins even
        // when both components are present.
        let mut app = App::new();
        let e = app.world.spawn();
        app.world.add_component(e, Transform::default());
        app.world.add_component(e, Sprite::colored(1.0, 1.0, 1.0));
        app.world.add_component(e, PointLight::default());
        assert_eq!(entity_type_icon(&app.world, e), "💡");
    }

    #[test]
    fn sort_insertion_preserves_raw_order() {
        let mut app = App::new();
        let ents: Vec<Entity> = ["Zebra", "apple", "Mango"]
            .iter()
            .map(|n| {
                let e = app.world.spawn();
                app.world.add_component(e, Tag((*n).into()));
                e
            })
            .collect();
        let tm = tag_map(&app, &ents);
        let out = sorted_entity_list(&ents, EntitySortMode::Insertion, &app.world, &tm);
        assert_eq!(out, ents, "Default sort is the raw entity_list order");
    }

    #[test]
    fn sort_name_is_case_insensitive_alphabetical() {
        let mut app = App::new();
        let ents: Vec<Entity> = ["Zebra", "apple", "Mango"]
            .iter()
            .map(|n| {
                let e = app.world.spawn();
                app.world.add_component(e, Tag((*n).into()));
                e
            })
            .collect();
        let tm = tag_map(&app, &ents);
        let out = sorted_entity_list(&ents, EntitySortMode::Name, &app.world, &tm);
        let labels: Vec<&str> = out.iter().map(|e| tm[e].as_str()).collect();
        assert_eq!(labels, ["apple", "Mango", "Zebra"], "case-insensitive A–Z");
    }

    #[test]
    fn sort_kind_groups_by_entity_kind_then_name() {
        // Kinds rank Light < Sprite < Transform < Bare (the EntityKind variant order); within a kind,
        // ties fall back to case-insensitive name.
        let mut app = App::new();
        let bare = app.world.spawn();
        app.world.add_component(bare, Tag("bare".into()));
        let sprite = app.world.spawn();
        app.world.add_component(sprite, Tag("sprite".into()));
        app.world
            .add_component(sprite, Sprite::colored(0.5, 0.5, 0.5));
        let light = app.world.spawn();
        app.world.add_component(light, Tag("light".into()));
        app.world.add_component(light, PointLight::default());
        let xform = app.world.spawn();
        app.world.add_component(xform, Tag("xform".into()));
        app.world.add_component(xform, Transform::default());

        let ents = vec![bare, sprite, light, xform];
        let tm = tag_map(&app, &ents);
        let out = sorted_entity_list(&ents, EntitySortMode::Kind, &app.world, &tm);
        assert_eq!(
            out,
            vec![light, sprite, xform, bare],
            "grouped Light → Sprite → Transform → Bare"
        );
    }
}
