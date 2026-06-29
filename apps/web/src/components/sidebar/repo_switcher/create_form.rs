//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 11_ui_design/01_web#web-layout-persistence
//!
//! Create-repository form for the repository switcher view.

use crate::components::icons::{Check, X};
use crate::i18n::{Locale, t};
use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;

use super::logic::{repo_switcher_can_submit_create_repo, repo_switcher_create_input_marker};

#[component]
pub(super) fn RepoSwitcherCreateForm(
    repo_name: ReadSignal<String>,
    set_repo_name: WriteSignal<String>,
    set_show_create: WriteSignal<bool>,
    set_show_menu: WriteSignal<bool>,
    on_create_repo: Callback<String>,
    locale: RwSignal<Locale>,
) -> impl IntoView {
    view! {
        <div class="border-b border-default p-2">
            <form
                class="flex items-center gap-1"
                on:submit=move |ev: SubmitEvent| {
                    ev.prevent_default();
                    let name = repo_name.get_untracked();
                    if repo_switcher_can_submit_create_repo(&name) {
                        on_create_repo.run(name.trim().to_string());
                        set_repo_name.set(String::new());
                        set_show_create.set(false);
                        set_show_menu.set(false);
                    }
                }
            >
                <input
                    id="repo-switcher-create-name"
                    name="repo-name"
                    type="text"
                    class="min-w-0 flex-1 bg-input border border-default rounded px-2 py-1 text-xs outline-none focus:border-accent"
                    data-deve-repo-switcher-create-input=repo_switcher_create_input_marker()
                    prop:value=move || repo_name.get()
                    placeholder=move || t::sidebar::repository_name_placeholder(locale.get())
                    aria-label=move || t::sidebar::repository_name_placeholder(locale.get())
                    on:input=move |ev| set_repo_name.set(event_target_value(&ev))
                />
                <button
                    type="submit"
                    class="p-1 rounded text-secondary hover:text-accent hover:bg-accent-subtle disabled:opacity-40 disabled:hover:text-secondary disabled:hover:bg-transparent"
                    disabled=move || !repo_switcher_can_submit_create_repo(&repo_name.get())
                    title=move || t::sidebar::create_repository(locale.get())
                    aria-label=move || t::sidebar::create_repository(locale.get())
                >
                    <Check />
                </button>
                <button
                    type="button"
                    class="p-1 rounded text-secondary hover:text-danger hover:bg-hover"
                    title=move || t::common::cancel(locale.get())
                    aria-label=move || t::common::cancel(locale.get())
                    on:click=move |e: MouseEvent| {
                        e.stop_propagation();
                        set_repo_name.set(String::new());
                        set_show_create.set(false);
                    }
                >
                    <X />
                </button>
            </form>
        </div>
    }
}
