//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn git_bridge_mode_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "NoteGit authority; Git is an optional bridge",
        Locale::Zh => "NoteGit 是 authority；Git 是可选 bridge",
    }
}

pub fn git_bridge_mode_badge(_locale: Locale, mode: &str) -> String {
    match mode {
        "mirror" => "NoteGit + Git mirror".to_string(),
        "off" => "NoteGit only".to_string(),
        "unknown" => "NoteGit + Git unknown".to_string(),
        _ => "NoteGit + Git unknown".to_string(),
    }
}
