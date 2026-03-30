// apps\web\src\components\source_control
//! # Commit Component (提交组件)
//!
//! VS Code 风格:
//! - Input Message Box + AI 生成按钮 (Phase 5 占位)
//! - Blue "Commit" button with dropdown (Commit & Push 占位)

use crate::components::icons::*;
use crate::hooks::use_core::{ChatContext, ChatMessage, SourceControlContext};
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use web_sys::KeyboardEvent;

#[component]
pub fn Commit() -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let chat_ctx = expect_context::<ChatContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    let (msg, set_msg) = signal(String::new());
    let (is_generating, set_is_generating) = signal(false);
    let dropdown_open = RwSignal::new(false);
    let active_req_id = RwSignal::new(None::<String>);
    let saw_streaming = RwSignal::new(false);

    // 是否有暂存文件 (VS Code allows commiting all if none staged, but we keep it safe for now)
    let has_staged = move || !core.staged_changes.get().is_empty();
    let can_prepare_commit = move || core.can_write.get() && has_staged();
    let can_commit_now = move || can_prepare_commit() && !msg.get().trim().is_empty();

    let do_commit = move || {
        if !(core.can_write.get_untracked()
            && !core.staged_changes.get_untracked().is_empty()
            && !msg.get_untracked().trim().is_empty())
        {
            return;
        }
        core.on_commit.run(msg.get());
        set_msg.set(String::new());
    };

    let on_keydown = move |ev: KeyboardEvent| {
        if ev.ctrl_key() && ev.key() == "Enter" {
            do_commit();
        }
    };

    Effect::new(move |_| {
        let req_id = active_req_id.get();
        let is_streaming = chat_ctx.is_streaming.get();
        if let Some(req_id) = req_id {
            if let Some(content) = chat_ctx
                .messages
                .get()
                .iter()
                .rev()
                .find(|m| m.req_id.as_deref() == Some(req_id.as_str()))
                .map(|m| m.content.clone())
            {
                set_msg.set(content);
            }
            if is_streaming {
                saw_streaming.set(true);
            }
            if saw_streaming.get_untracked() && !is_streaming {
                set_is_generating.set(false);
                saw_streaming.set(false);
                active_req_id.set(None);
            }
        }
    });

    view! {
        <div class="px-2 pb-3 pt-1">
            <div class="flex flex-col gap-2">
                <div class="relative w-full">
                    <textarea
                        name="commit-message"
                        class="w-full h-9 p-1.5 pr-20 text-[13px] bg-input border border-default rounded-[2px] focus:outline-none focus:border-b-accent focus:ring-1 focus:ring-accent placeholder:text-muted text-primary font-sans resize-none block leading-tight"
                        placeholder=move || t::source_control::commit_message_placeholder(locale.get())
                        prop:value=msg
                        on:input=move |ev| set_msg.set(event_target_value(&ev))
                        on:keydown=on_keydown
                        disabled=can_prepare_commit
                    />
                    <button
                        class="absolute right-1 top-1 bottom-1 px-1.5 bg-accent hover:bg-accent-hover text-on-accent text-[10px] rounded flex items-center gap-1 transition-colors z-10 disabled:opacity-50 disabled:cursor-not-allowed"
                        title=move || t::source_control::generate_commit_message(locale.get())
                        disabled=move || !can_prepare_commit() || is_generating.get()
                        on:click=move |_| {
                            if !(core.can_write.get_untracked()
                                && !core.staged_changes.get_untracked().is_empty())
                            {
                                return;
                            }
                            let req_id = uuid::Uuid::new_v4().to_string();
                            let joined_paths = core
                                .staged_changes
                                .get()
                                .into_iter()
                                .map(|entry| entry.path)
                                .collect::<Vec<_>>()
                                .join("\n");
                            let prompt = format!(
                                "{}\n{}",
                                t::source_control::generate_prompt(locale.get()),
                                joined_paths
                            );
                            let args = vec![
                                serde_json::json!(req_id),
                                serde_json::json!(prompt),
                                serde_json::json!(""),
                            ];
                            active_req_id.set(Some(req_id.clone()));
                            saw_streaming.set(false);
                            set_is_generating.set(true);
                            chat_ctx.set_messages.update(|messages| {
                                messages.push(ChatMessage {
                                    role: "assistant".into(),
                                    content: String::new(),
                                    req_id: Some(req_id.clone()),
                                    ts_ms: js_sys::Date::now() as u64,
                                });
                            });
                            chat_ctx.set_is_streaming.set(true);
                            chat_ctx.on_plugin_call.run((
                                req_id,
                                "agent-bridge".to_string(),
                                "chat".to_string(),
                                args,
                            ));
                        }
                    >
                         <Sparkles class="w-3 h-3" />
                         {move || {
                             if is_generating.get() {
                                 t::source_control::generating(locale.get())
                             } else {
                                 t::source_control::generate(locale.get())
                             }
                         }}
                    </button>
                </div>

                <div class="flex relative">
                    <button
                        class="flex-1 bg-accent hover:bg-accent-hover text-on-accent text-[13px] font-medium py-1.5 rounded-l-[2px] flex items-center justify-center gap-1 disabled:opacity-50 disabled:bg-accent disabled:cursor-not-allowed transition-colors shadow-sm"
                        disabled=can_commit_now
                        on:click=move |_| { dropdown_open.set(false); do_commit(); }
                    >
                        <span class="codicon codicon-check"></span>
                        <span>{move || t::source_control::commit(locale.get())}</span>
                    </button>
                    <button
                        class="bg-accent hover:bg-accent-hover text-on-accent px-2 rounded-r-[2px] border-l border-white/20"
                        disabled=move || !can_prepare_commit()
                        on:click=move |_| dropdown_open.update(|b| *b = !*b)
                    >
                         <ChevronDown class="w-3.5 h-3.5" />
                    </button>

                    // 下拉菜单 (Commit 操作变体)
                    {move || if dropdown_open.get() {
                        view! {
                            <div class="absolute top-full left-0 right-0 mt-1 bg-dropdown border border-default rounded shadow-lg z-20 text-[13px]">
                                <button
                                    class="w-full text-left px-3 py-1.5 hover:bg-hover text-primary flex items-center gap-2"
                                    disabled=move || !can_commit_now()
                                    on:click=move |_| { dropdown_open.set(false); do_commit(); }
                                >
                                    <Check class="w-3.5 h-3.5" />
                                    {move || t::source_control::commit(locale.get())}
                                </button>
                                <button
                                    class="w-full text-left px-3 py-1.5 hover:bg-hover text-primary flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                                    disabled=move || !can_commit_now()
                                    on:click=move |_| {
                                        dropdown_open.set(false);
                                        core.on_commit_and_push.run(msg.get());
                                        set_msg.set(String::new());
                                    }
                                >
                                    <Upload class="w-3.5 h-3.5" />
                                    {move || t::source_control::commit_and_push(locale.get())}
                                </button>
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
