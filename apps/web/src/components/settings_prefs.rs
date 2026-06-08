//! plan_ref:
//!   - 15_settings#browser-ui-prefs
//!
//! Browser-local Settings preferences. These values are UI hints only and must
//! not carry repo authority, secrets, sync vectors, or business facts.

use crate::storage::prefs::{read_bool_pref, read_pref, write_bool_pref, write_pref};

const THEME_PREF_KEY: &str = "deve.ui.theme";
const EDITOR_WRAP_PREF_KEY: &str = "deve.editor.word_wrap";
const EDITOR_DENSITY_PREF_KEY: &str = "deve.editor.density";
pub(crate) const AI_CHAT_VISIBLE_PREF_KEY: &str = "deve.ui.ai_chat_visible";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ThemePreference {
    Auto,
    Light,
    Dark,
}

impl ThemePreference {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
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
        .unwrap_or(ThemePreference::Auto)
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

#[cfg(target_arch = "wasm32")]
fn apply_theme_preference_to_document(pref: ThemePreference) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(root) = document.document_element() else {
        return;
    };

    let _ = root.set_attribute("data-deve-theme-pref", pref.as_str());
    let dark = matches!(pref, ThemePreference::Dark);

    if dark {
        let _ = root.class_list().add_1("dark");
    } else {
        let _ = root.class_list().remove_1("dark");
    }
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
    fn theme_preference_defaults_to_auto_and_roundtrips() {
        remove_pref(THEME_PREF_KEY);
        assert_eq!(read_theme_preference(), ThemePreference::Auto);

        persist_theme_preference(ThemePreference::Dark);
        assert_eq!(read_theme_preference(), ThemePreference::Dark);

        write_pref(THEME_PREF_KEY, "unknown").expect("write invalid theme");
        assert_eq!(read_theme_preference(), ThemePreference::Auto);
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
}
