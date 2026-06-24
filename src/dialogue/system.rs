//! [`DialogueSystem`] — ticks every [`DialogueBox`]'s typewriter and renders the active box
//! (speaker + revealed text + choices/hint + optional portrait) as screen-space UI.

use crate::asset::{Handle, ImageAsset};
use crate::ecs::{System, World};
use crate::locale::LocaleResource;
use crate::renderer::{DrawImage, DrawText, TextQueue, UiImageQueue};
use crate::resources::ViewportSize;

use super::{DialogueBox, DialogueStyle, DialogueVars};

/// What [`DialogueSystem`] gathers per active box before drawing — pulled out of the query borrow
/// so the text + image queues can be written afterward.
struct DrawItem {
    speaker: String,
    body: String,
    full: bool,
    choices: Vec<String>,
    portrait: Option<Handle<ImageAsset>>,
}

/// Ticks every [`DialogueBox`]'s typewriter and renders the active box (speaker + revealed text +
/// an advance hint) as screen-space text near the bottom of the viewport. When a box sets a
/// [`portrait`](DialogueBox::portrait) (e.g. via [`with_portrait`](DialogueBox::with_portrait)),
/// the speaker's image is drawn to the left through the UI image queue and the text shifts right
/// to clear it.
///
/// Input-agnostic: the game advances a box by calling [`DialogueBox::advance`]. Rendering is
/// text + optional portrait (no background panel) so it composes with whatever box art the game
/// draws.
pub struct DialogueSystem;

impl System for DialogueSystem {
    fn run(&mut self, world: &mut World, dt: f32) {
        // 0. Resolve localized boxes against the current locale before ticking, so a live
        //    `set_locale` retranslates them. `resolve` needs both `&mut DialogueBox` (query_mut)
        //    and `&LocaleResource`, so clone the resource first to release the world borrow. Only
        //    runs when a `LocaleResource` exists, and `resolve` is a no-op for literal boxes.
        if let Some(locale) = world.resource::<LocaleResource>().cloned() {
            for (_e, d) in world.query_mut::<DialogueBox>() {
                d.resolve(&locale);
            }
        }

        // 1. Advance every dialogue box's typewriter (Phase-3 mutable query — no collect).
        for (_e, d) in world.query_mut::<DialogueBox>() {
            d.tick(dt);
        }

        // 2. Gather what to draw for active boxes (releases the query borrow).
        // Style is a game-overridable resource; absent, `default()` reproduces the original look.
        // Cloned once (it owns the hint label) to release the resource borrow before the queries.
        let style = world
            .resource::<DialogueStyle>()
            .cloned()
            .unwrap_or_default();
        let (vw, vh) = world
            .resource::<ViewportSize>()
            .map(|v| (v.width, v.height))
            .unwrap_or((style.fallback_viewport.x, style.fallback_viewport.y));
        // Clone DialogueVars (or empty) to filter conditional choices without holding a
        // resource borrow across the query below.
        let vars = world
            .resource::<DialogueVars>()
            .cloned()
            .unwrap_or_default();
        let items: Vec<DrawItem> = world
            .query::<DialogueBox>()
            .filter(|(_, d)| !d.is_finished())
            .map(|(_, d)| DrawItem {
                speaker: d.speaker.clone(),
                body: d.visible_text().to_string(),
                full: d.line_fully_revealed(),
                // Visible (cond-passing) choice labels, already resolved into `text` by step 0.
                choices: d
                    .visible_choices(&vars)
                    .iter()
                    .map(|c| c.text.clone())
                    .collect(),
                portrait: d.portrait.clone(),
            })
            .collect();
        if items.is_empty() {
            return;
        }

        // 3. Render near the bottom of the screen, driven by `style`.
        //
        // A box with a `portrait` draws a square image on the left and shifts its text right to
        // clear it; a box without one keeps the no-portrait left margin.
        let text_x = |has_portrait: bool| {
            if has_portrait {
                style.portrait_x + style.portrait_size + style.portrait_text_gap
            } else {
                style.text_margin
            }
        };

        // 3a. Portraits — screen-space images to the left of the text, drawn through the same UI
        //     image queue the renderer presents over the scene. Borrow `&items` here; the text
        //     loop below consumes `items`.
        if let Some(iq) = world.resource_mut::<UiImageQueue>() {
            for item in &items {
                if let Some(handle) = &item.portrait {
                    iq.push(DrawImage::with_handle(
                        style.portrait_x,
                        vh - style.portrait_bottom_offset,
                        style.portrait_size,
                        style.portrait_size,
                        handle.clone(),
                    ));
                }
            }
        }

        // 3b. Speaker + body + choices/hint text.
        if let Some(tq) = world.resource_mut::<TextQueue>() {
            for item in items {
                let DrawItem {
                    speaker,
                    body,
                    full,
                    choices,
                    portrait,
                } = item;
                let x0 = text_x(portrait.is_some());
                if !speaker.is_empty() {
                    tq.push(DrawText::new(
                        speaker,
                        crate::Vec2::new(x0, vh - style.speaker_bottom_offset),
                        style.speaker_font_size,
                        style.speaker_color,
                    ));
                }
                tq.push(
                    DrawText::new(
                        body,
                        crate::Vec2::new(x0, vh - style.body_bottom_offset),
                        style.body_font_size,
                        style.body_color,
                    )
                    .with_bounds(crate::Vec2::new(vw - 2.0 * x0, style.body_height)),
                );
                if !choices.is_empty() {
                    // A decision is pending → numbered choice list (replaces the advance hint).
                    let mut y = vh - style.choice_bottom_offset;
                    for (i, label) in choices.iter().enumerate() {
                        tq.push(DrawText::new(
                            format!("{}. {}", i + 1, label),
                            crate::Vec2::new(x0 + style.choice_indent, y),
                            style.choice_font_size,
                            style.choice_color,
                        ));
                        y += style.choice_line_step;
                    }
                } else if full {
                    tq.push(DrawText::new(
                        style.hint_label.clone(),
                        crate::Vec2::new(
                            vw - style.hint_right_offset,
                            vh - style.hint_bottom_offset,
                        ),
                        style.hint_font_size,
                        style.hint_color,
                    ));
                }
            }
        }
    }

    fn name(&self) -> &'static str {
        "dialogue_system"
    }
}
