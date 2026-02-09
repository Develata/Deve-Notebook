// apps\web\src\components\sidebar\source_control
//! # StagedSection 组件 (暂存区组件)
//!
//! 渲染暂存区 (Staged Changes) 的文件列表和操作按钮。
//! 支持折叠/展开 (VS Code 风格)。

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
    let (render_limit, set_render_limit) = signal(200usize);

    // 折叠状态
    let expanded = RwSignal::new(true);

    let staged_count = staged.len();
    let staged_list = StoredValue::new(staged.clone());
    let staged_list_for_action = StoredValue::new(staged);

    Effect::new(move |_| {
        let count = core.staged_changes.get().len();
        if count < render_limit.get() {
            set_render_limit.set(200);
        }
    });

    // 如果没有暂存文件，不渲染此区块
    if staged_count == 0 {
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
                        {move || t::source_control::staged_changes(locale.get())}
                    </span>
                </div>

                // 操作按钮 + 计数徽章
                <div class="flex items-center gap-2">
                    <div class="hidden group-hover:!flex items-center gap-1 text-[#333] dark:text-[#cccccc]" on:click=move |e| e.stop_propagation()>
                        <button
                            class="p-0.5 hover:bg-[#d0d0d0] dark:hover:bg-[#454545] rounded"
                            title=move || t::source_control::unstage_all_changes(locale.get())
                            disabled=move || core.sc_bulk_progress.get().is_some()
                            on:click=move |_| {
                                core.set_sc_bulk_failed_paths.set(vec![]);
                                let paths = staged_list_for_action
                                    .get_value()
                                    .into_iter()
                                    .map(|entry| entry.path)
                                    .collect::<Vec<_>>();
                                core.on_unstage_files.run(paths);
                            }
                        >
                            <svg xmlns="http://www.w3.org/2000/svg" class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="5" y1="12" x2="19" y2="12"/></svg>
                        </button>
                    </div>
                    <span class="bg-[#c4c4c4] dark:bg-[#454545] text-white dark:text-[#cccccc] text-[10px] px-1.5 rounded-full min-w-[16px] text-center">{staged_count}</span>
                </div>
            </div>

            // 文件列表 (可折叠)
            {move || if expanded.get() {
                view! {
                    <For
                        each=move || {
                            staged_list
                                .get_value()
                                .into_iter()
                                .take(render_limit.get())
                                .collect::<Vec<_>>()
                        }
                        key=|e| e.path.clone()
                        children=move |e| view! { <ChangeItem entry=e is_staged=true /> }
                    />
                    <Show when=move || { staged_count > render_limit.get() }>
                        <button
                            class="w-full py-1 text-[11px] text-blue-600 hover:bg-blue-50"
                            on:click=move |_| set_render_limit.update(|v| *v += 200)
                        >
                            {move || t::source_control::show_more(locale.get(), staged_count.saturating_sub(render_limit.get()))}
                        </button>
                    </Show>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }.into_any()
}
