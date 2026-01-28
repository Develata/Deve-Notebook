// apps/web/src/components/chat/panel.rs
use crate::components::chat::actions::{
    make_on_apply, make_send_example, make_send_message, make_send_text,
};
use crate::components::chat::drag_overlay::DragOverlay;
use crate::components::chat::drop_handler::{on_drag_leave, on_drag_over, on_drop};
use crate::components::chat::header::ChatHeader;
use crate::components::chat::input_area::InputArea;
use crate::components::chat::message_list::MessageList;
use crate::hooks::use_core::use_core;
use leptos::prelude::*;

#[component]
pub fn ChatPanel() -> impl IntoView {
    let core = use_core();
    let (input, set_input) = signal(String::new());
    let (is_drag_over, set_is_drag_over) = signal(false);

    let messages = core.chat_messages;
    let is_streaming = core.is_chat_streaming;

    let send_text = make_send_text(core.clone(), is_streaming);
    let send_message = make_send_message(input, set_input, is_streaming, send_text.clone());
    let send_example = make_send_example(send_text.clone(), set_input);
    let on_apply = make_on_apply(core.clone());

    view! {
        <div
            class="h-full flex flex-col bg-[#f3f3f3] dark:bg-[#1e1e1e] border-l border-[#e5e5e5] dark:border-[#252526] relative"
            on:dragover=on_drag_over(set_is_drag_over)
            on:dragleave=on_drag_leave(set_is_drag_over)
            on:drop=on_drop(set_input, set_is_drag_over)
        >
            <DragOverlay is_drag_over=is_drag_over />
            <ChatHeader ai_mode=core.ai_mode />
            <MessageList
                messages=messages
                is_streaming=is_streaming
                send_example=send_example
                on_apply=on_apply
            />
            <InputArea
                input=input
                set_input=set_input
                is_streaming=is_streaming
                send_message=send_message
            />
        </div>
    }
}
