// apps\web\src\components\sidebar
//! # ExtensionsView 组件 (ExtensionsView Component)
//!
//! 轻量展示当前第一方扩展能力，并为后续插件运行时预留接口位。

use crate::components::icons::{Puzzle, Terminal, Zap};
use crate::hooks::use_core::ChatContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn card_desc(locale: Locale, mode: &str) -> &'static str {
    match (locale, mode) {
        (Locale::En, "agent-bridge") => "External CLI bridge with MCP, tools, and history.",
        (Locale::Zh, "agent-bridge") => "外部 CLI 桥接，支持 MCP、工具与历史能力。",
        (Locale::En, _) => "Built-in OpenAI-compatible chat for lightweight workflows.",
        (Locale::Zh, _) => "内置 OpenAI 兼容轻量聊天通道，适合轻量任务。",
    }
}

fn runtime_title(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Plugin Runtime",
        Locale::Zh => "插件运行时",
    }
}

fn runtime_desc(locale: Locale) -> &'static str {
    match locale {
        Locale::En => "Interface-first: bundled channels are ready, installable plugins come next.",
        Locale::Zh => "先完成接口层：内置通道已可用，可安装插件运行时仍在开发中。",
    }
}

fn status_label(locale: Locale, active: bool) -> &'static str {
    match (locale, active) {
        (Locale::En, true) => "Active",
        (Locale::Zh, true) => "当前使用",
        (Locale::En, false) => "Switch",
        (Locale::Zh, false) => "切换",
    }
}

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

                <div class="space-y-3">
                    <button
                        class=move || if chat.ai_mode.get() == "agent-bridge" {
                            "w-full rounded-xl border border-accent bg-accent/10 p-4 text-left"
                        } else {
                            "w-full rounded-xl border border-default bg-panel hover:bg-active p-4 text-left transition-colors"
                        }
                        on:click=move |_| chat.set_ai_mode.set("agent-bridge".to_string())
                    >
                        <div class="flex items-start justify-between gap-3">
                            <div class="flex gap-3">
                                <div class="rounded-lg bg-active p-2 text-primary"><Terminal class="w-5 h-5" /></div>
                                <div>
                                    <div class="text-sm font-semibold text-primary">{move || t::chat::agent_bridge(locale.get())}</div>
                                    <p class="mt-1 text-xs text-muted">{move || card_desc(locale.get(), "agent-bridge")}</p>
                                </div>
                            </div>
                            <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                                {move || status_label(locale.get(), chat.ai_mode.get() == "agent-bridge")}
                            </span>
                        </div>
                    </button>

                    <button
                        class=move || if chat.ai_mode.get() == "ai-chat" {
                            "w-full rounded-xl border border-accent bg-accent/10 p-4 text-left"
                        } else {
                            "w-full rounded-xl border border-default bg-panel hover:bg-active p-4 text-left transition-colors"
                        }
                        on:click=move |_| chat.set_ai_mode.set("ai-chat".to_string())
                    >
                        <div class="flex items-start justify-between gap-3">
                            <div class="flex gap-3">
                                <div class="rounded-lg bg-active p-2 text-primary"><Zap class="w-5 h-5" /></div>
                                <div>
                                    <div class="text-sm font-semibold text-primary">{move || t::chat::ai_chat(locale.get())}</div>
                                    <p class="mt-1 text-xs text-muted">{move || card_desc(locale.get(), "ai-chat")}</p>
                                </div>
                            </div>
                            <span class="rounded-full border border-default px-2 py-1 text-[10px] font-medium text-secondary">
                                {move || status_label(locale.get(), chat.ai_mode.get() == "ai-chat")}
                            </span>
                        </div>
                    </button>
                </div>

                <div class="rounded-xl border border-dashed border-default bg-panel p-4">
                    <div class="flex items-start gap-3">
                        <div class="rounded-lg bg-active p-2 text-primary">
                            <Puzzle class="w-5 h-5" />
                        </div>
                        <div>
                            <div class="text-sm font-semibold text-primary">{move || runtime_title(locale.get())}</div>
                            <p class="mt-1 text-xs text-muted">{move || runtime_desc(locale.get())}</p>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
