use crate::hooks::use_core::BranchContext;
use crate::i18n::{Locale, t};
use leptos::prelude::*;
use crate::components::icons::ChevronRight;

#[component]
pub fn RepoSwitcher() -> impl IntoView {
    let core = expect_context::<BranchContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (show_menu, set_show_menu) = signal(false);

    // Derived active repo label
    let active_repo_label = Signal::derive(move || {
        core.current_repo
            .get()
            .unwrap_or_else(|| "default".to_string())
    });

    view! {
        <div class="relative">
             // Trigger Arrow
             <div
                class="p-1 rounded text-secondary hover:bg-hover cursor-pointer transform transition-transform"
                class:rotate-90=move || show_menu.get()
                on:click=move |e| {
                    e.stop_propagation();
                    set_show_menu.update(|v| *v = !*v);
                }
                title="Switch Repository"
             >
                 <ChevronRight />
             </div>

             // Dropdown Menu
             {move || if show_menu.get() {
                 view! {
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
                                     let repo_name_c = repo_name.clone();
                                     let is_active = repo_name == active_repo_label.get();
                                     view! {
                                         <div
                                             class="px-3 py-2 hover:bg-accent-subtle cursor-pointer text-xs flex items-center justify-between"
                                             class:bg-accent-subtle=move || is_active
                                             class:text-accent=move || is_active
                                             on:click=move |_| {
                                                 let name = repo_name_c.clone();
                                                 let cb = core.on_switch_repo.clone();
                                                 let set_menu = set_show_menu;
                                                 request_animation_frame(move || {
                                                     cb.run(name);
                                                     set_menu.set(false);
                                                 });
                                             }
                                         >
                                             <span class="truncate">{repo_name}</span>
                                             {if is_active {
                                                 view! { <span class="text-accent">"✓"</span> }.into_any()
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
