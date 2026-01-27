use crate::hooks::use_core::use_core;
use crate::utils::markdown::render_markdown;
use leptos::prelude::*;
use leptos::*;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;

#[component]
pub fn ChatPanel() -> impl IntoView {
    let core = use_core();

    // UI State
    let (input, set_input) = signal(String::new());
    let (is_drag_over, set_is_drag_over) = signal(false);

    // Auto-scroll ref
    let messages_end_ref = NodeRef::<html::Div>::new();

    // Derived signal for messages from core state
    let messages = core.chat_messages;
    let is_streaming = core.is_chat_streaming;

    Effect::new(move |_| {
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

        // Build context: current document path (if any)
        let current_doc_path = core
            .current_doc
            .get_untracked()
            .and_then(|doc_id| {
                core.docs
                    .get_untracked()
                    .iter()
                    .find(|(id, _)| *id == doc_id)
                    .map(|(_, path)| path.clone())
            })
            .unwrap_or_default();

        // Build context object for AI
        let context = serde_json::json!({
            "current_file": current_doc_path,
        });

        // Call Plugin with context
        let args = vec![serde_json::json!(req_id), serde_json::json!(msg), context];

        // Trigger Plugin Call
        core.on_plugin_call
            .run(("ai-chat".to_string(), "chat".to_string(), req_id, args));
    };

    // Drag & Drop Handlers
    let on_drag_over = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(true);
    };

    let on_drag_leave = move |ev: web_sys::DragEvent| {
        ev.prevent_default();
        set_is_drag_over.set(false);
    };

    let on_drop = {
        let set_input = set_input.clone();
        move |ev: web_sys::DragEvent| {
            ev.prevent_default();
            set_is_drag_over.set(false);

            if let Some(data_transfer) = ev.data_transfer() {
                if let Some(files) = data_transfer.files() {
                    for i in 0..files.length() {
                        if let Some(file) = files.item(i) {
                            let name = file.name();
                            // Only allow text files or code
                            // Simple heuristic: extension check or size check
                            // For now, assume user knows what they are doing, but we should limit size.
                            if file.size() > 1024.0 * 1024.0 {
                                leptos::logging::warn!("File too large: {}", name);
                                continue;
                            }

                            let reader = web_sys::FileReader::new().unwrap();
                            let reader_c = reader.clone();
                            let name_c = name.clone();
                            let set_input = set_input.clone();

                            let onload = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                                if let Ok(content) = reader_c
                                    .result()
                                    .and_then(|r| r.as_string().ok_or(wasm_bindgen::JsValue::NULL))
                                {
                                    set_input.update(|curr| {
                                        let block = format!("\n```{}\n{}\n```\n", name_c, content);
                                        curr.push_str(&block);
                                    });
                                }
                            })
                                as Box<dyn FnMut(_)>);

                            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
                            onload.forget(); // Leak memory to keep closure alive until callback (ok for SPA)

                            let _ = reader.read_as_text(&file);
                        }
                    }
                }
            }
        }
    };

    view! {
        <div
            class="h-full flex flex-col bg-[#f3f3f3] dark:bg-[#1e1e1e] border-l border-[#e5e5e5] dark:border-[#252526] relative"
            on:dragover=on_drag_over
            on:dragleave=on_drag_leave
            on:drop=on_drop
        >
            // Drag Overlay
            {move || if is_drag_over.get() {
                view! {
                    <div class="absolute inset-0 z-50 bg-blue-500/20 backdrop-blur-sm border-2 border-blue-500 border-dashed m-2 rounded-lg flex items-center justify-center pointer-events-none">
                        <span class="text-blue-600 dark:text-blue-400 font-bold text-lg bg-white/80 dark:bg-black/80 px-4 py-2 rounded-full">
                            "Drop files to add to context"
                        </span>
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

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
                                    render_markdown(&msg.content)
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

                <div node_ref=messages_end_ref></div>
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
                        on:keydown={
                            let send_message = send_message.clone();
                            move |ev| {
                                if ev.key() == "Enter" && !ev.shift_key() {
                                    ev.prevent_default();
                                    send_message();
                                }
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
