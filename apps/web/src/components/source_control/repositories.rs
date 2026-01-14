//! # Repositories Component (仓库列表组件)
//! 
//! VS Code 风格: 紧凑列表，图标+文字。

use leptos::prelude::*;
use crate::hooks::use_core::CoreState;
use deve_core::models::PeerId;

#[component]
pub fn Repositories(expanded: RwSignal<bool>) -> impl IntoView {
    let core = expect_context::<CoreState>();

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
                        // 本地 (Master)
                        <div 
                            class=move || format!(
                                "flex items-center px-4 py-1 cursor-pointer text-[13px] {}",
                                if core.active_repo.get().is_none() { "bg-[#e4ebf5] text-[#333]" } else { "hover:bg-[#f0f0f0] text-[#616161]" }
                            )
                            on:click=move |_| core.set_active_repo.set(None)
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 mr-2 opacity-80" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
                            <span class="truncate">"Local (Master)"</span>
                        </div>
                                        
                        // Shadow Repos (Remote Peers)
                        <For
                            each=move || core.shadow_repos.get()
                            key=|repo| repo.clone()
                            children=move |repo_id: String| {
                                let repo_id_for_class = repo_id.clone();
                                let repo_id_for_click = repo_id.clone();
                                let repo_id_display = if repo_id.len() > 8 {
                                    format!("Peer-{}...", &repo_id[0..8])
                                } else {
                                    format!("Peer-{}", repo_id)
                                };
                                view! {
                                    <div 
                                        class=move || format!(
                                            "flex items-center px-4 py-1 cursor-pointer text-[13px] {}",
                                            if core.active_repo.get().map(|p| p.to_string()) == Some(repo_id_for_class.clone()) { 
                                                "bg-[#e4ebf5] text-[#333]" 
                                            } else { 
                                                "hover:bg-[#f0f0f0] text-[#616161]" 
                                            }
                                        )
                                        on:click=move |_| {
                                            // Branch name is already the PeerId string
                                            core.set_active_repo.set(Some(PeerId(repo_id_for_click.clone())));
                                        }
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 mr-2 text-purple-600 opacity-80" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
                                        <span class="truncate">{repo_id_display}</span>
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
