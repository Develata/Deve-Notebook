use crate::i18n::{Locale, t};

pub fn compare_message(locale: Locale, base_label: &str, target_label: &str) -> String {
    t::source_control::history_compare_message(locale, base_label, target_label)
}

pub fn base_selected_message(locale: Locale, base_label: &str) -> String {
    t::source_control::history_base_selected_message(locale, base_label)
}

pub fn selected_target_message(locale: Locale, target_label: &str) -> String {
    t::source_control::history_selected_target_message(locale, target_label)
}

pub fn clear_label(locale: Locale) -> &'static str {
    t::source_control::clear_label(locale)
}

pub fn use_as_base_label(locale: Locale) -> &'static str {
    t::source_control::use_as_base_label(locale)
}
