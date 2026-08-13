// apps\web\src\components
//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 15_settings#browser-ui-prefs
//!
//! # SettingsModal 组件 (SettingsModal Component)
//!
//! 设置模态框，允许用户更改语言、同步模式等全局配置。
//! 显示版本信息和未来功能占位符（如混合模式）。

use crate::components::main_layout::{SettingsControl, SettingsSection};
use crate::components::settings_sections::{
    AiBackendSection, AiChatVisibilitySection, AiProviderSettingsSection, AppearanceSection,
    EditorBasicsSection, NativeBackendSection, RuntimeDiagnosticsSection, SyncModeSection,
};
use crate::components::settings_sections_policy::{language_button_state, reserved_setting_state};
use crate::components::{focus_scope, icons::X};
use crate::i18n::{Locale, persist_locale_preference, t};
use leptos::prelude::*;

const SETTINGS_OVERLAY_CLASS: &str = concat!(
    "fixed inset-0 z-[var(--z-modal)] flex items-end justify-center ",
    "bg-black/50 backdrop-blur-sm transition-opacity sm:items-center sm:p-4"
);
const SETTINGS_PANEL_CLASS: &str = concat!(
    "bg-panel w-full max-h-[100dvh] overflow-y-auto rounded-t-xl p-4 shadow-2xl ",
    "transform transition-all scale-100 opacity-100 sm:max-w-2xl sm:max-h-[88vh] ",
    "sm:rounded-xl sm:p-6"
);
const SETTINGS_ICON_BUTTON_CLASS: &str =
    "min-h-[44px] min-w-[44px] p-2 hover:bg-hover rounded-full text-muted";

#[component]
pub fn SettingsModal(show: ReadSignal<bool>, set_show: WriteSignal<bool>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let language_state = Signal::derive(move || language_button_state(locale.get()));
    let reserved_state = Signal::derive(move || reserved_setting_state(locale.get()));
    let settings_control = expect_context::<SettingsControl>();
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let close_button_ref = NodeRef::<leptos::html::Button>::new();
    focus_scope::attach_modal_focus_restore_effect_with_selector(
        move || show.get(),
        move || initial_settings_focus_selector(settings_control.section.get_untracked()),
        move || settings_control.focus_request.get(),
        panel_ref,
        close_button_ref,
    );

    view! {
        <Show when=move || show.get()>
            <div class=SETTINGS_OVERLAY_CLASS data-deve-settings-overlay="true">
                <div
                    node_ref=panel_ref
                    role="dialog"
                    aria-modal="true"
                    aria-labelledby="deve-settings-title"
                    tabindex="-1"
                    class=SETTINGS_PANEL_CLASS
                    data-deve-settings-surface="modal"
                    on:keydown=move |ev| {
                        if ev.key() == "Escape" {
                            ev.prevent_default();
                            set_show.set(false);
                            return;
                        }
                        let _ = focus_scope::handle_focus_trap_keydown(&ev, panel_ref);
                    }
                >
                    <div class="mb-6 flex items-center justify-between gap-3">
                        <h2 id="deve-settings-title" class="text-xl font-bold text-primary">{move || t::settings::title(locale.get())}</h2>
                        <button
                            node_ref=close_button_ref
                            class=SETTINGS_ICON_BUTTON_CLASS
                            aria-label=move || t::settings::close(locale.get())
                            data-deve-settings-close="icon"
                            on:click=move |_| set_show.set(false)
                        >
                            <X class="w-6 h-6"/>
                        </button>
                    </div>

                    <div class="space-y-6">
                        // 版本信息
                        <div class="bg-sidebar p-4 rounded-lg border border-default">
                            <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-2">{move || t::settings::about(locale.get())}</h3>
                            <div class="flex flex-wrap items-center justify-between gap-2 text-sm">
                                <span class="text-secondary">{move || t::settings::version(locale.get())}</span>
                                <span class="font-mono text-primary">{env!("CARGO_PKG_VERSION")}</span>
                            </div>
                        </div>

                        // 外观设置
                        <AppearanceSection locale=locale />

                        // 语言设置
                        <div class="bg-sidebar p-4 rounded-lg border border-default flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                            <span class="font-medium text-primary">{move || t::settings::language(locale.get())}</span>
                            <div class="flex flex-wrap gap-2">
                                <button
                                    class=move || language_state.get().english_class
                                    on:click=move |_| {
                                        persist_locale_preference(Locale::En);
                                        locale.set(Locale::En);
                                    }
                                >
                                    {t::settings::english_language_label()}
                                </button>
                                <button
                                    class=move || language_state.get().chinese_class
                                    on:click=move |_| {
                                        persist_locale_preference(Locale::Zh);
                                        locale.set(Locale::Zh);
                                    }
                                >
                                    {t::settings::chinese_language_label()}
                                </button>
                            </div>
                        </div>

                        // 同步模式设置
                        <SyncModeSection locale=locale />

                        // AI 后端设置
                        <AiBackendSection locale=locale />

                        // Native AI provider server settings
                        <AiProviderSettingsSection locale=locale />

                        // AI Chat 面板显示设置
                        <AiChatVisibilitySection locale=locale />

                        // Native 后端选择
                        <NativeBackendSection locale=locale />

                        // 编辑器基础偏好
                        <EditorBasicsSection locale=locale />

                        // 开发运行诊断入口
                        <RuntimeDiagnosticsSection locale=locale />

                        // 当前设置边界
                        <div class="bg-sidebar p-4 rounded-lg border border-default">
                            <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-2">
                                {move || t::settings::current_boundary(locale.get())}
                            </h3>
                            <p class="text-xs text-muted leading-relaxed">
                                {move || t::settings::current_boundary_desc(locale.get())}
                            </p>
                        </div>

                        // 混合模式占位符
                        <div
                            class=move || reserved_state.get().class
                            data-deve-setting-disabled=move || reserved_state.get().disabled_attr
                            aria-disabled=move || reserved_state.get().aria_disabled
                            title=move || reserved_state.get().reason
                        >
                             <div class="flex items-center justify-between">
                                <div>
                                    <h3 class="font-medium text-primary">{move || t::settings::hybrid_mode(locale.get())}</h3>
                                    <p class="text-sm text-muted">{move || t::settings::hybrid_desc(locale.get())}</p>
                                </div>
                                <div class="w-11 h-6 bg-active rounded-full relative">
                                    <div class="absolute left-1 top-1 w-4 h-4 bg-white rounded-full shadow"></div>
                                </div>
                             </div>
                             <p class="text-xs text-accent mt-2">{move || reserved_state.get().reason}</p>
                        </div>
                    </div>

                </div>
            </div>
        </Show>
    }
}

