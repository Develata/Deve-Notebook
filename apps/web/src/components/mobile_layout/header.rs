// apps/web/src/components/mobile_layout/header.rs
//! # Mobile Header

use crate::components::icons::{Book, Home, Terminal};
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
                    <Home class="w-[18px] h-[18px]"/>
                </button>
                <button
                    class=action_btn
                    title=move || t::header::open(locale.get())
                    aria-label=move || t::header::open(locale.get())
                    on:click=move |_| on_open.run(())
                >
                    <Book class="w-[18px] h-[18px]"/>
                </button>
                <button
                    class=action_btn
                    title=move || t::header::command(locale.get())
                    aria-label=move || t::header::command(locale.get())
                    on:click=move |_| on_command.run(())
                >
                    <Terminal class="w-[18px] h-[18px]"/>
                </button>
            </div>
        </div>
    }
}
