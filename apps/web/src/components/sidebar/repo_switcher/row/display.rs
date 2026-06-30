//! plan_ref:
//!   - 04_repository#repo-selector-resolution-contract
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Display-mode row for the repository switcher.

use crate::components::icons::{Check, EllipsisVertical};
use crate::hooks::use_core::RepoSwitchRequest;
use crate::i18n::{Locale, t};
use deve_core::models::RepoId;
use leptos::ev::MouseEvent;
use leptos::prelude::*;

use super::super::logic::{
    RepoSwitcherRow, repo_switcher_action_button_marker, repo_switcher_after_item_click,
    repo_switcher_item_marker, repo_switcher_row_is_active, repo_switcher_switch_target,
};

#[component]
pub(super) fn RepoSwitcherDisplayRow(
    row: RepoSwitcherRow,
    current_repo: ReadSignal<Option<String>>,
    current_repo_id: ReadSignal<Option<String>>,
    on_switch_repo: Callback<RepoSwitchRequest>,
    set_show_menu: WriteSignal<bool>,
    set_show_create: WriteSignal<bool>,
    set_action_repo: WriteSignal<Option<RepoId>>,
    set_renaming_repo: WriteSignal<Option<RepoId>>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    let row_id = row.repo_id;
    let active_bg_row = row.clone();
    let active_text_row = row.clone();
    let active_badge_row = row.clone();
    let switch_target = repo_switcher_switch_target(&row);
    let label_name = row.name.clone();
    let title_name = row.name;

    view! {
        <div
            class="group flex items-center hover:bg-accent-subtle"
            class:bg-accent-subtle=move || {
                repo_switcher_row_is_active(current_repo.get(), current_repo_id.get(), &active_bg_row)
            }
            class:text-accent=move || {
                repo_switcher_row_is_active(
                    current_repo.get(),
                    current_repo_id.get(),
                    &active_text_row,
                )
            }
        >
            <button
                type="button"
                data-deve-repo-switcher-item=repo_switcher_item_marker()
                data-deve-repo-switcher-item-name=label_name.clone()
                class="min-w-0 flex-1 px-3 py-2 cursor-pointer text-xs flex items-center justify-between gap-2"
                role="menuitem"
                on:click=move |_| {
                    let request = match switch_target.repo_id {
                        Some(repo_id) => RepoSwitchRequest::exact(
                            switch_target.selector_name.clone(),
                            switch_target.expected_name.clone(),
                            repo_id,
                        ),
                        None => RepoSwitchRequest::by_name(switch_target.selector_name.clone()),
                    };
                    let cb = on_switch_repo.clone();
                    let set_menu = set_show_menu;
                    request_animation_frame(move || {
                        cb.run(request);
                        set_menu.set(repo_switcher_after_item_click());
                    });
                }
                title=title_name
            >
                <span class="truncate text-left">{label_name.clone()}</span>
                {move || if repo_switcher_row_is_active(
                    current_repo.get(),
                    current_repo_id.get(),
                    &active_badge_row,
                ) {
                    view! { <Check class="w-3 h-3 text-accent" /> }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </button>
            {move || if let Some(repo_id) = row_id {
                view! {
                    <button
                        type="button"
                        class="mr-1 p-1 rounded text-secondary hover:text-primary hover:bg-hover opacity-80 group-hover:opacity-100"
                        data-deve-repo-switcher-actions=repo_switcher_action_button_marker()
                        title=move || t::sidebar::repository_actions(locale.get())
                        aria-label=move || t::sidebar::repository_actions(locale.get())
                        on:click=move |e: MouseEvent| {
                            e.stop_propagation();
                            set_show_create.set(false);
                            set_renaming_repo.set(None);
                            set_action_repo.update(|open| {
                                *open = if *open == Some(repo_id) {
                                    None
                                } else {
                                    Some(repo_id)
                                };
                            });
                        }
                    >
                        <EllipsisVertical class="w-3.5 h-3.5" />
                    </button>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
