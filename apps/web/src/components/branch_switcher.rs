//! # Branch Switcher Component (分支切换器)
//!
//! **架构作用**:
//! VS Code 风格的分支切换器，显示在状态栏左侧，
//! 允许用户在本地仓库 (Local/Master) 和远端影子库 (Shadow/Peer-XXX) 之间切换。
//!
//! **核心功能**:
//! - 显示当前活动分支
//! - 下拉列表显示所有可用分支
//! - 远端分支以只读模式 (Spectator Mode) 标识
//!
//! **类型**: Core MAY (扩展可选)

use leptos::prelude::*;
use crate::hooks::use_core::CoreState;
use deve_core::models::PeerId;

#[component]
pub fn BranchSwitcher() -> impl IntoView {
    let core = expect_context::<CoreState>();
    
    // 下拉菜单开启状态
    let (is_open, set_is_open) = signal(false);
    
    // 挂载时请求 Shadow 列表
    Effect::new(move |_| {
        core.on_list_shadows.run(());
    });
    
    // 获取当前分支名称
    let current_branch = move || {
        match core.active_repo.get() {
            None => "Local (Master)".to_string(),
            Some(peer) => peer.to_string(),
        }
    };
    
    // 判断是否为 Spectator (只读) 模式
    let is_spectator = move || core.active_repo.get().is_some();
    
    // 切换下拉菜单
    let toggle_dropdown = move |_| {
        set_is_open.update(|v| *v = !*v);
    };
    
    // 选择本地分支
    let select_local = move |_| {
        core.set_active_repo.set(None);
        set_is_open.set(false);
    };
    
    // 简化处理: 任何选择后关闭下拉菜单
    
    view! {
        <div class="relative">
            // 分支按钮
            <button 
                class="flex items-center gap-1.5 px-2 py-0.5 rounded hover:bg-gray-200 transition-colors text-xs font-medium text-gray-700"
                on:click=toggle_dropdown
            >
                // Git 分支图标
                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <line x1="6" y1="3" x2="6" y2="15"/>
                    <circle cx="18" cy="6" r="3"/>
                    <circle cx="6" cy="18" r="3"/>
                    <path d="M18 9a9 9 0 0 1-9 9"/>
                </svg>
                <span>{current_branch}</span>
                {move || if is_spectator() {
                    view! { <span class="text-[10px] bg-amber-100 text-amber-700 px-1 rounded font-normal">"READ"</span> }.into_any()
                } else {
                    view! {}.into_any()
                }}
                // 下拉箭头
                <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3 text-gray-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="6 9 12 15 18 9"/>
                </svg>
            </button>
            
            // 下拉菜单
            {move || if is_open.get() {
                view! {
                    <div class="absolute bottom-full left-0 mb-1 w-56 bg-white rounded-lg shadow-lg border border-gray-200 py-1 z-50">
                        // 标题头
                        <div class="px-3 py-1.5 text-[10px] uppercase tracking-wider text-gray-400 font-semibold border-b border-gray-100">
                            "Switch Branch"
                        </div>
                        
                        // Local (Master) - 始终在第一位
                        <button 
                            class=move || format!(
                                "w-full flex items-center justify-between px-3 py-1.5 text-sm hover:bg-gray-50 transition-colors {}",
                                if core.active_repo.get().is_none() { "bg-blue-50 text-blue-700" } else { "text-gray-700" }
                            )
                            on:click=select_local
                        >
                            <div class="flex items-center gap-2">
                                // 主页图标
                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                    <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/>
                                    <polyline points="9 22 9 12 15 12 15 22"/>
                                </svg>
                                <span>"Local (Master)"</span>
                            </div>
                            {move || if core.active_repo.get().is_none() {
                                view! { <span class="text-[10px] bg-blue-100 text-blue-600 px-1.5 py-0.5 rounded">"HEAD"</span> }.into_any()
                            } else {
                                view! {}.into_any()
                            }}
                        </button>
                        
                        // 分隔线 (如果有远程分支)
                        {move || if !core.shadow_repos.get().is_empty() {
                            view! {
                                <div class="border-t border-gray-100 my-1"></div>
                                <div class="px-3 py-1 text-[10px] uppercase tracking-wider text-gray-400 font-semibold">
                                    "Remote Branches"
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                        
                        // 远程分支列表 (来自 shadow_repos)
                        <For
                            each=move || core.shadow_repos.get()
                            key=|name| name.clone()
                            children=move |name| {
                                let name_for_click = name.clone();
                                let name_for_display = name.clone();
                                let name_for_check = name.clone();
                                let set_active = core.set_active_repo;
                                let set_open = set_is_open;
                                
                                view! {
                                    <button 
                                        class=move || {
                                            let is_active = core.active_repo.get()
                                                .map(|p| p.to_string() == name_for_check)
                                                .unwrap_or(false);
                                            format!(
                                                "w-full flex items-center justify-between px-3 py-1.5 text-sm hover:bg-gray-50 transition-colors {}",
                                                if is_active { "bg-amber-50 text-amber-700" } else { "text-gray-700" }
                                            )
                                        }
                                        on:click=move |_| {
                                            set_active.set(Some(PeerId::new(name_for_click.clone())));
                                            set_open.set(false);
                                        }
                                    >
                                        <div class="flex items-center gap-2">
                                            // 远程云图标
                                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-gray-400" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                                                <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z"/>
                                            </svg>
                                            <span>{name_for_display}</span>
                                        </div>
                                        <span class="text-[10px] bg-gray-100 text-gray-500 px-1.5 py-0.5 rounded">"READ"</span>
                                    </button>
                                }
                            }
                        />
                        
                        // 空状态 (如果没有远程分支)
                        {move || if core.shadow_repos.get().is_empty() {
                            view! {
                                <div class="px-3 py-2 text-xs text-gray-400 italic">
                                    "No remote branches found"
                                </div>
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }
}