fn initial_settings_focus_selector(section: SettingsSection) -> Option<&'static str> {
    match section {
        SettingsSection::General => None,
        SettingsSection::NativeAiProvider => Some("[data-deve-settings-ai-provider=\"true\"]"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SETTINGS_ICON_BUTTON_CLASS, SETTINGS_OVERLAY_CLASS, SETTINGS_PANEL_CLASS,
        initial_settings_focus_selector,
    };
    use crate::components::main_layout::SettingsSection;

    #[test]
    fn settings_modal_classes_keep_narrow_viewport_constraints() {
        assert!(SETTINGS_OVERLAY_CLASS.contains("items-end"));
        assert!(SETTINGS_OVERLAY_CLASS.contains("sm:items-center"));
        assert!(SETTINGS_PANEL_CLASS.contains("w-full"));
        assert!(SETTINGS_PANEL_CLASS.contains("max-h-[100dvh]"));
        assert!(SETTINGS_PANEL_CLASS.contains("sm:max-w-2xl"));
    }

    #[test]
    fn settings_modal_close_controls_keep_touch_safe_targets() {
        assert!(SETTINGS_ICON_BUTTON_CLASS.contains("min-h-[44px]"));
        assert!(SETTINGS_ICON_BUTTON_CLASS.contains("min-w-[44px]"));
    }

    #[test]
    fn settings_modal_uses_icon_close_only() {
        assert_eq!(
            SETTINGS_ICON_BUTTON_CLASS.matches("min-h-[44px]").count(),
            1
        );
    }

    #[test]
    fn ai_settings_command_selects_provider_section_as_initial_focus() {
        assert_eq!(
            initial_settings_focus_selector(SettingsSection::General),
            None
        );
        assert_eq!(
            initial_settings_focus_selector(SettingsSection::NativeAiProvider),
            Some("[data-deve-settings-ai-provider=\"true\"]")
        );
    }
}
