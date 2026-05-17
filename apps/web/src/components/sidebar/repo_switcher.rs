//! plan_ref:
//!   - 06_repository#repo-scope-runtime
//!   - 06_repository#repo-selector-resolution-contract
//!   - 08_ui_design_01_web#web-layout-persistence
//!
use crate::components::icons::ChevronRight;
use crate::hooks::use_core::BranchContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

fn repo_switcher_trigger_marker() -> &'static str {
    "repo-switcher-trigger"
}

fn repo_switcher_menu_marker(open: bool) -> Option<&'static str> {
    open.then_some("visible")
}

fn repo_switcher_backdrop_marker() -> &'static str {
    "repo-switcher-outside"
}

fn repo_switcher_item_marker() -> &'static str {
    "repo-switcher-item"
}

fn repo_switcher_after_trigger_click(open: bool) -> bool {
    !open
}

fn repo_switcher_after_outside_click() -> bool {
    false
}

fn repo_switcher_after_item_click() -> bool {
    false
}

#[component]
pub fn RepoSwitcher() -> impl IntoView {
    let core = expect_context::<BranchContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (show_menu, set_show_menu) = signal(false);

    view! {
        <div class="relative">
             // Trigger Arrow
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
                }
                title=move || t::sidebar::switch_repository(locale.get())
             >
                 <ChevronRight />
             </button>

             // Dropdown Menu
             {move || if show_menu.get() {
                 view! {
                     <div
                        class="fixed inset-0 z-[var(--z-floating)]"
                        data-deve-repo-switcher-backdrop=repo_switcher_backdrop_marker()
                        on:click=move |_| set_show_menu.set(repo_switcher_after_outside_click())
                     ></div>
                     <div
                        class="absolute left-0 top-full mt-1 w-48 bg-panel border border-default shadow-lg rounded-md z-[calc(var(--z-floating)_+_1)] py-1"
                        data-deve-repo-switcher-menu=move || repo_switcher_menu_marker(show_menu.get())
                        role="menu"
                        on:click=move |e| e.stop_propagation()
                     >
                         <div class="px-3 py-2 text-xs font-semibold text-secondary border-b border-default">
                             {move || t::source_control::repositories(locale.get())}
                         </div>
                         <div class="max-h-64 overflow-y-auto">
                             <For
                                 each=move || core.repo_list.get()
                                 key=|repo| repo.clone()
                                 children=move |repo_name| {
                                     let active_bg_name = repo_name.clone();
                                     let active_text_name = repo_name.clone();
                                     let click_name = repo_name.clone();
                                     let badge_name = repo_name.clone();
                                     let label_name = repo_name.clone();
                                     let title_name = repo_name.clone();
                                     view! {
                                         <button
                                             type="button"
                                             data-deve-repo-switcher-item=repo_switcher_item_marker()
                                             data-deve-repo-switcher-item-name=repo_name.clone()
                                             class="px-3 py-2 hover:bg-accent-subtle cursor-pointer text-xs flex items-center justify-between"
                                             class:bg-accent-subtle=move || core.current_repo.get().as_deref() == Some(active_bg_name.as_str())
                                             class:text-accent=move || core.current_repo.get().as_deref() == Some(active_text_name.as_str())
                                             role="menuitem"
                                             on:click=move |_| {
                                                 let name = click_name.clone();
                                                 let cb = core.on_switch_repo.clone();
                                                 let set_menu = set_show_menu;
                                                 request_animation_frame(move || {
                                                     cb.run(name);
                                                     set_menu.set(repo_switcher_after_item_click());
                                                 });
                                             }
                                             title=title_name
                                         >
                                             <span class="truncate text-left">{label_name}</span>
                                             {move || if core.current_repo.get().as_deref() == Some(badge_name.as_str()) {
                                                 view! { <span class="text-accent">"✓"</span> }.into_any()
                                             } else {
                                                 view! {}.into_any()
                                             }}
                                         </button>
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

#[cfg(test)]
mod tests {
    use super::{
        repo_switcher_after_item_click, repo_switcher_after_outside_click,
        repo_switcher_after_trigger_click, repo_switcher_backdrop_marker,
        repo_switcher_item_marker, repo_switcher_menu_marker, repo_switcher_trigger_marker,
    };

    #[test]
    fn repo_switcher_trigger_marker_is_stable() {
        assert_eq!(repo_switcher_trigger_marker(), "repo-switcher-trigger");
    }

    #[test]
    fn repo_switcher_menu_marker_only_exists_when_open() {
        assert_eq!(repo_switcher_menu_marker(true), Some("visible"));
        assert_eq!(repo_switcher_menu_marker(false), None);
    }

    #[test]
    fn repo_switcher_backdrop_marker_is_stable() {
        assert_eq!(repo_switcher_backdrop_marker(), "repo-switcher-outside");
    }

    #[test]
    fn repo_switcher_item_marker_is_stable() {
        assert_eq!(repo_switcher_item_marker(), "repo-switcher-item");
    }

    #[test]
    fn repo_switcher_trigger_click_toggles_menu() {
        assert!(repo_switcher_after_trigger_click(false));
        assert!(!repo_switcher_after_trigger_click(true));
    }

    #[test]
    fn repo_switcher_outside_click_closes_menu() {
        assert!(!repo_switcher_after_outside_click());
    }

    #[test]
    fn repo_switcher_item_click_closes_menu() {
        assert!(!repo_switcher_after_item_click());
    }
}
