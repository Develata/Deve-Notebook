// apps\web\src\components\sidebar
//! plan_ref:
//!   - 10_ai_agent#native-ai-chat-runtime
//!   - 10_ai_agent#trusted-agent-bridge
//!   - 17_plugins#plugin-runtime-boundary
//!
//! # ExtensionsView 组件 (ExtensionsView Component)
//!
//! 轻量展示当前第一方扩展能力，并为后续插件运行时预留接口位。

#[path = "extensions_channels.rs"]
mod channels;

use crate::components::icons::{Book, Puzzle};
use crate::hooks::use_core::ChatContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ExtensionsView() -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));
    let chat = expect_context::<ChatContext>();

    view! {
        <div class="h-full w-full bg-sidebar flex flex-col">
            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-default">
                <span class="font-medium text-sm text-primary">{move || t::sidebar::extensions(locale.get())}</span>
            </div>
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                <div class="rounded-xl border border-default bg-panel p-4">
                    <div class="flex items-center gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary">
                            <Puzzle class="w-5 h-5" />
                        </div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || t::sidebar::extensions(locale.get())}</div>
                            <p class="text-xs text-muted">{move || t::sidebar::extensions_desc(locale.get())}</p>
                        </div>
                    </div>
                </div>
                <channels::AiChannelCards locale=locale chat=chat />
                <div class="space-y-3">
                    <div class="text-[11px] font-semibold uppercase tracking-wider text-muted">
                        {move || t::extensions::system_title(locale.get())}
                    </div>
                    <div class="rounded-xl border border-default bg-panel p-4">
                        <div class="flex items-start justify-between gap-3">
                            <div class="flex gap-3">
                                <div class="rounded-lg bg-active p-2 text-primary"><Book class="w-5 h-5" /></div>
                                <div>
                                    <div class="text-sm font-semibold text-primary">"KaTeX"</div>
                                    <p class="mt-1 text-xs text-muted">{move || t::extensions::katex_desc(locale.get())}</p>
                                </div>
                            </div>
                            <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                                {move || t::extensions::bundled_label(locale.get())}
                            </span>
                        </div>
                    </div>
                    <div class="rounded-xl border border-default bg-panel p-4">
                        <div class="flex items-start justify-between gap-3">
                            <div class="flex gap-3">
                                <div class="rounded-lg bg-active p-2 text-primary"><Puzzle class="w-5 h-5" /></div>
                                <div>
                                    <div class="text-sm font-semibold text-primary">"mhchem"</div>
                                    <p class="mt-1 text-xs text-muted">{move || t::extensions::mhchem_desc(locale.get())}</p>
                                </div>
                            </div>
                            <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                                {move || t::extensions::planned_label(locale.get())}
                            </span>
                        </div>
                    </div>
                    <div class="rounded-xl border border-dashed border-default bg-panel p-4 opacity-75">
                        <div class="flex items-start justify-between gap-3">
                            <div class="flex gap-3">
                                <div class="rounded-lg bg-active p-2 text-primary"><Puzzle class="w-5 h-5" /></div>
                                <div>
                                    <div class="text-sm font-semibold text-primary">{move || t::extensions::calculation_runtime_title(locale.get())}</div>
                                    <p class="mt-1 text-xs text-muted">{move || t::extensions::calculation_runtime_desc(locale.get())}</p>
                                </div>
                            </div>
                            <div class="flex flex-col items-end gap-2">
                                <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                                    {move || t::extensions::planned_label(locale.get())}
                                </span>
                                <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-muted">
                                    {move || t::extensions::code_execution_disabled(locale.get())}
                                </span>
                            </div>
                        </div>
                    </div>
                </div>
                <div class="rounded-xl border border-dashed border-default bg-panel p-4">
                    <div class="flex items-start gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary">
                            <Puzzle class="w-5 h-5" />
                        </div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || t::extensions::runtime_title(locale.get())}</div>
                            <p class="mt-1 text-xs text-muted">{move || t::extensions::runtime_desc(locale.get())}</p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
