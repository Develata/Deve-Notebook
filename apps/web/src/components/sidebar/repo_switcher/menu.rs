//! plan_ref:
//!   - 04_repository#repo-catalog-contract
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Menu shell for the repository switcher view.

use crate::components::icons::Plus;
use crate::hooks::use_core::BranchContext;
use crate::i18n::{Locale, t};
use deve_core::models::RepoId;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

use super::create_form::RepoSwitcherCreateForm;
use super::logic::{
    repo_switcher_after_outside_click, repo_switcher_backdrop_marker,
    repo_switcher_create_button_marker, repo_switcher_menu_marker, repo_switcher_rows,
};
use super::row::RepoSwitcherRowView;

#[component]
pub(super) fn RepoSwitcherMenu(
    core: BranchContext,
    show_menu: ReadSignal<bool>,
    set_show_menu: WriteSignal<bool>,
    show_create: ReadSignal<bool>,
    set_show_create: WriteSignal<bool>,
    repo_name: ReadSignal<String>,
    set_repo_name: WriteSignal<String>,
    action_repo: ReadSignal<Option<RepoId>>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    renaming_repo: ReadSignal<Option<RepoId>>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    rename_name: ReadSignal<String>,
    set_rename_name: WriteSignal<String>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let row_source = core.clone();

    view! {
        <div
            class="fixed inset-0 z-[var(--z-floating)]"
            data-deve-repo-switcher-backdrop=repo_switcher_backdrop_marker()
            on:click=move |_| {
                set_show_menu.set(repo_switcher_after_outside_click());
                set_action_repo.set(None);
                set_renaming_repo.set(None);
            }
        ></div>
        <div
            class="absolute left-0 top-full mt-1 w-60 bg-panel border border-default shadow-lg rounded-md z-[calc(var(--z-floating)_+_1)] py-1"
            data-deve-repo-switcher-menu=move || repo_switcher_menu_marker(show_menu.get())
            role="menu"
            on:click=move |e: MouseEvent| e.stop_propagation()
        >
            <div class="px-3 py-1.5 text-xs font-semibold text-secondary border-b border-default flex items-center justify-between gap-2">
                <span class="truncate">{move || t::source_control::repositories(locale.get())}</span>
                <button
                    type="button"
                    class="shrink-0 p-1 rounded text-secondary hover:text-accent hover:bg-accent-subtle"
                    data-deve-repo-switcher-create=repo_switcher_create_button_marker()
                    title=move || t::sidebar::new_repository(locale.get())
                    aria-label=move || t::sidebar::new_repository(locale.get())
                    on:click=move |e: MouseEvent| {
                        e.stop_propagation();
                        set_show_create.set(true);
                        set_action_repo.set(None);
                        set_renaming_repo.set(None);
                    }
                >
                    <Plus />
                </button>
            </div>
            <Show when=move || show_create.get()>
                <RepoSwitcherCreateForm
                    repo_name=repo_name
                    set_repo_name=set_repo_name
                    set_show_create=set_show_create
                    set_show_menu=set_show_menu
                    on_create_repo=core.on_create_repo.clone()
                    locale=locale
                />
            </Show>
            <div class="max-h-64 overflow-y-auto py-1">
                <For
                    each=move || {
                        repo_switcher_rows(row_source.repo_entries.get())
                    }
                    key=|row| row.key()
                    children=move |row| {
                        view! {
                            <RepoSwitcherRowView
                                row=row
                                current_repo_id=core.current_repo_id
                                on_switch_repo=core.on_switch_repo.clone()
                                on_rename_repo=core.on_rename_repo.clone()
                                on_remove_repo=core.on_remove_repo.clone()
                                set_show_menu=set_show_menu
                                set_show_create=set_show_create
                                action_repo=action_repo
                                set_action_repo=set_action_repo
                                renaming_repo=renaming_repo
                                set_renaming_repo=set_renaming_repo
                                rename_name=rename_name
                                set_rename_name=set_rename_name
                                locale=locale
                            />
                        }
                    }
                />
            </div>
        </div>
    }
}
