// apps/web/src/components/mobile_layout/header.rs
//! # Mobile Header

use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn MobileHeader(
    title: Memo<String>,
    on_menu: Callback<()>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let action_btn = "h-11 min-w-11 px-3 text-base text-primary rounded-md hover:bg-hover active:bg-active transition-colors duration-200 ease-out";
    view! {
        <div
            class="flex items-center justify-between px-2 py-1 bg-panel border-b border-default"
            style="padding-top: env(safe-area-inset-top);"
        >
            <button
                class=action_btn
                title=move || t::header::file_tree(locale.get())
                aria-label=move || t::header::file_tree(locale.get())
                on:click=move |_| on_menu.run(())
            >
                "≡"
            </button>
            <div class="flex-1 mx-2 text-sm font-semibold text-primary truncate text-center">
                {move || title.get()}
            </div>
            <div class="flex items-center gap-2">
                <button
                    class=action_btn
                    title=move || t::header::home(locale.get())
                    aria-label=move || t::header::home(locale.get())
                    on:click=move |_| on_home.run(())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
                        <polyline points="9 22 9 12 15 12 15 22"></polyline>
                    </svg>
                </button>
                <button
                    class=action_btn
                    title=move || t::header::open(locale.get())
                    aria-label=move || t::header::open(locale.get())
                    on:click=move |_| on_open.run(())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path>
                        <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path>
                    </svg>
                </button>
                <button
                    class=action_btn
                    title=move || t::header::command(locale.get())
                    aria-label=move || t::header::command(locale.get())
                    on:click=move |_| on_command.run(())
                >
                    <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="4 17 10 11 4 5"></polyline>
                        <line x1="12" y1="19" x2="20" y2="19"></line>
                    </svg>
                </button>
            </div>
        </div>
    }
}
