use crate::components::icons::X;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub(super) fn LeftDrawerHeader(
    locale: RwSignal<Locale>,
    title: Signal<String>,
    on_close: Callback<()>,
) -> impl IntoView {
    view! {
        <div
            class="h-12 px-3 flex items-center justify-between border-b border-default text-sm font-semibold"
            style="padding-top: env(safe-area-inset-top);"
        >
            <span class="text-primary flex items-center gap-1">{move || title.get()}</span>
            <button
                class="h-11 min-w-11 px-3 text-sm font-medium text-secondary rounded-md hover:bg-hover active:bg-active transition-colors duration-200 ease-out"
                title=move || t::sidebar::close_file_tree(locale.get())
                aria-label=move || t::sidebar::close_file_tree(locale.get())
                on:click=move |_| on_close.run(())
            >
                <X class="w-4 h-4 mx-auto"/>
            </button>
        </div>
    }
}
