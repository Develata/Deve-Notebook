use crate::hooks::use_core::CoreState;

use leptos::prelude::*;

#[component]
pub fn RepoSwitcher() -> impl IntoView {
    let core = expect_context::<CoreState>();
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
                class="p-1 rounded text-gray-500 hover:bg-gray-200 cursor-pointer transform transition-transform"
                class:rotate-90=move || show_menu.get()
                on:click=move |e| {
                    e.stop_propagation();
                    set_show_menu.update(|v| *v = !*v);
                }
                title="Switch Repository"
             >
                 <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4">
                     <path fill-rule="evenodd" d="M7.21 14.77a.75.75 0 01.02-1.06L11.168 10 7.23 6.29a.75.75 0 111.04-1.08l4.5 4.25a.75.75 0 010 1.08l-4.5 4.25a.75.75 0 01-1.06-.02z" clip-rule="evenodd" />
                 </svg>
             </div>

             // Dropdown Menu
             {move || if show_menu.get() {
                 view! {
                     <div
                        class="absolute left-0 top-full mt-1 w-48 bg-white dark:bg-[#252526] border border-gray-200 dark:border-[#454545] shadow-lg rounded-md z-50 py-1"
                        on:click=move |e| e.stop_propagation()
                     >
                         <div class="px-3 py-2 text-xs font-semibold text-gray-500 border-b border-gray-100 dark:border-[#454545]">
                             "Repositories"
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
                                             class="px-3 py-2 hover:bg-blue-50 dark:hover:bg-[#37373d] cursor-pointer text-xs flex items-center justify-between"
                                             class:bg-blue-50=move || is_active
                                             class:text-blue-600=move || is_active
                                             on:click=move |_| {
                                                 core.set_current_repo.set(Some(repo_name_c.clone()));
                                                 set_show_menu.set(false);
                                             }
                                         >
                                             <span class="truncate">{repo_name}</span>
                                             {if is_active {
                                                 view! { <span class="text-blue-600">"✓"</span> }.into_any()
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
