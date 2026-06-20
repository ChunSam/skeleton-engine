//! Audio mixer panel — native only.
//!
//! Renders the "Audio" tab of the bottom editor panel: one volume slider per audio
//! bus (as reported by [`AudioManager::bus_names`]). Dragging a slider applies the new
//! value live via [`AudioManager::set_bus_volume`].
//!
//! Borrow strategy: the [`AudioManager`] lives in `app.world`. We snapshot the bus
//! names + current volumes under an immutable borrow, render the sliders against local
//! copies, then re-borrow mutably to apply any changes — the same collect-then-apply
//! pattern used by the data-table panel.

#![cfg(not(target_arch = "wasm32"))]

use crate::app::editor::tr;
use crate::app::App;
use crate::audio::AudioManager;

/// Render the Audio mixer panel body into `ui`.
///
/// Called from the docked bottom panel when the bottom tab is set to 2.
pub(in crate::app) fn audio_mixer_panel_body(ui: &mut egui::Ui, app: &mut App) {
    // Snapshot (name, current volume) under an immutable borrow, then drop it.
    let buses: Vec<(String, f32)> = match app.world.resource::<AudioManager>() {
        Some(audio) => audio
            .bus_names()
            .into_iter()
            .map(|name| {
                let vol = audio.bus_volume(&name);
                (name, vol)
            })
            .collect(),
        None => {
            ui.label(tr(
                "No AudioManager — the game has not initialized audio.",
                "AudioManager 없음 — 게임이 오디오를 초기화하지 않았습니다.",
            ));
            return;
        }
    };

    if buses.is_empty() {
        ui.label(tr(
            "No audio buses. Assign one with AudioManager::assign_bus(channel, bus).",
            "오디오 버스 없음. AudioManager::assign_bus(channel, bus)로 할당하세요.",
        ));
        return;
    }

    ui.strong(tr("Bus volumes", "버스 볼륨"));
    ui.separator();

    // Render sliders against local copies; collect any changed values.
    let mut changes: Vec<(String, f32)> = Vec::new();
    for (name, vol) in &buses {
        let mut v = *vol;
        ui.horizontal(|ui| {
            ui.label(name);
            if ui
                .add(egui::Slider::new(&mut v, 0.0..=1.0).fixed_decimals(2))
                .changed()
            {
                changes.push((name.clone(), v));
            }
        });
    }

    // Apply changes under a fresh mutable borrow (the snapshot borrow is long gone).
    if !changes.is_empty() {
        if let Some(audio) = app.world.resource_mut::<AudioManager>() {
            for (name, v) in changes {
                audio.set_bus_volume(&name, v);
            }
        }
    }
}
