// apps/web/src/components/mobile_layout/header.rs
//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-responsive-layout
//!   - 04_repository#repo-scope-runtime
//!
//! # Mobile Header

use crate::components::icons::{Book, Home, Terminal};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn topbar_button_class() -> &'static str {
    "h-11 min-w-[44px] px-3 text-base text-primary rounded-md hover:bg-hover active:bg-active transition-colors duration-200 ease-out"
}

#[component]
pub fn MobileHeader(
    title: Memo<String>,
    on_menu: Callback<()>,
    on_home: Callback<()>,
    on_open: Callback<()>,
    on_command: Callback<()>,
    on_logout: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div
            class="flex items-center justify-between px-2 py-1 bg-panel border-b border-default"
            style="padding-top: env(safe-area-inset-top);"
        >
            <button
                type="button"
                data-deve-mobile-touch-target="topbar_buttons"
                data-deve-mobile-header-action="open_left_drawer"
                class=topbar_button_class()
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
                    type="button"
                    data-deve-mobile-touch-target="topbar_buttons"
                    class=topbar_button_class()
                    title=move || t::header::home(locale.get())
                    aria-label=move || t::header::home(locale.get())
                    on:click=move |_| on_home.run(())
                >
                    <Home class="w-[18px] h-[18px]"/>
                </button>
                <button
                    type="button"
                    data-deve-mobile-touch-target="topbar_buttons"
                    class=topbar_button_class()
                    title=move || t::header::open(locale.get())
                    aria-label=move || t::header::open(locale.get())
                    on:click=move |_| on_open.run(())
                >
                    <Book class="w-[18px] h-[18px]"/>
                </button>
                <button
                    type="button"
                    data-deve-mobile-touch-target="topbar_buttons"
                    class=topbar_button_class()
                    title=move || t::header::command(locale.get())
                    aria-label=move || t::header::command(locale.get())
                    on:click=move |_| on_command.run(())
                >
                    <Terminal class="w-[18px] h-[18px]"/>
                </button>
                <button
                    type="button"
                    data-deve-mobile-touch-target="topbar_buttons"
                    class=topbar_button_class()
                    title=move || t::header::logout(locale.get())
                    aria-label=move || t::header::logout(locale.get())
                    on:click=move |_| on_logout.run(())
                >
                    {move || t::header::logout(locale.get())}
                </button>
            </div>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::topbar_button_class;

    #[test]
    fn mobile_touch_targets_topbar_buttons_are_at_least_44px() {
        let class = topbar_button_class();
        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
