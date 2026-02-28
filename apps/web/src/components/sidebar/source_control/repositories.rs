use crate::components::icons::*;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn RepositoriesSection(expanded: RwSignal<bool>, visible: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<crate::hooks::use_core::BranchContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    // Derived State for Active Repo Name
    let active_repo_label = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| "default.redb".to_string())
    });

    view! {
        {move || if visible.get() {
            view! {
                <div class="border-t border-default">
                    <button
                            class="w-full flex items-center px-1 py-0.5 hover:bg-hover text-[11px] font-bold text-primary uppercase group focus:outline-none"
                            on:click=move |_| expanded.update(|b| *b = !*b)
                    >
                        <span class=move || if expanded.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                            <ChevronRight class="w-3 h-3" />
                        </span>
                        <span class="flex-1 text-left">{move || t::source_control::repositories(locale.get())}</span>
                    </button>

                    {move || if expanded.get() {
                        view! {
                            <div class="px-0 pb-1">
                                // Active Repo Row
                                <div class="flex items-center h-6 px-3 hover:bg-hover cursor-pointer group text-primary">
                                    // Icon
                                    <Book class="w-3.5 h-3.5 mr-2 opacity-70" />

                                    // Repo Name
                                    <span class="truncate font-medium flex-1">{active_repo_label}</span>

                                    // Branch Info (Right side)
                                    <div class="flex items-center gap-2 text-xs opacity-80">
                                        <div class="flex items-center gap-1 hover:text-accent">
                                            <GitBranch class="w-3 h-3" />
                                            <span>{move || t::source_control::branch_main(locale.get())}</span>
                                        </div>
                                        <div class="flex items-center gap-1 hover:text-accent">
                                                <Upload class="w-3.5 h-3.5" />
                                        </div>
                                        <div class="flex items-center gap-1 hover:text-accent">
                                                <Download class="w-3.5 h-3.5" />
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
