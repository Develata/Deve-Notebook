// apps/web/src/components/chat/empty_state.rs
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn EmptyState(send_example: Callback<String>) -> impl IntoView {
    let send_example_1 = send_example.clone();
    let send_example_2 = send_example.clone();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let example_1 = move || t::chat::example_summarize(locale.get());
    let example_2 = move || t::chat::example_find_bugs(locale.get());

    view! {
        <div class="h-full flex flex-col items-center justify-center text-center text-muted">
            <div class="text-sm uppercase tracking-widest text-muted">{move || t::chat::empty_brand(locale.get())}</div>
            <div class="mt-2 text-lg font-semibold text-primary">{move || t::chat::try_these(locale.get())}</div>
            <div class="mt-4 flex flex-col gap-2 w-full max-w-xs">
                <button
                    class="h-11 px-3 rounded border border-default bg-panel text-sm active:bg-hover"
                    on:click=move |_| send_example_1.run(example_1().to_string())
                >
                    {example_1}
                </button>
                <button
                    class="h-11 px-3 rounded border border-default bg-panel text-sm active:bg-hover"
                    on:click=move |_| send_example_2.run(example_2().to_string())
                >
                    {example_2}
                </button>
                <div class="text-xs text-muted mt-2">
                    {move || t::chat::tip(locale.get())}
                </div>
            </div>
        </div>
    }
}
