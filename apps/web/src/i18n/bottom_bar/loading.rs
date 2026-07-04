//! plan_ref:
//!   - 13_i18n#i18n-keys-reference
//!
//! Bottom bar loading progress copy.

use super::super::Locale;

pub fn loading(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Loading...",
        Locale::Zh => "加载中...",
    }
}

pub fn loading_progress(locale: Locale, done: usize, total: usize, eta_ms: u64) -> String {
    match locale {
        Locale::En => {
            if eta_ms > 0 {
                format!("Loading {}/{} (~{}ms)", done, total, eta_ms)
            } else {
                format!("Loading {}/{}", done, total)
            }
        }
        Locale::Zh => {
            if eta_ms > 0 {
                format!("加载中 {}/{} (~{}ms)", done, total, eta_ms)
            } else {
                format!("加载中 {}/{}", done, total)
            }
        }
    }
}

pub fn loading_progress_compact(locale: Locale, done: usize, total: usize) -> String {
    match locale {
        Locale::En => format!("Load {done}/{total}"),
        Locale::Zh => format!("加载 {done}/{total}"),
    }
}
