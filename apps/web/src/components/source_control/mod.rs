// apps\web\src\components\source_control
//! # Source Control Module (代码控制模块)
//! 
//! 包含用于版本控制的所有组件。
//! 采用 VS Code 风格的紧凑布局。
//! 
//! 样式更新 (Refined):
//! - Header: "源代码管理"
//! - Dark/Light mode compatible classes (preparatory)

pub mod repositories;
pub mod changes;
pub mod commit;
pub mod history;

use leptos::prelude::*;
use self::repositories::Repositories;
use self::changes::Changes;
use self::commit::Commit;
use self::history::History;



#[component]
pub fn SourceControlView() -> impl IntoView {
    
    let expand_repo = RwSignal::new(true);
    let expand_changes = RwSignal::new(true);
    let expand_history = RwSignal::new(false);

    // Section Visibility States
    let show_repo = RwSignal::new(true);
    let show_changes = RwSignal::new(true);
    let show_graph = RwSignal::new(true);

    // Menu State
    let show_menu = RwSignal::new(false);


    view! {
        <div class="h-full w-full bg-[#f3f3f3] dark:bg-[#252526] flex flex-col font-sans select-none overflow-hidden text-[13px] text-[#3b3b3b] dark:text-[#cccccc] relative">
            // Global Header
            <div class="flex-none h-9 flex items-center justify-between px-4 hover:bg-[#e4e6f1] dark:hover:bg-[#2a2d2e] group">
                 <span class="font-normal text-[11px] text-[#616161] dark:text-[#cccccc] uppercase">"源代码管理"</span>
                 <div class="flex gap-1 opacity-100 dark:text-[#cccccc] relative">
                    <button 
                        class="p-1 hover:bg-[#0000001a] dark:hover:bg-[#ffffff1a] rounded"
                        title="More Actions"
                        on:click=move |e| {
                            e.stop_propagation();
                            show_menu.update(|v| *v = !*v);
                        }
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/></svg>
                    </button>

                    // Context Menu
                    {move || if show_menu.get() {
                        view! {
                            <div 
                                class="absolute right-0 top-full mt-1 w-32 bg-white dark:bg-[#252526] border border-[#e5e5e5] dark:border-[#454545] shadow-lg rounded z-50 text-[12px] py-1"
                                on:click=move |e| e.stop_propagation()
                            >
                                <div 
                                    class="px-3 py-1.5 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between"
                                    on:click=move |_| { show_repo.update(|v| *v = !*v); }
                                >
                                    <span>"存储库"</span>
                                    {move || if show_repo.get() {
                                        view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                                <div 
                                    class="px-3 py-1.5 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between"
                                    on:click=move |_| { show_changes.update(|v| *v = !*v); }
                                >
                                    <span>"更改"</span>
                                    {move || if show_changes.get() {
                                        view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                                <div 
                                    class="px-3 py-1.5 hover:bg-[#e8e8e8] dark:hover:bg-[#37373d] cursor-pointer flex items-center justify-between"
                                    on:click=move |_| { show_graph.update(|v| *v = !*v); }
                                >
                                    <span>"图形"</span>
                                    {move || if show_graph.get() {
                                        view! { <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg> }.into_any()
                                    } else {
                                        view! {}.into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }}
                 </div>
            </div>
            
            // Scrollable Content
            <div class="flex-1 overflow-y-auto">
                {move || if show_repo.get() {
                    view! { <Repositories expanded=expand_repo /> }.into_any()
                } else {
                    view! {}.into_any()
                }}
                
                // Changes Section (Changes + Commit)
                {move || if show_changes.get() {
                    view! {
                        <div class="border-t border-[#e5e5e5] dark:border-[#252526]">
                            <button 
                                 class="w-full flex items-center px-1 py-0.5 hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e] text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase group"
                                 on:click=move |_| expand_changes.update(|b| *b = !*b)
                            >
                                <span class=move || if expand_changes.get() { "transform rotate-90 w-4 h-4 flex items-center justify-center transition-transform" } else { "w-4 h-4 flex items-center justify-center transition-transform" }>
                                    <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                                </span>
                                <span class="flex-1 text-left">"更改"</span>
                                <div class="hidden group-hover:flex items-center gap-1">
                                    // Add action icons if needed, e.g. Discard All, Stage All (Placeholders)
                                </div>
                            </button>
                            
                            {move || if expand_changes.get() {
                                view! {
                                    <div>
                                        <Commit />
                                        <Changes />
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
                
                {move || if show_graph.get() {
                    view! { <History expanded=expand_history /> }.into_any()
                } else {
                    view! {}.into_any()
                }}
                
                <div class="h-8"></div>
            </div>
        </div>
    }
}
