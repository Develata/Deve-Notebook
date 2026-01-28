// apps/web/src/components/chat/empty_state.rs
use leptos::prelude::*;

#[component]
pub fn EmptyState(send_example: Callback<String>) -> impl IntoView {
    let send_example_1 = send_example.clone();
    let send_example_2 = send_example.clone();

    view! {
        <div class="h-full flex flex-col items-center justify-center text-center text-[#616161] dark:text-[#858585]">
            <div class="text-sm uppercase tracking-widest text-[#9aa1a8]">"Deve-Note AI"</div>
            <div class="mt-2 text-lg font-semibold text-[#3b3b3b] dark:text-[#cccccc]">"Try these"</div>
            <div class="mt-4 flex flex-col gap-2 w-full max-w-xs">
                <button
                    class="px-3 py-2 rounded border border-[#e5e5e5] dark:border-[#3e3e42] bg-white dark:bg-[#252526] text-sm hover:bg-[#f3f3f3] dark:hover:bg-[#3e3e42]"
                    on:click=move |_| send_example_1.run("git_status".to_string())
                >
                    "git_status"
                </button>
                <button
                    class="px-3 py-2 rounded border border-[#e5e5e5] dark:border-[#3e3e42] bg-white dark:bg-[#252526] text-sm hover:bg-[#f3f3f3] dark:hover:bg-[#3e3e42]"
                    on:click=move |_| send_example_2.run("git_diff \"path/to/file.md\"".to_string())
                >
                    "git_diff \"path/to/file.md\""
                </button>
                <div class="text-xs text-[#9aa1a8] mt-2">
                    "AI can stage and commit after review"
                </div>
            </div>
        </div>
    }
}
