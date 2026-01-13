//! # Changes Component (变更列表组件)
//! 
//! 样式参考用户截图:
//! - Header: "暂存的更改" (Staged Changes) + 计数 Badge.
//! - List Item: [Icon] [Filename] [Path] ... [Status]
//! - Colors: M (Orange), A (Green), D (Red)

use leptos::prelude::*;
use crate::hooks::use_core::CoreState;
use deve_core::source_control::{ChangeEntry, ChangeStatus};

#[component]
pub fn Changes() -> impl IntoView {
    let core = expect_context::<CoreState>();
    
    Effect::new(move |_| {
        core.on_get_changes.run(());
    });
    
    // Helper to render a file item
    let render_item = move |entry: ChangeEntry, is_staged: bool| {
        let full_path = entry.path.clone();
        let path_parts: Vec<&str> = full_path.split('/').collect();
        let filename = path_parts.last().unwrap_or(&"?").to_string();
        // Path excluding filename, displayed in gray
        let directory = if path_parts.len() > 1 {
            path_parts[..path_parts.len()-1].join("/")
        } else {
            String::new()
        };

        let path_for_stage = full_path.clone();
        let path_for_unstage = full_path.clone();
        
        let (icon_char, color_cls) = match entry.status {
            ChangeStatus::Modified => ("M", "text-[#d7ba7d]"),
            ChangeStatus::Added => ("A", "text-[#73c991]"),
            ChangeStatus::Deleted => ("D", "text-[#f14c4c]"),
        };

        view! {
            <div class="flex items-center px-4 py-0.5 hover:bg-[#2a2d2e] hover:text-white text-[13px] group cursor-pointer h-[22px] text-[#cccccc]">
                <div class="flex items-center gap-1.5 flex-1 overflow-hidden">
                    <svg xmlns="http://www.w3.org/2000/svg" class=format!("w-3.5 h-3.5 min-w-3.5 {}", if filename.ends_with(".rs") { "text-[#dea584]" } else { "text-gray-400" }) viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z"/><polyline points="14 2 14 8 20 8"/></svg>
                    
                    <span class="truncate">{filename}</span>
                    <span class="text-xs text-[#808080] truncate shrink-0 ml-1">
                        {directory}
                    </span>
                </div>
                
                <div class="flex items-center gap-2 pl-2">
                    <div class="hidden group-hover:flex items-center gap-1 mr-1">
                        {if is_staged {
                            view! {
                                <button 
                                    class="p-0.5 hover:bg-[#454545] rounded text-white"
                                    title="Unstage Changes"
                                    on:click=move |ev| { ev.stop_propagation(); core.on_unstage_file.run(path_for_unstage.clone()); }
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"/></svg>
                                </button>
                            }.into_any()
                        } else {
                            view! {
                                <button 
                                    class="p-0.5 hover:bg-[#454545] rounded text-white"
                                    title="Stage Changes"
                                    on:click=move |ev| { ev.stop_propagation(); core.on_stage_file.run(path_for_stage.clone()); }
                                >
                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                                </button>
                            }.into_any()
                        }}
                    </div>

                    <span class=format!("{} text-[11px] font-bold w-3 text-center", color_cls)>
                        {icon_char}
                    </span>
                </div>
            </div>
        }
    };

    view! {
        <div>
            {move || {
                let staged = core.staged_changes.get();
                let unstaged = core.unstaged_changes.get();
                let staged_count = staged.len();
                let unstaged_count = unstaged.len();
                
                view! {
                    <div>
                        // Staged Section
                        {
                            let staged_list = staged.clone();
                            view! {
                                <div>
                                    <div class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e]">
                                        <div class="flex items-center">
                                            <span class="w-4 h-4 flex items-center justify-center text-[#424242] dark:text-[#cccccc] transform rotate-90">
                                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                                            </span>
                                            <span class="text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase">"暂存的更改"</span>
                                        </div>
                                        <span class="bg-[#c4c4c4] dark:bg-[#454545] text-white dark:text-[#cccccc] text-[10px] px-1.5 rounded-full min-w-[16px] text-center">{staged_count}</span>
                                    </div>
                                    <For
                                        each=move || staged_list.clone()
                                        key=|e| e.path.clone()
                                        children=move |e| render_item(e, true)
                                    />
                                </div>
                            }.into_any()
                        }
                        
                        // Unstaged Section
                        {
                            let unstaged_list = unstaged.clone();
                            view! {
                                <div>
                                    <div class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e]">
                                        <div class="flex items-center">
                                             <span class="w-4 h-4 flex items-center justify-center text-[#424242] dark:text-[#cccccc] transform rotate-90">
                                                <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                                            </span>
                                            <span class="text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase">"更改"</span>
                                        </div>
                                        <span class="bg-[#c4c4c4] dark:bg-[#454545] text-white dark:text-[#cccccc] text-[10px] px-1.5 rounded-full min-w-[16px] text-center">{unstaged_count}</span>
                                    </div>
                                    <For
                                        each=move || unstaged_list.clone()
                                        key=|e| e.path.clone()
                                        children=move |e| render_item(e, false)
                                    />
                                </div>
                            }
                        }
                    </div>
                }
            }}
        </div>
    }
}
