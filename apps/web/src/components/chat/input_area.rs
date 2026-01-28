// apps/web/src/components/chat/input_area.rs
use leptos::prelude::*;

#[component]
pub fn InputArea(
    input: ReadSignal<String>,
    set_input: WriteSignal<String>,
    is_streaming: ReadSignal<bool>,
    send_message: Callback<()>,
) -> impl IntoView {
    view! {
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
                                send_message.run(());
                            }
                        }
                    }
                ></textarea>
                <div class="flex justify-between items-center px-2 pb-2">
                    <span class="text-[10px] text-[#858585]">"Markdown supported"</span>
                    <button
                        class="p-1.5 rounded hover:bg-[#f3f3f3] dark:hover:bg-[#3e3e42] text-[#007acc] disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                        disabled=move || input.get().trim().is_empty() || is_streaming.get()
                        on:click=move |_| send_message.run(())
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <line x1="22" y1="2" x2="11" y2="13"></line>
                            <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
                        </svg>
                    </button>
                </div>
            </div>
        </div>
    }
}
