// apps/web/src/components/chat/drag_overlay.rs
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn DragOverlay(is_drag_over: ReadSignal<bool>) -> impl IntoView {
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    view! {
        {move || if is_drag_over.get() {
            view! {
                <div class="absolute inset-0 z-50 bg-blue-500/20 backdrop-blur-sm border-2 border-blue-500 border-dashed m-2 rounded-lg flex items-center justify-center pointer-events-none">
                    <span class="text-blue-600 dark:text-blue-400 font-bold text-lg bg-white/80 dark:bg-black/80 px-4 py-2 rounded-full">
                        {move || t::chat::drop_files(locale.get())}
                    </span>
                </div>
            }.into_any()
        } else {
            view! {}.into_any()
        }}
    }
}
