// apps/web/src/components/chat/message_item.rs
use crate::hooks::use_core::types::ChatMessage;
use crate::utils::markdown::render_markdown;
use leptos::prelude::*;

#[component]
pub fn MessageItem(msg: ChatMessage) -> impl IntoView {
    let is_user = msg.role == "user";
    let content = msg.content.clone();

    view! {
        <div class="flex flex-col gap-1">
            <div class={format!("flex items-center gap-2 {}", if is_user { "flex-row-reverse" } else { "flex-row" })}>
                <div class={format!("w-6 h-6 rounded flex items-center justify-center text-xs font-bold {}",
                    if is_user { "bg-[#007acc] text-white" } else { "bg-[#2d2d2d] text-[#cccccc]" }
                )}>
                    {if is_user { "U" } else { "AI" }}
                </div>
                <span class="text-xs text-[#616161] dark:text-[#858585]">{if is_user { "You" } else { "Assistant" }}</span>
            </div>

            <div class={format!("rounded px-3 py-2 text-sm leading-relaxed max-w-[90%] {}",
                if is_user {
                    "bg-[#e1f0fa] dark:bg-[#0e2a3f] text-[#3b3b3b] dark:text-[#cccccc] self-end ml-8"
                } else {
                    "bg-white dark:bg-[#252526] text-[#3b3b3b] dark:text-[#cccccc] border border-[#e5e5e5] dark:border-[#3e3e42] self-start mr-8"
                }
            )}>
                <div class="markdown-body" inner_html={render_markdown(&content)}></div>
            </div>
        </div>
    }
}
