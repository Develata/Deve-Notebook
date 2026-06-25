//! plan_ref:
//!   - 15_settings#browser-ui-prefs
//!
//! Browser-local Settings preferences. These values are UI hints only and must
//! not carry repo authority, secrets, sync vectors, or business facts.

use crate::storage::prefs::{
    read_bool_pref, read_i32_pref, read_pref, write_bool_pref, write_i32_pref, write_pref,
};

const THEME_PREF_KEY: &str = "deve.ui.theme";
const EDITOR_WRAP_PREF_KEY: &str = "deve.editor.word_wrap";
const EDITOR_DENSITY_PREF_KEY: &str = "deve.editor.density";
pub(crate) const AI_CHAT_VISIBLE_PREF_KEY: &str = "deve.ui.ai_chat_visible";
pub(crate) const MAX_DOCUMENT_TABS_PREF_KEY: &str = "deve.ui.max_document_tabs";
pub(crate) const DEFAULT_MAX_DOCUMENT_TABS: usize = 8;
pub(crate) const MIN_MAX_DOCUMENT_TABS: usize = 1;
pub(crate) const MAX_MAX_DOCUMENT_TABS: usize = 20;

/// Browser-local visual style. Three flat named styles (15_settings §2.1):
/// `warm` (default) / `cold` / `night`. Supersedes the legacy `auto/light/dark`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ThemePreference {
    Warm,
    Cold,
    Night,
}

