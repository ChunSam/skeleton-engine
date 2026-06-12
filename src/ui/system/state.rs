use glam::Vec2;
use winit::event::MouseButton;
use winit::keyboard::KeyCode;

use crate::ecs::{Entity, Events, World};
use crate::input::InputState;
use crate::renderer::{DrawRect, DrawText, TextQueue, UiQueue};
use crate::resources::ViewportSize;
use crate::ui::node::UiNode;

use super::UiEvent;

pub(super) struct InputSnapshot {
    pub(super) cursor: Vec2,
    pub(super) just_pressed: bool,
    pub(super) just_released: bool,
    pub(super) is_held: bool,
    pub(super) scroll_delta: f32,
    pub(super) chars: Vec<char>,
    pub(super) ime_preedit: String,
    pub(super) press_cursor: Vec2,
    pub(super) release_cursor: Vec2,
    pub(super) nav_left: bool,
    pub(super) nav_right: bool,
    pub(super) nav_home: bool,
    pub(super) nav_end: bool,
    pub(super) nav_delete: bool,
}

impl InputSnapshot {
    pub(super) fn from_world(world: &World) -> Option<Self> {
        let input = world.resource::<InputState>()?;
        let cursor = input.cursor();

        // Hit-test clicks against the cursor at the press/release moment, not the
        // live cursor. Hover/drag still use `cursor`.
        Some(Self {
            cursor,
            just_pressed: input.mouse_just_pressed(MouseButton::Left),
            just_released: input.mouse_just_released(MouseButton::Left),
            is_held: input.is_mouse_pressed(MouseButton::Left),
            scroll_delta: input.scroll(),
            chars: input.text_chars().to_vec(),
            ime_preedit: input.ime_preedit().to_string(),
            press_cursor: input.mouse_press_cursor(MouseButton::Left),
            release_cursor: input.mouse_release_cursor(MouseButton::Left),
            nav_left: input.just_pressed(KeyCode::ArrowLeft),
            nav_right: input.just_pressed(KeyCode::ArrowRight),
            nav_home: input.just_pressed(KeyCode::Home),
            nav_end: input.just_pressed(KeyCode::End),
            nav_delete: input.just_pressed(KeyCode::Delete),
        })
    }
}

#[derive(Default)]
pub(super) struct UiOutput {
    pub(super) rects: Vec<DrawRect>,
    pub(super) texts: Vec<DrawText>,
    pub(super) events: Vec<UiEvent>,
}

pub(super) fn viewport_from_world(world: &World) -> Option<ViewportSize> {
    world.resource::<ViewportSize>().copied()
}

pub(super) fn submit_output(world: &mut World, output: UiOutput) {
    if let Some(ui_queue) = world.resource_mut::<UiQueue>() {
        for rect in output.rects {
            ui_queue.push(rect);
        }
    }
    if let Some(text_queue) = world.resource_mut::<TextQueue>() {
        for text in output.texts {
            text_queue.push(text);
        }
    }

    if !output.events.is_empty() {
        if let Some(events) = world.resource_mut::<Events<UiEvent>>() {
            for ev in output.events {
                events.send(ev);
            }
        }
    }
}

pub(super) fn in_bounds(cursor: Vec2, pos: Vec2, size: Vec2) -> bool {
    cursor.x >= pos.x
        && cursor.x <= pos.x + size.x
        && cursor.y >= pos.y
        && cursor.y <= pos.y + size.y
}

/// Reads the layout fields of the [`UiNode`] attached to `entity`.
///
/// Returns `Some((screen_pos, size, z, visible))` if the node exists,
/// or `None` if the entity has no `UiNode` component.
pub(super) fn node_layout(
    world: &World,
    entity: Entity,
    viewport: &ViewportSize,
) -> Option<(Vec2, Vec2, f32, bool)> {
    let node = world.get::<UiNode>(entity)?;
    Some((node.screen_pos(viewport), node.size, node.z, node.visible))
}
