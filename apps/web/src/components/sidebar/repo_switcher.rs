//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::hooks::use_core::BranchContext;
use crate::i18n::Locale;
use deve_core::models::RepoId;
use leptos::prelude::*;

mod create_form;
mod logic;
mod menu;
mod row;
mod trigger;

#[cfg(test)]
mod tests;

use self::menu::RepoSwitcherMenu;
use self::trigger::RepoSwitcherTrigger;

#[component]
pub fn RepoSwitcher() -> impl IntoView {
    let core = expect_context::<BranchContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (show_menu, set_show_menu) = signal(false);
    let (show_create, set_show_create) = signal(false);
    let (repo_name, set_repo_name) = signal(String::new());
    let (action_repo, set_action_repo) = signal(None::<RepoId>);
    let (renaming_repo, set_renaming_repo) = signal(None::<RepoId>);
    let (rename_name, set_rename_name) = signal(String::new());

    view! {
        <div class="relative">
            <RepoSwitcherTrigger
                show_menu=show_menu
                set_show_menu=set_show_menu
                set_action_repo=set_action_repo
                set_renaming_repo=set_renaming_repo
                locale=locale
            />
            <Show when=move || show_menu.get()>
                <RepoSwitcherMenu
                    core=core.clone()
                    show_menu=show_menu
                    set_show_menu=set_show_menu
                    show_create=show_create
                    set_show_create=set_show_create
                    repo_name=repo_name
                    set_repo_name=set_repo_name
                    action_repo=action_repo
                    set_action_repo=set_action_repo
                    renaming_repo=renaming_repo
                    set_renaming_repo=set_renaming_repo
                    rename_name=rename_name
                    set_rename_name=set_rename_name
                    locale=locale
                />
            </Show>
        </div>
    }
}
