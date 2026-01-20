// apps\web\src\i18n
//! # I18n Playback Module (回放翻译)

use super::Locale;

pub fn label(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "PLAYBACK",
        Locale::Zh => "回放控制",
    }
}
