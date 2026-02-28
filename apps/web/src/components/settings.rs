// apps\web\src\components
//! # SettingsModal 组件 (SettingsModal Component)
//!
//! 设置模态框，允许用户更改语言、同步模式等全局配置。
//! 显示版本信息和未来功能占位符（如混合模式）。

use crate::components::settings_sections::{AiBackendSection, SyncModeSection};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn SettingsModal(show: ReadSignal<bool>, set_show: WriteSignal<bool>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <Show when=move || show.get()>
            <div class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 backdrop-blur-sm transition-opacity">
                <div class="bg-panel rounded-xl shadow-2xl w-full max-w-md p-6 transform transition-all scale-100 opacity-100">
                    <div class="flex items-center justify-between mb-6">
                        <h2 class="text-xl font-bold text-primary">{move || t::settings::title(locale.get())}</h2>
                        <button
                            class="p-1 hover:bg-hover rounded-full text-muted"
                            on:click=move |_| set_show.set(false)
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
                        </button>
                    </div>

                    <div class="space-y-6">
                        // 版本信息
                        <div class="bg-sidebar p-4 rounded-lg border border-default">
                            <h3 class="text-sm font-semibold text-muted uppercase tracking-wider mb-2">{move || t::settings::about(locale.get())}</h3>
                            <div class="flex justify-between items-center text-sm">
                                <span class="text-secondary">{move || t::settings::version(locale.get())}</span>
                                <span class="font-mono text-primary">"0.5.0-alpha"</span>
                            </div>
                        </div>

                        // 语言设置
                        <div class="bg-sidebar p-4 rounded-lg border border-default flex justify-between items-center">
                            <span class="font-medium text-primary">{move || t::settings::language(locale.get())}</span>
                            <div class="flex gap-2">
                                <button
                                    class=move || {
                                        if locale.get() == Locale::En {
                                            "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors"
                                        } else {
                                            "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                                        }
                                    }
                                    on:click=move |_| locale.set(Locale::En)
                                >
                                    "English"
                                </button>
                                <button
                                    class=move || {
                                        if locale.get() == Locale::Zh {
                                            "px-3 py-1 text-xs font-bold bg-accent text-on-accent rounded transition-colors"
                                        } else {
                                            "px-3 py-1 text-xs font-medium text-muted hover:bg-active rounded transition-colors"
                                        }
                                    }
                                    on:click=move |_| locale.set(Locale::Zh)
                                >
                                    "中文"
                                </button>
                            </div>
                        </div>

                        // 同步模式设置
                        <SyncModeSection locale=locale />

                        // AI 后端设置
                        <AiBackendSection locale=locale />

                        // 混合模式占位符
                        <div class="opacity-50 pointer-events-none grayscale">
                             <div class="flex items-center justify-between">
                                <div>
                                    <h3 class="font-medium text-primary">{move || t::settings::hybrid_mode(locale.get())}</h3>
                                    <p class="text-sm text-muted">{move || t::settings::hybrid_desc(locale.get())}</p>
                                </div>
                                <div class="w-11 h-6 bg-active rounded-full relative">
                                    <div class="absolute left-1 top-1 w-4 h-4 bg-white rounded-full shadow"></div>
                                </div>
                             </div>
                             <p class="text-xs text-accent mt-2">{move || t::settings::coming_soon(locale.get())}</p>
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
