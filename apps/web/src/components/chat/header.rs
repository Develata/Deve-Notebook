// apps/web/src/components/chat/header.rs
use crate::components::icons::*;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn ChatHeader(
    #[prop(optional)] _ai_mode: Option<ReadSignal<String>>,
    #[prop(optional)] mobile: bool,
    on_close: Callback<()>,
) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        <div class=move || if mobile {
            "h-12 flex items-center px-3 border-b border-default bg-panel"
        } else {
            "h-9 flex items-center px-4 border-b border-default bg-panel"
        } style=move || if mobile {
            "padding-top: env(safe-area-inset-top); height: calc(48px + env(safe-area-inset-top));"
        } else {
            ""
        }>
            <span class="text-xs font-bold text-primary uppercase tracking-wider">{move || t::chat::panel_title(locale.get())}</span>
            <span class="ml-2 text-[10px] font-mono px-2 py-[2px] rounded bg-badge-success text-badge-success border border-badge-success">
                {move || t::chat::agent_bridge(locale.get())}
            </span>
            <div class="flex-1"></div>
            {move || if mobile {
                view! {
                    <button
                        class="chat-close-button h-11 min-w-11 rounded-md text-secondary active:bg-hover transition-colors duration-200 ease-out"
                        on:click=move |_| on_close.run(())
                        title=move || t::chat::toggle_mobile_chat(locale.get())
                        aria-label=move || t::chat::toggle_mobile_chat(locale.get())
                    >
                        <X class="w-4 h-4 mx-auto" />
                    </button>
                }
                    .into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
