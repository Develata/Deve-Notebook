use crate::hooks::use_core::UseCore;
use leptos::*;

#[component]
pub fn ChatPanel() -> impl IntoView {
    let core = UseCore::use_core();

    // UI State
    let (input, set_input) = create_signal(String::new());

    // Auto-scroll ref
    let messages_end_ref = create_node_ref::<html::Div>();

    // Derived signal for messages from core state
    let messages = core.chat_messages;
    let is_streaming = core.is_chat_streaming;

    create_effect(move |_| {
        // Auto scroll to bottom when messages change
        messages.track();
        if let Some(el) = messages_end_ref.get() {
            let _ = el.scroll_into_view();
        }
    });

    let send_message = move || {
        let msg = input.get().trim().to_string();
        if msg.is_empty() || is_streaming.get() {
            return;
        }

        set_input.set(String::new()); // Clear input immediately

        // Generate Request ID
        let req_id = uuid::Uuid::new_v4().to_string();

        // Optimistic Update
        core.append_chat_message("user", &msg, None);

        // Call Plugin (Real Logic)
        let args = vec![serde_json::json!(req_id), serde_json::json!(msg)];

        // Trigger Plugin Call
        // This sends `ClientMessage::PluginCall { plugin_id: "ai-chat", fn_name: "chat", ... }`
        core.on_plugin_call
            .run(("ai-chat".to_string(), "chat".to_string(), req_id, args));
    };

    view! {
        <div class="h-full flex flex-col bg-[#f3f3f3] dark:bg-[#1e1e1e] border-l border-[#e5e5e5] dark:border-[#252526]">
            // Header
            <div class="h-9 flex items-center px-4 border-b border-[#e5e5e5] dark:border-[#252526] bg-[#f8f8f8] dark:bg-[#2d2d2d]">
                <span class="text-xs font-bold text-[#3b3b3b] dark:text-[#cccccc] uppercase tracking-wider">"AI Assistant"</span>
                <div class="flex-1"></div>
                // Model Selector (Mock)
                <select class="text-xs bg-transparent border-none outline-none text-[#616161] dark:text-[#858585] cursor-pointer">
                    <option>"GPT-4o"</option>
                    <option>"Claude 3.5"</option>
                    <option>"DeepSeek V3"</option>
                </select>
            </div>

            // Messages Area
            <div class="flex-1 overflow-y-auto p-4 space-y-4">
                {move || messages.get().iter().map(|msg| {
                    let is_user = msg.role == "user";
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

                            // Message Bubble
                            <div class={format!("rounded px-3 py-2 text-sm leading-relaxed max-w-[90%] {}",
                                if is_user {
                                    "bg-[#e1f0fa] dark:bg-[#0e2a3f] text-[#3b3b3b] dark:text-[#cccccc] self-end ml-8"
                                } else {
                                    "bg-white dark:bg-[#252526] text-[#3b3b3b] dark:text-[#cccccc] border border-[#e5e5e5] dark:border-[#3e3e42] self-start mr-8"
                                }
                            )}>
                                // Render Markdown Content
                                // Note: We need a Markdown renderer component.
                                // For now, simple text or innerHTML if trusted.
                                <div class="markdown-body" inner_html={
                                    // Use a helper to render markdown?
                                    // Or just raw text for now.
                                    // crate::utils::render_markdown(&msg.content)
                                    msg.content.clone()
                                }></div>
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}

                // Loading Indicator
                {move || if is_streaming.get() {
                    view! {
                        <div class="flex items-center gap-2 text-xs text-[#616161]">
                            <span class="animate-pulse">"Thinking..."</span>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}

                <div _ref=messages_end_ref></div>
            </div>

            // Input Area
            <div class="p-3 border-t border-[#e5e5e5] dark:border-[#252526] bg-white dark:bg-[#1e1e1e]">
                <div class="relative rounded border border-[#e5e5e5] dark:border-[#3e3e42] bg-white dark:bg-[#252526] focus-within:border-[#007acc] dark:focus-within:border-[#007acc] transition-colors">
                    <textarea
                        class="w-full max-h-32 p-2 bg-transparent border-none outline-none text-sm resize-none dark:text-[#cccccc] font-sans"
                        placeholder="Ask anything... (Shift+Enter to send)"
                        rows="1"
                        prop:value=input
                        on:input=move |ev| set_input.set(event_target_value(&ev))
                        on:keydown=move |ev| {
                            if ev.key() == "Enter" && !ev.shift_key() {
                                ev.prevent_default();
                                send_message();
                            }
                        }
                    ></textarea>
                    <div class="flex justify-between items-center px-2 pb-2">
                        <span class="text-[10px] text-[#858585]">"Markdown supported"</span>
                        <button
                            class="p-1.5 rounded hover:bg-[#f3f3f3] dark:hover:bg-[#3e3e42] text-[#007acc] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                            disabled=move || input.get().trim().is_empty() || is_streaming.get()
                            on:click=move |_| send_message()
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                <line x1="22" y1="2" x2="11" y2="13"></line>
                                <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
                            </svg>
                        </button>
                    </div>
                </div>
            </div>
        </div>
    }
}
