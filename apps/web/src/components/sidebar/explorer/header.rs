//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::icons::Plus;
use crate::components::main_layout::SearchControl;
use crate::hooks::use_core::write_gate::repo_write_allowed_for_core_tracked;
use crate::hooks::use_core::{BranchContext, CoreState};
use crate::i18n::{Locale, t};
use leptos::prelude::*;

#[component]
pub(super) fn ExplorerHeader(
    locale: RwSignal<Locale>,
    branch: BranchContext,
    core: CoreState,
    search_control: SearchControl,
    is_readonly: Signal<bool>,
) -> impl IntoView {
    let core_for_create = core.clone();
    let request_create = Callback::new(move |parent: Option<String>| {
        search_control.set_mode.set(super::new_doc_search_query(
            &core_for_create,
            parent.as_deref(),
        ));
        search_control.set_show.set(true);
    });

    let active_repo_label = Signal::derive(move || {
        branch
            .current_repo
            .get()
            .unwrap_or_else(|| t::sidebar::knowledge_base(locale.get()).to_string())
    });
    let can_write = Signal::derive(move || repo_write_allowed_for_core_tracked(&core));

    view! {
        <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-default hover:bg-hover transition-colors group">
            <div class="flex items-center gap-2 flex-1 min-w-0 text-primary">
                <crate::components::sidebar::repo_switcher::RepoSwitcher />
                <div class="overflow-hidden flex-1">
                    <span class="font-medium text-sm truncate block" title=move || active_repo_label.get()>
                        {move || active_repo_label.get()}
                    </span>
                </div>
            </div>

            <Show when=move || can_write.get() && !is_readonly.get()>
                <div class="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                    <button
                        class="p-1 rounded hover:bg-hover text-secondary"
                        title=move || t::sidebar::new_doc(locale.get())
                        on:click=move |_| request_create.run(None)
                    >
                        <Plus />
                    </button>
                </div>
            </Show>
        </div>
    }
}
