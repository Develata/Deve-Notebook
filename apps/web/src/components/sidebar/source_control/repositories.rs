//! plan_ref:
//!   - 07_diff_logic#source-control-runtime
//!   - 06_repository#repo-scope-runtime
//!
use crate::components::branch_label::current_branch_label;
use crate::components::icons::*;
use crate::components::sidebar::repo_switcher::RepoSwitcher;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub fn RepositoriesSection(expanded: RwSignal<bool>, visible: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<crate::hooks::use_core::BranchContext>();
    let source_control = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().unwrap_or_else(|| RwSignal::new(Locale::En));

    // Derived State for Active Repo Name
    let active_repo_label = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| "default.redb".to_string())
    });
    let current_branch =
        Signal::derive(move || current_branch_label(core.active_branch.get(), locale.get()));
    let branch_suffix = Signal::derive(move || {
        if source_control.staged_changes.get().is_empty()
            && source_control.unstaged_changes.get().is_empty()
        {
            ""
        } else {
            "*"
        }
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
                                <div class="flex items-center h-6 px-3 text-primary">
                                    // Icon
                                    <Book class="w-3.5 h-3.5 mr-2 opacity-70" />

                                    <RepoSwitcher />

                                    // Repo Name
                                    <span class="truncate font-medium flex-1 ml-2">{active_repo_label}</span>

                                    // Branch Info (Right side)
                                        <div class="flex items-center gap-1 text-xs opacity-80 ml-2">
                                            <div class="flex items-center gap-1">
                                            <GitBranch class="w-3 h-3" />
                                            <span>
                                                {move || format!("{}{}", current_branch.get(), branch_suffix.get())}
                                            </span>
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
