use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn RepositoriesSection(expanded: RwSignal<bool>, visible: RwSignal<bool>) -> impl IntoView {
    let core = use_context::<crate::hooks::use_core::CoreState>().expect("CoreState missing");
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // Derived State for Active Repo Name
    let active_repo_label = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| "default.redb".to_string())
    });

    view! {
        {move || if visible.get() {
            view! {
                <div class="border-t border-[#e5e5e5] dark:border-[#252526]">
                    <button
                            class="w-full flex items-center px-1 py-0.5 hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e] text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase group focus:outline-none"
                            on:click=move |_| expanded.update(|b| *b = !*b)
                    >
                        <span class=move || if expanded.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                        </span>
                        <span class="flex-1 text-left">{move || t::source_control::repositories(locale.get())}</span>
                    </button>

                    {move || if expanded.get() {
                        view! {
                            <div class="px-0 pb-1">
                                // Active Repo Row
                                <div class="flex items-center h-6 px-3 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer group text-[#3b3b3b] dark:text-[#cccccc]">
                                    // Icon
                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 mr-2 opacity-70" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20"></path><path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z"></path></svg>

                                    // Repo Name
                                    <span class="truncate font-medium flex-1">{active_repo_label}</span>

                                    // Branch Info (Right side)
                                    <div class="flex items-center gap-2 text-xs opacity-80">
                                        <div class="flex items-center gap-1 hover:text-blue-500">
                                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="6" y1="3" x2="6" y2="15"></line><circle cx="18" cy="6" r="3"></circle><circle cx="6" cy="18" r="3"></circle><path d="M18 9a9 9 0 0 1-9 9"></path></svg>
                                            <span>{move || t::source_control::branch_main(locale.get())}</span>
                                        </div>
                                        <div class="flex items-center gap-1 hover:text-blue-500">
                                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="17 8 12 3 7 8"></polyline><line x1="12" y1="3" x2="12" y2="15"></line></svg>
                                        </div>
                                        <div class="flex items-center gap-1 hover:text-blue-500">
                                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path><polyline points="7 10 12 15 17 10"></polyline><line x1="12" y1="15" x2="12" y2="3"></line></svg>
                                        </div>
                                    </div>
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
            }.into_any()
        } else {
            view! {}.into_any()
        }}
    }
}
