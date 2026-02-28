// apps/web/src/components/chat/panel.rs
use crate::components::chat::actions::{
    make_on_apply, make_send_example, make_send_message, make_send_text,
};
use crate::components::chat::drag_overlay::DragOverlay;
use crate::components::chat::drop_handler::{on_drag_leave, on_drag_over, on_drop};
use crate::components::chat::header::ChatHeader;
use crate::components::chat::input_area::InputArea;
use crate::components::chat::message_list::MessageList;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ChatPanel(#[prop(optional)] mobile: bool, on_close: Callback<()>) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (input, set_input) = signal(String::new());
    let (is_drag_over, set_is_drag_over) = signal(false);
    let (last_prompt, set_last_prompt) = signal(String::new());
    let (error_msg, set_error_msg) = signal(None::<String>);
    let (pending_reqs, set_pending_reqs) = signal(Vec::<String>::new());

    let messages = core.chat_messages;
    let is_streaming = core.is_chat_streaming;

    let on_req_id = Callback::new(move |req_id: String| {
        set_error_msg.set(None);
        set_pending_reqs.update(|v| v.push(req_id));
    });
    let on_user_text = Callback::new(move |msg: String| {
        set_last_prompt.set(msg);
    });

    let send_text = make_send_text(
        core.clone(),
        is_streaming,
        Some(on_req_id),
        Some(on_user_text),
    );
    let send_message = make_send_message(input, set_input, is_streaming, send_text.clone());
    let send_example = make_send_example(send_text.clone(), set_input);
    let on_apply = make_on_apply(core.clone());
    let retry = Callback::new(move |_| {
        let prompt = last_prompt.get_untracked();
        if !prompt.is_empty() {
            send_text.run(prompt);
        }
    });

    Effect::new(move |_| {
        let Some((req_id, _result, error)) = core.plugin_last_response.get() else {
            return;
        };
        let matched = pending_reqs.get_untracked().iter().any(|id| id == &req_id);
        if !matched {
            return;
        }
        set_pending_reqs.update(|v| v.retain(|id| id != &req_id));
        if let Some(err) = error {
            set_error_msg.set(Some(err));
        }
    });

    let loading = Signal::derive(move || is_streaming.get() || !pending_reqs.get().is_empty());

    view! {
        <div
            class=move || if mobile {
                "h-full flex flex-col bg-[#f3f3f3] dark:bg-[#1e1e1e] relative"
            } else {
                "h-full flex flex-col bg-[#f3f3f3] dark:bg-[#1e1e1e] border-l border-[#e5e5e5] dark:border-[#252526] relative"
            }
            on:dragover=on_drag_over(set_is_drag_over)
            on:dragleave=on_drag_leave(set_is_drag_over)
            on:drop=on_drop(set_input, set_is_drag_over)
        >
            <DragOverlay is_drag_over=is_drag_over />
            <ChatHeader mobile=mobile on_close=on_close />
            <MessageList
                messages=messages
                is_streaming=is_streaming
                send_example=send_example
                on_apply=on_apply
                mobile=mobile
            />
            <Show when=move || error_msg.get().is_some()>
                <div class="mx-2 mb-2 rounded border border-red-200 bg-red-50 px-2 py-2 text-xs text-red-700 flex items-center justify-between gap-2">
                    <div class="min-w-0 truncate">
                        {move || {
                            let suffix = error_msg.get().unwrap_or_default();
                            format!("{}: {}", t::chat::send_failed(locale.get()), suffix)
                        }}
                    </div>
                    <button
                        class="h-11 min-w-11 px-3 rounded bg-white border border-red-200 text-red-700 active:bg-red-100"
                        on:click=move |_| retry.run(())
                    >
                        {move || t::chat::retry(locale.get())}
                    </button>
                </div>
            </Show>
            <Show when=move || loading.get()>
                <div class="px-3 pb-1 text-[11px] text-[#6d6d6d]">{move || t::chat::loading(locale.get())}</div>
            </Show>
            <InputArea
                input=input
                set_input=set_input
                is_streaming=is_streaming
                send_message=send_message
                mobile=mobile
            />
        </div>
    }
}
