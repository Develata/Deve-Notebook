use crate::components::icons::ChevronRight;
use crate::hooks::use_core::BranchContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;

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
                class="p-1 rounded text-secondary hover:bg-hover cursor-pointer transform transition-transform"
                class:rotate-90=move || show_menu.get()
                aria-expanded=move || show_menu.get()
                aria-haspopup="menu"
                on:click=move |e| {
                    e.stop_propagation();
                    set_show_menu.update(|v| *v = !*v);
                }
                title=move || t::sidebar::switch_repository(locale.get())
             >
                 <ChevronRight />
             </button>

             // Dropdown Menu
             {move || if show_menu.get() {
                 view! {
                     <div
                        class="fixed inset-0 z-40"
                        on:click=move |_| set_show_menu.set(false)
                     ></div>
                     <div
                        class="absolute left-0 top-full mt-1 w-48 bg-panel border border-default shadow-lg rounded-md z-50 py-1"
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
                                             class="px-3 py-2 hover:bg-accent-subtle cursor-pointer text-xs flex items-center justify-between"
                                             class:bg-accent-subtle=move || core.current_repo.get().as_deref() == Some(active_bg_name.as_str())
                                             class:text-accent=move || core.current_repo.get().as_deref() == Some(active_text_name.as_str())
                                             on:click=move |_| {
                                                 let name = click_name.clone();
                                                 let cb = core.on_switch_repo.clone();
                                                 let set_menu = set_show_menu;
                                                 request_animation_frame(move || {
                                                     cb.run(name);
                                                     set_menu.set(false);
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
