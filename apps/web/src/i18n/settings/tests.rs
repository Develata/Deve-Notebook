use super::{
    chinese_language_label, coming_soon, current_boundary_desc, english_language_label,
    native_backend_unavailable, remote_backend_requires_validation,
};
use crate::i18n::Locale;

#[test]
fn boundary_copy_mentions_config_toml_and_cli_set() {
    for locale in [Locale::En, Locale::Zh] {
        let text = current_boundary_desc(locale);
        assert!(text.contains("config.toml"));
        assert!(text.contains("deve config set"));
    }
}

#[test]
fn language_buttons_use_self_labels() {
    assert_eq!(english_language_label(), "English");
    assert_eq!(chinese_language_label(), "中文");
}

#[test]
fn reserved_setting_copy_marks_future_boundary() {
    assert!(coming_soon(Locale::En).contains("Future setting"));
    assert!(coming_soon(Locale::Zh).contains("未来设置"));
}

#[test]
fn native_backend_copy_marks_native_only_and_validation_boundary() {
    assert!(native_backend_unavailable(Locale::En).contains("native menu"));
    assert!(native_backend_unavailable(Locale::Zh).contains("原生菜单"));
    assert!(remote_backend_requires_validation(Locale::En).contains("validation"));
    assert!(remote_backend_requires_validation(Locale::Zh).contains("校验"));
}
