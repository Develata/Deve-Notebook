// apps/web/src/components/chat/message_list.rs
use crate::components::chat::empty_state::EmptyState;
use crate::components::chat::message_item::MessageItem;
use crate::hooks::use_core::types::ChatMessage;
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use leptos::html;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

#[component]
pub fn MessageList(
    messages: ReadSignal<Vec<ChatMessage>>,
    is_streaming: ReadSignal<bool>,
    send_example: Callback<String>,
    on_apply: Callback<String>,
) -> impl IntoView {
    let messages_end_ref = NodeRef::<html::Div>::new();

    Effect::new(move |_| {
        messages.track();
        if let Some(el) = messages_end_ref.get() {
            el.scroll_into_view();
        }
        if let Some(window) = web_sys::window() {
            let hljs = js_sys::Reflect::get(&window, &"hljs".into()).ok();
            if let Some(hljs) = hljs
                && let Ok(func) = js_sys::Reflect::get(&hljs, &"highlightAll".into())
                && let Some(func) = func.dyn_ref::<js_sys::Function>()
            {
                let _ = func.call0(&hljs);
            }
        }
    });

    let on_click = move |ev: web_sys::MouseEvent| {
        let mut el = ev
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        while let Some(node) = el {
            if node.class_list().contains("apply-code")
                && let Some(data) = node.get_attribute("data-code")
                && let Ok(bytes) = STANDARD.decode(data)
                && let Ok(code) = String::from_utf8(bytes)
            {
                on_apply.run(code);
                break;
            }
            el = node.parent_element();
        }
    };

    view! {
        <div class="flex-1 overflow-y-auto p-4 space-y-4" on:click=on_click>
            {move || if messages.get().is_empty() {
                view! { <EmptyState send_example=send_example.clone() /> }.into_any()
            } else {
                view! {
                    <For
                        each=move || messages.get()
                        key=|msg| msg.req_id.clone().unwrap_or_else(|| msg.content.chars().take(32).collect())
                        children=move |msg| view! { <MessageItem msg=msg /> }
                    />
                }.into_any()
            }}

            {move || if is_streaming.get() {
                view! {
                    <div class="flex items-center gap-2 text-xs text-[#616161]">
                        <span class="animate-pulse">"Thinking..."</span>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            <div node_ref=messages_end_ref></div>
        </div>
    }
}
