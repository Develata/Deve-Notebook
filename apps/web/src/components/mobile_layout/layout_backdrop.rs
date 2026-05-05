use leptos::prelude::*;

#[component]
pub fn MobileDrawerBackdrop(drawer_open: Signal<bool>, on_close: Callback<()>) -> impl IntoView {
    move || {
        if drawer_open.get() {
            view! {
                <div
                    class="fixed inset-0 bg-black/40 z-[var(--z-overlay)] transition-opacity duration-200 ease-out"
                    on:click=move |_| on_close.run(())
                ></div>
            }
            .into_any()
        } else {
            view! {}.into_any()
        }
    }
}
