//! Lightweight editor localization.
//!
//! The docked / overlay editor is egui (immediate mode), so UI strings are translated at the call
//! site with [`tr`]: `ui.button(tr("Pause", "일시정지"))`. **English is the source of truth** — it is
//! the first argument and the fallback, so an untranslated call (`tr("X", "X")`) still renders
//! correctly. The Korean translation lives inline next to the English, which keeps the diff local
//! (no central catalog to keep in sync) and lets a forker read the English at a glance.
//!
//! The active [`EditorLocale`] is a thread-local set once per frame from `EditorState::locale`
//! (see [`set_locale`]); `tr` reads it with no extra plumbing through the deeply-nested egui
//! closures. The editor runs on a single (UI) thread, so a thread-local is sufficient.
//!
//! ⚠️ **That in-memory field is not the persisted setting until the editor has been Docked once.**
//! `editor_settings.ron` is read on exactly one transition — the first Off/Overlay→Docked — so an
//! F1 overlay session runs on `EditorState::new`'s default (Korean) no matter what the file says.
//! Docking once and leaving loads and then saves it. Making Overlay honour the file is open work.

// `set_locale` / `EditorLocale::{label, toggled}` are only used by the native editor toolbar; on
// wasm only `tr` (the shared overlay path) is exercised, so they'd be flagged dead there.
#![cfg_attr(target_arch = "wasm32", allow(dead_code))]

use std::cell::Cell;

/// Editor UI language. Defaults to **Korean** (the engine author's working language); a forker can
/// flip it to English from the editor settings panel (the choice persists in `editor_settings.ron`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub(in crate::app) enum EditorLocale {
    English,
    #[default]
    Korean,
}

impl EditorLocale {
    /// Short label for the locale toggle button.
    pub(in crate::app) fn label(self) -> &'static str {
        match self {
            EditorLocale::English => "EN",
            EditorLocale::Korean => "한국어",
        }
    }

    /// The other locale (for a single-button toggle).
    pub(in crate::app) fn toggled(self) -> EditorLocale {
        match self {
            EditorLocale::English => EditorLocale::Korean,
            EditorLocale::Korean => EditorLocale::English,
        }
    }
}

thread_local! {
    static ACTIVE: Cell<EditorLocale> = const { Cell::new(EditorLocale::Korean) };
}

/// Sets the active editor locale for the current (UI) thread. Call once at the top of the editor
/// UI build each frame from `EditorState::locale` so every [`tr`] in that frame agrees — see the
/// module doc for why that field is not the persisted setting until the editor has been Docked.
pub(in crate::app) fn set_locale(locale: EditorLocale) {
    ACTIVE.with(|a| a.set(locale));
}

/// The active editor locale on this thread.
pub(in crate::app) fn locale() -> EditorLocale {
    ACTIVE.with(|a| a.get())
}

/// Localized string: returns `ko` when the active locale is Korean, otherwise the English `en`.
pub(in crate::app) fn tr(en: &'static str, ko: &'static str) -> &'static str {
    match locale() {
        EditorLocale::Korean => ko,
        EditorLocale::English => en,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tr_follows_active_locale() {
        set_locale(EditorLocale::English);
        assert_eq!(tr("Pause", "일시정지"), "Pause");
        set_locale(EditorLocale::Korean);
        assert_eq!(tr("Pause", "일시정지"), "일시정지");
    }

    #[test]
    fn default_locale_is_korean() {
        assert_eq!(EditorLocale::default(), EditorLocale::Korean);
    }

    #[test]
    fn toggled_round_trips() {
        assert_eq!(EditorLocale::Korean.toggled(), EditorLocale::English);
        assert_eq!(
            EditorLocale::English.toggled().toggled(),
            EditorLocale::English
        );
    }
}