impl ThemePreference {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Warm => "warm",
            Self::Cold => "cold",
            Self::Night => "night",
        }
    }

    /// Parse a stored value, migrating legacy markers: `dark -> night`,
    /// `light`/`auto -> warm`. Unknown values return `None` so the caller
    /// falls back to the default (`warm`).
    fn parse(value: &str) -> Option<Self> {
        match value {
            "warm" => Some(Self::Warm),
            "cold" => Some(Self::Cold),
            "night" => Some(Self::Night),
            "dark" => Some(Self::Night),
            "light" | "auto" => Some(Self::Warm),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorWrapPreference {
    On,
    Off,
}

impl EditorWrapPreference {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::On => "on",
            Self::Off => "off",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "on" => Some(Self::On),
            "off" => Some(Self::Off),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EditorDensityPreference {
    Comfortable,
    Compact,
}

impl EditorDensityPreference {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Comfortable => "comfortable",
            Self::Compact => "compact",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "comfortable" => Some(Self::Comfortable),
            "compact" => Some(Self::Compact),
            _ => None,
        }
    }
}

pub(super) fn read_theme_preference() -> ThemePreference {
    read_pref(THEME_PREF_KEY)
        .as_deref()
        .and_then(ThemePreference::parse)
        .unwrap_or(ThemePreference::Warm)
}

/// Apply the persisted visual style on app boot so a saved theme survives a
/// reload even before the Settings panel mounts. The inline bootstrap in
/// `index.html` sets the marker pre-paint; this keeps the Rust pref authoritative.
pub(crate) fn apply_persisted_theme() {
    apply_theme_preference(read_theme_preference());
}

pub(super) fn persist_theme_preference(pref: ThemePreference) {
    let _ = write_pref(THEME_PREF_KEY, pref.as_str());
    apply_theme_preference(pref);
}

pub(super) fn apply_theme_preference(pref: ThemePreference) {
    apply_theme_preference_to_document(pref);
}

pub(super) fn read_editor_wrap_preference() -> EditorWrapPreference {
    read_pref(EDITOR_WRAP_PREF_KEY)
        .as_deref()
        .and_then(EditorWrapPreference::parse)
        .unwrap_or(EditorWrapPreference::On)
}

pub(super) fn persist_editor_wrap_preference(pref: EditorWrapPreference) {
    let _ = write_pref(EDITOR_WRAP_PREF_KEY, pref.as_str());
    apply_editor_wrap_preference(pref);
}

pub(super) fn apply_editor_wrap_preference(pref: EditorWrapPreference) {
    set_root_attr("data-deve-editor-wrap", pref.as_str());
}

pub(super) fn read_editor_density_preference() -> EditorDensityPreference {
    read_pref(EDITOR_DENSITY_PREF_KEY)
        .as_deref()
        .and_then(EditorDensityPreference::parse)
        .unwrap_or(EditorDensityPreference::Comfortable)
}

pub(super) fn persist_editor_density_preference(pref: EditorDensityPreference) {
    let _ = write_pref(EDITOR_DENSITY_PREF_KEY, pref.as_str());
    apply_editor_density_preference(pref);
}

pub(super) fn apply_editor_density_preference(pref: EditorDensityPreference) {
    set_root_attr("data-deve-editor-density", pref.as_str());
}

pub(crate) fn read_ai_chat_visible_preference() -> bool {
    read_bool_pref(AI_CHAT_VISIBLE_PREF_KEY).unwrap_or(true)
}

pub(crate) fn persist_ai_chat_visible_preference(visible: bool) {
    let _ = write_bool_pref(AI_CHAT_VISIBLE_PREF_KEY, visible);
}

pub(crate) fn clamp_max_document_tabs(value: usize) -> usize {
    value.clamp(MIN_MAX_DOCUMENT_TABS, MAX_MAX_DOCUMENT_TABS)
}

pub(crate) fn read_max_document_tabs_preference() -> usize {
    read_i32_pref(MAX_DOCUMENT_TABS_PREF_KEY)
        .and_then(|value| usize::try_from(value).ok())
        .filter(|value| (MIN_MAX_DOCUMENT_TABS..=MAX_MAX_DOCUMENT_TABS).contains(value))
        .unwrap_or(DEFAULT_MAX_DOCUMENT_TABS)
}

pub(crate) fn persist_max_document_tabs_preference(value: usize) {
    let _ = write_i32_pref(
        MAX_DOCUMENT_TABS_PREF_KEY,
        clamp_max_document_tabs(value) as i32,
    );
}

#[cfg(target_arch = "wasm32")]
fn apply_theme_preference_to_document(pref: ThemePreference) {
    // The `data-deve-theme-pref` marker is the single selector that drives all
    // three token blocks (_variables.css `:root` warm default,
    // _variables-cold.css, _variables-night.css). No `.dark` class is involved.
    set_root_attr("data-deve-theme-pref", pref.as_str());
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_theme_preference_to_document(_pref: ThemePreference) {}

#[cfg(target_arch = "wasm32")]
fn set_root_attr(name: &str, value: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(root) = document.document_element() else {
        return;
    };
    let _ = root.set_attribute(name, value);
}

#[cfg(not(target_arch = "wasm32"))]
fn set_root_attr(_name: &str, _value: &str) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::prefs::{remove_pref, write_pref};

    #[test]
    fn theme_preference_defaults_to_warm_and_roundtrips() {
        remove_pref(THEME_PREF_KEY);
        assert_eq!(read_theme_preference(), ThemePreference::Warm);

        persist_theme_preference(ThemePreference::Night);
        assert_eq!(read_theme_preference(), ThemePreference::Night);

        persist_theme_preference(ThemePreference::Cold);
        assert_eq!(read_theme_preference(), ThemePreference::Cold);

        // Legacy markers migrate: dark -> night, light/auto -> warm.
        write_pref(THEME_PREF_KEY, "dark").expect("write legacy dark");
        assert_eq!(read_theme_preference(), ThemePreference::Night);
        write_pref(THEME_PREF_KEY, "light").expect("write legacy light");
        assert_eq!(read_theme_preference(), ThemePreference::Warm);
        write_pref(THEME_PREF_KEY, "auto").expect("write legacy auto");
        assert_eq!(read_theme_preference(), ThemePreference::Warm);

        write_pref(THEME_PREF_KEY, "unknown").expect("write invalid theme");
        assert_eq!(read_theme_preference(), ThemePreference::Warm);
    }

    #[test]
    fn editor_preferences_default_to_safe_values_and_roundtrip() {
        remove_pref(EDITOR_WRAP_PREF_KEY);
        remove_pref(EDITOR_DENSITY_PREF_KEY);

        assert_eq!(read_editor_wrap_preference(), EditorWrapPreference::On);
        assert_eq!(
            read_editor_density_preference(),
            EditorDensityPreference::Comfortable
        );

        persist_editor_wrap_preference(EditorWrapPreference::Off);
        persist_editor_density_preference(EditorDensityPreference::Compact);

        assert_eq!(read_editor_wrap_preference(), EditorWrapPreference::Off);
        assert_eq!(
            read_editor_density_preference(),
            EditorDensityPreference::Compact
        );
    }

    #[test]
    fn ai_chat_visibility_preference_defaults_visible_and_roundtrips() {
        remove_pref(AI_CHAT_VISIBLE_PREF_KEY);
        assert!(read_ai_chat_visible_preference());

        persist_ai_chat_visible_preference(false);
        assert!(!read_ai_chat_visible_preference());

        persist_ai_chat_visible_preference(true);
        assert!(read_ai_chat_visible_preference());
    }

    #[test]
    fn max_document_tabs_preference_defaults_clamps_and_roundtrips() {
        remove_pref(MAX_DOCUMENT_TABS_PREF_KEY);
        assert_eq!(
            read_max_document_tabs_preference(),
            DEFAULT_MAX_DOCUMENT_TABS
        );

        persist_max_document_tabs_preference(12);
        assert_eq!(read_max_document_tabs_preference(), 12);

        persist_max_document_tabs_preference(0);
        assert_eq!(read_max_document_tabs_preference(), MIN_MAX_DOCUMENT_TABS);

        persist_max_document_tabs_preference(99);
        assert_eq!(read_max_document_tabs_preference(), MAX_MAX_DOCUMENT_TABS);

        write_pref(MAX_DOCUMENT_TABS_PREF_KEY, "invalid").expect("write invalid tab limit");
        assert_eq!(
            read_max_document_tabs_preference(),
            DEFAULT_MAX_DOCUMENT_TABS
        );
    }
}
