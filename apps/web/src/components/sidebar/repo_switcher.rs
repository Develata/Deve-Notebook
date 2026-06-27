//! plan_ref:
//!   - 04_repository#repo-scope-runtime
//!   - 04_repository#repo-selector-resolution-contract
//!   - 11_ui_design/01_web#web-layout-persistence
//!
use crate::components::icons::{Check, ChevronRight, EllipsisVertical, Pencil, Plus, Trash2, X};
use crate::hooks::use_core::{
    BranchContext, RepoRemoveRequest, RepoRenameRequest, RepoSwitchRequest,
};
use crate::i18n::{Locale, t};
use deve_core::models::RepoId;
use leptos::ev::{MouseEvent, SubmitEvent};
use leptos::prelude::*;

mod logic;

#[cfg(test)]
mod tests;

use self::logic::{
    repo_switcher_action_button_marker, repo_switcher_after_item_click,
    repo_switcher_after_outside_click, repo_switcher_after_trigger_click,
    repo_switcher_backdrop_marker, repo_switcher_can_submit_create_repo,
    repo_switcher_can_submit_rename_repo, repo_switcher_create_button_marker,
    repo_switcher_create_input_marker, repo_switcher_item_marker, repo_switcher_menu_marker,
    repo_switcher_remove_button_marker, repo_switcher_remove_confirmed,
    repo_switcher_rename_button_marker, repo_switcher_rename_input_marker,
    repo_switcher_row_is_active, repo_switcher_rows, repo_switcher_switch_target,
    repo_switcher_trigger_marker,
};

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
             <button
                type="button"
                data-deve-repo-switcher-trigger=repo_switcher_trigger_marker()
                class="p-1 rounded text-secondary hover:bg-hover cursor-pointer transform transition-transform"
                class:rotate-90=move || show_menu.get()
                aria-expanded=move || show_menu.get()
                aria-haspopup="menu"
                on:click=move |e| {
                    e.stop_propagation();
                    set_show_menu.update(|v| *v = repo_switcher_after_trigger_click(*v));
                    set_action_repo.set(None);
                    set_renaming_repo.set(None);
                }
                title=move || t::sidebar::switch_repository(locale.get())
             >
                 <ChevronRight />
             </button>

             {move || if show_menu.get() {
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
                         {move || if show_create.get() {
                             let submit_name = repo_name;
                             view! {
                                 <div class="border-b border-default p-2">
                                     <form
                                        class="flex items-center gap-1"
                                        on:submit=move |ev: SubmitEvent| {
                                            ev.prevent_default();
                                            let name = submit_name.get_untracked();
                                            if repo_switcher_can_submit_create_repo(&name) {
                                                core.on_create_repo.run(name.trim().to_string());
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
                             }.into_any()
                         } else {
                             view! {}.into_any()
                         }}
                         <div class="max-h-64 overflow-y-auto py-1">
                             <For
                                 each=move || repo_switcher_rows(core.repo_list.get(), core.repo_entries.get())
                                 key=|row| row.key()
                                 children=move |row| {
                                     let row_id = row.repo_id;
                                     let active_bg_row = row.clone();
                                     let active_text_row = row.clone();
                                     let active_badge_row = row.clone();
                                     let switch_target = repo_switcher_switch_target(&row);
                                     let label_name = row.name.clone();
                                     let title_name = row.name.clone();
                                     let rename_current_name = row.name.clone();
                                     let remove_current_name = row.name.clone();
                                     let action_title_name = row.name.clone();
                                     let rename_value_name = row.name.clone();
                                     let rename_input_id = row_id;
                                     let rename_submit_id = row_id;
                                     let remove_id = row_id;
                                     view! {
                                         <div class="relative">
                                             {move || if renaming_repo.get() == rename_input_id {
                                                 let current_name = rename_current_name.clone();
                                                 let current_name_for_submit = current_name.clone();
                                                 let current_name_for_disabled = current_name.clone();
                                                 view! {
                                                     <form
                                                        class="px-2 py-1.5 flex items-center gap-1 bg-accent-subtle"
                                                        on:submit=move |ev: SubmitEvent| {
                                                            ev.prevent_default();
                                                            let Some(repo_id) = rename_submit_id else {
                                                                return;
                                                            };
                                                            let new_name = rename_name.get_untracked();
                                                            if repo_switcher_can_submit_rename_repo(&current_name_for_submit, &new_name) {
                                                                core.on_rename_repo.run(RepoRenameRequest {
                                                                    repo_id,
                                                                    current_name: current_name_for_submit.clone(),
                                                                    new_name: new_name.trim().to_string(),
                                                                });
                                                                set_rename_name.set(String::new());
                                                                set_renaming_repo.set(None);
                                                                set_action_repo.set(None);
                                                                set_show_menu.set(false);
                                                            }
                                                        }
                                                     >
                                                         <input
                                                            type="text"
                                                            class="min-w-0 flex-1 bg-input border border-default rounded px-2 py-1 text-xs outline-none focus:border-accent"
                                                            data-deve-repo-switcher-rename-input=repo_switcher_rename_input_marker()
                                                            prop:value=move || rename_name.get()
                                                            aria-label=move || t::sidebar::rename_repository(locale.get())
                                                            on:input=move |ev| set_rename_name.set(event_target_value(&ev))
                                                         />
                                                         <button
                                                            type="submit"
                                                            class="p-1 rounded text-secondary hover:text-accent hover:bg-hover disabled:opacity-40"
                                                            disabled=move || !repo_switcher_can_submit_rename_repo(&current_name_for_disabled, &rename_name.get())
                                                            title=move || t::common::confirm(locale.get())
                                                            aria-label=move || t::common::confirm(locale.get())
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
                                                                set_rename_name.set(String::new());
                                                                set_renaming_repo.set(None);
                                                            }
                                                         >
                                                             <X />
                                                         </button>
                                                     </form>
                                                 }.into_any()
                                             } else {
                                                 let active_bg_row = active_bg_row.clone();
                                                 let active_text_row = active_text_row.clone();
                                                 let active_badge_row = active_badge_row.clone();
                                                 let switch_target = switch_target.clone();
                                                 let label_name = label_name.clone();
                                                 let title_name = title_name.clone();
                                                 view! {
                                                     <div
                                                        class="group flex items-center hover:bg-accent-subtle"
                                                        class:bg-accent-subtle=move || repo_switcher_row_is_active(
                                                            core.current_repo.get(),
                                                            core.current_repo_id.get(),
                                                            &active_bg_row,
                                                        )
                                                        class:text-accent=move || repo_switcher_row_is_active(
                                                            core.current_repo.get(),
                                                            core.current_repo_id.get(),
                                                            &active_text_row,
                                                        )
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
                                                                     None => RepoSwitchRequest::by_name(
                                                                         switch_target.selector_name.clone(),
                                                                     ),
                                                                 };
                                                                 let cb = core.on_switch_repo.clone();
                                                                 let set_menu = set_show_menu;
                                                                 request_animation_frame(move || {
                                                                     cb.run(request);
                                                                     set_menu.set(repo_switcher_after_item_click());
                                                                 });
                                                             }
                                                             title=title_name.clone()
                                                         >
                                                             <span class="truncate text-left">{label_name.clone()}</span>
                                                             {move || if repo_switcher_row_is_active(
                                                                 core.current_repo.get(),
                                                                 core.current_repo_id.get(),
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
                                                 }.into_any()
                                             }}
                                             {move || if let Some(repo_id) = remove_id {
                                                 if action_repo.get() != Some(repo_id) {
                                                     return view! {}.into_any();
                                                 }
                                                 let fallback_name = core
                                                     .repo_entries
                                                     .get()
                                                     .into_iter()
                                                     .find(|entry| entry.repo_id != repo_id)
                                                     .map(|entry| entry.name);
                                                 let rename_value_name = rename_value_name.clone();
                                                 let action_title_name = action_title_name.clone();
                                                 let remove_current_name = remove_current_name.clone();
                                                 let fallback_name_for_remove = fallback_name.clone();
                                                 view! {
                                                     <div class="absolute right-2 top-8 z-[calc(var(--z-floating)_+_2)] w-36 bg-panel border border-default shadow-lg rounded-md py-1 text-xs">
                                                         <button
                                                            type="button"
                                                            class="w-full px-2 py-1.5 flex items-center gap-2 text-left hover:bg-hover"
                                                            data-deve-repo-switcher-rename=repo_switcher_rename_button_marker()
                                                            on:click=move |e: MouseEvent| {
                                                                e.stop_propagation();
                                                                set_rename_name.set(rename_value_name.clone());
                                                                set_renaming_repo.set(Some(repo_id));
                                                                set_action_repo.set(None);
                                                            }
                                                         >
                                                             <Pencil class="w-3.5 h-3.5" />
                                                             <span class="truncate">{move || t::sidebar::rename_repository(locale.get())}</span>
                                                         </button>
                                                         <button
                                                            type="button"
                                                            class="w-full px-2 py-1.5 flex items-center gap-2 text-left text-danger hover:bg-hover"
                                                            data-deve-repo-switcher-remove=repo_switcher_remove_button_marker()
                                                            on:click=move |e: MouseEvent| {
                                                                e.stop_propagation();
                                                                if repo_switcher_remove_confirmed(locale.get(), &action_title_name) {
                                                                    core.on_remove_repo.run(RepoRemoveRequest {
                                                                        repo_id,
                                                                        current_name: remove_current_name.clone(),
                                                                        fallback_name: fallback_name_for_remove.clone(),
                                                                    });
                                                                    set_action_repo.set(None);
                                                                    set_renaming_repo.set(None);
                                                                    set_show_menu.set(false);
                                                                }
                                                            }
                                                         >
                                                             <Trash2 class="w-3.5 h-3.5" />
                                                             <span class="truncate">{move || t::sidebar::remove_repository(locale.get())}</span>
                                                         </button>
                                                     </div>
                                                 }.into_any()
                                             } else {
                                                 view! {}.into_any()
                                             }}
                                         </div>
                                     }
                                 }
                             />
                         </div>
                     </div>
                 }.into_any()
             } else {
                 view! {}.into_any()
             }}
        </div>
    }
}
