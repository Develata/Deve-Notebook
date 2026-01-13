//! # SourceControlView 组件 (SourceControlView Component)
//!
//! 显示仓库状态、更改列表和版本历史。
//! 支持查看本地和远程分支，以及待处理的合并操作。

use leptos::prelude::*;
use crate::hooks::use_core::CoreState;
use deve_core::models::PeerId;

#[component]
pub fn SourceControlView() -> impl IntoView {
    let core = expect_context::<CoreState>();
    
    // 展开/折叠部分的状态
    let (expanded_repos, set_expanded_repos) = signal(true);
    let (expanded_changes, set_expanded_changes) = signal(true);
    let (expanded_history, set_expanded_history) = signal(false);

    // 格式化 PeerId 的辅助函数
    let format_peer_id = |id: &PeerId| {
        let s = id.to_string();
        if s.len() > 8 {
            format!("{}...", &s[0..8])
        } else {
            s
        }
    };

    view! {
        <div class="h-full w-full bg-[#f7f7f7] flex flex-col font-sans select-none overflow-hidden">
            // Header
            <div class="flex-none h-12 flex items-center justify-between px-3 border-b border-gray-200 bg-gray-50">
                 <span class="font-medium text-xs uppercase tracking-wider text-gray-500">"Source Control"</span>
                 <div class="flex gap-2">
                    <button 
                        class="p-1 hover:bg-gray-200 rounded text-gray-500"
                        title="Sync Changes"
                        // Trigger Sync?
                        on:click=move |_| { /* Trigger Sync */ }
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg>
                    </button>
                    <button 
                        class="p-1 hover:bg-gray-200 rounded text-gray-500"
                        title="View as List"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="8" y1="6" x2="21" y2="6"/><line x1="8" y1="12" x2="21" y2="12"/><line x1="8" y1="18" x2="21" y2="18"/><line x1="3" y1="6" x2="3.01" y2="6"/><line x1="3" y1="12" x2="3.01" y2="12"/><line x1="3" y1="18" x2="3.01" y2="18"/></svg>
                    </button>
                 </div>
            </div>
            
            <div class="flex-1 overflow-y-auto">
                
                // 部分: 仓库
                <div class="border-b border-gray-200">
                    <button 
                        class="w-full flex items-center px-2 py-1 hover:bg-gray-200 transition-colors text-xs font-bold text-gray-700"
                        on:click=move |_| set_expanded_repos.update(|b| *b = !*b)
                    >
                        <span class=move || if expanded_repos.get() { "transform rotate-90 mr-1 transition-transform" } else { "transform mr-1 transition-transform" }>
                            "▶"
                        </span>
                        "REPOSITORIES"
                    </button>
                    
                    {move || if expanded_repos.get() {
                        view! {
                            <div class="pl-0 pb-2">
                                // 本地 (Master)
                                <div 
                                    class=move || format!(
                                        "flex justify-between items-center px-4 py-1 cursor-pointer text-sm {}",
                                        if core.active_repo.get().is_none() { "bg-blue-100 text-blue-800" } else { "hover:bg-gray-100 text-gray-700" }
                                    )
                                    on:click=move |_| core.set_active_repo.set(None)
                                >
                                    <div class="flex items-center gap-2">
                                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
                                        <span>"Local (Master)"</span>
                                    </div>
                                    {move || if core.active_repo.get().is_none() { view!{<span class="text-[10px] bg-blue-200 px-1 rounded">"HEAD"</span>}.into_any() } else { view!{}.into_any() }}
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
                                                                    "flex justify-between items-center px-4 py-1 cursor-pointer text-sm {}",
                                                                    if core.active_repo.get().map(|p| p.to_string()) == Some(repo_id_for_class.clone()) { 
                                                                        "bg-purple-100 text-purple-800" 
                                                                    } else { 
                                                                        "hover:bg-gray-100 text-gray-700" 
                                                                    }
                                                                )
                                                                on:click=move |_| {
                                                                    if let Ok(uuid) = repo_id_for_click.parse::<uuid::Uuid>() {
                                                                        core.set_active_repo.set(Some(PeerId(uuid.to_string())));
                                                                    }
                                                                }
                                                            >
                                                                <div class="flex items-center gap-2">
                                                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 text-purple-500" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M23 21v-2a4 4 0 0 0-3-3.87"/><path d="M16 3.13a4 4 0 0 1 0 7.75"/></svg>
                                                                    <span>{repo_id_display}</span>
                                                                </div>
                                                                <span class="text-[10px] bg-purple-200 text-purple-700 px-1 rounded">"Shadow"</span>
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
                
                // 部分: 更改 (待处理的合并)
                <div class="border-b border-gray-200">
                    <button 
                        class="w-full flex items-center px-2 py-1 hover:bg-gray-200 transition-colors text-xs font-bold text-gray-700"
                        on:click=move |_| set_expanded_changes.update(|b| *b = !*b)
                    >
                         <span class=move || if expanded_changes.get() { "transform rotate-90 mr-1 transition-transform" } else { "transform mr-1 transition-transform" }>
                            "▶"
                        </span>
                        <div class="flex justify-between w-full pr-2">
                            <span>"CHANGES"</span>
                            <span class="bg-gray-200 text-gray-600 px-1.5 rounded-full text-[10px]">{core.pending_ops_count}</span>
                        </div>
                    </button>
                    
                    {move || if expanded_changes.get() {
                        let has_pending = core.pending_ops_count.get() > 0;
                        view! {
                            <div class="p-2">
                                {if has_pending {
                                    view! {
                                        <div class="flex flex-col gap-2">
                                            <div class="text-xs text-gray-500 mb-2">
                                                {format!("{} pending operations from peers", core.pending_ops_count.get())}
                                            </div>
                                            <div class="flex gap-2">
                                                <button 
                                                    class="flex-1 bg-green-600 hover:bg-green-700 text-white text-xs py-1.5 rounded flex items-center justify-center gap-1"
                                                    on:click=move |_| {
                                                        // Providing explicit merge trigger is tricky if modal is in App.rs
                                                        // But we provided set_show_merge context in App.rs? No, context is per subtree
                                                        // App -> Sidebar -> ...
                                                        // So we can expect context set_show_merge
                                                        if let Some(setter) = use_context::<WriteSignal<bool>>() {
                                                            setter.set(true);
                                                        } else {
                                                            leptos::logging::warn!("No MergeModal setter found");
                                                        }
                                                    }
                                                >
                                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
                                                    "Review & Merge"
                                                </button>
                                                <button 
                                                    class="flex-1 bg-red-50 hover:bg-red-100 text-red-600 border border-red-200 text-xs py-1.5 rounded flex items-center justify-center gap-1"
                                                    on:click=move |_| core.on_discard_pending.run(())
                                                >
                                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                                                    "Discard"
                                                </button>
                                            </div>
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="text-xs text-gray-400 text-center py-4 italic">
                                            "No pending changes"
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                </div>
                
                // 部分: 历史
                <div class="border-b border-gray-200">
                    <button 
                        class="w-full flex items-center px-2 py-1 hover:bg-gray-200 transition-colors text-xs font-bold text-gray-700"
                        on:click=move |_| set_expanded_history.update(|b| *b = !*b)
                    >
                         <span class=move || if expanded_history.get() { "transform rotate-90 mr-1 transition-transform" } else { "transform mr-1 transition-transform" }>
                            "▶"
                        </span>
                        "HISTORY"
                    </button>
                    {move || if expanded_history.get() {
                        let max_ver = core.doc_version;
                        let curr_ver = core.playback_version;
                        let set_ver = core.set_playback_version;
                         view! {
                            <div class="p-4 bg-gray-50 border-t border-gray-100">
                                <div class="flex items-center justify-between mb-2">
                                    <span class="text-xs font-semibold text-gray-500">"Time Travel"</span>
                                    <span class="text-xs font-mono bg-white border border-gray-200 px-1.5 py-0.5 rounded text-gray-600 shadow-sm">
                                        {move || format!("v{} / v{}", curr_ver.get(), max_ver.get())}
                                    </span>
                                </div>
                                <input 
                                    type="range" 
                                    min="0" 
                                    max=move || max_ver.get().to_string()
                                    value=move || curr_ver.get().to_string()
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev).parse::<u64>().unwrap_or(0);
                                        set_ver.set(val);
                                    }
                                    class="w-full h-2 bg-gray-200 rounded-lg appearance-none cursor-pointer accent-blue-500 mb-3"
                                />
                                <div class="flex justify-end">
                                    <button 
                                        class="text-xs flex items-center gap-1 text-blue-600 hover:text-blue-700 hover:bg-blue-50 px-2 py-1 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                                        disabled=move || curr_ver.get() >= max_ver.get()
                                        on:click=move |_| set_ver.set(max_ver.get())
                                    >
                                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="5 4 15 12 5 20 5 4"/><line x1="19" y1="5" x2="19" y2="19"/></svg>
                                        "Jump to Latest"
                                    </button>
                                </div>
                            </div>
                         }.into_any()
                    } else {
                        view!{}.into_any()
                    }}
                </div>
            </div>
        </div>
    }
}
