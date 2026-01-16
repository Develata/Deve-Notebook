//! # Repositories Component (仓库列表组件)
//! 
//! VS Code 风格: 紧凑列表，图标+文字。

use leptos::prelude::*;

#[component]
pub fn Repositories(expanded: RwSignal<bool>) -> impl IntoView {
    let core = use_context::<crate::hooks::use_core::CoreState>().expect("CoreState missing");

    view! {
        <div class="border-t border-[#e5e5e5]">
            <button 
                class="w-full flex items-center px-1 py-0.5 hover:bg-[#e8e8e8] text-[11px] font-bold text-[#424242] uppercase"
                on:click=move |_| expanded.update(|b| *b = !*b)
            >
                <span class=move || if expanded.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 text-[#424242]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                </span>
                "Repositories"
            </button>
            
            {move || if expanded.get() {
                view! {
                    <div class="pb-2">
                        <For
                            each=move || core.repo_list.get()
                            key=|repo| repo.clone()
                            children=move |repo_name| {
                                view! {
                                    <div 
                                        class="flex justify-between items-center px-4 py-1 cursor-pointer text-[13px] group hover:bg-[#f0f0f0] text-[#616161]"
                                        title="Repository Database"
                                    >
                                        <div class="flex items-center overflow-hidden">
                                            // Database Icon
                                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 mr-2 opacity-80 flex-shrink-0 text-blue-600" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                <ellipse cx="12" cy="5" rx="9" ry="3"></ellipse>
                                                <path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"></path>
                                                <path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"></path>
                                            </svg>
                                            <span class="truncate font-medium">{repo_name}.redb</span>
                                        </div>
                                        
                                        // Action Buttons (e.g. Settings, Compact)
                                        <div class="hidden group-hover:!flex items-center gap-1">
                                            <button class="p-0.5 hover:bg-[#d0d0d0] rounded" title="Database Settings">
                                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 5 9.4a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>
                                            </button>
                                        </div>
                                    </div>
                                }
                            }
                        />
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
