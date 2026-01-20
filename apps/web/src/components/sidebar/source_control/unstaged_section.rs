// apps\web\src\components\sidebar\source_control
//! # UnstagedSection 组件 (工作区组件)
//!
//! 渲染工作区 (Unstaged Changes) 的文件列表和操作按钮。

use super::change_item::ChangeItem;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

/// 工作区组件
#[component]
pub fn UnstagedSection(unstaged: Vec<ChangeEntry>) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let unstaged_count = unstaged.len();
    let unstaged_list = unstaged.clone();
    let unstaged_list_for_action = unstaged.clone();

    view! {
        <div>
            <div class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e]">
                <div class="flex items-center">
                     <span class="w-4 h-4 flex items-center justify-center text-[#424242] dark:text-[#cccccc] transform rotate-90">
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                    </span>
                    <span class="text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase">
                        {move || t::source_control::changes(locale.get())}
                    </span>
                </div>

                // 操作按钮 + 计数徽章
                <div class="flex items-center gap-2">
                    <div class="hidden group-hover:!flex items-center gap-1 text-[#333] dark:text-[#cccccc]" on:click=move |e| e.stop_propagation()>
                        // Open Changes
                        <button class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded" title="Open Changes" on:click=move |_| { /* TODO */ }>
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
                        </button>
                        // Discard All
                        <button class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded" title="Discard All Changes" on:click=move |_| { /* TODO */ }>
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v5h5"/><path d="M3.05 13A9 9 0 1 0 6 5.3L3 8"/></svg>
                        </button>
                        // Stage All
                        <button
                            class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded"
                            title="Stage All Changes"
                            on:click=move |_| {
                                for entry in unstaged_list_for_action.clone() {
                                    core.on_stage_file.run(entry.path);
                                }
                            }
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
                        </button>
                    </div>
                    <span class="bg-[#c4c4c4] dark:bg-[#454545] text-white dark:text-[#cccccc] text-[10px] px-1.5 rounded-full min-w-[16px] text-center">{unstaged_count}</span>
                </div>
            </div>
            <For
                each=move || unstaged_list.clone()
                key=|e| e.path.clone()
                children=move |e| view! { <ChangeItem entry=e is_staged=false /> }
            />
        </div>
    }
}
