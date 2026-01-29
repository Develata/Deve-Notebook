// apps\web\src\components\sidebar\source_control
//! # UnstagedSection 组件 (工作区组件)
//!
//! 渲染工作区 (Unstaged Changes) 的文件列表和操作按钮。
//! 支持折叠/展开 (VS Code 风格)。

use super::change_item::ChangeItem;
use crate::hooks::use_core::CoreState;
use crate::i18n::{t, Locale};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

/// 工作区组件
#[component]
pub fn UnstagedSection(unstaged: Vec<ChangeEntry>) -> impl IntoView {
    let core = expect_context::<CoreState>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");

    // 折叠状态
    let expanded = RwSignal::new(true);

    let unstaged_count = unstaged.len();
    let unstaged_list = StoredValue::new(unstaged.clone());
    let unstaged_list_for_stage = StoredValue::new(unstaged.clone());
    let unstaged_list_for_discard = StoredValue::new(unstaged);

    // 如果没有未暂存文件，不渲染此区块
    if unstaged_count == 0 {
        return view! {}.into_any();
    }

    view! {
        <div>
            // 区块标题 (可折叠)
            <div
                class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-[#e8e8e8] dark:hover:bg-[#2a2d2e]"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <div class="flex items-center">
                    <span class=move || format!("w-4 h-4 flex items-center justify-center text-[#424242] dark:text-[#cccccc] transition-transform {}", if expanded.get() { "rotate-90" } else { "" })>
                        <svg xmlns="http://www.w3.org/2000/svg" class="w-3 h-3" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 18l6-6-6-6"/></svg>
                    </span>
                    <span class="text-[11px] font-bold text-[#424242] dark:text-[#cccccc] uppercase">
                        {move || t::source_control::changes(locale.get())}
                    </span>
                </div>

                // 操作按钮 + 计数徽章
                <div class="flex items-center gap-2">
                    <div class="hidden group-hover:!flex items-center gap-1 text-[#333] dark:text-[#cccccc]" on:click=move |e| e.stop_propagation()>
                        // Discard All (刷新图标 - 恢复到已提交状态)
                        <button
                            class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded"
                            title="Discard All Changes"
                            on:click=move |_| {
                                for entry in unstaged_list_for_discard.get_value() {
                                    core.on_discard_file.run(entry.path);
                                }
                            }
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 3v5h5"/><path d="M3.05 13A9 9 0 1 0 6 5.3L3 8"/></svg>
                        </button>
                        // Stage All
                        <button
                            class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded"
                            title="Stage All Changes"
                            on:click=move |_| {
                                for entry in unstaged_list_for_stage.get_value() {
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

            // 文件列表 (可折叠)
            {move || if expanded.get() {
                view! {
                    <For
                        each=move || unstaged_list.get_value()
                        key=|e| e.path.clone()
                        children=move |e| view! { <ChangeItem entry=e is_staged=false /> }
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }.into_any()
}
