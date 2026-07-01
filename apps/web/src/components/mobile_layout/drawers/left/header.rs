//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!   - 04_repository#repo-scope-runtime
//!
use crate::components::icons::X;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

pub(super) fn drawer_close_button_class() -> &'static str {
    "h-11 min-w-[44px] px-3 text-sm font-medium text-secondary rounded-md hover:bg-hover active:bg-active transition-colors duration-200 ease-out"
}

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
                data-deve-mobile-touch-target="drawer_close_buttons"
                class=drawer_close_button_class()
                title=move || t::sidebar::close_sidebar(locale.get())
                aria-label=move || t::sidebar::close_sidebar(locale.get())
                on:click=move |_| on_close.run(())
            >
                <X class="w-4 h-4 mx-auto"/>
            </button>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::drawer_close_button_class;

    #[test]
    fn mobile_touch_targets_left_drawer_close_button_is_at_least_44px() {
        let class = drawer_close_button_class();
        assert!(class.contains("h-11"));
        assert!(class.contains("min-w-[44px]"));
    }
}
