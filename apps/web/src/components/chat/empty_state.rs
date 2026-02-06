// apps/web/src/components/chat/empty_state.rs
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn EmptyState(send_example: Callback<String>) -> impl IntoView {
    let send_example_1 = send_example.clone();
    let send_example_2 = send_example.clone();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    view! {
        <div class="h-full flex flex-col items-center justify-center text-center text-[#616161] dark:text-[#858585]">
            <div class="text-sm uppercase tracking-widest text-[#9aa1a8]">{move || t::chat::empty_brand(locale.get())}</div>
            <div class="mt-2 text-lg font-semibold text-[#3b3b3b] dark:text-[#cccccc]">{move || t::chat::try_these(locale.get())}</div>
            <div class="mt-4 flex flex-col gap-2 w-full max-w-xs">
                <button
                    class="h-11 px-3 rounded border border-[#e5e5e5] dark:border-[#3e3e42] bg-white dark:bg-[#252526] text-sm active:bg-[#f3f3f3] dark:active:bg-[#3e3e42]"
                    on:click=move |_| send_example_1.run("git_status".to_string())
                >
                    "git_status"
                </button>
                <button
                    class="h-11 px-3 rounded border border-[#e5e5e5] dark:border-[#3e3e42] bg-white dark:bg-[#252526] text-sm active:bg-[#f3f3f3] dark:active:bg-[#3e3e42]"
                    on:click=move |_| send_example_2.run("git_diff \"path/to/file.md\"".to_string())
                >
                    "git_diff \"path/to/file.md\""
                </button>
                <div class="text-xs text-[#9aa1a8] mt-2">
                    {move || t::chat::tip(locale.get())}
                </div>
            </div>
        </div>
    }
}
