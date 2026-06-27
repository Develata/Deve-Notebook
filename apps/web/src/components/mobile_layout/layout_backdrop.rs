//! plan_ref:
//!   - 11_ui_design/03_mobile#mobile-interaction-design
//!
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn MobileDrawerBackdrop(drawer_open: Signal<bool>, on_close: Callback<()>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    move || {
        if drawer_open.get() {
            view! {
                <button
                    type="button"
                    class="fixed inset-0 bg-black/40 z-[var(--z-overlay)] border-0 p-0 transition-opacity duration-200 ease-out"
                    title=move || t::common::close(locale.get())
                    aria-label=move || t::common::close(locale.get())
                    on:click=move |_| on_close.run(())
                ></button>
            }
            .into_any()
        } else {
            view! {}.into_any()
        }
    }
}
