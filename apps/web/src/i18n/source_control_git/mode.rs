//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!   - 14_commands#command-palette-shortcuts

use crate::i18n::Locale;

pub fn source_control_authority_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "NoteGit/ngit authority; Git main is a terminal-state mirror",
        Locale::Zh => "NoteGit/ngit 是 authority；Git main 只是终态 mirror",
    }
}

pub fn source_control_authority_badge(locale: Locale, authority: &str) -> String {
    match (locale, authority) {
        (Locale::Zh, "ngit") => "ngit authority".to_string(),
        (Locale::En, "ngit") => "ngit authority".to_string(),
        (Locale::Zh, _) => "ngit unknown".to_string(),
        (Locale::En, _) => "ngit unknown".to_string(),
    }
}
