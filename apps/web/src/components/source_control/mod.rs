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

use crate::hooks::use_core::CoreState;

#[component]
pub fn SourceControlView() -> impl IntoView {
    let core = expect_context::<CoreState>();
    
    let expand_repo = RwSignal::new(true);
    let expand_changes = RwSignal::new(true);
    let expand_history = RwSignal::new(false);

    let on_sync = move |_| {
        core.on_list_shadows.run(());
        core.on_get_changes.run(());
        core.on_get_history.run(20);
    };

    view! {
        <div class="h-full w-full bg-[#f3f3f3] dark:bg-[#252526] flex flex-col font-sans select-none overflow-hidden text-[13px] text-[#3b3b3b] dark:text-[#cccccc]">
            // Global Header
            <div class="flex-none h-9 flex items-center justify-between px-4 hover:bg-[#e4e6f1] dark:hover:bg-[#2a2d2e] group">
                 <span class="font-normal text-[11px] text-[#616161] dark:text-[#cccccc] uppercase">"源代码管理"</span>
                 <div class="flex gap-1 opacity-100 dark:text-[#cccccc]"> // Visible by default or hover? VS Code is visible on hover/focus usually, kept visible for clarity
                    <button 
                        class="p-1 hover:bg-[#0000001a] dark:hover:bg-[#ffffff1a] rounded"
                        title="Sync/Refresh"
                        on:click=on_sync
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 2v6h-6"/><path d="M3 12a9 9 0 0 1 15-6.7L21 8"/><path d="M3 22v-6h6"/><path d="M21 12a9 9 0 0 1-15 6.7L3 16"/></svg>
                    </button>
                    <button 
                        class="p-1 hover:bg-[#0000001a] dark:hover:bg-[#ffffff1a] rounded"
                        title="More Actions"
                    >
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/></svg>
                    </button>
                 </div>
            </div>
            
            // Scrollable Content
            <div class="flex-1 overflow-y-auto">
                <Repositories expanded=expand_repo />
                
                // Changes Section (Changes + Commit)
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
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5 text-[#616161] dark:text-[#cccccc]" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2v6h6"/><path d="M4 2v20h10"/><path d="M16 20h2a2 2 0 0 0 2-2V8l-6-6"/><path d="M14 2v6h6"/></svg> // Placeholder for 'View as List' or similar
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
                
                <History expanded=expand_history />
                
                <div class="h-8"></div>
            </div>
        </div>
    }
}
