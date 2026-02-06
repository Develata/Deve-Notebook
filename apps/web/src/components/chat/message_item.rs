// apps/web/src/components/chat/message_item.rs
use crate::hooks::use_core::types::ChatMessage;
use crate::i18n::{Locale, t};
use crate::utils::markdown::render_markdown;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Handles click events on markdown content.
/// Prevents link navigation unless Ctrl/Meta key is pressed.
fn handle_link_click(ev: web_sys::MouseEvent) {
    // Check if click target is an <a> element
    let Some(target) = ev.target() else { return };
    let Ok(el) = target.dyn_into::<web_sys::HtmlElement>() else {
        return;
    };

    // Use closest() to handle clicks on nested elements within <a>
    if el.closest("a").ok().flatten().is_some() {
        // Only allow navigation when Ctrl/Meta is pressed
        if !ev.ctrl_key() && !ev.meta_key() {
            ev.prevent_default();
        }
    }
}

#[component]
pub fn MessageItem(msg: ChatMessage, #[prop(optional)] mobile: bool) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let is_user = msg.role == "user";
    let content = msg.content.clone();
    let ts_text = {
        let date = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(msg.ts_ms as f64));
        format!("{:02}:{:02}", date.get_hours(), date.get_minutes())
    };

    view! {
        <div class="flex flex-col gap-1">
            <div class={format!("flex items-center gap-2 {}", if is_user { "flex-row-reverse" } else { "flex-row" })}>
                <div class={format!("w-6 h-6 rounded flex items-center justify-center text-xs font-bold {}",
                    if is_user { "bg-[#007acc] text-white" } else { "bg-[#2d2d2d] text-[#cccccc]" }
                )}>
                    {if is_user { "U" } else { "AI" }}
                </div>
                <span class="text-xs text-[#616161] dark:text-[#858585]">{if is_user {
                    t::chat::you(locale.get())
                } else {
                    t::chat::assistant(locale.get())
                }}</span>
            </div>

            <div class={format!("rounded px-3 py-2 text-sm leading-relaxed {} {}",
                if mobile { "max-w-[96%]" } else { "max-w-[90%]" },
                if is_user {
                    if mobile {
                        "bg-[#e1f0fa] dark:bg-[#0e2a3f] text-[#3b3b3b] dark:text-[#cccccc] self-end ml-3"
                    } else {
                        "bg-[#e1f0fa] dark:bg-[#0e2a3f] text-[#3b3b3b] dark:text-[#cccccc] self-end ml-8"
                    }
                } else if mobile {
                    "bg-white dark:bg-[#252526] text-[#3b3b3b] dark:text-[#cccccc] border border-[#e5e5e5] dark:border-[#3e3e42] self-start mr-3"
                } else {
                    "bg-white dark:bg-[#252526] text-[#3b3b3b] dark:text-[#cccccc] border border-[#e5e5e5] dark:border-[#3e3e42] self-start mr-8"
                }
            )}>
                <div
                    class="markdown-body break-words overflow-x-auto"
                    inner_html={render_markdown(&content, t::chat::apply(locale.get()))}
                    on:click=handle_link_click
                ></div>
                <div class="mt-1 text-[10px] text-[#8a8a8a] text-right">{ts_text}</div>
            </div>
        </div>
    }
}
