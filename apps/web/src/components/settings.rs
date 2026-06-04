// apps\web\src\components
//! plan_ref:
//!   - 15_settings#configuration-settings
//!   - 15_settings#browser-ui-prefs
//!
//! # SettingsModal 组件 (SettingsModal Component)
//!
//! 设置模态框，允许用户更改语言、同步模式等全局配置。
//! 显示版本信息和未来功能占位符（如混合模式）。

use crate::components::settings_sections::{
    AiBackendSection, AppearanceSection, EditorBasicsSection, RuntimeDiagnosticsSection,
    SyncModeSection,
};
use crate::components::settings_sections_policy::{language_button_state, reserved_setting_state};
use crate::components::{focus_scope, icons::X};
use crate::i18n::{Locale, persist_locale_preference, t};
use leptos::prelude::*;

#[component]
pub fn SettingsModal(show: ReadSignal<bool>, set_show: WriteSignal<bool>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let language_state = Signal::derive(move || language_button_state(locale.get()));
    let reserved_state = Signal::derive(move || reserved_setting_state(locale.get()));
    let panel_ref = NodeRef::<leptos::html::Div>::new();
    let close_button_ref = NodeRef::<leptos::html::Button>::new();
    focus_scope::attach_modal_focus_restore_effect(move || show.get(), close_button_ref);

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-[var(--z-modal)] flex items-center justify-center bg-black/50 backdrop-blur-sm transition-opacity">
                <div
                    node_ref=panel_ref
                    role="dialog"
                    aria-modal="true"
                    tabindex="-1"
                    class="bg-panel rounded-xl shadow-2xl w-full max-w-2xl max-h-[88vh] overflow-y-auto p-6 transform transition-all scale-100 opacity-100"
                    on:keydown=move |ev| {
                        let _ = focus_scope::handle_focus_trap_keydown(&ev, panel_ref);
                    }
                >
                    <div class="flex items-center justify-between mb-6">
                        <h2 class="text-xl font-bold text-primary">{move || t::settings::title(locale.get())}</h2>
                        <button
                            node_ref=close_button_ref
                            class="p-1 hover:bg-hover rounded-full text-muted"
                            on:click=move |_| set_show.set(false)
                        >
                            <X class="w-6 h-6"/>
                        </button>
                    </div>

                    <div class="space-y-6">
                        // 版本信息
                        <div class="bg-sidebar p-4 rounded-lg border border-default">
                            <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-2">{move || t::settings::about(locale.get())}</h3>
                            <div class="flex justify-between items-center text-sm">
                                <span class="text-secondary">{move || t::settings::version(locale.get())}</span>
                                <span class="font-mono text-primary">{env!("CARGO_PKG_VERSION")}</span>
                            </div>
                        </div>

                        // 外观设置
                        <AppearanceSection locale=locale />

                        // 语言设置
                        <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                            <span class="font-medium text-primary">{move || t::settings::language(locale.get())}</span>
                            <div class="flex gap-2">
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

                    <div class="mt-8 pt-4 border-t border-default text-center">
                        <button
                            class="w-full py-2 bg-accent text-on-accent rounded-lg hover:opacity-90 transition-colors font-medium"
                            on:click=move |_| set_show.set(false)
                        >
                            {move || t::settings::close(locale.get())}
                        </button>
                    </div>
                </div>
            </div>
        </Show>
    }
}
