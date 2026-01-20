// apps\web\src\components\sidebar\source_control
//! # StagedSection 组件 (暂存区组件)
//!
//! 渲染暂存区 (Staged Changes) 的文件列表和操作按钮。

use super::change_item::ChangeItem;
use crate::hooks::use_core::CoreState;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

/// 暂存区组件
#[component]
pub fn StagedSection(staged: Vec<ChangeEntry>) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    let staged_count = staged.len();
    let staged_list = staged.clone();
    let staged_list_for_action = staged.clone();

    view! {
        <div>
            <div class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e]">
                <div class="flex items-center">
                    <span class="w-4 h-4 flex items-center justify-center text-[#424242] dark:text-[#cccccc] transform rotate-90">
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                    </span>
                    <span class="text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase">
                        {move || t::source_control::staged_changes(locale.get())}
                    </span>
                </div>

                // 操作按钮 + 计数徽章
                <div class="flex items-center gap-2">
                    <div class="hidden group-hover:!flex items-center gap-1 text-[#333] dark:text-[#cccccc]" on:click=move |e| e.stop_propagation()>
                        <button
                            class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded"
                            title="Unstage All Changes"
                            on:click=move |_| {
                                for entry in staged_list_for_action.clone() {
                                    core.on_unstage_file.run(entry.path);
                                }
                            }
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"/></svg>
                        </button>
                    </div>
                    <span class="bg-[#c4c4c4] dark:bg-[#454545] text-white dark:text-[#cccccc] text-[10px] px-1.5 rounded-full min-w-[16px] text-center">{staged_count}</span>
                </div>
            </div>
            <For
                each=move || staged_list.clone()
                key=|e| e.path.clone()
                children=move |e| view! { <ChangeItem entry=e is_staged=true /> }
            />
        </div>
    }
}
