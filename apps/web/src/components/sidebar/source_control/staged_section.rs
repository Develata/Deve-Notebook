// apps\web\src\components\sidebar\source_control
//! # StagedSection 组件 (暂存区组件)
//!
//! 渲染暂存区 (Staged Changes) 的文件列表和操作按钮。
//! 支持折叠/展开 (VS Code 风格)。

use super::change_item::ChangeItem;
use crate::components::icons::*;
use crate::hooks::use_core::SourceControlContext;
use crate::i18n::{Locale, t};
use deve_core::source_control::ChangeEntry;
use leptos::prelude::*;

/// 暂存区组件
#[component]
pub fn StagedSection(staged: Vec<ChangeEntry>) -> impl IntoView {
    let core = expect_context::<SourceControlContext>();
    let locale = use_context::<RwSignal<Locale>>().expect("locale context");
    let (bulk_busy, set_bulk_busy) = signal(false);

    // 折叠状态
    let expanded = RwSignal::new(true);

    let staged_count = staged.len();
    let staged_list = StoredValue::new(staged.clone());
    let staged_list_for_action = StoredValue::new(staged);

    Effect::new(move |_| {
        let _ = core.unstaged_changes.get();
        let _ = core.staged_changes.get();
        set_bulk_busy.set(false);
    });

    // 如果没有暂存文件，不渲染此区块
    if staged_count == 0 {
        return view! {}.into_any();
    }

    view! {
        <div>
            // 区块标题 (可折叠)
            <div
                class="px-2 py-0.5 flex justify-between items-center group cursor-pointer hover:bg-hover"
                on:click=move |_| expanded.update(|v| *v = !*v)
            >
                <div class="flex items-center">
                    <span class=move || format!("w-4 h-4 flex items-center justify-center text-primary transition-transform {}", if expanded.get() { "rotate-90" } else { "" })>
                        <ChevronRight class="w-3 h-3" />
                    </span>
                    <span class="text-[11px] font-bold text-primary uppercase">
                        {move || t::source_control::staged_changes(locale.get())}
                    </span>
                </div>

                // 操作按钮 + 计数徽章
                <div class="flex items-center gap-2">
                    <div class="hidden group-hover:!flex items-center gap-1 text-primary" on:click=move |e| e.stop_propagation()>
                        <button
                            class="p-0.5 hover:bg-active rounded"
                            title=move || t::source_control::unstage_all_changes(locale.get())
                            disabled=move || bulk_busy.get()
                            on:click=move |_| {
                                set_bulk_busy.set(true);
                                let paths = staged_list_for_action
                                    .get_value()
                                    .into_iter()
                                    .map(|entry| entry.path)
                                    .collect::<Vec<_>>();
                                core.on_unstage_files.run(paths);
                            }
                        >
                            <Minus class="w-3.5 h-3.5" />
                        </button>
                    </div>
                    <span class="bg-badge-count text-on-accent text-[10px] px-1.5 rounded-full min-w-[16px] text-center">{staged_count}</span>
                </div>
            </div>

            // 文件列表 (可折叠)
            {move || if expanded.get() {
                view! {
                    <For
                        each=move || staged_list.get_value()
                        key=|e| e.path.clone()
                        children=move |e| view! { <ChangeItem entry=e is_staged=true /> }
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }.into_any()
}
