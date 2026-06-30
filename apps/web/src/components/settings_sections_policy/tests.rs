use super::{
    BUTTON_CLASS_ACCENT_ACTIVE, BUTTON_CLASS_DISABLED, BUTTON_CLASS_IDLE, SYNC_AUTO_CLASS_ACTIVE,
    SYNC_MANUAL_CLASS_ACTIVE, ai_backend_button_state, ai_chat_visibility_button_state,
    editor_density_button_state, editor_wrap_button_state, language_button_state,
    native_backend_button_state, native_backend_can_switch_local,
    native_backend_unavailable_feedback, native_backend_validation_state, reserved_setting_state,
    sync_mode_button_state, theme_button_state,
};
use crate::api::{AI_BACKEND_NATIVE, AI_BACKEND_TRUSTED_CLI, AiBackendCapabilities};
use crate::components::settings_prefs::{
    EditorDensityPreference, EditorWrapPreference, ThemePreference,
};
use crate::i18n::{Locale, t};

#[test]
fn language_buttons_reflect_current_locale() {
    let english = language_button_state(Locale::En);
    assert_eq!(english.english_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert_eq!(english.chinese_class, BUTTON_CLASS_IDLE);

    let chinese = language_button_state(Locale::Zh);
    assert_eq!(chinese.english_class, BUTTON_CLASS_IDLE);
    assert_eq!(chinese.chinese_class, BUTTON_CLASS_ACCENT_ACTIVE);
}

#[test]
fn sync_mode_buttons_reflect_current_mode() {
    let auto = sync_mode_button_state("auto");
    assert_eq!(auto.auto_class, SYNC_AUTO_CLASS_ACTIVE);
    assert_eq!(auto.manual_class, BUTTON_CLASS_IDLE);

    let manual = sync_mode_button_state("manual");
    assert_eq!(manual.auto_class, BUTTON_CLASS_IDLE);
    assert_eq!(manual.manual_class, SYNC_MANUAL_CLASS_ACTIVE);
}

#[test]
fn reserved_setting_state_exposes_disabled_reason() {
    let state = reserved_setting_state(Locale::En);
    assert_eq!(
        state.class,
        "bg-sidebar p-4 rounded-lg border border-default opacity-80 cursor-not-allowed select-none"
    );
    assert_eq!(state.disabled_attr, "true");
    assert_eq!(state.aria_disabled, "true");
    assert!(state.reason.contains("Future setting"));
    assert!(state.reason.contains("current release"));
}

#[test]
fn settings_buttons_keep_mobile_safe_touch_targets() {
    for class in [
        BUTTON_CLASS_IDLE,
        BUTTON_CLASS_DISABLED,
        BUTTON_CLASS_ACCENT_ACTIVE,
        SYNC_AUTO_CLASS_ACTIVE,
        SYNC_MANUAL_CLASS_ACTIVE,
    ] {
        assert!(
            class.contains("min-h-[44px]"),
            "{class} must preserve a 44px minimum touch target"
        );
    }
}

#[test]
fn sync_mode_buttons_treat_unknown_mode_as_auto_safe_default() {
    let state = sync_mode_button_state("unexpected");
    assert_eq!(state.auto_class, SYNC_AUTO_CLASS_ACTIVE);
    assert_eq!(state.manual_class, BUTTON_CLASS_IDLE);
}

#[test]
fn theme_buttons_reflect_browser_local_preference() {
    let warm = theme_button_state(ThemePreference::Warm);
    assert_eq!(warm.warm_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert_eq!(warm.cold_class, BUTTON_CLASS_IDLE);
    assert_eq!(warm.night_class, BUTTON_CLASS_IDLE);

    let cold = theme_button_state(ThemePreference::Cold);
    assert_eq!(cold.cold_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert_eq!(cold.warm_class, BUTTON_CLASS_IDLE);

    let night = theme_button_state(ThemePreference::Night);
    assert_eq!(night.warm_class, BUTTON_CLASS_IDLE);
    assert_eq!(night.night_class, BUTTON_CLASS_ACCENT_ACTIVE);
}

#[test]
fn editor_preference_buttons_reflect_local_feedback_state() {
    let wrap = editor_wrap_button_state(EditorWrapPreference::Off);
    assert_eq!(wrap.on_class, BUTTON_CLASS_IDLE);
    assert_eq!(wrap.off_class, BUTTON_CLASS_ACCENT_ACTIVE);

    let density = editor_density_button_state(EditorDensityPreference::Compact);
    assert_eq!(density.comfortable_class, BUTTON_CLASS_IDLE);
    assert_eq!(density.compact_class, BUTTON_CLASS_ACCENT_ACTIVE);
}

#[test]
fn ai_chat_visibility_buttons_reflect_local_feedback_state() {
    let visible = ai_chat_visibility_button_state(true);
    assert_eq!(visible.show_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert_eq!(visible.hide_class, BUTTON_CLASS_IDLE);

    let hidden = ai_chat_visibility_button_state(false);
    assert_eq!(hidden.show_class, BUTTON_CLASS_IDLE);
    assert_eq!(hidden.hide_class, BUTTON_CLASS_ACCENT_ACTIVE);
}

#[test]
fn native_backend_buttons_reflect_local_remote_mode() {
    let local = native_backend_button_state("local");
    assert_eq!(local.local_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert_eq!(local.remote_class, BUTTON_CLASS_IDLE);

    let remote = native_backend_button_state("remote");
    assert_eq!(remote.local_class, BUTTON_CLASS_IDLE);
    assert_eq!(remote.remote_class, BUTTON_CLASS_ACCENT_ACTIVE);

    let unknown = native_backend_button_state("unexpected");
    assert_eq!(unknown.local_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert_eq!(unknown.remote_class, BUTTON_CLASS_IDLE);
}

#[test]
fn native_backend_switch_local_only_has_effect_from_remote_mode() {
    assert!(!native_backend_can_switch_local("local"));
    assert!(native_backend_can_switch_local("remote"));
    assert!(!native_backend_can_switch_local("unexpected"));
}

#[test]
fn native_backend_validation_state_does_not_treat_failed_feedback_as_success() {
    assert_eq!(
        native_backend_validation_state(false, "remote backend probe failed", "remote", false),
        "failed"
    );
}

#[test]
fn native_backend_validation_state_marks_only_current_remote_success() {
    assert_eq!(
        native_backend_validation_state(false, "Remote backend saved", "remote", true),
        "success"
    );
    assert_eq!(
        native_backend_validation_state(false, "Local backend saved", "local", false),
        "idle"
    );
    assert_eq!(
        native_backend_validation_state(true, "Validating remote backend", "remote", false),
        "pending"
    );
}

#[test]
fn native_backend_unavailable_feedback_localizes_known_bridge_reason_at_render_time() {
    assert_eq!(
        native_backend_unavailable_feedback(Locale::Zh, "native backend bridge unavailable"),
        t::settings::native_backend_unavailable(Locale::Zh)
    );
    assert_eq!(
        native_backend_unavailable_feedback(Locale::En, "native backend bridge unavailable"),
        "Native-only setting unavailable in a regular browser."
    );
    assert_eq!(
        native_backend_unavailable_feedback(Locale::Zh, ""),
        t::settings::native_backend_unavailable(Locale::Zh)
    );
    assert_eq!(
        native_backend_unavailable_feedback(Locale::Zh, "remote backend probe failed"),
        "remote backend probe failed"
    );
}

#[test]
fn ai_backend_buttons_disable_only_unavailable_backends() {
    let state = ai_backend_button_state(
        AI_BACKEND_NATIVE,
        &AiBackendCapabilities {
            native_available: true,
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            ..AiBackendCapabilities::default()
        },
        Locale::En,
    );

    assert_eq!(state.native_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert!(!state.native_disabled);
    assert!(state.native_title.is_empty());
    assert_eq!(state.trusted_class, BUTTON_CLASS_DISABLED);
    assert!(state.trusted_disabled);
    assert_eq!(state.trusted_title, "External agent disabled");

    let zh_state = ai_backend_button_state(
        AI_BACKEND_NATIVE,
        &AiBackendCapabilities {
            native_available: true,
            trusted_cli_available: false,
            trusted_cli_reason: Some("external agent disabled".to_string()),
            ..AiBackendCapabilities::default()
        },
        Locale::Zh,
    );

    assert_eq!(
        zh_state.trusted_title,
        t::extensions::ai_backend_reason(Locale::Zh, "external agent disabled")
    );
}

#[test]
fn trusted_cli_default_off_keeps_native_visible_and_disables_trusted_backend() {
    let state = ai_backend_button_state(
        AI_BACKEND_NATIVE,
        &AiBackendCapabilities::default(),
        Locale::En,
    );

    assert_eq!(state.native_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert!(!state.native_disabled);
    assert!(state.native_title.is_empty());
    assert_eq!(state.trusted_class, BUTTON_CLASS_DISABLED);
    assert!(state.trusted_disabled);
    assert_eq!(state.trusted_title, "External agent disabled");
}

#[test]
fn ai_backend_buttons_show_disabled_reason_only_for_disabled_native() {
    let state = ai_backend_button_state(
        AI_BACKEND_NATIVE,
        &AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: true,
            ..AiBackendCapabilities::default()
        },
        Locale::En,
    );

    assert_eq!(state.native_class, BUTTON_CLASS_DISABLED);
    assert!(state.native_disabled);
    assert_eq!(state.native_title, "Native AI disabled by config");
    assert_eq!(state.trusted_class, BUTTON_CLASS_IDLE);
    assert!(!state.trusted_disabled);
    assert!(state.trusted_title.is_empty());

    let zh_state = ai_backend_button_state(
        AI_BACKEND_NATIVE,
        &AiBackendCapabilities {
            native_available: false,
            native_reason: Some("native AI disabled by config".to_string()),
            trusted_cli_available: true,
            ..AiBackendCapabilities::default()
        },
        Locale::Zh,
    );

    assert_eq!(
        zh_state.native_title,
        t::extensions::ai_backend_reason(Locale::Zh, "native AI disabled by config")
    );
}

#[test]
fn ai_backend_buttons_mark_trusted_cli_active_when_selected_and_available() {
    let state = ai_backend_button_state(
        AI_BACKEND_TRUSTED_CLI,
        &AiBackendCapabilities {
            native_available: true,
            trusted_cli_available: true,
            ..AiBackendCapabilities::default()
        },
        Locale::En,
    );

    assert_eq!(state.native_class, BUTTON_CLASS_IDLE);
    assert!(!state.native_disabled);
    assert_eq!(state.trusted_class, BUTTON_CLASS_ACCENT_ACTIVE);
    assert!(!state.trusted_disabled);
    assert!(state.native_title.is_empty());
    assert!(state.trusted_title.is_empty());
}
