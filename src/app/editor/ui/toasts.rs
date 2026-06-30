//! Editor action toasts — transient bottom-right feedback popups ("Scene saved", "3 deleted", …).
//!
//! Editor actions (save / delete / duplicate / paste / prefab) currently write a status string into
//! a panel where it's easy to miss. A toast surfaces the same outcome prominently for a couple of
//! seconds, then fades out. Pushed via [`App::push_editor_toast`] (internal, picks a [`ToastKind`])
//! or the public [`App::editor_toast`]; aged + drawn each frame by [`App::draw_editor_toasts`].

#![cfg(not(target_arch = "wasm32"))]

use super::*;
use crate::app::editor::state::{Toast, ToastKind, DEFAULT_TOAST_TTL};

impl App {
    /// Push an action-feedback toast (bottom-right popup, auto-expires after [`DEFAULT_TOAST_TTL`]).
    /// Internal editor actions use this to pick a [`ToastKind`]; the queue is capped so a burst of
    /// actions can't grow it unbounded.
    pub(in crate::app) fn push_editor_toast(
        &mut self,
        message: impl Into<String>,
        kind: ToastKind,
    ) {
        self.editor.toasts.push(Toast {
            message: message.into(),
            kind,
            age: 0.0,
            ttl: DEFAULT_TOAST_TTL,
        });
        const MAX_TOASTS: usize = 5;
        let overflow = self.editor.toasts.len().saturating_sub(MAX_TOASTS);
        if overflow > 0 {
            self.editor.toasts.drain(0..overflow);
        }
    }

    /// Show a neutral (info) action-feedback toast in the editor (bottom-right, auto-expires). Public
    /// so a game or a headless capture can surface a message; editor actions use richer kinds.
    pub fn editor_toast(&mut self, message: impl Into<String>) {
        self.push_editor_toast(message, ToastKind::Info);
    }

    /// Show a **success** (green) editor toast.
    pub fn editor_toast_success(&mut self, message: impl Into<String>) {
        self.push_editor_toast(message, ToastKind::Success);
    }

    /// Show an **error** (red) editor toast.
    pub fn editor_toast_error(&mut self, message: impl Into<String>) {
        self.push_editor_toast(message, ToastKind::Error);
    }

    /// Age the active toasts by `dt`, drop the expired ones, and draw the rest as a bottom-right
    /// stack (each fading out over its last ~0.6 s). Called each frame from the editor UI build, so
    /// toasts show in both overlay and docked modes.
    pub(in crate::app) fn draw_editor_toasts(&mut self, ctx: &egui::Context, dt: f32) {
        for t in &mut self.editor.toasts {
            t.age += dt;
        }
        self.editor.toasts.retain(|t| t.age < t.ttl);
        if self.editor.toasts.is_empty() {
            return;
        }

        let toasts = &self.editor.toasts;
        egui::Area::new(egui::Id::new("editor_action_toasts"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -12.0))
            .interactable(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
                    for t in toasts {
                        // Fade alpha over the last 0.6 s of the toast's life.
                        let fade = ((t.ttl - t.age) / 0.6).clamp(0.0, 1.0);
                        let a = (fade * 235.0) as u8;
                        let (fill, fg) = toast_colors(t.kind, a);
                        egui::Frame::NONE
                            .fill(fill)
                            .corner_radius(egui::CornerRadius::same(6))
                            .inner_margin(egui::Margin::symmetric(10, 6))
                            .show(ui, |ui| {
                                ui.colored_label(fg, &t.message);
                            });
                        ui.add_space(6.0);
                    }
                });
            });
    }
}

/// `(fill, text)` colours for a toast of `kind` at alpha `a` (so it can fade out).
fn toast_colors(kind: ToastKind, a: u8) -> (egui::Color32, egui::Color32) {
    let fg = egui::Color32::from_rgba_unmultiplied(245, 245, 245, a);
    let fill = match kind {
        ToastKind::Info => egui::Color32::from_rgba_unmultiplied(48, 52, 64, a),
        ToastKind::Success => egui::Color32::from_rgba_unmultiplied(34, 84, 54, a),
        ToastKind::Error => egui::Color32::from_rgba_unmultiplied(112, 44, 44, a),
    };
    (fill, fg)
}
